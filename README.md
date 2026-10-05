# Screen Bright Controller

A dark, tray-based Windows utility for dimming individual displays or all displays together. Built with Rust and Tauri 2; uses GPU gamma adjustment, not a screen overlay or DDC/CI.

[한국어](README.ko.md)

## Features

- Per-display or linked adjustment: **0–90 dimming, in steps of 5**.
- Continuous dimming and a 15-second preview.
- Shared controls in the main window and tray popup.
- Original gamma restoration on Quit, with an independent recovery watchdog.
- No automatic dimming at startup.

## Use

Set the dimming amount, confirm consent, then choose **Apply continuous** or **Preview 15s**. Closing the main window keeps continuous dimming active in the tray; **Restore original** resets it.

**0** means original gamma; **90** requests 10% of the original gamma values—not 10% backlight brightness. Requires Windows x64 and Microsoft Edge WebView2 Runtime.

## Build

Requires Rust/MSVC and the Tauri Windows build prerequisites. Node is only needed for UI tests.

```sh
cargo build --release --workspace
cargo test --workspace
node scripts/ui-smoke.cjs
```

Run `target/release/gamma-dimmer-app.exe`; it can be renamed to `ScreenBrightController.exe`. Prebuilt local artifact: `dist/ScreenBrightController-v0.3.exe`.

## Limitations

Use with HDR off and avoid other dimmers or color-adjustment tools. Strong dimming may be rejected by the driver. Restore and quit before changing monitor connections. Recovery is best-effort under driver or system failures.

See [VALIDATION.md](VALIDATION.md) for testing scope and manual checks. Binaries and local diagnostic evidence are not included in this source repository.
