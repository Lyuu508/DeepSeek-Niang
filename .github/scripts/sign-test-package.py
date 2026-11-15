"""Sign a tested package with the EXISTING test2 identity. Never generates a key."""
import argparse
import base64
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
import verify_android
import version
from test_release_identity import existing_bundle

VERSION='0.0.3'
APP_ID='io.local.deepseekcompanion.test2'

def run(args,env=None,**kwargs):
    result=subprocess.run(list(map(str,args)),env=env,capture_output=True,timeout=300,**kwargs)
    if result.returncode:raise RuntimeError('Package verification command failed: '+Path(str(args[0])).name)
    return result.stdout

def read(path):return json.loads(path.read_text(encoding='utf-8-sig'))
def write(path,value):path.write_text(json.dumps(value,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
def digest(path):return hashlib.sha256(path.read_bytes()).hexdigest()
def api(path):return json.loads(run(['gh','api',f"repos/{os.environ['GH_REPO']}/{path}"]))
def reference(tag,name,destination):
    release=api('releases/tags/'+tag)
    asset=next(a for a in release['assets'] if a['name']==name)
    with urllib.request.urlopen(asset['browser_download_url']+'?asset='+str(asset['id']),timeout=120) as response:
        assert response.status==200
        destination.write_bytes(response.read())
    assert asset['digest']=='sha256:'+digest(destination) and asset['size']==destination.stat().st_size
    return destination

def public_identity(directory):
    identity=read(reference('v0.0.2','SIGNING-IDENTITIES.json',directory/'reference-identities.json'))
    assert identity['android_application_id']==APP_ID
    return identity

def base_report(source,platform):
    sha=run(['git','rev-parse','HEAD'],cwd=ROOT).decode().strip()
    assert sha==os.environ['EXPECTED_SOURCE_SHA'] and re.fullmatch('[0-9a-f]{40}',sha)
    assert not run(['git','status','--porcelain'],cwd=ROOT).strip(),'Source must remain clean'
    spec=version.check()
    assert spec=={'version':VERSION,'androidVersionCode':1003}
    report=read(source/(platform+'-build.json'))
    assert report['source_commit']==sha and report['version']==VERSION and report['working_tree_clean']
    package=source/report['file']
    assert package.is_file() and digest(package)==report['sha256']
    return sha,report,package

def windows(source,output):
    assert os.name=='nt' and os.environ.get('RUNNER_ENVIRONMENT')=='github-hosted'
    sha,report,original=base_report(source,'windows')
    assert original.read_bytes().startswith(b'MZ')
    protected=Path(os.environ['RUNNER_TEMP'])/'deepseek-protected-signing'
    bundle=read(protected/'runtime.json');public=read(protected/'public.json')
    assert bundle['provisioning_complete'] and bundle['android_application_id']==APP_ID
    assert report['hosted_runner_install_smoke']
    old=read(source/'BUILD-RESULTS-WINDOWS.json')
    assert old['build_commit']==sha and old['installed_process_started'] and old['installer_language_verified']
    exe=output/f'DeepSeek-Niang_{VERSION}_windows-x64-setup.exe'
    shutil.copyfile(original,exe)
    context=os.environ.copy()
    context.update(TEST_WINDOWS_INSTALLER=str(exe),TEST_WINDOWS_PASSWORD=bundle['windows_password'],
        TEST_WINDOWS_PFX=str(protected/'windows-test.pfx'),TEST_WINDOWS_CER=str(protected/'windows-test.cer'))
    run(['pwsh','-NoProfile','-File',ROOT/'.github/scripts/windows-test-sign.ps1','-Action','Sign'],env=context)
    assert digest(protected/'windows-test.cer')==public['windows_certificate_sha256']
    target=Path(os.environ['RUNNER_TEMP'])/'stage3-signed-install-target'
    native=target/'x86_64-pc-windows-msvc/release';(native/'bundle/nsis').mkdir(parents=True,exist_ok=True)
    shutil.copyfile(exe,native/'bundle/nsis/setup.exe')
    shutil.copyfile(source/f'DeepSeek-Niang_{VERSION}_windows-x64.exe',native/'companion-app.exe')
    context.update(CARGO_TARGET_DIR=str(target),TEST_SMOKE_SCRIPT=str(ROOT/'.github/scripts/windows-package-smoke.ps1'),
        TEST_SMOKE_LOG=str(output/'signed-windows-install.log'))
    wrapper="$ErrorActionPreference='Stop'; Start-Transcript -LiteralPath $env:TEST_SMOKE_LOG | Out-Null; try { & $env:TEST_SMOKE_SCRIPT } finally { Stop-Transcript | Out-Null }"
    smoke=subprocess.run(['pwsh','-NoProfile','-Command',wrapper],cwd=ROOT,env=context,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL,timeout=300)
    assert smoke.returncode==0,'Signed Windows installation/language/startup failed; see retained log'
    proof=read(Path(os.environ['RUNNER_TEMP'])/'deepseek-windows-artifacts/BUILD-RESULTS-WINDOWS.json')
    assert proof['build_commit']==sha and proof['installer_sha256']==digest(exe)
    assert proof['installed_on_windows_runner'] and proof['installed_process_started'] and proof['installer_language_verified']
    assert proof['window_title']=='DeepSeek娘'
    proof.update(authenticode_signed=True,authenticode_verified_on_disposable_runner=True,publicly_trusted_publisher=False,
        signer_certificate_sha256=public['windows_certificate_sha256'])
    write(output/'BUILD-RESULTS-WINDOWS.json',proof)
    report.update(sha256=digest(exe),bytes=exe.stat().st_size,file=exe.name,unsigned_build_sha256=report['sha256'],signing=proof)
    write(output/'windows-build.json',report)
    public.update(current_test2_update_identity_stable=True,production_upgrade_compatible=False)
    write(output/'SIGNING-IDENTITIES.json',public)
    shutil.copyfile(protected/'windows-test.cer',output/'WINDOWS-TEST-SIGNER.cer')
    shutil.copyfile(source/'windows-native-build.log',output/'windows-build.log')
    for name in ['windows-installer-language.json','windows-installed-window.png']:
        path=Path(os.environ['RUNNER_TEMP'])/'deepseek-windows-artifacts'/name
        if path.is_file():shutil.copyfile(path,output/name)

def android(source,output):
    sha,raw,apk=base_report(source,'android')
    unsigned=verify_android.verify(apk,False,True,True)
    assert unsigned['source_commit']==sha and unsigned['signature_absence_checked']
    assert unsigned['application_id']==APP_ID and unsigned['working_tree_clean']
    bundle=existing_bundle(os.environ.get('TEST_RELEASE_SIGNING_B64',''))
    assert bundle['provisioning_complete'],'Incomplete signing bundle; no key generation permitted'
    temp=Path(os.environ['RUNNER_TEMP'])/'stage3-android-signing'
    temp.mkdir(mode=0o700,exist_ok=True)
    keystore=temp/'existing-test2.p12'
    try:
        keystore.write_bytes(base64.b64decode(bundle['android_keystore'],validate=True));keystore.chmod(0o600)
        context=os.environ.copy();context['EXISTING_TEST_PASSWORD']=bundle['android_password']
        public=public_identity(temp)
        common=['-keystore',keystore,'-alias',bundle['android_alias'],'-storepass:env','EXISTING_TEST_PASSWORD']
        keytool=Path(os.environ['JAVA_HOME'])/'bin'/('keytool.exe' if os.name=='nt' else 'keytool')
        certificate=run([keytool,'-exportcert',*common],env=context)
        run([keytool,'-certreq',*common,'-keypass:env','EXISTING_TEST_PASSWORD'],env=context)
        assert hashlib.sha256(certificate).hexdigest()==public['android_certificate_sha256']
        tools=Path(os.environ['ANDROID_HOME'])/'build-tools/36.0.0'
        reference_apk=reference('v0.0.2','DeepSeek-Niang_0.0.2_android-arm64-isolated-test.apk',temp/'reference-test2.apk')
        signature=run(verify_android.sdk_command(tools,'apksigner','verify','--verbose','--print-certs',str(reference_apk))).decode()
        reference_cert=re.findall(r'certificate SHA-256 digest: ([0-9a-f]+)',signature)
        assert reference_cert==[public['android_certificate_sha256']]
        badging=run(verify_android.sdk_command(tools,'aapt2','dump','badging',str(reference_apk))).decode()
        assert "package: name='"+APP_ID+"' versionCode='1002' versionName='0.0.2'" in badging
        original=reference('v0.0.1','DeepSeek-Niang_0.0.1_android-arm64.apk',temp/'reference-production.apk')
        production=verify_android.verify_reference(original)
        assert production['signer_certificate_sha256']!=reference_cert[0]
        write(output/'ANDROID-SIGNING-REFERENCES.json',{'production':production,'previous_test2':{
            'application_id':APP_ID,'version':'0.0.2','version_code':1002,'sha256':digest(reference_apk),
            'signer_certificate_sha256':reference_cert[0],'apksigner_output':signature}})
        aligned=temp/'aligned.apk';signed=output/f'DeepSeek-Niang_{VERSION}_android-arm64-isolated-test.apk'
        run(verify_android.sdk_command(tools,'zipalign','-f','-P','16','4',str(apk),str(aligned)))
        run(verify_android.sdk_command(tools,'apksigner','sign','--ks',str(keystore),'--ks-key-alias',bundle['android_alias'],
            '--ks-pass','env:EXISTING_TEST_PASSWORD','--key-pass','env:EXISTING_TEST_PASSWORD','--v4-signing-enabled','false','--out',str(signed),str(aligned)),env=context)
        result=verify_android.verify(signed,True,True,True)
        assert result['source_commit']==sha and result['working_tree_clean'] and result['signer_certificate_sha256']==reference_cert[0]
        result.update(unsigned_build_sha256=raw['sha256'],test_signer_persistence_guaranteed=True,
            signature_source='existing GitHub encrypted TEST_RELEASE_SIGNING_B64',current_test2_update_identity_stable=True,
            upgrade_compatible_with_test2_v0_0_2=True,physical_upgrade_tested=False,production_upgrade_compatible=False,
            certificate_matches_v0_0_1=False)
        write(output/'android-build.json',result);write(output/'android-reverified.json',result)
        (output/'apksigner-0.0.3.txt').write_bytes(run(verify_android.sdk_command(tools,'apksigner','verify','--verbose','--print-certs',str(signed))))
        public.update(current_test2_update_identity_stable=True,production_upgrade_compatible=False)
        write(output/'SIGNING-IDENTITIES.json',public)
        for name in ['android-build.log','android-toolchain.log']:shutil.copyfile(source/name,output/name)
    finally:
        keystore.unlink(missing_ok=True)
        shutil.rmtree(temp,ignore_errors=True)

def main():
    parser=argparse.ArgumentParser();parser.add_argument('platform',choices=['windows','android']);parser.add_argument('artifacts',type=Path);parser.add_argument('output',type=Path)
    args=parser.parse_args()
    assert os.environ.get('GITHUB_ACTIONS')=='true','Signing is limited to the authorized Actions workflow'
    args.output.mkdir(parents=True,exist_ok=True)
    (windows if args.platform=='windows' else android)(args.artifacts.resolve(),args.output.resolve())
    assert not any(p.suffix in ['.p12','.pfx','.jks','.keystore','.key'] for p in args.output.rglob('*'))
    print(args.platform+' persisted signature and package verification passed. No private values printed.')

if __name__=='__main__':
    try:main()
    except Exception as error:
        # No subprocess output, environment variables or bundle values in diagnostics.
        import traceback
        line=traceback.extract_tb(error.__traceback__)[-1].lineno
        print('::error title=Persisted package signing::'+type(error).__name__+' at line '+str(line)+': package verification failed; private output withheld.',flush=True)
        raise SystemExit(1)
