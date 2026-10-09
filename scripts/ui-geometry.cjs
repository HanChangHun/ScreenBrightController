// Production popup gestures with injected native window API; no gamma/registry/GUI.
const fs = require('node:fs'), path = require('node:path'), vm = require('node:vm');
const assert = require('node:assert/strict');
let assertions = 0;
const check = (value, message) => { assert.ok(value, message); assertions++; };
const equal = (actual, expected) => { assert.deepEqual(JSON.parse(JSON.stringify(actual)), JSON.parse(JSON.stringify(expected))); assertions++; };
const file = path.join(__dirname, '../app/ui/popup-geometry.js');
function fixture() {
  const calls = [], warnings = [];
  const header = { events: {}, addEventListener(name, fn) { this.events[name] = fn; } };
  const directions = ['North', 'NorthEast', 'East', 'SouthEast', 'South', 'SouthWest', 'West', 'NorthWest'];
  const handles = directions.map(direction => ({ dataset: { resize: direction }, events: {},
    addEventListener(name, fn) { this.events[name] = fn; } }));
  const document = { querySelector: () => header, querySelectorAll: () => handles };
  const window = { __TAURI__: { window: { getCurrentWindow: () => ({
    startDragging: async () => calls.push(['drag']),
    startResizeDragging: async direction => calls.push(['resize', direction])
  }) } } };
  vm.runInNewContext(fs.existsSync(file) ? fs.readFileSync(file, 'utf8') : '', {
    document, window, console: { warn: (...args) => warnings.push(args) }
  });
  return { calls, header, handles, warnings };
}
const event = (extra = {}) => ({ isTrusted: true, button: 0, detail: 1,
  target: { closest: () => null }, preventDefault() { this.prevented = true; }, ...extra });
(async () => {
  const f = fixture();
  check(typeof f.header.events.mousedown === 'function', 'Header must wire native mouse drag');
  const down = event(); f.header.events.mousedown(down);
  await Promise.resolve(); equal(f.calls, [['drag']]); equal(down.prevented, true);
  for (const extra of [{ isTrusted: false }, { button: 1 }, { button: 2 }, { detail: 2 },
    { target: { closest: () => ({ tagName: 'BUTTON' }) } },
    { target: { closest: () => ({ tagName: 'INPUT' }) } }]) {
    f.header.events.mousedown(event(extra));
  }
  await Promise.resolve(); equal(f.calls, [['drag']]);
  equal(f.warnings, []);
  for (const handle of f.handles) {
    check(typeof handle.events.mousedown === 'function', `${handle.dataset.resize} must wire native resize`);
    const down = event(); handle.events.mousedown(down);
    equal(down.prevented, true);
  }
  await Promise.resolve();
  equal(f.calls.slice(1), f.handles.map(handle => ['resize', handle.dataset.resize]));
  for (const handle of f.handles) {
    handle.events.mousedown(event({ isTrusted: false }));
    handle.events.mousedown(event({ button: 2 }));
    handle.events.mousedown(event({ detail: 2 }));
  }
  await Promise.resolve(); equal(f.calls.length, 9);
  const store = new Map();
  const demoWindow = { addEventListener() {} };
  vm.runInNewContext(fs.readFileSync(path.join(__dirname, '../app/ui/demo.js'), 'utf8'), {
    window: demoWindow, location: { search: '?demo=1' }, URLSearchParams,
    localStorage: { getItem: key => store.get(key) || null, setItem: (key, value) => store.set(key, value) }
  });
  const before = JSON.stringify(await demoWindow.__TAURI__.core.invoke('status'));
  check(Boolean(demoWindow.__TAURI__.window), 'Browser demo must support memory-only window gestures');
  await demoWindow.__TAURI__.window.getCurrentWindow().startDragging();
  await demoWindow.__TAURI__.window.getCurrentWindow().startResizeDragging('SouthEast');
  equal(demoWindow.__TAURI__.window.gestures.map(row => row.command),
    ['plugin:window|start_dragging', 'plugin:window|start_resize_dragging']);
  equal(JSON.stringify(await demoWindow.__TAURI__.core.invoke('status')), before);
  const nativeBridge = { native: true }, nativeWindow = { __TAURI__: nativeBridge };
  vm.runInNewContext(fs.readFileSync(path.join(__dirname, '../app/ui/demo.js'), 'utf8'), {
    window: nativeWindow, location: { search: '?demo=1' }, URLSearchParams
  });
  equal(nativeWindow.__TAURI__, nativeBridge);
  console.log(`Geometry UI PASS: ${assertions} assertions`);
})().catch(error => { console.error(error); process.exitCode = 1; });
