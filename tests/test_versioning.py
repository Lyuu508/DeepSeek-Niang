import unittest
from test_build_policy import module


class Versioning(unittest.TestCase):
    def test_carries_follow_the_user_sequence(self):
        version = module('version')
        for before, after in [('0.0.1', '0.0.2'), ('0.0.9', '0.0.10'), ('0.0.10', '0.1.0'),
                              ('0.1.10', '0.2.0'), ('0.9.10', '1.0.0'), ('1.9.10', '2.0.0')]:
            self.assertEqual(version.next_version(before), after)

    def test_invalid_versions_cannot_allocate_releases(self):
        version = module('version')
        for value in ['v0.0.2', '0.00.2', '0.10.0', '0.0.11', '-1.0.0', '0.0.2-rc.1']:
            with self.assertRaises(ValueError):
                version.next_version(value)

    def test_platform_versions_and_identity_match_the_single_source(self):
        self.assertEqual(module('version').check()['version'], '0.0.2')


class AndroidVersionSequence(unittest.TestCase):
    def test_codes_cannot_decrease_reuse_or_skip_project_versions(self):
        version = module('version')
        history = [{'version': '0.0.1', 'androidVersionCode': 1001}]
        version.validate_state({'version': '0.0.2', 'androidVersionCode': 1002}, history)
        for data in [{'version': '0.0.2', 'androidVersionCode': 1001},
                     {'version': '0.0.2', 'androidVersionCode': 1000},
                     {'version': '0.0.3', 'androidVersionCode': 1003},
                     {'version': '0.0.1', 'androidVersionCode': 1002}]:
            with self.assertRaises(ValueError):
                version.validate_state(data, history)
