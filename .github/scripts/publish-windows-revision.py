"""Append a verified Chinese Windows revision without replacing the signed phone APK."""
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import urllib.request

ROOT = Path(__file__).resolve().parents[2]
ANDROID_NAME = 'DeepSeek-Niang_0.0.2_android-arm64-isolated-test.apk'


def validate_windows(report, smoke, language, installer, sha):
    digest = hashlib.sha256(installer.read_bytes()).hexdigest()
    assert installer.read_bytes().startswith(b'MZ')
    assert report['version'] == '0.0.2' and report['source_commit'] == sha
    assert report['working_tree_clean'] and report['hosted_runner_install_smoke']
    assert report['sha256'] == digest == smoke['installer_sha256']
    assert smoke['build_commit'] == sha and smoke['installed_on_windows_runner'] and smoke['installed_process_started']
    assert smoke['installer_language'] == 'zh-CN' and smoke['installer_language_verified']
    assert language['source_commit'] == sha and language['language'] == 'zh-CN' and language['native_wizard_verified']
    assert any('下一步' in text or '取消' in text for text in language['labels'])
    return digest


def preserved_android(release, request):
    assert release['id'] == request['release_id'] and release['tag_name'] == 'v0.0.2'
    assert not release['draft'] and release['prerelease']
    assert release['target_commitish'] == request['original_source_sha']
    assets = [a for a in release['assets'] if a['name'] == ANDROID_NAME]
    assert len(assets) == 1 and assets[0]['digest'] == 'sha256:' + request['android_sha256']


