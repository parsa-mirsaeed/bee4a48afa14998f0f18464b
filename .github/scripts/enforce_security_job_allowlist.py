#!/usr/bin/env python3
"""Ensure the production security job uses only approved execution paths.

The security job is intentionally implemented with RustSec's cargo-audit plus
repository-owned policy code. Third-party scanner actions are not permitted in
that job; only GitHub's checkout and artifact plumbing actions are allowed.
"""

from __future__ import annotations

from pathlib import Path
import re
import sys

WORKFLOW = Path(".github/workflows/production-operations.yml")
ALLOWED_USES = {
    "actions/checkout@v4",
    "actions/upload-artifact@v4",
}
REQUIRED_COMMANDS = (
    "cargo audit",
    "deploy/production/operations/security_policy.py",
)


def security_job(text: str) -> str:
    lines = text.splitlines()
    start: int | None = None
    end = len(lines)
    for index, line in enumerate(lines):
        if line == "  security-scan:":
            start = index
            continue
        if start is not None and index > start and re.match(r"^  [A-Za-z0-9_-]+:$", line):
            end = index
            break
    if start is None:
        raise RuntimeError("production workflow is missing security-scan job")
    return "\n".join(lines[start:end]) + "\n"


def main() -> int:
    try:
        job = security_job(WORKFLOW.read_text(encoding="utf-8"))
    except Exception as exc:
        print(f"Security job policy could not be evaluated: {exc}", file=sys.stderr)
        return 1

    violations: list[str] = []
    for match in re.finditer(r"^\s*-?\s*uses:\s*([^\s#]+)", job, flags=re.MULTILINE):
        action = match.group(1)
        if action not in ALLOWED_USES:
            violations.append(f"unapproved action in security-scan job: {action}")

    for command in REQUIRED_COMMANDS:
        if command not in job:
            violations.append(f"required security command missing: {command}")

    if violations:
        for violation in violations:
            print(violation, file=sys.stderr)
        return 1

    print("Production security-job allowlist passed.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
