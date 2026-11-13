"""Publish only verified dual builds; never overwrite an existing version."""
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import time
import urllib.request
sys.path.insert(0, 'scripts')
from version import check

version = check()['version']
tag = 'v' + version
commit = subprocess.check_output(['git', 'rev-parse', 'HEAD'], text=True).strip()
repo = os.environ['GH_REPO']


def api(endpoint, method='GET', data=None):
    command = ['gh', 'api', endpoint, '--method', method]
    if data is not None:
        command += ['--input', '-']
    result = subprocess.run(command, input=json.dumps(data) if data is not None else None,
                            capture_output=True, text=True, check=True)
    return json.loads(result.stdout) if result.stdout.strip() else None


# A network/permission error must not be mistaken for an unused version.
for endpoint in [f'repos/{repo}/git/ref/tags/{tag}', f'repos/{repo}/releases/tags/{tag}']:
    probe = subprocess.run(['gh', 'api', endpoint], capture_output=True, text=True)
    if probe.returncode == 0:
        raise SystemExit('Existing historical tag/release must never be overwritten')
    if 'HTTP 404' not in probe.stderr:
        raise SystemExit('Unable to confirm unused version; release blocked')
assets = sorted(Path('release-files').iterdir())
notes = Path(f'docs/RELEASE_{version}.md')
# Keep the actual ID returned by our create operation for safe failed-draft cleanup.
draft = api(f'repos/{repo}/releases', 'POST', {'tag_name': tag, 'target_commitish': commit,
    'name': f'DeepSeek娘 {version} 测试版', 'body': notes.read_text(), 'draft': True, 'prerelease': True})
release_id = draft['id']
try:
    subprocess.run(['gh', 'release', 'upload', tag, *map(str, assets)], check=True)
    remote = api(f'repos/{repo}/releases/{release_id}')
    assert remote['draft'] and remote['prerelease'] and remote['target_commitish'] == commit
    assert {a['name'] for a in remote['assets']} == {p.name for p in assets}
    for entry in remote['assets']:
        local = Path('release-files') / entry['name']
        assert entry['size'] == local.stat().st_size
        if entry.get('digest'):
            assert entry['digest'] == 'sha256:' + hashlib.sha256(local.read_bytes()).hexdigest()
    subprocess.run(['gh', 'release', 'edit', tag, '--draft=false', '--prerelease'], check=True)
except Exception:
    # Only our known unpublished draft can be removed. Never delete public history.
    current = api(f'repos/{repo}/releases/{release_id}')
    if current['draft'] and current['tag_name'] == tag and current['target_commitish'] == commit:
        api(f'repos/{repo}/releases/{release_id}', 'DELETE')
        ref = subprocess.run(['gh', 'api', f'repos/{repo}/git/ref/tags/{tag}'], capture_output=True, text=True)
        if ref.returncode == 0 and json.loads(ref.stdout)['object']['sha'] == commit:
            api(f'repos/{repo}/git/refs/tags/{tag}', 'DELETE')
    raise

# Verify public URLs by actually downloading their bytes; size/PID alone is not success.
remote = api(f'repos/{repo}/releases/{release_id}')
assert not remote['draft'] and remote['prerelease']
for entry in remote['assets']:
    local = Path('release-files') / entry['name']
    expected = hashlib.sha256(local.read_bytes()).hexdigest()
    for attempt in range(5):
        try:
            digest = hashlib.sha256()
            size = 0
            with urllib.request.urlopen(entry['browser_download_url'], timeout=60) as response:
                while chunk := response.read(1024 * 1024):
                    size += len(chunk)
                    digest.update(chunk)
            assert size == local.stat().st_size and digest.hexdigest() == expected
            break
        except Exception:
            if attempt == 4:
                raise RuntimeError('Release exists; public download verification is incomplete. Revalidate this release; do not allocate a replacement version.')
            time.sleep(attempt + 1)
    print(entry['browser_download_url'])
print(f'Published and downloaded prerelease {tag} from {commit}; physical acceptance remains pending.')
