#!/usr/bin/env python3
"""Regression contract for Stage-1 specialized workflow ownership."""

from __future__ import annotations

import importlib.util
from pathlib import Path
import unittest


ROOT = Path(__file__).resolve().parents[2]
CI = ROOT / ".github/workflows/ci.yml"
PACKAGE = ROOT / ".github/workflows/package.yml"
FOUNDATION = ROOT / ".github/workflows/production-foundation.yml"
OPERATIONS = ROOT / ".github/workflows/production-operations.yml"
APPLIANCE = ROOT / ".github/workflows/air-gapped-appliance.yml"
FINAL = ROOT / ".github/workflows/final-release-acceptance.yml"
MODULE_PATH = Path(__file__).with_name("stage1_change_classifier.py")
SPEC = importlib.util.spec_from_file_location("stage1_change_classifier", MODULE_PATH)
classifier = importlib.util.module_from_spec(SPEC)
assert SPEC and SPEC.loader
SPEC.loader.exec_module(classifier)


def specialized(files, labels=()):
    result = classifier.classify(files)
    labels = set(labels)
    derived = result["derived"]
    return {
        "package": derived["needs_package_definition"] or "ci:package" in labels,
        "production": derived["needs_production_definition"] or "ci:production" in labels,
        "operations": derived["needs_operations_definition"] or "ci:operations" in labels,
        "appliance": derived["needs_appliance_definition"] or "ci:appliance" in labels,
    }


def on_block(text: str) -> str:
    return text.split("on:\n", 1)[1].split("\npermissions:\n", 1)[0]


class SpecializedTriggerContractTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.ci = CI.read_text(encoding="utf-8")
        cls.package = PACKAGE.read_text(encoding="utf-8")
        cls.foundation = FOUNDATION.read_text(encoding="utf-8")
        cls.operations = OPERATIONS.read_text(encoding="utf-8")
        cls.appliance = APPLIANCE.read_text(encoding="utf-8")
        cls.final = FINAL.read_text(encoding="utf-8")

    def test_ordinary_ui_and_private_api_do_not_select_specialized_proof(self):
        for files in (
            ["packages/web/src/views/role_based/teacher/dashboard.rs"],
            ["packages/ui/src/card.rs"],
            ["packages/api/src/services/grade_scale.rs"],
        ):
            self.assertEqual(
                specialized(files),
                {"package": False, "production": False, "operations": False, "appliance": False},
                files,
            )

    def test_each_specialized_boundary_selects_its_owner(self):
        self.assertTrue(specialized(["Dockerfile"])["package"])
        self.assertTrue(
            specialized(["deploy/production/compose.production.yaml"])["production"]
        )
        self.assertTrue(
            specialized(["deploy/production/operations/edutalent_ops.py"])["operations"]
        )
        self.assertTrue(specialized(["deploy/appliance/images.lock"])["appliance"])

    def test_escalation_labels_only_add_specialized_proof(self):
        base = specialized(["README.md"])
        self.assertEqual(
            base,
            {"package": False, "production": False, "operations": False, "appliance": False},
        )
        labels = {"ci:package", "ci:production", "ci:operations", "ci:appliance"}
        escalated = specialized(["README.md"], labels=labels)
        self.assertTrue(all(escalated.values()))

    def test_specialized_workflows_are_reusable_without_direct_pr_path_filters(self):
        for name, text in (
            ("package", self.package),
            ("production", self.foundation),
            ("operations", self.operations),
            ("appliance", self.appliance),
        ):
            trigger = on_block(text)
            self.assertIn("workflow_call:", trigger, name)
            self.assertNotIn("pull_request:", trigger, name)

    def test_ai_change_proof_invokes_only_classifier_selected_owners(self):
        expected = {
            "package-proof": ("packaging", "./.github/workflows/package.yml"),
            "production-proof": ("production", "./.github/workflows/production-foundation.yml"),
            "operations-proof": ("operations", "./.github/workflows/production-operations.yml"),
            "appliance-proof": ("appliance", "./.github/workflows/air-gapped-appliance.yml"),
        }
        for job, (flag, workflow) in expected.items():
            section = self.ci.split(f"  {job}:\n", 1)[1].split("\n  ", 1)[0]
            self.assertIn("needs: classify", section)
            self.assertIn(f"needs.classify.outputs.{flag} == 'true'", section)
            self.assertIn(f"uses: {workflow}", section)

    def test_ai_gate_enforces_selected_specialized_results(self):
        gate = self.ci.split("  gate:\n", 1)[1]
        for job, env_name, required_env in (
            ("package-proof", "PACKAGE_RESULT", "PACKAGE_REQUIRED"),
            ("production-proof", "PRODUCTION_RESULT", "PRODUCTION_REQUIRED"),
            ("operations-proof", "OPERATIONS_RESULT", "OPERATIONS_REQUIRED"),
            ("appliance-proof", "APPLIANCE_RESULT", "APPLIANCE_REQUIRED"),
        ):
            self.assertIn(f"      - {job}", gate)
            self.assertIn(f"{env_name}: ${{{{ needs.{job}.result }}}}", gate)
            self.assertIn(f'(yes("{required_env}"), os.environ.get("{env_name}", "")', gate)

    def test_policy_self_test_executes_this_contract(self):
        self.assertIn(
            "python3 scripts/ci/test_stage1_specialized_trigger_contract.py",
            self.ci,
        )

    def test_final_release_still_reaches_every_complete_specialized_owner(self):
        self.assertIn(
            "reuse_or_dispatch 'Production Foundation' production-foundation.yml",
            self.final,
        )
        self.assertIn(
            "reuse_or_dispatch 'Production Operations' production-operations.yml",
            self.final,
        )
        self.assertIn("reuse_or_dispatch 'Package' package.yml", self.final)
        self.assertIn("uses: ./.github/workflows/air-gapped-appliance.yml", self.final)
        self.assertIn("complete: true", self.final)


if __name__ == "__main__":
    unittest.main(verbosity=2)
