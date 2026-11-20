"""Verify and publish the authorized v0.0.8 test installers."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import time
import urllib.request

ROOT=Path(__file__).resolve().parents[2]
sys.path.insert(0,str(ROOT/'scripts'))
import version

VERSION='0.0.8'
TAG='v'+VERSION
APP_ID='io.local.deepseekcompanion.test2'
STAGE='source and test validation'
def mark(value):
    global STAGE
    STAGE=value
    print('::notice title=Release validation::'+value,flush=True)
def digest(path):return hashlib.sha256(path.read_bytes()).hexdigest()
def read(path):return json.loads(path.read_text(encoding='utf-8-sig'))
def write(path,value):path.write_text(json.dumps(value,ensure_ascii=False,indent=2)+'\n')
def run(args,**kwargs):
    result=subprocess.run(list(map(str,args)),capture_output=True,timeout=300,**kwargs)
    if result.returncode:raise RuntimeError('Release operation failed: '+Path(str(args[0])).name)
    return result.stdout
def api(path):return json.loads(run(['gh','api',f"repos/{os.environ['GH_REPO']}/{path}",'-H','Cache-Control: no-cache']))
def find_release(tag):
    # /releases/tags/{tag} only finds published releases. Drafts have no Git tag yet.
    page=1
    while True:
        items=api(f'releases?per_page=100&page={page}')
        matches=[item for item in items if item['tag_name']==tag]
        assert len(matches)<=1,'Ambiguous release version'
        if matches:return matches[0]
        if len(items)<100:return None
        page+=1
def one(root,name):
    matches=list(root.rglob(name));assert matches,'Expected a verified '+name
    if len(matches)>1:
        assert all(re.fullmatch(r'signed-(windows|android)-'+re.escape(VERSION)+r'-[0-9]+',p.parent.name) for p in matches)
        matches=sorted(matches,key=lambda p:int(p.parent.name.rsplit('-',1)[1]))
    return matches[-1]
def validate_pair(windows,android,sha):
    assert windows['source_commit']==android['source_commit']==sha
    assert windows['version']==android['version']==VERSION
    assert windows['working_tree_clean'] and android['working_tree_clean']
    assert windows['signing']['authenticode_signed'] and windows['signing']['installed_process_started']
    assert windows['signing']['installer_language_verified'] and not windows['signing']['publicly_trusted_publisher']
    assert windows['signing'].get('desktop_pet_bundled') and windows['signing'].get('local_video_bundled')
    assert windows['signing']['voice_bundled'] and windows['signing']['native_voice_probe'] in {
        'system_voice_wav_generated', 'runner_has_no_usable_system_voice',
        'runner_system_synthesis_failed', 'runner_system_synthesis_timed_out'}
    assert windows['signing'].get('application_identifier')=='io.local.deepseekcompanion'
    assert android['version_code']==1008 and android['application_id']==APP_ID
    assert android['signature_verified'] and android['test_signer_persistence_guaranteed']
    assert android['offline_voice_bridge_bundled'] and android['microphone_permission_declared']
    assert android['upgrade_compatible_with_test2_v0_0_7'] and not android['production_upgrade_compatible']
    assert not android['physical_device_tested'] and not windows['physical_device_tested']
def automation(tests):
    logs={p.name:p.read_text() for p in tests.glob('*.txt')}
    assert logs and 'rust-tests.txt' in logs and 'python-tests.txt' in logs and 'sync-model.txt' in logs
    assert 'test result: FAILED' not in logs['rust-tests.txt']
    rust=sum(map(int,re.findall(r'test result: ok\. (\d+) passed; 0 failed; 0 ignored;',logs['rust-tests.txt'])))
    py=re.search(r'Ran (\d+) tests',logs['python-tests.txt'])
    assert rust>=142 and py and int(py[1])>=24 and '\nOK' in logs['python-tests.txt']
    node=re.search(r'(?:# )?pass (\d+)',logs['sync-model.txt']);assert node and int(node[1])>=5
    ui={mode:read(tests/mode/'results.json') for mode in ['ui','ui-isolated']}
    assert all(item['passed']>=32 and 'Mock' in item['scope'] for item in ui.values())
    ui['desktop-pet-ui']=read(tests/'desktop-pet-ui'/'results.json')
    assert ui['desktop-pet-ui']['passed']>=7 and 'Browser IPC fixture' in ui['desktop-pet-ui']['scope']
    ui['voice-ui']=read(tests/'voice-ui'/'results.json')
    assert ui['voice-ui']['passed']>=5 and 'Browser IPC fixture' in ui['voice-ui']['scope']
    gait=re.search(r'test result: ok\. (\d+) passed; 0 failed;',logs['desktop-pet-gait-tests.txt'])
    assert gait and int(gait[1])>=3
    assert 'real_media_reads_both_halves_and_continuous_audio ... ok' in logs['rust-tests.txt']
    assert 'guide_has_real_notes_and_silence_between_them ... ok' in logs['rust-tests.txt']
    return {'real_media_decoding_fixture':True,'rust':rust,'python':int(py[1]),'node':int(node[1]),'desktop_pet_gait':int(gait[1]),
            'ui_fixture':{mode:item['passed'] for mode,item in ui.items()}},logs,ui
def main():
    parser=argparse.ArgumentParser();parser.add_argument('artifacts',type=Path);parser.add_argument('tests',type=Path);parser.add_argument('output',type=Path);parser.add_argument('--ci-run',type=int,required=True)
    args=parser.parse_args()
    assert os.environ.get('GITHUB_ACTIONS')=='true'
    sha=run(['git','rev-parse','HEAD'],cwd=ROOT).decode().strip()
    assert sha==os.environ['EXPECTED_SOURCE_SHA']
    assert api('commits/main')['sha']==sha,'main moved; rebuild the latest source rather than publishing an older candidate'
    assert version.check()=={'version':VERSION,'androidVersionCode':1008}
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
    for name in ['android-reverified.json','android-build.log','android-toolchain.log','ANDROID-SIGNING-REFERENCES.json',f'apksigner-{VERSION}.txt']:
        shutil.copyfile(android_path.parent/name,output/name)
    references=read(android_path.parent/'ANDROID-SIGNING-REFERENCES.json')
    previous=references['previous_test2']
    assert previous['version']=='0.0.7' and previous['version_code']==1007 and previous['application_id']==APP_ID
    assert previous['signer_certificate_sha256']==android['signer_certificate_sha256']=='fefb57342751c7bd5ef8271e4a8b09f8702c0285e0120e8407b557d20dbbf153'
    assert 'Verified using' in previous['apksigner_output'] and re.fullmatch('[0-9a-f]{64}',previous['sha256'])
    assert public_win['windows_certificate_sha256']=='069f0150a9698a8178bdaf7dfea13f5509185aa69b9ba940f5473edbafa8af62'
    write(output/'SIGNING-IDENTITIES.json',public_win)
    report={'version':VERSION,'android_version_code':1008,'source_commit':sha,'ci_run':args.ci_run,
        'signing_workflow_run':os.environ['GITHUB_RUN_ID'],'distribution':'persisted-test2-update',
        'windows':windows,'android':android,'tests':counts,'files':files,
        'physical_windows_android_tested':False,'home_ethernet_wifi_tested':False,'real_deepseek_api_tested':False,
        'test_scopes':['real Rust/SQLite migrations and domain tests','real loopback TLS 1.3 mutual authentication and sync','loopback provider fixture; not live DeepSeek','explicit browser UI fixtures'],
        'same_source_verified':True,'production_android_upgrade':False}
    write(output/'BUILD-RESULTS.json',report)
    (output/'SHA256SUMS.txt').write_text(''.join(value+'  '+name+'\n' for name,value in sorted(files.items())))
    (output/'AUTOMATED-TEST-RESULTS.txt').write_text('\n\n'.join(name+'\n'+body for name,body in sorted(logs.items())))
    write(output/'UI-TEST-RESULTS.json',{'source_commit':sha,'scope':'explicit UI fixtures; not real devices','runs':ui})
    for source,name in [
        ('docs/phases/07-voice-singing.md','INSTALL-AND-ACCEPTANCE.md'),
        ('apps/companion/src/desktop-pet/media/NOTICE.md','ANIMATION-CREDITS.md'),
        ('CHANGELOG.md','CHANGELOG.md'),
    ]:
        shutil.copyfile(ROOT/source,output/name)
    for path in output.iterdir():
        assert path.is_file() and path.suffix not in ['.jks','.keystore','.p12','.pfx','.key']
        if path.suffix in ['.md','.txt','.json','.log']:
            content=path.read_text(encoding='utf-8-sig')
            assert not re.search(r'-----BEGIN (?:RSA |EC |ENCRYPTED )?PRIVATE KEY-----|github_pat_[A-Za-z0-9_]{20,}|gh[pousr]_[A-Za-z0-9]{20,}',content),'Private material pattern; publication blocked'
    repo=os.environ['GH_REPO'];base='https://github.com/'+repo+'/releases/download/'+TAG+'/'
    body=f"""DeepSeek娘 {VERSION} 本机语音测试版。两端主客户端的「设置」提供语音与麦克风配置。可在聊天中点击「朗读」或「语音输入」。Windows 桌宠根据实际播放音量提示发声。

