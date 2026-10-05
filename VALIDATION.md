# Verified release — Screen Bright Controller 0.4.0

## Automated verification

- `cargo test --workspace`: **39 Rust tests passed, 0 failed**.
- `node scripts/ui-smoke.cjs`: **46 assertions passed** against the production UI script in separate main/popup fake DOMs sharing memory IPC.
- `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all -- --check`, JavaScript syntax checks and `cargo build --release --workspace`: exit 0.
- RED/GREEN tool runs observed missing live IPC, rejection of integer 23, redundant unchanged-target writes, ignored updates incorrectly restoring originals, old footer markup and old popup geometry. All corresponding tests pass after implementation. A startup-status error regression additionally verifies that the conditional banner remains visible even before the first snapshot. Popup focus-loss policy was also introduced through a failing test.
- Exact integer 0–90 backend validation; startup/status remain write-free. Controls never round or silently remap high dimming.
- Live updates use saved immutable originals, arm before the first SET, update existing targets without rearming, skip unchanged verified ramps and restore zero/excluded targets.
- Latest-wins per-control intents coalesce over 80 ms, with one live write in flight. Pending local values overlay status responses. Revision checks reject stale responses; backend generation fencing invalidates pre-Restore intents across windows, including tray-menu restoration. Restore cancels unsent local requests and waits for the in-flight write before restoring.
- Trusted user input is the live authorization boundary. Programmatic DOM synchronization / synthetic input does not apply. Pointer range and number steps are 1; slider arrows / wheel use ±5 from the current integer, Home/End use 0/90. No consent, Apply, preview button, confirmation or alert modal remains in either surface.
- Backend ignores/rejections are reported, not claimed as success. Further live attempts are fenced until Restore. If readback after an ignored update still matches the prior verified ramp, its expectation remains protected without more gamma SETs. Unknown/mismatched ramps retain best-effort watchdog recovery; external color changes restore/disarm rather than fight.
- Boundary mock regression covers 44, 45, 46, 49, 50, 51, 55, 90 with both API-accepted/readback-mismatch and API-rejected/readback-mismatch, retaining the prior verified ramp and restoring immutable originals. These injected failures do **not** prove native cutoffs.
- Continuous main-close hides without restoring; Quit restores first and cancels exit on failure. Native popup policy ignores focus loss; only deliberate close actions hide it. A Rust policy test exercises Focused(false/true); browser blur is not proof of native WebView2 behavior.

## Actual executable and native safety boundary

`python scripts/verify-release.py`, `verify-icon.py`, `verify-continuous.py` exited 0 against the final release artifact.

- Read-only diagnostics found two readable active display paths.
- `--startup-check` exercised the independent native watchdog, unarmed; no restoration writes.
- Executable mock tests verified timeout, EOF and process-handle parent death while the pipe remained open. Only dedicated memory-test parent processes were killed; existing dimmer/application/watchdog processes were untouched.
- Real elapsed-time memory-watchdog test renewed a bounded 10-second continuous lease every 2 seconds for **18 seconds**; a separate 15-second preview expired independently. Stopping renewal restored after 10 seconds. Late/invalid renewal, duplicate arm and unknown IDs were rejected. Continuous EOF restored immediately.
- Final native ramp snapshots before release verification and after all release/watchdog checks were identical. **Native gamma SET/restore calls by implementation and verification: 0.**
- One earlier read-only comparison failed because native ramp state changed during the session; no code attempted to correct it. A fresh complete verification run and its final after-all-checks comparison passed unchanged. Do not interpret the final stable interval as proof that unrelated applications never changed gamma.

## Compact browser UI

Actual production HTML/CSS/JS was rendered through a browser-only memory bridge that never overrides a native Tauri bridge. Final compact-layout evidence contains **12 passing assertions**: no footer/action region, accessible header Restore, conditionally hidden error banner, controls fit, typed 23 applies without a modal, slider arrow 23→28, shared popup state, browser blur visibility and header Restore. Earlier functional browser evidence contains **14 passing assertions**, including numeric arrow 23→24, pointer input and Home/End; it predates the final footer removal.

Final screenshots: main **740×358 content** (configured native outer window 740×390), popup **430×340**. Dark original horizontal design preserved; everyday labels are Display 1 / Display 2 in native enumeration order. Raw IDs remain internal / tooltip only. Header ↺ and native tray Restore / Restore and quit preserve recovery without a normal status footer.

Browser mouse-wheel CDP dispatch timed out once; custom wheel behavior passed the fake-DOM event tests, but actual browser wheel acceptance is not claimed. Browser interactions do not prove native WebView2/tray integration or visible native gamma effects.

## Artifacts and identity

- `dist/ScreenBrightController-v0.4.exe`: **8,046,592 bytes**, SHA-256 `29c1723729e51e806a347ac1d131a424dcb9514e9726cb9be0e0fb7bbd9e0928`.
- `dist/gamma-cli.exe`: **666,624 bytes**, SHA-256 `4caadde9da10bf659d7f0434dea3113752f730f1b64777cd770867e7f660fe16`.
- Product/FileDescription: **Screen Bright Controller**; ProductVersion: **0.4.0**.
- Supplied ICO SHA-256 unchanged: `1cbe4d747d8ef3e26840a6b700b48e83a3440fa333fbf6c994114acea72f6398`. All seven PE icon entries, window PNG and tray RGBA match the original supplied icon. No icon assets regenerated.
- Old versioned executable remains untouched. No commit, push, signing, installation, registry gamma-range change or publication performed.
- Verification scripts derive paths from their own repository location; the parent may rename the source folder after this task finishes.

Ignored local evidence: `evidence/tests-v04.txt`, `ui-smoke-v04.txt`, `clippy-v04.txt`, `build-v04.txt`, `verification-v04.json`, `continuous-v04.json`, `icon-v04.json`, `diagnose-v04-before.json`, `diagnose-v04-final.json`, `browser-v04-compact.json`, `v04-main.png`, `v04-tray.png`.

## Remaining native acceptance / operational constraints

See [MANUAL_TESTS.md](MANUAL_TESTS.md) for the short deliberate test checklist. Native live drag, wheel, WebView2 focus, popup persistence / placement and visual gamma acceptance have not been exercised by this implementation.

The user reported v0.3 dim 45 darkest and higher requests becoming ineffective / bright. Windows/driver gamma restrictions can silently ignore a ramp even when the API returns success. v0.3 also replaced its expected ramp before readback; heartbeat mismatch then restored originals. v0.4 retains a readback-confirmed prior ramp on ignored updates and displays failures, but does not bypass restrictions or guarantee dim 50–90. The exact native threshold remains unverified.

HDR / ICC / Night Light conflicts, hardware cursor differences, driver latency, hotplug / recycled display IDs, GPU/OS resets and both-process failure remain limitations. Readback/API success does not establish perceived visual effect; this is gamma scaling, not backlight brightness. Recovery is best effort if native calls block or the OS / both processes fail. No native failure injection, automatic boundary sweep or registry workaround was performed.
