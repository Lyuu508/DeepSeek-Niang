#!/usr/bin/env python3
"""Create/reuse only an isolated development signer, never a production identity."""
import hashlib
import os
from pathlib import Path
import subprocess

from android_signing import EXPECTED_CERT

ROOT = Path(__file__).resolve().parents[1]


def prepare():
    directory = ROOT / 'apps/companion/src-tauri/gen/android/.test-signing'
    directory.mkdir(mode=0o700, parents=True, exist_ok=True)
    path = directory / 'debug.keystore'
    keytool = Path(os.environ['JAVA_HOME']) / 'bin' / ('keytool.exe' if os.name == 'nt' else 'keytool')
    context = os.environ.copy()
    # Public Android debug convention, not a recovered/user/production password.
    # Keep it out of command arguments and command output nonetheless.
    context['COMPANION_TEST_DEBUG_PASSWORD'] = 'android'
    access = ['-keystore', str(path), '-alias', 'androiddebugkey',
              '-storepass:env', 'COMPANION_TEST_DEBUG_PASSWORD']
    if not path.exists():
        result = subprocess.run([str(keytool), '-genkeypair', *access,
                                 '-keypass:env', 'COMPANION_TEST_DEBUG_PASSWORD',
                                 '-storetype', 'JKS', '-keyalg', 'RSA', '-keysize', '2048',
                                 '-validity', '10000', '-dname',
                                 'CN=DeepSeek Niang Isolated Test,O=Development Test,C=US'],
                                env=context, capture_output=True)
        if result.returncode:
            raise SystemExit('Cannot create isolated development signer; no production key was used.')
        path.chmod(0o600)
    certificate = subprocess.run([str(keytool), '-exportcert', *access],
                                 env=context, capture_output=True)
    if certificate.returncode or not certificate.stdout:
        raise SystemExit('Existing development signer cannot be verified; it was not replaced.')
    if hashlib.sha256(certificate.stdout).hexdigest() == EXPECTED_CERT:
        raise SystemExit('Original upgrade signer must not be used for the isolated test package.')
    # A successful CSR operation exercises the private key without exporting it.
    proof = subprocess.run([str(keytool), '-certreq', *access,
                            '-keypass:env', 'COMPANION_TEST_DEBUG_PASSWORD'],
                           env=context, capture_output=True)
    if proof.returncode or not proof.stdout:
        raise SystemExit('Development private key is unusable; signing stopped.')
    print('Isolated development signer verified; original upgrade identity was not used.')


if __name__ == '__main__':
    prepare()
