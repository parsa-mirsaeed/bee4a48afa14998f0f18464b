#!/usr/bin/env python3
"""Select the smallest safe Playwright smoke slice for changed files.

An unknown or ambiguous browser-sensitive path returns the complete @smoke tier.
Selectors only narrow known ownership boundaries; they never select zero tests.
"""

from __future__ import annotations

import argparse
from typing import Iterable


def select(files: Iterable[str]) -> str:
    paths = sorted({p.strip().replace('\\', '/') for p in files if p.strip()})
    if not paths:
        return '@smoke'

    if any(
        p.startswith('tests/e2e/')
        or p.startswith('scripts/ci/run_browser_')
        or p == 'scripts/ci/verify_browser_harness.sh'
        for p in paths
    ):
        return '@smoke'

    tags: set[str] = set()
    ambiguous = False

    for path in paths:
        if path.startswith('packages/api/src/middleware/auth') or path.startswith('packages/api/src/middleware/authorization') or path.startswith('packages/api/src/middleware/endpoint_authorization'):
            tags.update({'@auth', '@authorization'})
            continue

        if path.startswith('packages/web/src/') and any(token in path for token in ('login', 'auth', 'session')):
            tags.add('@auth')
            continue

        if path.startswith('packages/web/src/') and any(
            token in path for token in (
                'school_manager/user_creation',
                'school_manager/knowledge_upload',
                'teacher/assignments',
                'teacher/submissions',
                'student/assignments',
                'parent/children',
            )
        ):
            tags.add('@workflow-truth')
            continue

        if path.startswith('packages/api/src/') and any(
            token in path for token in (
                'server_functions/user_management',
                'server_functions/assignment',
                'server_functions/submission',
                'server_functions/parent_scoped',
                'handlers/knowledge_upload',
            )
        ):
            tags.add('@workflow-truth')
            continue

        # Shared navigation/shell/form changes can affect several role journeys.
        # Keep the whole smoke tier until narrower tagged coverage exists.
        ambiguous = True

    if ambiguous or not tags:
        return '@smoke'
    return '|'.join(sorted(tags))


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument('files', nargs='*')
    parser.add_argument('--files-from')
    args = parser.parse_args()
    files = list(args.files)
    if args.files_from:
        with open(args.files_from, encoding='utf-8') as fh:
            files.extend(line.rstrip('\n') for line in fh)
    print(select(files))
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
