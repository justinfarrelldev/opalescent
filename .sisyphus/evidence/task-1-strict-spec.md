# Task 1 Strict Guard Spec Inventory

Generated on 2026-05-12 from the current workspace baseline.

## Commands executed

```text
cargo test guard --lib -- --nocapture
cargo test --features integration guard -- --nocapture
cargo test --features integration guard_stmt_print_only_project_emits_missing_terminal_diagnostic -- --nocapture
cargo test --features integration guard_stmt_ignored_alias_project_emits_missing_terminal_diagnostic -- --nocapture
cargo test --features integration guard_stmt_propagate_call_valid_project_compiles_links_and_runs -- --nocapture
cargo test --features integration delete_downloads_strict_project_compiles_and_runs_with_strict_terminal_handlers -- --nocapture
```

## Test files in scope

- `tests/integration_e2e/guard_stmt.rs`
- `tests/integration_e2e/guard_shorthand.rs`
- `src/type_system/tests.rs`

## Valid forms that must stay covered

- `guard-stmt-propagate-err` — named guard error clause with side effects followed by final `propagate err` (runtime-pass).
- `guard-stmt-propagate-call-valid` — shorthand `propagate <call>()` outside named guard error clauses (runtime-pass).
- `guard-stmt-wrapper-valid` — direct typed wrapper source return using exact `source: err` (runtime-pass).
- `guard-stmt-typed-binding` — named success binding remains available after successful guard completion (runtime-pass).
- `delete-downloads-strict` — restored strict fixture directory exists and the focused runtime integration test currently passes.

## Invalid forms that must compile-fail deterministically

- `guard-stmt-print-only` — print-only named guard error clause should fail with `TypeError::GuardErrorClauseMissingTerminal`.
- `guard-stmt-ignored-alias` — `_ignored_*` alias handling should fail with `TypeError::GuardErrorClauseMissingTerminal`.
- `guard-stmt-only-propagate` — named guard clause using only shorthand guidance path should fail with `TypeError::GuardShorthandRequired`.
- `guard-stmt-return-err-banned` — `return err` should fail with `TypeError::GuardReturnErrInvalid`.
- `guard-stmt-wrapper-invalid-alias` — wrapper `source` alias should fail with `TypeError::GuardWrapperSourceInvalid`.
- `guard-stmt-wrapper-invalid-shadowed` — shadowed wrapper `source` should fail with `TypeError::GuardWrapperSourceInvalid`.
- `guard-stmt-wrapper-invalid-missing-source` — wrapper without exact `source: err` should fail with `TypeError::GuardWrapperSourceInvalid`.
- `guard-stmt-success-binding-leak` — success binding use inside the error clause should fail with the existing scope diagnostic.

## Current truthful baseline notes

- `tests/integration_e2e/guard_stmt.rs` no longer contains any baseline-divergence fallback logic.
- The focused runtime tests for `guard-stmt-propagate-call-valid` and `delete-downloads-strict` pass in this workspace.
- The focused compile-fail tests for `guard-stmt-print-only` and `guard-stmt-ignored-alias` currently fail because the compiler still compiles those fixtures. This is the active strict regression now surfaced directly by the deterministic assertions.
- `task-1-integration-inventory.txt` contains unrelated baseline failures in the broader integration guard run (`fs_state_guard::manifest_diff`) in addition to the strict-guard regressions above; that output is preserved verbatim as command evidence.
