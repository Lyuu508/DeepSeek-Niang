#!/usr/bin/env sh
set -eu
project_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
for tool in node npm cargo rustup python3; do command -v "$tool" >/dev/null || { printf 'Missing prerequisite: %s\n' "$tool" >&2; exit 1; }; done
test -n "${JAVA_HOME:-}" && test -x "$JAVA_HOME/bin/javac" || { printf '%s\n' 'JAVA_HOME must point to a complete JDK, not a JRE.' >&2; exit 1; }
test -n "${ANDROID_HOME:-}" && test -d "$ANDROID_HOME" || { printf '%s\n' 'Android SDK missing: install platform 36, build/platform tools; set ANDROID_HOME and accept SDK licenses in Android Studio.' >&2; exit 1; }
test -n "${NDK_HOME:-}" && test -d "$NDK_HOME/toolchains/llvm" || { printf '%s\n' 'Android NDK missing: install an official side-by-side NDK and set NDK_HOME to its exact directory.' >&2; exit 1; }
if test "${1:-}" = '--check-only'; then printf '%s\n' 'Prerequisite paths exist; APK/device validation still required.'; exit 0; fi
cd "$project_root"
mode="${1:-signed}"
unset VITE_COMPANION_TEST_PACKAGE
case "$mode" in
  signed) python3 scripts/android_signing.py ;;
  --unsigned-ci) ;;
  --test-package)
    export VITE_COMPANION_TEST_PACKAGE=1
    export CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_DEV_STRIP=debuginfo
    ;;
  *) printf 'Unknown build mode: %s\n' "$mode" >&2; exit 1 ;;
esac
node scripts/check-branding.mjs
rustup target add aarch64-linux-android
npm ci --ignore-scripts
cargo test --workspace --locked
npm run tauri -- android init --ci
python3 scripts/prepare_android.py
if test "$mode" = '--test-package'; then
    python3 scripts/android_test_signing.py
    npm run tauri -- android build --apk --debug --target aarch64
    python3 scripts/verify_android.py --test-package
elif test "$mode" = '--unsigned-ci'; then
    npm run tauri -- android build --apk --target aarch64
else
    python3 scripts/android_signing.py --configure
    npm run tauri -- android build --apk --target aarch64
    python3 scripts/verify_android.py
fi
python3 - "$mode" <<'PY'
from pathlib import Path
import hashlib
import sys
variant = 'debug' if sys.argv[1] == '--test-package' else 'release'
packages = [p for p in Path('apps/companion/src-tauri/gen/android/app/build/outputs/apk').rglob('*.apk') if p.parent.name == variant]
if len(packages) != 1:
    raise SystemExit('Expected exactly one APK for the requested variant; no installation success is claimed.')
for package in packages:
    print(hashlib.sha256(package.read_bytes()).hexdigest(), package)
print('APK files exist; installation, Keystore and real API tests require a device.')
if sys.argv[1] == '--test-package':
    print('Isolated .test applicationId with a development signer; cannot replace the original app. Test signer continuity is not guaranteed across runners.')
PY
