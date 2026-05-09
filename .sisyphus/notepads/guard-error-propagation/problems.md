## 2026-05-08T00:35:00Z Task: 1
No unresolved blockers for Task 1 after evidence correction. Main risk for next tasks is avoiding scope creep into implementation before parser/typechecker RED task slices are completed.

## 2026-05-09 03:54:55Z
- Wine/MSVC guard-shorthand execution is still environment-sensitive on this host: the harness now records known `Unhandled page fault` crashes as skips when they surface through `run_under_wine` error returns, but the underlying Wine instability remains external technical debt.
- The parser red evidence command needs fully qualified unit names; otherwise `cargo test -- --list` can show the tests while the targeted filtered run reports `0 tests`.
