# Learnings

## 2026-05-22 Task 1 memory signal audit
- RC-array leak primary metric should be `opal_runtime_live_heap_bytes()` because `runtime/opal_rc.h` and `runtime/opal_rc.c` limit that accounting to allocations made through `opal_rc_alloc`/`opal_rc_alloc_tracked`, which includes `opal_array_alloc`.
- `opal_runtime_live_heap_bytes()` does not cover malloc-backed interpolation temporaries: `src/codegen/expressions_string.rs` allocates interpolation buffers with plain `malloc`, and `runtime/opal_string.c` uses plain `malloc` for `*_to_string` helpers.
- Existing maintainable alternate path for call-temp leaks is the sanitizer workflow in `scripts/array_memory_sanitizer.sh` (ASAN+LSAN, Valgrind fallback), rather than adding new production instrumentation in Task 1.
- Existing `memory_model_counters` baseline is green and the direct harness output reports `counter_status=balanced`.

## 2026-05-22 Task 5 call-temp measurement
- Existing `scope_leak_counters` coverage is a useful green baseline for binding-based owned-string cleanup, but it does not prove safety for direct call arguments that never become scope-tracked bindings.
- The maintainable primary RED/GREEN signal for leak class #2 is the existing sanitizer lane in `scripts/array_memory_sanitizer.sh` because interpolation buffers in `src/codegen/expressions_string.rs` and `*_to_string` helper results in `runtime/opal_string.c` are plain `malloc` allocations outside `opal_runtime_live_heap_bytes()`.
- Task 6 should drive direct interpolation / owned call-temp scenarios through sanitizer-backed integration tests: pre-fix should produce leak markers, post-fix should run clean without relying on RSS-only assertions.

## 2026-05-22 Task 2 RC store RED regressions
- Added  with deterministic RC-array store ownership regression cases for direct assignment, push no-grow, push grow, self-overwrite, aliased source safety, and a second-class-reference-adjacent assignment path.
- Added  harness using constructor/atexit hooks to reset and report  array counters plus /peak accounting without changing production runtime logic.
- RED evidence confirms leak-class behavior pre-fix: selector  fails with imbalanced array counters/live heap bytes for direct assignment, push no-grow, push grow, and second-class-ref-adjacent, while aliased source safety and self-overwrite remain green in current baseline.

## 2026-05-22 Task 2 RC store RED regressions
- Added tests/integration_e2e/rc_store_leak_regressions.rs with deterministic RC-array store ownership regression cases for direct assignment, push no-grow, push grow, self-overwrite, aliased source safety, and a second-class-reference-adjacent assignment path.
- Added tests/integration_e2e/fixtures/rc_store_leak_regressions.c harness using constructor/atexit hooks to reset and report opal_rc_debug_* array counters plus opal_runtime_live_heap_bytes/peak accounting without changing production runtime logic.
- RED evidence confirms leak-class behavior pre-fix: selector rc_store_ fails with imbalanced array counters/live heap bytes for direct assignment, push no-grow, push grow, and second-class-ref-adjacent, while aliased source safety and self-overwrite remain green in current baseline.

## 2026-05-22 Task 3 store mode abstraction
- Making RC store ownership explicit with `StoreMode::{Retain, TakeOwned}` is safest when existing helpers keep `Retain` as the wrapper default and `TakeOwned` stays definition-only until a later task proves freshness at specific lowering sites.

## 2026-05-22 Task 4 RC store lowering
- `TakeOwned` stayed safe only at overwrite sites where the fresh RC value is produced and consumed in the same lowering flow without also being returned elsewhere: direct array-literal assignment and `push` branches that allocate a replacement backing array before rebinding the receiver.
- `reserve` and `clear` must stay functional instead of silently rebinding mutable receivers, because their returned arrays can alias the same fresh RC owner and create use-after-free bugs if the receiver store is treated like a transfer.
- The sanitizer lane is a necessary guardrail for Task 4: it caught under-retain bugs that the targeted `rc_store_` leak regressions alone would not detect.

- Task 4 fix validated that `TakeOwned` stays safe only for fresh assignment array literals and push replacement arrays, while `reserve` noop must still return an independent array value to avoid aliasing one RC owner across both the receiver and the returned result.

