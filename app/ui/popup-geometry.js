/* Native window gestures only. Never dispatch a display control or startup command. */
'use strict';
const popupInteractive = 'button, input, select, textarea, a, label, [role="button"], [contenteditable]:not([contenteditable="false"])';
function popupPrimaryMouse(event) {
  return event.isTrusted && event.button === 0 && event.detail === 1;
}
document.querySelector('.titlebar').addEventListener('mousedown', event => {
  if (!popupPrimaryMouse(event) || event.target.closest(popupInteractive)) return;
  event.preventDefault();
  window.__TAURI__.window.getCurrentWindow().startDragging()
    .catch(error => console.warn('Window drag unavailable:', error));
});
for (const handle of document.querySelectorAll('[data-resize]')) {
  handle.addEventListener('mousedown', event => {
    if (!popupPrimaryMouse(event)) return;
    event.preventDefault();
    window.__TAURI__.window.getCurrentWindow().startResizeDragging(handle.dataset.resize)
      .catch(error => console.warn('Window resize unavailable:', error));
  });
}
