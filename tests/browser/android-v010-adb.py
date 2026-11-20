"""Exercise the public APKs with Android's own activity, filesystem and UI tools."""
import json
import os
import re
import subprocess
import time
import xml.etree.ElementTree as ET
from pathlib import Path


APP = 'io.local.deepseekcompanion.test2'
EMULATOR_LAUNCHER = 'com.google.android.apps.nexuslauncher'
OUTPUT = Path(os.environ['COMPANION_NATIVE_EVIDENCE'])
APKS = Path(os.environ['APK_DIRECTORY'])
MARKER = 'deepseek-v010-emulator-upgrade-fixture'
UI_DUMP = '/data/local/tmp/companion-v010-window.xml'
REPORT = {
    'scope': 'Android emulator using the public signed APKs; separate disposable app data.',
    'application_id': APP,
    'physical_device_tested': False,
    'screenshots': [],
    'checked_native_pages': [],
}


def adb(*args, timeout=35, binary=False):
    value = subprocess.check_output(['adb', *args], timeout=timeout, stderr=subprocess.STDOUT)
    return value if binary else value.decode().strip()


def save():
    (OUTPUT / 'adb-runtime-results.json').write_text(json.dumps(REPORT, indent=2, ensure_ascii=False) + '\n')


def tree():
    try:
        adb('shell', 'uiautomator', 'dump', UI_DUMP, timeout=12)
        value = adb('exec-out', 'cat', UI_DUMP, timeout=5)
        return ET.fromstring(value)
    except (ET.ParseError, subprocess.SubprocessError):
        return None


def labeled_node(root, labels):
    if root is None:
        return None
    for node in root.iter('node'):
        if node.get('package') != APP:
            continue
        strings = {node.get('text', '').strip(), node.get('content-desc', '').strip()}
        if strings.intersection(labels):
            return node
    return None


def tap(root, *labels):
    node = labeled_node(root, labels)
    if node is None:
        return False
    left, top, right, bottom = map(int, re.findall(r'\d+', node.get('bounds', '')))
    adb('shell', 'input', 'tap', str((left + right) // 2), str((top + bottom) // 2))
    time.sleep(0.65)
    return True


def snapshot(filename):
    image = adb('exec-out', 'screencap', '-p', binary=True)
    assert image.startswith(b'\x89PNG\r\n\x1a\n'), 'Android did not return a PNG screenshot'
    (OUTPUT / filename).write_bytes(image)
    REPORT['screenshots'].append({'name': filename, 'bytes': len(image)})
    save()


def foreground():
    window = adb('shell', 'dumpsys', 'window')
    activity = adb('shell', 'dumpsys', 'activity', 'activities')
    focus_lines = [line.strip() for line in (window + '\n' + activity).splitlines() if
                   any(key in line for key in ('mCurrentFocus=', 'mFocusedApp=', 'topResumedActivity=', 'mResumedActivity:'))]
    (OUTPUT / 'android-focus.txt').write_text('\n'.join(focus_lines) + '\n')
    if any(f'Application Not Responding: {EMULATOR_LAUNCHER}' in line for line in focus_lines):
        adb('shell', 'am', 'force-stop', EMULATOR_LAUNCHER)
        REPORT['emulator_launcher_dialog_dismissed'] = True
        return False
    return any(APP + '/' in line and 'mCurrentFocus=' in line for line in focus_lines)


def install(version, upgrade):
    args = ['install', '-r'] if upgrade else ['install']
    result = adb(*args, str(APKS / f'{version}.apk'), timeout=90)
    assert 'Success' in result, result
    info = adb('shell', 'dumpsys', 'package', APP)
    assert f'versionCode={1010 if upgrade else 1009} ' in info
    assert f'versionName={version}' in info
    activity = adb('shell', 'cmd', 'package', 'resolve-activity', '--brief', APP).splitlines()[-1]
    assert activity.startswith(APP + '/'), activity
    launch = adb('shell', 'am', 'start', '-W', '-n', activity)
    assert re.search(r'Status:\s*ok', launch), launch
    REPORT[f'v{version}'] = {'installed': True, 'launch_command_succeeded': True}
    save()
    REPORT[f'v{version}']['activity_foreground'] = False
    for _ in range(20):
        if foreground():
            REPORT[f'v{version}']['activity_foreground'] = True
            break
        time.sleep(1)
    time.sleep(2)
    snapshot(f'android-v{version}-screen.png')
    REPORT[f'v{version}']['sqlite_files'] = adb('shell', 'run-as', APP, 'find', '.', '-name', 'companion.sqlite3').splitlines()
    save()


def main():
    REPORT['android_sdk'] = adb('shell', 'getprop', 'ro.build.version.sdk')
    REPORT['abis'] = adb('shell', 'getprop', 'ro.product.cpu.abilist')
    install('0.0.9', False)
    adb('shell', 'am', 'force-stop', APP)
    # Place a fixture marker through the debug-only run-as command on the clean emulator.
    adb('shell', f'run-as {APP} sh -c "mkdir -p files; printf %s {MARKER} > files/{MARKER}"')
    install('0.0.10', True)
    assert adb('shell', 'run-as', APP, 'cat', f'files/{MARKER}') == MARKER
    REPORT['same_application_data_directory_preserved'] = True
    REPORT['sqlite_path_preserved'] = bool(REPORT['v0.0.9']['sqlite_files']) and REPORT['v0.0.9']['sqlite_files'] == REPORT['v0.0.10']['sqlite_files']
    save()
    root = tree()
    REPORT['native_accessibility_webview'] = root is not None and any(
        node.get('package') == APP and node.get('class') == 'android.webkit.WebView' for node in root.iter('node')
    )
    if REPORT['native_accessibility_webview']:
        REPORT['checked_native_pages'].append('chat')
        (OUTPUT / 'android-accessibility.xml').write_text(ET.tostring(root, encoding='unicode'))
    for key, labels, expected in [
        ('personas', ('手机人格', '人格'), ('人格管理', '新增人格')),
        ('settings', ('手机设置', '设置'), ('应用外观', '设置', '界面外观')),
    ]:
        if tap(root, *labels):
            root = tree()
            if labeled_node(root, expected) is not None:
                snapshot(f'android-native-{key}.png')
                REPORT['checked_native_pages'].append(key)
    assert foreground(), 'New Android version lost its foreground activity'
    REPORT['passed'] = True


try:
    main()
except Exception as error:
    REPORT['passed'] = False
    REPORT['issue'] = str(error)
    messages = adb('logcat', '-d', '-t', '1400', '-s', 'AndroidRuntime:E', 'ActivityTaskManager:I', 'WindowManager:W', 'libc:F', 'chromium:E')
    crash = adb('logcat', '-d', '-b', 'crash', '-t', '250')
    (OUTPUT / 'android-app-errors.txt').write_text(messages + '\n' + crash)
    raise
finally:
    save()
