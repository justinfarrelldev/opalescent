- `cargo fmt --all -- --check` surfaced pre-existing formatting differences in several unrelated files (`control_flow.rs`, `expr_collections.rs`, `expressions.rs`, `module_checking.rs`, `statements.rs`, `errors.rs`, `type_system/tests.rs`, and some integration tests).
- The guard refactor itself did not introduce diagnostics in `src/type_system/checker/expressions_guard.rs`.
- The integration guard suite emitted a transient fixture panic message during execution, but Cargo still reported success for the full run.

- Working tree still includes broad pre-existing evidence and planning artifacts under `.sisyphus/evidence` and `.sisyphus/plans`, so commit slicing must stage only strict-guard Task 10 scope.
- `git status --porcelain` remains non-clean prior to commit batching by design; final clean-state evidence must be generated after all Task 10 commits.
- Recent history includes a dual-subject commit line (`feat(delete-downloads)... feat(move-downloads)...`), so keep upcoming commit subjects singular and atomic.
- Task 11 subagent attempts repeatedly failed due orchestration/session errors (`Bad Request`) and were completed manually in Atlas session.
