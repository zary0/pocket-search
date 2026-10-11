# SPDX-License-Identifier: GPL-3.0-only
"""Origin fixtures and local HTTP unit tests; never physical/native TLS evidence."""
import hashlib
from http.client import HTTPConnection
import io
import json
import struct
import threading
import time
import unittest
from scripts import native_fixture as f
from scripts.generate_chromatic_sources import registry


class NativeFixture(unittest.TestCase):
    def test_profile_stays_inside_production_url_admission(self):
        value = f.source_profile("https://fixture.example.org")
        self.assertIn("node1.html", registry(value))
        for origin in ("http://fixture.example.org", "https://127.0.0.1", "https://fixture.local",
                       "https://fixture.example.org:8443", "https://fixture.example.org/",
                       "https://fixture.example.org/other", "https://user@fixture.example.org"):
            with self.subTest(origin=origin), self.assertRaises(ValueError):
                f.source_profile(origin)

    def test_four_original_pages_and_strict_route_boundary(self):
        self.assertEqual(f.page(1), (f.ROOT / "examples/fixtures/native-lab.html").read_bytes())
        for number in range(1, 5):
            value = f.reply(f.BASE + "node" + str(number) + ".html", {})
            self.assertEqual(value.status, 200)
            self.assertLess(len(value.body), 2560)
            self.assertIn("日本語".encode(), value.body)
            self.assertIn(b"PocketLab", value.body)
            self.assertIn(b'bars.jpg', value.body)
            self.assertEqual(value.etag, '"' + hashlib.sha256(value.body).hexdigest()[:24] + '"')
        for path in ("/", "/etc/passwd", f.BASE + "../bars.jpg", f.BASE + "node5.html",
                     f.BASE + "bars.jpg?token=private", f.BASE + "%2e%2e/node1.html"):
            self.assertEqual(f.reply(path, {}).status, 404)

    def test_conditional_not_modified_and_changed_content(self):
        path = f.BASE + "node2.html"
        before = f.reply(path, {})
        same = f.reply(path, {"if-none-match": before.etag})
        self.assertEqual((same.status, same.body), (304, b""))
        changed = f.reply(path, {"If-None-Match": before.etag}, "changed")
        self.assertEqual(changed.status, 200)
        self.assertNotEqual(changed.etag, before.etag)
        self.assertIn(b"updated", changed.body)
        self.assertEqual(f.reply(path, {"If-None-Match": changed.etag}, "changed").status, 304)
        self.assertEqual(f.reply(path, {"If-Modified-Since": f.MODIFIED}).status, 304)
        self.assertEqual(f.reply(path, {"If-None-Match": '"different"', "If-Modified-Since": f.MODIFIED}).status, 200)

    def test_failure_modes_are_bounded_and_do_not_infect_other_pages(self):
        path = f.BASE + "node2.html"
        self.assertEqual(f.reply(path, {}, "timeout").delay_seconds, 20)
        self.assertEqual(f.reply(path, {}, "unavailable").status, 503)
        self.assertEqual(len(f.reply(path, {}, "oversized").body), 65537)
        self.assertEqual(f.reply(path, {}, "wrong-type").media, "application/octet-stream")
        truncated = f.reply(path, {}, "truncated")
        self.assertEqual((len(truncated.body), truncated.declared_extra), (64, 256))
        self.assertEqual(f.reply(f.BASE + "bars.jpg", {}, "malformed-image").body, b"\xff\xd8\xff\xd9")
        for mode in f.MODES:
            self.assertEqual(f.reply(f.BASE + "node1.html", {}, mode).status, 200)

    def test_original_image_has_small_baseline_dimensions(self):
        image = f.reply(f.BASE + "bars.jpg", {}).body
        self.assertLess(len(image), 24576)
        self.assertEqual(image[:2], b"\xff\xd8")
        self.assertEqual(image[-2:], b"\xff\xd9")
        i = image.index(b"\xff\xc0")
        self.assertEqual(struct.unpack(">HH", image[i + 5:i + 9]), (128, 192))
        self.assertNotIn(b"\xff\xc2", image)

    def test_local_http_transport_preserves_headers_and_redacts_trace(self):
        trace = io.StringIO()
        # Plain loopback is a host transport unit fixture, not a native TLS test.
        with f.FixtureServer(("127.0.0.1", 0), "normal", trace) as server:
            thread = threading.Thread(target=server.serve_forever, daemon=True)
            thread.start()
            try:
                connection = HTTPConnection(*server.server_address, timeout=3)
                connection.request("GET", f.BASE + "node1.html")
                response = connection.getresponse()
                etag = response.getheader("ETag")
                self.assertEqual(response.status, 200)
                self.assertEqual(int(response.getheader("Content-Length")), len(response.read()))
                connection.close()
                connection = HTTPConnection(*server.server_address, timeout=3)
                connection.request("GET", f.BASE + "node1.html", headers={"If-None-Match": etag})
                response = connection.getresponse()
                self.assertEqual((response.status, response.read()), (304, b""))
                connection.close()
                connection = HTTPConnection(*server.server_address, timeout=3)
                connection.request("GET", "/?token=private", headers={"Authorization": "private"})
                response = connection.getresponse()
                self.assertEqual(response.status, 404)
                response.read(); connection.close()
                deadline = time.monotonic() + 1
                while time.monotonic() < deadline:
                    with server.trace_lock:
                        if len(trace.getvalue().splitlines()) == 3:
                            break
                    time.sleep(.01)
            finally:
                server.shutdown(); thread.join(3)
        records = [json.loads(line) for line in trace.getvalue().splitlines()]
        self.assertEqual(len(records), 3)
        self.assertTrue(records[1]["conditional"])
        self.assertEqual(records[2]["resource"], "unlisted")
        self.assertNotIn("private", trace.getvalue())
        self.assertNotIn("127.0.0.1", trace.getvalue())


if __name__ == "__main__":
    unittest.main()
