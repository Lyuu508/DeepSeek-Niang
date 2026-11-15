"""Provisioning policy tests; no real key or fabricated native signing result."""
import base64
import importlib.util
import json
from pathlib import Path
import sys
import unittest

ROOT=Path(__file__).resolve().parents[1]
sys.path.insert(0,str(ROOT/'scripts'))
import test_release_identity as identity
spec=importlib.util.spec_from_file_location('refresh_policy',ROOT/'.github/scripts/refresh-test-release.py')
refresh=importlib.util.module_from_spec(spec);spec.loader.exec_module(refresh)


class PersistedSigning(unittest.TestCase):
    def test_restore_only_requires_complete_existing_bundle(self):
        data={'format':1,'android_application_id':identity.APP_ID,'android_keystore':'synthetic-fixture',
              'android_password':'synthetic-fixture','android_alias':'synthetic-fixture',
              'windows_pfx':'synthetic-fixture','windows_password':'synthetic-fixture','provisioning_complete':False}
        encode=lambda d:base64.b64encode(json.dumps(d).encode()).decode()
        with self.assertRaises(AssertionError):identity.existing_bundle('')
        with self.assertRaises(AssertionError):identity.existing_bundle(encode(data))
        data['provisioning_complete']=True
        self.assertTrue(identity.existing_bundle(encode(data))['provisioning_complete'])
    def test_complete_bundle_keeps_a_separate_test_identity(self):
        data={'format':1,'android_application_id':identity.APP_ID,'android_keystore':'synthetic-fixture',
              'android_password':'synthetic-fixture','android_alias':'synthetic-fixture',
              'windows_pfx':'synthetic-fixture','windows_password':'synthetic-fixture'}
        encode=lambda d:base64.b64encode(json.dumps(d).encode()).decode()
        self.assertEqual(identity.load_bundle(encode(data))['android_application_id'],'io.local.deepseekcompanion.test2')
        for changed in [{**data,'android_application_id':'io.local.deepseekcompanion.test'},
                        {**data,'android_application_id':'io.local.deepseekcompanion'},
                        {**data,'windows_pfx':''},{**data,'android_password':''}]:
            with self.assertRaises(AssertionError):identity.load_bundle(encode(changed))

    def test_archive_names_preserve_prior_asset_identity_without_clobbering(self):
        first=refresh.archive_name({'id':1,'name':'setup.exe'},'ab'*20)
        self.assertNotEqual(first,refresh.archive_name({'id':2,'name':'setup.exe'},'ab'*20))
        self.assertTrue(first.startswith('ARCHIVE-'))
        with self.assertRaises(AssertionError):refresh.archive_name({'id':1,'name':'../private.pfx'},'ab'*20)


if __name__=='__main__':unittest.main()
