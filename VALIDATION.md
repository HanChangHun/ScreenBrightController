# Tray-only recovery and compact UI — 0.5.1

## Changes

- Expose backend `live_blocked` and a typed recovery reason (`apply`, `safety`, `restore`) in the status snapshot. The existing gamma controller, readback policy and independent watchdog are unchanged.
- Create only one initially hidden popup; remove the native main window, Open app command/button/menu and its extra HTML entry point. Normal startup, login startup and second-instance activation do not open a window. Only a tray left-click shows/toggles the popup. Settings now lives in the popup; right-click Restore and Restore-and-quit remain available.
- Pause all dimming inputs on application failure; discard unsent intents and stop repeated requests. Keep requested values visibly unconfirmed rather than claiming a rejected value was applied. Preserve generation fencing across Restore, including stale in-flight responses from an independent test client.
- Show one compact recovery notice with an explicit **Restore** button and collapsed **Details** (API result versus readback). Failed restoration remains paused and retryable; unavailable status also blocks new intents until status returns. No automatic retry, driver-range clamp, registry bypass or native dimming sweep.
- Remove the visible eyebrow, version badge, Dimming amount/range caption, and repeated row notes. Preserve app/display labels, link/include checkboxes, exact 0–90 controls, Settings and header Restore. The compact settings and collapsed recovery panel fit the tray popup. Errors update the tray tooltip instead of creating or focusing a separate window.

## Executed validation

- **56 Rust tests**, **160 UI assertions** (41 live, 31 settings, 74 recovery, 14 copy), **5 packaging tests**, and **1 tray-only contract test** passed. Cargo fmt/clippy passed with warnings denied.
- A delayed old-generation rejection cannot cancel fresh input queued after an external Restore. The new deterministic fake-timer/deferred-IPC regression failed before the fix, then passed; the parent reran the suites and verified the same ordering with trusted number-field input in a browser. Current failures and blocked recovery still cancel pending requests, and Restore alone does not re-dim.
- The source packages are now `screen-bright-controller` and `screen-bright-controller-tray`; the diagnostic target is `screen-bright-controller-cli`. Renamed 10 source/asset files, preserved all 9 reference icon assets byte-for-byte, updated imports, both watchdog environment readers/writers and both native/frontend `display-error` endpoints. The public executable/install identifier remain unchanged. **1 branding contract test** and **4 icon-asset tests** passed. External Cargo dependency records did not change. A fresh filesystem audit (excluding Git internals and third-party node_modules) found no legacy-named paths; current source has no old product identifiers except intentional negative test fixtures. Gamma API terminology and historical evidence are not rewritten as new results.
- **8 browser-only scenarios** passed against the final single production popup at 430×340: clean tray-only UI, settings read, paused state, technical details, trusted Restore without any automatic live call, X hide, Escape hide, and fresh input surviving an obsolete rejection after external Restore. Normal, settings and collapsed-error layouts had no horizontal/vertical overflow or captured script errors. Browser tests used a memory bridge, not native gamma.
- Actual 0.5.1 executable passed read-only native diagnostics/unarmed startup, memory watchdog timeout/EOF/parent-death and continuous lease expiry checks. Before/after gamma snapshots and startup registration were unchanged during these checks.
- Supplied ICO unchanged; all **7 PE icon entries** match. Build executable SHA-256: `f24ec50c45f5e031093e9436b4c0d4e731a30d38962017d9c129ebf60bac465c` (8,109,056 bytes).
- Installer SHA-256: `3ca223be1da678b9e354a9b40846e741e571ed5a33d7a480c94f4e62c7be8530` (1,841,121 bytes).

Evidence: `evidence/v051-final-r2/checks.json` and `packaged/` contain the final rebuilt executable checks; `evidence/v051-status-fix/` contains status/attention RED-GREEN tests. The 8 browser cases in `evidence/v051-final-r1/browser/checks.json` preceded the final status/attention fix and are not native acceptance. Final fixes fence stale status failures and publish authoritative tray attention on the main thread without holding worker operation locks. Focused independent correction review passed. Earlier failing queue regression is in `evidence/v051-queue-fix/`. Older build evidence remains historical and is not substituted for this run. Installed-app replacement requires normal Restore-and-quit of the older real app; never force-kill its recovery watchdog. The reported driver rejection at requested 52 is not a measured native cutoff and is not bypassed by this update.

