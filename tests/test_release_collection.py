"""Synthetic release metadata policy tests; not native installation/signature tests."""
import hashlib
import json
from pathlib import Path
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]
ORIGINAL = '9de96ea5fb1d0b3f91ea73d9a16c030428453b15838927a85f86773eaef208a1'


class ReleaseCollection(unittest.TestCase):
    def collect(self, *, explicit=True, signed=True, source_matches=True, smoke=True, original_key=False):
        with tempfile.TemporaryDirectory() as folder:
            base = Path(folder)
            source = base / 'artifacts'
            commit = subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip()
            version = json.loads((ROOT / 'VERSION.json').read_text())['version']
            for platform, file, data in [('windows', 'test-setup.exe', b'MZ synthetic policy fixture'),
                                         ('android', 'isolated-test.apk', b'synthetic policy fixture, not an installable APK')]:
                directory = source / f'{platform}-package-1'
                directory.mkdir(parents=True)
                (directory / file).write_bytes(data)
                digest = hashlib.sha256(data).hexdigest()
                report = {'source_commit': commit if source_matches else '0' * 40,
                          'version': version, 'working_tree_clean': True, 'sha256': digest,
                          'signature_verified': signed, 'application_id': 'io.local.deepseekcompanion.test',
                          'distribution': 'isolated-test', 'upgrade_compatible': False,
                          'signer_certificate_sha256': ORIGINAL if original_key else '12' * 32,
                          'debuggable': True, 'automatic_backup': False, 'cleartext_traffic': False,
                          'hosted_runner_install_smoke': smoke}
                (directory / f'{platform}-build.json').write_text(json.dumps(report))
                if platform == 'windows':
                    (directory / 'BUILD-RESULTS-WINDOWS.json').write_text(json.dumps({
                        'build_commit': commit, 'installer_sha256': digest,
                        'installed_on_windows_runner': smoke, 'installed_process_started': smoke}))
            evidence = source / 'automated-test-reports-1'
            evidence.mkdir()
            (evidence / 'tests.txt').write_text('Synthetic metadata policy fixture only.')
            for name in ['ui', 'ui-isolated']:
                (evidence / name).mkdir()
                (evidence / name / 'results.json').write_text(json.dumps({'passed': 1}))
            command = ['python3', str(ROOT / '.github/scripts/collect-packages.py'), str(source), str(base / 'result')]
            if explicit:
                command.append('--isolated-test')
            result = subprocess.run(command, cwd=ROOT, capture_output=True, text=True)
            if result.returncode == 0:
                manifest = json.loads((base / 'result/BUILD-RESULTS.json').read_text())
                self.assertEqual(manifest['distribution'], 'isolated-test')
                self.assertTrue(list((base / 'result').glob('*isolated-test.apk')))
                self.assertFalse(list((base / 'result').glob('*_android-arm64.apk')))
            return result.returncode

    def test_test_prerelease_requires_explicit_distribution_authorization(self):
        self.assertNotEqual(self.collect(explicit=False), 0)
        self.assertEqual(self.collect(), 0)

    def test_unsigned_and_original_upgrade_keys_cannot_be_test_release_signers(self):
        self.assertNotEqual(self.collect(signed=False), 0)
        self.assertNotEqual(self.collect(original_key=True), 0)

    def test_wrong_source_or_missing_windows_native_install_proof_is_rejected(self):
        self.assertNotEqual(self.collect(source_matches=False), 0)
        self.assertNotEqual(self.collect(smoke=False), 0)


if __name__ == '__main__':
    unittest.main()
