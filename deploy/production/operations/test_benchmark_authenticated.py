"""Local-only unit/fixture regression for the opt-in authenticated SSR probe."""
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import importlib.util
import json
import os
from pathlib import Path
import stat
import subprocess
import sys
import tempfile
import threading
import unittest

HERE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location("probe", HERE / "benchmark_authenticated.py")
probe = importlib.util.module_from_spec(spec)
spec.loader.exec_module(probe)


class Fixture(BaseHTTPRequestHandler):
    def do_GET(self):
        if self.path == "/dashboard/redirect":
            self.send_response(302)
            self.send_header("Location", "/")
            self.end_headers()
            return
        if self.path == "/dashboard/student" and self.headers.get("Cookie") == "access_token=fixture_student":
            body = b"Student dashboard grade list"
            status = 200
        elif self.path == "/dashboard/teacher" and self.headers.get("Cookie") == "access_token=fixture_teacher":
            body = b"Teacher dashboard assignment list"
            status = 200
        else:
            body = b"Login page"
            status = 401
        self.send_response(status)
        self.send_header("Content-Type", "text/html")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def log_message(self, *args):
        pass


class ProbeTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.server = ThreadingHTTPServer(("127.0.0.1", 0), Fixture)
        cls.thread = threading.Thread(target=cls.server.serve_forever, daemon=True)
        cls.thread.start()
        cls.origin = f"http://127.0.0.1:{cls.server.server_port}"

    @classmethod
    def tearDownClass(cls):
        cls.server.shutdown()
        cls.server.server_close()
        cls.thread.join(timeout=3)

    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.sessions_file = Path(self.tmp.name) / "sessions.json"
        self.rows = [
            {"id": "student-1", "role": "Student", "path": "/dashboard/student", "cookie": "access_token=fixture_student", "expect_contains": "Student dashboard"},
            {"id": "teacher-1", "role": "Teacher", "path": "/dashboard/teacher", "cookie": "access_token=fixture_teacher", "expect_contains": "Teacher dashboard"},
        ]
        self.save()

    def save(self):
        self.sessions_file.write_text(json.dumps(self.rows), encoding="utf-8")
        self.sessions_file.chmod(0o600)

    def cli(self, *opts):
        return subprocess.run(
            [sys.executable, str(HERE / "benchmark_authenticated.py"), "--origin", self.origin,
             "--sessions-file", str(self.sessions_file), *opts],
            capture_output=True, text=True, timeout=15,
        )

    def test_preflight_and_concurrent_probe(self):
        r = self.cli("--workers", "4", "--duration", "1", "--think-ms", "10")
        self.assertEqual(r.returncode, 0, r.stderr)
        data = json.loads(r.stdout)
        self.assertGreater(data["requests"], 0)
        self.assertEqual(data["failures"], 0)
        self.assertEqual(data["active_distinct_session_count"], 2)
        self.assertGreaterEqual(data["client_in_flight_peak"], 1)
        self.assertIsNotNone(data["latency_ms"]["p99"])
        self.assertNotIn("fixture_student", r.stdout)
        self.assertNotIn("fixture_teacher", r.stdout)

    def test_preflight_rejects_bad_auth_content(self):
        self.rows[0]["expect_contains"] = "Another private school"
        self.save()
        r = self.cli("--preflight-only")
        self.assertNotEqual(r.returncode, 0)
        self.assertIn("marker_absent", r.stderr)

    def test_redirect_cannot_be_counted_as_login_success(self):
        self.rows[0]["path"] = "/dashboard/redirect"
        self.save()
        r = self.cli("--preflight-only")
        self.assertNotEqual(r.returncode, 0)
        self.assertIn("redirect", r.stderr)

    def test_secure_session_file_required(self):
        self.sessions_file.chmod(0o644)
        r = self.cli("--preflight-only")
        self.assertNotEqual(r.returncode, 0)
        self.assertIn("chmod 600", r.stderr)

    def test_cookie_injection_rejected(self):
        self.rows[0]["cookie"] = "access_token=abc\r\nX-Injected: true"
        self.save()
        r = self.cli("--preflight-only")
        self.assertNotEqual(r.returncode, 0)
        self.assertIn("Cookie must contain", r.stderr)

    def test_no_remote_load_without_approval(self):
        with self.assertRaises(ValueError):
            probe.validate_target("https://school.example")
        with self.assertRaises(ValueError):
            probe.validate_target("https://school.example", approved_host="another.example")
        with self.assertRaises(ValueError):
            probe.validate_target("https://someone:secret@school.example")

    def test_output_file_restricted_and_no_secret(self):
        output = Path(self.tmp.name) / "out.json"
        r = self.cli("--preflight-only", "--output", str(output))
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertEqual(stat.S_IMODE(output.stat().st_mode), 0o600)
        self.assertNotIn("fixture_student", output.read_text())
        self.assertEqual(json.loads(output.read_text())["status"], "preflight_passed")

    def test_marker_percentile_and_reporting(self):
        self.assertEqual(probe.percentile([1, 2, 3, 4], .95), 4)
        self.assertIsNone(probe.percentile([], .95))
        self.assertEqual(probe.validate_target(self.origin).hostname, "127.0.0.1")
        self.assertEqual(len(probe.load_sessions(self.sessions_file)), 2)


if __name__ == "__main__":
    unittest.main()