## 2026-05-22 Task 6 call-temp RED regressions
- Added `tests/integration_e2e/call_temp_leak_regressions.rs` with five deterministic, timeout-bounded sanitizer harness tests focused on direct call-argument temporaries and propagate-driven early exits.
- For leak-class #2, reliable RED assertions come from sanitizer markers (`LeakSanitizer`, `ERROR: AddressSanitizer`, `detected memory leaks`) rather than `opal_runtime_live_heap_bytes()` because interpolation/call-temporary allocations are malloc-backed.
- Required RED selectors now produce sanitizer-backed failures in pre-fix state and evidence is captured in `.sisyphus/evidence/task-6-direct-interpolation-red.txt` and `.sisyphus/evidence/task-6-propagate-red.txt`.

- Task 7: direct ephemeral owned call arguments now need their own temporary lexical scope in `src/codegen/functions_call.rs` so existing `cleanup_scopes_to_depth_with_malloc_string_release` can free them on normal call completion and on partial-lowering failure paths.
- Task 7: `expr_requires_malloc_string_cleanup` also needs to recognize `Expr::StringInterpolation` directly, otherwise let-bound interpolated strings bypass existing scope cleanup even though they are malloc-backed.
- Task 7 verification: sanitizer markers are the reliable signal for call-temp leak regressions; propagate-entry fixtures may exit non-zero by design once the entry wrapper surfaces the forwarded error.

## 2026-05-22 Task 8 executable stress harness
- Added `tests/integration_e2e/game_of_life_full_memory_stress.rs` as an ignored integration stress test gated by `OPAL_RUN_STRESS=1`, so default `cargo test` remains clean while explicit stress runs are deterministic.
- The harness compiles and runs the real `test-projects/game-of-life-full` executable, samples Linux `/proc/<pid>/status` `VmRSS` every 250ms for a nominal 15s window, and enforces a hard stop at 20s.
- Stability signal is bounded post-warmup behavior rather than peak-only RSS: require `MIN_SAMPLES`, discard `WARMUP_SAMPLES`, and assert both max post-warmup growth and spread limits.
- Cleanup path always attempts to terminate/reap child processes on success/failure/timeout paths, and failure diagnostics include the full sample series plus warmup/threshold/kill/final-status details.

## 2026-05-22 Task 9 deterministic verification wiring
- `cargo test --features integration --test integration_e2e ... -- --exact` requires fully-qualified selectors from `cargo test -- --list` (for example `tests::call_temp_leak_regressions::call_temp_owned_arg_freed_on_return`); short names can silently filter every intended verification test out.
- Keeping the ignored Game of Life memory stress hook behind `OPAL_RUN_STRESS=1` preserves deterministic default verification while still letting the sanitizer script expose an explicit opt-in stress lane.


## 2026-05-23T01:26:37Z Task F3 real manual QA
- Hands-on rerun passed for required gates: `cargo test --workspace`, `bash scripts/array_memory_sanitizer.sh`, and opt-in stress selector `tests::game_of_life_full_memory_stress::game_of_life_full_memory_stress` with `OPAL_RUN_STRESS=1`.
- Deterministic memory hooks are currently wired with fully-qualified exact selectors in `scripts/array_memory_sanitizer.sh` (`tests::<module>::<test>`), and command output showed each required hook executing as a single exact test (`running 1 test` + `... ok`).
- Stress lane remained opt-in in both script logic and runtime behavior (script prints skip message unless `OPAL_RUN_STRESS=1`); explicit stress run completed in ~15.48s, consistent with 15s window and below 20s hard timeout envelope.


## 2026-05-23T01:27:21Z Task F2 code quality gate
- Store ownership mode remains conservative-by-default: `store_binding_overwrite_rc_safe` and `store_array_binding` default to `StoreMode::Retain`, with explicit `TakeOwned` only at proven-fresh sites (array literal assignment and `push` fresh replacement arrays).
- Call-temp cleanup uses dedicated per-call scope + cleanup records in `src/codegen/functions_call.rs`, and transferred cleanup exclusions remain explicit/closed (`call_argument_takes_owned_value` currently has no transfer whitelist entries).
- Deterministic verification hooks are exact-selector based in `scripts/array_memory_sanitizer.sh` and include mandatory RC-store + five call-temp regressions; ignored stress hook remains opt-in via `OPAL_RUN_STRESS=1`.
- Targeted regression selectors all passed in this gate (`rc_store_*` and five `call_temp_*` tests), and scoped diagnostics/anti-pattern scans found no blocking quality defects.

