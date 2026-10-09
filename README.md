# Screen Bright Controller

A dark, tray-only Windows utility for dimming individual displays or all displays together. Built with Rust and Tauri 2; uses GPU gamma adjustment, not a screen overlay or DDC/CI.

[한국어](README.ko.md)

## Install (Windows x64)

Download the latest **`ScreenBrightController_<version>_x64-setup.exe`** from [GitHub Releases](https://github.com/HanChangHun/ScreenBrightController/releases/latest) and run it. The current-user installer needs no administrator rights, creates Start menu/desktop shortcuts, and registers the app in Windows Installed apps.

Default location: `%LOCALAPPDATA%\Screen Bright Controller\ScreenBrightController.exe`. Microsoft Edge WebView2 is downloaded only if missing. The installer is not code-signed; Windows may show an unknown-publisher/SmartScreen warning. Download only from this repository and compare the supplied `SHA256SUMS.txt` when needed.

Installation does **not** enable Windows startup or apply dimming. Launch from the Start menu, then **left-click its tray icon** to open the controls. There is no separate main window or Open app action. Before updating or uninstalling, use **Restore and quit** in the tray menu. The installer refuses to replace/remove an executable that is still running; it does not forcibly stop the app or its recovery watchdog.

Updates are **manual**: download and run the newer installer. An automatic updater is not included. Old portable copies in a development `dist` folder are not required by the installed app and must not run alongside it.

## Features

- Per-display or linked adjustment: **0–90 dimming, every integer**.
- Live dimming from slider, keyboard, wheel or number controls.
- One compact tray popup with clean Display 1, Display 2 labels.
- Original gamma restoration on Quit, with an independent recovery watchdog.
- Optional **Start with Windows** in header ⚙ Settings, default off.
- Normal launch, login launch and second-instance activation stay in the tray, with no automatic window opening, dimming or brightness restoration.

## Use

Left-click the tray icon to show/hide the popup. Changes apply immediately—no consent checkbox, Apply button or modal. Drag and number controls use steps of 1; slider arrow keys and wheel use ±5, Home/End use 0/90. X or Escape hides the popup without ending continuous dimming; **Restore original** resets it. The popup stays open on focus loss. Right-click the tray icon for Restore or **Restore and quit**. The gear inside the popup contains the optional Windows startup setting.

If Windows/the driver rejects a request or readback cannot confirm it, the tray popup pauses dimming controls and discards queued requests. The numbers are **requested values, not confirmed applied values** while paused. A compact notice provides **Restore**; after a successful restore, deliberately try a lower amount. Nothing is automatically retried or forced past the driver limit. **Details** keeps API acceptance and readback results separate. Restoration failure keeps controls paused and recovery available; a status-read failure also prevents new requests until status returns.

**0** means original gamma; **90** requests 10% of the original gamma values—not 10% backlight brightness. Requires Windows x64 and Microsoft Edge WebView2 Runtime.

Settings reads the actual per-user Windows Run registration. Explicit toggles are verified by readback; errors revert the entry when possible and display the verified state, or disable the checkbox if state is unknown. No administrator rights or automatic registration. Keep the executable in a stable location; after moving/renaming it, enable again to update the path. Windows Startup Apps/Task Manager or organization policy can separately block login launches; the checkbox reflects this app’s Run entry, not an override of those policies.

## Build

Requires Rust/MSVC and the Tauri Windows build prerequisites. Node/npm are used for the pinned Tauri packaging CLI and UI tests; Python 3 is used for verification.

```sh
npm --prefix app ci --ignore-scripts
npm --prefix app run build
cargo test --workspace --locked
node scripts/ui-smoke.cjs
node scripts/ui-settings.cjs
node scripts/ui-recovery.cjs
node scripts/ui-copy.cjs
python scripts/test_packaging.py
python scripts/test_tray_only.py
python scripts/test_branding.py
python scripts/verify-release.py
python scripts/verify-icon.py
python scripts/verify-continuous.py
```

Build outputs: `target/release/ScreenBrightController.exe` and `target/release/bundle/nsis/Screen Bright Controller_<version>_x64-setup.exe`. Verification does not recreate old `dist` executables. Each verifier accepts `--exe "<installed executable>"` and `--evidence-dir "<output directory>"`; native diagnostics are read-only and watchdog failure tests use memory only.

Source packages are `screen-bright-controller` (core) and `screen-bright-controller-tray` (Tauri host). The optional diagnostic binary target is `screen-bright-controller-cli`; build it with `cargo build --release --locked --bin screen-bright-controller-cli`. Icon reference assets use the `screen-bright-controller` filename prefix. The public executable, installation identifier and supplied ICO remain unchanged. Gamma API names are technical terms, not old product branding.

See [docs/RELEASING.md](docs/RELEASING.md) for repeatable packaging and publication.

See [VALIDATION.md](VALIDATION.md) for testing scope and manual checks. Binaries and local diagnostic evidence are not included in this source repository.
