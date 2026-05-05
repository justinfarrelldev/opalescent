# Issues

## 2026-05-05 Task: bootstrap
- Repository contains unrelated modified/untracked files from previous sessions; avoid including them in continuation scope.
- Serialized integration commands with `--test-threads=1` are required to avoid shared `target/program` race behavior.

## 2026-05-05 Task 2: Pair smoke coverage
- `.op` LSP diagnostics are unavailable in this environment because no language server is configured for `.op`; end-to-end cargo integration runs were used as the verification path for `test-projects/array-pair/src/main.op`.
- Workspace contains unrelated modified/untracked files from prior sessions, so commit staging must stay scoped to the Task 2 fixture, test, evidence, and notepad updates only.

## 2026-05-05T18:48:47-04:00 Task 3: array zip
- The first RED capture was a false pass because `array_zip_runs` had not been added to `tests/array_integration.rs` yet; rerunning after wiring the targeted test produced the correct failure (`array method 'zip' is not implemented yet`).
- Full-gate verification exposed a pre-existing clippy blocker in `src/type_system/test_integration_generics.rs`; fixing that lint was necessary to satisfy the required `cargo clippy --all-targets --all-features -- -D warnings` gate for this task.
- `cargo fmt --all -- --check` reformatted several touched Rust files after implementation, so final verification must always include a formatting pass before the last check gate.
