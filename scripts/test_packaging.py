"""Regression checks for the Windows installer contract (no native writes)."""
import json
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]


class PackagingTests(unittest.TestCase):
    def test_current_user_nsis_installer_preserves_app_identity(self):
        config = json.loads((ROOT / "app/src-tauri/tauri.conf.json").read_text(encoding="utf-8"))
        bundle = config["bundle"]
        self.assertTrue(bundle["active"], "Installer bundling must be enabled")
        self.assertEqual(bundle["targets"], ["nsis"])
        self.assertEqual(config["mainBinaryName"], "ScreenBrightController")
        self.assertEqual(config["identifier"], "io.github.screenbrightcontroller")
        self.assertEqual(bundle["windows"]["nsis"]["installMode"], "currentUser")
        self.assertFalse(bundle["windows"]["allowDowngrades"])
        self.assertFalse(bundle["createUpdaterArtifacts"])
        self.assertEqual(bundle["icon"], ["icons/icon.png", "icons/icon.ico"])
        self.assertEqual(bundle["windows"]["webviewInstallMode"], {
            "type": "downloadBootstrapper", "silent": True,
        })

    def test_installer_refuses_running_apps_without_terminating_them(self):
        hooks = ROOT / "app/src-tauri/windows/installer-hooks.nsh"
        self.assertTrue(hooks.is_file(), "Non-terminating installer hooks are required")
        source = hooks.read_text(encoding="utf-8")
        self.assertIn("!macroundef CheckIfAppIsRunning", source)
        self.assertIn("!macro CheckIfAppIsRunning executablePath productName", source)
        self.assertIn("RmGetList", source)
        self.assertIn("SetErrorLevel 1618", source)
        self.assertIn("SetErrorLevel 1603", source)
        self.assertIn("Restore and quit", source)
        for forbidden in ("RmShutdown", "RmForceShutdown", "TerminateProcess", "taskkill", "_KillProcess"):
            self.assertNotIn(forbidden, source)
        config = json.loads((ROOT / "app/src-tauri/tauri.conf.json").read_text(encoding="utf-8"))
        self.assertEqual(config["bundle"]["windows"]["nsis"]["installerHooks"],
                         "windows/installer-hooks.nsh")

    def test_uninstall_only_removes_its_exact_opt_in_startup_command(self):
        source = (ROOT / "app/src-tauri/windows/installer-hooks.nsh").read_text(encoding="utf-8")
        self.assertIn("!macro NSIS_HOOK_POSTUNINSTALL", source)
        cleanup = source.split("!macro NSIS_HOOK_POSTUNINSTALL", 1)[1]
        self.assertIn("${If} $UpdateMode <> 1", cleanup)
        self.assertIn('ReadRegStr $0 HKCU', cleanup)
        self.assertIn('"ScreenBrightController"', cleanup)
        self.assertIn('${If} $0 ==', cleanup)
        self.assertIn('--autostart', cleanup)
        self.assertIn('DeleteRegValue HKCU', cleanup)
        self.assertNotIn('DeleteRegKey', cleanup)
        self.assertNotIn('WriteReg', source)

    def test_packaging_toolchain_is_pinned_and_uses_cargo_lock(self):
        manifest_path = ROOT / "app/package.json"
        self.assertTrue(manifest_path.is_file(), "A reproducible packaging command is required")
        manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
        self.assertEqual(manifest["devDependencies"]["@tauri-apps/cli"], "2.12.1")
        self.assertEqual(manifest["scripts"]["build"], "tauri build --bundles nsis -- --locked")
        self.assertTrue(manifest["private"])
        config = json.loads((ROOT / "app/src-tauri/tauri.conf.json").read_text(encoding="utf-8"))
        self.assertEqual(manifest["version"], config["version"])

    def test_verifiers_use_explicit_or_current_binary_not_legacy_dist(self):
        for name in ("verify-release.py", "verify-icon.py", "verify-continuous.py"):
            with self.subTest(script=name):
                source = (ROOT / "scripts" / name).read_text(encoding="utf-8")
                self.assertIn('"--exe"', source)
                self.assertIn('target/release/ScreenBrightController.exe', source)
                self.assertNotIn('ScreenBrightController-v0.5.exe', source)
                self.assertNotIn('shutil.copy2', source)
                self.assertNotIn('browser-settings-v05.json', source)


if __name__ == "__main__":
    unittest.main()
