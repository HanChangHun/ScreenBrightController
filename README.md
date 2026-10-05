# Screen Bright Controller

A dark, tray-based Windows utility for dimming individual displays or all displays together. Built with Rust and Tauri 2; uses GPU gamma adjustment, not a screen overlay or DDC/CI.

[한국어](README.ko.md)

## Features

- Per-display or linked adjustment: **0–90 dimming, every integer**.
- Live dimming from slider, keyboard, wheel or number controls.
- Shared main/tray controls with clean Display 1, Display 2 labels.
- Original gamma restoration on Quit, with an independent recovery watchdog.
- No automatic dimming at startup.

## Use

Changes apply immediately—no consent checkbox, Apply button or modal. Drag and number controls use steps of 1; slider arrow keys and wheel use ±5, Home/End use 0/90. Closing the main window keeps dimming in the tray; **Restore original** resets it. The popup stays open on focus loss; X, Escape, tray toggle or Open app hides it.

**0** means original gamma; **90** requests 10% of the original gamma values—not 10% backlight brightness. Requires Windows x64 and Microsoft Edge WebView2 Runtime.

## Build

Requires Rust/MSVC and the Tauri Windows build prerequisites. Node is only needed for UI tests.

```sh
cargo build --release --workspace
cargo test --workspace
node scripts/ui-smoke.cjs
```

Run `target/release/gamma-dimmer-app.exe`; it can be renamed to `ScreenBrightController.exe`. Prebuilt local artifact: `dist/ScreenBrightController-v0.4.exe`.

See [VALIDATION.md](VALIDATION.md) for testing scope and manual checks. Binaries and local diagnostic evidence are not included in this source repository.
