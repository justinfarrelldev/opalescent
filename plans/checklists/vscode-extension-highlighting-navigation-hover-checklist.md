# VS Code Extension Highlighting, Navigation, and Hover Checklist

## Preparation

- [x] Read README.md, STDLIB.md, OPALESCENT_CRASH_COURSE.md, and all files under language-spec/ before implementation.
- [x] Preserve existing uncommitted extension.ts changes outside this task.
- [x] Keep this checklist updated during implementation.

## Syntax highlighting TDD

- [x] Red: add grammar tests for `using`, declaration `let`, distinct `propagate`, and `.types.op` member/property scopes.
- [x] Green: update the TextMate grammar to satisfy the new highlighting tests.
- [x] Refactor/review the grammar for current Opalescent syntax consistency without changing ESLint rules.

## Navigation and hover TDD

- [x] Red: add symbol-index tests for documentation comments, function parameters, scoped local resolution, and definition-site implementation lookup.
- [x] Green: implement scoped symbol collection and lookup helpers.
- [x] Green: wire go-to-definition, find-implementation, and hover providers through the improved lookup helpers.
- [x] Refactor/review navigation behavior against the terminal-simple-editor `status` case.

## Verification and commits

- [x] Run targeted red tests before implementation.
- [x] Run extension tests after implementation.
- [x] Run lint/compile checks for the extension without modifying ESLint rules.
- [x] Perform final code review.
- [x] Make atomic commit(s) without modifying or skipping the pre-commit hook.
- [x] Restore pre-existing uncommitted extension.ts changes after task commits.
