"""Build/export policy tests with temporary fixtures; not Android native verification."""
import importlib.util
import json
import tempfile
import unittest
from pathlib import Path
import xml.etree.ElementTree as ET

ROOT = Path(__file__).resolve().parents[1]


def module(name):
    spec = importlib.util.spec_from_file_location(name, ROOT / "scripts" / f"{name}.py")
    value = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(value)
    return value


class BuildPolicy(unittest.TestCase):
    def test_windows_installer_defaults_to_simplified_chinese(self):
        config = json.loads((ROOT / 'apps/companion/src-tauri/tauri.conf.json').read_text())
        self.assertEqual(config['bundle']['windows']['nsis']['languages'], ['SimpChinese'])
        self.assertFalse(config['bundle']['windows']['nsis']['displayLanguageSelector'])
        self.assertEqual(config['bundle']['windows']['wix']['language'], 'zh-CN')
        self.assertEqual(config['identifier'], 'io.local.deepseekcompanion')
    def test_android_preparation_is_idempotent_and_disables_automatic_backup(self):
        prepare = module("prepare_android")
        with tempfile.TemporaryDirectory() as folder:
            project = Path(folder)
            manifest = project / "app/src/main/AndroidManifest.xml"
            manifest.parent.mkdir(parents=True)
            manifest.write_text('<manifest xmlns:android="http://schemas.android.com/apk/res/android"><application><activity android:name=".MainActivity"/></application></manifest>')
            gradle = project / "app/build.gradle.kts"
            gradle.write_text("android {\n    compileSdk = 37\n    defaultConfig {\n        targetSdk = 37\n    }\n}\n")
            prepare.prepare(project)
            prepare.prepare(project)
            root = ET.parse(manifest).getroot()
            android = "{http://schemas.android.com/apk/res/android}"
            self.assertEqual(len(root.findall("uses-permission")), 4)
            self.assertEqual(root.find("uses-feature").get(android + "required"), "false")
            self.assertEqual(root.find("application").get(android + "allowBackup"), "false")
            self.assertEqual(root.find("application").get(android + "usesCleartextTraffic"), "false")
            self.assertEqual(root.find("application/activity").get(android + "windowSoftInputMode"), "adjustResize")
            self.assertTrue((project / "app/src/main/res/xml/companion_data_extraction_rules.xml").is_file())
            self.assertIn("compileSdk = 36", gradle.read_text())
            self.assertIn("targetSdk = 36", gradle.read_text())
            activity = project / 'app/src/main/java/io/local/deepseekcompanion/MainActivity.kt'
            self.assertEqual(activity.read_bytes(), (ROOT / 'apps/companion/android/MainActivity.kt').read_bytes())

    def test_android_debug_installation_keeps_production_identity_and_isolates_test_data(self):
        prepare = module("prepare_android")
        with tempfile.TemporaryDirectory() as folder:
            project = Path(folder)
            manifest = project / "app/src/main/AndroidManifest.xml"
            manifest.parent.mkdir(parents=True)
            manifest.write_text('<manifest xmlns:android="http://schemas.android.com/apk/res/android"><application><activity android:name=".MainActivity"/></application></manifest>')
            gradle = project / "app/build.gradle.kts"
            original = 'android {\n compileSdk = 36\n defaultConfig {\n targetSdk = 36\n applicationId = "io.local.deepseekcompanion"\n }\n}\n'
            gradle.write_text(original)
            prepare.prepare(project)
            prepare.prepare(project)
            text = gradle.read_text()
            self.assertIn('applicationId = "io.local.deepseekcompanion"', text)
            self.assertEqual(text.count('applicationIdSuffix = ".test"'), 1)
            self.assertIn('buildTypes.getByName("debug")', text)
            main = ET.parse(project / "app/src/main/res/values/strings.xml").getroot()
            debug = ET.parse(project / "app/src/debug/res/values/companion_test.xml").getroot()
            self.assertEqual(main.find("string[@name='app_name']").text, "DeepSeek娘")
            self.assertEqual(debug.find("string[@name='app_name']").text, "DeepSeek娘 测试版")

    def test_export_excludes_databases_keys_dependencies_and_symlinks(self):
        exporter = module("export_source")
        with tempfile.TemporaryDirectory() as folder:
            exporter.ROOT = Path(folder)
            for name in ["README.md", ".gitattributes", "Cargo.lock", "crates/core/src/lib.rs", "data/chat.sqlite3", "apps/app/user.db-wal", "apps/app/key.pem", "apps/app/node_modules/package.json", "plugins/test/android/.tauri/copied-sdk.kt", "plugins/test/android/build/outputs/aar/plugin.aar", "plugins/test/android/.gradle/cache.bin", "backups/private.bak", "apps/app/notes.backup", ".env"]:
                file = exporter.ROOT / name
                file.parent.mkdir(parents=True, exist_ok=True)
                file.write_text("fixture")
            try:
                (exporter.ROOT / "apps/app/linked.rs").symlink_to(exporter.ROOT / ".env")
            except (OSError, NotImplementedError):
                pass
            names = {str(relative) for _, relative in exporter.source_files()}
            self.assertEqual(names, {"README.md", ".gitattributes", "Cargo.lock", "crates/core/src/lib.rs"})

    def test_unsigned_test_compile_never_creates_a_signer_and_can_restore_signed_build_policy(self):
        prepare = module('prepare_android')
        with tempfile.TemporaryDirectory() as folder:
            project = Path(folder)
            manifest = project / 'app/src/main/AndroidManifest.xml'
            manifest.parent.mkdir(parents=True)
            manifest.write_text('<manifest xmlns:android="http://schemas.android.com/apk/res/android"><application><activity android:name=".MainActivity"/></application></manifest>')
            gradle = project / 'app/build.gradle.kts'
            gradle.write_text('android {\n compileSdk = 36\n defaultConfig {\n targetSdk = 36\n applicationId = "io.local.deepseekcompanion"\n }\n}\n')
            prepare.prepare(project, unsigned_test=True)
            prepare.prepare(project, unsigned_test=True)
            self.assertEqual(gradle.read_text().count('signingConfig = null'), 1)
            self.assertIn('applicationIdSuffix = ".test"', gradle.read_text())
            self.assertEqual(list(project.rglob('*.keystore')), [])
            prepare.prepare(project, unsigned_test=True, fresh_test=True)
            self.assertIn('applicationIdSuffix = ".test2"', gradle.read_text())
            self.assertIn('io.local.deepseekcompanion', gradle.read_text())
            debug = ET.parse(project / 'app/src/debug/res/values/companion_test.xml').getroot()
            self.assertEqual(debug.find("string[@name='app_name']").text, 'DeepSeek娘 扫码测试版')
            prepare.prepare(project)
            self.assertNotIn('signingConfig = null', gradle.read_text())
            self.assertIn('applicationIdSuffix = ".test"', gradle.read_text())

    def test_export_rejects_credential_shaped_content_even_in_env_example(self):
        exporter = module("export_source")
        with tempfile.TemporaryDirectory() as folder:
            exporter.ROOT = Path(folder)
            # Construct a synthetic pattern for the scanner; never an actual credential.
            (exporter.ROOT / ".env.example").write_text("KEY=" + "sk-" + "x" * 30)
            with self.assertRaises(SystemExit):
                list(exporter.source_files())


if __name__ == "__main__":
    unittest.main()
