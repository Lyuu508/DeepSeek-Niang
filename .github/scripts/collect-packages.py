"""Assemble release evidence only after both platform builds finish successfully."""
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import sys

root = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(root / 'scripts'))
from version import check

version = check()['version']
commit = subprocess.check_output(['git', 'rev-parse', 'HEAD'], text=True).strip()
source = Path(sys.argv[1])
dest = Path(sys.argv[2])
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
        assert report['application_id'] == 'io.local.deepseekcompanion'
        assert report['distribution'] == 'upgrade' and report['upgrade_compatible'], 'Isolated test APK cannot enter an upgrade release'
        assert report['signature_verified'] and report['signer_certificate_sha256'] == '9de96ea5fb1d0b3f91ea73d9a16c030428453b15838927a85f86773eaef208a1'
    name = f'DeepSeek-Niang_{version}_' + ('windows-x64-setup.exe' if platform == 'windows' else 'android-arm64.apk')
    shutil.copyfile(artifact, dest / name)
    reports[platform] = report
checksums = ''.join(f'{hashlib.sha256(p.read_bytes()).hexdigest()}  {p.name}\n' for p in sorted(dest.iterdir()) if p.suffix in ['.exe', '.apk'])
(dest / 'SHA256SUMS.txt').write_text(checksums)
(dest / 'BUILD-RESULTS.json').write_text(json.dumps({'version': version, 'source_commit': commit, 'builds': reports,
    'physical_windows_tested': False, 'physical_android_tested': False, 'home_ethernet_wifi_sync_tested': False}, indent=2) + '\n')
shutil.copyfile(root / 'docs/PHASE2_ACCEPTANCE.md', dest / 'INSTALL-AND-ACCEPTANCE.md')
shutil.copyfile(root / 'CHANGELOG.md', dest / 'CHANGELOG.md')
