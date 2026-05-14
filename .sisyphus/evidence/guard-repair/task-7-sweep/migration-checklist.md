# Task 7 Guard Sweep Migration Checklist

Date: 2026-05-13

## Scope scanned
- `tests/`
- `test-projects/`
- `src/type_system/tests.rs`
- `src/type_system/test_integration.rs`
- docs/proposals with guard usage (`README.md`, `error-handler-proposals/**`, `stdlib-proposals/**`, `ARRAY_FEATURES.md`)

## Reviewed/touched guard occurrences and disposition

### A) Core guard semantics suites (reviewed, no Task-7 code changes)
- `tests/integration_e2e/guard_stmt.rs` — reviewed; strict compile-fail + legal sibling pass coverage already aligned.
- `tests/integration_e2e/guard_optional_binding.rs` — reviewed; strict behavior and negative `return err` coverage preserved.
- `src/type_system/tests.rs` — reviewed; strict variant/span assertions preserved.

### B) Guard inline `.op` templates inside integration tests (reviewed, no Task-7 code changes)
- `tests/integration_e2e/fs_append_file_string.rs`
- `tests/integration_e2e/fs_copy_file.rs`
- `tests/integration_e2e/fs_delete_directory_recursive.rs`
- `tests/integration_e2e/fs_metadata.rs`
- `tests/integration_e2e/fs_normalize_path.rs`
- `tests/integration_e2e/fs_read_bytes.rs`
- `tests/integration_e2e/fs_read_first_line.rs`
- `tests/integration_e2e/fs_read_text.rs`
- `tests/integration_e2e/fs_write_file_bytes.rs`
- `tests/integration_e2e/fs_write_file_string.rs`

Disposition: reviewed under strict semantics; all-features regression gate is green, no further migration edits required in Task 7.

### C) Guard fixtures/projects (reviewed, no Task-7 code changes)
- `test-projects/delete-downloads/src/main.op` — intentionally invalid compile-fail fixture retained.
- `test-projects/delete-downloads-strict/src/main.op` — intentionally invalid compile-fail fixture retained.
- `test-projects/delete-downloads-legal/src/main.op` — legal sibling retained (`print(...)` + final `propagate err`).
- `test-projects/delete-downloads-strict-legal/src/main.op` — legal sibling retained (`print(...)` + final `propagate err`).
- Additional reviewed guard fixtures:
  - `test-projects/_absolute_path_sync/src/main.op`
  - `test-projects/_absolute_path_sync/src/resolver.op`
  - `test-projects/_fs_append_log/src/main.op`
  - `test-projects/_fs_read_text_lines/src/main.op`
  - `test-projects/ambiguous-guard-if/src/main.op`
  - `test-projects/fs-directory-operations/src/main.op`
  - `test-projects/fs-directory-operations/src/operations/list.op`
  - `test-projects/fs-markdown-roundtrip/src/main.op`
  - `test-projects/fs-path-manipulation/src/main.op`
  - `test-projects/fs-path-manipulation/src/path_ops/absolute.op`
  - `test-projects/guard-stmt-ignored-alias/src/main.op`
  - `test-projects/guard-stmt-only-propagate/src/main.op`
  - `test-projects/guard-stmt-print-only/src/main.op`
  - `test-projects/guard-stmt-propagate-err/src/main.op`
  - `test-projects/guard-stmt-return-err-banned/src/main.op`
  - `test-projects/guard-stmt-success-binding-leak/src/main.op`
  - `test-projects/guard-stmt-wrapper-invalid-alias/src/wrapper.op`
  - `test-projects/guard-stmt-wrapper-invalid-missing-source/src/wrapper.op`
  - `test-projects/guard-stmt-wrapper-invalid-shadowed/src/wrapper.op`
  - `test-projects/guard-stmt-wrapper-valid/src/wrapper.op`
  - `test-projects/move-downloads/src/main.op`

Disposition: reviewed for strict-guard terminal compatibility and expected negative/positive intent; no new edits required in Task 7.

### D) Type-system integration guard coverage (reviewed)
- `src/type_system/test_integration.rs` — reviewed; strict named-guard behavior remains compatible with current expectations.

### E) Docs/proposals (reviewed, no changes)
- `README.md`
- `error-handler-proposals/COMPARISON.md`
- `error-handler-proposals/core-local-handlers/proposal.md`
- `error-handler-proposals/cleanup-probe-ignore/proposal.md`
- `error-handler-proposals/observable-handlers/proposal.md`
- `error-handler-proposals/propagation-only/proposal.md`
- `error-handler-proposals/typed-match-recovery/proposal.md`
- `stdlib-proposals/**` (guard/propgation proposal examples)
- `ARRAY_FEATURES.md`

Disposition: proposal/documentation content only; not active fixture/runtime scope for Task 7 migration.

## Verification status for Task 7 gate
- `cargo test --features integration guard_stmt -- --nocapture` — PASS (16 passed, 0 failed; see `guard-stmt.txt`)
- `cargo test --all-features` — PASS (suite green; see `all-features.txt`)

## Conclusion
Task 7 sweep confirms strict named-guard terminal semantics are preserved in reviewed scope, invalid originals remain compile-fail coverage, legal siblings remain deterministic pass coverage, and no additional guard migration edits were required in this closure pass.