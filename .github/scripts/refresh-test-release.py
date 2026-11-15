"""Replace the authorized 0.0.2 assets after durable dual signing and verification."""
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import urllib.request

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0,str(ROOT/'scripts'))
from test_release_identity import APP_ID, run


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def api(path, method='GET', data=None):
    args=['gh','api',f'repos/{os.environ["GH_REPO"]}/'+path,'--method',method]
    if data is not None:args += ['--input','-']
    value=run(args,value=json.dumps(data).encode() if data is not None else None)
    return json.loads(value) if value.strip() else None


def archive_name(asset, sha):
    assert '/' not in asset['name'] and '\\' not in asset['name']
    return f'ARCHIVE-{sha[:8]}-{asset["id"]}-{asset["name"]}'


def main():
    request=json.loads((ROOT/'.github/release-requests/v0.0.2-refresh.json').read_text())
    sha=request['source_sha'];assert re.fullmatch('[0-9a-f]{40}',sha)
    source=Path(sys.argv[1]).resolve();artifacts=Path(sys.argv[2]).resolve();smoke_root=Path(sys.argv[3]).resolve()
    assert subprocess.check_output(['git','-C',str(source),'rev-parse','HEAD'],text=True).strip()==sha
    sys.path.insert(0,str(source/'scripts'))
    import verify_android
    original=api('releases/'+str(request['release_id']))
    assert original['id']==445111880 and original['tag_name']=='v0.0.2' and original['prerelease'] and not original['draft']
    assert api('git/ref/tags/v0.0.2')['object']['sha']==request['original_tag_sha']
    protected=Path(os.environ['RUNNER_TEMP'])/'deepseek-protected-signing'
    identity=json.loads((protected/'runtime.json').read_text())
    assert identity['provisioning_complete'] and identity['android_application_id']==APP_ID
    public=json.loads((protected/'public.json').read_text())
    output=ROOT/'release-files';output.mkdir(exist_ok=True)
    windows=artifacts/'windows-package-1';android=artifacts/'android-package-1';tests=artifacts/'automated-test-reports-1'
    win=json.loads((windows/'windows-build.json').read_text(encoding='utf-8-sig'))
    apk=json.loads((android/'android-build.json').read_text())
    assert win['source_commit']==apk['source_commit']==sha and win['working_tree_clean'] and apk['working_tree_clean']
    assert win['version']==apk['version']=='0.0.2' and apk['application_id']==APP_ID and not apk['signature_verified']
    spec=importlib.util.spec_from_file_location('windows_policy',ROOT/'.github/scripts/publish-windows-revision.py')
    policy=importlib.util.module_from_spec(spec);spec.loader.exec_module(policy)
    old_smoke=json.loads((windows/'BUILD-RESULTS-WINDOWS.json').read_text(encoding='utf-8-sig'))
    language=json.loads((windows/'windows-installer-language.json').read_text(encoding='utf-8-sig'))
    original_exe=windows/win['file'];policy.validate_windows(win,old_smoke,language,original_exe,sha)
    assert digest(android/apk['file'])==apk['sha256']
    exe=output/'DeepSeek-Niang_0.0.2_windows-x64-setup.exe';shutil.copyfile(original_exe,exe)
    context=os.environ.copy();context.update(TEST_WINDOWS_INSTALLER=str(exe),TEST_WINDOWS_PASSWORD=identity['windows_password'],
        TEST_WINDOWS_PFX=str(protected/'windows-test.pfx'),TEST_WINDOWS_CER=str(protected/'windows-test.cer'))
    run(['pwsh','-NoProfile','-File',str(ROOT/'.github/scripts/windows-test-sign.ps1'),'-Action','Sign'],env=context)
    signed_apk=output/'DeepSeek-Niang_0.0.2_android-arm64-isolated-test.apk'
    context.update(TEST_ANDROID_PASSWORD=identity['android_password'])
    tools=Path(os.environ['ANDROID_HOME'])/'build-tools/36.0.0'
    aligned=Path(os.environ['RUNNER_TEMP'])/'deepseek-aligned-test.apk'
    run(verify_android.sdk_command(tools,'zipalign','-f','-P','16','4',str(android/apk['file']),str(aligned)))
    run(verify_android.sdk_command(tools,'apksigner','sign','--ks',str(protected/'android-test.p12'),
        '--ks-key-alias',identity['android_alias'],'--ks-pass','env:TEST_ANDROID_PASSWORD','--key-pass','env:TEST_ANDROID_PASSWORD',
        '--out',str(signed_apk),str(aligned)),env=context)
    verified=verify_android.verify(signed_apk,True,True,True)
    assert verified['source_commit']==sha and verified['working_tree_clean'] and verified['signer_certificate_sha256']==public['android_certificate_sha256']
    verified.update(test_signer_persistence_guaranteed=True,signature_source='GitHub encrypted repository signing bundle',
                    old_test_upgrade_compatible=False,future_test2_upgrade_identity_stable=True)
    # Re-run the real signed Windows installer, using the exact compiled source.
    target=Path(os.environ['RUNNER_TEMP'])/'signed-windows-smoke-target'
    native=target/'x86_64-pc-windows-msvc/release';(native/'bundle/nsis').mkdir(parents=True,exist_ok=True)
    shutil.copyfile(exe,native/'bundle/nsis/setup.exe')
    shutil.copyfile(windows/'DeepSeek-Niang_0.0.2_windows-x64.exe',native/'companion-app.exe')
    context.update(CARGO_TARGET_DIR=str(target))
    result=subprocess.run(['pwsh','-NoProfile','-File',str(source/'.github/scripts/windows-package-smoke.ps1')],cwd=source,env=context,capture_output=True)
    if result.returncode:raise RuntimeError('Signed Windows native installation/language smoke failed')
    (output/'signed-windows-install.log').write_bytes(result.stdout+result.stderr)
    signed_smoke=json.loads((smoke_root/'BUILD-RESULTS-WINDOWS.json').read_text(encoding='utf-8-sig'))
    assert signed_smoke['build_commit']==sha and signed_smoke['installed_process_started'] and signed_smoke['installer_sha256']==digest(exe)
    signed_smoke.update(authenticode_signed=True,authenticode_verified_on_disposable_runner=True,
                        publicly_trusted_publisher=False,signer_certificate_sha256=public['windows_certificate_sha256'])
    shutil.copyfile(exe,output/'DeepSeek-Niang_0.0.2_windows-x64-setup-zh.exe')
    shutil.copyfile(protected/'windows-test.cer',output/'WINDOWS-TEST-SIGNER.cer')
    (output/'SIGNING-IDENTITIES.json').write_text(json.dumps(public,indent=2)+'\n')
    shutil.copyfile(ROOT/'docs/TEST_SIGNING_SETUP.md',output/'SIGNING-SETUP.md')
    (output/'android-reverified.json').write_text(json.dumps(verified,indent=2)+'\n')
    (output/'BUILD-RESULTS-WINDOWS.json').write_text(json.dumps(signed_smoke,ensure_ascii=False,indent=2)+'\n')
    shutil.copyfile(windows/'windows-native-build.log',output/'windows-build.log')
    shutil.copyfile(android/'android-build.log',output/'android-build.log')
    shutil.copyfile(android/'android-toolchain.log',output/'android-toolchain.log')
    logs={p.name:p.read_text() for p in tests.glob('*.txt')}
    rust=re.findall(r'test result: ok\. (\d+) passed; 0 failed; 0 ignored;',logs['rust-tests.txt'])
    assert sum(map(int,rust))>=77
    python=re.search(r'Ran (\d+) tests',logs['python-tests.txt']);assert python and int(python[1])>=19 and '\nOK' in logs['python-tests.txt']
    policy.node_passes(logs['sync-model.txt'])
    ui={mode:json.loads((tests/mode/'results.json').read_text()) for mode in ['ui','ui-isolated']}
    assert all(item['passed']>=23 and 'Mock' in item['scope'] for item in ui.values())
    report={'source_commit':sha,'ci_run':request['ci_run_id'],'version':'0.0.2','scope':'same-source newly signed standalone test refresh',
        'windows':{**win,'file':exe.name,'bytes':exe.stat().st_size,'sha256':digest(exe),'unsigned_build_sha256':win['sha256'],'signing':signed_smoke},'android':verified,
        'tests':{'rust':sum(map(int,rust)),'python':int(python[1]),'node':5,'ui_fixture':{key:item['passed'] for key,item in ui.items()}},
        'physical_windows_android_tested':False,'original_tag_source':request['original_tag_sha'],
        'files':{p.name:digest(p) for p in output.iterdir() if p.suffix in ['.exe','.apk']}}
    (output/'BUILD-RESULTS.json').write_text(json.dumps(report,ensure_ascii=False,indent=2)+'\n')
    (output/'SHA256SUMS.txt').write_text(''.join(f'{value}  {name}\n' for name,value in report['files'].items()))
    (output/'AUTOMATED-TEST-RESULTS.txt').write_text('\n\n'.join(name+'\n'+body for name,body in logs.items()))
    (output/'INSTALL-AND-ACCEPTANCE.md').write_text('''# 0.0.2 扫码测试修订

Windows 下载 EXE 直接安装，关闭运行中的应用后覆盖程序文件，保留数据目录。安装向导为中文，使用新自签名测试证书；该证书不是公共 CA 认证发布者，系统仍可能提示未知发布者，不要关闭系统防护。

Android 打开“DeepSeek娘 扫码测试版”，身份 io.local.deepseekcompanion.test2，与旧测试版及正式版并存。不要卸载旧应用或清数据；旧手机尚未同步的数据不会自动迁移到新安装身份。新应用可通过扫码从电脑同步，API Key 各设备独立配置。后续 test2 更新必须复用同一 GitHub Secrets 签名。

电脑点“连接手机”显示二维码，手机点“扫一扫”直接调用相机。手动同步保留，本机信息在底部。真实手机通知栏/刘海/键盘、相机、家庭网线+Wi-Fi、断线补传与用户旧库保留仍待实机验收。
''',encoding='utf-8')
    complete=next((a for a in original['assets'] if a['name']=='BUILD-RESULTS.json'),None)
    if complete:
        with urllib.request.urlopen(complete['browser_download_url'],timeout=60) as response:previous=json.load(response)
        if previous.get('source_commit')==sha and previous.get('scope')==report['scope']:
            remote={a['name']:a for a in original['assets']}
            if all(name in remote and remote[name]['digest']=='sha256:'+value for name,value in previous['files'].items()):
                assert previous['android']['signer_certificate_sha256']==public['android_certificate_sha256']
                assert previous['windows']['signing']['signer_certificate_sha256']==public['windows_certificate_sha256']
                for name,value in previous['files'].items():
                    with urllib.request.urlopen(remote[name]['browser_download_url'],timeout=120) as response:assert hashlib.sha256(response.read()).hexdigest()==value
                print('This same-source signed refresh is already published; existing assets were preserved.')
                return
    # All checks and durable key storage finish BEFORE any public asset mutation.
    latest=api('releases/'+str(request['release_id']))
    assert {(a['id'],a['name'],a['digest']) for a in latest['assets']}=={(a['id'],a['name'],a['digest']) for a in original['assets']}, 'Release changed concurrently; refusing to replace it'
    archive=[]
    for item in original['assets']:
        if not item['name'].startswith('ARCHIVE-'):
            name=archive_name(item,sha)
            api('releases/assets/'+str(item['id']),'PATCH',{'name':name,'label':'历史附件 / previous build'})
            archive.append({'id':item['id'],'old_name':item['name'],'archive_name':name,'digest':item['digest']})
    archive_file='ARCHIVE-MANIFEST-'+sha[:8]+'-'+os.environ['GITHUB_RUN_ID']+'-'+os.environ.get('GITHUB_RUN_ATTEMPT','1')+'.json'
    (output/archive_file).write_text(json.dumps({'previous_release_body':original['body'],'assets':archive},ensure_ascii=False,indent=2)+'\n')
    subprocess.run(['gh','release','upload','v0.0.2',*map(str,sorted(output.iterdir())),'--repo',os.environ['GH_REPO']],check=True)
    current=api('releases/'+str(request['release_id']))
    by_id={item['id']:item for item in current['assets']}
    assert all(by_id[item['id']]['digest']==item['digest'] for item in original['assets']), 'Historical asset bytes must remain unchanged'
    for item in current['assets']:
        path=output/item['name']
        if path.is_file():
            assert item['digest']=='sha256:'+digest(path)
            with urllib.request.urlopen(item['browser_download_url'],timeout=120) as response:
                assert hashlib.sha256(response.read()).hexdigest()==digest(path)
    names={a['name'] for a in current['assets']};assert all(p.name in names for p in output.iterdir())
    repo=os.environ['GH_REPO'];base=f'https://github.com/{repo}/releases/download/v0.0.2/'
    body=f'''这是 0.0.2 扫码测试修订，已替换当前下载附件，无需解压。

- [Windows 中文 EXE]({base}{exe.name})
- [Android 扫码测试 APK]({base}{signed_apk.name})
- [SHA256SUMS.txt]({base}SHA256SUMS.txt) · [BUILD-RESULTS.json]({base}BUILD-RESULTS.json) · [签名身份]({base}SIGNING-IDENTITIES.json)

电脑一个“连接手机”按钮弹出二维码；手机一个“扫一扫”调用相机，保留手动同步。手机已加入独立底部导航与原生系统栏边距。

Android 使用持久保存的新测试签名与 `{APP_ID}`，独立安装、保留旧应用和聊天，不能覆盖旧 `.test` 或正式版。Windows 使用新的自签名测试证书，仍非公共 CA 信任发布者。私钥仅保存在仓库加密 Secrets；后续协作者通过同一仓库工作流复用。

两端安装包源码：[`{sha}`](https://github.com/{repo}/commit/{sha})，CI {request['ci_run_id']} 全部成功。实机相机、通知栏/键盘和家庭局域网仍待用户验收。

旧附件以 ARCHIVE- 开头保留原字节。原 v0.0.2 Tag 保留历史指向，页面自动 Source code 归档属于历史源码；本次安装包完整源码以上方提交链接和构建报告为准。
'''
    api('releases/'+str(request['release_id']),'PATCH',{'body':body,'name':'0.0.2测试版 · 扫码修订','prerelease':True})
    assert api('git/ref/tags/v0.0.2')['object']['sha']==request['original_tag_sha']
    print(base+exe.name);print(base+signed_apk.name)


if __name__=='__main__':
    main()
