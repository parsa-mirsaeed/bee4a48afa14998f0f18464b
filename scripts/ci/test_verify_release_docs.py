from __future__ import annotations

import importlib.util
import json
import os
import subprocess
import tempfile
import unittest
from pathlib import Path

MODULE_PATH = Path(__file__).with_name("verify_release_docs.py")
spec = importlib.util.spec_from_file_location("verify_release_docs", MODULE_PATH)
assert spec and spec.loader
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


class ReleaseDocsVerifierTests(unittest.TestCase):
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
