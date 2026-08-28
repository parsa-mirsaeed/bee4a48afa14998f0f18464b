#!/usr/bin/env python3
"""Regression ledger for intentional S1-PR-01 classifier changes.

This is not a second production classifier. It models only the legacy decisions
needed to prove that every reduction is intentional and every security-sensitive
case is preserved or strengthened.
"""

import importlib.util
import pathlib
import re
import unittest

MODULE_PATH = pathlib.Path(__file__).with_name("stage1_change_classifier.py")
SPEC = importlib.util.spec_from_file_location("stage1_change_classifier", MODULE_PATH)
classifier = importlib.util.module_from_spec(SPEC)
assert SPEC and SPEC.loader
SPEC.loader.exec_module(classifier)


def legacy(files):
    changed = "\n".join(files)
    rust = bool(re.search(r"(?m)^(Cargo\.toml|Cargo\.lock|packages/)", changed))
    workspace = bool(re.search(r"(?m)^(Cargo\.toml|Cargo\.lock|\.github/workflows/(ci|full-validation)\.yml)$", changed))
    api = workspace or bool(re.search(r"(?m)^packages/api/", changed))
    web = workspace or bool(re.search(r"(?m)^packages/(web|ui)/", changed))
    database = workspace or bool(re.search(
        r"(?m)^(migrations/|packages/api/migration/|scripts/ci/(apply_migrations|verify_knowledge_schema|verify_knowledge_security|verify_transaction_scoped_rls)\.sh)",
        changed,
    ))
    browser = bool(re.search(
        r"(?m)^(tests/e2e/|packages/(web|ui)/|packages/api/src/(middleware|handlers)/|scripts/ci/(run_browser_e2e|run_browser_smoke|run_browser_final|verify_browser_harness)\.sh)",
        changed,
    ))
    return {
        "rust": rust,
        "workspace": workspace,
        "api": api,
        "web": web,
        "database_job": api or web or database,
        "browser": browser,
    }


class LegacyDeltaTests(unittest.TestCase):
    def new(self, files):
        return classifier.classify(files)["derived"]

    def test_css_removes_rust_db_and_browser_overtrigger(self):
        files = ["packages/web/assets/main.css"]
        old, new = legacy(files), self.new(files)
        self.assertTrue(old["rust"] and old["database_job"] and old["browser"])
        self.assertFalse(new["rust"])
        self.assertFalse(new["needs_postgres"])
        self.assertFalse(new["needs_browser"])

    def test_generic_dioxus_keeps_rust_but_removes_db_and_browser_overtrigger(self):
        files = ["packages/web/src/views/role_based/teacher/dashboard.rs"]
        old, new = legacy(files), self.new(files)
        self.assertTrue(old["rust"] and old["database_job"] and old["browser"])
        self.assertTrue(new["rust"] and new["web"])
        self.assertFalse(new["needs_postgres"])
        self.assertFalse(new["needs_browser"])

    def test_client_login_keeps_rust_and_browser_but_removes_db_overtrigger(self):
        files = ["packages/web/src/views/login.rs"]
        old, new = legacy(files), self.new(files)
        self.assertTrue(old["rust"] and old["database_job"] and old["browser"])
        self.assertTrue(new["rust"] and new["web"] and new["needs_browser"])
        self.assertFalse(new["api"])
        self.assertFalse(new["needs_postgres"])
        self.assertTrue(classifier.classify(files)["category_flags"]["auth_authorization"])

    def test_backend_auth_preserves_rust_db_and_browser_escalation_category(self):
        files = ["packages/api/src/middleware/auth.rs"]
        old, new = legacy(files), self.new(files)
        self.assertTrue(old["rust"] and old["database_job"] and old["browser"])
        self.assertTrue(new["rust"] and new["needs_postgres"])
        # The controlling workflow escalates backend auth middleware to browser
        # proof even though the path itself is not a Web file.
        self.assertTrue(classifier.classify(files)["category_flags"]["auth_authorization"])

    def test_api_pure_logic_remains_db_backed_until_sqlx_compile_coupling_is_removed(self):
        files = ["packages/api/src/services/grade_scale.rs"]
        old, new = legacy(files), self.new(files)
        self.assertTrue(old["rust"] and old["database_job"])
        self.assertTrue(new["rust"] and new["api"] and new["needs_postgres"])

    def test_repository_preserves_database_proof(self):
        files = ["packages/api/src/repositories/user_repository.rs"]
        old, new = legacy(files), self.new(files)
        self.assertTrue(old["database_job"])
        self.assertTrue(new["rust"] and new["needs_postgres"])

    def test_migration_strengthens_rust_regression_proof(self):
        files = ["migrations/20260828_example.sql"]
        old, new = legacy(files), self.new(files)
        self.assertTrue(old["database_job"])
        self.assertFalse(old["rust"])
        self.assertTrue(new["needs_postgres"])
        self.assertTrue(new["rust"] and new["api"])

    def test_unknown_executable_changes_from_silent_to_fail_closed(self):
        files = ["tools/new_gate.py"]
        old = legacy(files)
        result = classifier.classify(files)
        self.assertFalse(any(old.values()))
        self.assertTrue(result["category_flags"]["unknown"])
        self.assertFalse(result["safe_to_control_ci"])

    def test_cargo_lock_keeps_workspace_db_until_compile_coupling_is_removed(self):
        files = ["Cargo.lock"]
        old, new = legacy(files), self.new(files)
        self.assertTrue(old["rust"] and old["workspace"] and old["database_job"])
        self.assertTrue(new["rust"] and new["workspace"])
        self.assertTrue(new["needs_postgres"])


if __name__ == "__main__":
    unittest.main(verbosity=2)
