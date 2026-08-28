#!/usr/bin/env python3
"""Stage-1 shadow change-impact classifier.

S1-PR-00 deliberately does not use this classifier to select CI jobs. It is
executed in shadow mode so S1-PR-01 can compare decisions before changing proof.
"""

from __future__ import annotations

import argparse
import json
import re
from pathlib import PurePosixPath
from typing import Iterable

CATEGORY_NAMES = (
    "docs",
    "workflow_policy",
    "web_assets",
    "web_logic",
    "web_browser_behavior",
    "api_logic",
    "api_data_access",
    "auth_authorization",
    "database",
    "ai_gateway",
    "worker_rag",
    "dependencies",
    "packaging",
    "production_topology",
    "operations",
    "appliance",
    "release",
    "unknown",
)

DOC_ROOT_FILES = {
    "README.md",
    "SECURITY.md",
    "LICENSE",
    "AGENTS.md",
    "EduTalent-Production-Readiness-AI-Implementation-Plan.md",
    "EduTalent-Full-UI-UX-Redesign-and-Workflow-Hardening-Plan.md",
    "EduTalent-Stage1-Smart-CI-CD-and-Build-Optimization-Plan.md",
    "EduTalent-Workflow-Trigger-Guide.md",
}

WORKFLOW_POLICY_RE = re.compile(
    r"^\.github/(?:workflows/.*\.ya?ml|FULL_VALIDATION\.md|scripts/.*)$"
)
DB_RE = re.compile(
    r"^(?:migrations/|packages/api/migration/|"
    r"scripts/ci/(?:apply_migrations|verify_knowledge_schema|"
    r"verify_knowledge_security|verify_transaction_scoped_rls|"
    r"configure_database_role)\.(?:sh|py))"
)
API_DATA_RE = re.compile(
    r"^packages/api/(?:src/)?(?:repositories?/|.*(?:repository|query|store|storage|database|db)\w*\.rs$)"
)
AUTH_RE = re.compile(
    r"^(?:packages/api/(?:endpoint_authorization_manifest\.psv|src/(?:middleware/(?:auth|authorization|endpoint_authorization).*|"
    r".*(?:authorized|authorization|permission|capability).*\.rs))|"
    r"scripts/ci/.*(?:authorization|rls|security).*|"
    r"packages/web/src/.*(?:login|auth|role_guard|session).*)"
)
AI_GATEWAY_RE = re.compile(
    r"^(?:packages/api/src/.*ai_gateway.*|deploy/production/.*ai.*gateway.*|"
    r"docs/(?:adr|security)/.*ai.*gateway.*)"
)
WORKER_RAG_RE = re.compile(
    r"^(?:packages/api/src/.*(?:knowledge|ingest|embedding|vector|qdrant|personalization|job|worker).*|"
    r"scripts/ci/.*(?:knowledge|vector|qdrant|job|worker).*)"
)
PACKAGING_RE = re.compile(
    r"^(?:Dockerfile(?:\.appliance-tools)?|compose(?:\.release)?\.ya?ml|edutalent|Makefile|"
    r"\.dockerignore|docker/|scripts/(?:package|release)/|\.github/workflows/package\.yml)"
)
PRODUCTION_RE = re.compile(
    r"^(?:deploy/production/|\.github/workflows/production-foundation\.yml|"
    r"scripts/(?:production|ci/configure_database_role).*|docker/entrypoint\.sh)"
)
OPERATIONS_RE = re.compile(
    r"^(?:deploy/(?:operations|monitoring|backup|recovery)/|scripts/(?:operations|backup|recovery)/|"
    r"\.github/workflows/production-operations\.yml|docs/(?:operations|runbooks)/)"
)
APPLIANCE_RE = re.compile(
    r"^(?:deploy/appliance/|scripts/appliance/|Dockerfile\.appliance-tools|"
    r"\.github/workflows/(?:air-gapped-appliance|air-gapped-release)\.yml)"
)
RELEASE_RE = re.compile(
    r"^(?:\.github/workflows/(?:mirror-final-proof|final-release-acceptance|air-gapped-release)\.yml|"
    r"docs/(?:release|acceptance)/|scripts/(?:release|ci/verify_release).*)"
)

DEPENDENCY_BASENAMES = {
    "Cargo.toml",
    "Cargo.lock",
    "package.json",
    "package-lock.json",
    "pnpm-lock.yaml",
    "yarn.lock",
    "rust-toolchain.toml",
}
DEPENDENCY_PATH_RE = re.compile(
    r"^(?:\.cargo/(?:audit\.toml|config\.toml)|packages/[^/]+/Cargo\.toml|tests/e2e/(?:package|package-lock)\.json)$"
)
WEB_ASSET_RE = re.compile(
    r"^(?:packages/(?:web|ui)/(?:assets|public|static)/|"
    r"packages/(?:web|ui)/.*\.(?:css|scss|sass|svg|png|jpe?g|webp|ico|woff2?|ttf|otf))"
)
WEB_LOGIC_RE = re.compile(r"^packages/(?:web|ui)/(?:src/|Cargo\.toml)")
BROWSER_HARNESS_RE = re.compile(
    r"^(?:tests/e2e/|scripts/ci/(?:run_browser_e2e|run_browser_smoke|run_browser_final|verify_browser_harness)\.sh)"
)
BROWSER_BEHAVIOR_RE = re.compile(
    r"^packages/web/src/.*(?:login|routing|navigation|sidebar|header|form|modal|dialog|drawer|popover|auth|session).*"
)
API_LOGIC_RE = re.compile(r"^packages/api/(?:src/|Cargo\.toml)")
GENERIC_EXECUTABLE_SUFFIXES = {
    ".rs", ".py", ".sh", ".bash", ".js", ".mjs", ".cjs", ".ts", ".tsx",
    ".sql", ".toml", ".yaml", ".yml", ".json", ".psv",
}


