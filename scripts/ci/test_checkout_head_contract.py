#!/usr/bin/env python3
"""Regression guard: PR workflows must prove the *checked-out* exact head.

GitHub's default PR checkout is a merge-preview ref, not the PR head. A
workflow's PR_HEAD_SHA environment variable does not correct its checkout.
"""
from pathlib import Path
import os
import re
import subprocess
import unittest

ROOT = Path(__file__).resolve().parents[2]
WORKFLOWS = ROOT / ".github" / "workflows"
CHECKOUT = re.compile(r"^\s*- uses: actions/checkout@")
EXPECTED_REF = re.compile(
    r"ref: \$\{\{ github\.event\.pull_request\.head\.sha(?: \|\| github\.sha)? \}\}"
)
CRITICAL = [
    "ci.yml",
    "full-validation.yml",
    "package.yml",
    "release-docs.yml",
    "production-foundation.yml",
]


class ExactHeadCheckoutContract(unittest.TestCase):
    def test_every_checkout_explicitly_pins_pr_head(self):
        seen = 0
        for workflow in sorted(WORKFLOWS.glob("*.yml")):
            lines = workflow.read_text(encoding="utf-8").splitlines()
            for index, line in enumerate(lines):
                if not CHECKOUT.search(line):
                    continue
                seen += 1
                following = "\n".join(lines[index + 1:index + 6])
                self.assertRegex(
                    following, EXPECTED_REF,
                    f"{workflow.name}:{index + 1} does not pin PR head; "
                    "default PR checkout silently fetches merge-preview",
                )
        self.assertGreater(seen, 20, "checkout inventory unexpectedly shrank")

    def test_critical_jobs_assert_actual_checkout_identity(self):
        for name in CRITICAL:
            data = (WORKFLOWS / name).read_text(encoding="utf-8")
            # Every critical job has an identity assertion before any proof.
            checkout_count = len(CHECKOUT.findall(data)) if False else len(
                re.findall(r"(?m)^\s*- uses: actions/checkout@", data)
            )
            assert_count = data.count("bash scripts/ci/stage1_verify_proof_head.sh")
            self.assertGreater(checkout_count, 0, name)
            self.assertGreaterEqual(
                assert_count, checkout_count,
                f"{name}: not every checked-out job asserts actual git HEAD",
            )

    def test_browser_runner_will_not_accept_pretend_head(self):
        script = (ROOT / "scripts/ci/run_browser_e2e.sh").read_text(encoding="utf-8")
        self.assertIn('PROOF_HEAD_SHA="${E2E_HEAD_SHA}" bash scripts/ci/stage1_verify_proof_head.sh', script)

    def test_identity_script_rejects_mismatch_and_accepts_actual_sha(self):
        script = ROOT / "scripts/ci/stage1_verify_proof_head.sh"
        head = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip()
        self.assertRegex(head, r"^[a-f0-9]{40}$")

        bad = subprocess.run(["bash", str(script)], cwd=ROOT, text=True,
                             env={**os.environ, "PROOF_HEAD_SHA": "0" * 40},
                             capture_output=True, check=False)
        self.assertNotEqual(bad.returncode, 0)
        self.assertIn("Proof rejected", bad.stderr)

        good = subprocess.run(["bash", str(script)], cwd=ROOT, text=True,
                              env={**os.environ, "PROOF_HEAD_SHA": head},
                              capture_output=True, check=False)
        self.assertEqual(good.returncode, 0, good.stderr)
        self.assertIn(head, good.stdout)


if __name__ == "__main__":
    unittest.main(verbosity=2)
