## 2026-05-07T00:00:00Z Task: task-7-ambiguous-guard-test-project
- Added a dedicated compile-failure fixture project (`test-projects/ambiguous-guard-if`) to keep ambiguous guarded-if coverage isolated and convention-aligned.
- Implemented the integration assertion in `tests/integration_e2e/compile_failures.rs` (no additional module wiring required) to minimize unrelated suite churn.
- Chose direct source compilation (`compile_program`) in the test to assert parser-level `GuardAmbiguousIfElse` diagnostics and rendered miette help text without relying on broader project compile wrappers.
- Kept integration module wiring unchanged because `compile_failures.rs` was already registered in `tests/integration_e2e/tests.rs`.

## 2026-05-07T05:14:21Z Task: task-8-host-guard-shorthand-project
- Chose a real test project () over inline source to match Task 8 acceptance criteria and prove host project compilation/linking path, not just single-file compile flow.
- Kept one named-binding guard in the same fixture so the test validates shorthand behavior and compatibility behavior in one deterministic runtime trace.

## 2026-05-07T05:14:37Z Task: task-8-host-guard-shorthand-project (decision-correction)
- Fixture path chosen explicitly as test-projects/guard-shorthand so acceptance command validates project compile/link/run behavior.
- Marker literals are pinned in test assertions to keep host integration deterministic and avoid false positives.


## 2026-05-07T05:19:19Z Task: task-9-wine-guard-shorthand
- Reused the existing Windows/Wine harness style directly in `windows_wine.rs` instead of introducing new helper flow, so prereq gating, skip evidence capture, and Wine-host limitation handling remain consistent across tests.
- Chose to capture workspace snapshot from `test-projects/guard-shorthand/target` for this test because guard-shorthand validation is stdout-marker focused and does not require bespoke filesystem artifact assertions.
- 2026-05-07: Chose minimal-risk refactor by moving only guard-statement methods from `statements.rs` to `statements_guard.rs`; kept other statement parsers in place to avoid behavior drift.
