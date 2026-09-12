#!/usr/bin/env python3
import importlib.util
import json
import os
import pathlib
import re
import subprocess
import tempfile
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

    def test_ordinary_proof_checkouts_pin_and_verify_the_claimed_head(self):
        for name in ("ci.yml", "full-validation.yml", "package.yml", "release-docs.yml"):
            with self.subTest(workflow=name):
                text = (ROOT.parent.parent / ".github/workflows" / name).read_text()
                blocks = re.findall(
                    r"      - uses: actions/checkout@[^\n]+\n(.*?)(?=      - |\Z)",
                    text, re.S,
                )
                self.assertTrue(blocks)
                for block in blocks:
                    self.assertIn("ref: ${{ github.event.pull_request.head.sha || github.sha }}", block)
                self.assertEqual(len(blocks), text.count("run: bash scripts/ci/stage1_verify_proof_head.sh"))
                self.assertEqual(len(blocks), text.count("PROOF_HEAD_SHA: ${{ github.event.pull_request.head.sha || github.sha }}"))

    def test_browser_rejects_a_claimed_head_different_from_source(self):
        text = (ROOT / "run_browser_e2e.sh").read_text()
        self.assertIn('PROOF_HEAD_SHA="${E2E_HEAD_SHA}" bash scripts/ci/stage1_verify_proof_head.sh', text)

    def test_head_verifier_accepts_exact_and_rejects_stale_missing_or_invalid_sha(self):
        script = (ROOT / "stage1_verify_proof_head.sh").resolve()
        with tempfile.TemporaryDirectory() as directory:
            def git(*args):
                return subprocess.check_output(["git", *args], cwd=directory, stderr=subprocess.DEVNULL, text=True).strip()
            git("init", "--quiet")
            git("-c", "user.name=Proof Test", "-c", "user.email=proof@example.test", "commit", "--allow-empty", "--quiet", "-m", "first")
            old = git("rev-parse", "HEAD")
            git("-c", "user.name=Proof Test", "-c", "user.email=proof@example.test", "commit", "--allow-empty", "--quiet", "-m", "second")
            current = git("rev-parse", "HEAD")
            for expected, success in ((current, True), (old, False), (current[:12], False), ("", False)):
                with self.subTest(expected=expected):
                    result = subprocess.run(["bash", str(script)], cwd=directory,
                        env={**os.environ, "PROOF_HEAD_SHA": expected}, capture_output=True, text=True)
                    self.assertEqual(result.returncode == 0, success, result.stderr)


if __name__ == "__main__":
    unittest.main(verbosity=2)