def main():
    request = json.loads((ROOT / '.github/release-requests/v0.0.2-windows-zh.json').read_text())
    repo = os.environ['GH_REPO']
    def api(path, method='GET', data=None):
        args = ['gh', 'api', f'repos/{repo}/{path}', '--method', method]
        if data is not None:
            args += ['--input', '-']
        result = subprocess.run(args, input=json.dumps(data) if data is not None else None, capture_output=True, text=True, check=True)
        return json.loads(result.stdout) if result.stdout.strip() else None
    original = api('releases/' + str(request['release_id']))
    preserved_android(original, request)
    ref = api('git/ref/tags/v0.0.2')
    assert ref['object']['sha'] == request['original_source_sha'], 'Original version tag must stay unchanged'
    source = Path(sys.argv[1])
    report = json.loads((source / 'windows-build.json').read_text(encoding='utf-8-sig'))
    smoke = json.loads((source / 'BUILD-RESULTS-WINDOWS.json').read_text(encoding='utf-8-sig'))
    language = json.loads((source / 'windows-installer-language.json').read_text(encoding='utf-8-sig'))
    assert Path(report['file']).name == report['file']
    installer = source / report['file']
    digest = validate_windows(report, smoke, language, installer, request['source_sha'])
    output = ROOT / '.cache/windows-zh-release'
    output.mkdir(parents=True, exist_ok=True)
    name = 'DeepSeek-Niang_0.0.2_windows-x64-setup-zh.exe'
    shutil.copyfile(installer, output / name)
    (output / 'WINDOWS-ZH-SHA256SUMS.txt').write_text(f'{digest}  {name}\n')
    (output / 'WINDOWS-ZH-INSTALLER-LANGUAGE.json').write_text(json.dumps(language, ensure_ascii=False, indent=2)+'\n')
    manifest = {'version':'0.0.2','scope':'windows-only-chinese-revision','source_commit':request['source_sha'],
                'file':name,'sha256':digest,'windows':report,'native_install_and_language':smoke,
                'ci_run':request['ci_run_id'],'android_replaced':False,'original_pair_source_commit':request['original_source_sha'],
                'android_sha256':request['android_sha256'],'phone_scan_update':'source implemented; delivery waits for the same Android signing identity',
                'physical_windows_android_lan_tested':False}
    (output / 'WINDOWS-ZH-BUILD-RESULTS.json').write_text(json.dumps(manifest, ensure_ascii=False, indent=2)+'\n')
    shutil.copyfile(source / 'windows-build.log', output / 'WINDOWS-ZH-BUILD.log')
    tests = Path(sys.argv[2])
    logs = {name:(tests / name).read_text() for name in ['rust-tests.txt','python-tests.txt','sync-model.txt','ui-tests.txt','ui-isolated-tests.txt','clippy.txt','schema.txt','frontend.txt','branding.txt','version.txt']}
    rust = re.findall(r'test result: ok\. (\d+) passed; 0 failed; 0 ignored;', logs['rust-tests.txt'])
    assert rust and sum(map(int,rust)) >= 77
    assert 'Ran 17 tests' in logs['python-tests.txt'] and '\nOK' in logs['python-tests.txt']
    assert '# pass 5' in logs['sync-model.txt'] and '# fail 0' in logs['sync-model.txt']
    for mode in ['ui','ui-isolated']:
        ui = json.loads((tests / mode / 'results.json').read_text())
        assert ui['passed'] == 20 and 'Mock' in ui['scope']
    test_report = {'source_commit':request['source_sha'],'ci_run':request['ci_run_id'],
                   'rust_passed':sum(map(int,rust)),'python_passed':17,'node_passed':5,
                   'browser_ui_fixture_passed':{'standard':20,'isolated_banner':20},
                   'physical_windows_android_tested':False,'note':'Real loopback TCP/TLS and SQLite are automated; browser camera lifecycle uses synthetic streams, not physical devices.'}
    (output / 'WINDOWS-ZH-AUTOMATED-TESTS.json').write_text(json.dumps(test_report,ensure_ascii=False,indent=2)+'\n')
    (output / 'WINDOWS-ZH-TESTS.log').write_text('\n\n'.join(name+'\n'+content for name,content in logs.items()))
    before = {a['name']:(a['id'],a['digest']) for a in original['assets']}
    for path in sorted(output.iterdir()):
        if not path.is_file():continue
        current = api('releases/' + str(request['release_id']))
        existing = [a for a in current['assets'] if a['name'] == path.name]
        expected = 'sha256:' + hashlib.sha256(path.read_bytes()).hexdigest()
        if existing:
            assert len(existing)==1 and existing[0]['digest']==expected, 'Existing revision assets must not be overwritten'
        else:
            subprocess.run(['gh','release','upload','v0.0.2',str(path),'--repo',repo],check=True)
    release = api('releases/' + str(request['release_id']))
    preserved_android(release, request)
    after = {a['name']:(a['id'],a['digest']) for a in release['assets']}
    assert all(after.get(name)==identity for name,identity in before.items()), 'Original assets must stay byte-identical'
    for path in sorted(output.iterdir()):
        if not path.is_file():continue
        asset = next(a for a in release['assets'] if a['name']==path.name)
        data = hashlib.sha256();size=0
        with urllib.request.urlopen(asset['browser_download_url'],timeout=60) as response:
            while block:=response.read(1024*1024):data.update(block);size+=len(block)
        assert size==path.stat().st_size and data.hexdigest()==hashlib.sha256(path.read_bytes()).hexdigest()
    marker = '<!-- windows-chinese-revision -->'
    if marker not in (release['body'] or ''):
        body = f'''{marker}
## Windows 中文安装修订

- [下载 Windows 中文安装程序](https://github.com/{repo}/releases/download/v0.0.2/{name})
- [中文修订校验文件](https://github.com/{repo}/releases/download/v0.0.2/WINDOWS-ZH-SHA256SUMS.txt) · [构建与中文向导验证](https://github.com/{repo}/releases/download/v0.0.2/WINDOWS-ZH-BUILD-RESULTS.json)

电脑安装向导已验证为简体中文。修订源码：`{request['source_sha']}`。
按你的选择，**Android APK 保持原文件/原签名，不要求卸载或清除数据**；新版扫码代码等待相同手机签名可用后交付，旧手机请使用电脑中的“旧版手机：显示兼容配对码”。此附件是 Windows 单端修订，不冒称与旧 APK 来自同一源码。

下面保留最初成套测试版的文件与说明，作为回退；原 Tag、APK、安装包和原校验/报告均未改动。

---
'''
        api('releases/'+str(request['release_id']),'PATCH',{'body':body+(release['body'] or '')})
    print(f'https://github.com/{repo}/releases/download/v0.0.2/{name}')
    print('Chinese Windows revision published; original tag, Android APK and all original assets preserved.')


if __name__ == '__main__':
    main()
