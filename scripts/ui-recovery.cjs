// Recovery behavior against production UI, with memory-only IPC. No native gamma.
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');
const assert = require('node:assert/strict');
const source = fs.readFileSync(path.join(__dirname, '../app/ui/app.js'), 'utf8');
let assertions = 0;
const eq = (actual, expected) => { assert.deepEqual(actual, expected); assertions++; };
const ok = value => { assert.ok(value); assertions++; };
const settle = () => new Promise(resolve => setTimeout(resolve, 150));
class Element {
  constructor() { this.children = []; this.events = {}; this.dataset = {}; this.attributes = {}; this.value = '0'; this.textContent = ''; this.hidden = false; this.disabled = false; }
  addEventListener(name, fn) { this.events[name] = fn; }
  setAttribute(name, value) { this.attributes[name] = value; }
  append(...children) { this.children.push(...children); }
  replaceChildren(...children) { this.children = children; }
}
const walk = element => [element, ...element.children.flatMap(walk)];
const fire = (element, name, extra = {}) => element.events[name]({ isTrusted: true, preventDefault() {}, ...extra });
function backend() {
  const state = { mode: 'demo', operation: 'continuous', generation: 0, revision: 1, live_blocked: false, recovery: null,
    monitors: [{ id: 'raw-1', name: 'GPU path', supported: true }, { id: 'raw-2', name: 'GPU path', supported: true }],
    controls: { master: { dim: 35, enabled: true }, 'raw-1': { dim: 0, enabled: true }, 'raw-2': { dim: 0, enabled: true } },
    armed: ['raw-1', 'raw-2'], outcomes: [], restore_errors: [], message: 'Continuous operation applied.' };
  const calls = [], listeners = [];
  const api = { state, calls, listeners, failLive: true, failRestore: false, failStatus: false, liveDelay: null };
  api.invoke = async (command, args = {}) => {
    calls.push({ command, args });
    if (command === 'status') { if (api.statusDelay) await api.statusDelay; if (api.failStatus) throw Error('status transport failure'); return structuredClone(state); }
    if (command === 'live_control') {
      if (api.liveDelay) await api.liveDelay;
      if (args.generation !== state.generation || state.live_blocked) throw Error('Live request cancelled; Restore before retrying.');
      state.controls[args.id] = { dim: args.dim, enabled: args.enabled }; state.revision++;
      if (api.failLive) {
        state.live_blocked = true; state.recovery = 'apply'; state.generation++;
        state.outcomes = [{ id: 'raw-1', api_success: true, readback_matches: false, readback_error: null }];
        state.message = 'Live update stopped: driver ignored dimming. Restore before retrying.';
        listeners.forEach(fn => fn());
        throw Error('driver ignored dimming');
      }
    } else if (command === 'restore') {
      state.generation++; state.revision++;
      if (api.failRestore) {
        state.live_blocked = true; state.recovery = 'restore'; state.message = 'Restore failed: mock failure. Keep the app open and retry.';
        listeners.forEach(fn => fn()); throw Error('mock restore failure');
      }
      state.live_blocked = false; state.recovery = null; state.armed = []; state.outcomes = []; state.operation = 'idle';
      for (const control of Object.values(state.controls)) control.dim = 0;
      state.message = 'Saved original gamma restored.';
    } else if (command === 'reset_baseline') {
      for (const monitor of state.monitors) monitor.dimmed = false;
      state.revision++;
    } else throw Error('Unexpected command: ' + command);
    listeners.forEach(fn => fn());
    return structuredClone(state);
  };
  return api;
}
function surface(api, popup = false, timeouts = { setTimeout, clearTimeout }) {
  const ids = ['monitors', 'restore', 'notice', 'notice-message', 'notice-details', 'notice-detail', 'recover', 'reset-baseline', 'close-popup'];
  const nodes = Object.fromEntries(ids.map(id => [id, new Element()]));
  const document = { body: { dataset: { surface: 'popup' } }, activeElement: null,
    getElementById: id => nodes[id], createElement: () => new Element(), addEventListener() {} };
  let poll;
  vm.runInNewContext(source, { document, window: { __TAURI__: { core: { invoke: api.invoke }, event: { listen: (name, fn) => { if (name === 'state-changed') { api.listeners.push(fn); poll = fn; } } } } },
    ...timeouts, console });
  return { nodes, document, poll: () => poll(), find: (id, type) => walk(nodes.monitors).find(e => e.dataset.control === id && e.type === type) };
}
(async () => {
  const api = backend(), main = surface(api), popup = surface(api, true);
  await settle();
  const slider = main.find('master', 'range');
  eq(slider.disabled, false);
  slider.value = '52'; fire(slider, 'input'); await settle(); await settle();
  eq(slider.disabled, true);
  eq(main.find('master', 'checkbox').disabled, true);
  eq(popup.find('master', 'range').disabled, true);
  eq(main.nodes.restore.disabled, false);
  eq(main.nodes.recover.hidden, false);
  ok(main.nodes['notice-message'].textContent.includes('unconfirmed'));
  ok(!main.nodes['notice-message'].textContent.includes('Live request cancelled'));
  ok(main.nodes['notice-detail'].textContent.includes('API: accepted'));
  ok(main.nodes['notice-detail'].textContent.includes('readback: not matched'));
  eq(main.nodes.notice.hidden, false);
  const count = api.calls.filter(call => call.command === 'live_control').length;
  slider.value = '80'; fire(slider, 'input'); await settle();
  eq(api.calls.filter(call => call.command === 'live_control').length, count);
  await main.poll(); await settle();
  eq(slider.disabled, true);
  await fire(popup.nodes.recover, 'click'); await settle();
  eq(api.calls.filter(call => call.command === 'restore').length, 1);
  eq(main.find('master', 'number').value, '0');
  eq(slider.disabled, false);
  eq(main.nodes.notice.hidden, true);
  eq(main.nodes.recover.hidden, true);
  slider.value = '52'; fire(slider, 'input'); await settle();
  api.failRestore = true;
  await fire(main.nodes.recover, 'click'); await settle();
  eq(main.nodes['notice-message'].textContent, 'Restore failed. Keep the app open and try again.');
  eq(slider.disabled, true);
  eq(main.nodes.recover.disabled, false);
  ok(main.nodes['notice-detail'].textContent.includes('mock failure'));
  api.failRestore = false;
  await fire(main.nodes.recover, 'click'); await settle();
  eq(main.nodes.notice.hidden, true);
  Object.assign(api.state, { live_blocked: true, recovery: 'safety', message: 'Safety stop: raw-2 changed externally.', generation: api.state.generation + 1, revision: api.state.revision + 1 });
  await popup.poll();
  eq(popup.nodes['notice-message'].textContent, 'Display state changed. Restore to continue.');
  ok(popup.nodes['notice-detail'].textContent.includes('Display 2 changed externally'));
  eq(popup.find('master', 'checkbox').disabled, true);
  await fire(popup.nodes.recover, 'click'); await settle();
  slider.value = '25'; fire(slider, 'input');
  api.failStatus = true; await main.poll();
  eq(slider.disabled, true);
  eq(main.nodes['notice-message'].textContent, 'Display status unavailable. Restore is still available.');
  ok(main.nodes['notice-detail'].textContent.includes('status transport failure'));
  eq(main.nodes.recover.hidden, false);
  const beforeStatusFailure = api.calls.filter(call => call.command === 'live_control').length;
  await settle();
  eq(api.calls.filter(call => call.command === 'live_control').length, beforeStatusFailure);
  api.failStatus = false; await main.poll();
  eq(slider.disabled, false);
  eq(main.nodes.notice.hidden, true);
  eq(main.find('master', 'number').value, '0');
  api.state.restore_errors = ['raw-1: restore readback mismatch']; api.state.revision++;
  await main.poll();
  eq(slider.disabled, true);
  eq(main.nodes['notice-message'].textContent, 'Restore failed. Keep the app open and try again.');
  ok(main.nodes['notice-detail'].textContent.includes('Display 1: restore readback mismatch'));
  api.state.restore_errors = []; api.state.revision++; await main.poll();
  // A dimmed saved original is only reported; resetting it needs a trusted click.
  const baselineApi = backend();
  baselineApi.state.monitors[0].dimmed = true;
  const baseline = surface(baselineApi), resets = () => baselineApi.calls.filter(call => call.command === 'reset_baseline').length;
  await settle();
  ok(baseline.nodes['notice-message'].textContent.startsWith('Display 1 started dimmed'));
  eq(baseline.nodes['reset-baseline'].hidden, false);
  eq(baseline.nodes.recover.hidden, true);
  await fire(baseline.nodes['reset-baseline'], 'click', { isTrusted: false });
  eq(resets(), 0);
  await fire(baseline.nodes['reset-baseline'], 'click'); await settle();
  eq(resets(), 1);
  eq(baseline.nodes.notice.hidden, true);
  eq(baseline.nodes['reset-baseline'].hidden, true);
  const store = new Map(), demoWindow = { addEventListener() {} };
  vm.runInNewContext(fs.readFileSync(path.join(__dirname, '../app/demo/demo.js'), 'utf8'), {
    window: demoWindow, location: { search: '?demo=1' }, URLSearchParams,
    localStorage: { getItem: key => store.get(key) || null, setItem: (key, value) => store.set(key, value) },
  });
  const demo = demoWindow.__TAURI__.core;
  const initial = await demo.invoke('status');
  eq(initial.live_blocked, false);
  await demo.invoke('live_control', { id: 'master', dim: 35, enabled: true, generation: 0 });
  const [key] = store.keys();
  const blocked = JSON.parse(store.get(key)); blocked.live_blocked = true; blocked.recovery = 'apply';
  store.set(key, JSON.stringify(blocked));
  await assert.rejects(demo.invoke('live_control', { id: 'master', dim: 52, enabled: true, generation: 0 })); assertions++;
  const reset = await demo.invoke('restore');
  eq(reset.live_blocked, false); eq(reset.recovery, null);
  eq(reset.controls.master.dim, 0);
  const queuedApi = backend(), queued = surface(queuedApi);
  await settle();
  let releaseLive; queuedApi.liveDelay = new Promise(resolve => { releaseLive = resolve; });
  const queuedSlider = queued.find('master', 'range'), focusedNumber = queued.find('master', 'number');
  queued.document.activeElement = focusedNumber;
  queuedSlider.value = '52'; fire(queuedSlider, 'input'); await settle();
  queuedSlider.value = '80'; fire(queuedSlider, 'input');
  releaseLive(); await settle(); await settle();
  eq(queuedApi.calls.filter(call => call.command === 'live_control').length, 1);
  eq(focusedNumber.value, '52');
  eq(queuedSlider.disabled, true);
  const restoreCount = queuedApi.calls.filter(call => call.command === 'restore').length;
  await fire(queued.nodes.recover, 'click', { isTrusted: false });
  eq(queuedApi.calls.filter(call => call.command === 'restore').length, restoreCount);
  const staleApi = backend(), staleMain = surface(staleApi), stalePopup = surface(staleApi, true);
  staleApi.failLive = false;
  await settle();
  let releaseStale; staleApi.liveDelay = new Promise(resolve => { releaseStale = resolve; });
  const staleSlider = staleMain.find('master', 'range');
  staleSlider.value = '52'; fire(staleSlider, 'input'); await settle();
  await fire(stalePopup.nodes.restore, 'click');
  releaseStale(); await settle(); await settle();
  eq(staleMain.nodes.notice.hidden, true);
  eq(staleMain.find('master', 'number').value, '0');
  eq(staleSlider.disabled, false);
  eq(staleApi.calls.filter(call => call.command === 'live_control').length, 1);
  // Control the debounce clock and deferred IPC independently: no wall-clock race.
  const pendingTimers = new Map(); let timerId = 0;
  const timeouts = {
    setTimeout(fn) { const id = ++timerId; pendingTimers.set(id, fn); return id; },
    clearTimeout(id) { pendingTimers.delete(id); },
  };
  const flushMicrotasks = () => new Promise(setImmediate);
  const runTimers = async () => {
    const ready = [...pendingTimers.values()]; pendingTimers.clear();
    for (const fn of ready) fn();
    await flushMicrotasks();
  };
  const freshApi = backend(), freshMain = surface(freshApi, false, timeouts), freshPopup = surface(freshApi, true, timeouts);
  freshApi.failLive = false;
  await flushMicrotasks();
  let releaseOld; freshApi.liveDelay = new Promise(resolve => { releaseOld = resolve; });
  const freshSlider = freshMain.find('master', 'range');
  freshSlider.value = '52'; fire(freshSlider, 'input'); await runTimers();
  eq(freshApi.calls.filter(call => call.command === 'live_control').length, 1);
  await fire(freshPopup.nodes.restore, 'click');
  eq(freshApi.state.generation, 1);
  eq(freshMain.find('master', 'number').value, '0');
  eq(freshSlider.disabled, false);
  // A new trusted gesture arrives only after the other client completed Restore.
  freshSlider.value = '27'; fire(freshSlider, 'input'); await runTimers();
  eq(freshMain.find('master', 'number').value, '27');
  eq(freshApi.state.controls.master.dim, 0);
  eq(freshApi.calls.filter(call => call.command === 'live_control').length, 1);
  releaseOld(); await flushMicrotasks(); await runTimers();
  assert.deepEqual(freshApi.calls.filter(call => call.command === 'live_control').map(({ args }) => ({
    id: args.id, dim: args.dim, generation: args.generation,
  })), [
    { id: 'master', dim: 52, generation: 0 },
    { id: 'master', dim: 27, generation: 1 },
  ], 'Obsolete rejection must preserve the fresh trusted intent queued after Restore'); assertions++;
  eq(freshApi.state.controls.master.dim, 27);
  eq(freshMain.find('master', 'number').value, '27');
  eq(freshPopup.find('master', 'number').value, '27');
  eq(freshSlider.disabled, false);
  eq(freshMain.nodes.notice.hidden, true);
  eq(freshPopup.nodes.notice.hidden, true);
  // Deferred status errors must not invalidate an accepted external Restore.
  const statusApi = backend(), statusMain = surface(statusApi, false, timeouts);
  statusApi.failLive = false;
  await flushMicrotasks();
  const originalInvoke = statusApi.invoke;
  let rejectStatus;
  // surface holds the original function, so inject deferral in the backend itself.
  statusApi.statusDelay = new Promise((resolve, reject) => { rejectStatus = reject; });
  const oldStatus = statusMain.poll();
  statusApi.statusDelay = null;
  await originalInvoke('restore'); await flushMicrotasks();
  const statusSlider = statusMain.find('master', 'range');
  statusSlider.value = '27'; fire(statusSlider, 'input');
  rejectStatus(Error('obsolete status failure')); await oldStatus;
  eq(statusSlider.disabled, false);
  eq(statusMain.nodes.notice.hidden, true);
  await runTimers();
  eq(statusApi.calls.filter(call => call.command === 'live_control').map(call => call.args.dim), [27]);
  // A later successful read fences the earlier failure even at equal revision.
  statusApi.statusDelay = new Promise((resolve, reject) => { rejectStatus = reject; });
  const olderSameGeneration = statusMain.poll();
  statusApi.statusDelay = null;
  await statusMain.poll();
  statusSlider.value = '31'; fire(statusSlider, 'input');
  rejectStatus(Error('older same-generation status failure')); await olderSameGeneration;
  eq(statusSlider.disabled, false);
  eq(statusMain.nodes.notice.hidden, true);
  await runTimers();
  eq(statusApi.calls.filter(call => call.command === 'live_control').map(call => call.args.dim), [27, 31]);
  // A genuinely current transport failure still cancels pending writes.
  statusSlider.value = '40'; fire(statusSlider, 'input');
  statusApi.failStatus = true; await statusMain.poll();
  eq(statusSlider.disabled, true);
  eq(statusMain.nodes.recover.hidden, false);
  await runTimers();
  eq(statusApi.calls.filter(call => call.command === 'live_control').map(call => call.args.dim), [27, 31]);
  const native = { core: { invoke() { throw Error('Native bridge must not be called by demo initialization'); } } };
  const nativeWindow = { __TAURI__: native };
  vm.runInNewContext(fs.readFileSync(path.join(__dirname, '../app/demo/demo.js'), 'utf8'), { window: nativeWindow, location: { search: '?demo=1' }, URLSearchParams });
  eq(nativeWindow.__TAURI__, native);
  console.log(`Recovery UI PASS: ${assertions} assertions`);
})().catch(error => { console.error(error); process.exitCode = 1; });
