#!/usr/bin/env python3
"""Bounded authenticated SSR GET probe for disposable EduTalent staging.

A passed run establishes only the supplied server-rendered read routes. It does
not establish browser-interaction throughput or maximum supported users.
"""
import argparse
from collections import Counter
from concurrent.futures import ThreadPoolExecutor
from datetime import datetime, timezone
import http.client
import json
import math
import os
from pathlib import Path
import stat
import threading
import time
from urllib.parse import urlsplit

LOCAL = {"127.0.0.1", "localhost", "::1"}


def percentile(values, fraction):
    if not values:
        return None
    items = sorted(values)
    return round(items[max(0, math.ceil(len(items) * fraction) - 1)], 3)


def validate_target(raw, approved_host=None):
    url = urlsplit(raw)
    if (url.scheme not in ("http", "https") or not url.hostname or
            url.username or url.password or url.path not in ("", "/") or
            url.query or url.fragment):
        raise ValueError("Provide an http(s) origin without credentials, path or query")
    if url.hostname not in LOCAL:
        if (os.environ.get("EDUTALENT_STAGING_APPROVED") != "YES" or
                approved_host != url.hostname):
            raise ValueError("Remote target needs EDUTALENT_STAGING_APPROVED=YES and exact --approved-host")
        if url.scheme != "https":
            raise ValueError("Non-local staging requires HTTPS")
    return url


def load_sessions(path):
    p = Path(path)
    mode = stat.S_IMODE(p.stat().st_mode)
    if mode & 0o077:
        raise ValueError("Session file must not be group/world-readable (chmod 600)")
    data = json.loads(p.read_text(encoding="utf-8"))
    if not isinstance(data, list) or not data or len(data) > 128:
        raise ValueError("Session file must be a JSON array of 1..128 configured sessions")
    output = []
    ids = set()
    for row in data:
        if not isinstance(row, dict):
            raise ValueError("Session entries must be objects")
        name = row.get("id")
        role = row.get("role")
        route = row.get("path")
        cookie = row.get("cookie")
        marker = row.get("expect_contains")
        if not all(isinstance(x, str) for x in (name, role, route, cookie, marker)):
            raise ValueError("Each session requires id, role, path, cookie and expect_contains strings")
        if (not name or len(name) > 50 or name in ids or not role or len(role) > 30 or
                not (route == "/dashboard" or route.startswith("/dashboard/")) or
                any(c in route for c in "?#\\\r\n") or "//" in route or ".." in route or
                len(route) > 160 or len(marker) < 4 or len(marker) > 120):
            raise ValueError("Invalid session id, role, read-only route or response marker")
        if (not cookie.startswith("access_token=") or any(c in cookie for c in "\r\n;\t ") or
                len(cookie) > 8192):
            raise ValueError("Cookie must contain one access_token value only (no refresh or other cookies)")
        ids.add(name)
        output.append({"id": name, "role": role, "path": route,
                       "cookie": cookie, "marker": marker.encode("utf-8")})
    return output


def request_once(url, session, timeout, max_body=1024 * 1024, connection=None):
    port = url.port or (443 if url.scheme == "https" else 80)
    if connection is None:
        cls = http.client.HTTPSConnection if url.scheme == "https" else http.client.HTTPConnection
        connection = cls(url.hostname, port, timeout=timeout)
    started = time.monotonic()
    status = None
    category = "transport"
    try:
        connection.request("GET", session["path"], headers={
            "Cookie": session["cookie"], "Accept": "text/html",
            "User-Agent": "EduTalent-auth-benchmark/1", "Cache-Control": "no-store",
        })
        response = connection.getresponse()
        status = response.status
        body = response.read(max_body + 1)
        if 300 <= status < 400:
            category = "redirect"
        elif status != 200:
            category = "unexpected_status"
        elif len(body) > max_body:
            category = "response_too_large"
        elif session["marker"] not in body:
            category = "marker_absent"
        else:
            category = "ok"
        # Close when peer advertises closed connection; urllib does not follow redirects.
        if response.will_close:
            connection.close()
            connection = None
    except (OSError, http.client.HTTPException, TimeoutError):
        connection.close()
        connection = None
    duration_ms = (time.monotonic() - started) * 1000.0
    return connection, status, category, duration_ms


