#!/usr/bin/env python3
"""Verify actual APK identity, ABI, version, ZIP contents and signing continuity."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import zipfile
from android_signing import EXPECTED_CERT
from version import check, ROOT


def verify_reference(apk):
    """Read the certificate from the actual historical APK, not a copied digest."""
    tools = Path(os.environ['ANDROID_HOME']) / 'build-tools/36.0.0'
    signature = subprocess.check_output([str(tools / 'apksigner'), 'verify', '--verbose', '--print-certs', str(apk)], text=True)
    certificates = re.findall(r'Signer #\d+ certificate SHA-256 digest: ([0-9a-f]+)', signature)
    assert certificates == [EXPECTED_CERT], 'Historical APK certificate differs; signing continuity must be investigated'
    badging = subprocess.check_output([str(tools / 'aapt2'), 'dump', 'badging', str(apk)], text=True)
    match = re.search(r"package: name='([^']+)' versionCode='([^']+)' versionName='([^']+)'", badging)
    assert match and match.groups() == ('io.local.deepseekcompanion', '1001', '0.0.1')
    return {'reference_version': '0.0.1', 'application_id': match[1],
            'sha256': hashlib.sha256(apk.read_bytes()).hexdigest(),
            'signer_certificate_sha256': certificates[0], 'signature_verified': True,
            'apksigner_output': signature}


def verify(apk, signed=True, test_package=False):
    assert not test_package or signed, 'Isolated test APK must have a real debug signature'
    version = check()
    tools = Path(os.environ['ANDROID_HOME']) / 'build-tools/36.0.0'
    with zipfile.ZipFile(apk) as package:
        assert package.testzip() is None
        names = package.namelist()
        assert 'AndroidManifest.xml' in names
        assert 'lib/arm64-v8a/libcompanion_app_lib.so' in names
        assert not any(n.endswith(('.jks', '.keystore', '.pem', '.key')) for n in names)
        assert not any(n.startswith('lib/') and not n.startswith('lib/arm64-v8a/') for n in names)
    badging = subprocess.check_output([str(tools / 'aapt2'), 'dump', 'badging', str(apk)], text=True)
    match = re.search(r"package: name='([^']+)' versionCode='([^']+)' versionName='([^']+)'", badging)
    expected_id = 'io.local.deepseekcompanion.test' if test_package else 'io.local.deepseekcompanion'
    assert match and match.groups() == (expected_id, str(version['androidVersionCode']), version['version'])
    label = 'DeepSeek娘 测试版' if test_package else 'DeepSeek娘'
    assert f"application-label:'{label}'" in badging, 'Incorrect launcher label'
    debuggable = 'application-debuggable' in badging
    assert debuggable == test_package, 'Debug APK cannot be treated as an upgrade release'
    manifest = subprocess.check_output([str(tools / 'aapt2'), 'dump', 'xmltree', '--file', 'AndroidManifest.xml', str(apk)], text=True)
    for setting in ['allowBackup', 'usesCleartextTraffic']:
        assert re.search(rf'android:{setting}\([^)]*\)=(?:false|\(type 0x12\)0x0)(?:\s|$)', manifest), f'Unsafe APK manifest setting: {setting}'
    assert 'android:sharedUserId' not in manifest, 'Shared application UID is not allowed'
    certificate = None
    if signed:
        signature = subprocess.check_output([str(tools / 'apksigner'), 'verify', '--verbose', '--print-certs', str(apk)], text=True)
        certificates = re.findall(r'Signer #\d+ certificate SHA-256 digest: ([0-9a-f]+)', signature)
        assert len(certificates) == 1, 'Expected one verified APK signer'
        if test_package:
            assert certificates[0] != EXPECTED_CERT, 'Isolated tests must not use the original upgrade key'
        else:
            assert certificates == [EXPECTED_CERT], 'Upgrade signature changed; release blocked'
        certificate = certificates[0]
    return {'platform': 'android', 'source_commit': subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip(),
            'working_tree_clean': not bool(subprocess.check_output(['git','status','--porcelain'],cwd=ROOT,text=True).strip()), 'version': version['version'], 'version_code': version['androidVersionCode'], 'application_id': match[1],
            'abi': 'arm64-v8a', 'file': apk.name, 'bytes': apk.stat().st_size, 'sha256': hashlib.sha256(apk.read_bytes()).hexdigest(),
            'signature_verified': signed, 'signer_certificate_sha256': certificate,
            'distribution': 'isolated-test' if test_package else ('upgrade' if signed else 'unsigned-ci'),
            'upgrade_compatible': signed and not test_package,
            'test_signer_persistence_guaranteed': False if test_package else None,
            'automatic_backup': False, 'cleartext_traffic': False, 'shared_application_uid': False,
            'debuggable': debuggable,
            'physical_device_tested': False}


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--apk', type=Path)
    parser.add_argument('--reference-apk', type=Path)
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument('--unsigned-ci', action='store_true')
    mode.add_argument('--test-package', action='store_true')
    parser.add_argument('--output', type=Path)
    args = parser.parse_args()
    if args.reference_apk:
        assert not args.apk and not args.unsigned_ci and not args.test_package
        data = verify_reference(args.reference_apk)
    else:
        variant = 'debug' if args.test_package else 'release'
        packages = [args.apk] if args.apk else sorted(p for p in (ROOT / 'apps/companion/src-tauri/gen/android/app/build/outputs/apk').rglob('*.apk') if p.parent.name == variant)
        assert len(packages) == 1, 'Expected exactly one ARM64 APK from this build'
        data = verify(packages[0], not args.unsigned_ci, args.test_package)
    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(json.dumps(data, indent=2) + '\n')
    print(json.dumps(data))
