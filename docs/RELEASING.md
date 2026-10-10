# Windows release procedure

The distributable is a per-user NSIS installer. Keep `productName`, `identifier`, `mainBinaryName` and `publisher` stable so upgrades retain the same installation and WebView2 data. No automatic updater or signing key is configured; do not represent checksums as a code signature.

## Build and verify

From the repository root on Windows x64 with Rust/MSVC, Node/npm and Python 3.11+:

```sh
npm --prefix app ci --ignore-scripts
python scripts/test_packaging.py
uv run --with pillow python scripts/test_icon_assets.py
cargo test --workspace --locked
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
node scripts/ui-live.cjs
node scripts/ui-settings.cjs
node scripts/ui-recovery.cjs
node scripts/ui-geometry.cjs
npm --prefix app run build
python scripts/verify-release.py
python scripts/verify-icon.py
python scripts/verify-continuous.py
```

This is the canonical check list. `cargo test` and the verify scripts read real gamma ramps but never write them.

The CLI is pinned to 2.12.1 in `app/package.json` and `app/package-lock.json`. Review its generated NSIS `CheckIfAppIsRunning` call sites when upgrading the CLI: `windows/installer-hooks.nsh` replaces that shared macro to **refuse** a running executable instead of closing processes. Restart Manager errors are fail-closed. Both the app and its watchdog use the same executable, so either must be stopped normally before installation/removal. Never force-kill a real gamma process to build, install or delete old artifacts.

Outputs:

- `target/release/ScreenBrightController.exe`
- `target/release/bundle/nsis/Screen Bright Controller_<version>_x64-setup.exe`
- `evidence/package/`: fresh read-only/memory-only executable checks; no dependency on historical browser evidence.

For a version bump, align `Cargo.toml`, `app/src-tauri/Cargo.toml`, `app/src-tauri/tauri.conf.json` and `app/package.json`, and refresh their lockfiles using Cargo/npm. Do not alter the supplied icon or gamma behavior as part of packaging.

The workspace packages are `screen-bright-controller` and `screen-bright-controller-tray`; the optional diagnostic target is `screen-bright-controller-cli`. The internal watchdog parent variable is `SCREEN_BRIGHT_CONTROLLER_WATCHDOG_PARENT`, and native/frontend error events use `display-error`. Keep both ends and the memory verification fixtures in sync. A source-package rename must not change the public installation identity. Do not run the icon generator over the supplied ICO.

## Install acceptance

1. Confirm no real app/watchdog is running. Use tray **Restore and quit** if necessary.
2. Install the setup executable (interactive or `/S`). Do not pass `/R` during acceptance; launch deliberately after inspection. Default path: `%LOCALAPPDATA%\Screen Bright Controller`.
3. Read back the Installed apps entry, version, executable hash, uninstaller and shortcut targets. Verify the installed executable against the **NSIS payload**, not blindly against the standalone build hash. Tauri CLI 2.12.1 changes the single `__TAURI_BUNDLE_TYPE_VAR_UNK` marker to `__TAURI_BUNDLE_TYPE_VAR_NSS` while bundling, then restores the standalone binary. For this unsigned release, require the marker to be unique and compare the installed bytes with exactly that one replacement (or independently extract the NSIS payload); no other byte differences are allowed. Do not modify the installed or standalone executable to make a checksum match.
4. Run the three verification scripts with `--exe "<installed path>/ScreenBrightController.exe" --evidence-dir evidence/installed`.
5. For refusal tests only, launch the installed executable with **`--mock-parent-wait`**, never a live dimming session. While that dedicated memory-only test process is running, run setup `/S` and uninstaller `/S _?=<install directory>`; expect exit 1618, the mock process still alive, and unchanged installed files/registration. Stop only the dedicated mock parent afterward. Repeat both operations with an unarmed `--watchdog-mock` fixture, then close its input pipe normally. The final `_?=` path must be unquoted according to NSIS command-line syntax; do not invoke it through a shell.
6. Launch the installed app normally. Verify only the tray icon and recovery child start: the sole popup is hidden and no main window exists. Left-click the tray to open controls and inspect the popup, then hide it with −/Escape. A normal second instance must not create/show another window. Compare read-only native ramps before/after; do not move dimming controls. Windows startup remains an explicit user opt-in in the popup's gear settings.
7. Perform the popup geometry checklist in `MANUAL_TESTS.md`: trusted header-only drag, every resize edge/corner, minimum/scrolling, retained session geometry across hide/reopen, mixed-DPI/work-area recovery, and unchanged read-only gamma/startup state. Browser memory gesture traces and pure geometry tests do not establish native WebView2/Win32 mouse behavior. Do not install or launch a new real controller over a running older version to perform this check.

The installer hooks never touch the per-user `ScreenBrightController` Run value, so manual upgrades keep the startup opt-in and uninstalling leaves it in place (users turn it off in the popup settings first). Uninstallers from 0.5.x still contain the old cleanup, so an interactive "uninstall before installing" upgrade from 0.5.x can drop the value once; say so in release notes. Real startup toggles and actual login acceptance are separate user-authorized checks.

## Publish

1. Review and commit the tested source; push and verify the remote commit.
2. Copy the installer to `dist/ScreenBrightController_<version>_x64-setup.exe`. Compute `SHA256SUMS.txt` from its actual bytes.
3. Create a **draft** GitHub release for the exact source commit, with the installer and checksum file. Never replace an already published version silently.
4. Read back release assets, download them again, and compare hashes. Complete install acceptance before publishing the draft as the latest normal release.
5. Link the release and note that updates are manual and the executable is not code-signed.
6. Once the separate installed app is verified, delete only the enumerated obsolete `dist` executables. Keep the current installer and checksum file; do not delete source, icons, app data, or active executables.

Native dimming limits, HDR/ICC/Night Light conflicts, hotplug and recovery limitations remain as documented in the README and `MANUAL_TESTS.md`. Passing installer checks does not establish perceived gamma effects or Windows login behavior.
