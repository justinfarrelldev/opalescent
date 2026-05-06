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
- 2026-05-05 22:57:38Z: fixed clippy::needless_borrowed_reference in src/type_system/test_integration_generics.rs by removing the borrowed/ref pattern from the Pair reserved-name assertion so the strict lint gate passes.

## 2026-05-05 Task 4: double arrays
- C/header LSP diagnostics were unavailable in this environment (`clang` initialization failed), so runtime-side changes were verified through the required cargo test/clippy/fmt gates instead.
- A first attempt to build bounds messages in LLVM via `format_interpolated_string` failed at link time because generated programs do not currently link that helper; replacing it with a dedicated runtime helper fixed the diagnostic path cleanly.

## 2026-05-05 Task F3: real manual array CLI QA
- No mismatches or runtime exit-code failures were observed across the required positive array fixtures in this run.

## 2026-05-05 Task F2: immutable array push checker regression
- The regression lived in checker call resolution, where array member calls were accepted by signature alone and the receiver binding mutability was never validated during .
- The first implementation attempt used a nonexistent  helper; switching the diagnostic to the receiver identifier span kept the fix compatible with the existing span API and still pointed at the immutable binding.

## 2026-05-05 Task F2: immutable array push checker regression (correction)
- The regression lived in checker call resolution, where array member calls were accepted by signature alone and the receiver binding mutability was never validated during `opal check`.
- The first implementation attempt used a nonexistent `Span::union` helper; switching the diagnostic to the receiver identifier span kept the fix compatible with the existing span API and still pointed at the immutable binding.
