# VS Code Extension Type Member and Definition Reference Follow-up Checklist

## Preparation

- [x] Confirm working tree is clean and preserve existing stashed extension formatting changes.
- [x] Keep this checklist up to date while implementing.

## Red tests

- [x] Add failing symbol-index tests for type variant navigation (`EditorStatus.BellFailed`).
- [x] Add failing definition-site top-level reference navigation tests (`named_key_text`).
- [x] Add failing code-lens title tests for `Build Project` and `Run Program`.
- [x] Run targeted tests and confirm failures before implementation.

## Green implementation

- [x] Collect type-member symbols from `.types.op` files.
- [x] Resolve type-member uses such as `EditorStatus.BellFailed` to their member declarations.
- [x] Return references when go-to-definition starts on any definition site with references.
- [x] Shorten entry code-lens titles to `Build Project` and `Run Program`.
- [x] Run extension tests and lint.

## Review and commit

- [x] Review implementation for maintainability and false positives.
- [x] Commit atomically without modifying or skipping pre-commit hooks.
- [x] Restore pre-existing uncommitted `extension.ts` formatting changes after task commits if no more extension work remains.
