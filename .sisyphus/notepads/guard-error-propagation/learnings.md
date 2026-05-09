## 2026-05-08T00:35:00Z Task: 1
Baseline characterization confirmed current semantics: statement-guard success binding is visible in else clause, and return-err behavior remains non-finalized for guard-specific semantics. Existing parser/typechecker/codegen references are mapped in task-1 impact evidence.

## 2026-05-09 03:54:55Z
- Clippy `needless_borrowed_reference` is satisfied in `src/type_system/tests.rs` by matching iterator items as `matches!(*error, TypeError::TypeMismatch { .. })` instead of `matches!(error, &TypeError::TypeMismatch { .. })`; this matches the existing deref-first test style already used elsewhere in the repo.
- The required `cargo test --all-features` gate can be blocked by Wine host crashes surfacing as `Err(...)` from `run_under_wine`, not just non-zero exit codes inside a successful `WineRun`.
- Parser red captures for guard-error-propagation slice 2 must use fully qualified `parser::tests::...` names; filtering by the bare function name can return zero tests even when the unit is registered.
