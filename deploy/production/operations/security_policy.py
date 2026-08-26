#!/usr/bin/env python3
"""Fail-closed, repository-owned production configuration security checks.

This intentionally relies only on Docker Compose's canonical JSON rendering and
Python's standard library. It validates the concrete controls EduTalent depends
on in production instead of delegating policy to an opaque third-party scanner.
"""

from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
import re
import subprocess
import sys
from typing import Any

REQUIRED_ENV_RE = re.compile(r"\$\{([A-Z0-9_]+):\?[^}]*\}")
FORBIDDEN_CAPABILITIES = {"ALL", "SYS_ADMIN", "SYS_PTRACE", "NET_ADMIN", "NET_RAW"}
FORBIDDEN_HOST_NAMESPACES = {"host"}
FORBIDDEN_SOCKET_FRAGMENTS = ("/var/run/docker.sock", "/run/docker.sock")


def fail(report: dict[str, Any], code: str, message: str, *, service: str | None = None) -> None:
    item: dict[str, str] = {"code": code, "message": message}
    if service is not None:
        item["service"] = service
    report["violations"].append(item)


def load_env_template(path: Path) -> dict[str, str]:
    values: dict[str, str] = {}
    for raw in path.read_text(encoding="utf-8").splitlines():
        line = raw.strip()
        if not line or line.startswith("#") or "=" not in line:
            continue
        key, value = line.split("=", 1)
        values[key.strip()] = value.strip()
    return values


def ensure_pinned_supabase_runtime(base_compose: Path) -> None:
    if base_compose.is_file():
        return

    bootstrap = Path("deploy/production/bootstrap-supabase.sh")
    if not bootstrap.is_file():
        raise RuntimeError(f"pinned Supabase bootstrap is missing: {bootstrap}")

    completed = subprocess.run(
        ["bash", str(bootstrap)],
        text=True,
        capture_output=True,
        check=False,
    )
    if completed.returncode != 0:
        detail = completed.stderr.strip() or completed.stdout.strip() or "bootstrap failed without output"
        raise RuntimeError(f"pinned Supabase runtime bootstrap failed:\n{detail}")
    if not base_compose.is_file():
        raise RuntimeError(f"pinned Supabase runtime bootstrap did not materialize {base_compose}")


def compose_environment(
    compose_texts: list[str],
    supabase_env_template: Path,
    app_env_template: Path,
    production_dir: Path,
) -> dict[str, str]:
    values = load_env_template(supabase_env_template)
    values.update(load_env_template(app_env_template))
    env = os.environ.copy()
    env.update(values)
    env["EDUTALENT_PRODUCTION_DIR"] = str(production_dir.resolve())

    # Required secrets are needed only for interpolation during static policy
    # validation. Use deterministic validation-only placeholders and never
    # print them or persist them to disk. Empty template values count as absent
    # because Compose's ${VAR:?message} form rejects empty strings too.
    required_names: set[str] = set()
    for compose_text in compose_texts:
        required_names.update(REQUIRED_ENV_RE.findall(compose_text))
    for name in sorted(required_names):
        if not env.get(name):
            env[name] = f"validation-only-{name.lower()}"
    return env


def canonical_compose(
    base_compose: Path,
    overlay_compose: Path,
    supabase_env_template: Path,
    app_env_template: Path,
) -> dict[str, Any]:
    ensure_pinned_supabase_runtime(base_compose)
    if not supabase_env_template.is_file():
        raise RuntimeError(f"pinned Supabase environment template is missing: {supabase_env_template}")

    base_text = base_compose.read_text(encoding="utf-8")
    overlay_text = overlay_compose.read_text(encoding="utf-8")
    env = compose_environment(
        [base_text, overlay_text],
        supabase_env_template,
        app_env_template,
        overlay_compose.parent,
    )

    command = [
        "docker",
        "compose",
        "--project-name",
        "edutalent-security-policy",
        "--project-directory",
        str(base_compose.parent.resolve()),
        "--env-file",
        str(supabase_env_template.resolve()),
        "--env-file",
        str(app_env_template.resolve()),
        "-f",
        str(base_compose.resolve()),
        "-f",
        str(overlay_compose.resolve()),
    ]

    profile = env.get("EMBEDDING_PROFILE", "").strip()
    if profile == "local-bge-v1":
        command.extend(["--profile", "local-embedding"])
    elif profile not in {"", "openai-v1"}:
        raise RuntimeError(f"unsupported EMBEDDING_PROFILE in production template: {profile}")

    command.extend(["config", "--format", "json"])
    completed = subprocess.run(command, env=env, text=True, capture_output=True, check=False)
    if completed.returncode != 0:
        raise RuntimeError(
            "docker compose config failed; static security policy cannot be evaluated:\n"
            + completed.stderr.strip()
        )
    try:
        return json.loads(completed.stdout)
    except json.JSONDecodeError as exc:
        raise RuntimeError(f"docker compose emitted invalid JSON: {exc}") from exc


def volume_targets(service: dict[str, Any]) -> list[str]:
    targets: list[str] = []
    for volume in service.get("volumes") or []:
        if isinstance(volume, str):
            targets.append(volume)
        elif isinstance(volume, dict):
            source = str(volume.get("source") or "")
            target = str(volume.get("target") or "")
            targets.extend([source, target])
    return targets


