"""Assemble release evidence only after both platform builds finish successfully."""
import hashlib
import argparse
import json
from pathlib import Path
import shutil
import subprocess
import sys

root = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(root / 'scripts'))
import version as version_module

parser = argparse.ArgumentParser()
parser.add_argument('artifacts', type=Path)
parser.add_argument('destination', type=Path)
parser.add_argument('--source-root', type=Path, default=root)
parser.add_argument('--isolated-test', action='store_true', help='Explicitly authorized standalone test prerelease; never an upgrade APK')
args = parser.parse_args()
source_root = args.source_root.resolve()
version_module.ROOT = source_root

version = version_module.check()['version']
commit = subprocess.check_output(['git', '-C', str(source_root), 'rev-parse', 'HEAD'], text=True).strip()
source = args.artifacts
dest = args.destination
dest.mkdir(parents=True, exist_ok=True)
reports = {}
for platform, extension in [('windows', '.exe'), ('android', '.apk')]:
    report_paths = list(source.rglob(f'{platform}-build.json'))
    assert report_paths, f'Missing successful {platform} build report'
    # Rerunning only failed jobs retains a successful peer job from an earlier attempt.
    # Keep each attempt's diagnostics and select the latest complete report per platform.
    import re
    def attempt(path):
        match = re.search(r'-(\d+)$', path.parent.name)
        return int(match[1]) if match else 0
    report_path = max(report_paths, key=attempt)
    candidates = list(report_path.parent.rglob('*setup.exe' if platform == 'windows' else '*.apk'))
    assert len(candidates) == 1, f'Expected exactly one {platform} package in selected attempt'
    artifact = candidates[0]
    report = json.loads(report_path.read_text(encoding='utf-8-sig'))
    assert report['source_commit'] == commit and report['version'] == version
    assert report['working_tree_clean'], 'Package build modified tracked source or included untracked source'
    assert report['sha256'] == hashlib.sha256(artifact.read_bytes()).hexdigest()
    if platform == 'android':
        original = '9de96ea5fb1d0b3f91ea73d9a16c030428453b15838927a85f86773eaef208a1'
        assert report['signature_verified']
        if args.isolated_test:
            assert report['application_id'] == 'io.local.deepseekcompanion.test'
            assert report['distribution'] == 'isolated-test' and report['upgrade_compatible'] is False
            assert len(report['signer_certificate_sha256']) == 64 and report['signer_certificate_sha256'] != original
            assert report['debuggable'] and not report['automatic_backup'] and not report['cleartext_traffic']
        else:
            assert report['application_id'] == 'io.local.deepseekcompanion'
            assert report['distribution'] == 'upgrade' and report['upgrade_compatible'], 'Isolated test APK cannot enter an upgrade release'
            assert report['signer_certificate_sha256'] == original
    else:
        assert artifact.read_bytes().startswith(b'MZ'), 'Windows installer is not a PE executable'
        assert report['hosted_runner_install_smoke'], 'Windows native installation was not verified'
        smoke = json.loads((report_path.parent / 'BUILD-RESULTS-WINDOWS.json').read_text(encoding='utf-8-sig'))
        assert smoke['build_commit'] == commit and smoke['installer_sha256'] == report['sha256']
        assert smoke['installed_on_windows_runner'] and smoke['installed_process_started']
    android_name = 'android-arm64-isolated-test.apk' if args.isolated_test else 'android-arm64.apk'
    name = f'DeepSeek-Niang_{version}_' + ('windows-x64-setup.exe' if platform == 'windows' else android_name)
    shutil.copyfile(artifact, dest / name)
    reports[platform] = report
checksums = ''.join(f'{hashlib.sha256(p.read_bytes()).hexdigest()}  {p.name}\n' for p in sorted(dest.iterdir()) if p.suffix in ['.exe', '.apk'])
(dest / 'SHA256SUMS.txt').write_text(checksums)
(dest / 'BUILD-RESULTS.json').write_text(json.dumps({'version': version, 'source_commit': commit, 'builds': reports,
    'distribution': 'isolated-test' if args.isolated_test else 'upgrade',
    'physical_windows_tested': False, 'physical_android_tested': False, 'home_ethernet_wifi_sync_tested': False}, indent=2) + '\n')
shutil.copyfile(source_root / 'docs/PHASE2_ACCEPTANCE.md', dest / 'INSTALL-AND-ACCEPTANCE.md')
shutil.copyfile(source_root / 'CHANGELOG.md', dest / 'CHANGELOG.md')
for name in ['windows-build.log', 'android-build.log', 'android-toolchain.log', 'BUILD-RESULTS-WINDOWS.json', 'android-reverified.json']:
    matches = list(source.rglob(name))
    if len(matches) == 1:
        shutil.copyfile(matches[0], dest / name)
if args.isolated_test:
    report_dirs = list(source.glob('automated-test-reports-*'))
    assert len(report_dirs) == 1, 'Successful automation evidence is required'
    evidence = report_dirs[0]
    text = []
    for path in sorted(evidence.glob('*.txt')):
        text.append(f'\n## {path.name}\n{path.read_text()}')
    assert text, 'Automation report files missing'
    (dest / 'AUTOMATED-TEST-RESULTS.txt').write_text(''.join(text))
    ui = {name: json.loads((evidence / name / 'results.json').read_text()) for name in ['ui', 'ui-isolated']}
    assert all(value['passed'] > 0 for value in ui.values())
    (dest / 'UI-TEST-RESULTS.json').write_text(json.dumps({'source_commit': commit, 'scope': 'Explicit UI fixtures, not physical devices', 'runs': ui}, indent=2) + '\n')
