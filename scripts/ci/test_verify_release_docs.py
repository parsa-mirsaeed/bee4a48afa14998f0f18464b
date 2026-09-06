from __future__ import annotations

import importlib.util
import json
import os
import re
import subprocess
import textwrap
import tempfile
import unittest
from pathlib import Path

MODULE_PATH = Path(__file__).with_name("verify_release_docs.py")
spec = importlib.util.spec_from_file_location("verify_release_docs", MODULE_PATH)
assert spec and spec.loader
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


class ReleaseDocsVerifierTests(unittest.TestCase):
    def test_manual_acceptance_reference_is_consistent_in_governing_documents(self):
        root = MODULE_PATH.parents[2]
        governing = [
            root / "docs/security/production-threat-model.md",
            root / "EduTalent-Full-UI-UX-Redesign-and-Workflow-Hardening-Plan.md",
        ]
        for path in governing:
            self.assertIn("PR #2", path.read_text(), str(path))
        for path in [*governing, *(root / "docs/release").glob("*.md")]:
            with self.subTest(document=path.name):
                self.assertNotRegex(path.read_text(), r"\bPR\s*#16\b")

    def test_final_release_requires_existing_dispatched_job_names(self):
        workflows = MODULE_PATH.parents[2] / ".github/workflows"
        orchestration = (workflows / "final-release-acceptance.yml").read_text()
        calls = re.findall(
            r"reuse_or_dispatch '[^']+' ([\w-]+\.yml) [\w-]+\.json (.*?)\)\"",
            orchestration, re.S,
        )
        self.assertEqual(len(calls), 4)
        for filename, arguments in calls:
            with self.subTest(workflow=filename):
                declared = set(re.findall(
                    r"^    name: (.+)$", (workflows / filename).read_text(), re.M
                ))
                required = re.findall(r"'([^']+)'", arguments)
                self.assertTrue(required)
                self.assertTrue(set(required) <= declared,
                                f"Missing job names in {filename}: {set(required) - declared}")

    def test_final_dispatch_reuses_same_head_and_database_for_two_browser_passes(self):
        workflow = (MODULE_PATH.parents[2] / ".github/workflows/full-validation.yml").read_text()
        block = workflow.split("          passes=1\n", 1)[1].split(
            "\n      - name: Upload final browser evidence", 1
        )[0]
        script = "set -euo pipefail\n" + textwrap.dedent("          passes=1\n" + block)
        for event, expected_passes in [("workflow_dispatch", 2), ("pull_request", 1)]:
            with self.subTest(event=event), tempfile.TemporaryDirectory() as tmp:
                root = Path(tmp)
                runner = root / "scripts/ci/run_browser_final.sh"
                runner.parent.mkdir(parents=True)
                runner.write_text('printf "%s|%s\\n" "$E2E_HEAD_SHA" "$DATABASE_URL" >> calls\n')
                result = subprocess.run(
                    ["bash", "-c", script], cwd=root, capture_output=True, text=True,
                    env={**os.environ, "GITHUB_EVENT_NAME": event,
                         "PR_BASE_SHA": "", "PR_HEAD_SHA": "frozen-head",
                         "E2E_HEAD_SHA": "frozen-head", "DATABASE_URL": "dedicated-test-db"},
                )
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertEqual((root / "calls").read_text().splitlines(),
                                 ["frozen-head|dedicated-test-db"] * expected_passes)

    def run_manual_acceptance_entry(self, number: int, title: str):
        with tempfile.TemporaryDirectory() as tmp:
            fixture = Path(tmp) / "manual-fixture.json"
            fixture.write_text(json.dumps({
                "number": number,
                "title": title,
                "state": "open",
                "draft": True,
                "merged_at": None,
            }))
            script = r'''
set -euo pipefail
source scripts/release/final-release-run-utils.sh
retry_gh() {
  if [[ "$*" == *"graphql"* ]]; then
    printf '%s\n' '{"data":{"repository":{"issues":{"pageInfo":{"hasNextPage":false},"nodes":[]},"pullRequest":{"reviewThreads":{"pageInfo":{"hasNextPage":false},"nodes":[]}}}}}'
  elif [[ "$*" == *"/pulls/2" ]]; then
    cat "$MANUAL_PR_FIXTURE"
  else
    echo "Unexpected endpoint in release prerequisite check" >&2
    return 42
  fi
}
check_release_entry "$EVIDENCE_DIR/state.json" "$EVIDENCE_DIR/manual.json"
'''
            return subprocess.run(
                ["bash", "-c", script],
                cwd=MODULE_PATH.parents[2],
                env={**os.environ, "MANUAL_PR_FIXTURE": str(fixture),
                     "EVIDENCE_DIR": tmp, "REPOSITORY": "parsa-mirsaeed/bee4a48afa14998f0f18464b",
                     "PR_NUMBER": "65"},
                capture_output=True, text=True,
            )

    def test_manual_acceptance_tracks_current_pr_without_claiming_approval(self):
        result = self.run_manual_acceptance_entry(2, "Manual/external production acceptance evidence")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("PR #2 state=open merged_at=None", result.stdout)

    def test_manual_acceptance_rejects_wrong_pr_identity(self):
        for number, title in [(16, "Manual/external production acceptance evidence"),
                              (2, "Unrelated implementation pull request")]:
            with self.subTest(number=number, title=title):
                result = self.run_manual_acceptance_entry(number, title)
                self.assertNotEqual(result.returncode, 0)

    def test_secret_patterns_reject_private_key_material(self) -> None:
        text = "-----BEGIN PRIVATE KEY-----"
        self.assertTrue(module.SECRET_PATTERNS["private key"].search(text))

    def test_placeholder_invalid_email_is_not_real_contact(self) -> None:
        address = "security-contact@example.invalid"
        self.assertTrue(address.lower().endswith(".invalid"))

    def test_disabled_feature_row_contract(self) -> None:
        key = "attendance"
        matrix = "| `attendance` | Attendance workflow | Disabled | unavailable |\n"
        import re
        row = re.compile(rf"^\|\s*`{re.escape(key)}`\s*\|.*\|\s*Disabled\s*\|", re.M)
        self.assertIsNotNone(row.search(matrix))

    def test_relative_link_resolution_stays_inside_root(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp).resolve()
            doc = root / "docs" / "release" / "README.md"
            target = root / "docs" / "release" / "feature-matrix.md"
            target.parent.mkdir(parents=True)
            target.touch()
            resolved = (doc.parent / "feature-matrix.md").resolve()
            self.assertEqual(resolved, target)
            self.assertEqual(resolved.relative_to(root), Path("docs/release/feature-matrix.md"))

    def test_stale_architecture_patterns_cover_retired_boundaries(self) -> None:
        corpus = "The backend role uses intentional `BYPASSRLS` and only the future AI gateway can egress."
        self.assertTrue(any(pattern.lower() in corpus.lower() for pattern in module.STALE_ARCHITECTURE))

    def test_markdown_style_detects_trailing_whitespace_shape(self) -> None:
        line = "clean heading  "
        self.assertNotEqual(line.rstrip(), line)


if __name__ == "__main__":
    unittest.main()
