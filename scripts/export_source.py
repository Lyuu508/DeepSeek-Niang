#!/usr/bin/env python3
"""Create a local source-only ZIP. Never uploads, pushes, or publishes anything."""
import argparse
import hashlib
import json
import re
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SKIP = {".git", ".tauri", ".gradle", ".kotlin", "build", "node_modules", "target", "dist", ".cache", "__pycache__", "gen", "data", "private", "backups", "exports", "media-cache", "secrets", "output", "work", "artifacts"}
ROOT_FILES = {".env.example", ".gitattributes", ".gitignore", ".npmrc", "AGENTS.md", "README.md", "CHANGELOG.md", "SECURITY.md", "DEVELOPMENT_PLAN.md", "Cargo.toml", "Cargo.lock", "package.json", "package-lock.json"}
SOURCE_DIRS = {".github", "apps", "assets", "crates", "db", "docs", "packages", "plugins", "scripts", "tests"}
DENIED = re.compile(r"\.(?:db|sqlite|sqlite3|pem|key|p12|pfx|jks|keystore|age|bak|backup|log|zip|class|aar|jar|apk|aab|exe|dll|msi|so|dylib)(?:[-.]|$)", re.I)


def source_files():
    for file in sorted(ROOT.rglob("*")):
        relative = file.relative_to(ROOT)
        if any(part in SKIP for part in relative.parts) or file.is_symlink() or not file.is_file():
            continue
        if relative.parts[0] not in SOURCE_DIRS and str(relative) not in ROOT_FILES:
            continue
        if DENIED.search(file.name) or file.name.startswith(".env") and file.name != ".env.example" or file.name.endswith(".local.json"):
            continue
        if file.name == ".env.example" or file.suffix in {".json", ".ts", ".tsx", ".rs", ".md", ".toml", ".txt", ".py", ".ps1", ".sh"}:
            if re.search(r"sk-(?:proj-)?[A-Za-z0-9_-]{24,}", file.read_text(encoding="utf-8")):
                raise SystemExit(f"Credential-like content detected; export stopped: {relative}")
        yield file, relative


def export(destination: Path):
    destination.parent.mkdir(parents=True, exist_ok=True)
    manifest = []
    with zipfile.ZipFile(destination, "w", zipfile.ZIP_DEFLATED, compresslevel=9) as archive:
        for file, relative in source_files():
            data = file.read_bytes()
            archive.writestr("deepseek-ai-companion/" + relative.as_posix(), data)
            manifest.append({"path": relative.as_posix(), "bytes": len(data), "sha256": hashlib.sha256(data).hexdigest()})
        archive.writestr("deepseek-ai-companion/SOURCE-MANIFEST.json", json.dumps(manifest, ensure_ascii=False, indent=2) + "\n")
    with zipfile.ZipFile(destination) as archive:
        if archive.testzip() is not None:
            raise SystemExit("Archive integrity check failed")
    checksum = hashlib.sha256(destination.read_bytes()).hexdigest()
    destination.with_suffix(destination.suffix + ".sha256").write_text(f"{checksum}  {destination.name}\n", encoding="utf-8")
    print(json.dumps({"archive": str(destination), "source_files": len(manifest), "bytes": destination.stat().st_size, "sha256": checksum}, ensure_ascii=False))


if __name__ == "__main__":
    version = json.loads((ROOT / "package.json").read_text())["version"]
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=Path, default=ROOT.parent / f"DeepSeek-Companion-stage1-v{version}.zip")
    export(parser.parse_args().output.resolve())
