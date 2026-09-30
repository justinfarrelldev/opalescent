# VS Code Extension Parity Checklist

## Planning

- [x] Read README.md, STDLIB.md, OPALESCENT_CRASH_COURSE.md, and language-spec files before implementation.
- [x] Create implementation plan.
- [x] Create live checklist.

## Phase 1: Compiler-backed machine diagnostics

- [x] Add failing Rust tests for JSON diagnostic conversion and project-aware check behavior.
- [x] Implement machine-readable diagnostic data model.
- [x] Add `opal check --json` for single-file checks.
- [x] Add `opal check --project [path] --json` for project-aware checks.
- [x] Preserve existing human-readable `opal check` behavior.
- [x] Run Rust tests for touched compiler/app code (`cargo test --lib`).

## Phase 2: Extension TypeScript foundation

- [x] Add TypeScript/pnpm scaffolding and tests.
- [x] Add failing extension helper tests.
- [x] Implement project/file detection helpers.
- [x] Implement compiler JSON parser helpers.
- [x] Implement command argument helpers.
- [x] Implement conservative symbol index helpers.
- [x] Run extension unit tests (`pnpm test` in `vscode-extension`).

## Phase 3: VS Code integration

- [x] Implement activation entry point.
- [x] Implement Opalescent binary resolution and prompt fallback.
- [x] Implement diagnostics collection with compiler-backed lint/check.
- [x] Implement formatting provider and command via `opal fmt`.
- [x] Implement command-palette lint/check command.
- [x] Implement command-palette build command.
- [x] Implement command-palette run command.
- [x] Implement code lenses above entry functions for Build and Run.
- [x] Implement go-to-definition provider.
- [x] Implement find-implementation provider.
- [x] Run TypeScript compilation (`pnpm test` in `vscode-extension`).

## Phase 4: Packaging and grammar parity

- [x] Refresh package contributions and extension settings.
- [x] Update TextMate grammar for current public syntax.
- [x] Update language configuration for current indentation/comment behavior.
- [x] Add `.vscodeignore`.
- [x] Add cargo-make tasks to build/test/package VSIX.
- [x] Run `cargo make vscode-extension-vsix`.

## Review and commits

- [ ] Perform code review of implemented work.
- [ ] Run relevant regression tests.
- [ ] Make atomic commits without modifying/skipping pre-commit hook.