def validate_service(name: str, service: dict[str, Any], report: dict[str, Any]) -> None:
    if service.get("privileged") is True:
        fail(report, "privileged-container", "privileged=true is forbidden", service=name)

    for namespace_key in ("network_mode", "pid", "ipc"):
        value = str(service.get(namespace_key) or "").lower()
        if value in FORBIDDEN_HOST_NAMESPACES:
            fail(
                report,
                "host-namespace",
                f"{namespace_key}=host is forbidden",
                service=name,
            )

    capabilities = {str(value).upper() for value in (service.get("cap_add") or [])}
    dangerous = sorted(capabilities & FORBIDDEN_CAPABILITIES)
    if dangerous:
        fail(
            report,
            "dangerous-capability",
            f"forbidden capabilities requested: {', '.join(dangerous)}",
            service=name,
        )

    security_opt = {str(value).lower() for value in (service.get("security_opt") or [])}
    if "no-new-privileges:true" not in security_opt:
        fail(
            report,
            "missing-no-new-privileges",
            "service must set no-new-privileges:true",
            service=name,
        )

    for value in volume_targets(service):
        lowered = value.lower()
        if any(fragment in lowered for fragment in FORBIDDEN_SOCKET_FRAGMENTS):
            fail(
                report,
                "docker-socket-mount",
                "mounting the Docker daemon socket is forbidden",
                service=name,
            )

    image = str(service.get("image") or "")
    if image:
        digest_pinned = "@sha256:" in image.lower()
        image_without_digest = image.split("@", 1)[0]
        final_component = image_without_digest.rsplit("/", 1)[-1]
        tagged = ":" in final_component
        if not digest_pinned and (not tagged or image_without_digest.endswith(":latest")):
            fail(
                report,
                "floating-image-tag",
                f"image must use an explicit non-latest tag or sha256 digest: {image}",
                service=name,
            )

    ports = service.get("ports") or []
    if ports and name != "gateway":
        fail(
            report,
            "unexpected-published-port",
            "only the TLS gateway may publish host ports",
            service=name,
        )


def validate_env_template(path: Path, report: dict[str, Any]) -> None:
    values = load_env_template(path)
    cidrs = values.get("ADMIN_ALLOWED_CIDRS", "")
    tokens = {token.strip() for token in cidrs.replace(",", " ").split() if token.strip()}
    if {"0.0.0.0/0", "::/0"} & tokens:
        fail(
            report,
            "internet-wide-admin-cidr",
            "ADMIN_ALLOWED_CIDRS must not expose administration to the entire internet",
        )

    for key, value in values.items():
        normalized = value.strip().lower()
        if key.endswith(("_INSECURE", "_SKIP_TLS_VERIFY")) and normalized in {"1", "true", "yes", "on"}:
            fail(
                report,
                "insecure-default",
                f"{key} must not default to an insecure value",
            )


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--base-compose",
        default="deploy/production/runtime/supabase/docker-compose.yml",
    )
    parser.add_argument("--compose", default="deploy/production/compose.production.yaml")
    parser.add_argument(
        "--supabase-env-template",
        default="deploy/production/runtime/supabase/.env.example",
    )
    parser.add_argument("--env-template", default="deploy/production/.env.edutalent.example")
    parser.add_argument("--output", default="production-security-policy.json")
    args = parser.parse_args()

    base_compose = Path(args.base_compose)
    compose_path = Path(args.compose)
    supabase_env_template = Path(args.supabase_env_template)
    env_template = Path(args.env_template)
    report: dict[str, Any] = {
        "schema_version": 1,
        "base_compose": str(base_compose),
        "compose": str(compose_path),
        "supabase_env_template": str(supabase_env_template),
        "env_template": str(env_template),
        "checks": [
            "no privileged containers",
            "no host network/pid/ipc namespaces",
            "no dangerous Linux capability additions",
            "no-new-privileges on every service",
            "no Docker daemon socket mounts",
            "no floating/latest image references",
            "only gateway publishes host ports",
            "no internet-wide admin CIDR default",
            "no insecure TLS defaults",
        ],
        "violations": [],
    }

    try:
        compose = canonical_compose(
            base_compose,
            compose_path,
            supabase_env_template,
            env_template,
        )
        services = compose.get("services") or {}
        if not isinstance(services, dict) or not services:
            fail(report, "missing-services", "canonical Compose configuration has no services")
        else:
            for name, service in sorted(services.items()):
                if isinstance(service, dict):
                    validate_service(str(name), service, report)
                else:
                    fail(report, "invalid-service", "service did not render as an object", service=str(name))
        validate_env_template(env_template, report)
    except Exception as exc:  # fail closed on parser/tooling errors
        fail(report, "policy-evaluation-error", str(exc))

    report["passed"] = not report["violations"]
    Path(args.output).write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")

    if report["violations"]:
        for violation in report["violations"]:
            prefix = f"[{violation['code']}]"
            if violation.get("service"):
                prefix += f" service={violation['service']}"
            print(f"{prefix} {violation['message']}", file=sys.stderr)
        return 1

    print("Production configuration security policy passed.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
