"""Verify built executables without desktop interaction or native gamma writes."""
from pathlib import Path
import argparse, subprocess, json, os, secrets, threading, queue, hashlib
root = Path(__file__).resolve().parents[1]
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--exe", type=Path, default=root / 'target/release/ScreenBrightController.exe')
parser.add_argument("--evidence-dir", type=Path, default=root / 'evidence/package')
options = parser.parse_args()
app = options.exe.resolve()
if not app.is_file():
    parser.error(f'Executable not found: {app}; build with npm --prefix app run build first')
evidence = options.evidence_dir.resolve()
evidence.mkdir(parents=True, exist_ok=True)

def run(exe, arg, output):
    result = subprocess.run([str(exe), arg], capture_output=True, timeout=15)
    if result.returncode:
        raise RuntimeError(f'{exe.name} {arg}: exit={result.returncode} {result.stderr.decode(errors="replace")}')
    parsed = json.loads(result.stdout)
    (evidence / output).write_bytes(result.stdout)
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

import winreg
startup_key = r'Software\Microsoft\Windows\CurrentVersion\Run'
def registration():
    try:
        with winreg.OpenKey(winreg.HKEY_CURRENT_USER, startup_key, access=winreg.KEY_READ) as key:
            return winreg.QueryValueEx(key, 'ScreenBrightController')
    except FileNotFoundError:
        return None
registration_before = registration()
startup_setting = run(app, '--autostart-check', 'autostart-readonly.json')
assert startup_setting['registration_writes'] == startup_setting['native_display_writes'] == 0
before = run(app, '--diagnose', 'diagnose-before.json')
startup = run(app, '--startup-check', 'startup-check.json')
smoke = run(app, '--self-test', 'self-test.json')
assert not startup['armed'] and not startup['restore_errors']
assert all(data['native_display_writes'] == 0 for data in (before, startup, smoke))
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
    (evidence / 'process-death.json').write_text(json.dumps(restored, indent=2), encoding='utf-8')
finally:
    if parent.poll() is None:
        parent.kill()
        parent.wait(timeout=5)
    if guard is not None and guard.poll() is None:
        guard.stdin.close()  # Memory-only mock guard; EOF is the safe exit path.
        guard.wait(timeout=7)

def ramps(d):
    return {m['id']:m['original'] for m in d['monitors']}
after = run(app, '--diagnose', 'diagnose-after.json')
unchanged = ramps(before) == ramps(after)
assert unchanged, 'Native gamma readback changed since baseline; investigate external software.'
assert after['native_display_writes'] == 0
assert registration_before == registration(), 'Startup registration changed during read-only verification'
report = {
    'startup_registration_unchanged': True,
    'startup_registration_writes': 0,
    'startup_registration': startup_setting['startup'],

    'active_monitors':[(m['id'], m['name'], m['original'] is not None) for m in after['monitors']],
    'native_gamma_snapshots_unchanged':unchanged,
    'real_watchdog_startup_unarmed':not startup['armed'],
    'app_mock_watchdog_timeout_restored':smoke['watchdog_timeout_restored'],
    'app_mock_watchdog_eof_restored':smoke['watchdog_disconnect_restored'],
    'app_mock_parent_death_with_open_pipe_restored':True,
    'native_display_writes':0,
    'artifact':{'path':str(app),'bytes':app.stat().st_size,'sha256':hashlib.sha256(app.read_bytes()).hexdigest()},
}
(evidence / 'verification.json').write_text(json.dumps(report, ensure_ascii=False, indent=2), encoding='utf-8')
print(json.dumps(report, ensure_ascii=False, indent=2))
