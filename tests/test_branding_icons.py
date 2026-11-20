import hashlib
import json
from pathlib import Path
import struct
import unittest

ROOT = Path(__file__).resolve().parents[1]


class BrandingIcons(unittest.TestCase):
    def test_multi_density_ico_and_launcher_full_bleed(self):
        icons = ROOT / 'apps/companion/src-tauri/icons'
        ico = (icons / 'icon.ico').read_bytes()
        reserved, kind, count = struct.unpack_from('<HHH', ico)
        self.assertEqual((reserved, kind, count), (0, 1, 7))
        sizes = set()
        for entry in range(count):
            w, h, _, _, _, bits, size, offset = struct.unpack_from('<BBBBHHII', ico, 6 + entry * 16)
            self.assertEqual(w, h)
            self.assertEqual(bits, 32)
            self.assertEqual(ico[offset:offset+8], b'\x89PNG\r\n\x1a\n')
            self.assertLessEqual(offset+size, len(ico))
            self.assertEqual(ico[offset+25], 6)  # PNG RGBA
            sizes.add(w or 256)
        self.assertEqual(sizes, {16,24,32,48,64,128,256})
        brand = ROOT / 'assets/branding'
        manifest = json.loads((brand/'manifest.json').read_text())
        self.assertEqual(manifest['adaptive_content_dp'], manifest['adaptive_visible_dp'])
        self.assertEqual(manifest['adaptive_canvas_dp'], 108)
        self.assertEqual(hashlib.sha256((brand/'deepseek-niang-master.png').read_bytes()).hexdigest(),
                         '070adedf4a8896745ea8fb7755b3cbe4b673bcb51d1edc88cc76e46ea937aff1')
        for density, pixels in [('mdpi',108),('hdpi',162),('xhdpi',216),('xxhdpi',324),('xxxhdpi',432)]:
            path=brand/f'android/mipmap-{density}/ic_launcher_foreground.png'
            self.assertEqual(struct.unpack_from('>II',path.read_bytes(),16),(pixels,pixels))
            self.assertEqual(path.read_bytes(),(icons/path.relative_to(brand)).read_bytes())
