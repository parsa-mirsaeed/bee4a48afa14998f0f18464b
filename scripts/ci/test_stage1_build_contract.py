#!/usr/bin/env python3
"""Regression tests for the Stage-1 Docker/package build boundary."""

from pathlib import Path
import unittest


DOCKERFILE = Path("Dockerfile")
PACKAGE_WORKFLOW = Path(".github/workflows/package.yml")
EDUTALENT = Path("edutalent")


class Stage1BuildBoundaryTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.dockerfile = DOCKERFILE.read_text(encoding="utf-8")
        cls.package = PACKAGE_WORKFLOW.read_text(encoding="utf-8")
        cls.edutalent = EDUTALENT.read_text(encoding="utf-8")
        cls.pre_runtime, cls.runtime = cls.dockerfile.split(
            "FROM debian:trixie-slim AS runtime\n", 1
        )
        cls.api_builder = cls.dockerfile.split(
            "FROM api-dependencies AS api-builder\n", 1
        )[1].split("FROM web-dependencies AS web-builder\n", 1)[0]
        cls.targeted = cls.package.split("  targeted:\n", 1)[1].split(
            "  package:\n", 1
        )[0]
        cls.complete_package = cls.package.split("  package:\n", 1)[1]

    def test_api_and_web_builds_have_independent_dependency_stages(self):
        self.assertIn("FROM toolchain AS api-dependencies", self.dockerfile)
        self.assertIn("FROM toolchain AS web-dependencies", self.dockerfile)
        self.assertIn("FROM api-dependencies AS api-builder", self.dockerfile)
        self.assertIn("FROM web-dependencies AS web-builder", self.dockerfile)
        self.assertIn(
            "cargo chef cook --release --recipe-path recipe.json --package api --features server",
            self.dockerfile,
        )
        self.assertIn(
            "cargo chef cook --release --recipe-path recipe.json --package web --features server",
            self.dockerfile,
        )

    def test_presentation_churn_does_not_invalidate_api_source_copy(self):
        self.assertIn("COPY packages/api packages/api", self.api_builder)
        self.assertNotIn("COPY . .", self.api_builder)
        self.assertNotIn("packages/web/assets", self.api_builder)
        self.assertIn(
            "cargo build --release --package api --features server --bin ai_gateway",
            self.api_builder,
        )

    def test_artifact_compilation_does_not_start_or_install_postgres_server(self):
        self.assertNotIn("service postgresql start", self.pre_runtime)
        self.assertNotIn("createdb edutalent_build", self.pre_runtime)
        self.assertNotIn("edutalent_build", self.pre_runtime)
        self.assertNotIn("bash scripts/ci/apply_migrations.sh", self.pre_runtime)
        self.assertNotIn("postgresql postgresql-client", self.pre_runtime)

    def test_runtime_keeps_migration_and_database_client_contract(self):
        self.assertIn("postgresql-client", self.runtime)
        self.assertIn(
            "COPY --chown=65532:65532 packages/api/migration/migrations/ /opt/edutalent/packages/api/migration/migrations/",
            self.runtime,
        )
        self.assertIn(
            "COPY --chown=65532:65532 migrations/ /opt/edutalent/migrations/",
            self.runtime,
        )
        self.assertIn(
            "COPY --chown=65532:65532 scripts/ci/apply_migrations.sh /opt/edutalent/scripts/ci/apply_migrations.sh",
            self.runtime,
        )
        self.assertIn(
            "COPY --chown=65532:65532 scripts/ci/configure_database_role.sh /opt/edutalent/scripts/ci/configure_database_role.sh",
            self.runtime,
        )

    def test_buildkit_cache_scope_is_shared_and_immutable_proof_is_not_replaced(self):
        self.assertIn(
            '--cache-from "type=gha,scope=${EDUTALENT_BUILD_CACHE_SCOPE}"',
            self.edutalent,
        )
        self.assertIn(
            '--cache-to "type=gha,mode=max,scope=${EDUTALENT_BUILD_CACHE_SCOPE}"',
            self.edutalent,
        )
        self.assertGreaterEqual(
            self.package.count("EDUTALENT_BUILD_CACHE_SCOPE: edutalent-runtime"), 2
        )
        self.assertIn("bash edutalent package", self.complete_package)
        self.assertIn("SHA256SUMS", self.complete_package)

    def test_targeted_package_escalation_builds_real_image_and_replays_migrations(self):
        self.assertIn("'ci:package'", self.targeted)
        self.assertIn("bash edutalent build local", self.targeted)
        self.assertIn("docker run --rm --entrypoint /bin/sh edutalent:local", self.targeted)
        self.assertEqual(self.targeted.count("run --rm migrate"), 2)
        self.assertIn("targeted-runtime-build.log", self.targeted)
        self.assertIn("targeted-migrations.log", self.targeted)

    def test_complete_package_still_replays_packaged_migrations_twice(self):
        self.assertEqual(self.complete_package.count("run --rm migrate"), 2)
        self.assertIn("Docker image and release bundle", self.complete_package)


if __name__ == "__main__":
    unittest.main(verbosity=2)
