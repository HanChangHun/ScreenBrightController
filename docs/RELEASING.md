# Windows release procedure

The distributable is a per-user NSIS installer. Keep `productName`, `identifier`, `mainBinaryName` and `publisher` stable so upgrades retain the same installation and WebView2 data. No automatic updater or signing key is configured; do not represent checksums as a code signature.

## Build and verify

From the repository root on Windows x64 with Rust/MSVC, Node/npm and Python 3:

```sh
npm --prefix app ci --ignore-scripts
python scripts/test_packaging.py
cargo test --workspace --locked
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
node scripts/ui-smoke.cjs
node scripts/ui-settings.cjs
npm --prefix app run build
python scripts/verify-release.py
python scripts/verify-icon.py
python scripts/verify-continuous.py
```

The CLI is pinned to 2.12.1 in `app/package.json` and `app/package-lock.json`. Review its generated NSIS `CheckIfAppIsRunning` call sites when upgrading the CLI: `windows/installer-hooks.nsh` replaces that shared macro to **refuse** a running executable instead of closing processes. Restart Manager errors are fail-closed. Both the app and its watchdog use the same executable, so either must be stopped normally before installation/removal. Never force-kill a real gamma process to build, install or delete old artifacts.

Outputs:

- `target/release/ScreenBrightController.exe`
- `target/release/bundle/nsis/Screen Bright Controller_<version>_x64-setup.exe`
- `evidence/package/`: fresh read-only/memory-only executable checks; no dependency on historical browser evidence.

For a version bump, align `Cargo.toml`, `app/src-tauri/Cargo.toml`, `app/src-tauri/tauri.conf.json` and `app/package.json`, and refresh their lockfiles using Cargo/npm. Do not alter the supplied icon or gamma behavior as part of packaging.

## Install acceptance

1. Confirm no real app/watchdog is running. Use tray **Restore and quit** if necessary.
2. Install the setup executable (interactive or `/S`). Do not pass `/R` during acceptance; launch deliberately after inspection. Default path: `%LOCALAPPDATA%\Screen Bright Controller`.
3. Read back the Installed apps entry, version, executable hash, uninstaller and shortcut targets. Verify the installed executable hash equals the bundled build output.
4. Run the three verification scripts with `--exe "<installed path>/ScreenBrightController.exe" --evidence-dir evidence/installed`.
5. For refusal tests only, launch the installed executable with **`--mock-parent-wait`**, never a live dimming session. While that dedicated memory-only test process is running, run setup `/S` and uninstaller `/S _?=<install directory>`; expect nonzero exit, the mock process still alive, and unchanged installed files/registration. Stop only the dedicated mock parent afterward.
6. Launch the installed app normally. Verify its own main window and recovery child; compare read-only native ramps before/after. Do not move any dimming controls. Windows startup remains an explicit user opt-in.

Uninstall cleanup only removes the app's `ScreenBrightController` Run value when it exactly matches this installed executable's quoted `--autostart` command, and preserves it during updates. Real startup toggles and actual login acceptance are separate user-authorized checks.

## Publish

1. Review and commit the tested source; push and verify the remote commit.
2. Copy the installer to `dist/ScreenBrightController_<version>_x64-setup.exe`. Compute `SHA256SUMS.txt` from its actual bytes.
3. Create a **draft** GitHub release for the exact source commit, with the installer and checksum file. Never replace an already published version silently.
4. Read back release assets, download them again, and compare hashes. Complete install acceptance before publishing the draft as the latest normal release.
5. Link the release and note that updates are manual and the executable is not code-signed.
6. Once the separate installed app is verified, delete only the enumerated obsolete `dist` executables. Keep the current installer and checksum file; do not delete source, icons, app data, or active executables.

Native dimming limits, HDR/ICC/Night Light conflicts, hotplug and recovery limitations remain as documented in `MANUAL_TESTS.md` and `VALIDATION.md`. Passing installer checks does not establish perceived gamma effects or Windows login behavior.