## 2026-05-23T02:00:00Z Task F4 scope fidelity
- Final F4 review rejected the branch on scope fidelity: the planned RC-store fix, call-temp cleanup, deterministic regressions, and opt-in Game of Life stress wiring are present, but `src/codegen/functions_call/array/intrinsics.rs` also changes `reserve` noop behavior to allocate/copy a fresh array.
- That `reserve.noop` allocation path is a material behavior change outside the two authorized Game of Life leak classes, so it counts as scope creep even if ownership-safety motivated it.
- Task 7 green proof exists in `.sisyphus/evidence/task-7/verification.md`, but the artifact name does not match the exact plan-requested filename `task-7-call-temp-green.txt`.


## 2026-05-23T01:33:33Z Task F1 plan compliance audit
- F1 audit found plan-scope implementation and verification largely aligned: RC-store RED→GREEN evidence exists, call-temp RED→GREEN evidence exists, sanitizer hooks run exact fully-qualified selectors, and stress remains `#[ignore]` + `OPAL_RUN_STRESS=1` gated.
- Blocking compliance gap: Task 8 evidence does not include the plan-required note explaining absent pre-fix stress RED evidence / why targeted RED tests were the actual RED drivers.
- Blocking compliance gap: current git evidence shows implementation changes still uncommitted in the working tree during review, so the plan's atomic-commit requirement is not concretely satisfied by repository history evidence.
- 2026-05-23T01:43:47Z: F1/F4 remediation restored `reserve` noop scope fidelity in `src/codegen/functions_call/array/intrinsics.rs` by removing the fresh allocate/copy path and returning an owned alias instead; `src/codegen/statements.rs` consumes `reserve(...)` assignment results with `StoreMode::TakeOwned` so noop results stay alive without reintroducing out-of-plan behavior drift.
- 2026-05-23T01:49:05Z Task F1 rerun: the prior Task 8 blocker is now concretely closed by `.sisyphus/evidence/task-8-stress-prefx-red-feasibility.md`, and the exact plan-requested Task 7 GREEN artifact now exists at `.sisyphus/evidence/task-7-call-temp-green.txt`.
- 2026-05-23T01:49:05Z Task F1 rerun: current `reserve` noop lowering is back in-bounds — `src/codegen/functions_call/array/intrinsics.rs` now returns an owned alias via `emit_inc(array_value)` instead of allocate/copy, while `src/codegen/statements.rs` treats `reserve(...)` assignment as `StoreMode::TakeOwned` to preserve ownership safely.
- 2026-05-23T02:09:50Z Task F1 rerun: atomic-commit compliance becomes approvable only when the planned slices exist as actual git history, not just evidence notes; the decisive proof here was the six-commit chain `ca6c6e2 -> 61bc86a -> 6d04e16 -> ba2ca54 -> 96b792d -> 46d08dd` plus empty tracked diff output from `git status --short` / `git diff --stat`.
- 2026-05-23T02:09:50Z Task F1 rerun: untracked `.sisyphus/*` review artifacts do not block compliance once tracked implementation drift is gone; for this gate the relevant delivery-state check is clean tracked tree state, not absence of untracked evidence files.
- 2026-05-23T01:47:27Z Task F4 rerun: the prior `reserve.noop` scope-creep blocker is resolved in the live code (`emit_inc(array_value)` owned-alias return in both noop branches), and the plan evidence gaps for Task 7 green naming and Task 8 stress-feasibility note are now present. The remaining scope-fidelity blocker is the unrelated `.sisyphus/boulder.json` diff, so exact-scope approval still fails until that file is excluded from delivery.
- 2026-05-23T02:09:15Z Task F4 rerun: live git inspection now shows no tracked diff at all, so the previous delivery-state drift blocker is closed. Recent history also contains the expected atomic leak-remediation slices (`test(memory)` RC regressions, `fix(codegen)` RC lowering, `test(memory)` call-temp regressions, `fix(codegen)` call-temp cleanup, `test(gol)` stress, `ci(memory)` sanitizer wiring), which brings the final scope gate back to APPROVE.
