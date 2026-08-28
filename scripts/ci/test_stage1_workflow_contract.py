#!/usr/bin/env python3
"""Structural and effective-policy tests for Stage-1 AI Change Proof."""

import importlib.util
from pathlib import Path
import unittest


WORKFLOW = Path(".github/workflows/ci.yml")
DOCKERFILE = Path("Dockerfile")
PACKAGE_WORKFLOW = Path(".github/workflows/package.yml")
APPLIANCE_BUILD = Path("scripts/appliance/build.sh")
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
        cls.dockerfile = DOCKERFILE.read_text(encoding="utf-8")
        cls.package_workflow = PACKAGE_WORKFLOW.read_text(encoding="utf-8")
        cls.appliance_build = APPLIANCE_BUILD.read_text(encoding="utf-8")
        cls.no_db = cls.text.split("  targeted-rust-no-db:\n", 1)[1].split(
            "  targeted-rust-db:\n", 1
        )[0]
        cls.db = cls.text.split("  targeted-rust-db:\n", 1)[1].split(
            "  browser-smoke:\n", 1
        )[0]
        cls.browser = cls.text.split("  browser-smoke:\n", 1)[1].split(
            "  gate:\n", 1
        )[0]

    def test_no_db_lane_is_client_only(self):
        self.assertNotIn("    services:\n", self.no_db)
        self.assertIn("wasm32-unknown-unknown", self.no_db)
        self.assertIn("cargo check -p web --features web", self.no_db)
        self.assertIn("cargo clippy -p web --features web", self.no_db)
        self.assertIn("--bin web", self.no_db)
        self.assertNotIn("--lib --locked", self.no_db)
        self.assertIn("cargo test -p web --locked", self.no_db)
        self.assertNotIn("--features server", self.no_db)
        self.assertNotIn("cargo check -p api", self.no_db)

    def test_rust_cache_domains_are_runner_and_build_mode_specific(self):
        self.assertIn("shared-key: ai-change-web-wasm", self.no_db)
        self.assertIn("key: ${{ runner.os }}-${{ runner.arch }}-wasm32-web", self.no_db)
        self.assertIn("shared-key: ai-change-server-native", self.db)
        self.assertIn("key: ${{ runner.os }}-${{ runner.arch }}-native-server", self.db)
        self.assertIn("shared-key: ai-change-browser-release", self.browser)
        self.assertIn("key: ${{ runner.os }}-${{ runner.arch }}-release-web-bundle", self.browser)

    def test_rust_cache_writes_are_same_repo_only_and_do_not_cache_tool_bins(self):
        save_guard = (
            "save-if: ${{ github.event_name != 'pull_request' || "
            "github.event.pull_request.head.repo.full_name == github.repository }}"
        )
        for section in (self.no_db, self.db, self.browser):
            self.assertIn(save_guard, section)
            self.assertIn("cache-bin: false", section)
            self.assertIn("cache-on-failure: true", section)

    def test_cache_hits_cannot_skip_rust_or_browser_proof(self):
        self.assertNotIn("cache-hit", self.no_db)
        self.assertNotIn("cache-hit", self.db)
        self.assertIn("Run focused browser smoke on exact head", self.browser)
        # Only Dioxus/Chromium installation may be conditional on their own
        # dedicated tool-cache hits. The browser proof itself is unconditional.
        proof = self.browser.split("- name: Run focused browser smoke on exact head", 1)[1]
        self.assertNotIn("if: steps.rust-cache", proof)

    def test_runtime_build_separates_gateway_from_web_source_invalidation(self):
        self.assertIn("FROM toolchain AS build-deps", self.dockerfile)
        self.assertIn("FROM build-deps AS gateway-builder", self.dockerfile)
        self.assertIn("FROM build-deps AS web-builder", self.dockerfile)
        gateway = self.dockerfile.split("FROM build-deps AS gateway-builder", 1)[1].split(
            "FROM build-deps AS web-builder", 1
        )[0]
        web = self.dockerfile.split("FROM build-deps AS web-builder", 1)[1].split(
            "FROM debian:trixie-slim AS runtime", 1
        )[0]
        self.assertIn("COPY packages/api/ packages/api/", gateway)
        self.assertNotIn("COPY packages/web", gateway)
        self.assertNotIn("COPY packages/ui", gateway)
        self.assertIn("cargo build --release --package api --features server --bin ai_gateway", gateway)
        self.assertIn("bash scripts/ci/apply_migrations.sh", gateway)
        self.assertIn("COPY . .", web)
        self.assertIn("dx bundle --web --release --package web", web)
        self.assertIn("bash scripts/ci/apply_migrations.sh", web)
        runtime = self.dockerfile.split("FROM debian:trixie-slim AS runtime", 1)[1]
        self.assertIn("COPY --from=web-builder", runtime)
        self.assertIn("COPY --from=gateway-builder", runtime)

    def test_package_full_build_is_explicit_pr_escalation_only(self):
        self.assertIn("contains(github.event.pull_request.labels.*.name, 'ci:package')", self.package_workflow)
        self.assertIn("EDUTALENT_BUILD_CACHE_SCOPE: edutalent-runtime", self.package_workflow)
        self.assertIn("Verify packaged migrations are repeatable", self.package_workflow)

    def test_appliance_runtime_reuses_package_buildkit_scope(self):
        self.assertIn('EDUTALENT_BUILD_CACHE_SCOPE:-edutalent-runtime', self.appliance_build)
        self.assertNotIn('EDUTALENT_BUILD_CACHE_SCOPE:-edutalent-appliance-${ARCH}', self.appliance_build)

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