def run_probe(url, sessions, workers, duration, timeout, think_ms, max_errors):
    stop = threading.Event()
    start_barrier = threading.Barrier(workers)
    lock = threading.Lock()
    counts = Counter()
    by_role = {}
    times = []
    active = 0
    peak = 0
    cutoff = time.monotonic() + duration
    request_cap = 100000

    def runner(index):
        nonlocal active, peak
        session = sessions[index % len(sessions)]
        conn = None
        start_barrier.wait()
        while not stop.is_set() and time.monotonic() < cutoff:
            with lock:
                active += 1
                peak = max(peak, active)
            try:
                conn, status, category, ms = request_once(url, session, timeout, connection=conn)
            finally:
                with lock:
                    active -= 1
            with lock:
                counts[category] += 1
                if status is not None:
                    counts[f"http_{status}"] += 1
                times.append(ms)
                if len(times) >= request_cap:
                    stop.set()
                role_stats = by_role.setdefault(session["role"], Counter())
                role_stats[category] += 1
                if sum(counts[k] for k in ("redirect", "unexpected_status", "response_too_large", "marker_absent", "transport")) >= max_errors:
                    stop.set()
            if think_ms:
                stop.wait(think_ms / 1000.0)
        if conn is not None:
            conn.close()

    started = time.monotonic()
    with ThreadPoolExecutor(max_workers=workers) as pool:
        list(pool.map(runner, range(workers)))
    elapsed = time.monotonic() - started
    total = sum(counts[k] for k in ("ok", "redirect", "unexpected_status", "response_too_large", "marker_absent", "transport"))
    failures = total - counts["ok"]
    used = sessions[:min(workers, len(sessions))]
    return {
        "kind": "authenticated-read-only-SSR-probe-not-maximum-capacity",
        "timestamp_utc": datetime.now(timezone.utc).isoformat(),
        "worker_count": workers,
        "configured_session_count": len(sessions),
        "active_distinct_session_count": len({s["cookie"] for s in used}),
        "client_in_flight_peak": peak,
        "server_tcp_connection_peak": None,
        "elapsed_seconds": round(elapsed, 3),
        "requests": total, "passed_content_checks": counts["ok"],
        "failures": failures, "error_rate": round(failures / total, 6) if total else 1,
        "throughput_all_rps": round(total / elapsed, 3),
        "throughput_verified_rps": round(counts["ok"] / elapsed, 3),
        "latency_ms": {"p50": percentile(times, 0.50), "p95": percentile(times, 0.95),
                       "p99": percentile(times, 0.99)},
        "outcomes": dict(counts),
        "by_role": {k: dict(v) for k, v in sorted(by_role.items())},
        "stopped_for_error_limit": failures >= max_errors,
        "stopped_for_request_cap": total >= request_cap,
        "limitations": "No browser JS or writes, reusable sessions per worker, no server-side socket/CPU/DB metrics; not concurrent-user capacity",
    }


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--origin", required=True)
    p.add_argument("--sessions-file", required=True)
    p.add_argument("--approved-host", help="Exact hostname for approved isolated staging")
    p.add_argument("--workers", type=int, default=1)
    p.add_argument("--duration", type=float, default=15)
    p.add_argument("--timeout", type=float, default=5)
    p.add_argument("--think-ms", type=int, default=500)
    p.add_argument("--max-errors", type=int, default=10)
    p.add_argument("--max-error-rate", type=float, default=0.005)
    p.add_argument("--max-p95-ms", type=float, default=1000)
    p.add_argument("--output", type=Path)
    p.add_argument("--preflight-only", action="store_true")
    a = p.parse_args()
    try:
        target = validate_target(a.origin, a.approved_host)
        sessions = load_sessions(a.sessions_file)
        if not (1 <= a.workers <= 128 and 1 <= a.duration <= 300 and
                0.2 <= a.timeout <= 15 and 0 <= a.think_ms <= 1500 and
                1 <= a.max_errors <= 1000 and 0 <= a.max_error_rate <= 0.5 and
                10 <= a.max_p95_ms <= 10000):
            raise ValueError("Unsafe worker/duration/timeout/think/error/latency bounds")
        for s in sessions:
            conn, status, result, _ = request_once(target, s, a.timeout)
            if conn is not None:
                conn.close()
            if result != "ok":
                raise ValueError(f"Preflight failed for role {s['role']}: {result}, HTTP {status}")
            # Reject role markers that can be fetched without authentication.
            anon = dict(s, cookie="")
            conn, _, anonymous_result, _ = request_once(target, anon, a.timeout)
            if conn is not None:
                conn.close()
            if anonymous_result == "ok":
                raise ValueError(f"Preflight failed: role {s['role']} marker is public without a cookie")
    except (ValueError, OSError, json.JSONDecodeError) as exc:
        p.error(str(exc))
    if a.preflight_only:
        result = {"status": "preflight_passed", "session_count": len(sessions),
                  "kind": "SSR-read-only-marker-preflight-not-capacity"}
    else:
        result = run_probe(target, sessions, a.workers, a.duration, a.timeout,
                           a.think_ms, a.max_errors)
    rendered = json.dumps(result, indent=2, sort_keys=True) + "\n"
    if a.output is not None:
        # Exclusive create, 0600: avoid overwriting evidence from a previous run.
        fd = os.open(a.output, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
        with os.fdopen(fd, "w", encoding="utf-8") as f:
            f.write(rendered)
    print(rendered, end="")
    if not a.preflight_only and (result["error_rate"] > a.max_error_rate or
                                 result["latency_ms"]["p95"] > a.max_p95_ms):
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
