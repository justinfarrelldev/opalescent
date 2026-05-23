# Task 7 verification

## Modified files
- `src/codegen/functions_call.rs`
- `src/codegen/scope_tracker.rs`
- `tests/integration_e2e/call_temp_leak_regressions.rs`

## Diagnostics
- `lsp_diagnostics src/codegen/functions_call.rs`: no diagnostics
- `lsp_diagnostics src/codegen/scope_tracker.rs`: no diagnostics
- `lsp_diagnostics tests/integration_e2e/call_temp_leak_regressions.rs`: rust-analyzer unlinked-file hint only

## Targeted selectors
- `cargo test --features integration call_temp_owned_arg_freed_on_return` ✅
- `cargo test --features integration call_temp_owned_arg_freed_on_propagate` ✅
- `cargo test --features integration call_temp_mixed_disposition` ✅
- `cargo test --features integration call_temp_nested_later_failure_cleanup` ✅
- `cargo test --features integration call_temp_take_owned_no_double_free` ✅

## Workspace verification
- `cargo test --workspace` ✅

## Notes
- Call lowering now opens a temporary scope for direct ephemeral owned call arguments and routes cleanup through existing malloc-string scope cleanup helpers.
- Regression harness now keys pass/fail on sanitizer markers rather than process exit code so propagate-entry fixtures can validate leak cleanup even when entry error handling exits non-zero by design.
- `scope_tracker::expr_requires_malloc_string_cleanup` now treats `Expr::StringInterpolation` as owned malloc-backed string for binding cleanup.
