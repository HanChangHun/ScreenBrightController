# Usage guide

[한국어](USAGE.ko.md) · [README](../README.md)

## Installation and updates

Download `ScreenBrightController_<version>_x64-setup.exe` from [GitHub Releases](https://github.com/HanChangHun/ScreenBrightController/releases/latest). Requires Windows x64 and Microsoft Edge WebView2 Runtime.

- The installer runs without administrator rights and installs for the current user.
- It creates Start menu/desktop shortcuts and an entry in Windows Installed apps.
- The default executable is `%LOCALAPPDATA%\Screen Bright Controller\ScreenBrightController.exe`.
- It downloads WebView2 only if the runtime is missing.
- The installer is not code-signed, so Windows may show an unknown-publisher or SmartScreen warning.
- Download only from this repository and compare the supplied `SHA256SUMS.txt` when needed.

Installation does not enable Windows startup or apply dimming. Launch from the Start menu, then left-click the tray icon to open the controls. There is no separate main window or Open app action. Normal launch, login launch and second-instance activation leave the popup hidden and do not apply dimming or restore brightness.

Updates are manual; download and run the newer installer. Before updating or uninstalling, use **Restore and quit** in the tray menu. The installer refuses to replace or remove a running executable and does not forcibly stop the app or its recovery watchdog.

Old portable copies in a development `dist` folder are not required by the installed app. Do not run them alongside it or run two gamma controllers together.

## Controls

Changes apply immediately, without an Apply button or confirmation dialog.

| Action | Result |
| --- | --- |
| Left-click the tray icon | Show or hide the popup |
| Drag a slider or edit a number | Adjust in steps of 1 |
| Use slider arrow keys or the mouse wheel | Adjust by ±5 |
| Press Home / End on a slider | Request 0 / 90 |
| Click − or press Escape | Hide the popup while keeping dimming active |
| Choose Restore original | Restore the saved original gamma |
| Right-click the tray icon | Open Restore and Restore and quit actions |
| Open ⚙ Settings | Show the optional Windows startup setting |

The popup stays open on focus loss. Closing or hiding it is not the same as quitting.

## Move and resize

Drag the title or empty header space to move the popup. Buttons and inputs do not initiate window dragging. Resize from any border or corner; the lower-right corner has a small grip.

- The initial size is 430×340 logical pixels, with a 430×260 minimum.
- Expanded settings, details or many displays use internal scrolling.
- Position and size survive hiding and reopening during the current running session only.
- Reopening does not reset the popup to its default size or tray position.
- Restarting opens a fresh popup near the tray; geometry is not saved between launches.
- Work-area and DPI changes keep the existing geometry within the available screen area.
- If a monitor becomes unavailable, the popup moves to the nearest remaining work area.
- Very small work areas take priority over the minimum size.

Moving or resizing does not apply dimming or change startup settings.

## Start with Windows

Open ⚙ Settings and toggle **Start with Windows**. It is off by default, needs no administrator rights and only registers when you explicitly change the setting. Login launches remain in the tray without opening the popup, applying dimming or restoring brightness.

The checkbox reads the actual current-user Windows Run registration. Changes are verified by readback. On error, the app attempts to restore the previous entry and shows the verified state; if the state is unknown, it disables the checkbox.

Keep the executable in a stable location. After moving or renaming it, enable the setting again to update the path. Windows Startup Apps, Task Manager or organization policy can separately block login launches; the checkbox reflects this app's Run entry and does not override those policies.

## Recovery and limitations

**0** means original gamma. **90** requests 10% of the original gamma values, not 10% backlight brightness. The app uses GPU gamma adjustment, not a screen overlay or DDC/CI.

If Windows or the driver rejects a request, or readback cannot confirm it, the app pauses dimming controls and discards queued requests. While paused, the numbers are requested values, not confirmed applied values.

1. Use **Restore** in the recovery notice.
2. After a successful restore, deliberately try a lower amount.
3. Expand **Details** to distinguish API acceptance from readback results if needed.

The app does not automatically retry, silently clamp the requested range or bypass driver restrictions. If restoration fails, controls remain paused and recovery stays available. A status-read failure also blocks new requests until status returns. If restoration fails when quitting, the app stays open so you can retry Restore.

HDR, ICC profiles, Night Light, driver behavior and GPU/OS resets can interfere with gamma adjustment. Hardware cursors may look different from the dimmed screen. Support across the full requested range depends on the system; API acceptance or matching readback alone does not establish the visible effect. Recovery is best effort if native calls block or the OS or both processes fail.

See [manual checks](../MANUAL_TESTS.md) for deliberate native acceptance.

## Build and verification

The [release guide](RELEASING.md) contains prerequisites, the full test commands, build outputs and publication steps. Python 3.11+ is required for verification. Source packages are `screen-bright-controller` and `screen-bright-controller-tray`; build the optional diagnostic target with `cargo build --release --locked --bin screen-bright-controller-cli`.

The executable verifiers accept `--exe "<installed executable>"` and `--evidence-dir "<output directory>"`. They do not recreate old `dist` executables; native diagnostics are read-only and watchdog failure tests use memory only. Binaries and local diagnostic evidence are not included in this source repository.