def _normalise(files: Iterable[str]) -> list[str]:
    result = []
    for raw in files:
        path = raw.strip().replace("\\", "/")
        while path.startswith("./"):
            path = path[2:]
        if path:
            result.append(path)
    return sorted(set(result))


def _mark(categories: dict[str, bool], name: str) -> None:
    categories[name] = True


def _is_docs(path: str) -> bool:
    return path in DOC_ROOT_FILES or path.startswith("docs/") or path.endswith((".md", ".mdx"))


def _looks_executable_or_config(path: str) -> bool:
    p = PurePosixPath(path)
    return p.suffix.lower() in GENERIC_EXECUTABLE_SUFFIXES or path in {"Dockerfile", "Makefile", "edutalent"}


def classify(files: Iterable[str]) -> dict:
    changed_files = _normalise(files)
    categories = {name: False for name in CATEGORY_NAMES}
    rust_file_changed = False
    cargo_workspace_changed = False

    for path in changed_files:
        matched = False
        basename = PurePosixPath(path).name

        if _is_docs(path):
            _mark(categories, "docs")
            matched = True

        if WORKFLOW_POLICY_RE.match(path) or path in {
            "EduTalent-Workflow-Trigger-Guide.md",
            "EduTalent-Stage1-Smart-CI-CD-and-Build-Optimization-Plan.md",
            "scripts/ci/stage1_change_classifier.py",
            "scripts/ci/test_stage1_change_classifier.py",
            "scripts/ci/test_stage1_evidence_contract.py",
            "scripts/ci/evidence_schema.json",
        }:
            _mark(categories, "workflow_policy")
            matched = True

        if WEB_ASSET_RE.match(path):
            _mark(categories, "web_assets")
            matched = True
        if WEB_LOGIC_RE.match(path):
            _mark(categories, "web_logic")
            matched = True
        if BROWSER_HARNESS_RE.match(path) or BROWSER_BEHAVIOR_RE.match(path):
            _mark(categories, "web_browser_behavior")
            matched = True
        if API_LOGIC_RE.match(path):
            _mark(categories, "api_logic")
            matched = True
        if API_DATA_RE.match(path):
            _mark(categories, "api_data_access")
            matched = True
        if AUTH_RE.match(path):
            _mark(categories, "auth_authorization")
            matched = True
        if DB_RE.match(path):
            _mark(categories, "database")
            matched = True
        if AI_GATEWAY_RE.match(path):
            _mark(categories, "ai_gateway")
            matched = True
        if WORKER_RAG_RE.match(path):
            _mark(categories, "worker_rag")
            matched = True

        if basename in DEPENDENCY_BASENAMES or DEPENDENCY_PATH_RE.match(path):
            _mark(categories, "dependencies")
            matched = True
            if path in {"Cargo.toml", "Cargo.lock", "rust-toolchain.toml"}:
                cargo_workspace_changed = True

        if PACKAGING_RE.match(path):
            _mark(categories, "packaging")
            matched = True
        if PRODUCTION_RE.match(path):
            _mark(categories, "production_topology")
            matched = True
        if OPERATIONS_RE.match(path):
            _mark(categories, "operations")
            matched = True
        if APPLIANCE_RE.match(path):
            _mark(categories, "appliance")
            matched = True
        if RELEASE_RE.match(path):
            _mark(categories, "release")
            matched = True

        if path.endswith(".rs"):
            rust_file_changed = True

        if not matched and _looks_executable_or_config(path):
            _mark(categories, "unknown")

    active = [name for name in CATEGORY_NAMES if categories[name]]
    rust = rust_file_changed or any(
        categories[name]
        for name in ("web_logic", "api_logic", "api_data_access", "auth_authorization", "ai_gateway", "worker_rag")
    )

    return {
        "schema_version": 1,
        "changed_files": changed_files,
        "categories": active,
        "category_flags": categories,
        "derived": {
            "rust": rust,
            "needs_postgres": any(categories[name] for name in ("api_data_access", "auth_authorization", "database", "worker_rag")),
            "needs_browser": categories["web_browser_behavior"],
            "needs_workspace_compile": cargo_workspace_changed,
            "needs_dependency_audit": categories["dependencies"],
            "needs_package_definition": categories["packaging"],
            "needs_production_definition": categories["production_topology"],
            "needs_operations_definition": categories["operations"],
            "needs_appliance_definition": categories["appliance"],
        },
        "safe_to_control_ci": not categories["unknown"],
        "mode": "shadow",
    }


def _read_files(args: argparse.Namespace) -> list[str]:
    files = list(args.files)
    if args.files_from:
        with open(args.files_from, "r", encoding="utf-8") as fh:
            files.extend(line.rstrip("\n") for line in fh)
    return files


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("files", nargs="*")
    parser.add_argument("--files-from")
    parser.add_argument("--output")
    parser.add_argument("--pretty", action="store_true")
    args = parser.parse_args()

    result = classify(_read_files(args))
    payload = json.dumps(result, indent=2 if args.pretty else None, sort_keys=True) + "\n"
    if args.output:
        with open(args.output, "w", encoding="utf-8") as fh:
            fh.write(payload)
    else:
        print(payload, end="")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
