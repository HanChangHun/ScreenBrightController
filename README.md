# Screen Bright Controller

Dim individual monitors or all displays together from the Windows system tray.

[Download](https://github.com/HanChangHun/ScreenBrightController/releases/latest) · [한국어](README.ko.md)

## Features

- Adjust monitors independently or together, with dimming values from 0 to 90.
- Apply slider, keyboard, wheel or number input immediately.
- Move and resize a compact, dark tray popup.
- Restore original gamma on exit, with an independent recovery watchdog.
- Optionally start with Windows; disabled by default.

## Get started

1. Download and install `ScreenBrightController_<version>_x64-setup.exe` from the [latest release](https://github.com/HanChangHun/ScreenBrightController/releases/latest).
2. Launch from the Start menu, then left-click the tray icon to open the controls.
3. Adjust the sliders. Use **Restore original** to reset, or right-click the tray icon and choose **Restore and quit** to exit.

Requires Windows x64. The installer needs no administrator rights and installs WebView2 if missing, using Microsoft's bootstrapper bundled in the installer (internet connection required). Launching does not apply dimming; hiding the popup keeps active dimming running.

## Important notes

- This adjusts GPU gamma, not monitor backlight brightness.
- Driver restrictions and HDR/ICC/Night Light can affect results; the full 0–90 range is not guaranteed to work on every system.
- Known limits: a hardware cursor may stay bright, drivers may apply changes late, and hotplug, recycled display IDs or GPU/OS resets while dimmed are unsupported. An accepted API call or matching readback does not prove the visible effect, and recovery is best effort if native calls hang or both the app and its watchdog stop.
- If controls pause after an error, use **Restore** before trying a lower value.
- If a display's saved gamma is already far below normal at launch (for example, both the app and its watchdog stopped while dimmed), the popup warns and offers **Reset to default ramp**. It never resets on its own, because calibrated ramps are legitimately non-linear.
- Updates are manual. Use **Restore and quit** before updating or uninstalling.
- The installer is unsigned and may trigger SmartScreen; see [installation notes](docs/USAGE.md#installation-and-updates).

## Build

Built with Rust and Tauri 2. On Windows with Rust/MSVC, Node/npm and the Tauri Windows build prerequisites:

```sh
npm --prefix app ci --ignore-scripts
npm --prefix app run build
```

See the [release guide](docs/RELEASING.md) for tests, build outputs and packaging.

## Documentation

[Usage guide](docs/USAGE.md) · [Manual checks](MANUAL_TESTS.md) · [Issues](https://github.com/HanChangHun/ScreenBrightController/issues)
