// Shared fake DOM for the UI scripts: production app.js with injected memory IPC. No native gamma.
const fs = require('node:fs'), path = require('node:path'), vm = require('node:vm');
const source = fs.readFileSync(path.join(__dirname, '../app/ui/app.js'), 'utf8');
class Element {
  constructor() { this.children = []; this.events = {}; this.dataset = {}; this.attributes = {}; this.value = '0'; this.textContent = ''; this.hidden = false; this.disabled = false; }
  addEventListener(name, fn) { this.events[name] = fn; }
  setAttribute(name, value) { this.attributes[name] = value; }
  append(...children) { this.children.push(...children); }
  replaceChildren(...children) { this.children = children; }
}
const walk = element => [element, ...element.children.flatMap(walk)];
const ids = ['monitors', 'restore', 'notice', 'notice-message', 'notice-details', 'notice-detail', 'recover', 'reset-baseline', 'close-popup'];
// One popup client in its own context, sharing `invoke` and the state-changed `listeners`.
function surface({ invoke, listeners, timeouts = { setTimeout, clearTimeout } }) {
  const nodes = Object.fromEntries(ids.map(id => [id, new Element()]));
  const document = { body: { dataset: { surface: 'popup' } }, activeElement: null, events: {},
    getElementById: id => nodes[id], createElement: () => new Element(), addEventListener(name, fn) { this.events[name] = fn; } };
  let poll;
  const listen = (name, fn) => { if (name === 'state-changed') { listeners.push(fn); poll = fn; } };
  vm.runInNewContext(source, { document, window: { __TAURI__: { core: { invoke }, event: { listen } } }, ...timeouts, console });
  return { nodes, document, poll: () => poll(), find: (id, type) => walk(nodes.monitors).find(e => e.dataset.control === id && e.type === type) };
}
module.exports = { walk, surface };
