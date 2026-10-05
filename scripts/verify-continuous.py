"""Exercise the actual release watchdog in memory only; never invokes native gamma set."""
from pathlib import Path
import json, os, secrets, subprocess, time, hashlib
root = Path(__file__).resolve().parents[1]
exe = root / 'dist' / 'ScreenBrightController-v0.3.exe'
secret = secrets.token_hex(32)
env = dict(os.environ, GAMMA_WATCHDOG_TOKEN=secret, GAMMA_WATCHDOG_PARENT=str(os.getpid()))
guard = subprocess.Popen([str(exe), '--watchdog-mock'], stdin=subprocess.PIPE, stdout=subprocess.PIPE, env=env)
def request(payload):
    guard.stdin.write((json.dumps(payload)+'\n').encode())
    guard.stdin.flush()
    line = guard.stdout.readline()
    assert line, 'watchdog unexpectedly exited'
    return json.loads(line)
def accepted(payload):
    result = request(payload)
    assert result['ok'], result
    return result
try:
    accepted({'command':'Hello','token':secret})
    accepted({'command':'Continuous','id':'mock-1','lease':10})
    accepted({'command':'Arm','id':'mock-2','seconds':15})
    assert not request({'command':'Continuous','id':'mock-1','lease':10})['ok']
    assert not request({'command':'Renew','id':'mock-2','lease':10})['ok']
    assert not request({'command':'Renew','id':'missing','lease':10})['ok']
    assert not request({'command':'Renew','id':'mock-1','lease':11})['ok']
    for _ in range(9):
        time.sleep(2)
        accepted({'command':'Renew','id':'mock-1','lease':10})
    healthy = accepted({'command':'Status'})
    assert healthy['armed'] == ['mock-1'], healthy
    assert healthy['mock_values'] == {'mock-1':100,'mock-2':50000}, healthy
    time.sleep(10.4)
    expired = accepted({'command':'Status'})
    assert not expired['armed'] and expired['mock_values']['mock-1'] == 40000, expired
    assert not request({'command':'Renew','id':'mock-1','lease':10})['ok']
    accepted({'command':'Continuous','id':'mock-1','lease':10})
    guard.stdin.close()  # Continuous-mode EOF must restore immediately, not wait for lease.
    eof = json.loads(guard.stdout.readline())
    assert eof['mock_values']['mock-1'] == 40000 and not eof['armed'], eof
    assert guard.wait(timeout=5) == 0
    result = {'release_exe':exe.name,'sha256':hashlib.sha256(exe.read_bytes()).hexdigest(), 'continuous_survived_18_seconds':True,'preview_expired_independently_after_15_seconds':True,'lease_loss_restored_after_10_seconds':True,'continuous_pipe_eof_restored':True,'duplicate_unknown_invalid_lease_and_preview_renew_rejected':True,'native_display_writes':0}
    (root/'evidence/continuous-v03.json').write_text(json.dumps(result,indent=2),encoding='utf-8')
    print(json.dumps(result,indent=2))
finally:
    if guard.poll() is None:
        if not guard.stdin.closed: guard.stdin.close()
        guard.wait(timeout=7)
