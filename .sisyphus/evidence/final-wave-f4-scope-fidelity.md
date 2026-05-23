# Final Wave F4 — Scope Fidelity Check

Verdict: APPROVE

## Fresh scope verdict
The previous F4 rejection reasons are now closed in the current repository state. `reserve.noop` no longer performs the out-of-plan allocate/copy rewrite, and the delivery-state drift blocker is also resolved because the current tracked diff is empty while the leak-remediation work is present as focused commits in recent history. On a strict re-check against the plan, the delivered implementation is now within scope.

## Scope requirements mapped to committed files and current tracked state

### Current tracked delivery state
Observed with live git inspection:
- `GIT_MASTER=1 git status --short` shows no tracked modifications.
- `GIT_MASTER=1 git diff --stat` shows no tracked diff.
- `GIT_MASTER=1 git diff --name-only` shows no tracked diff.

Untracked evidence/notepad files are present under `.sisyphus/`, but they are review artifacts rather than implementation drift. The previous blocker about unrelated tracked delivery-state changes is therefore no longer applicable.

Recent committed chain (`GIT_MASTER=1 git log --oneline -15`) includes the expected focused implementation slices:
- `ca6c6e2 test(memory): add rc store leak regressions`
- `61bc86a fix(codegen): take owned rc values on proven fresh stores`
- `6d04e16 test(memory): add call argument temporary regressions`
- `ba2ca54 fix(codegen): clean owned call argument temporaries on all exits`
- `96b792d test(gol): add bounded memory stress for full executable`
- `46d08dd ci(memory): wire leak regressions into sanitizer verification`

Assessment:
- Atomic implementation slices are now visible in history.
- The prior tracked-drift blocker is resolved.
- PASS.

## Plan deliverables mapped to committed files and evidence

### 1. RC store leak class (in scope)
Plan requirement:
- Fix only the confirmed RC store leak class using explicit `StoreMode::{Retain, TakeOwned}` with conservative default and whitelist-only `TakeOwned` usage.

Committed files / modules involved:
- `src/codegen/binding_store.rs`
- `src/codegen/functions_call/array/helpers.rs`
- `src/codegen/functions_call/array/intrinsics.rs`
- `src/codegen/statements.rs`
- `tests/integration_e2e/rc_store_leak_regressions.rs`
- `tests/integration_e2e/fixtures/rc_store_leak_regressions.c`
- `tests/integration_e2e/tests.rs`
- `scripts/array_memory_sanitizer.sh`

Evidence:
- RED: `.sisyphus/evidence/task-2-rc-store-red.txt`
- GREEN: `.sisyphus/evidence/task-4-rc-store-green.txt`
- Sanitizer: `.sisyphus/evidence/task-4-sanitizer.txt`
- Final verification: `.sisyphus/evidence/task-9-final-verification.txt`

Assessment:
- `StoreMode` remains explicit and conservative by default.
- Live code still confines `TakeOwned` to proven-fresh sites.
- Required RC-store regressions remain registered in `tests/integration_e2e/tests.rs` and wired into `scripts/array_memory_sanitizer.sh` as exact selectors.
- PASS.

### 2. Call-temp / interpolation leak class (in scope)
Plan requirement:
- Add generic ephemeral owned call-argument cleanup for malloc-backed direct call temporaries, including propagate/early-return behavior.

Committed files / modules involved:
- `src/codegen/functions_call.rs`
- `src/codegen/scope_tracker.rs`
- `tests/integration_e2e/call_temp_leak_regressions.rs`
- `tests/integration_e2e/tests.rs`
- `scripts/array_memory_sanitizer.sh`

Evidence:
- RED: `.sisyphus/evidence/task-6-direct-interpolation-red.txt`
- RED: `.sisyphus/evidence/task-6-propagate-red.txt`
- GREEN: `.sisyphus/evidence/task-7-call-temp-green.txt`
- Final verification: `.sisyphus/evidence/task-9-final-verification.txt`

Assessment:
- The required call-temp regression family is present and exact-selector wired.
- The prior artifact naming drift is resolved because `.sisyphus/evidence/task-7-call-temp-green.txt` exists and contains the five required passing selectors.
- PASS.

