#!/usr/bin/env python3
"""Structural regression tests for the Stage-1 AI Change Proof workflow."""

from pathlib import Path
import unittest


WORKFLOW = Path(".github/workflows/ci.yml")


class WorkflowContractTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.text = WORKFLOW.read_text(encoding="utf-8")
        cls.no_db = cls.text.split("  targeted-rust-no-db:\n", 1)[1].split(
            "  targeted-rust-db:\n", 1
        )[0]

    def test_no_db_lane_is_client_only(self):
        self.assertNotIn("    services:\n", self.no_db)
        self.assertIn("wasm32-unknown-unknown", self.no_db)
        self.assertIn("cargo check -p web --features web", self.no_db)
        self.assertIn("cargo clippy -p web --features web", self.no_db)
        self.assertIn("cargo test -p web --locked", self.no_db)
        self.assertNotIn("--features server", self.no_db)
        self.assertNotIn("cargo check -p api", self.no_db)

    def test_database_lane_is_selected_only_by_classifier_output(self):
        self.assertIn(
            "if: needs.classify.outputs.database == 'true'",
            self.text,
        )

    def test_workspace_manual_escalation_also_escalates_database(self):
        self.assertIn(
            'database = derived["needs_postgres"] or workspace or "ci:db" in labels',
            self.text,
        )

    def test_unknown_executable_change_fails_gate(self):
        self.assertIn('if yes("UNKNOWN"):', self.text)
        self.assertIn(
            "unknown executable/configuration change requires classifier ownership",
            self.text,
        )


if __name__ == "__main__":
    unittest.main(verbosity=2)
