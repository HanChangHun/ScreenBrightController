# Verified release — Screen Bright Controller 0.3.0

## Build / automated tests

- `cargo build --release --workspace`: exit 0; main/popup HTML/CSS/JS embedded with custom-protocol.
- `cargo test --workspace`: **33 Rust tests passed, 0 failed**, including independent process-handle death detection while the watchdog pipe remains open.
- `cargo clippy --workspace --all-targets -- -D warnings`: exit 0, no warnings.
- `cargo fmt --all -- --check`: exit 0.
- `node --check app/ui/app.js`, `app/ui/demo.js`, `scripts/ui-smoke.cjs`: exit 0.
- Headless main + popup real UI script: **41 assertions**; shared backend, horizontal 0–90 step-5 inputs, read-only nonzero editing, exact-step validation, consent, explicit continuous Apply, restore, mismatch reporting, popup Escape / Open.
- New RED/GREEN work observed missing continuous APIs and UI Apply listener, and reproduced behavioral failures for renewing an expired failed-restoration lease and unknown-ID Restore. Tests pass after implementation. Historical v0.2 RED/GREEN logs are separate from current verification.

## Actual release executable / independent watchdog

`python scripts/verify-release.py`, `verify-icon.py`, `verify-continuous.py` exited 0.

- `--diagnose`: native Win32 read-only diagnostics, two readable active NVIDIA adapter display paths.
- `--startup-check`: actual independent native watchdog captured originals but remained unarmed; exited with no gamma restoration writes.
- `--self-test` and CLI `--mock`: injected memory controller and independent mock watchdog; timeout and EOF restoration passed.
- Continuous mock watchdog maintained a 10-second lease renewed every 2 seconds for **18 seconds**, beyond the fixed 15-second preview limit. A separate preview target expired and restored independently.
- Stopping renewal restored the persistent target after **10 seconds**. Late renewal, preview renewal, duplicate arm, unknown IDs and invalid lease were rejected.
- Continuous-mode EOF restored immediately. The actual release executable's continuous parent-death test kept stdin open, killed only the dedicated mock parent helper, and restored through process-handle detection.
- Controller heartbeat test proves no repeated gamma SET, safely updates active targets from the original snapshot, restores zero/excluded targets, detects external color changes and disarms instead of fighting them.
- Main-close backend test preserves continuous operation. Restore resets shared values; existing restoration-failure tests retain attempted targets for retry. Tauri Quit checks restoration before exit and cancels exit on failure.
- Native gamma snapshots read before and after **all** readonly/mock checks were identical.
- **Native gamma SET/restore calls during development and verification: 0.** Existing native application/watchdog processes were left untouched.

## Browser rendering

Actual production HTML/CSS/JS rendered with a clearly isolated browser-only memory bridge. **24 browser assertions** passed: main keyboard ArrowRight 0→5, exact range attributes, no tabs, consent lock/cancel, continuous Apply/update, simulated passage beyond preview timeout, reset; tray fit, per-display operation, invalid step, zero/exclusion restoration and preview expiry. Browser time simulation is not native lease verification; the independent release-watchdog test used real elapsed time.

Primary surfaces inspected visually at **740×610 main** and **430×540 popup**. Horizontal controls, Apply/Restore and status are visible; secondary diagnostics may scroll. Long display names have tooltips; native labels show distinct actual device IDs. Focus outlines and high-contrast text are implemented. Palette checks: body text 14+:1, muted text 7+:1, primary button text 8+:1. Composition audit: **Operate** surface, intentional Segoe UI for Windows utility, charcoal + restrained teal; no tabs, vertical panel composition, hero, generic feature tiles, gradients or decorative metrics.

Local evidence paths (ignored from source publication):
- `evidence/v03-main.png`
- `evidence/v03-tray.png`
- `evidence/browser-v03.json`
- `evidence/verification-v03.json`, `continuous-v03.json`, `icon-v03.json`
- `evidence/tests-v03.txt`, `clippy-v03.txt`, `build-v03.txt`, `ui-smoke-v03.txt`

## Local artifacts

- `dist/ScreenBrightController-v0.3.exe`
  - 8,043,520 bytes
  - SHA-256 `e82fddd250bd9d499b769d7daf7c5598da3c1338e4eb215740c0c6f604719f24`
- `dist/gamma-cli.exe`
  - 667,648 bytes
  - SHA-256 `8554ff30275c641aa0d938fa42e645f381dc879c8f2cfd1ab4eca52727597b96`

The versioned application filename leaves old running binaries intact. Product metadata is exactly **Screen Bright Controller / 0.3.0**. User-supplied ICO unchanged: SHA-256 `1cbe4d747d8ef3e26840a6b700b48e83a3440fa333fbf6c994114acea72f6398`. All seven PE icon entries, window PNG and tray RGBA match its original payloads. No assets were deleted or regenerated.

This is a source-only public repository: binaries, build outputs, evidence, snapshots, environment/private state, generated schemas and backup bundles are excluded. No commit, remote rewrite, push, installer, signing or publication performed by this implementation task.

## Manual acceptance remaining — not claimed

Native WebView2/tray interaction and actual native gamma dimming were **not** exercised. Browser/fake-DOM tests do not prove desktop integration. Before using native mode, safely Restore/Quit any older dimmer, disable HDR / other color tools and test a mild amount deliberately.

Still verify manually:
1. Main/tray shared values, native keyboard focus, confirmation dialog and popup blur/reopen behavior.
2. Actual work-area placement on negative-origin / mixed-DPI / taskbar configurations. Geometry is unit-tested; desktop placement is not.
3. Deliberate persistent Apply visibly dims the intended screens, continues past 15 seconds with main hidden, safely updates the same target, and restores on zero / exclusion / Restore / Quit.
4. Driver/API/readback results and perceived visible effects, hardware cursor and topmost/fullscreen behavior. No guarantee for high dimming or HDR / ICC conflicts.
5. Restoration failure cancels native Quit and presents an actionable warning. Code paths and memory failures are tested, real driver failure is not.

Do not test crash recovery by killing both processes. Native failure injection / killing an active parent was not performed; independent memory-only recovery tests cover parent death, EOF and lease loss. Hotplug / recycled display IDs, blocked driver calls, OS/GPU reset and both-process failure remain best-effort limitations documented in README.
