"""Release metadata policy only, not a synthetic claim of Windows GUI execution."""
import hashlib
import importlib.util
from pathlib import Path
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location('windows_revision', ROOT / '.github/scripts/publish-windows-revision.py')
revision = importlib.util.module_from_spec(spec)
spec.loader.exec_module(revision)


class WindowsRevision(unittest.TestCase):
    def test_node_totals_accept_node24_spec_and_tap_and_reject_failures(self):
        for report in ['ℹ pass 5\nℹ fail 0\n','# pass 5\n# fail 0\n']:
            self.assertEqual(revision.node_passes(report),5)
        for report in ['ℹ pass 4\nℹ fail 1\n','# pass 5\n','unrelated output']:
            with self.assertRaises(AssertionError):revision.node_passes(report)
    def test_chinese_revision_requires_native_language_and_install_proof(self):
        with tempfile.TemporaryDirectory() as directory:
            installer = Path(directory) / 'setup.exe'
            installer.write_bytes(b'MZ metadata unit test fixture')
            digest = hashlib.sha256(installer.read_bytes()).hexdigest()
            sha = 'ab' * 20
            report = {'version':'0.0.2','source_commit':sha,'working_tree_clean':True,
                      'hosted_runner_install_smoke':True,'sha256':digest}
            smoke = {'build_commit':sha,'installer_sha256':digest,'installed_on_windows_runner':True,
                     'installed_process_started':True,'installer_language':'zh-CN','installer_language_verified':True}
            language = {'source_commit':sha,'language':'zh-CN','native_wizard_verified':True,'labels':['下一步(&N) >','取消']}
            self.assertEqual(revision.validate_windows(report,smoke,language,installer,sha),digest)
            with self.assertRaises(AssertionError):
                revision.validate_windows(report,smoke,{**language,'labels':['Next','Cancel']},installer,sha)
            with self.assertRaises(AssertionError):
                revision.validate_windows(report,{**smoke,'installed_process_started':False},language,installer,sha)

    def test_android_asset_and_original_release_identity_must_remain_unchanged(self):
        request = {'release_id':123,'original_source_sha':'ab'*20,'android_sha256':'12'*32}
        release = {'id':123,'tag_name':'v0.0.2','draft':False,'prerelease':True,'target_commitish':'ab'*20,
                   'assets':[{'name':revision.ANDROID_NAME,'digest':'sha256:'+'12'*32}]}
        revision.preserved_android(release,request)
        with self.assertRaises(AssertionError):
            revision.preserved_android({**release,'target_commitish':'00'*20},request)
        with self.assertRaises(AssertionError):
            revision.preserved_android({**release,'assets':[{'name':revision.ANDROID_NAME,'digest':'sha256:'+'00'*32}]},request)


if __name__ == '__main__':
    unittest.main()
