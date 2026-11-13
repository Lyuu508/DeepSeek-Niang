#!/usr/bin/env python3
"""Configure only an existing private signing identity; never generate a keystore."""
import argparse
import os
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[1]
EXPECTED_CERT = '9de96ea5fb1d0b3f91ea73d9a16c030428453b15838927a85f86773eaef208a1'
REQUIRED = ['COMPANION_ANDROID_KEYSTORE_FILE', 'COMPANION_ANDROID_KEYSTORE_PASSWORD',
            'COMPANION_ANDROID_KEY_ALIAS', 'COMPANION_ANDROID_KEY_PASSWORD']


def preflight():
    missing = [name for name in REQUIRED if not os.environ.get(name)]
    if missing:
        raise SystemExit('Original Android signing identity required. Configure protected GitHub Secrets: ' + ', '.join(missing))
    if not Path(os.environ[REQUIRED[0]]).is_file():
        raise SystemExit('Configured keystore file is unavailable')
    # Inspect certificate only; no keystore/password/private material is printed.
    result = subprocess.run(['keytool', '-exportcert', '-keystore', os.environ[REQUIRED[0]],
                             '-alias', os.environ[REQUIRED[2]], '-storepass:env', REQUIRED[1]],
                            capture_output=True, check=True)
    import hashlib
    if hashlib.sha256(result.stdout).hexdigest() != EXPECTED_CERT:
        raise SystemExit('Keystore certificate does not match the actual v0.0.1 APK. Upgrade blocked; applicationId unchanged.')


def configure():
    preflight()
    path = ROOT / 'apps/companion/src-tauri/gen/android/app/build.gradle.kts'
    text = path.read_text()
    marker = '// DeepSeek protected signing (environment bindings only)'
    if marker in text:
        return
    # Generated local file only. Secret values stay in process environment.
    text += '''
// DeepSeek protected signing (environment bindings only)
android {
    signingConfigs {
        create("companionContinuous") {
            storeFile = file(System.getenv("COMPANION_ANDROID_KEYSTORE_FILE"))
            storePassword = System.getenv("COMPANION_ANDROID_KEYSTORE_PASSWORD")
            keyAlias = System.getenv("COMPANION_ANDROID_KEY_ALIAS")
            keyPassword = System.getenv("COMPANION_ANDROID_KEY_PASSWORD")
        }
    }
    buildTypes.getByName("release") { signingConfig = signingConfigs.getByName("companionContinuous") }
}
'''
    path.write_text(text)


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--configure', action='store_true')
    args = parser.parse_args()
    configure() if args.configure else preflight()
    print('Existing Android signing certificate matches v0.0.1; private values were not printed.')
