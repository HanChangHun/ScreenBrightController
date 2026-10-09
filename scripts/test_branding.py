"""Current product identity; legacy spellings below are negative test fixtures only."""
from pathlib import Path
import json
import re
import unittest

ROOT = Path(__file__).resolve().parents[1]


class BrandingTests(unittest.TestCase):
    def test_current_brand_is_consistent_across_packages_files_and_protocol_names(self):
        core = (ROOT / 'Cargo.toml').read_text(encoding='utf-8')
        tray = (ROOT / 'app/src-tauri/Cargo.toml').read_text(encoding='utf-8')
        self.assertIn('name = "screen-bright-controller"', core)
        self.assertIn('name = "screen-bright-controller-tray"', tray)
        self.assertIn('screen-bright-controller = { path = "../.." }', tray)
        self.assertTrue((ROOT / 'src/bin/screen-bright-controller-cli.rs').is_file())
        self.assertFalse((ROOT / 'src/bin/gamma-cli.rs').exists())
        icons = ROOT / 'app/src-tauri/icons'
        for suffix in ['.png', '.svg', '-16.png', '-24.png', '-32.png', '-48.png', '-64.png', '-128.png', '-256.png']:
            self.assertTrue((icons / ('screen-bright-controller' + suffix)).is_file(), suffix)
        self.assertEqual(list(icons.glob('dimmer*')), [])
        legacy = re.compile(r'gamma[_-]dimmer|gamma-cli|gamma-error|GAMMA_WATCHDOG')
        paths = [ROOT / 'Cargo.toml', ROOT / 'Cargo.lock', ROOT / 'app/src-tauri/Cargo.toml',
                 ROOT / 'README.md', ROOT / 'README.ko.md', ROOT / 'MANUAL_TESTS.md', ROOT / 'VALIDATION.md']
        for folder, glob in [('src', '*.rs'), ('tests', '*.rs'), ('app/src-tauri/src', '*.rs'),
                             ('app/ui', '*.js'), ('scripts', '*.py')]:
            paths.extend((ROOT / folder).rglob(glob))
        for path in paths:
            if path.resolve() == Path(__file__).resolve():
                continue
            self.assertIsNone(legacy.search(path.read_text(encoding='utf-8')), str(path.relative_to(ROOT)))
        watchdog = (ROOT / 'src/watchdog.rs').read_text(encoding='utf-8')
        for name in ['SCREEN_BRIGHT_CONTROLLER_WATCHDOG_TOKEN', 'SCREEN_BRIGHT_CONTROLLER_WATCHDOG_PARENT']:
            self.assertIn(name, watchdog)
        self.assertIn('emit("display-error"', (ROOT / 'app/src-tauri/src/main.rs').read_text(encoding='utf-8'))
        self.assertIn("listen('display-error'", (ROOT / 'app/ui/app.js').read_text(encoding='utf-8'))
        config = json.loads((ROOT / 'app/src-tauri/tauri.conf.json').read_text(encoding='utf-8'))
        self.assertEqual(config['mainBinaryName'], 'ScreenBrightController')
        self.assertEqual(config['identifier'], 'io.github.screenbrightcontroller')
        self.assertEqual(config['productName'], 'Screen Bright Controller')


if __name__ == '__main__':
    unittest.main()
