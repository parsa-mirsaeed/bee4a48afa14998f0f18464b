"""ET-04/05: pinned Storage REST proof, restricted to the disposable CI stack."""
from concurrent.futures import ThreadPoolExecutor
from http.client import HTTPSConnection
import json
import os
from pathlib import Path
import socket
import ssl
import subprocess
from uuid import uuid4

HOST = "supabase.ci.internal"
MIB = 1024 * 1024
PDF = b"%PDF-1.4\n% synthetic fixture\n%%EOF\n"


class LocalCIConnection(HTTPSConnection):
    def connect(self):
        raw = socket.create_connection(("127.0.0.1", 443), self.timeout)
        try:
            self.sock = self._context.wrap_socket(raw, server_hostname=HOST)
        except BaseException:
            raw.close()
            raise


def check(condition, message):
    if not condition:
        raise RuntimeError(message)


def main():
    check(os.environ.get("GITHUB_ACTIONS") == "true", "CI-only Storage proof")
    tmp = Path(os.environ["RUNNER_TEMP"])
    context = ssl.create_default_context(cafile=str(tmp / "tls/fullchain.pem"))
    env = {}
    for line in Path("deploy/production/runtime/supabase/.env").read_text().splitlines():
        key, sep, value = line.partition("=")
        if sep and key in ("SUPABASE_SECRET_KEY", "SUPABASE_PUBLISHABLE_KEY"):
            env[key] = value
    secret = env["SUPABASE_SECRET_KEY"]
    publishable = env["SUPABASE_PUBLISHABLE_KEY"]
    token = json.loads((tmp / "login.json").read_text())["access_token"]
    check(bool(secret and publishable and token), "CI credentials unavailable")
    image = subprocess.check_output(
        ["docker", "inspect", "--format", "{{.Config.Image}}", "supabase-storage"],
        text=True,
    ).strip()
    check(image == "supabase/storage-api:v1.60.4", "unexpected Storage image pin")
    admin = {"apikey": secret, "Authorization": "Bearer " + secret}
    ordinary = {"apikey": publishable, "Authorization": "Bearer " + token}

    def request(method, path, headers, body=None, mime=None):
        connection = LocalCIConnection(HOST, timeout=45, context=context)
        try:
            h = dict(headers)
            if mime:
                h["Content-Type"] = mime
            if method == "POST" and path.startswith("object/"):
                h["x-upsert"] = "false"
            connection.request(method, "/storage/v1/" + path, body, h)
            response = connection.getresponse()
            data = response.read(65537)
            check(len(data) <= 65536, "oversized Storage response")
            return response.status, data
        finally:
            connection.close()

    def json_request(method, path, value):
        return request(method, path, admin, json.dumps(value).encode(), "application/json")

    def metadata(bucket):
        status, body = request("GET", "bucket/" + bucket, admin)
        check(status == 200, "bucket readback failed")
        return json.loads(body)

    def verify(bucket, limit, mime):
        value = metadata(bucket)
        check(value.get("public") is False, "bucket is not private")
        check(value.get("file_size_limit") == limit, "persisted size limit mismatch")
        check(sorted(value.get("allowed_mime_types") or []) == sorted(mime),
              "persisted MIME restrictions mismatch")

    for kind, limit, mime in (
        ("knowledge", 20 * MIB, ["application/pdf"]),
        ("originals", 10 * MIB, ["application/pdf", "image/jpeg", "image/png"]),
    ):
        # Never pre-create or alter the real application buckets.
        bucket = "ci-et-" + kind + "-" + uuid4().hex
        status, body = request("GET", "bucket/" + bucket, admin)
        check(status == 400, "pinned missing-bucket HTTP status changed")
        missing = json.loads(body)
        check(missing.get("statusCode") == "404"
              and missing.get("error") == "Bucket not found"
              and missing.get("message") == "Bucket not found"
              and "code" not in missing, "pinned missing-bucket envelope changed")
        payload = {"id": bucket, "name": bucket, "public": False,
                   "file_size_limit": limit, "allowed_mime_types": mime}
        with ThreadPoolExecutor(max_workers=2) as pool:
            statuses = [result[0] for result in pool.map(
                lambda _: json_request("POST", "bucket", payload), range(2))]
        check(any(200 <= status < 300 for status in statuses), "no creator succeeded")
        check(all(200 <= status < 300 or status in (400, 409) for status in statuses),
              "unexpected concurrent create status")
        verify(bucket, limit, mime)
        verify(bucket, limit, mime)
        path = "object/" + bucket + "/proof.pdf"
        status, _ = request("POST", path, admin, PDF, "application/pdf")
        check(200 <= status < 300, "authorized upload failed")
        status, body = request("GET", path, admin)
        check(status == 200 and body == PDF, "authorized readback mismatch")
        status, _ = request("POST", "object/" + bucket + "/wrong.txt", admin,
                            b"synthetic", "text/plain")
        check(status in (400, 415, 422), "wrong MIME was not rejected")
        oversized = PDF + b" " * (limit + 1 - len(PDF))
        status, _ = request("POST", "object/" + bucket + "/large.pdf", admin,
                            oversized, "application/pdf")
        check(status in (400, 413, 422), "oversize was not rejected")
        for unauthorized in ({}, ordinary):
            status, _ = request("GET", path, unauthorized)
            check(status in (400, 401, 403, 404), "unauthorized read not denied")
            status, _ = request("POST", "object/" + bucket + "/denied.pdf",
                                unauthorized, PDF, "application/pdf")
            check(status in (400, 401, 403, 404), "unauthorized write not denied")
        status, _ = request("GET", "object/public/" + bucket + "/proof.pdf", {})
        check(status in (400, 401, 403, 404), "public object route not denied")
        print(f"ET-04/05 {kind}: REST, concurrent creation, limits and private access verified")

    # Repair a disposable legacy bucket without deleting its object.
    bucket = "ci-et-legacy-" + uuid4().hex
    status, _ = json_request("POST", "bucket", {"id": bucket, "name": bucket, "public": False})
    check(200 <= status < 300, "legacy fixture creation failed")
    before = metadata(bucket)
    check(before.get("public") is False and before.get("file_size_limit") is None
          and before.get("allowed_mime_types") is None, "legacy fixture is not NULL/private")
    path = "object/" + bucket + "/keep.pdf"
    status, _ = request("POST", path, admin, PDF, "application/pdf")
    check(200 <= status < 300, "legacy fixture upload failed")
    status, _ = json_request("PUT", "bucket/" + bucket, {
        "public": False, "file_size_limit": 20 * MIB, "allowed_mime_types": ["application/pdf"]})
    check(200 <= status < 300, "legacy metadata update failed")
    verify(bucket, 20 * MIB, ["application/pdf"])
    status, body = request("GET", path, admin)
    check(status == 200 and body == PDF, "legacy object not preserved")
    print("ET-04/05 legacy metadata update preserved object integrity")
    # Existing CI teardown owns all disposable synthetic data cleanup.


if __name__ == "__main__":
    main()
