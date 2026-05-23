## 2026-05-23T00:43:19-04:00
- Task 1 regression coverage lives cleanly in `tests/integration_e2e/rc_store_leak_regressions.rs` without changing the shared C fixture because the existing harness already emits parser-compatible `rc_store_counter:*` and heap status lines.
- A mutable `board = next_generation(...)` loop over 128 iterations reproduces the leak deterministically before the compiler fix, yielding `alloc=514 free=386 live=128` and `rc_store_live_heap_bytes=5632`.
- An ignored characterization test can document the alias-return limitation by explicitly panicking with observed counters after a successful harness run, keeping the behavior discoverable without gating the normal suite.

## 2026-05-23T04:50:27Z — Task 2 learnings
- Reused `tests/integration_e2e/game_of_life_full_memory_stress.rs` infrastructure to keep stress behavior consistent with existing ignored + `OPAL_RUN_STRESS=1` conventions.
- Parameterizing stress limits with a small `StressLimits` struct enabled a second stress target without duplicating child lifecycle/kill-reap logic.
- The real `game-of-life-full` binary (`board = next_generation(...)`) shows monotonic post-warmup RSS growth pre-fix over a 120s sampling window, producing deterministic red output.

## 2026-05-23T05:00:00Z — Task 3 audit learnings
- `assignment_store_mode` is a single-use selector today, called only from assignment lowering in `src/codegen/statements.rs`.
- `initialize_binding_value` is intentionally shared across `let`, function parameter, and loop-iteration initialization paths, so the audit should not attempt to specialize it.
- The `let` initializer path already avoids double-retaining call results, so the correct fix target stays on assignment behavior only.

## 2026-05-23T04:55:36Z — Task 4 learnings
- `assignment_store_mode` now mirrors the let-init ownership rule for RC-bound call results by taking ownership only when the binding needs RC cleanup.
- Preserving the `Expr::Array` and `reserve(...)` branches kept the change narrow and avoided disturbing existing fresh-owner behavior.
- The focused regression and the existing RC-store suite both stayed green after threading `binding_type` into the assignment path.

## 2026-05-23T05:00:00Z — Task 5 learnings
- The `game_of_life_rc_return_stress` gate behaves correctly: without `OPAL_RUN_STRESS=1` it skips immediately, and with the env var it ran the full 120s window and stayed green.
- The sanitizer workflow evidence must explicitly record when `scripts/array_memory_sanitizer.sh` is absent or not executable so the report stays truthful.
- The Task 5 stress run completed within the 130s hard cap, which preserves the bounded verification contract for this fix.