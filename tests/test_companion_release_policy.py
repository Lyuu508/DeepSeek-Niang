"""Metadata-gate fixtures only; actual installers/signatures are verified on hosted runners."""
import copy
import importlib.util
from pathlib import Path
import unittest

ROOT=Path(__file__).resolve().parents[1]
spec=importlib.util.spec_from_file_location('companion_release',ROOT/'.github/scripts/publish-companion-test.py')
policy=importlib.util.module_from_spec(spec);spec.loader.exec_module(policy)

class CompanionReleasePolicy(unittest.TestCase):
    def test_mixed_source_unsigned_or_wrong_identity_cannot_enter_a_test_release(self):
        sha='a'*40
        windows={'source_commit':sha,'version':'0.0.3','working_tree_clean':True,'physical_device_tested':False,
                 'signing':{'authenticode_signed':True,'installed_process_started':True,'installer_language_verified':True,'publicly_trusted_publisher':False}}
        android={'source_commit':sha,'version':'0.0.3','working_tree_clean':True,'physical_device_tested':False,
                 'version_code':1003,'application_id':policy.APP_ID,'signature_verified':True,
                 'test_signer_persistence_guaranteed':True,'upgrade_compatible_with_test2_v0_0_2':True,'production_upgrade_compatible':False}
        policy.validate_pair(windows,android,sha)
        for changed in [{'source_commit':'b'*40},{'version_code':1002},{'application_id':'io.local.deepseekcompanion'},
                        {'signature_verified':False},{'test_signer_persistence_guaranteed':False},{'production_upgrade_compatible':True}]:
            with self.assertRaises(AssertionError):policy.validate_pair(windows,{**android,**changed},sha)
        unsigned=copy.deepcopy(windows);unsigned['signing']['authenticode_signed']=False
        with self.assertRaises(AssertionError):policy.validate_pair(unsigned,android,sha)

if __name__=='__main__':unittest.main()
