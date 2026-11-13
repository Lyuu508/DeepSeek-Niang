#!/usr/bin/env sh
set -eu
project_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
for tool in node npm cargo rustup python3; do command -v "$tool" >/dev/null || { printf 'Missing prerequisite: %s\n' "$tool" >&2; exit 1; }; done
test -n "${JAVA_HOME:-}" && test -x "$JAVA_HOME/bin/javac" || { printf '%s\n' 'JAVA_HOME must point to a complete JDK, not a JRE.' >&2; exit 1; }
test -n "${ANDROID_HOME:-}" && test -d "$ANDROID_HOME" || { printf '%s\n' 'Android SDK missing: install platform 36, build/platform tools; set ANDROID_HOME and accept SDK licenses in Android Studio.' >&2; exit 1; }
test -n "${NDK_HOME:-}" && test -d "$NDK_HOME/toolchains/llvm" || { printf '%s\n' 'Android NDK missing: install an official side-by-side NDK and set NDK_HOME to its exact directory.' >&2; exit 1; }
if test "${1:-}" = '--check-only'; then printf '%s\n' 'Prerequisite paths exist; APK/device validation still required.'; exit 0; fi
cd "$project_root"
if test "${1:-}" != '--unsigned-ci'; then python3 scripts/android_signing.py; fi
node scripts/check-branding.mjs
rustup target add aarch64-linux-android
npm ci --ignore-scripts
cargo test --workspace --locked
npm run tauri -- android init --ci
python3 scripts/prepare_android.py
if test "${1:-}" = '--unsigned-ci'; then
    npm run tauri -- android build --apk --target aarch64
else
    python3 scripts/android_signing.py --configure
    npm run tauri -- android build --apk --target aarch64
    python3 scripts/verify_android.py
fi
python3 - <<'PY'
from pathlib import Path
import hashlib
packages = list(Path('apps/companion/src-tauri/gen/android/app/build/outputs/apk').rglob('*.apk'))
if not packages:
    raise SystemExit('Build returned without an APK; no installation success is claimed.')
for package in packages:
    print(hashlib.sha256(package.read_bytes()).hexdigest(), package)
print('APK files exist; installation, Keystore and real API tests require a device.')
PY
