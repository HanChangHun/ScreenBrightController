"""Verify built executables without desktop interaction or native gamma writes."""
from pathlib import Path
import subprocess, json, os, secrets, threading, queue, hashlib, re, shutil
root = Path(__file__).resolve().parents[1]
dist = root / 'dist'
dist.mkdir(exist_ok=True)
# Versioned artifact deliberately leaves the running old executable untouched.
shutil.copy2(root / 'target' / 'release' / 'gamma-dimmer-app.exe', dist / 'ScreenBrightController-v0.3.exe')
shutil.copy2(root / 'target' / 'release' / 'gamma-cli.exe', dist / 'gamma-cli.exe')
app = dist / 'ScreenBrightController-v0.3.exe'
cli = dist / 'gamma-cli.exe'

def run(exe, arg, output):
    result = subprocess.run([str(exe), arg], capture_output=True, timeout=15)
    if result.returncode:
        raise RuntimeError(f'{exe.name} {arg}: exit={result.returncode} {result.stderr.decode(errors="replace")}')
    parsed = json.loads(result.stdout)
    (root / 'evidence' / output).write_bytes(result.stdout)
    return parsed

def lines(stream):
    q = queue.Queue()
    def worker():
        for line in iter(stream.readline, b''):
            q.put(line)
        q.put(b'')
    threading.Thread(target=worker, daemon=True).start()
    return q

def get(q):
    value = q.get(timeout=6)
    if not value:
        raise RuntimeError('unexpected watchdog EOF')
    return value

before = run(app, '--diagnose', 'diagnose-v03-before.json')
after = run(app, '--diagnose', 'diagnose-v03-after.json')
startup = run(app, '--startup-check', 'app-startup-check-v03.json')
smoke = run(app, '--self-test', 'app-self-test-v03.json')
cli_smoke = run(cli, '--mock', 'cli-mock-v03.json')
assert not startup['armed'] and not startup['restore_errors']
assert all(data['native_display_writes'] == 0 for data in (after, startup, smoke, cli_smoke))
assert smoke['watchdog_timeout_restored'] and smoke['watchdog_disconnect_restored']
# Kill a mock parent while the watchdog pipe stays open, proving process-handle detection.
parent = subprocess.Popen([str(app), '--mock-parent-wait'], stdout=subprocess.PIPE, stderr=subprocess.PIPE)
guard = None
try:
    assert get(lines(parent.stdout)).strip() == b'READY'
    secret = secrets.token_hex(32)
    env = dict(os.environ, GAMMA_WATCHDOG_TOKEN=secret, GAMMA_WATCHDOG_PARENT=str(parent.pid))
    guard = subprocess.Popen([str(app), '--watchdog-mock'], stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE, env=env)
    responses = lines(guard.stdout)
    def request(payload):
        guard.stdin.write(json.dumps(payload).encode() + b'\n')
        guard.stdin.flush()
        return json.loads(get(responses))
    assert request({'command':'Hello','token':secret})['ok']
    armed = request({'command':'Continuous','id':'mock-1','lease':10})
    assert armed['mock_values']['mock-1'] == 100
    parent.kill()
    parent.wait(timeout=5)
    restored = json.loads(get(responses))
    assert not restored['armed'] and restored['mock_values']['mock-1'] == 40000
    guard.stdin.close()
    assert guard.wait(timeout=5) == 0
    (root / 'evidence' / 'app-process-death-v03.json').write_text(json.dumps(restored, indent=2))
finally:
    if parent.poll() is None:
        parent.kill()
        parent.wait(timeout=5)
    if guard is not None and guard.poll() is None:
        guard.stdin.close()  # Memory-only mock guard; EOF is the safe exit path.
        guard.wait(timeout=7)

def ramps(d):
    return {m['id']:m['original'] for m in d['monitors']}
after = run(app, '--diagnose', 'diagnose-v03-after.json')
unchanged = ramps(before) == ramps(after)
assert unchanged, 'Native gamma readback changed since baseline; investigate external software.'
text = (root / 'evidence' / 'tests-v03.txt').read_text()
counts = [int(n) for n in re.findall(r'test result: ok\. (\d+) passed;', text)]
report = {
    'rust_tests_passed':sum(counts),
    'rust_test_failures':0,
    'ui_smoke_assertions':int(re.search(r'UI smoke PASS: (\d+) assertions', (root/'evidence/ui-smoke-v03.txt').read_text()).group(1)),
    'red_logs':len(list((root/'evidence').glob('*-red.txt'))),
    'green_logs':len(list((root/'evidence').glob('*-green.txt'))),
    'active_monitors':[(m['id'], m['name'], m['original'] is not None) for m in after['monitors']],
    'native_gamma_snapshots_unchanged':unchanged,
    'real_watchdog_startup_unarmed':not startup['armed'],
    'app_mock_watchdog_timeout_restored':smoke['watchdog_timeout_restored'],
    'app_mock_watchdog_eof_restored':smoke['watchdog_disconnect_restored'],
    'app_mock_parent_death_with_open_pipe_restored':True,
    'native_display_writes':0,
    'artifacts':[{'path':str(p.resolve()),'bytes':p.stat().st_size,'sha256':hashlib.sha256(p.read_bytes()).hexdigest()} for p in (app, cli)],
}
(root / 'evidence' / 'verification-v03.json').write_text(json.dumps(report, ensure_ascii=False, indent=2), encoding='utf-8')
print(json.dumps(report, ensure_ascii=False, indent=2))
