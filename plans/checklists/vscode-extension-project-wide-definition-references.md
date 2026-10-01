# VS Code Extension Project-wide Definition References Checklist

## Preparation

- [x] Preserve existing uncommitted `vscode-extension/src/extension.ts` formatting changes outside this task.
- [x] Keep this checklist up to date while implementing.

## Red tests

- [x] Add failing tests for project-wide references from a definition site such as `status_text_for` in `labels.op` used from `render.op`.
- [x] Run targeted tests and confirm failure before implementation.

## Green implementation

- [x] Scan project sources, not only the current document, when go-to-definition starts on a definition site.
- [x] Preserve local-scope behavior for function-local variables.
- [x] Run extension tests and lint.

## Review and commit

- [x] Review implementation for maintainability and false positives.
- [ ] Commit atomically without modifying or skipping pre-commit hooks.
- [ ] Restore pre-existing uncommitted `extension.ts` formatting changes after commit.
