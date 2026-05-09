# Task 11 Migration Checklist

## 2026-05-09T09:18:20Z

### Scope searched
- `test-projects/**`
- `tests/**`
- `src/type_system/tests.rs`

Search patterns executed:
- `return err`
- `propagate err`
- `else err`
- string-error assumptions (`CoreType::String`, `string error`)

### Migrated occurrences

1. **File:** `test-projects/fs-path-manipulation/src/path_ops/absolute.op`
   - **Old behavior/pattern:** guard error clause used direct `return err`:
     - `guard absolute_path_sync(path_from(raw)) into value else err =>`
     - `return err`
   - **New behavior/pattern:** convert bound error to textual return value without direct return-propagation:
     - `guard absolute_path_sync(path_from(raw)) into value else err =>`
     - `return '{err}'`
   - **Rationale:** preserves existing helper contract (“returns runtime error text”) while complying with Task 7/8 semantics that reject direct `return err` in guard error clauses.

### Reviewed and intentionally retained (semantics-valid)

- Compile-fail fixtures/tests that intentionally assert rejection of `return err` were retained unchanged:
  - `test-projects/guard-stmt-return-err-banned/src/main.op`
  - `tests/integration_e2e/guard_optional_binding.rs`
  - `src/type_system/tests.rs`
- Existing `propagate err` usages in guard-clause terminal position were retained unchanged where they are validating the new semantics.
- Existing `else err` guard handlers that locally handle errors (without forbidden direct `return err`) were retained.

### Verification summary
- `cargo test --features integration` passed (see `task-11-integration-green.txt`).
- `cargo test --all-features` failed only at known Wine flake `tests::windows_wine::tests::wine_msvc_guard_shorthand` (host crash/timeout), with no additional semantic regressions (see `task-11-all-features-green.txt`).
