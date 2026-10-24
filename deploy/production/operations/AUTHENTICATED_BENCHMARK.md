# EduTalent authenticated, read-only staging benchmark — Phase 2

This script tests **server-rendered GET responses with a valid access cookie**. It does not exercise the Dioxus browser JavaScript journey or server-function API mutations, and its worker count is **not** a count of unique students. Use the separate browser harness and end-to-end transaction tests for those claims.

## Scope and preconditions

* Pin `main`/release SHA; run an **isolated, disposable staging deployment** seeded with synthetic school A and B accounts (see `tests/e2e/fixtures/seed.sql`). Never load a real school's deployment without separately approved change control and safety limits.
* Validate the chosen route in the running app first. `GET /dashboard` and `GET /dashboard/<section>` exist in the Dioxus router but may render a loading shell rather than role-specific content. **A marker must be uniquely returned for that authenticated role**; if preflight fails, use a properly authenticated test API route/browser-based journey rather than dropping this assertion.
* Mint fresh test-only JWT access cookies using the staging identity provider. `access_token` normally expires after 15 minutes; **do not load login or refresh endpoints**. Never commit or export tokens, passwords, fixture accounts' real identities or an unredacted session file.
* Isolate the load generator and record its CPU, RAM, open files and network utilization; capture app/proxy DB and host metrics independently. The Python standard-library runner uses one HTTP/1.1 connection per worker where supported. It is diagnostic, not an unbiased maximum-throughput generator.

Save a file **outside the repository**, e.g. `/tmp/edutalent-staging-sessions.json`, with permissions `0600`:

```json
[
  {
    "id": "student-staging-a",
    "role": "Student",
    "path": "/dashboard",
    "cookie": "access_token=REPLACE_WITH_TEST_ONLY_ACCESS_JWT",
    "expect_contains": "REPLACE_WITH_AUTHENTICATED_STUDENT_ONLY_MARKER"
  }
]
```

```bash
chmod 600 /tmp/edutalent-staging-sessions.json
python3 -m unittest discover -s deploy/production/operations -p 'test_benchmark_authenticated.py' -v
python3 deploy/production/operations/benchmark_authenticated.py \
  --origin http://127.0.0.1:8080 \
  --sessions-file /tmp/edutalent-staging-sessions.json \
  --preflight-only
python3 deploy/production/operations/benchmark_authenticated.py \
  --origin http://127.0.0.1:8080 \
  --sessions-file /tmp/edutalent-staging-sessions.json \
  --workers 4 --duration 60 --think-ms 500 \
  --output /tmp/edutalent-staging-phase2.json
```

Remote staging requires **both** `EDUTALENT_STAGING_APPROVED=YES` and `--approved-host exact-staging-hostname`, with HTTPS. This double opt-in is **not** authorization to load production.

## Measurements and scientific interpretation

The report records verified responses/sec, all responses/sec, status/error breakdown, latency P50/P95/P99, configured/active unique session counts and observed client in-flight peak. A 302 redirect, login shell, bad token, unexpected response marker, or marker accessible with **no cookie** **fails** preflight. The load is bounded to 128 workers, 5 minutes, 100,000 samples and 10 errors by default. An error or P95 breach returns nonzero; sample cap and error-limit aborts must be identified in analysis.

Use at least five independent repeated runs at 1, 4, 8, 16, 32 workers and several datasets; verify generator is not saturated. Collect peak **server** established TCP sockets and DB pool waits separately; this harness does not measure them. Use open-loop k6 or equivalent for a defensible saturation curve. Repeat for role-specific **actual data** endpoints, plus negative cross-school tests before claiming tenant-safe load. Record host, Docker image digest, pinned Rust version, exact SHA, response payload size and dataset hash. Do **not** advertise any max users/RPS, availability SLO or Rust-vs-Python speedup from a health/SSR diagnostic alone.

**Workflow classification:** new executable Python production-operations benchmark and test, plus docs. Per the supplied workflow trigger guide this requires exact-head `AI Change Proof / AI change gate` and focused Production Operations definitions/regression/security proof. Comprehensive ops and Full Validation run only at stable final review, not every draft edit.
