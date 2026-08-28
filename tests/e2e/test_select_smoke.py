#!/usr/bin/env python3
import importlib.util
import pathlib
import unittest

MODULE_PATH = pathlib.Path(__file__).with_name('select_smoke.py')
SPEC = importlib.util.spec_from_file_location('select_smoke', MODULE_PATH)
selector = importlib.util.module_from_spec(SPEC)
assert SPEC and SPEC.loader
SPEC.loader.exec_module(selector)


class BrowserSelectorTests(unittest.TestCase):
    def test_login_selects_auth(self):
        self.assertEqual(selector.select(['packages/web/src/views/login.rs']), '@auth')

    def test_backend_auth_selects_auth_and_authorization(self):
        self.assertEqual(selector.select(['packages/api/src/middleware/auth.rs']), '@auth|@authorization')

    def test_assignment_workflow_selects_workflow_truth(self):
        self.assertEqual(selector.select(['packages/web/src/views/role_based/teacher/assignments.rs']), '@workflow-truth')

    def test_known_mixed_paths_union_tags(self):
        self.assertEqual(
            selector.select([
                'packages/web/src/views/login.rs',
                'packages/web/src/views/role_based/teacher/assignments.rs',
            ]),
            '@auth|@workflow-truth',
        )

    def test_shell_navigation_stays_full_smoke(self):
        self.assertEqual(selector.select(['packages/web/src/views/role_based/components/sidebar.rs']), '@smoke')

    def test_harness_change_stays_full_smoke(self):
        self.assertEqual(selector.select(['tests/e2e/playwright.config.ts']), '@smoke')

    def test_unknown_escalation_stays_full_smoke(self):
        self.assertEqual(selector.select(['future/browser-contract.yaml']), '@smoke')

    def test_empty_input_stays_full_smoke(self):
        self.assertEqual(selector.select([]), '@smoke')


if __name__ == '__main__':
    unittest.main(verbosity=2)
