#!/usr/bin/env python3
"""Keep a sanitized build log while preserving the exact child's exit status."""
import os
from pathlib import Path
import re
import subprocess
import sys

path = Path(sys.argv[1])
path.parent.mkdir(parents=True, exist_ok=True)
names = ['GH_TOKEN', 'GITHUB_TOKEN', 'KEYSTORE_B64', 'COMPANION_ANDROID_KEYSTORE_PASSWORD',
         'COMPANION_ANDROID_KEY_PASSWORD', 'COMPANION_ANDROID_KEY_ALIAS']
redact = [os.environ[name] for name in names if os.environ.get(name)]
failed_tests = set()
lints = set()
with path.open('w') as log:
    process = subprocess.Popen(sys.argv[2:], stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True, errors='replace')
    for line in process.stdout:
        for secret in redact:
            line = line.replace(secret, '[REDACTED]')
        log.write(line)
        log.flush()
        print(line, end='', flush=True)
        match = re.match(r'^test ([A-Za-z0-9_:]+) \.\.\. FAILED\s*$', line)
        if match:
            failed_tests.add(match[1])
        lints.update(re.findall(r'clippy::([a-z0-9_]+)', line))
    code = process.wait()
    if os.environ.get('GITHUB_ACTIONS') == 'true':
        # Fixed identifiers only, never raw output, exception bodies or inputs.
        # This also identifies the failing stage when log storage is unavailable.
        name = re.sub(r'[^A-Za-z0-9_.-]', '_', path.name)[:80]
        level = 'error' if code else 'notice'
        detail = '; tests=' + ','.join(sorted(failed_tests)) if failed_tests else ''
        if code and lints:
            detail += '; clippy=' + ','.join(sorted(lints))
        print(f'::{level} title=Check result::{name}: exit {code}{detail}', flush=True)
    raise SystemExit(code)
