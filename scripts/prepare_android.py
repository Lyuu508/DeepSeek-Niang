#!/usr/bin/env python3
"""Apply reproducible app policy to Tauri's generated Android manifest. No SDK installation."""
import argparse
import json
import re
import shutil
from pathlib import Path
import xml.etree.ElementTree as ET

ANDROID = "http://schemas.android.com/apk/res/android"
ET.register_namespace("android", ANDROID)
ET.register_namespace("tools", "http://schemas.android.com/tools")


def prepare(project: Path, branding: Path | None = None) -> None:
    manifest = project / "app/src/main/AndroidManifest.xml"
    if not manifest.is_file():
        raise SystemExit("Generated Android project missing. Run tauri android init with a valid SDK/NDK first.")
    tree = ET.parse(manifest)
    root = tree.getroot()
    name = f"{{{ANDROID}}}name"
    for permission in ["android.permission.INTERNET", "android.permission.ACCESS_NETWORK_STATE", "android.permission.ACCESS_LOCAL_NETWORK", "android.permission.CAMERA"]:
        if not any(p.get(name) == permission for p in root.findall("uses-permission")):
            ET.SubElement(root, "uses-permission", {name: permission})
    if not any(feature.get(name) == "android.hardware.camera.any" for feature in root.findall("uses-feature")):
        ET.SubElement(root, "uses-feature", {name: "android.hardware.camera.any", f"{{{ANDROID}}}required": "false"})
    application = root.find("application")
    if application is None:
        raise SystemExit("Application element missing; refusing to rewrite an unknown manifest.")
    for key, value in {"usesCleartextTraffic": "false", "allowBackup": "false",
                       "fullBackupContent": "@xml/companion_backup_rules",
                       "dataExtractionRules": "@xml/companion_data_extraction_rules"}.items():
        application.set(f"{{{ANDROID}}}{key}", value)
    for activity in application.findall("activity"):
        activity.set(f"{{{ANDROID}}}windowSoftInputMode", "adjustResize")
    application.set(f"{{{ANDROID}}}label", "@string/app_name")
    application.set(f"{{{ANDROID}}}icon", "@mipmap/ic_launcher")
    application.set(f"{{{ANDROID}}}roundIcon", "@mipmap/ic_launcher_round")
    for activity in application.findall("activity"):
        if activity.get(name) == ".MainActivity":
            activity.set(f"{{{ANDROID}}}label", "@string/main_activity_title")
    ET.indent(tree, space="    ")
    tree.write(manifest, encoding="utf-8", xml_declaration=True)
    res = project / "app/src/main/res"
    strings_file = res / "values/strings.xml"
    strings_file.parent.mkdir(parents=True, exist_ok=True)
    strings = ET.parse(strings_file).getroot() if strings_file.is_file() else ET.Element("resources")
    for key in ["app_name", "main_activity_title"]:
        entry = strings.find(f"string[@name='{key}']")
        if entry is None:
            entry = ET.SubElement(strings, "string", {"name": key})
        entry.text = "DeepSeek娘"
    ET.indent(strings, space="    ")
    ET.ElementTree(strings).write(strings_file, encoding="utf-8", xml_declaration=True)
    # Debug APKs are always side-by-side test installations. The release ID,
    # namespace, production resources and original signing identity stay intact.
    debug_strings = project / "app/src/debug/res/values/companion_test.xml"
    debug_strings.parent.mkdir(parents=True, exist_ok=True)
    debug_root = ET.Element("resources")
    for key in ["app_name", "main_activity_title"]:
        ET.SubElement(debug_root, "string", {"name": key}).text = "DeepSeek娘 测试版"
    ET.indent(debug_root, space="    ")
    ET.ElementTree(debug_root).write(debug_strings, encoding="utf-8", xml_declaration=True)
    branding = branding or Path(__file__).resolve().parents[1] / "assets/branding"
    brand_manifest = branding / "manifest.json"
    if brand_manifest.is_file() and json.loads(brand_manifest.read_text(encoding="utf-8")).get("status") == "ready":
        shutil.copytree(branding / "android", res, dirs_exist_ok=True)
        for qualifier in ["values", "values-night", "values-v31", "values-night-v31"]:
            themes_file = res / qualifier / "themes.xml"
            themes_file.parent.mkdir(parents=True, exist_ok=True)
            themes = ET.parse(themes_file).getroot() if themes_file.is_file() else ET.Element("resources")
            theme = themes.find("style[@name='Theme.companion_app']")
            if theme is None:
                theme = ET.SubElement(themes, "style", {"name": "Theme.companion_app", "parent": "Theme.Material3.DayNight.NoActionBar"})
            attributes = {"android:windowBackground": "@drawable/deepseek_launch_background"}
            if "v31" in qualifier:
                attributes.update({"android:windowSplashScreenBackground": "@color/deepseek_brand_background",
                                   "android:windowSplashScreenAnimatedIcon": "@drawable/deepseek_splash_icon",
                                   "android:windowSplashScreenAnimationDuration": "0"})
            for key, value in attributes.items():
                item = theme.find(f"item[@name='{key}']")
                if item is None:
                    item = ET.SubElement(theme, "item", {"name": key})
                item.text = value
            ET.indent(themes, space="    ")
            ET.ElementTree(themes).write(themes_file, encoding="utf-8", xml_declaration=True)
    # Keep the app aligned with the audited API 36 SDK used by Tauri's Android
    # library and our Keystore plugin, even when CLI templates change defaults.
    gradle = project / "app/build.gradle.kts"
    if gradle.is_file():
        text = gradle.read_text(encoding="utf-8")
        for setting in ("compileSdk", "targetSdk"):
            text, count = re.subn(rf"(?m)^(\s*{setting}\s*=\s*)\d+\s*$", rf"\g<1>36", text)
            if count != 1:
                raise SystemExit(f"Unknown generated Gradle {setting}; refusing to guess SDK policy.")
        marker = "// DeepSeek isolated debug installation"
        if marker not in text:
            text += '''
// DeepSeek isolated debug installation
android {
    signingConfigs.getByName("debug") {
        storeFile = rootProject.file(".test-signing/debug.keystore")
    }
    buildTypes.getByName("debug") {
        applicationIdSuffix = ".test"
    }
}
'''
        gradle.write_text(text, encoding="utf-8")
    wrapper = project / "gradle/wrapper/gradle-wrapper.properties"
    if wrapper.is_file():
        props = wrapper.read_text()
        if 'gradle-9.6.1-bin.zip' not in props:
            raise SystemExit('Unreviewed Gradle distribution; audit checksum before building.')
        props = re.sub(r'(?m)^distributionSha256Sum=.*\n?', '', props)
        wrapper.write_text(props.rstrip() + '\ndistributionSha256Sum=9c0f7faeeb306cb14e4279a3e084ca6b596894089a0638e68a07c945a32c9e14\n')
    resources = project / "app/src/main/res/xml"
    resources.mkdir(parents=True, exist_ok=True)
    domains = ["root", "file", "database", "sharedpref", "external"]
    backup = ET.Element("full-backup-content")
    for domain in domains:
        ET.SubElement(backup, "exclude", {"domain": domain, "path": "."})
    extraction = ET.Element("data-extraction-rules")
    for kind in ["cloud-backup", "device-transfer"]:
        rule = ET.SubElement(extraction, kind)
        for domain in domains:
            ET.SubElement(rule, "exclude", {"domain": domain, "path": "."})
    for filename, value in [("companion_backup_rules.xml", backup),
                             ("companion_data_extraction_rules.xml", extraction)]:
        ET.indent(value, space="    ")
        ET.ElementTree(value).write(resources / filename, encoding="utf-8", xml_declaration=True)


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--project", type=Path, default=Path(__file__).resolve().parents[1] / "apps/companion/src-tauri/gen/android")
    prepare(parser.parse_args().project)
    print("Android prepared: API 36, INTERNET, adjustResize, disabled automatic backup and production cleartext.")
