#!/usr/bin/env python3
"""Keep a sanitized build log while preserving the exact child's exit status."""
import os
from pathlib import Path
import subprocess
import sys

path = Path(sys.argv[1])
path.parent.mkdir(parents=True, exist_ok=True)
names = ['GH_TOKEN', 'GITHUB_TOKEN', 'KEYSTORE_B64', 'COMPANION_ANDROID_KEYSTORE_PASSWORD',
         'COMPANION_ANDROID_KEY_PASSWORD', 'COMPANION_ANDROID_KEY_ALIAS']
redact = [os.environ[name] for name in names if os.environ.get(name)]
with path.open('w') as log:
    process = subprocess.Popen(sys.argv[2:], stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True, errors='replace')
    for line in process.stdout:
        for secret in redact:
            line = line.replace(secret, '[REDACTED]')
        log.write(line)
        log.flush()
        print(line, end='', flush=True)
    raise SystemExit(process.wait())
