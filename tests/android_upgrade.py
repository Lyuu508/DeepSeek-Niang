"""ADB acceptance for two verified production APKs on a disposable emulator."""
import json
from pathlib import Path
import re
import subprocess
import sys
import time
import xml.etree.ElementTree as ET

APP = 'io.local.deepseekcompanion'
DUMP = '/data/local/tmp/deepseek-upgrade.xml'


def adb(*command, timeout=30):
    return subprocess.check_output(['adb', *command], timeout=timeout)


def shell(*command, timeout=30):
    return adb('shell', *command, timeout=timeout).decode().strip()


def install_time(version_code, version_name):
    info = shell('dumpsys', 'package', APP)
    assert f'versionCode={version_code} ' in info and f'versionName={version_name}' in info
    first = re.search(r'firstInstallTime=(.+)', info)
    assert first and first[1].strip()
    return first[1].strip()


def chat_visible():
    try:
        shell('uiautomator', 'dump', DUMP, timeout=12)
        window = ET.fromstring(adb('exec-out', 'cat', DUMP, timeout=5))
    except (ET.ParseError, subprocess.SubprocessError):
        return False
    nodes = [node for node in window.iter('node') if node.get('package') == APP]
    labels = {node.get(key) for node in nodes for key in ('text', 'content-desc')}
    return (any(node.get('class') == 'android.webkit.WebView' for node in nodes)
            and bool(labels.intersection({'聊天', '手机聊天', '手机设置', 'DeepSeek娘'}))
            and 'OPERATION_FAILED' not in labels)


def main(original, candidate, evidence):
    details = json.loads((evidence / 'candidate.json').read_text())
    assert details['signature_verified'] and details['upgrade_compatible']
    report = {'source_commit': details['source_commit'], 'apk_sha256': details['sha256'],
              'application_id': APP, 'original_version': '0.0.1',
              'version': details['version'], 'version_code': details['version_code'],
              'signer_certificate_sha256': details['signer_certificate_sha256'],
              'android_sdk': shell('getprop', 'ro.build.version.sdk'),
              'device_kind': 'Android emulator', 'physical_device_tested': False,
              'upgrade_installed': False, 'webview_chat_visible': False}
    try:
        assert 'arm64-v8a' in shell('getprop', 'ro.product.cpu.abilist')
        assert b'Success' in adb('install', str(original), timeout=120)
        first = install_time(1001, '0.0.1')
        assert b'Success' in adb('install', '-r', str(candidate), timeout=120)
        assert install_time(details['version_code'], details['version']) == first
        report['upgrade_installed'] = True
        activity = shell('cmd', 'package', 'resolve-activity', '--brief', APP).splitlines()[-1]
        assert activity.startswith(APP + '/')
        assert re.search(r'Status:\s*ok', shell('am', 'start', '-W', '-n', activity, timeout=90))
        for _ in range(12):
            if chat_visible():
                report['webview_chat_visible'] = True
                break
            time.sleep(3)
        image = adb('exec-out', 'screencap', '-p')
        assert image.startswith(b'\x89PNG\r\n\x1a\n')
        (evidence / 'ANDROID-APP.png').write_bytes(image)
        assert report['webview_chat_visible'], 'Installed Android chat was not verified in WebView'
    finally:
        (evidence / 'ANDROID-UPGRADE.json').write_text(json.dumps(report, indent=2) + '\n')
        (evidence / 'android-errors.log').write_bytes(adb('logcat', '-d', '-b', 'crash', '-t', '200'))


if __name__ == '__main__':
    main(*map(Path, sys.argv[1:]))
