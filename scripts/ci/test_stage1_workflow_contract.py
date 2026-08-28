#!/usr/bin/env python3
"""Structural and effective-policy tests for Stage-1 AI Change Proof."""

import importlib.util
from pathlib import Path
import unittest


WORKFLOW = Path(".github/workflows/ci.yml")
MODULE_PATH = Path(__file__).with_name("stage1_change_classifier.py")
SPEC = importlib.util.spec_from_file_location("stage1_change_classifier", MODULE_PATH)
classifier = importlib.util.module_from_spec(SPEC)
assert SPEC and SPEC.loader
SPEC.loader.exec_module(classifier)


def effective(files, labels=()):
    result = classifier.classify(files)
    labels = set(labels)
    derived = result["derived"]
    workspace = derived["workspace"] or "ci:workspace" in labels
    rust = derived["rust"] or workspace
    api = derived["api"] or workspace
    web = derived["web"] or workspace
    database = derived["needs_postgres"] or workspace or "ci:db" in labels
    auth_browser = any(
        path.startswith("packages/api/src/middleware/auth")
        or path.startswith("packages/api/src/middleware/authorization")
        or path.startswith("packages/api/src/middleware/endpoint_authorization")
        for path in result["changed_files"]
    )
    browser = derived["needs_browser"] or auth_browser or "ci:browser" in labels
    return {
        "rust": rust,
        "workspace": workspace,
        "api": api,
        "web": web,
        "database": database,
        "browser": browser,
        "unknown": result["category_flags"]["unknown"],
    }


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
        result = effective(["README.md"], labels={"ci:workspace"})
        self.assertTrue(result["workspace"] and result["rust"])
        self.assertTrue(result["api"] and result["web"] and result["database"])

    def test_web_component_is_fast_client_lane(self):
        result = effective(["packages/web/src/views/role_based/teacher/dashboard.rs"])
        self.assertTrue(result["rust"] and result["web"])
        self.assertFalse(result["api"])
        self.assertFalse(result["database"])
        self.assertFalse(result["browser"])

    def test_login_keeps_browser_without_standalone_db_invariants(self):
        result = effective(["packages/web/src/views/login.rs"])
        self.assertTrue(result["rust"] and result["web"] and result["browser"])
        self.assertFalse(result["api"])
        self.assertFalse(result["database"])

    def test_backend_auth_keeps_database_and_browser(self):
        result = effective(["packages/api/src/middleware/auth.rs"])
        self.assertTrue(result["rust"] and result["api"])
        self.assertTrue(result["database"] and result["browser"])

    def test_db_and_browser_labels_only_add_proof(self):
        result = effective(["README.md"], labels={"ci:db", "ci:browser"})
        self.assertTrue(result["database"] and result["browser"])
        self.assertFalse(result["rust"])

    def test_unknown_executable_change_fails_gate(self):
        self.assertIn('if yes("UNKNOWN"):', self.text)
        self.assertIn(
            "unknown executable/configuration change requires classifier ownership",
            self.text,
        )
        self.assertTrue(effective(["tools/new_unclassified_gate.py"])["unknown"])


if __name__ == "__main__":
    unittest.main(verbosity=2)
