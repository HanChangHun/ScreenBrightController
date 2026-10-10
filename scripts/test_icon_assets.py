from pathlib import Path
import unittest
from PIL import Image

ROOT = Path(__file__).resolve().parents[1]
ICONS = ROOT / 'app/src-tauri/icons'

class IconAssets(unittest.TestCase):
    def test_windows_icon_sizes(self):
        with Image.open(ICONS / 'icon.ico') as im:
            self.assertTrue({(16,16),(24,24),(32,32),(48,48),(64,64),(128,128),(256,256)} <= im.ico.sizes())
    def test_transparent_monitor_and_diagonal(self):
        with Image.open(ICONS / 'screen-bright-controller.png') as im:
            self.assertEqual(im.size, (256,256))
            self.assertEqual(im.convert('RGBA').getpixel((0,0))[3], 0)
            self.assertGreater(sum(im.getpixel((60,60))[:3]), sum(im.getpixel((190,150))[:3]))

if __name__ == '__main__':
    unittest.main()
