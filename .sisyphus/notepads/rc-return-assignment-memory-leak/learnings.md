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
## 2026-05-23T05:13:30Z — Task 6 learnings
- Task 6 evidence should include both initial full-suite run and targeted reruns so failures are reproducible and auditable without hiding instability.
- `cargo test` and `game_of_life_rc_return_stress` remained green (stress duration 120s), while integration failures were isolated to unrelated fs_* tests in this environment.
- Guardrail audit confirmed no diff in `test-projects/game-of-life-full/src/main.op`, `runtime/opal_rc.c`, `runtime/opal_rc.h`, or `src/codegen/control_flow.rs`.


- [2026-05-23 05:48:25Z] fs blocker root cause: array RC hook predicate was too broad. String/FilesystemPath/Pair elements were flowing through hook sites (`opal_rc_inc/dec`) that require RC payload pointers. Tightening hook eligibility to nested array elements fixed deterministic fs corruption while preserving array-child RC semantics.

- [2026-05-23 05:53:36Z] Restored assignment call-ownership gating in `assignment_store_mode` to use RC-cleanup predicate semantics (`binding_requires_rc_cleanup(binding_type)`) via helper wrapper, while preserving `Expr::Array` and `reserve(...)` TakeOwned branches.

- [2026-05-23 F4] Final scope audit passed because the committed reassignment fix stayed anchored to `assignment_store_mode` ownership gating, while the Task 6 follow-up remained a narrow RC-hook eligibility correction rather than a redesign.

## 2026-05-23T06:00:00Z — F2 code quality review learnings
- The assignment ownership fix in `src/codegen/statements.rs` is readable and maintainable: binding-type threading plus RC-cleanup-gated call `TakeOwned` is explicit and aligns with let/assignment ownership intent.
- Required quality scans found no `TODO/FIXME/HACK/as any/@ts-ignore/console.log` markers in the reviewed changed production/test files.
- For strict final-wave criteria, even valid follow-up production fixes in adjacent files can invalidate a “surgical in statements.rs” claim unless explicitly broadened in the gate definition.

## 2026-05-23T06:13:08Z — F1 plan compliance audit
- Final-wave F1 re-audit approved the plan state because Task 6 explicitly allowed a focused follow-up fix and rerun, and the evidence commit `9e22c43` records that post-fix verification.
- The audited guardrail commands remained clean for `test-projects/game-of-life-full/src/main.op`, `runtime/opal_rc.c`, `runtime/opal_rc.h`, and `src/codegen/control_flow.rs`, while the tests-first reassignment chain still appears in recent git history.

## 2026-05-23T06:25:00Z — F3 real manual QA learnings
- Full required F3 command sequence passed in strict order with exit code 0 for cargo test, cargo test --features integration, and the opt-in stress target.
- The stress test  completed in 120.46s, remaining within the 130s acceptance budget.
- Sanitizer execution must be reported as skipped when script exists but lacks executable permission; this preserves truthful QA evidence and avoids fabricated green signals.

## 2026-05-23T06:25:00Z — F3 real manual QA learnings
- Full required F3 command sequence passed in strict order with exit code 0 for cargo test, cargo test --features integration, and the opt-in stress target.
- The stress test `game_of_life_rc_return_stress` completed in 120.46s, remaining within the 130s acceptance budget.
- Sanitizer execution must be reported as skipped when script exists but lacks executable permission; this preserves truthful QA evidence and avoids fabricated green signals.

## 2026-05-23T06:25:00Z — F2 rerun learnings
- Re-evaluating F2 against the plan text plus final commit chain (not just the immediate fix commit) changes the quality outcome: small, cohesive follow-up corrections can still satisfy maintainability/scope quality.
- `3de44f8` is best classified as a verification-driven RC-hook eligibility correction, not broad refactor churn, because it is localized and semantically coupled to the same RC ownership surface.
- For strict final-wave reviews, commit-range context (`a4640b1^..3de44f8`) is essential to distinguish acceptable focused follow-ups from true scope drift.
