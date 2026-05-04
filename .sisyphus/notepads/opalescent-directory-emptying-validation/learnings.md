# Learnings

## 2026-05-04
- `fs_predicates_matrix` already provides `.op` coverage for `is_directory_sync`.
- Integration E2E coverage for `delete_directory_recursive_sync` was not present before this task.


- [2026-05-04T17:53:44-04:00] Added  E2E test using inline  source, sandbox temp-dir guard, nested fixture, and post-delete checks (, , and fixture root removal).
- [2026-05-04T17:53:57-04:00] Added fs_delete_directory_recursive::fs_recursive_delete_from_op_source with inline op source, nested temp fixture creation, explicit temp-dir prefix guard, exists_after/dir_after output parsing, and fixture-root removal assertion.
- [2026-05-04T18:00:57-04:00] Task 3 workflow test required converting each `list_directory_sync` item from `FilesystemPath` to `string` via `path_to_string(...)` before `join_path_components(base, [child_name])`; direct `[child_entry]` fails type checking as `[FilesystemPath]` vs expected `[string]`.
- [2026-05-04T18:02:39-04:00] Scope-fix learning: temporary compile-debug artifacts must be removed before finalizing Task 3 deliverables; keep Task 3 files strictly limited to allowed test/evidence/notepad paths.
- [2026-05-04T00:00:00Z] Task 4 negative-path coverage worked once the inline .op guard used the indented `else err =>` block form and escaped Rust-format braces as `{{err}}`; `print('ERR_PATH={err}')` must be emitted literally from the Op source, not interpolated by Rust.
- [2026-05-04T18:12:45-04:00] Task 5 showed the three plan-listed cargo filters do not match the integration harness names because the tests are exposed as `tests::fs_delete_directory_recursive::...`; the features themselves still pass when invoked with the fully qualified names.
- [2026-05-04T18:12:45-04:00] Objective path gate result: direct `join_path_components(base, [child_entry])` does not typecheck after `list_directory_sync(base)` because the builtin signatures are `FilesystemPath[]` and `string[]`; the current required replacement remains `path_to_string(child_entry)` before joining.
- [2026-05-04T00:00:00Z] Task 6 introduced a narrow call-resolution bridge for `join_path_components` so `[FilesystemPath]` can satisfy its components parameter in this specific builtin case, enabling ergonomic `.op` iteration from `list_directory_sync` without per-entry conversion.
## Task 7: Readiness Reporting
- Integration tests with exact names require fully qualified names (e.g., `tests::module::test_name`) if they are nested in modules, otherwise `cargo test --exact` will filter them out.
- The `join_path_components` bridge in Task 6 successfully enabled the `.op` workflow without broad type system changes, but introduced a clippy violation (identical if/else blocks).

## Task 7 Learnings
- Importance of maintaining strict scope control during verification tasks.
- Qualified test names are essential for reliable targeted integration testing.
- [2026-05-04T00:00:00Z] Baseline snapshot captured after removing out-of-scope churn; keep future follow-up work in separate atomic commits.