"""Publish v0.0.3 only after independent native builds, signatures and tests pass."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import urllib.request

ROOT=Path(__file__).resolve().parents[2]
sys.path.insert(0,str(ROOT/'scripts'))
import version

VERSION='0.0.3'
TAG='v'+VERSION
APP_ID='io.local.deepseekcompanion.test2'
def digest(path):return hashlib.sha256(path.read_bytes()).hexdigest()
def read(path):return json.loads(path.read_text(encoding='utf-8-sig'))
def write(path,value):path.write_text(json.dumps(value,ensure_ascii=False,indent=2)+'\n')
def run(args,**kwargs):
    result=subprocess.run(list(map(str,args)),capture_output=True,timeout=300,**kwargs)
    if result.returncode:raise RuntimeError('Release operation failed: '+Path(str(args[0])).name)
    return result.stdout
def api(path):return json.loads(run(['gh','api',f"repos/{os.environ['GH_REPO']}/{path}"]))
def one(root,name):
    matches=list(root.rglob(name));assert matches,'Expected a verified '+name
    if len(matches)>1:
        assert all(re.fullmatch('signed-(windows|android)-0.0.3-[0-9]+',p.parent.name) for p in matches)
        matches=sorted(matches,key=lambda p:int(p.parent.name.rsplit('-',1)[1]))
    return matches[-1]
def validate_pair(windows,android,sha):
    assert windows['source_commit']==android['source_commit']==sha
    assert windows['version']==android['version']==VERSION
    assert windows['working_tree_clean'] and android['working_tree_clean']
    assert windows['signing']['authenticode_signed'] and windows['signing']['installed_process_started']
    assert windows['signing']['installer_language_verified'] and not windows['signing']['publicly_trusted_publisher']
    assert android['version_code']==1003 and android['application_id']==APP_ID
    assert android['signature_verified'] and android['test_signer_persistence_guaranteed']
    assert android['upgrade_compatible_with_test2_v0_0_2'] and not android['production_upgrade_compatible']
    assert not android['physical_device_tested'] and not windows['physical_device_tested']
def automation(tests):
    logs={p.name:p.read_text() for p in tests.glob('*.txt')}
    assert logs and 'rust-tests.txt' in logs and 'python-tests.txt' in logs and 'sync-model.txt' in logs
    assert 'test result: FAILED' not in logs['rust-tests.txt']
    rust=sum(map(int,re.findall(r'test result: ok\. (\d+) passed; 0 failed; 0 ignored;',logs['rust-tests.txt'])))
    py=re.search(r'Ran (\d+) tests',logs['python-tests.txt'])
    assert rust>=103 and py and int(py[1])>=23 and '\nOK' in logs['python-tests.txt']
    node=re.search(r'(?:# )?pass (\d+)',logs['sync-model.txt']);assert node and int(node[1])>=5
    ui={mode:read(tests/mode/'results.json') for mode in ['ui','ui-isolated']}
    assert all(item['passed']>=26 and 'Mock' in item['scope'] for item in ui.values())
    return {'rust':rust,'python':int(py[1]),'node':int(node[1]),'ui_fixture':{mode:item['passed'] for mode,item in ui.items()}},logs,ui
def main():
    parser=argparse.ArgumentParser();parser.add_argument('artifacts',type=Path);parser.add_argument('tests',type=Path);parser.add_argument('output',type=Path);parser.add_argument('--ci-run',type=int,required=True)
    args=parser.parse_args()
    assert os.environ.get('GITHUB_ACTIONS')=='true'
    sha=run(['git','rev-parse','HEAD'],cwd=ROOT).decode().strip()
    assert sha==os.environ['EXPECTED_SOURCE_SHA']
    assert api('commits/main')['sha']==sha,'main moved; rebuild the latest source rather than publishing an older candidate'
    assert version.check()=={'version':VERSION,'androidVersionCode':1003}
    ci=api('actions/runs/'+str(args.ci_run))
    assert ci['head_sha']==sha and ci['conclusion']=='success' and ci['head_branch']=='main'
    windows_path=one(args.artifacts,'windows-build.json');android_path=one(args.artifacts,'android-build.json')
    windows=read(windows_path);android=read(android_path);validate_pair(windows,android,sha)
    public_win=read(windows_path.parent/'SIGNING-IDENTITIES.json');public_android=read(android_path.parent/'SIGNING-IDENTITIES.json')
    for key in ['android_application_id','android_certificate_sha256','windows_certificate_sha256']:
        assert public_win[key]==public_android[key]
    assert public_win['android_certificate_sha256']==android['signer_certificate_sha256']
    assert public_win['windows_certificate_sha256']==windows['signing']['signer_certificate_sha256']
    counts,logs,ui=automation(args.tests)
    counts['windows_rust']=sum(map(int,re.findall(r'test result: ok\. (\d+) passed; 0 failed; 0 ignored;',(windows_path.parent/'windows-build.log').read_text(encoding='utf-8-sig'))))
    assert counts['windows_rust']>=counts['rust']
    output=args.output;output.mkdir(parents=True,exist_ok=True)
    files={}
    for report,path in [(windows,windows_path.parent),(android,android_path.parent)]:
        name=report['file'];assert Path(name).name==name
        package=path/name;assert digest(package)==report['sha256'] and package.stat().st_size==report['bytes']
        if name.endswith('.exe'):assert package.read_bytes().startswith(b'MZ')
        shutil.copyfile(package,output/name);files[name]=digest(package)
    for name in ['BUILD-RESULTS-WINDOWS.json','windows-installer-language.json','windows-installed-window.png',
                 'WINDOWS-TEST-SIGNER.cer','signed-windows-install.log','windows-build.log']:
        path=windows_path.parent/name
        if path.is_file():shutil.copyfile(path,output/name)
    for name in ['android-reverified.json','android-build.log','android-toolchain.log','ANDROID-SIGNING-REFERENCES.json','apksigner-0.0.3.txt']:
        shutil.copyfile(android_path.parent/name,output/name)
    write(output/'SIGNING-IDENTITIES.json',public_win)
    report={'version':VERSION,'android_version_code':1003,'source_commit':sha,'ci_run':args.ci_run,
        'signing_workflow_run':os.environ['GITHUB_RUN_ID'],'distribution':'persisted-test2-update',
        'windows':windows,'android':android,'tests':counts,'files':files,
        'physical_windows_android_tested':False,'home_ethernet_wifi_tested':False,'real_deepseek_api_tested':False,
        'test_scopes':['real Rust/SQLite migrations and domain tests','real loopback TLS 1.3 mutual authentication and sync','loopback provider fixture; not live DeepSeek','explicit browser UI fixtures'],
        'same_source_verified':True,'production_android_upgrade':False}
    write(output/'BUILD-RESULTS.json',report)
    (output/'SHA256SUMS.txt').write_text(''.join(value+'  '+name+'\n' for name,value in sorted(files.items())))
    (output/'AUTOMATED-TEST-RESULTS.txt').write_text('\n\n'.join(name+'\n'+body for name,body in sorted(logs.items())))
    write(output/'UI-TEST-RESULTS.json',{'source_commit':sha,'scope':'explicit UI fixtures; not real devices','runs':ui})
    for source,name in [('docs/PHASE3_REPORT.md','PHASE3-REPORT.md'),('docs/PHASE2_PERSONA_IDENTITY_FIX.md','PAIRING-FIX-REPORT.md'),('docs/PHASE3_ACCEPTANCE.md','INSTALL-AND-ACCEPTANCE.md'),('docs/TEST_SIGNING_SETUP.md','SIGNING-SETUP.md'),('CHANGELOG.md','CHANGELOG.md')]:
        shutil.copyfile(ROOT/source,output/name)
    for path in output.iterdir():
        assert path.is_file() and path.suffix not in ['.jks','.keystore','.p12','.pfx','.key']
        if path.suffix in ['.md','.txt','.json','.log']:
            content=path.read_text(encoding='utf-8-sig')
            assert not re.search(r'-----BEGIN (?:RSA |EC |ENCRYPTED )?PRIVATE KEY-----|github_pat_[A-Za-z0-9_]{20,}|gh[pousr]_[A-Za-z0-9]{20,}',content),'Private material pattern; publication blocked'
    repo=os.environ['GH_REPO'];base='https://github.com/'+repo+'/releases/download/'+TAG+'/'
    body=f"""这是 DeepSeek娘 0.0.3 测试版：修复扫码后默认人格重复，新增关系、情绪、分层记忆和虚拟生活基础。

