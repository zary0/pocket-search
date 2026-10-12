#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Original, deterministic source content for an operator-controlled native lab.

This is a fixture origin, not a search/render service or physical E2E driver.
It makes no outbound requests and never evaluates a URL as a filesystem path.
"""
import argparse
from contextlib import nullcontext
from dataclasses import dataclass
import hashlib
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import json
from pathlib import Path
import re
import ssl
import threading
import time

ROOT = Path(__file__).resolve().parents[1]
BASE = "/pocket-fixture/"
MODES = ("normal", "changed", "timeout", "unavailable", "oversized", "wrong-type", "truncated", "malformed-image", "png", "progressive")
MODIFIED = "Thu, 01 Jan 2026 00:00:00 GMT"


@dataclass(frozen=True)
class Reply:
    status: int
    media: str
    body: bytes
    etag: str = ""
    delay_seconds: int = 0
    declared_extra: int = 0


def source_profile(origin):
    # Reuse production admission, including the no-port/no-private-host boundary.
    if not isinstance(origin, str) or origin.endswith("/"):
        raise ValueError("Use an exact public HTTPS origin without a trailing slash")
    result = {"schema_version": 1, "profiles": [{"name": "Native Lab",
        "base": origin + BASE, "seed": origin + BASE + "node1.html",
        "robots": origin + "/robots.txt", "scope": "node-html"}]}
    if __package__:
        from .generate_chromatic_sources import registry
    else:
        from generate_chromatic_sources import registry
    registry(result)
    return result


def page(number, changed=False):
    heading = "PocketLab updated" if changed and number == 2 else "PocketLab document " + str(number)
    # Stay below retained text/tree/style budgets, including Japanese UTF-8 bytes.
    links = "".join('<li><a href="node' + str(n) + '.html">Document ' + str(n) + '</a></li>' for n in range(1, 5) if n != number)
    paragraphs = "".join("<p>Line " + str(n) + ": Read local content. 日本語とEnglishを読む。</p>" for n in range(1, 9))
    return ("<!doctype html><html><head><meta charset=\"utf-8\"><title>" + heading + "</title>"
        "<style>h1 {font-size:18px;color:#224466} p {font-size:12px} small {font-size:10px}</style></head>"
        "<body><h1>" + heading + "</h1><p><strong>Native fixture</strong> for standalone reading.</p>"
        '<img src="bars.jpg" alt="Original red green blue bars">' + paragraphs + "<ul>" + links + "</ul>"
        "<small>Original GPL-3.0-only lab content. End of document.</small></body></html>").encode("utf-8")


def reply(path, headers, mode="normal"):
    if mode not in MODES:
        raise ValueError("Unknown lab mode")
    if path == "/robots.txt":
        return Reply(200, "text/plain", b"User-agent: *\nAllow: /pocket-fixture/\nDisallow: /\n")
    match = re.fullmatch(re.escape(BASE) + r"node([1-4])\.html", path)
    if match:
        number = int(match[1])
        if number == 2:
            if mode == "timeout":
                return Reply(200, "text/html; charset=utf-8", page(number), delay_seconds=20)
            if mode == "unavailable":
                return Reply(503, "text/plain", b"Fixture unavailable\n")
            if mode == "oversized":
                return Reply(200, "text/html; charset=utf-8", b"X" * 65537)
            if mode == "wrong-type":
                return Reply(200, "application/octet-stream", page(number))
            if mode == "truncated":
                return Reply(200, "text/html; charset=utf-8", page(number)[:64], declared_extra=256)
        body = page(number, mode == "changed")
        if mode in ("png", "progressive"):
            body = body.replace(b"bars.jpg", b"bars.png" if mode == "png" else b"bars-progressive.jpg")
        etag = '"' + hashlib.sha256(body).hexdigest()[:24] + '"'
        normalized = {k.lower(): v for k, v in headers.items()}
        unchanged = normalized.get("if-none-match") == etag
        if "if-none-match" not in normalized:
            unchanged = normalized.get("if-modified-since") == MODIFIED and mode != "changed"
        return Reply(304 if unchanged else 200, "text/html; charset=utf-8", b"" if unchanged else body, etag)
    if path == BASE + "bars.jpg":
        body = b"\xff\xd8\xff\xd9" if mode == "malformed-image" else (ROOT / "ci/native-fixture/bars.jpg").read_bytes()
        return Reply(200, "image/jpeg", body)
    if path in (BASE + "bars.png", BASE + "palette.png", BASE + "bars-progressive.jpg"):
        name = path[len(BASE):]
        return Reply(200, "image/png" if name.endswith(".png") else "image/jpeg", (ROOT / "ci/native-fixture" / name).read_bytes())
    return Reply(404, "text/plain", b"Not a fixture resource\n")


class FixtureServer(ThreadingHTTPServer):
    daemon_threads = True

    def __init__(self, address, mode, trace, tls_context=None):
        super().__init__(address, FixtureHandler)
        self.mode, self.trace = mode, trace
        self.tls_context = tls_context
        self.trace_lock = threading.Lock()
        self.sequence = 0
        self.slots = threading.BoundedSemaphore(8)

    def get_request(self):
        request, address = super().get_request()
        request.settimeout(10)
        if self.tls_context:
            try:
                request = self.tls_context.wrap_socket(request, server_side=True)
            except BaseException:
                request.close()
                raise
        return request, address

    def process_request(self, request, address):
        if not self.slots.acquire(blocking=False):
            self.shutdown_request(request)
            return
        try:
            super().process_request(request, address)
        except BaseException:
            self.slots.release()
            raise

    def process_request_thread(self, request, address):
        try:
            super().process_request_thread(request, address)
        finally:
            self.slots.release()


class FixtureHandler(BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.0"

    def setup(self):
        self.request.settimeout(10)
        super().setup()

    def log_message(self, *args):
        # No source IPs, arbitrary headers, query strings or terminal escapes.
        pass

    def do_GET(self):
        self.serve(False)

    def do_HEAD(self):
        self.serve(True)

    def serve(self, head):
        mode = self.server.mode
        value = reply(self.path, self.headers, mode)
        sent = False
        try:
            if value.delay_seconds:
                time.sleep(value.delay_seconds)
            self.send_response(value.status)
            self.send_header("Content-Type", value.media)
            self.send_header("Content-Length", str(len(value.body) + value.declared_extra))
            self.send_header("Cache-Control", "no-store")
            if value.etag:
                self.send_header("ETag", value.etag)
                self.send_header("Last-Modified", MODIFIED)
            self.end_headers()
            if not head:
                self.wfile.write(value.body)
                self.wfile.flush()
            sent = True
        except OSError:
            pass
        if self.server.trace:
            with self.server.trace_lock:
                self.server.sequence += 1
                record = {"sequence": self.server.sequence, "mode": mode,
                    "resource": self.path if value.status != 404 else "unlisted",
                    "status": value.status, "body_bytes": len(value.body) if sent and not head else 0,
                    "conditional": bool(self.headers.get("If-None-Match") or self.headers.get("If-Modified-Since")),
                    "response_written": sent}
                self.server.trace.write(json.dumps(record, sort_keys=True) + "\n")
                self.server.trace.flush()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--origin", required=True)
    parser.add_argument("--profile-output", type=Path)
    parser.add_argument("--listen", default="127.0.0.1")
    parser.add_argument("--port", type=int, default=8443)
    parser.add_argument("--cert", type=Path)
    parser.add_argument("--key", type=Path)
    parser.add_argument("--mode", choices=MODES, default="normal")
    parser.add_argument("--trace", type=Path)
    args = parser.parse_args()
    profile = source_profile(args.origin)
    if args.profile_output:
        with args.profile_output.open("x") as output:
            output.write(json.dumps(profile, indent=2) + "\n")
        return
    if not args.cert or not args.key:
        parser.error("Serving requires the origin's certificate chain and private key")
    context = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
    context.minimum_version = ssl.TLSVersion.TLSv1_2
    context.load_cert_chain(args.cert, args.key)
    with (args.trace.open("x") if args.trace else nullcontext(None)) as trace:
        with FixtureServer((args.listen, args.port), args.mode, trace, context) as server:
            server.serve_forever()


if __name__ == "__main__":
    main()
