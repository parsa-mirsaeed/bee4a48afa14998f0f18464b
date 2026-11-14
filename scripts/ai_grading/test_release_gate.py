import hashlib
import json
import tempfile
import unittest
from pathlib import Path

from evaluate import InvalidBenchmark
from release_gate import check, read_manifest


def row(case_id, cohort, language, score=18):
    return {
        "case_id": case_id,
        "cohort": cohort,
        "language": language,
        "grade_scale": 20,
        "reference_grade": 18,
        "predicted_grade": score,
        "confidence": 0.9,
    }


def manifest(digest):
    return {
        "schema_version": 1,
        "benchmark_sha256": digest,
        "provider": "openai",
        "model": "gpt-4o",
        "prompt_sha256": "a" * 64,
        "rubric_policy_sha256": "b" * 64,
        "reference_approval_id": "synthetic_approval_001",
        "required_cohort_languages": ["math_g7/en", "math_g7/fa"],
        "thresholds": {
            "min_samples": 1,
            "min_coverage": 1,
            "max_mae_pp": 5,
            "max_severe_rate": 0,
            "max_overconfident_rate": 0,
            "severe_error_pp": 10,
            "high_confidence": 0.8,
        },
    }


class ReleaseEvidenceGateTests(unittest.TestCase):
    def fixtures(self, rows=None, mutator=lambda x: x):
        temp = tempfile.TemporaryDirectory()
        directory = Path(temp.name)
        records = rows or [
            row("synthetic_en", "math_g7", "en"),
            row("synthetic_fa", "math_g7", "fa"),
        ]
        bench = directory / "grades.jsonl"
        bench.write_text(
            "\n".join(json.dumps(x) for x in records) + "\n",
            encoding="utf-8",
        )
        config = mutator(manifest(hashlib.sha256(bench.read_bytes()).hexdigest()))
        path = directory / "manifest.json"
        path.write_text(json.dumps(config), encoding="utf-8")
        self.addCleanup(temp.cleanup)
        return path, bench

    def test_exact_document_provenance_and_en_fa_quality_pass_without_release_signoff(self):
        m, b = self.fixtures()
        code, report = check(m, b)
        self.assertEqual(code, 0)
        self.assertTrue(report["technical_evidence_passed"])
        self.assertFalse(report["release_approved"])
        self.assertTrue(report["manual_teacher_and_privacy_signoff_required"])
        self.assertEqual(len(report["quality"]["cohort_languages"]), 2)

    def test_missing_declared_group_fails_closed(self):
        m, b = self.fixtures(rows=[row("synthetic_en", "math_g7", "en")])
        code, report = check(m, b)
        self.assertEqual(code, 1)
        self.assertFalse(report["technical_evidence_passed"])
        self.assertIn(
            "missing required cohort_language:math_g7/fa",
            report["failures"],
        )

    def test_severe_confident_disagreement_fails(self):
        m, b = self.fixtures(rows=[
            row("synthetic_en", "math_g7", "en", score=4),
            row("synthetic_fa", "math_g7", "fa"),
        ])
        code, report = check(m, b)
        self.assertEqual(code, 1)
        self.assertFalse(report["release_approved"])
        self.assertGreater(report["quality"]["overall"]["overconfident_severe_count"], 0)

    def test_data_tampering_invalidates_sha256(self):
        m, b = self.fixtures()
        b.write_text(b.read_text() + " ", encoding="utf-8")
        with self.assertRaisesRegex(InvalidBenchmark, "digest mismatch"):
            check(m, b)

    def test_no_free_form_pii_fields_in_manifest(self):
        def with_pii(x):
            x["student_name"] = "not allowed"
            return x
        m, b = self.fixtures(mutator=with_pii)
        with self.assertRaisesRegex(InvalidBenchmark, "fields missing or unexpected"):
            check(m, b)

    def test_unsupported_model_cannot_reuse_pilot_evidence(self):
        def wrong_model(x):
            x["model"] = "unapproved-model"
            return x
        m, b = self.fixtures(mutator=wrong_model)
        with self.assertRaisesRegex(InvalidBenchmark, "unapproved model"):
            check(m, b)

    def test_duplicate_manifest_keys_are_rejected(self):
        m, b = self.fixtures()
        m.write_text(m.read_text().replace(
            '"schema_version": 1', '"schema_version": 1, "schema_version": 1'
        ), encoding="utf-8")
        with self.assertRaisesRegex(InvalidBenchmark, "duplicate JSON field"):
            read_manifest(m)

    def test_unregistered_intersection_fails_without_diluting_results(self):
        m, b = self.fixtures(rows=[
            row("synthetic_en", "history_g9", "en"),
            row("synthetic_fa", "math_g7", "fa"),
        ])
        with self.assertRaisesRegex(InvalidBenchmark, "unregistered cohort"):
            check(m, b)


if __name__ == "__main__":
    unittest.main()