### 3. Stress test and deterministic sanitizer wiring (in scope)
Plan requirement:
- Add opt-in, ignored Game of Life Full stress test with hard timeout and explicit diagnostics.
- Keep deterministic RC-store and call-temp checks in sanitizer verification.
- Document the Task 8 RED-feasibility limitation when executable-stress RED is unavailable on the remediated tree.

Committed files / modules involved:
- `tests/integration_e2e/game_of_life_full_memory_stress.rs`
- `tests/integration_e2e/tests.rs`
- `scripts/array_memory_sanitizer.sh`

Evidence:
- Stress green: `.sisyphus/evidence/task-8-stress-green.txt`
- Timeout proof: `.sisyphus/evidence/task-8-stress-timeout.txt`
- RED-feasibility note: `.sisyphus/evidence/task-8-stress-prefx-red-feasibility.md`
- Final verification: `.sisyphus/evidence/task-9-final-verification.txt`
- Atomicity note: `.sisyphus/evidence/task-9-atomicity.md`

Assessment:
- `tests/integration_e2e/game_of_life_full_memory_stress.rs` remains `#[ignore]` and opt-in via `OPAL_RUN_STRESS=1`.
- `scripts/array_memory_sanitizer.sh` still runs deterministic exact selectors for RC-store and call-temp regressions, and only invokes stress behind `OPAL_RUN_STRESS=1`.
- The required feasibility note exists and correctly points back to the targeted RED drivers.
- PASS.

## Re-evaluation of the prior `reserve.noop` scope-creep claim

### Previous rejection claim
The prior F4 report rejected because `reserve.noop` in `src/codegen/functions_call/array/intrinsics.rs` allocated a fresh array, copied elements, and returned that new array even when `requested_capacity <= current_capacity`.

### Current implementation
That specific behavior is no longer present.

Observed current behavior in `src/codegen/functions_call/array/intrinsics.rs`:
- Mutable noop path (`reserve.noop`): increments the existing array RC via `emitter.emit_inc(array_value)?` and returns the same pointer as an owned result.
- Functional noop path (`reserve.functional.noop`): likewise increments RC and returns the same pointer as an owned result.
- There is no noop-path `allocate_array_with_capacity(... "reserve.noop" ...)` and no noop-path element copy.

Supporting code points from the current file:
- `src/codegen/functions_call/array/intrinsics.rs:332-339`
- `src/codegen/functions_call/array/intrinsics.rs:428-435`

Supporting assignment-side ownership handling:
- `src/codegen/statements.rs:633-649` routes assignment through `assignment_store_mode(...)`.
- `src/codegen/statements.rs:933-950` marks `reserve(...)` assignment results as `StoreMode::TakeOwned`, so the owned alias is consumed without adding a second retain.

Scope assessment of this remediation:
- This removes the earlier out-of-plan allocate/copy semantic rewrite.
- The current noop behavior is now narrowly tied to ownership correctness inside the already-authorized RC-lowering fix.
- The old `reserve.noop` blocker is therefore resolved.
- PASS.

## Prohibited scope expansion audit

### Runtime refcount invariant change
Plan must-not:
- Preserve `opal_rc_alloc` / `opal_array_alloc` initial refcount of 1.

Assessment:
- No runtime refcount-invariant change is evidenced by the committed scope-critical files reviewed here.
- PASS.

### GC / collector introduction
Plan must-not:
- No tracing GC, no cycle collector, no global sweep.

Assessment:
- No such machinery appears in the reviewed implementation set or recent commit chain.
- PASS.

### Game of Life source workaround hack
Plan must-not:
- Do not solve the leak by rewriting Game of Life sources.

Assessment:
- No `test-projects/game-of-life-full/src/*` workaround rewrite is part of the reviewed delivery.
- PASS.

### Stress remains opt-in and timeout-bounded
Assessment:
- `tests/integration_e2e/game_of_life_full_memory_stress.rs` remains ignored and opt-in.
- `scripts/array_memory_sanitizer.sh` invokes it only behind `OPAL_RUN_STRESS=1`.
- PASS.

## Final decision
APPROVE

Reason:
- The previous `reserve.noop` scope-creep finding is directly resolved in the live code.
- The previous delivery-state blocker is also resolved: there is no tracked diff now, and the leak-fix work is represented by focused atomic commits in recent history.
- The remaining committed implementation aligns with the plan scope: the two authorized leak classes plus deterministic verification wiring and opt-in Game of Life stress coverage.