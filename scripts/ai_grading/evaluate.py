#!/usr/bin/env python3
"""Offline, fail-closed grading benchmark evaluator; no model or DB access."""
from __future__ import annotations

import argparse
import json
import math
import sys
from collections import defaultdict
from dataclasses import dataclass
from pathlib import Path

ALLOWED_FIELDS = frozenset({"case_id", "cohort", "language", "grade_scale", "reference_grade", "predicted_grade", "confidence"})


class InvalidBenchmark(ValueError):
    pass


@dataclass(frozen=True)
class Example:
    case_id: str
    cohort: str
    language: str
    grade_scale: int
    reference: float
    predicted: float | None
    confidence: float | None


def number(value: object, field: str) -> float:
    if isinstance(value, bool) or not isinstance(value, (int, float)) or not math.isfinite(value):
        raise InvalidBenchmark(f"{field} must be a finite number")
    return float(value)


def validate(row: object, line: int) -> Example:
    if not isinstance(row, dict):
        raise InvalidBenchmark(f"line {line}: expected a JSON object")
    unexpected = set(row) - ALLOWED_FIELDS
    missing = ALLOWED_FIELDS - set(row)
    if unexpected or missing:
        raise InvalidBenchmark(f"line {line}: unexpected fields {sorted(unexpected)}; missing fields {sorted(missing)}")
    for field, maximum in (("case_id", 80), ("cohort", 48), ("language", 16)):
        value = row[field]
        if not isinstance(value, str) or not value or len(value) > maximum or not all(c.isalnum() or c in "_-" for c in value):
            raise InvalidBenchmark(f"line {line}: {field} must be a short anonymous token")
    scale = row["grade_scale"]
    if isinstance(scale, bool) or not isinstance(scale, int) or not 1 <= scale <= 100:
        raise InvalidBenchmark(f"line {line}: invalid grade_scale")
    reference = number(row["reference_grade"], "reference_grade")
    if not 0 <= reference <= scale:
        raise InvalidBenchmark(f"line {line}: reference out of scale")
    predicted_value = row["predicted_grade"]
    confidence_value = row["confidence"]
    if predicted_value is None:
        if confidence_value is not None:
            raise InvalidBenchmark(f"line {line}: abstention must have null confidence")
        predicted = None
        confidence = None
    else:
        predicted = number(predicted_value, "predicted_grade")
        confidence = number(confidence_value, "confidence")
        if not 0 <= predicted <= scale or not 0 <= confidence <= 1:
            raise InvalidBenchmark(f"line {line}: prediction/confidence out of bounds")
    return Example(row["case_id"], row["cohort"], row["language"], scale, reference, predicted, confidence)


def reject_duplicate_fields(pairs: list[tuple[str, object]]) -> dict:
    """Reject ambiguous JSON; the last-value-wins decoder can hide tampering."""
    row = {}
    for key, value in pairs:
        if key in row:
            raise InvalidBenchmark(f"duplicate JSON field: {key}")
        row[key] = value
    return row


def read_benchmark(path: Path) -> list[Example]:
    seen = set()
    examples = []
    for line_num, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        if not line.strip():
            continue
        try:
            row = json.loads(line, object_pairs_hook=reject_duplicate_fields, parse_constant=lambda x: (_ for _ in ()).throw(ValueError("nonfinite")))
            example = validate(row, line_num)
        except (ValueError, TypeError) as exc:
            raise InvalidBenchmark(f"line {line_num}: {exc}") from exc
        if example.case_id in seen:
            raise InvalidBenchmark(f"line {line_num}: duplicate case_id")
        seen.add(example.case_id)
        examples.append(example)
    if not examples:
        raise InvalidBenchmark("benchmark contains no valid examples")
    return examples


