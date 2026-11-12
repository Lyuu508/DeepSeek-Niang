"""Synthetic image fixtures only: these tests never create a replacement character."""
import hashlib
import importlib.util
import json
import subprocess
import tempfile
import unittest
from pathlib import Path

from test_build_policy import ROOT, module


@unittest.skipUnless(importlib.util.find_spec("PIL"), "Reviewed Pillow installation required for resize tests")
class BrandingPolicy(unittest.TestCase):
    def fixture(self, folder):
        from PIL import Image
        root = Path(folder)
        original = root / "synthetic-input.png"
        Image.new("RGBA", (256, 256), (25, 75, 155, 255)).save(original)
        config = root / "apps/companion/src-tauri/tauri.conf.json"
        config.parent.mkdir(parents=True)
        config.write_bytes((ROOT / "apps/companion/src-tauri/tauri.conf.json").read_bytes())
        return root, original

    def gate(self, root):
        source = (ROOT / "scripts/check-branding.mjs").as_uri()
        return subprocess.run(["node", "--input-type=module", "-e", f"import {{checkBranding}} from {json.dumps(source)}; checkBranding(process.argv[1]);", str(root)], capture_output=True, text=True)

    def test_original_bytes_ico_sizes_and_adaptive_safe_area(self):
        from PIL import Image
        with tempfile.TemporaryDirectory() as folder:
            root, original = self.fixture(folder)
            raw = original.read_bytes()
            manifest = module("prepare_branding").prepare(original, root)
            self.assertEqual((root / "assets/branding/deepseek-niang-master.png").read_bytes(), raw)
            self.assertEqual(manifest["source_sha256"], hashlib.sha256(raw).hexdigest())
            self.assertEqual(self.gate(root).returncode, 0)
            with Image.open(root / "apps/companion/src-tauri/icons/icon.ico") as image:
                self.assertEqual(image.ico.sizes(), {(n, n) for n in [16, 24, 32, 48, 64, 128, 256]})
            with Image.open(root / "assets/branding/android/mipmap-mdpi/ic_launcher_foreground.png") as image:
                self.assertEqual(image.size, (108, 108))
                left, top, right, bottom = image.getbbox()
                self.assertLessEqual(((right - left) ** 2 + (bottom - top) ** 2) ** .5, 66)
            # Rerunning with the identical original is safe and deterministic.
            again = module("prepare_branding").prepare(original, root)
            self.assertEqual(again["derived_sha256"], manifest["derived_sha256"])

    def test_packaging_rejects_pending_source_and_modified_resources(self):
        with tempfile.TemporaryDirectory() as folder:
            root, original = self.fixture(folder)
            branding = root / "assets/branding"
            branding.mkdir(parents=True)
            (branding / "manifest.json").write_text('{"status":"pending_source"}')
            self.assertNotEqual(self.gate(root).returncode, 0)
            module("prepare_branding").prepare(original, root)
            (root / "apps/companion/src-tauri/icons/icon.ico").write_bytes(b"tampered fixture")
            self.assertNotEqual(self.gate(root).returncode, 0)
            (branding / "deepseek-niang-master.png").write_bytes(b"different master fixture")
            with self.assertRaises(SystemExit):
                module("prepare_branding").prepare(original, root)

    def test_android_labels_resources_and_splash_are_applied_idempotently(self):
        import xml.etree.ElementTree as ET
        with tempfile.TemporaryDirectory() as folder:
            root, original = self.fixture(folder)
            module("prepare_branding").prepare(original, root)
            project = root / "android-fixture"
            manifest = project / "app/src/main/AndroidManifest.xml"
            manifest.parent.mkdir(parents=True)
            manifest.write_text('<manifest xmlns:android="http://schemas.android.com/apk/res/android"><application><activity android:name=".MainActivity"/></application></manifest>')
            prepare = module("prepare_android")
            for _ in range(2):
                prepare.prepare(project, root / "assets/branding")
            res = project / "app/src/main/res"
            self.assertEqual(ET.parse(res / "values/strings.xml").find("string[@name='app_name']").text, "DeepSeek娘")
            self.assertTrue((res / "drawable-xxxhdpi/ic_stat_deepseek.png").is_file())
            self.assertTrue((res / "mipmap-anydpi-v26/ic_launcher_round.xml").is_file())
            theme = ET.parse(res / "values-v31/themes.xml")
            self.assertEqual(theme.find("style/item[@name='android:windowSplashScreenAnimatedIcon']").text, "@drawable/deepseek_splash_icon")
            self.assertEqual(ET.parse(manifest).find("application").get("{http://schemas.android.com/apk/res/android}roundIcon"), "@mipmap/ic_launcher_round")


if __name__ == "__main__":
    unittest.main()
