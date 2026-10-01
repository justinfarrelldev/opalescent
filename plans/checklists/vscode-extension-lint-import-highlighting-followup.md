# VS Code Extension Lint, Import Navigation, and Highlighting Follow-up Checklist

## Preparation

- [x] Preserve existing uncommitted `vscode-extension/src/extension.ts` formatting changes outside this task.
- [x] Keep this checklist up to date while implementing the follow-up.

## Red tests

- [x] Add failing grammar metadata tests for `ref`, explicit `errors` clauses, and consistent type-member value scopes.
- [x] Add failing hover documentation tests that strip the `Description:` doc-comment label.
- [x] Add failing local lint tests for unsaved snake_case and documentation-comment issues.
- [x] Add failing import-target navigation tests for `from ./editor.types`.
- [x] Add failing definition-site variable reference navigation tests.
- [x] Run targeted tests and confirm failures before implementation.

## Green implementation

- [x] Update TextMate grammar keyword/error/type-member scopes.
- [x] Strip `Description:` from hover documentation text.
- [x] Implement local unsaved-buffer lint diagnostics and merge them with compiler diagnostics.
- [x] Implement ctrl+click import-target file navigation.
- [x] Implement definition-site variable reference navigation.
- [x] Run extension tests and lint.

## Review and commit

- [x] Review implementation for maintainability and duplicate diagnostics.
- [ ] Commit atomically without modifying or skipping pre-commit hooks.
- [ ] Restore pre-existing uncommitted `extension.ts` formatting changes after commit.
