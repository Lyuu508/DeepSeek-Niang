#!/usr/bin/env bash
# Reusable dependency setup only. Never edits tracked source, manifests or lockfiles.
set -euo pipefail
cd /workspace/DeepSeek-Niang
cache=/workspace/.cache
mkdir -p "$cache" "$cache/apt/lists/partial" "$cache/apt/cache/archives/partial" "$cache/debs"
export CARGO_HOME="$cache/cargo" RUSTUP_HOME="$cache/rustup" PATH="$cache/cargo/bin:$PATH"
if ! command -v rustup >/dev/null; then
  curl -fsS --retry 3 https://sh.rustup.rs -o "$cache/rustup-init.sh"
  sh "$cache/rustup-init.sh" -y --no-modify-path --profile minimal --default-toolchain 1.99.0 --component clippy --component rustfmt
else
  rustup toolchain install 1.99.0 --profile minimal --component clippy --component rustfmt
  rustup default 1.99.0
fi
rustup target add aarch64-linux-android
npm ci --ignore-scripts --cache "$cache/npm"
cat > "$cache/apt/sources.list" <<'SOURCES'
deb [signed-by=/usr/share/keyrings/debian-archive-keyring.gpg] https://deb.debian.org/debian trixie main
deb [signed-by=/usr/share/keyrings/debian-archive-keyring.gpg] https://deb.debian.org/debian trixie-updates main
deb [signed-by=/usr/share/keyrings/debian-archive-keyring.gpg] https://security.debian.org/debian-security trixie-security main
SOURCES
apt_args=(-o Dir::Etc::Parts=- -o "Dir::Etc::sourcelist=$cache/apt/sources.list" -o Dir::Etc::sourceparts=- -o "Dir::State::lists=$cache/apt/lists" -o "Dir::Cache=$cache/apt/cache" -o Debug::NoLocking=true -o APT::Sandbox::User=agent)
apt-get "${apt_args[@]}" update
(cd "$cache/debs" && apt-get "${apt_args[@]}" download openjdk-21-jdk-headless openjdk-21-jre-headless sqlite3)
apt-get "${apt_args[@]}" --download-only --no-install-recommends -y install libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev patchelf
python3 - <<'PY'
from pathlib import Path
import shutil, subprocess, tempfile
cache=Path('/workspace/.cache')
# Extract complete JRE+JDK together. Java must report a writable local JDK home to Gradle.
with tempfile.TemporaryDirectory(dir=cache,prefix='cloud-jdk-') as folder:
    for package in ['openjdk-21-jre-headless','openjdk-21-jdk-headless']:
        candidates=sorted((cache/'debs').glob(package+'_*.deb'),key=lambda p:p.stat().st_mtime)
        subprocess.run(['dpkg-deb','-x',str(candidates[-1]),folder],check=True)
    target=cache/'java21'
    if target.exists(): shutil.rmtree(target)
    shutil.copytree(Path(folder)/'usr/lib/jvm/java-21-openjdk-amd64',target,symlinks=True)
with tempfile.TemporaryDirectory(dir=cache,prefix='cloud-sysroot-') as folder:
    root=Path(folder)
    for deb in (cache/'apt/cache/archives').glob('*.deb'):
        subprocess.run(['dpkg-deb','-x',str(deb),str(root)],check=True)
    for relative in ['usr/include','usr/lib/x86_64-linux-gnu/pkgconfig','usr/share/pkgconfig']:
        dest=root/relative;dest.mkdir(parents=True,exist_ok=True)
        for source in Path('/'+relative).iterdir():
            link=dest/source.name
            if not link.exists() and not link.is_symlink(): link.symlink_to(source)
    target=cache/'linux-sysroot'
    if target.exists(): shutil.rmtree(target)
    shutil.copytree(root,target,symlinks=True)
for deb in sorted((cache/'debs').glob('sqlite3_*.deb'),key=lambda p:p.stat().st_mtime)[-1:]:
    subprocess.run(['dpkg-deb','-x',str(deb),str(cache/'sqlite')],check=True)
PY
if [ ! -x "$cache/android-sdk/cmdline-tools/latest/bin/android" ]; then
  curl -fsS --retry 3 https://dl.google.com/android/repository/commandlinetools-linux-16111833_latest.zip -o "$cache/android-commandlinetools.zip"
  python3 - <<'PY'
from pathlib import Path
import hashlib,zipfile
cache=Path('/workspace/.cache');archive=cache/'android-commandlinetools.zip'
assert hashlib.sha256(archive.read_bytes()).hexdigest()=='0877a1d048fe4a24efe2eff536ca4223f7adeb58648bb81909d33c446918cfa8'
root=cache/'android-sdk/cmdline-tools';root.mkdir(parents=True,exist_ok=True)
with zipfile.ZipFile(archive) as package: package.extractall(root/'staging')
(root/'staging/cmdline-tools').rename(root/'latest')
for executable in (root/'latest/bin').iterdir(): executable.chmod(0o755)
PY
fi
for package in 'platforms;android-36' 'build-tools;36.0.0' 'platform-tools' 'ndk;28.2.13676358'; do
  python3 scripts/cloud_env.py "$cache/android-sdk/cmdline-tools/latest/bin/android" --no-metrics --sdk="$cache/android-sdk" sdk install "$package"
done
python3 scripts/cloud_env.py cargo fetch --locked
python3 scripts/cloud_env.py javac -version
python3 scripts/cloud_env.py pkg-config --modversion webkit2gtk-4.1 ayatana-appindicator3-0.1
python3 scripts/cloud_env.py python3 scripts/check_environment.py
