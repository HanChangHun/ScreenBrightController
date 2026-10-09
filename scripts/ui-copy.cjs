const fs = require('node:fs');
const path = require('node:path');
const assert = require('node:assert/strict');
let assertions = 0;
function check(value, message) { assert.ok(value, message); assertions++; }
for (const file of ['index.html']) {
  const source = fs.readFileSync(path.join(__dirname, '../app/ui', file), 'utf8');
  for (const text of ['DISPLAY / OPERATE', 'Dimming amount', '0–90 / integer', 'class="version"', 'class="section-heading"']) {
    check(!source.includes(text), `${file} must omit ${text}`);
  }
  check(source.includes('Screen Bright Controller'), 'Keep the app title');
  check(source.includes('aria-label="Display dimming controls"'), 'Keep the accessible section name');
  check(source.includes('aria-label="Restore original"'), 'Keep recovery accessible');
  check(source.includes('id="recover"'), 'Provide an explicit recovery action');
  check(/id="close-popup"[^>]*>−<\/button>/.test(source), 'Use a minus sign for hiding, not a close X');
  check(/id="close-popup"[^>]*aria-label="Hide popup"/.test(source), 'Retain accessible hide behavior');
}
const source = fs.readFileSync(path.join(__dirname, '../app/ui/app.js'), 'utf8');
for (const text of ['One value for included displays', 'control-note', "?'Linked':"]) {
  check(!source.includes(text), `Remove redundant visible helper text: ${text}`);
}
check(source.includes("'Link displays'"), 'Keep the link checkbox label');
check(source.includes('Display ${'), 'Keep individual display labels');
console.log(`UI copy PASS: ${assertions} assertions`);
