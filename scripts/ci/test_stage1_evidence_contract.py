#!/usr/bin/env python3
import importlib.util
import json
import pathlib
import re
import unittest

ROOT = pathlib.Path(__file__).parent
CLASSIFIER_PATH = ROOT / "stage1_change_classifier.py"
SCHEMA_PATH = ROOT / "evidence_schema.json"
SPEC = importlib.util.spec_from_file_location("stage1_change_classifier", CLASSIFIER_PATH)
classifier = importlib.util.module_from_spec(SPEC)
assert SPEC and SPEC.loader
SPEC.loader.exec_module(classifier)


class EvidenceContractTests(unittest.TestCase):
    def setUp(self):
        self.schema = json.loads(SCHEMA_PATH.read_text(encoding="utf-8"))

    def test_schema_categories_match_classifier(self):
        categories = self.schema["properties"]["categories"]["items"]["enum"]
        self.assertEqual(list(classifier.CATEGORY_NAMES), categories)

    def test_required_exact_head_fields_are_present(self):
        required = set(self.schema["required"])
        self.assertTrue({
            "head_sha", "base_sha", "comparison", "categories",
            "required_sections", "executed_sections", "jobs", "gate_result",
            "mode", "controls_ci", "unknown",
        }.issubset(required))

    def test_sha_pattern_is_strict(self):
        pattern = self.schema["properties"]["head_sha"]["pattern"]
        self.assertRegex("a" * 40, re.compile(pattern))
        self.assertIsNone(re.fullmatch(pattern, "a" * 39))
        self.assertIsNone(re.fullmatch(pattern, "g" * 40))

    def test_shadow_mode_cannot_be_mistaken_for_control(self):
        modes = self.schema["properties"]["mode"]["enum"]
        gate_results = self.schema["properties"]["gate_result"]["enum"]
        self.assertIn("shadow", modes)
        self.assertIn("control", modes)
        self.assertIn("not_controlling", gate_results)


if __name__ == "__main__":
    unittest.main(verbosity=2)