def summarize(rows: list[Example], severe_error_pp: float, high_confidence: float) -> dict:
    graded = [x for x in rows if x.predicted is not None]
    errors = [100 * abs(x.predicted - x.reference) / x.grade_scale for x in graded]
    severe = sum(error >= severe_error_pp for error in errors)
    overconfident = sum(error >= severe_error_pp and x.confidence >= high_confidence for x, error in zip(graded, errors))
    # Preserve full precision for acceptance decisions. Rounding to six decimals
    # before the comparison can silently approve a borderline failing cohort.
    return {
        "sample_count": len(rows),
        "graded_count": len(graded),
        "abstention_count": len(rows) - len(graded),
        "coverage": len(graded) / len(rows),
        "mae_percentage_points": sum(errors) / len(errors) if errors else None,
        "severe_disagreement_count": severe,
        "severe_disagreement_rate": severe / len(graded) if graded else None,
        "overconfident_severe_count": overconfident,
        "overconfident_severe_rate": overconfident / len(graded) if graded else None,
    }


def evaluate(rows: list[Example], args: argparse.Namespace) -> dict:
    by_cohort = defaultdict(list)
    by_language = defaultdict(list)
    for row in rows:
        by_cohort[row.cohort].append(row)
        by_language[row.language].append(row)
    overall = summarize(rows, args.severe_error_pp, args.high_confidence)
    cohorts = {k: summarize(v, args.severe_error_pp, args.high_confidence) for k, v in sorted(by_cohort.items())}
    languages = {k: summarize(v, args.severe_error_pp, args.high_confidence) for k, v in sorted(by_language.items())}
    failures = []
    for scope, metrics in [("overall", overall)] + [(f"cohort:{key}", val) for key, val in cohorts.items()] + [(f"language:{key}", val) for key, val in languages.items()]:
        if metrics["sample_count"] < args.min_samples:
            failures.append(f"{scope}: insufficient samples")
        if metrics["coverage"] < args.min_coverage:
            failures.append(f"{scope}: insufficient prediction coverage")
        if metrics["mae_percentage_points"] is None or metrics["mae_percentage_points"] > args.max_mae_pp:
            failures.append(f"{scope}: MAE is unavailable or excessive")
        if metrics["severe_disagreement_rate"] is None or metrics["severe_disagreement_rate"] > args.max_severe_rate:
            failures.append(f"{scope}: severe disagreement rate is unavailable or excessive")
        if metrics["overconfident_severe_rate"] is None or metrics["overconfident_severe_rate"] > args.max_overconfident_rate:
            failures.append(f"{scope}: overconfident severe disagreement rate is unavailable or excessive")
    return {"version": 1, "passed": not failures, "overall": overall, "cohorts": cohorts, "languages": languages, "failures": failures,
            "policy": {"min_samples_per_group": args.min_samples, "min_coverage": args.min_coverage, "max_mae_percentage_points": args.max_mae_pp,
                       "max_severe_disagreement_rate": args.max_severe_rate, "max_overconfident_severe_rate": args.max_overconfident_rate,
                       "severe_error_percentage_points": args.severe_error_pp, "high_confidence": args.high_confidence}}


def main(argv: list[str] | None = None) -> int:
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--input", type=Path, required=True)
    p.add_argument("--output", type=Path)
    p.add_argument("--min-samples", type=int, required=True)
    p.add_argument("--min-coverage", type=float, required=True)
    p.add_argument("--max-mae-pp", type=float, required=True)
    p.add_argument("--max-severe-rate", type=float, required=True)
    p.add_argument("--max-overconfident-rate", type=float, required=True)
    p.add_argument("--severe-error-pp", type=float, default=10.0)
    p.add_argument("--high-confidence", type=float, default=0.8)
    args = p.parse_args(argv)
    if args.min_samples < 1 or not (0 <= args.min_coverage <= 1 and 0 <= args.max_severe_rate <= 1 and 0 <= args.max_overconfident_rate <= 1 and 0 <= args.high_confidence <= 1 and 0 < args.max_mae_pp <= 100 and 0 < args.severe_error_pp <= 100):
        p.error("invalid quality threshold")
    try:
        result = evaluate(read_benchmark(args.input), args)
    except (InvalidBenchmark, OSError) as exc:
        print(f"Benchmark invalid: {exc}", file=sys.stderr)
        return 2
    rendered = json.dumps(result, sort_keys=True, ensure_ascii=False, indent=2) + "\n"
    if args.output:
        args.output.write_text(rendered, encoding="utf-8")
    else:
        sys.stdout.write(rendered)
    return 0 if result["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