---

# Historical installer packaging — 0.5.0

The first GitHub installer packages the existing 0.5.0 functionality; the older sections below describe historical **local portable builds**, not the current installation or publication state.

- Per-user NSIS x64 setup, stable `ScreenBrightController.exe`, original app identifier and supplied icon preserved. No runtime gamma/UI code changed.
- Locked Tauri CLI 2.12.1; **5 packaging contract tests**, **48 Rust tests**, **46 live UI assertions**, and **29 settings assertions** passed. Cargo fmt/clippy and npm audit passed (no npm vulnerabilities reported).
- The bundled executable passed read-only native startup/diagnostics, memory watchdog timeout/EOF/parent-death, continuous lease expiry and original PE-icon checks. Startup registration and the before/after native gamma snapshots were unchanged.
- The shared NSIS process check is replaced with a read-only Restart Manager query. A running executable or a query error aborts installation/removal; no forced process shutdown is requested. Uninstall only removes the exact installed executable's opt-in startup command, and preserves it on updates.
- `verify-release.py`, `verify-icon.py` and `verify-continuous.py` accept `--exe` and `--evidence-dir`; they no longer copy old binaries into `dist` or reuse historical browser test counts.
- Build executable SHA-256: `21302e552afb9cc0bcbb09df272ea02712ad083866bec485545a9db38af7211c` (8,177,152 bytes). Installer acceptance and publication are separately verified before the GitHub draft release is made public.
- Installed NSIS executable SHA-256: `c3771a22ef770070d73a3cddc251ced93093ca2e4d14de5e923821f1f679eead` (8,177,152 bytes). The entire payload was compared with the standalone binary after the single expected Tauri bundle marker substitution (`UNK` to `NSS`); no other byte difference exists. The pinned upstream bundler patches for NSIS, then restores the standalone output. Neither executable was modified by verification.
- The uploaded installer was downloaded again from GitHub and hash-checked before installation. Setup returned 0; Windows version/path/shortcuts were read back. Four actual installer refusal cases (install and uninstall, each with a dedicated mock app or memory watchdog) returned **1618**, left the test process alive, and preserved the installed binary and registration.
- The **installed** executable passed all three native read-only/memory-only verification scripts. Its actual main window rendered both displays at dimming 0, the installed recovery child was running, and native gamma readback remained identical across launch. Startup remained OFF. Evidence: `evidence/installed/`.
- Build host tools: Rust 1.98.1 / x86_64-pc-windows-msvc, Node 26.7.0, npm 11.19.0; installed MSVC toolset 14.44.35207. Fresh local evidence is under `evidence/package/`.

See [docs/RELEASING.md](docs/RELEASING.md) for the installation/refusal acceptance procedure. The installer is unsigned and updates are manual. Packaging checks do not establish actual dimming effects or login autostart behavior.

---

# Historical local build — Screen Bright Controller 0.5.0

## Windows login setting

Header ⚙ expands **Start with Windows**, plus “Launch in tray · no dimming applied at login.” Default OFF; only an explicit checkbox change registers the app. Actual state is read asynchronously on initialization, Settings open and window focus. Busy requests are serialized; errors display verified state or disable the checkbox if unknown.

Implementation: standard per-user `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` through `winreg`, a robust equivalent to the Tauri autostart plugin. Only the `ScreenBrightController` entry is touched; no administrator rights, tasks, shell commands, gamma registry settings or StartupApproved changes. The exact current executable is quoted (including spaced paths), followed only by `--autostart`. A stale moved-path entry is reported. Windows Startup Apps/Task Manager and organization policy may independently block execution; this checkbox reflects the app's Run entry, not those external switches.

Native async IPC runs off the UI thread under a mutex. Read before write, verify the complete command by readback, attempt prior-command rollback on set/readback failure or mismatch, and return a single final verified state. Failed rollback is explicit; unavailable readback never becomes optimistic success. Main is initially invisible to prevent login flash, then shown for normal/demo launches only. Exact `--autostart` launches the same fresh unarmed/read-only session into the tray; no dimming or independent brightness restoration. Autostart second-instance activation does not show main; normal activation still does. Existing watchdog/diagnostic arguments remain separate.

### Current automated evidence

