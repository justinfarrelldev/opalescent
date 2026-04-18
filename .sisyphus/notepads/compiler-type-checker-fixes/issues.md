# Issues — compiler-type-checker-fixes

## [2026-04-17] Task 3 commit hook failure due to Task 2 code
- Pre-commit clippy checks failed on `src/type_system/checker/expressions.rs` because the existing Task 2 changes used `unwrap()` in `type_check_expr` and had duplicate match arms.
- This blocked commit creation for Task 3 even though `cargo build`/`cargo test` passed.
- Resolution deferred: Task 2 file remains included per orchestrator requirement; commit could not be created in this task scope without violating "do not modify type checker code".

## [2026-04-18] Scope fidelity audit issue
- Task 1 implementation commit includes three additional `.sisyphus/evidence/*` files that are not part of the Task 1 spec requirement (which called for only `src/type_system/module_resolver.rs`).
- This introduces unaccounted file changes and breaks strict 1:1 scope fidelity for that task.
