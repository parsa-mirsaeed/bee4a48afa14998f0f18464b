#!/usr/bin/env python3
"""Validate offline grading quality evidence without contacting an LLM.

This checks file integrity, provenance metadata and every declared language/
subject intersection. It cannot authenticate teachers or approve a release.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys
from pathlib import Path
from types import SimpleNamespace

from evaluate import InvalidBenchmark, evaluate, read_benchmark, reject_duplicate_fields

FIELDS = frozenset({
    "schema_version", "benchmark_sha256", "provider", "model",
    "prompt_sha256", "rubric_policy_sha256",
    "reference_approval_id", "required_cohort_languages", "thresholds",
})
THRESHOLDS = frozenset({
    "min_samples", "min_coverage", "max_mae_pp",
    "max_severe_rate", "max_overconfident_rate",
    "severe_error_pp", "high_confidence",
})
DIGEST = re.compile(r"^[0-9a-f]{64}$")
TOKEN = re.compile(r"^[A-Za-z0-9_-]{8,80}$")
GROUP = re.compile(r"^[A-Za-z0-9_-]{1,48}/(?:en|fa)$")


def fail(message: str) -> None:
    raise InvalidBenchmark(message)


def read_manifest(path: Path) -> dict:
    if path.stat().st_size > 16_384:
        fail("manifest too large")
    data = json.loads(
        path.read_text(encoding="utf-8"),
        object_pairs_hook=reject_duplicate_fields,
        parse_constant=lambda value: fail("nonfinite manifest value"),
    )
    if not isinstance(data, dict) or set(data) != FIELDS:
        fail("manifest fields missing or unexpected")
    if data["schema_version"] != 1 or isinstance(data["schema_version"], bool):
        fail("unsupported manifest version")
    if (data["provider"], data["model"]) != ("openai", "gpt-4o"):
        fail("unapproved model/provider provenance")
    for key in ("benchmark_sha256", "prompt_sha256", "rubric_policy_sha256"):
        if not isinstance(data[key], str) or not DIGEST.fullmatch(data[key]):
            fail("invalid sha256 provenance")
    if not isinstance(data["reference_approval_id"], str) or not TOKEN.fullmatch(data["reference_approval_id"]):
        fail("missing anonymous reference approval identifier")
    groups = data["required_cohort_languages"]
    if not isinstance(groups, list) or not groups or len(groups) > 100:
        fail("missing required cohort/language coverage plan")
    if any(not isinstance(g, str) or not GROUP.fullmatch(g) for g in groups) or len(set(groups)) != len(groups):
        fail("invalid or duplicate cohort/language group")
    if not {"en", "fa"}.issubset({g.split("/")[1] for g in groups}):
        fail("English and Farsi cohorts are both required")
    thresholds = data["thresholds"]
    if not isinstance(thresholds, dict) or set(thresholds) != THRESHOLDS:
        fail("missing or unexpected quality thresholds")
    from evaluate import number
    for key in THRESHOLDS - {"min_samples"}:
        value = number(thresholds[key], key)
        if not 0 <= value <= (100 if key in ("max_mae_pp", "severe_error_pp") else 1):
            fail("quality threshold out of range")
    samples = thresholds["min_samples"]
    if isinstance(samples, bool) or not isinstance(samples, int) or samples < 1:
        fail("invalid minimum sample requirement")
    if thresholds["max_mae_pp"] <= 0 or thresholds["severe_error_pp"] <= 0:
        fail("invalid error threshold")
    return data


def check(manifest_path: Path, benchmark_path: Path) -> tuple[int, dict]:
    manifest = read_manifest(manifest_path)
    if benchmark_path.stat().st_size > 10_000_000:
        fail("benchmark size exceeds 10 MB policy")
    digest = hashlib.sha256(benchmark_path.read_bytes()).hexdigest()
    if digest != manifest["benchmark_sha256"]:
        fail("benchmark digest mismatch")
    rows = read_benchmark(benchmark_path)
    observed = {f"{row.cohort}/{row.language}" for row in rows}
    required = set(manifest["required_cohort_languages"])
    if not observed.issubset(required):
        fail("benchmark includes an unregistered cohort/language")
    missing = sorted(required - observed)
    metrics = evaluate(rows, SimpleNamespace(**manifest["thresholds"]))
    errors = list(metrics["failures"])
    errors.extend(f"missing required cohort_language:{group}" for group in missing)
    passed = not errors
    # Never claim actual teacher approval or production readiness from an
    # unverified manifest. An independent signoff must check both provenance
    # and model/prompt identity outside this deterministic offline tool.
    summary = {
        "version": 1,
        "technical_evidence_passed": passed,
        "release_approved": False,
        "manual_teacher_and_privacy_signoff_required": True,
        "model": manifest["model"],
        "benchmark_sha256": digest,
        "required_groups": sorted(required),
        "quality": metrics,
        "failures": errors,
    }
    return (0 if passed else 1), summary


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--manifest", type=Path, required=True)
    parser.add_argument("--benchmark", type=Path, required=True)
    parser.add_argument("--output", type=Path)
    args = parser.parse_args(argv)
    try:
        status, summary = check(args.manifest, args.benchmark)
    except (ValueError, TypeError, OSError) as exc:
        print(f"Quality evidence invalid: {exc}", file=sys.stderr)
        return 2
    text = json.dumps(summary, indent=2, sort_keys=True, ensure_ascii=False) + "\n"
    if args.output:
        args.output.write_text(text, encoding="utf-8")
    else:
        sys.stdout.write(text)
    return status


if __name__ == "__main__":
    raise SystemExit(main())