- [Windows 中文主客户端 EXE]({base}{windows['file']})
- [Android ARM64 APK]({base}{android['file']})
- [SHA-256 校验]({base}SHA256SUMS.txt) · [构建结果]({base}BUILD-RESULTS.json)
- [本机准备与实测范围]({base}INSTALL-AND-ACCEPTANCE.md) · [桌宠动画署名]({base}ANIMATION-CREDITS.md)

主客户端提供本机离线朗读、语音输入和原创元音唱段。语音转写先填入输入框。Windows 录音转写需要已安装的 whisper-cli 与模型，Android 语音输入需要 Android 12 或更新版本和设备离线普通话识别能力。两端音色由本机系统提供，音色和播放偏好按人格保存在本机。

Windows 安装器在 Windows runner 安装并启动。Android 保持 v0.0.7 test2 签名，已经过 APK 内容和签名核验。实体设备麦克风、语音音质和真实 DeepSeek 对话待用户设备验收。唱段为按七个音符合成的元音。

[源码](https://github.com/{repo}/commit/{sha}) · [同源 CI](https://github.com/{repo}/actions/runs/{args.ci_run})
"""
    prior=find_release(TAG)
    if prior:
        assert prior['draft'] and prior['target_commitish']==sha,'Published version already exists; never overwrite or reuse it'
        assert all(a['name'] in {p.name for p in output.iterdir()} for a in prior['assets'])
    else:
        tag=subprocess.run(['gh','api',f'repos/{repo}/git/ref/tags/{TAG}'],capture_output=True,text=True)
        assert tag.returncode!=0 and '404' in tag.stderr,'Version tag already exists; never move it'
        mark('create unpublished release by official REST API')
        payload=Path(os.environ['RUNNER_TEMP'])/'companion-release-draft.json'
        write(payload,{'tag_name':TAG,'target_commitish':sha,'name':f'{VERSION}测试版 · 本机语音','draft':True,'prerelease':True,'body':body})
        prior=json.loads(run(['gh','api','--method','POST',f'repos/{repo}/releases','--input',payload]))
    release_id=prior['id']
    existing_assets={a['name']:a for a in prior['assets']}
    for path in sorted(output.iterdir()):
        mark('upload '+path.name)
        old=existing_assets.get(path.name)
        if old and old.get('digest')=='sha256:'+digest(path) and old['size']==path.stat().st_size:continue
        run(['gh','release','upload',TAG,path,'--repo',repo,'--clobber'])
    mark('verify canonical release metadata and all asset digests')
    for attempt in range(5):
        release=api(f'releases/{release_id}?verification={time.time_ns()}')
        assets={a['name']:a for a in release['assets']}
        complete=set(assets)=={p.name for p in output.iterdir()} and all(a.get('digest')=='sha256:'+digest(output/name) and a['size']==(output/name).stat().st_size for name,a in assets.items())
        if complete:break
        if attempt<4:time.sleep(2)
    assert release['draft'] and release['prerelease'] and release['target_commitish']==sha and complete
    print('Both signed packages, test reports and '+str(len(release['assets']))+' uploaded asset digests verified.',flush=True)
    assets={a['name']:a for a in release['assets']}
    assert set(assets)=={p.name for p in output.iterdir()}
    # GitHub computes these digests over the actual uploaded bytes. Require
    # equality with locally verified signed packages BEFORE publication. A draft
    # GITHUB_TOKEN download is not portable; verify real public HTTP bytes below.
    if os.environ.get('PUBLISH_VERIFIED_RELEASE')!='true':
        mark('verified draft ready; public release and Git tag intentionally pending')
        print(json.dumps({'draft_release_id':release_id,'source_commit':sha,'uploaded_assets':len(assets),'all_server_digests_match_verified_files':True,'publicly_published':False}),flush=True)
        return
    mark('create verified version tag without moving an existing tag')
    tag=subprocess.run(['gh','api',f'repos/{repo}/git/ref/tags/{TAG}'],capture_output=True,text=True)
    if tag.returncode==0:assert json.loads(tag.stdout)['object']['sha']==sha
    else:
        assert '404' in tag.stderr
        payload=Path(os.environ['RUNNER_TEMP'])/'stage3-tag.json';write(payload,{'ref':'refs/tags/'+TAG,'sha':sha})
        run(['gh','api','--method','POST',f'repos/{repo}/git/refs','--input',payload])
    mark('publish verified draft by fixed release ID')
    payload=Path(os.environ['RUNNER_TEMP'])/'stage3-publish.json';write(payload,{'draft':False,'prerelease':True,'body':body})
    run(['gh','api','--method','PATCH',f'repos/{repo}/releases/{release_id}','--input',payload])
    published=api('releases/'+str(release['id']))
    assert not published['draft'] and published['prerelease']
    assert api('git/ref/tags/'+TAG)['object']['sha']==sha
    for asset in published['assets']:
        mark('verify public HTTP download '+asset['name'])
        request=urllib.request.Request(asset['browser_download_url']+'?verified_asset='+str(asset['id']),headers={'Cache-Control':'no-cache'})
        with urllib.request.urlopen(request,timeout=180) as response:
            assert response.status==200;data=response.read()
        assert 'sha256:'+hashlib.sha256(data).hexdigest()==asset['digest'] and len(data)==asset['size']
    print(json.dumps({'release_url':published['html_url'],'source_commit':sha,'download_verified_assets':len(published['assets']),'status':'published and verified'},ensure_ascii=False))

if __name__=='__main__':
    try:main()
    except Exception as error:
        import traceback
        frame=traceback.extract_tb(error.__traceback__)[-1]
        print('::error title=Release validation::'+STAGE+'; '+type(error).__name__+' at '+Path(frame.filename).name+':'+str(frame.lineno)+'. No private output disclosed.',flush=True)
        raise SystemExit(1)
