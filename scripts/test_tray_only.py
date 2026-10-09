"""Tray-only application contract; source/config checks never launch native gamma."""
import json
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]


class TrayOnlyTests(unittest.TestCase):
    def test_only_the_hidden_tray_popup_is_created_and_can_be_opened(self):
        config = json.loads((ROOT / 'app/src-tauri/tauri.conf.json').read_text(encoding='utf-8'))
        self.assertEqual([w['label'] for w in config['app']['windows']], ['popup'])
        popup = config['app']['windows'][0]
        self.assertFalse(popup['visible'])
        self.assertTrue(popup['skipTaskbar'])
        self.assertFalse(popup['decorations'])
        self.assertEqual(popup['url'], 'index.html')
        capability = json.loads((ROOT / 'app/src-tauri/capabilities/main.json').read_text(encoding='utf-8'))
        self.assertEqual(capability['windows'], ['popup'])
        native = (ROOT / 'app/src-tauri/src/main.rs').read_text(encoding='utf-8')
        self.assertNotIn('get_webview_window("main")', native)
        self.assertNotIn('open_main', native)
        self.assertNotIn('Open Screen Bright Controller', native)
        self.assertNotIn('UiSession::main_close', native)
        self.assertIn('single_instance::init(|_, _, _| {})', native)
        self.assertNotIn('if mode != LaunchMode::Tray', native)
        self.assertIn('Restore and quit', native)
        html = (ROOT / 'app/ui/index.html').read_text(encoding='utf-8')
        self.assertIn('data-surface="popup"', html)
        self.assertIn('id="close-popup"', html)
        self.assertIn('id="settings-toggle"', html)
        self.assertIn('id="autostart"', html)
        self.assertNotIn('Open app', html)
        self.assertNotIn('open-main', html)
        source = (ROOT / 'app/ui/app.js').read_text(encoding='utf-8')
        self.assertNotIn('open_main', source)
        self.assertNotIn('open-main', source)
        self.assertNotIn('!popup', source)
        self.assertFalse((ROOT / 'app/ui/popup.html').exists(), 'Keep only one popup HTML entry point')
        harness = (ROOT / 'app/ui/harness.html').read_text(encoding='utf-8')
        self.assertEqual(harness.count('<iframe'), 1)
        self.assertIn('src="index.html?demo=1"', harness)


if __name__ == '__main__':
    unittest.main()
