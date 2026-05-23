# Final Wave F4 — Scope Fidelity Check

VERDICT: APPROVE

## Scope decision
The final state is within scope for `.sisyphus/plans/rc-return-assignment-memory-leak.md`.
The reassignment leak fix remains centered on assignment ownership gating, the alias-return limitation is documented and explicitly not claimed fixed, there is no Game of Life source workaround, and there are no semantic changes in `runtime/opal_rc.c`, `runtime/opal_rc.h`, or `src/codegen/control_flow.rs`.

## Required checks

### 1. Intended reassignment leak behavior is the thing being fixed
The final assignment path still routes mutable reassignment through the target-type-aware store-mode selector:
- `src/codegen/statements.rs:632-649` — assignment lowers the RHS with `Some(&binding_type)` and selects `StoreMode` via `assignment_store_mode(value, &binding_type)`.
- `src/codegen/statements.rs:933-953` — `assignment_store_mode` preserves the existing `Expr::Array` and `reserve(...)` `TakeOwned` behavior and adds the RC-cleanup-gated ordinary call arm.

This matches the plan's intended narrow behavioral change: RC-typed reassignment from fresh call results consumes the owned result instead of retaining again.

Supporting evidence:
- `.sisyphus/evidence/task-4-reassignment-green.txt`
- `.sisyphus/evidence/task-5-stress-green.txt`

### 2. Alias-return limitation is documented, not claimed fixed
The known limitation remains explicit and truthful:
- `tests/integration_e2e/rc_store_leak_regressions.rs:391-424` defines ignored test `alias_return_assignment_known_limitation`.
- The failure text states that alias-returning user functions do not prove fresh RC ownership and that fixing this would require separate alias/provenance tracking.

This is consistent with the plan's “Known Limitations” and does not over-claim a fix.

Supporting evidence:
- `.sisyphus/evidence/task-1-alias-known-limitation.txt`

### 3. No Game of Life source workaround
The project source keeps the direct reassignment shape:
- `test-projects/game-of-life-full/src/main.op:23-27` uses `board = next_generation(board, config.width, config.height)` directly.

Required command check:
- `git diff -- test-projects/game-of-life-full/src/main.op` → empty in the current final state.

Assessment:
- No banned workaround such as `let next_board = ...; board = next_board` was introduced.

### 4. No runtime / return-lowering semantic change
Required command check:
- `git diff -- runtime/opal_rc.c runtime/opal_rc.h src/codegen/control_flow.rs` → empty in the current final state.

Assessment:
- No runtime RC semantics changed.
- No return-lowering behavior changed.

### 5. No provenance / metadata redesign
The scoped compiler change remains simple ownership gating rather than any redesign:
- No new return-ownership metadata, callee annotations, provenance tracking, escape analysis, or borrow-checking machinery appears in the scoped diff.
- `git diff -- src/codegen/statements.rs src/codegen/expressions_array.rs src/codegen/functions_call/array/helpers.rs src/codegen/functions_call/array/intrinsics.rs tests/integration_e2e/rc_store_leak_regressions.rs tests/integration_e2e/game_of_life_full_memory_stress.rs` → empty in the current final state because the plan work is fully committed.

### 6. Evaluation of the Task 6 follow-up commit
Recent commit chain:
- `9e22c43 docs(sisyphus): record task 6 verification evidence`
- `3de44f8 fix(codegen): tighten rc hooks and restore call ownership gating`
- `e333296 test(rc): verify reassignment stress workflow`
- `1b3f8f7 fix(codegen): take ownership of rc call assignment results`
- `59ac066 test(rc): document reassignment ownership audit`
- `0747cbe test(rc): add game of life reassignment stress`
- `a4640b1 test(rc): add reassignment return leak regression`

The plan text for Task 6 explicitly allows a focused follow-up commit if verification fails.
`3de44f8` is acceptable under that rule because it is still narrowly tied to the same ownership/RC-correctness surface:
- `src/codegen/statements.rs:943-953` restores the final call-ownership gating to the RC-cleanup predicate.
- `src/codegen/expressions_array.rs:1305-1309` narrows RC hook eligibility to the element kinds that actually lower to RC payload pointers.
- `src/codegen/functions_call/array/helpers.rs:122-128` and `src/codegen/functions_call/array/intrinsics.rs:1140-1171` / `1228-1231` consume that same hook-eligibility predicate at the existing hook sites.

Why this remains in scope:
- It is a verification-driven correction inside the same compiler ownership/RC-lowering area.
- It does not change runtime files, return lowering, or Game of Life source.
- It does not introduce a new subsystem or redesign.
- It resolves final verification fallout without broadening the feature target beyond reassignment leak correctness.

### 7. Working tree sanity
Required command check:
- `git diff --stat` shows only unrelated `.sisyphus/boulder.json` working-tree drift.

Assessment:
- The only live diff is unrelated session bookkeeping, not plan implementation drift.
- No scoped production/test files for this plan are currently dirty.

## Final rationale
This plan asked for a narrow reassignment leak fix, truthful limitation documentation, no source workaround, and no runtime/return-lowering/provenance redesign. The final repository state satisfies those constraints. The one extra compiler follow-up commit is justified by the plan's Task 6 allowance for a focused verification fix and remains tightly bound to RC hook eligibility plus the final intended ownership gating.