- **48 Rust tests**, including **9 startup tests**; 0 failed.
- **46 live UI assertions** (production main/popup script with shared memory IPC), **29 settings UI assertions** (injected IPC, markup/wiring and browser-only memory bridge).
- **12 independent browser settings assertions** with trusted clicks and production HTML/CSS/JS at 740×358 content size: collapse/expand, OFF→ON→OFF, memory readback, unarmed gamma, compact settings fit/no horizontal overflow, Restore present and no footer. Expanded Settings scrolls with the display list. Browser results are not native WebView2/login acceptance.
- `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, JS syntax checks and release build: exit 0.
- Actual release `--autostart-check`: **OFF**, no error, **0 registration writes / 0 native display writes**. Independent before/after registration comparison unchanged.
- Release `--startup-check`: real watchdog unarmed. Actual memory-watchdog timeout/EOF/process-handle parent-death (pipe held open) checks passed. Existing app/watchdog untouched; only dedicated mock-test parents terminated.
- Actual memory continuous lease survived 18 seconds; independent preview expired at 15 seconds; lost renewal restored after 10 seconds; continuous EOF restored; invalid/late/unknown/duplicate requests rejected.
- Read-only native gamma before/after release verification identical; final after-all-checks comparison recorded separately. No development command requested real gamma SET or startup registration modification.
- Original ICO hash unchanged; **7 PE icon entries**, window PNG and tray RGBA match the supplied icon. No native icon assets changed.

### Observed RED → GREEN evidence

1. Existing-registration UI assertion failed `false !== true` → asynchronous actual read initializes checkbox.
2. Missing gear/change handlers → expansion, trusted serialized async toggles, verified error rollback and unknown-state disabling passed.
3. Missing Rust get/set APIs → injected read and explicit enable/disable readback tests passed, including quoted spaced paths and write-free default.
4. Set failure returned `None` instead of `Some(false)` → actual registration returned with error.
5. Initial-get/readback/mismatch tests failed (unexpected write, incorrect enabled state, missing rollback) → transactional toggle and verified prior-entry restoration passed.
6. Missing exact startup argument policy/hidden-window/native IPC/markup → route, read-only session and integration assertions passed.
7. Browser settings command rejected as unknown → local-memory-only settings support passed without overriding any native bridge.
8. Rollback final-read regression lacked the unavailable-readback error → one final read now determines both rollback error and checkbox state, avoiding an earlier stale read.

The renamed repository had obsolete absolute paths in cached Tauri release permission metadata. Initial release build failed; only Tauri/single-instance release dependency caches were cleaned and regenerated. Subsequent build and actual release checks passed. No source-path workaround or old-path directory created.

### v0.5 artifact

- `dist/ScreenBrightController-v0.5.exe`: **8,176,640 bytes**, SHA-256 `897b2b6def60bb05192df0f35cff7929ad707b06d6ca6857222bcd9ed7ba1da1`.
- Historical diagnostic CLI (removed): **665,088 bytes**, SHA-256 `b40762b151b16f742c9dfdc8e8705f1565eb4962002d8d142a34d6baa0180248`.
- Product/FileDescription **Screen Bright Controller**; ProductVersion **0.5.0**. Identifier unchanged. Old versioned app executable not replaced; no commit/push/install/signing/publication.

Ignored local evidence: `evidence/tests-v05.txt`, `ui-smoke-v05.txt`, `ui-settings-v05.txt`, `clippy-v05.txt`, `build-v05.txt`, `verification-v05.json`, `autostart-readonly-v05.json`, `continuous-v05.json`, `icon-v05.json`, `browser-settings-v05.json`, `diagnose-v05-before.json`, `diagnose-v05-final.json`, `safety-final-v05.json`.

### Remaining acceptance / preserved scope

Real registration toggles, real Windows sign-in, hidden tray/window behavior, second-instance native activation, spaced-path execution and external startup policy interactions remain manual: the user has **not enabled startup**. See [MANUAL_TESTS.md](MANUAL_TESTS.md). Memory/browser tests do not prove login execution.

Controls remain exact 0–90: pointer/number 1, slider arrows/wheel 5; compact main/popup and no focus-loss hiding preserved. No independent brightness restoration, range remapping, gamma registry bypass or high-dimming promises. The user's dim 54 screenshot failure and real high-dimming boundary remain **unverified and unrelated to autostart**. Screenshots cannot establish native gamma effect; existing HDR/ICC/Night Light/hotplug/driver/OS/both-process limitations remain.

---

# Historical v0.4 verification (previous release only)

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
- Executable mock tests verified timeout, EOF and process-handle parent death while the pipe remained open. Only dedicated memory-test parent processes were killed; existing controller/application/watchdog processes were untouched.
- Real elapsed-time memory-watchdog test renewed a bounded 10-second continuous lease every 2 seconds for **18 seconds**; a separate 15-second preview expired independently. Stopping renewal restored after 10 seconds. Late/invalid renewal, duplicate arm and unknown IDs were rejected. Continuous EOF restored immediately.
- Final native ramp snapshots before release verification and after all release/watchdog checks were identical. **Native gamma SET/restore calls by implementation and verification: 0.**
- One earlier read-only comparison failed because native ramp state changed during the session; no code attempted to correct it. A fresh complete verification run and its final after-all-checks comparison passed unchanged. Do not interpret the final stable interval as proof that unrelated applications never changed gamma.

## Compact browser UI

Actual production HTML/CSS/JS was rendered through a browser-only memory bridge that never overrides a native Tauri bridge. Final compact-layout evidence contains **12 passing assertions**: no footer/action region, accessible header Restore, conditionally hidden error banner, controls fit, typed 23 applies without a modal, slider arrow 23→28, shared popup state, browser blur visibility and header Restore. Earlier functional browser evidence contains **14 passing assertions**, including numeric arrow 23→24, pointer input and Home/End; it predates the final footer removal.

Final screenshots: main **740×358 content** (configured native outer window 740×390), popup **430×340**. Dark original horizontal design preserved; everyday labels are Display 1 / Display 2 in native enumeration order. Raw IDs remain internal / tooltip only. Header ↺ and native tray Restore / Restore and quit preserve recovery without a normal status footer.

Browser mouse-wheel CDP dispatch timed out once; custom wheel behavior passed the fake-DOM event tests, but actual browser wheel acceptance is not claimed. Browser interactions do not prove native WebView2/tray integration or visible native gamma effects.

## Artifacts and identity

- `dist/ScreenBrightController-v0.4.exe`: **8,046,592 bytes**, SHA-256 `29c1723729e51e806a347ac1d131a424dcb9514e9726cb9be0e0fb7bbd9e0928`.
- Historical diagnostic CLI (removed): **666,624 bytes**, SHA-256 `4caadde9da10bf659d7f0434dea3113752f730f1b64777cd770867e7f660fe16`.
- Product/FileDescription: **Screen Bright Controller**; ProductVersion: **0.4.0**.
- Supplied ICO SHA-256 unchanged: `1cbe4d747d8ef3e26840a6b700b48e83a3440fa333fbf6c994114acea72f6398`. All seven PE icon entries, window PNG and tray RGBA match the original supplied icon. No icon assets regenerated.
- Old versioned executable remains untouched. No commit, push, signing, installation, registry gamma-range change or publication performed.
- Verification scripts derive paths from their own repository location; the parent may rename the source folder after this task finishes.

Ignored local evidence: `evidence/tests-v04.txt`, `ui-smoke-v04.txt`, `clippy-v04.txt`, `build-v04.txt`, `verification-v04.json`, `continuous-v04.json`, `icon-v04.json`, `diagnose-v04-before.json`, `diagnose-v04-final.json`, `browser-v04-compact.json`, `v04-main.png`, `v04-tray.png`.

## Remaining native acceptance / operational constraints

See [MANUAL_TESTS.md](MANUAL_TESTS.md) for the short deliberate test checklist. Native live drag, wheel, WebView2 focus, popup persistence / placement and visual gamma acceptance have not been exercised by this implementation.

The user reported v0.3 dim 45 darkest and higher requests becoming ineffective / bright. Windows/driver gamma restrictions can silently ignore a ramp even when the API returns success. v0.3 also replaced its expected ramp before readback; heartbeat mismatch then restored originals. v0.4 retains a readback-confirmed prior ramp on ignored updates and displays failures, but does not bypass restrictions or guarantee dim 50–90. The exact native threshold remains unverified.

HDR / ICC / Night Light conflicts, hardware cursor differences, driver latency, hotplug / recycled display IDs, GPU/OS resets and both-process failure remain limitations. Readback/API success does not establish perceived visual effect; this is gamma scaling, not backlight brightness. Recovery is best effort if native calls block or the OS / both processes fail. No native failure injection, automatic boundary sweep or registry workaround was performed.
