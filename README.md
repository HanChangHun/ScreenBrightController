# Screen Bright Controller

A dark, tray-based Windows utility for dimming individual displays or all displays together. Built with Rust and Tauri 2; uses GPU gamma adjustment, not a screen overlay or DDC/CI.

[한국어](README.ko.md)

## Features

- Per-display or linked adjustment: **0–90 dimming, every integer**.
- Live dimming from slider, keyboard, wheel or number controls.
- Shared main/tray controls with clean Display 1, Display 2 labels.
- Original gamma restoration on Quit, with an independent recovery watchdog.
- Optional **Start with Windows** in header ⚙ Settings, default off.
- Login launch is tray-only, with no automatic dimming or brightness restoration.

## Use

Changes apply immediately—no consent checkbox, Apply button or modal. Drag and number controls use steps of 1; slider arrow keys and wheel use ±5, Home/End use 0/90. Closing the main window keeps dimming in the tray; **Restore original** resets it. The popup stays open on focus loss; X, Escape, tray toggle or Open app hides it.

**0** means original gamma; **90** requests 10% of the original gamma values—not 10% backlight brightness. Requires Windows x64 and Microsoft Edge WebView2 Runtime.

Settings reads the actual per-user Windows Run registration. Explicit toggles are verified by readback; errors revert the entry when possible and display the verified state, or disable the checkbox if state is unknown. No administrator rights or automatic registration. Keep the executable in a stable location; after moving/renaming it, enable again to update the path. Windows Startup Apps/Task Manager or organization policy can separately block login launches; the checkbox reflects this app’s Run entry, not an override of those policies.

## Build

Requires Rust/MSVC and the Tauri Windows build prerequisites. Node is only needed for UI tests.

```sh
cargo build --release --workspace
cargo test --workspace
node scripts/ui-smoke.cjs
node scripts/ui-settings.cjs
```

Run `target/release/gamma-dimmer-app.exe`; it can be renamed to `ScreenBrightController.exe`. Prebuilt local artifact: `dist/ScreenBrightController-v0.5.exe`.

See [VALIDATION.md](VALIDATION.md) for testing scope and manual checks. Binaries and local diagnostic evidence are not included in this source repository.
