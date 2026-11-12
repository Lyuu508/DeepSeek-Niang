#!/usr/bin/env python3
"""Read-only, cross-platform prerequisite check. Never reads credentials."""
import json
import os
import platform
import shutil
import subprocess
from pathlib import Path


def probe(command, version_args):
    executable = shutil.which(command)
    if not executable:
        return {"available": False}
    try:
        result = subprocess.run([executable, *version_args], text=True, capture_output=True, timeout=20)
        return {"available": True, "exit_code": result.returncode,
                "version": (result.stdout + result.stderr).strip().splitlines()[:2]}
    except (OSError, subprocess.TimeoutExpired) as exc:
        return {"available": True, "error": type(exc).__name__}


report = {"os": platform.system(), "architecture": platform.machine(), "tools": {}}
for name, args in {"node": ["--version"], "npm": ["--version"], "rustc": ["--version"],
                   "cargo": ["--version"], "java": ["-version"], "javac": ["-version"],
                   "adb": ["version"], "sdkmanager": ["--version"],
                   "cl": [], "pkg-config": ["--version"]}.items():
    report["tools"][name] = probe(name, args)
report["android_sdk_configured"] = any(Path(os.environ[n]).is_dir() for n in
    ["ANDROID_HOME", "ANDROID_SDK_ROOT"] if os.environ.get(n))
report["android_ndk_configured"] = bool(os.environ.get("NDK_HOME") and Path(os.environ["NDK_HOME"]).is_dir())
report["limits"] = ["This check does not prove Windows/Android build or device compatibility.",
                    "No API key, chat database, signing key or account data was inspected."]
print(json.dumps(report, ensure_ascii=False, indent=2))