- [Windows 中文 EXE]({base}{windows['file']})
- [Android ARM64 测试 APK]({base}{android['file']})
- [SHA256SUMS.txt]({base}SHA256SUMS.txt) · [BUILD-RESULTS.json]({base}BUILD-RESULTS.json)
- [阶段 2 人格修复报告]({base}PAIRING-FIX-REPORT.md) · [阶段 3 功能报告]({base}PHASE3-REPORT.md) · [实机清单]({base}INSTALL-AND-ACCEPTANCE.md)

两端源码：[{sha}](https://github.com/{repo}/commit/{sha})；[同源 CI {args.ci_run}](https://github.com/{repo}/actions/runs/{args.ci_run})。无需解压，直接选择 EXE/APK 安装。

Android 保持 {APP_ID} 和现有持久签名，真实旧 APK/apksigner 检查确认与 0.0.2「DeepSeek娘 扫码测试版」相同、versionCode 增至 1003。签名和版本满足该测试版覆盖条件；真实手机覆盖安装仍待验收。它不能覆盖 v0.0.1 正式版或旧 .test，不要求卸载/清数据；迁移方案见报告。没有生成替代签名。

Windows 已验证中文向导、签后安装及启动，沿用现有自签名测试证书，仍非公共 CA 信任发布者。

已配对手机通过认证连接采用电脑默认人格 ID；原自动默认记录以可审计关联保留，聊天/记忆/消息 ID 不删除，手动人格不合并。请同时更新两端，旧同步 schema 会明确拒绝混用。

关系不是每条消息加分；情绪与记忆有来源，记忆冲突需确认。虚拟看视频/听音乐不代表真实媒体播放或理解。主动消息默认关闭；规则与记忆整理不额外调用模型。实机、真实 DeepSeek 和家庭网络尚待主人验收，不作为自动化通过项。完成本版本后停止，不自动开始阶段 4。
"""
    notes=Path(os.environ['RUNNER_TEMP'])/'stage3-release-notes.md';notes.write_text(body)
    existing=subprocess.run(['gh','api',f'repos/{repo}/releases/tags/{TAG}'],capture_output=True,text=True)
    if existing.returncode==0:
        prior=json.loads(existing.stdout)
        assert prior['draft'] and prior['target_commitish']==sha,'Published version already exists; never overwrite or reuse it'
        assert all(a['name'] in {p.name for p in output.iterdir()} for a in prior['assets'])
    else:
        assert '404' in existing.stderr,'Release metadata permission failed'
        tag=subprocess.run(['gh','api',f'repos/{repo}/git/ref/tags/{TAG}'],capture_output=True,text=True)
        assert tag.returncode!=0 and '404' in tag.stderr,'Version tag already exists; never move it'
        run(['gh','release','create',TAG,'--repo',repo,'--target',sha,'--title','0.0.3测试版 · 关系与记忆','--prerelease','--draft','--notes-file',notes])
    for path in sorted(output.iterdir()):run(['gh','release','upload',TAG,path,'--repo',repo,'--clobber'])
    release=api('releases/tags/'+TAG);assert release['draft'] and release['prerelease']
    assets={a['name']:a for a in release['assets']}
    assert set(assets)=={p.name for p in output.iterdir()}
    verification=Path(os.environ['RUNNER_TEMP'])/'stage3-download-verify';verification.mkdir(exist_ok=True)
    for name,asset in assets.items():
        assert asset['digest']=='sha256:'+digest(output/name) and asset['size']==(output/name).stat().st_size
        with (verification/name).open('wb') as downloaded:
            result=subprocess.run(['gh','api',f"repos/{repo}/releases/assets/{asset['id']}",'-H','Accept: application/octet-stream'],stdout=downloaded,stderr=subprocess.PIPE,timeout=180)
        assert result.returncode==0 and digest(verification/name)==digest(output/name),'Draft download verification failed'
    run(['gh','release','edit',TAG,'--repo',repo,'--draft=false','--prerelease','--notes-file',notes])
    published=api('releases/tags/'+TAG)
    assert not published['draft'] and published['prerelease']
    assert api('git/ref/tags/'+TAG)['object']['sha']==sha
    for asset in published['assets']:
        request=urllib.request.Request(asset['browser_download_url']+'?verified_asset='+str(asset['id']),headers={'Cache-Control':'no-cache'})
        with urllib.request.urlopen(request,timeout=180) as response:
            assert response.status==200;data=response.read()
        assert 'sha256:'+hashlib.sha256(data).hexdigest()==asset['digest'] and len(data)==asset['size']
    print(json.dumps({'release_url':published['html_url'],'source_commit':sha,'download_verified_assets':len(published['assets']),'status':'published and verified'},ensure_ascii=False))

if __name__=='__main__':main()
