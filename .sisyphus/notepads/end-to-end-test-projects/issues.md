# Issues

## [2026-04-11] Known Issues (Pre-existing)

- `resolve_callee_function` silent i64 fallback — fixed in Task 3
- `Expr::StringInterpolation` missing match arm — fixed in Task 6
- Import system has no codegen handler — fixed in Task 12
- `string_to_int32` wrong signature in type checker — fixed in Task 13
- `random_int32` wrong signature in type checker — fixed in Task 13
- `src/type_system/test_integration.rs:460` uses guard with string_to_int32 — fixed in Task 13

## Guardrails Reminders
- `cargo make test` MUST pass after EVERY task
- `cargo make lint` MUST pass (zero warnings)
- `scripts/check-line-count.sh` MUST pass
- No `unwrap/expect/panic/todo/unimplemented`
- No `as` conversions
- No `str.to_string()` — use `to_owned()` or `String::from()`
- No `HashMap` in core modules — use `BTreeMap`
- No `#[allow(...)]` — use `#[expect(..., reason = "...")]`
- No `--no-verify` on git
- No newly created files > 500 lines (1000 for test files)
