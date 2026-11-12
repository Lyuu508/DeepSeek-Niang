#!/usr/bin/env python3
"""Optional owner-operated loopback download. Serves exactly one ZIP, not the workspace."""
import argparse
import hashlib
import html
import shutil
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from urllib.parse import urlsplit, quote


def serve(archive: Path, port: int):
    archive = archive.resolve(strict=True)
    if not archive.is_file() or archive.suffix.lower() != ".zip":
        raise SystemExit("Choose an existing source ZIP")

    class Handler(BaseHTTPRequestHandler):
        def log_message(self, *_):
            pass  # Never log request headers or untrusted query strings.

        def do_GET(self):
            route = urlsplit(self.path).path
            if route == "/download":
                self.send_response(200)
                self.send_header("Content-Type", "application/zip")
                self.send_header("Content-Disposition", f"attachment; filename=\"companion-source.zip\"; filename*=UTF-8''{quote(archive.name, safe='')}")
                self.send_header("Content-Length", str(archive.stat().st_size))
                self.send_header("Cache-Control", "no-store")
                self.end_headers()
                with archive.open("rb") as stream:
                    shutil.copyfileobj(stream, self.wfile)
            elif route in {"/", "/sha256"}:
                digest = hashlib.sha256(archive.read_bytes()).hexdigest()
                body = (f'<meta charset="utf-8"><title>Local source export</title><a href="download">Download {html.escape(archive.name)}</a><pre>SHA256 {digest}</pre>' if route == "/" else digest).encode()
                self.send_response(200)
                self.send_header("Content-Type", "text/html; charset=utf-8" if route == "/" else "text/plain")
                self.send_header("Content-Length", str(len(body)))
                self.end_headers()
                self.wfile.write(body)
            else:
                self.send_error(404)

    print(f"Local-only download: http://127.0.0.1:{port}/download", flush=True)
    print("For a remote workspace, use its private port-forwarding/download feature; this is not a public attachment URL.", flush=True)
    ThreadingHTTPServer(("127.0.0.1", port), Handler).serve_forever()


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("archive", type=Path)
    parser.add_argument("--port", type=int, default=8766)
    arguments = parser.parse_args()
    serve(arguments.archive, arguments.port)
