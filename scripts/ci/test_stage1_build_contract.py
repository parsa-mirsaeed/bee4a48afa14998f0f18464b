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
        cls.build_deps = cls.dockerfile.split(
            "FROM toolchain AS build-deps\n", 1
        )[1].split("FROM build-deps AS gateway-builder\n", 1)[0]
        cls.gateway_builder = cls.dockerfile.split(
            "FROM build-deps AS gateway-builder\n", 1
        )[1].split("FROM build-deps AS web-builder\n", 1)[0]
        cls.web_builder = cls.dockerfile.split(
            "FROM build-deps AS web-builder\n", 1
        )[1].split("FROM debian:trixie-slim AS runtime\n", 1)[0]
        cls.targeted = cls.package.split("  targeted:\n", 1)[1].split(
            "  package:\n", 1
        )[0]
        cls.complete_package = cls.package.split("  package:\n", 1)[1]

    def test_gateway_and_web_source_builds_are_independent(self):
        self.assertIn("FROM toolchain AS build-deps", self.dockerfile)
        self.assertIn("FROM build-deps AS gateway-builder", self.dockerfile)
        self.assertIn("FROM build-deps AS web-builder", self.dockerfile)
        self.assertIn(
            "cargo chef cook --release --recipe-path recipe.json --package web --features server",
            self.build_deps,
        )
        self.assertIn(
            "cargo chef cook --release --recipe-path recipe.json --package web --features web --target wasm32-unknown-unknown",
            self.build_deps,
        )

    def test_presentation_churn_does_not_invalidate_gateway_source_copy(self):
        self.assertIn("COPY packages/api/ packages/api/", self.gateway_builder)
        self.assertNotIn("COPY . .", self.gateway_builder)
        self.assertNotIn("packages/web", self.gateway_builder)
        self.assertNotIn("packages/ui", self.gateway_builder)
        self.assertIn(
            "cargo build --release --package api --features server --bin ai_gateway",
            self.gateway_builder,
        )
        self.assertIn("COPY . .", self.web_builder)
        self.assertIn("dx bundle --web --release --package web", self.web_builder)

    def test_sqlx_compile_schema_boundary_is_explicit_and_minimal(self):
        self.assertIn("postgresql postgresql-client", self.build_deps)
        self.assertNotIn("COPY . .", self.build_deps)
        self.assertIn("service postgresql start", self.gateway_builder)
        self.assertIn("createdb -p 55432 edutalent_build", self.gateway_builder)
        self.assertIn("bash scripts/ci/apply_migrations.sh", self.gateway_builder)
        self.assertIn("service postgresql start", self.web_builder)
        self.assertIn("createdb -p 55433 edutalent_build", self.web_builder)
        self.assertIn("bash scripts/ci/apply_migrations.sh", self.web_builder)
        self.assertEqual(self.pre_runtime.count("FROM toolchain AS build-deps"), 1)
        self.assertEqual(self.pre_runtime.count("postgresql postgresql-client"), 1)

    def test_parallel_source_builds_use_isolated_database_ports(self):
        self.assertIn("port = 55432", self.gateway_builder)
        self.assertIn("127.0.0.1:55432/edutalent_build", self.gateway_builder)
        self.assertNotIn("55433", self.gateway_builder)
        self.assertIn("port = 55433", self.web_builder)
        self.assertIn("127.0.0.1:55433/edutalent_build", self.web_builder)
        self.assertNotIn("55432", self.web_builder)
        self.assertNotIn("127.0.0.1:5432/edutalent_build", self.gateway_builder)
        self.assertNotIn("127.0.0.1:5432/edutalent_build", self.web_builder)

    def test_runtime_keeps_migration_and_database_client_contract(self):
        self.assertIn("postgresql-client", self.runtime)
        self.assertIn(
            "COPY --from=web-builder --chown=65532:65532 /opt/edutalent-web/ /opt/edutalent/",
            self.runtime,
        )
        self.assertIn(
            "COPY --from=gateway-builder --chown=65532:65532 /workspace/target/release/ai_gateway /opt/edutalent/ai_gateway",
            self.runtime,
        )
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
