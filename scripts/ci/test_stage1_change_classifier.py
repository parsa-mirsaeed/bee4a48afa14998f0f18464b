#!/usr/bin/env python3
import importlib.util
import pathlib
import unittest

MODULE_PATH = pathlib.Path(__file__).with_name("stage1_change_classifier.py")
SPEC = importlib.util.spec_from_file_location("stage1_change_classifier", MODULE_PATH)
classifier = importlib.util.module_from_spec(SPEC)
assert SPEC and SPEC.loader
SPEC.loader.exec_module(classifier)


class ClassifierTests(unittest.TestCase):
    def assert_categories(self, files, required=(), forbidden=(), **derived):
        result = classifier.classify(files)
        categories = set(result["categories"])
        for item in required:
            self.assertIn(item, categories, (files, result))
        for item in forbidden:
            self.assertNotIn(item, categories, (files, result))
        for key, value in derived.items():
            self.assertEqual(result["derived"][key], value, (files, result))
        self.assertEqual(result["mode"], "control")
        return result

    def test_docs_only(self):
        result = self.assert_categories(
            ["README.md", "docs/architecture.md"],
            required=("docs",),
            forbidden=("database", "web_logic", "unknown"),
            rust=False,
            docs_only=True,
            needs_postgres=False,
            needs_browser=False,
        )
        self.assertTrue(result["safe_to_control_ci"])

    def test_css_only(self):
        self.assert_categories(
            ["packages/web/assets/main.css"],
            required=("web_assets",),
            forbidden=("web_logic", "database", "unknown"),
            rust=False,
            web=False,
            needs_postgres=False,
            needs_browser=False,
        )

    def test_local_font_or_image(self):
        self.assert_categories(
            ["packages/web/assets/fonts/app.woff2", "packages/web/assets/logo.svg"],
            required=("web_assets",),
            forbidden=("database", "unknown"),
            rust=False,
            needs_postgres=False,
        )

    def test_web_rust_component(self):
        self.assert_categories(
            ["packages/web/src/views/role_based/teacher/dashboard.rs"],
            required=("web_logic",),
            forbidden=("database", "unknown"),
            rust=True,
            web=True,
            api=False,
            needs_postgres=False,
            needs_browser=False,
        )

    def test_ui_crate_rust_component(self):
        self.assert_categories(
            ["packages/ui/src/card.rs"],
            required=("web_logic",),
            forbidden=("database", "unknown"),
            rust=True,
            web=True,
            needs_postgres=False,
        )

    def test_login_is_browser_sensitive_but_not_db_invariant(self):
        self.assert_categories(
            ["packages/web/src/views/login.rs"],
            required=("web_logic", "web_browser_behavior", "auth_authorization"),
            rust=True,
            web=True,
            api=False,
            needs_browser=True,
            needs_postgres=False,
        )

    def test_api_pure_service_stays_db_backed_until_sqlx_compile_coupling_is_removed(self):
        self.assert_categories(
            ["packages/api/src/services/grade_scale.rs"],
            required=("api_logic",),
            forbidden=("api_data_access", "database", "unknown"),
            rust=True,
            api=True,
            web=False,
            needs_postgres=True,
        )

    def test_api_repository_requires_db(self):
        self.assert_categories(
            ["packages/api/src/repositories/user_repository.rs"],
            required=("api_logic", "api_data_access"),
            rust=True,
            api=True,
            needs_postgres=True,
        )

    def test_backend_auth_middleware_requires_db(self):
        self.assert_categories(
            ["packages/api/src/middleware/auth.rs"],
            required=("api_logic", "auth_authorization"),
            rust=True,
            api=True,
            needs_postgres=True,
        )

    def test_migration_requires_db_and_api_regression(self):
        self.assert_categories(
            ["migrations/20260828_example.sql"],
            required=("database",),
            rust=True,
            api=True,
            needs_postgres=True,
        )

    def test_rls_verifier_is_policy_and_db(self):
        self.assert_categories(
            ["scripts/ci/verify_transaction_scoped_rls.sh"],
            required=("database", "auth_authorization"),
            rust=True,
            api=True,
            needs_postgres=True,
        )

    def test_ai_gateway_stays_db_backed_while_api_server_compile_is_coupled(self):
        self.assert_categories(
            ["packages/api/src/ai_gateway_runtime.rs"],
            required=("api_logic", "ai_gateway"),
            forbidden=("web_browser_behavior",),
            rust=True,
            api=True,
            needs_postgres=True,
        )

    def test_knowledge_worker(self):
        self.assert_categories(
            ["packages/api/src/services/knowledge_ingestion_worker.rs"],
            required=("api_logic", "worker_rag"),
            rust=True,
            api=True,
            needs_postgres=True,
        )

    def test_root_cargo_lock_keeps_workspace_db_backed_until_compile_coupling_is_removed(self):
        self.assert_categories(
            ["Cargo.lock"],
            required=("dependencies",),
            rust=True,
            workspace=True,
            needs_workspace_compile=True,
            needs_dependency_audit=True,
            needs_postgres=True,
        )

    def test_api_manifest_is_rust_dependency_change_and_currently_db_backed(self):
        self.assert_categories(
            ["packages/api/Cargo.toml"],
            required=("api_logic", "dependencies"),
            rust=True,
            api=True,
            needs_dependency_audit=True,
            needs_postgres=True,
        )

    def test_package_file(self):
        self.assert_categories(
            ["Dockerfile"],
            required=("packaging",),
            needs_package_definition=True,
        )

    def test_production_compose(self):
        self.assert_categories(
            ["deploy/production/compose.production.yaml"],
            required=("production_topology",),
            forbidden=("operations",),
            needs_production_definition=True,
            needs_operations_definition=False,
        )

    def test_operations_script(self):
        self.assert_categories(
            ["scripts/operations/verify_backup.sh"],
            required=("operations",),
            needs_operations_definition=True,
        )

    def test_real_production_operations_tree_is_operations_not_topology(self):
        self.assert_categories(
            ["deploy/production/operations/edutalent_ops.py"],
            required=("operations",),
            forbidden=("production_topology", "unknown"),
            needs_production_definition=False,
            needs_operations_definition=True,
        )

    def test_real_operations_entrypoint_is_operations_not_topology(self):
        self.assert_categories(
            ["deploy/production/edutalent-operations"],
            required=("operations",),
            forbidden=("production_topology", "unknown"),
            needs_production_definition=False,
            needs_operations_definition=True,
        )

    def test_operations_adr_and_threat_model_keep_operations_owner(self):
        self.assert_categories(
            [
                "docs/adr/0004-production-operations.md",
                "docs/security/production-operations-threat-model.md",
            ],
            required=("docs", "operations"),
            forbidden=("production_topology", "unknown"),
            docs_only=False,
            needs_production_definition=False,
            needs_operations_definition=True,
        )

    def test_readiness_boundary_requires_operations_definition(self):
        self.assert_categories(
            ["packages/api/src/readiness.rs"],
            required=("api_logic", "operations"),
            needs_operations_definition=True,
        )

    def test_web_readiness_entrypoint_requires_operations_definition(self):
        self.assert_categories(
            ["packages/web/src/main.rs"],
            required=("web_logic", "operations"),
            forbidden=("unknown",),
            needs_operations_definition=True,
        )

    def test_production_markdown_remains_documentation_only(self):
        self.assert_categories(
            ["deploy/production/HOST_BASELINE.md", "deploy/production/README.md"],
            required=("docs",),
            forbidden=("production_topology", "operations", "unknown"),
            docs_only=True,
            needs_production_definition=False,
            needs_operations_definition=False,
        )

    def test_appliance_definition(self):
        self.assert_categories(
            ["deploy/appliance/images.lock"],
            required=("appliance",),
            needs_appliance_definition=True,
        )

    def test_workflow_policy(self):
        self.assert_categories(
            [".github/workflows/ci.yml"],
            required=("workflow_policy",),
            forbidden=("unknown",),
            rust=False,
            needs_postgres=False,
            needs_browser=False,
        )

    def test_browser_reset_helper_is_owned_by_browser_harness(self):
        result = self.assert_categories(
            ["scripts/ci/reset_browser_fixture_db.sh"],
            required=("web_browser_behavior",),
            forbidden=("unknown",),
            rust=True,
            web=True,
            needs_postgres=False,
            needs_browser=True,
        )
        self.assertTrue(result["safe_to_control_ci"])

    def test_unknown_executable_fails_closed(self):
        result = self.assert_categories(
            ["tools/new_unclassified_gate.py"],
            required=("unknown",),
        )
        self.assertFalse(result["safe_to_control_ci"])

    def test_mixed_is_union(self):
        result = self.assert_categories(
            [
                "packages/web/src/views/login.rs",
                "packages/api/src/repositories/session_repository.rs",
                "migrations/20260828_sessions.sql",
            ],
            required=(
                "web_logic",
                "web_browser_behavior",
                "auth_authorization",
                "api_logic",
                "api_data_access",
                "database",
            ),
            rust=True,
            api=True,
            web=True,
            needs_postgres=True,
            needs_browser=True,
        )
        self.assertFalse(result["category_flags"]["unknown"])


if __name__ == "__main__":
    unittest.main(verbosity=2)
