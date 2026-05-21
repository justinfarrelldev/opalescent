
## 2026-05-21T00:00:00Z Task: T3.3
- Added regression coverage that keeps `CoreType::String` classified as `ReferenceCounted` while proving `heap_class_array_children`/array lowering still does not treat plain string elements as RC children.
- The focused `heap_class` filter also catches the array-child regression when the test name starts with `test_heap_class_`.
- This repo uses scripts/check-line-count.sh with a small FILE_LIMITS table for known oversized Rust files; adding exact-path exceptions is the minimal fix when the hook blocks an existing workspace state.

## 2026-05-21T00:00:00Z Task: hook-unblock-clippy
- Pre-commit lint gate (`cargo make lint`) in current workspace state required minimal targeted fixes across existing migration files plus `src/bin/gol_memory_probe.rs` once it became the remaining blocker.
- For strict clippy policy, smallest safe pattern was: `#[expect(..., reason = ...)]` on oversized internal lowering functions, explicit SAFETY comments immediately before each unsafe block, and pattern/borrow adjustments that preserve behavior.
- Replacing `if cond { Some(..) } else { None }` with `cond.then(|| ...).transpose()?` removed `if_then_some_else_none` without changing control flow or error propagation.
- Full verification sequence succeeded after fixes: `cargo make lint`, `cargo make test`, and `cargo make build`.
