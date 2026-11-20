#!/usr/bin/env python3
"""One version source; validation never changes tags or allocates a release."""
import argparse
import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def next_version(value):
    if not re.fullmatch(r"(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)", value):
        raise ValueError("invalid version")
    major, minor, patch = map(int, value.split('.'))
    if minor > 9 or patch > 10:
        raise ValueError("version outside project sequence")
    if patch < 10:
        patch += 1
    else:
        patch = 0
        minor += 1
        if minor == 10:
            major += 1
            minor = 0
    return f"{major}.{minor}.{patch}"


def expected():
    data = json.loads((ROOT / "VERSION.json").read_text())
    next_version(data['version'])
    if not isinstance(data['androidVersionCode'], int) or not 0 < data['androidVersionCode'] < 2100000000:
        raise ValueError("invalid Android versionCode")
    validate_state(data,json.loads((ROOT/'VERSION_HISTORY.json').read_text())['releases'])
    return data


def validate_state(data, releases):
    if not releases:
        raise ValueError('verified release history is required')
    versions=[r['version'] for r in releases]
    codes=[r['androidVersionCode'] for r in releases]
    if len(set(versions))!=len(versions) or any(b<=a for a,b in zip(codes,codes[1:])):
        raise ValueError('release history must be unique and strictly increasing')
    latest=releases[-1]
    if data['version']==latest['version']:
        if data['androidVersionCode']!=latest['androidVersionCode']:
            raise ValueError('published version identity cannot change')
    elif data['version']!=next_version(latest['version']) or data['androidVersionCode']<=max(codes):
        raise ValueError('new release must be the next project version with an increasing versionCode')


def check():
    data = expected()
    for relative in ['package.json', 'apps/companion/package.json', 'packages/contracts/package.json',
                     'apps/companion/src-tauri/tauri.conf.json']:
        assert json.loads((ROOT / relative).read_text())['version'] == data['version'], relative
    lock = json.loads((ROOT / 'package-lock.json').read_text())
    for key in ['', 'apps/companion', 'packages/contracts']:
        assert lock['packages'][key]['version'] == data['version'], key
    assert lock['version'] == data['version']
    assert json.loads((ROOT / 'apps/companion/package.json').read_text())['dependencies']['@deepseek/contracts'] == data['version']
    assert lock['packages']['apps/companion']['dependencies']['@deepseek/contracts'] == data['version']
    for relative in ['Cargo.toml', 'apps/companion/src-tauri/Cargo.toml', 'plugins/companion-keystore/Cargo.toml', 'plugins/companion-voice/Cargo.toml']:
        assert re.search(r'^version = "' + re.escape(data['version']) + '"$', (ROOT / relative).read_text(), re.M), relative
    config = json.loads((ROOT / 'apps/companion/src-tauri/tauri.conf.json').read_text())
    assert config['identifier'] == 'io.local.deepseekcompanion'
    assert config['bundle']['android']['versionCode'] == data['androidVersionCode']
    return data


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--next')
    args = parser.parse_args()
    print(next_version(args.next) if args.next else json.dumps(check()))
