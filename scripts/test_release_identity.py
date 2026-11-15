"""Persist new test identities before signing; never publish a disposable key."""
import base64
import hashlib
import json
import os
from pathlib import Path
import secrets
import re
import subprocess
import sys
import urllib.request

ROOT = Path(__file__).resolve().parents[1]
APP_ID = 'io.local.deepseekcompanion.test2'
BUNDLE_SECRET = 'TEST_RELEASE_SIGNING_B64'


def run(args, env=None, value=None):
    result = subprocess.run(args, input=value, env=env, capture_output=True, timeout=180)
    if result.returncode:
        stage=re.search(rb'CERTIFICATE_STAGE_ERROR:(Generate|Check|Sign|Verify):(\d+):([A-Za-z.]+)',result.stdout)
        if stage:
            raise RuntimeError('Windows certificate '+stage[1].decode()+' failed at line '+stage[2].decode()+' ('+stage[3].decode()+')')
        if Path(args[0]).name == 'gh' and b'403' in result.stderr:
            raise RuntimeError('GitHub denied the requested repository Secrets operation; no private output was logged')
        raise RuntimeError('Signing operation failed: ' + Path(args[0]).name)
    return result.stdout


def load_bundle(value):
    bundle = json.loads(base64.b64decode(value, validate=True))
    assert bundle['format'] == 1 and bundle['android_application_id'] == APP_ID
    assert all(bundle.get(key) for key in ['android_keystore','android_password','android_alias','windows_pfx','windows_password'])
    return bundle


def store_secret(name, value, token, repo):
    context = os.environ.copy()
    context['GH_TOKEN'] = token
    run(['gh','secret','set',name,'--repo',repo], env=context, value=value.encode())


def main():
    assert os.name == 'nt' and os.environ.get('GITHUB_ACTIONS') == 'true'
    repo = os.environ['GH_REPO']
    directory = Path(os.environ['RUNNER_TEMP']) / 'deepseek-protected-signing'
    directory.mkdir(exist_ok=True)
    android = directory / 'android-test.p12'
    pfx = directory / 'windows-test.pfx'
    cer = directory / 'windows-test.cer'
    remote = json.loads(run(['gh','api',f'repos/{repo}/releases/tags/v0.0.2']))
    prior = next((a for a in remote['assets'] if a['name']=='SIGNING-IDENTITIES.json'),None)
    published = None
    if prior:
        with urllib.request.urlopen(prior['browser_download_url'],timeout=60) as response:
            published = json.load(response)
    raw = os.environ.get(BUNDLE_SECRET,'')
    bundle = load_bundle(raw) if raw else None
    token = os.environ.get('SIGNING_SECRETS_TOKEN','')
    if not bundle:
        assert published is None, 'Published signing identity exists; never regenerate a missing key'
        if not token:
            raise SystemExit('Configure SIGNING_SECRETS_TOKEN with repository Secrets read/write; no key has been generated.')
    if not bundle or not bundle.get('provisioning_complete'):
        assert token, 'A complete protected signing bundle must be saved before publication'
        context = os.environ.copy();context['GH_TOKEN'] = token
        run(['gh','api',f'repos/{repo}/actions/secrets/public-key'],env=context)
    if not bundle:
        bundle = {'format':1,'android_application_id':APP_ID,'android_password':secrets.token_urlsafe(48),
                  'android_alias':'deepseek-test','windows_password':secrets.token_urlsafe(48),'provisioning_complete':False}
    context = os.environ.copy()
    context.update(TEST_ANDROID_PASSWORD=bundle['android_password'],TEST_WINDOWS_PASSWORD=bundle['windows_password'],
                   TEST_WINDOWS_PFX=str(pfx),TEST_WINDOWS_CER=str(cer))
    keytool = str(Path(os.environ['JAVA_HOME'])/'bin/keytool.exe')
    common = ['-keystore',str(android),'-alias',bundle['android_alias'],'-storepass:env','TEST_ANDROID_PASSWORD']
    if raw:
        android.write_bytes(base64.b64decode(bundle['android_keystore'],validate=True))
        pfx.write_bytes(base64.b64decode(bundle['windows_pfx'],validate=True))
        run(['pwsh','-NoProfile','-File',str(ROOT/'.github/scripts/windows-test-sign.ps1'),'-Action','Check'],env=context)
    else:
        run([keytool,'-genkeypair',*common,'-keypass:env','TEST_ANDROID_PASSWORD','-storetype','PKCS12','-keyalg','RSA','-keysize','3072','-validity','10000','-dname','CN=DeepSeek Niang QR Test,O=Development Test'],env=context)
        run(['pwsh','-NoProfile','-File',str(ROOT/'.github/scripts/windows-test-sign.ps1'),'-Action','Generate'],env=context)
        bundle['android_keystore'] = base64.b64encode(android.read_bytes()).decode()
        bundle['windows_pfx'] = base64.b64encode(pfx.read_bytes()).decode()
    certificate = run([keytool,'-exportcert',*common],env=context)
    run([keytool,'-certreq',*common,'-keypass:env','TEST_ANDROID_PASSWORD'],env=context)
    public = {'android_application_id':APP_ID,'android_certificate_sha256':hashlib.sha256(certificate).hexdigest(),
              'windows_certificate_sha256':hashlib.sha256(cer.read_bytes()).hexdigest(),
              'windows_certificate_kind':'self-signed test; not a public-CA trusted publisher',
              'secret_bundle':BUNDLE_SECRET,'private_material_published':False,'old_android_data_preserved':True,
              'can_replace_old_test_apk':False,'can_replace_production_apk':False}
    if published:
        for key in ['android_application_id','android_certificate_sha256','windows_certificate_sha256']:
            assert public[key] == published[key], 'Persisted signing identity differs from the published identity'
    if not bundle.get('provisioning_complete'):
        # One encrypted repository secret holds the COMPLETE identity atomically.
        # A failed later compatibility-secret write can resume from this bundle.
        store_secret(BUNDLE_SECRET,base64.b64encode(json.dumps(bundle).encode()).decode(),token,repo)
        values={'ANDROID_TEST_KEYSTORE_B64':bundle['android_keystore'],'ANDROID_TEST_KEYSTORE_PASSWORD':bundle['android_password'],
                'ANDROID_TEST_KEY_ALIAS':bundle['android_alias'],'ANDROID_TEST_KEY_PASSWORD':bundle['android_password'],
                'WINDOWS_TEST_PFX_B64':bundle['windows_pfx'],'WINDOWS_TEST_PFX_PASSWORD':bundle['windows_password']}
        for name,value in values.items():store_secret(name,value,token,repo)
        bundle['provisioning_complete'] = True
        store_secret(BUNDLE_SECRET,base64.b64encode(json.dumps(bundle).encode()).decode(),token,repo)
    (directory/'public.json').write_text(json.dumps(public,indent=2)+'\n')
    # This private file stays only in RUNNER_TEMP, never in artifacts or source.
    (directory/'runtime.json').write_text(json.dumps(bundle))
    print('Test signing identities verified and persisted. No private values printed.')


if __name__ == '__main__':
    try:
        main()
    except Exception as error:
        import traceback
        frame=traceback.extract_tb(error.__traceback__)[-1]
        detail=str(error) if isinstance(error,RuntimeError) else type(error).__name__
        print(f'::error title=Signing identity setup::Line {frame.lineno}: {detail}',flush=True)
        raise
