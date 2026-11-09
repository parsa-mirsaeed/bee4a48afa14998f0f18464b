import argparse
import json
import tempfile
import unittest
from pathlib import Path

from evaluate import InvalidBenchmark, evaluate, read_benchmark


def policy(**overrides):
    args = dict(min_samples=1, min_coverage=0.5, max_mae_pp=5.0, max_severe_rate=0.0,
                max_overconfident_rate=0.0, severe_error_pp=10.0, high_confidence=0.8)
    args.update(overrides)
    return argparse.Namespace(**args)


def record(case_id="fixture_1", reference=18, predicted=18, confidence=0.9, cohort="math", language="fa"):
    return dict(case_id=case_id, cohort=cohort, language=language, grade_scale=20,
                reference_grade=reference, predicted_grade=predicted, confidence=confidence)


class EvaluationTest(unittest.TestCase):
    def parse(self, *rows):
        with tempfile.TemporaryDirectory() as d:
            path = Path(d) / "benchmark.jsonl"
            path.write_text("\n".join(json.dumps(row) for row in rows), encoding="utf-8")
            return read_benchmark(path)

    def test_identical_teacher_score_is_passing(self):
        result = evaluate(self.parse(record(), record("fixture_2", language="en")), policy())
        self.assertTrue(result["passed"])
        self.assertEqual(result["overall"]["mae_percentage_points"], 0)

    def test_one_abstention_and_minimum_coverage(self):
        result = evaluate(self.parse(record(), record("fixture_2", predicted=None, confidence=None)), policy(min_coverage=0.75))
        self.assertFalse(result["passed"])
        self.assertEqual(result["overall"]["abstention_count"], 1)

    def test_severe_disagreement_and_overconfidence(self):
        result = evaluate(self.parse(record(predicted=8)), policy())
        self.assertFalse(result["passed"])
        self.assertEqual(result["overall"]["severe_disagreement_count"], 1)
        self.assertEqual(result["overall"]["overconfident_severe_count"], 1)

    def test_small_language_group_fails_even_if_overall_passes(self):
        result = evaluate(self.parse(record(), record("fixture_2", language="en")), policy(min_samples=2))
        self.assertFalse(result["passed"])
        self.assertIn("language:en: insufficient samples", result["failures"])

    def test_unknown_pii_field_rejected(self):
        with self.assertRaises(InvalidBenchmark):
            self.parse(dict(record(), student_name="Sensitive name"))

    def test_nonfinite_and_boolean_scores_rejected(self):
        with self.assertRaises(InvalidBenchmark):
            self.parse(record(predicted=float("nan")))
        with self.assertRaises(InvalidBenchmark):
            self.parse(record(predicted=True))

    def test_duplicate_ids_rejected(self):
        with self.assertRaises(InvalidBenchmark):
            self.parse(record(), record())

    def test_unsafe_case_id_rejected(self):
        with self.assertRaises(InvalidBenchmark):
            self.parse(record(case_id="someone@example.com"))


if __name__ == "__main__":
    unittest.main()
