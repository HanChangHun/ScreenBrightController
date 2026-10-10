"""Installer and window configuration contract (reads config files only; no native writes)."""
import json
from pathlib import Path
import tomllib
import unittest

ROOT = Path(__file__).resolve().parents[1]


def load_json(path):
    return json.loads((ROOT / path).read_text(encoding="utf-8"))


CONFIG = load_json("app/src-tauri/tauri.conf.json")


class PackagingTests(unittest.TestCase):
    def test_current_user_nsis_installer_preserves_app_identity(self):
        bundle = CONFIG["bundle"]
        self.assertTrue(bundle["active"], "Installer bundling must be enabled")
        self.assertEqual(bundle["targets"], ["nsis"])
        self.assertEqual(CONFIG["mainBinaryName"], "ScreenBrightController")
        self.assertEqual(CONFIG["identifier"], "io.github.screenbrightcontroller")
        self.assertEqual(CONFIG["productName"], "Screen Bright Controller")
        self.assertEqual(bundle["windows"]["nsis"]["installMode"], "currentUser")
        self.assertEqual(bundle["windows"]["nsis"]["installerHooks"], "windows/installer-hooks.nsh")
        self.assertFalse(bundle["windows"]["allowDowngrades"])
        self.assertFalse(bundle["createUpdaterArtifacts"])
        self.assertEqual(bundle["icon"], ["icons/icon.png", "icons/icon.ico"])
        self.assertEqual(bundle["windows"]["webviewInstallMode"], {
            "type": "embedBootstrapper", "silent": True,
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
        # A gamma recovery watchdog must never be stopped by the installer.
        for forbidden in ("RmShutdown", "RmForceShutdown", "TerminateProcess", "taskkill", "_KillProcess"):
            self.assertNotIn(forbidden, source)
        # Upgrades must keep the user's startup opt-in.
        self.assertNotIn(r"CurrentVersion\Run", source)

    def test_only_a_hidden_tray_popup_with_local_window_permissions(self):
        self.assertEqual([w["label"] for w in CONFIG["app"]["windows"]], ["popup"])
        popup = CONFIG["app"]["windows"][0]
        self.assertFalse(popup["visible"])
        self.assertTrue(popup["skipTaskbar"])
        self.assertFalse(popup["decorations"])
        self.assertTrue(popup["resizable"])
        self.assertEqual(popup["url"], "index.html")
        self.assertEqual((popup["minWidth"], popup["minHeight"]), (430, 260))
        capability = load_json("app/src-tauri/capabilities/main.json")
        self.assertEqual(capability["windows"], ["popup"])
        self.assertNotIn("remote", capability)
        self.assertEqual(set(capability["permissions"]), {
            "core:event:allow-listen", "core:event:allow-unlisten",
            "core:window:allow-start-dragging", "core:window:allow-start-resize-dragging",
        })

    def test_packaging_toolchain_is_pinned_and_uses_cargo_lock(self):
        manifest = load_json("app/package.json")
        self.assertEqual(manifest["devDependencies"]["@tauri-apps/cli"], "2.12.1")
        self.assertEqual(manifest["scripts"]["build"], "tauri build --bundles nsis -- --locked")
        self.assertTrue(manifest["private"])

    def test_versions_match_across_manifests_and_lockfiles(self):
        version = CONFIG["version"]
        for file in ("Cargo.toml", "app/src-tauri/Cargo.toml"):
            self.assertEqual(tomllib.loads((ROOT / file).read_text())["package"]["version"], version, file)
        lock = tomllib.loads((ROOT / "Cargo.lock").read_text())
        local = [p["version"] for p in lock["package"]
                 if p["name"] in ("screen-bright-controller", "screen-bright-controller-tray")]
        self.assertEqual(local, [version, version])
        npm_lock = load_json("app/package-lock.json")
        self.assertEqual(load_json("app/package.json")["version"], version)
        self.assertEqual(npm_lock["version"], version)
        self.assertEqual(npm_lock["packages"][""]["version"], version)


if __name__ == "__main__":
    unittest.main()
