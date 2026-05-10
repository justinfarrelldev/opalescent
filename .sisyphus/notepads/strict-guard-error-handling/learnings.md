- Simplified `src/type_system/checker/expressions_guard.rs` by removing the redundant `GuardElseValidation`/`GuardTerminalKind` wrapper state.
- Named guard branches now validate terminal handling directly via the existing terminal-check functions; strict semantics for `propagate`, wrapper returns, and shorthand rejection were preserved.
- `cargo test guard --lib` and `cargo test --features integration guard` passed; `cargo fmt --all -- --check` reported unrelated formatting drift in other files.
- Ran rustfmt across the workspace to clear check-only drift; the strict guard checker stayed semantically unchanged.
- Guard test reruns with --nocapture passed after formatting, confirming the cleanup was formatting-only.

- Full Task 10 verification rerun passed (`cargo build`, `cargo test`, `cargo test --features integration`, strict clippy, and fmt check) and was captured in `.sisyphus/evidence/task-10-full-verification.txt`.
- Strict terminal guard behavior remains enforced end-to-end: non-terminal `propagate(err)` handlers stay rejected while valid wrapper terminal returns preserve source error provenance checks.
- Regression coverage now confirms wrapper-valid and wrapper-invalid guard statement fixtures stay deterministic across checker, codegen, and e2e execution.
