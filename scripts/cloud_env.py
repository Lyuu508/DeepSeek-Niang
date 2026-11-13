#!/usr/bin/env python3
"""Activate retained Cloud tools for a command, without changing HOME or printing secrets."""
import os
from pathlib import Path
import subprocess
import sys
from urllib.parse import urlsplit


def environment():
    env = os.environ.copy()
    cache = Path('/workspace/.cache')
    env.update(CARGO_HOME=str(cache/'cargo'), RUSTUP_HOME=str(cache/'rustup'), CARGO_BUILD_JOBS='4',
               JAVA_HOME=str(cache/'java21'), ANDROID_HOME=str(cache/'android-sdk'),
               ANDROID_USER_HOME=str(cache/'android-user'), GRADLE_USER_HOME=str(cache/'gradle'),
               NDK_HOME=str(cache/'android-sdk/ndk/28.2.13676358'),
               XDG_CACHE_HOME=str(cache), XDG_DATA_HOME=str(cache/'data'), XDG_CONFIG_HOME=str(cache/'config'))
    env['PATH'] = str(cache/'cargo/bin')+':'+str(cache/'java21/bin')+':'+str(cache/'sqlite/usr/bin')+':'+env['PATH']
    env['COMPANION_CHROMIUM_EXECUTABLE'] = '/usr/bin/chromium'
    ndk=Path(env['NDK_HOME'])/'toolchains/llvm/prebuilt/linux-x86_64/bin'
    if ndk.is_dir():
        env['CC_aarch64_linux_android']=str(ndk/'aarch64-linux-android29-clang')
        env['AR_aarch64_linux_android']=str(ndk/'llvm-ar')
        env['CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER']=str(ndk/'aarch64-linux-android29-clang')
    proxy = urlsplit(env.get('HTTPS_PROXY', env.get('https_proxy', '')))
    if proxy.hostname:
        # Java does not honor HTTP(S)_PROXY automatically. Only route metadata is copied.
        options = f'-Dhttps.proxyHost={proxy.hostname} -Dhttps.proxyPort={proxy.port or 80} -Dhttp.proxyHost={proxy.hostname} -Dhttp.proxyPort={proxy.port or 80} -Dhttp.nonProxyHosts=localhost|127.*'
        env['JAVA_OPTS'] = env.get('JAVA_OPTS', '')+' '+options
        env['GRADLE_OPTS'] = env.get('GRADLE_OPTS', '')+' '+options
    sysroot=cache/'linux-sysroot'
    if (sysroot/'usr/lib/x86_64-linux-gnu/pkgconfig').is_dir():
        env['PKG_CONFIG_SYSROOT_DIR']=str(sysroot)
        env['PKG_CONFIG_PATH']=str(sysroot/'usr/lib/x86_64-linux-gnu/pkgconfig')+':'+str(sysroot/'usr/share/pkgconfig')
        env['LIBRARY_PATH']=str(sysroot/'usr/lib/x86_64-linux-gnu')
        env['LD_LIBRARY_PATH']=str(sysroot/'usr/lib/x86_64-linux-gnu')
    return env


if __name__ == '__main__':
    if len(sys.argv)<2:
        raise SystemExit('Usage: python3 scripts/cloud_env.py <command> [args...]')
    raise SystemExit(subprocess.run(sys.argv[1:],env=environment()).returncode)
