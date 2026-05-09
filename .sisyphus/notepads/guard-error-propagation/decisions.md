## 2026-05-09 Task 1 follow-up
- Kept Task 1 scoped to characterization-only artifacts: `src/type_system/tests.rs`, `.sisyphus/evidence/task-1-*.{md,txt}`, and guard plan notepads.
- Repaired RED evidence by temporarily flipping a real characterization test to the future-semantics expectation, capturing the failing lib-test run, then immediately restoring the final baseline assertion before green verification.
- Preserved the current `return err` baseline as a characterization test asserting today’s actual failure mode (`TypeMismatch` from `string` to `unit`) rather than implementing any new semantics.
