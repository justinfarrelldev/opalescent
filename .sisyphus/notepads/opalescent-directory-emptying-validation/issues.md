# Issues

## 2026-05-04
- Attempting to register `fs_delete_directory_recursive` before the file exists breaks `cargo test --test integration_e2e`; the module line has to wait for the module file.


- [2026-05-04T17:53:44-04:00] Initial probe compile failed because the inline  error list used ; corrected to  plus the builtin recursive-delete error set (, , , ).
- [2026-05-04T17:53:57-04:00] Corrected inline op error signature after first compile failure: use IsNotADirectoryError and recursive delete builtin error set instead of NotADirectoryError.
- [2026-05-04T18:00:57-04:00] Task 3 first workflow attempt failed front-end parse/type checks due to (1) `entry` loop variable conflicting with reserved keyword and (2) passing `FilesystemPath[]` directly to `join_path_components` expecting `string[]`; fixed by `for child_entry in entries` + `path_to_string(child_entry)`.
- [2026-05-04T18:02:39-04:00] Scope correction applied: removed out-of-scope file `test-projects/_t26_debug_empty_workflow/src/main.op` and cleaned its empty directories; required Task 3 exact test re-run passed afterward.
- [2026-05-04T00:00:00Z] Task 4 compile failures were parser-related at first: braces after `else err =>` were rejected (`expected indentation block after '=>' in guard statement`), and later the Rust `format!` string consumed `{err}`; final fix was an indented guard block plus `{{err}}` in Rust so the Op source receives `ERR_PATH={err}` literally.
- [2026-05-04T18:12:45-04:00] Task 5 diagnosis found a verification-command issue: the plan’s bare test filters (`fs_recursive_delete_from_op_source`, etc.) currently run zero tests because the integration harness exposes them as `tests::fs_delete_directory_recursive::...`.
- [2026-05-04T18:12:45-04:00] Task 5 path gate probe failed exactly at type checking with `expected '[string]', found '[FilesystemPath]'` for `join_path_components(base, [child_entry])`, which activates the broader path API follow-up under the plan’s ergonomics rule.
- [2026-05-04T00:00:00Z] Task 6 scope-control run showed a pre-existing workspace diff in `tests/integration_e2e/tests.rs` (`mod fs_delete_directory_recursive;`) alongside Task 6 changes; this was documented explicitly to avoid attribution drift.
## Task 7: CI Verification Failures
- **Clippy**: Detected 4 errors.
  - `src/type_system/checker/call_resolution.rs`: `if_same_then_else` in the `join_path_components` bridge.
  - `src/codegen/adts.rs`, `src/compiler/compiler_helpers.rs`, `src/compiler.rs`: `needless_borrowed_reference`.
- **Fmt**: Formatting check failed on multiple files.
- **Test Command**: The plan's suggested command for `fs_predicates_matrix` fails to run tests because it lacks the module prefix.

## Task 7 Issues
- Discovered clippy and fmt regressions during final readiness check that required isolation and fixing.
- [2026-05-04T00:00:00Z] Baseline snapshot captured after reverting out-of-scope churn; remaining follow-up work must stay uncommitted.