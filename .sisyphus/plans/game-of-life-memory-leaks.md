# Game of Life Full Memory Leak Fixes

## TL;DR
> **Summary**: Add RED-first deterministic leak regressions, fix the two confirmed compiler-lowering leak classes, then add a timeout-protected Game of Life Full executable stress test. Preserve the language's Perceus-style RC direction by improving ownership transfer semantics in codegen instead of changing runtime allocation invariants.
> **Deliverables**:
> - Explicit RC store-mode abstraction for conservative retain vs proven-fresh take-owned stores.
> - Generic ephemeral owned call-argument cleanup in call lowering, including propagate early-return paths.
> - Deterministic counter regressions for array store leaks and string temporary leaks.
> - Ignored/gated ~15s Game of Life Full executable memory stress test with explicit timeout and sample diagnostics.
> - CI memory verification wiring for the new deterministic tests and optional stress invocation.
> **Effort**: Large
> **Parallel**: YES - 4 waves
> **Critical Path**: Task 1 → Task 2 → Task 4 → Task 7 → Task 8

## Context
### Original Request
Fix the issues in `GAME_OF_LIFE_MEMORY_LEAK_FINDINGS.md`. Do not just apply the fixes suggested in the document; research the best, most maintainable way to fix these issues given the current intent of the language and the direction the language is heading. Ensure the issues do not reappear via regression testing, and use testing to validate that the issue has been fully addressed via stress testing. You may need to create a stress testing harness to be able to read the memory values going higher than intended - please plan for the most maintainable way to do that in the repository. The tests should check memory usage over time for the Game of Life Full executable and should ensure the memory never exceeds a certain amount after, say, 15 seconds. The stress tests should initially be created before the leaks are fixed, that way they fail initially (confirming they work) and will pass once the memory leaks are fixed.

Additional user constraints:
- Stress tests need a timeout so they cannot run indefinitely.
- Use atomic commits.

### Interview Summary
- Work classified as Architecture: changes affect compiler lowering, ownership semantics, runtime-accounting tests, and long-running executable verification.
- Scope includes the two confirmed unbounded leak classes only: RC-array over-retention and malloc-backed string interpolation temporaries.
- Scope excludes changing runtime refcount initialization, adding tracing GC, or treating `FrameClock` atexit cleanup as part of this bug.
- Default decisions applied:
  - Primary stress test must execute the compiled `test-projects/game-of-life-full` executable because the request explicitly names the executable.
  - Deterministic in-process counter tests are still required as RED drivers because executable RSS/process sampling is too noisy to be the only signal.
  - Game of Life Full stress test should be gated/ignored and invoked explicitly from memory verification rather than run in default `cargo test`.
  - Threshold should be based on post-warm-up bounded-growth slope/sample spread, documented in the test, not calibrated to barely pass.

### Metis Review (gaps addressed)
- Added guardrail to use an explicit `StoreMode` enum (`Retain` vs `TakeOwned`) instead of threading ambiguous booleans.
- Added guardrail that `TakeOwned` is whitelist-only and must only be used for values proven fresh/linear at the lowering site.
- Added guardrail that ephemeral call-argument cleanup must act like scope-exit cleanup and run on normal return, propagate early return, and failure exits where lowered temporaries already exist.
- Added guardrail to verify whether existing runtime counters cover malloc-backed interpolation buffers before writing the call-temp RED test; if not, use a targeted malloc/RSS/sanitizer-backed probe rather than pretending RC counters cover it.
- Adjusted commit strategy for bisectability: per-leak vertical slices, with RED evidence captured before fixes, but no long-lived broken commits.

## Work Objectives
### Core Objective
Fix Game of Life Full memory leaks by correcting compiler ownership/lifetime lowering and proving the fixes with deterministic regression tests plus a timeout-protected executable stress test.

### Deliverables
- New RC store regression tests covering direct assignment, push no-grow, push grow, self-overwrite, aliased source safety, and Second-Class Ref-adjacent overwrite/grow behavior.
- New call-temp regression tests covering direct interpolation arguments, mixed borrowed/owned args, nested/direct calls, propagate early return, and no double-free for transferred ownership.
- `StoreMode` abstraction threaded through RC binding/array store paths with conservative retain as default.
- Generic ephemeral owned call-argument cleanup scope in `src/codegen/functions_call.rs`.
- `game_of_life_full_memory_stress` integration test that compiles/runs the actual Game of Life Full executable, samples memory over time, and enforces explicit timeout + bounded-growth criteria.
- CI memory verification update in `scripts/array_memory_sanitizer.sh` for deterministic memory tests, and documented explicit invocation for ignored stress.

### Definition of Done (verifiable conditions with commands)
- `cargo test --features integration --test integration_e2e memory_model_counters -- --nocapture --test-threads=1` exits 0.
- `OPAL_RUN_STRESS=1 cargo test --features integration --test integration_e2e tests::game_of_life_full_memory_stress::game_of_life_full_memory_stress -- --ignored --exact --nocapture --test-threads=1` exits 0 and completes within the test timeout.
- `bash scripts/array_memory_sanitizer.sh` exits 0.
- `cargo test --workspace` exits 0, excluding ignored stress tests.
- RED evidence exists in `.sisyphus/evidence/` showing the new targeted tests fail before each fix and pass after each fix.
- Every implementation commit is atomic and has a focused message; no commit leaves default tests failing.

### Must Have
- Preserve `opal_rc_alloc` / `opal_array_alloc` initial refcount of 1.
- Default RC overwrite semantics remain conservative retain.
- `TakeOwned`/move semantics are opt-in at proven-fresh producer sites only.
- Call-temp cleanup must avoid double-free for bindings and transferred ownership.
- Stress test must have both fixed sampling/iteration bounds and wall-clock timeout.
- Stress test must print sample series and threshold details on failure.

### Must NOT Have (guardrails, AI slop patterns, scope boundaries)
- Must not blindly apply `GAME_OF_LIFE_MEMORY_LEAK_FINDINGS.md` suggestions without aligning them with current ownership design.
- Must not introduce tracing GC, a global sweep, or a cycle collector.
- Must not solve leaks by rewriting Game of Life source code to avoid compiler bugs.
- Must not use raw RSS as the only regression signal.
- Must not add the 15s ignored stress test to default `cargo test`.
- Must not use vague booleans like `is_fresh` or `retain_new_value` across broad codegen paths when an explicit enum can encode intent.
- Must not leave a long-lived broken tree with failing default tests.

## Verification Strategy
> ZERO HUMAN INTERVENTION - all verification is agent-executed.
- Test decision: TDD / RED-GREEN-REFACTOR using Rust integration tests, C counter fixtures, existing sanitizer script, and ignored executable stress.
- QA policy: Every task has agent-executed scenarios.
- Evidence: `.sisyphus/evidence/task-{N}-{slug}.{ext}`
- RED evidence policy:
  - Before each fix, run its new targeted test(s) against the pre-fix tree and save failing output.
  - After the fix, run the same test(s) and save passing output.
  - If bisectability conflicts with committing RED tests alone, keep RED evidence in `.sisyphus/evidence/` and commit tests enabled only together with the fix.
- Memory signal hierarchy:
  1. Deterministic runtime counters (`opal_runtime_live_heap_bytes`, debug category counters) for RC-owned allocations.
  2. Existing sanitizer/Valgrind script for use-after-free/double-free/leak markers.
  3. Game of Life executable process memory slope as end-to-end guard after warm-up.

## Execution Strategy
### Parallel Execution Waves
> Target: 5-8 tasks per wave. <3 per wave (except final) = under-splitting.
> Extract shared dependencies as Wave-1 tasks for max parallelism.

Wave 1: Task 1 (test signal audit), Task 2 (RC RED regressions), Task 5 (call-temp counter coverage/probe design)
Wave 2: Task 3 (StoreMode abstraction), Task 6 (call-temp RED regressions)
Wave 3: Task 4 (RC lowering fix), Task 7 (generic call-temp cleanup)
Wave 4: Task 8 (Game of Life Full stress), Task 9 (CI wiring/docs/atomic commit verification)
Final: F1-F4 review agents

### Dependency Matrix (full, all tasks)
| Task | Depends On | Blocks |
|---|---|---|
| 1. Audit memory signals | None | 2, 5, 8 |
| 2. Add RC RED regressions | 1 | 3, 4 |
| 3. Add explicit StoreMode | 2 | 4 |
| 4. Fix RC store lowering | 3 | 8, 9 |
| 5. Verify call-temp measurement | 1 | 6 |
| 6. Add call-temp RED regressions | 5 | 7 |
| 7. Fix call-temp cleanup | 6 | 8, 9 |
| 8. Add Game of Life Full executable stress | 4, 7 | 9 |
| 9. Wire CI and verify atomic commits | 4, 7, 8 | Final |

### Agent Dispatch Summary (wave → task count → categories)
| Wave | Task Count | Categories |
|---|---:|---|
| 1 | 3 | deep, unspecified-high |
| 2 | 2 | deep, unspecified-high |
| 3 | 2 | deep |
| 4 | 2 | unspecified-high, deep |
| Final | 4 | oracle, unspecified-high, deep |

## TODOs
> Implementation + Test = ONE task. Never separate.
> EVERY task MUST have: Agent Profile + Parallelization + QA Scenarios.

- [x] 1. Audit memory measurement signals and establish RED/GREEN evidence workflow

  **What to do**: Verify which existing counters cover RC allocations and whether malloc-backed string interpolation buffers are visible to any existing counter. Inspect `runtime/opal_rc.h`, `runtime/opal_rc.c`, `runtime/opal_string.c`, and `tests/integration_e2e/fixtures/memory_model_counters.c`. Record the chosen measurement for each leak class in comments in the new tests: RC tests use `opal_runtime_live_heap_bytes()` or category counters; call-temp tests use an existing malloc-visible counter if present, otherwise a sanitizer/RSS/mallinfo-backed probe scoped only to the test harness. Create `.sisyphus/evidence/task-1-memory-signal-audit.md` with the finding and commands used.
  **Must NOT do**: Do not add broad runtime accounting unless no existing signal can detect leak class #2; do not change allocator semantics; do not use RSS as the only planned signal for RC tests.

  **Recommended Agent Profile**:
  - Category: `deep` - Requires reasoning about runtime accounting semantics and test reliability.
  - Skills: [] - No special skill needed.
  - Omitted: [`frontend-ui-ux`] - No UI work.

  **Parallelization**: Can Parallel: YES | Wave 1 | Blocks: [2, 5, 8] | Blocked By: []

  **References** (executor has NO interview context - be exhaustive):
  - Pattern: `tests/integration_e2e/memory_model_counters.rs` - Rust integration pattern for compiling and running C memory harnesses.
  - Pattern: `tests/integration_e2e/fixtures/memory_model_counters.c` - Existing C fixture using runtime counter APIs and printing balanced output.
  - API/Type: `runtime/opal_rc.h` - Declares `opal_runtime_reset_heap_accounting()`, `opal_runtime_live_heap_bytes()`, `opal_runtime_peak_heap_bytes()`, and debug test helpers.
  - API/Type: `runtime/opal_rc.c` - Implements RC heap accounting and `opal_array_alloc` refcount behavior.
  - API/Type: `runtime/opal_string.c` - `int64_to_string` and related helpers allocate malloc-backed strings that may not be represented in RC heap counters.
  - Test: `scripts/array_memory_sanitizer.sh` - Existing mandatory memory verification hook and sanitizer marker checks.

  **Acceptance Criteria** (agent-executable only):
  - [ ] `cargo test --features integration --test integration_e2e memory_model_counters -- --nocapture --test-threads=1` exits 0 before new changes.
  - [ ] `.sisyphus/evidence/task-1-memory-signal-audit.md` states which metric will be used for RC-array tests and which metric will be used for call-temp tests.
  - [ ] The audit explicitly states whether malloc-backed interpolation buffers are covered by `opal_runtime_live_heap_bytes()` or require an alternate probe.

  **QA Scenarios** (MANDATORY - task incomplete without these):
  ```
  Scenario: Existing counter harness still passes
    Tool: Bash
    Steps: Run `cargo test --features integration --test integration_e2e memory_model_counters -- --nocapture --test-threads=1`.
    Expected: Command exits 0 and output contains `counter_status=balanced`.
    Evidence: .sisyphus/evidence/task-1-memory-model-counters.txt

  Scenario: Measurement gap documented
    Tool: Bash
    Steps: Inspect the evidence file with `test -s .sisyphus/evidence/task-1-memory-signal-audit.md` and search for `malloc-backed interpolation`.
    Expected: File exists, is non-empty, and names the selected leak-class-#2 metric.
    Evidence: .sisyphus/evidence/task-1-memory-signal-audit.md
  ```

  **Commit**: NO | Message: N/A | Files: [.sisyphus/evidence/task-1-memory-signal-audit.md]

- [x] 2. Add RC-array RED regressions for store/array ownership leaks

  **What to do**: Add deterministic RC leak regression coverage before implementing the RC fix. Prefer extending the existing C harness pattern with a new fixture such as `tests/integration_e2e/fixtures/rc_store_leak_regressions.c` and a Rust invoker/module such as `tests/integration_e2e/rc_store_leak_regressions.rs`, then register the module in `tests/integration_e2e/tests.rs`. Cover direct assignment of a fresh RC array, push without grow, push with grow, self-overwrite, aliased source safety, and one Second-Class Ref-adjacent overwrite/grow scenario. Run the new tests against the pre-fix tree and save failing output as RED evidence.
  **Must NOT do**: Do not implement `StoreMode` or alter production code in this task; do not commit default-failing tests unless repository workflow explicitly permits it. If default-failing tests cannot be committed, create evidence first and keep test enablement paired with Task 4.

  **Recommended Agent Profile**:
  - Category: `unspecified-high` - Hands-on test harness work with compiler/runtime integration.
  - Skills: [] - No special skill needed.
  - Omitted: [`playwright`] - No browser testing.

  **Parallelization**: Can Parallel: YES | Wave 1 | Blocks: [3, 4] | Blocked By: [1]

  **References** (executor has NO interview context - be exhaustive):
  - Pattern: `tests/integration_e2e/memory_model_counters.rs` - Copy compile/run helper shape for C fixtures and timeout handling.
  - Pattern: `tests/integration_e2e/fixtures/memory_model_counters.c` - Fixture shape for exercising runtime counters and printing diagnostic lines.
  - Pattern: `tests/integration_e2e/tests.rs` - Add `mod rc_store_leak_regressions;` when the Rust module exists.
  - API/Type: `runtime/opal_rc.h` - Runtime counter APIs and array allocation functions.
  - Bug path: `src/codegen/binding_store.rs` `store_binding_overwrite_rc_safe` - Current unconditional retain is expected to make RED tests fail.
  - Bug path: `src/codegen/functions_call/array/intrinsics.rs` - Array push/grow paths must be represented by tests.

  **Acceptance Criteria** (agent-executable only):
  - [ ] Pre-fix RED run of `cargo test --features integration --test integration_e2e rc_store_ -- --nocapture --test-threads=1` exits non-zero or the specific new test selectors fail as expected; output saved to `.sisyphus/evidence/task-2-rc-store-red.txt`.
  - [ ] New tests include selectors or subcases named for direct assignment, push no-grow, push grow, self-overwrite, aliased source, and Second-Class Ref adjacency.
  - [ ] Tests have deterministic timeout no greater than `COUNTER_HARNESS_TIMEOUT` pattern used by `memory_model_counters.rs`.

  **QA Scenarios** (MANDATORY - task incomplete without these):
  ```
  Scenario: RC store RED proof
    Tool: Bash
    Steps: Run `cargo test --features integration --test integration_e2e rc_store_ -- --nocapture --test-threads=1` before production fixes.
    Expected: Command exits non-zero because at least one new RC leak assertion detects non-zero live bytes or imbalanced counters.
    Evidence: .sisyphus/evidence/task-2-rc-store-red.txt

  Scenario: Aliasing safety fixture compiles
    Tool: Bash
    Steps: Run the specific aliased-source selector after adding test harness code, even if it fails on leak assertion.
    Expected: Test compiles and either fails only on the intended leak assertion or passes if pre-existing behavior is already safe; no compile errors.
    Evidence: .sisyphus/evidence/task-2-rc-alias-selector.txt
  ```

  **Commit**: CONDITIONAL | Message: `test(memory): add rc store leak regressions` | Files: [tests/integration_e2e/tests.rs, tests/integration_e2e/rc_store_leak_regressions.rs, tests/integration_e2e/fixtures/rc_store_leak_regressions.c]

- [x] 3. Introduce explicit RC store mode abstraction with conservative default

  **What to do**: Add a small explicit enum in the codegen store layer, e.g. `StoreMode::Retain` and `StoreMode::TakeOwned`, in or near `src/codegen/binding_store.rs`. Update `store_binding_overwrite_rc_safe` and `store_array_binding` plumbing so existing call sites default to `StoreMode::Retain`. Do not yet switch bug-path call sites to `TakeOwned` unless required for compilation in the same edit; the purpose is to create a reviewable abstraction before behavior changes. Include comments defining `TakeOwned` as valid only for values proven fresh/linear by the lowering site. If Task 2 tests cannot be committed while failing, perform this refactor together with Task 4 in one atomic fix commit after preserving RED evidence.
  **Must NOT do**: Do not replace the enum with a bool; do not make `TakeOwned` the default; do not change `initialize_binding_value` semantics unless needed to share naming/helpers safely.

  **Recommended Agent Profile**:
  - Category: `deep` - Compiler ownership semantics require careful safety reasoning.
  - Skills: [] - No special skill needed.
  - Omitted: [`frontend-ui-ux`] - No UI work.

  **Parallelization**: Can Parallel: NO | Wave 2 | Blocks: [4] | Blocked By: [2]

  **References** (executor has NO interview context - be exhaustive):
  - API/Type: `src/codegen/binding_store.rs` - Defines `store_binding_overwrite_rc_safe`, `initialize_binding_value`, `retain_new_binding_value_if_needed`, and `release_binding_value_if_needed`.
  - Pattern: `src/codegen/statements.rs` - Let initialization already computes retain behavior for identifiers; use as conceptual reference but prefer enum naming.
  - Pattern: `src/codegen/functions_call/array/helpers.rs` - `store_array_binding` delegates to binding store and must preserve conservative default.
  - Design: `language-spec/requirements/memory-model.md` - Perceus-style ownership/reuse direction; do not modify spec unless implementation reveals docs are stale.

  **Acceptance Criteria** (agent-executable only):
  - [ ] `cargo test --workspace` exits 0 after adding the enum and preserving existing behavior.
  - [ ] Search for `StoreMode::TakeOwned` shows only definition/tests or no production behavior change yet, except if required by compiler signature changes and explicitly documented.
  - [ ] Existing memory verification `cargo test --features integration --test integration_e2e memory_model_counters -- --nocapture --test-threads=1` still exits 0.

  **QA Scenarios** (MANDATORY - task incomplete without these):
  ```
  Scenario: Conservative default preserves behavior
    Tool: Bash
    Steps: Run `cargo test --workspace` after introducing `StoreMode`.
    Expected: Command exits 0; no existing behavior changes are introduced by the abstraction alone.
    Evidence: .sisyphus/evidence/task-3-workspace-tests.txt

  Scenario: TakeOwned use is explicit
    Tool: Bash
    Steps: Search changed diff for `TakeOwned` and inspect every use.
    Expected: Every `TakeOwned` use has a nearby reason or is deferred to Task 4; default store paths use `Retain`.
    Evidence: .sisyphus/evidence/task-3-storemode-review.txt
  ```

  **Commit**: YES | Message: `refactor(codegen): make rc store ownership explicit` | Files: [src/codegen/binding_store.rs, src/codegen/functions_call/array/helpers.rs, src/codegen/statements.rs if signature requires]

- [x] 4. Fix RC store lowering with whitelist-only TakeOwned sites

  **What to do**: Switch only proven-fresh RC store sites to `StoreMode::TakeOwned`. Required sites: assignment lowering in `src/codegen/statements.rs` when RHS lowering produces a fresh allocation in the same expression; array push/grow/store-back paths in `src/codegen/functions_call/array/intrinsics.rs` and `src/codegen/functions_call/array/helpers.rs` where the stored value is the freshly allocated grown/copied array. Keep identifier/lvalue/alias/self-assignment stores as `StoreMode::Retain`. Run Task 2 tests before and after; save RED and GREEN evidence. If a lowering site cannot prove freshness structurally, leave it `Retain` and document the skipped optimization.
  **Must NOT do**: Do not infer freshness from runtime pointer values; do not use `TakeOwned` for arbitrary `Expr::Identifier`, array index loads, borrowed refs, or self-overwrite; do not change `opal_rc_alloc` refcount.

  **Recommended Agent Profile**:
  - Category: `deep` - High correctness risk: under-retain causes use-after-free.
  - Skills: [] - No special skill needed.
  - Omitted: [`playwright`] - No browser testing.

  **Parallelization**: Can Parallel: NO | Wave 3 | Blocks: [8, 9] | Blocked By: [3]

  **References** (executor has NO interview context - be exhaustive):
  - Bug path: `src/codegen/binding_store.rs` `store_binding_overwrite_rc_safe` - Should skip `opal_rc_inc` only for `StoreMode::TakeOwned`.
  - Bug path: `src/codegen/statements.rs` - Assignment lowering currently calls store helper; let init logic around identifier retain is only a reference, not a full solution.
  - Bug path: `src/codegen/functions_call/array/helpers.rs` - `store_array_binding` must accept/pass explicit store mode or offer a named take-owned variant.
  - Bug path: `src/codegen/functions_call/array/intrinsics.rs` - Array push unique grow/shared fallback paths store new arrays back into receiver binding.
  - API/Type: `runtime/opal_rc.c` - `opal_rc_alloc_tracked` initializes refcount to 1; this invariant must remain.
  - Source: `test-projects/game-of-life-full/src/rules.op` - `next_generation` hot path with `next_board.push(...)`.

  **Acceptance Criteria** (agent-executable only):
  - [ ] Pre-fix Task 2 RED evidence exists and shows at least one RC leak assertion failure.
  - [ ] Post-fix `cargo test --features integration --test integration_e2e rc_store_ -- --nocapture --test-threads=1` exits 0.
  - [ ] `bash scripts/array_memory_sanitizer.sh` exits 0 or, if Task 9 has not wired new tests yet, existing sanitizer selectors still exit 0.
  - [ ] Changed diff shows every `StoreMode::TakeOwned` use at a site that constructs/stores a fresh value in the same lowering flow.

  **QA Scenarios** (MANDATORY - task incomplete without these):
  ```
  Scenario: RC leak tests go GREEN
    Tool: Bash
    Steps: Run `cargo test --features integration --test integration_e2e rc_store_ -- --nocapture --test-threads=1` after the RC fix.
    Expected: Command exits 0; direct assignment, push no-grow, push grow, self-overwrite, aliasing, and Second-Class Ref-adjacent cases pass.
    Evidence: .sisyphus/evidence/task-4-rc-store-green.txt

  Scenario: Existing sanitizer safety remains clean
    Tool: Bash
    Steps: Run `bash scripts/array_memory_sanitizer.sh` or the existing array sanitizer selectors if CI wiring is deferred.
    Expected: Command exits 0 with no `heap-use-after-free`, `double-free`, or `detected memory leaks` markers.
    Evidence: .sisyphus/evidence/task-4-sanitizer.txt
  ```

  **Commit**: YES | Message: `fix(codegen): take owned rc values on proven fresh stores` | Files: [src/codegen/binding_store.rs, src/codegen/statements.rs, src/codegen/functions_call/array/helpers.rs, src/codegen/functions_call/array/intrinsics.rs, tests/integration_e2e/rc_store_leak_regressions.rs, tests/integration_e2e/fixtures/rc_store_leak_regressions.c]

- [x] 5. Verify call-temp measurement and design generic cleanup test probe

  **What to do**: Determine how the repository can deterministically observe malloc-backed interpolation leaks. Start with Task 1 audit. If `opal_runtime_live_heap_bytes()` does not include malloc strings from `runtime/opal_string.c` / `expressions_string.rs`, choose one maintainable test signal: existing sanitizer/LSAN hook, a small C malloc-stats probe, or a scoped runtime test-only malloc counter guarded by `OPAL_ENABLE_INTERNAL_TESTING`. Document the selected signal in `.sisyphus/evidence/task-5-call-temp-measurement.md` and in comments in the eventual tests.
  **Must NOT do**: Do not add a broad production runtime allocation framework for this bug; do not use an unbounded or flaky RSS-only assertion for deterministic call-temp RED tests.

  **Recommended Agent Profile**:
  - Category: `deep` - Requires selecting a reliable measurement without expanding runtime scope.
  - Skills: [] - No special skill needed.
  - Omitted: [`frontend-ui-ux`] - No UI work.

  **Parallelization**: Can Parallel: YES | Wave 1 | Blocks: [6] | Blocked By: [1]

  **References** (executor has NO interview context - be exhaustive):
  - API/Type: `src/codegen/expressions_string.rs` - `codegen_string_interpolation` and `allocate_interpolation_buffer` allocate the interpolation result.
  - API/Type: `runtime/opal_string.c` - `int64_to_string` and similar functions allocate caller-owned strings with `malloc`.
  - API/Type: `src/codegen/scope_tracker.rs` - Existing logic for binding-based malloc-string cleanup.
  - Test: `scripts/array_memory_sanitizer.sh` - ASAN/LSAN or Valgrind path may detect malloc leaks if harness is designed to exit cleanly.
  - Pattern: `tests/integration_e2e/scope_leak_counters.rs` - Existing leak-counter-related integration tests, if relevant during implementation.

  **Acceptance Criteria** (agent-executable only):
  - [ ] `.sisyphus/evidence/task-5-call-temp-measurement.md` identifies the chosen metric and why it is stable.
  - [ ] The chosen metric can distinguish pre-fix leaked direct interpolation call arguments from post-fix cleanup.
  - [ ] If a test-only runtime counter is required, the plan notes it must be behind `OPAL_ENABLE_INTERNAL_TESTING` and not change production ABI behavior.

  **QA Scenarios** (MANDATORY - task incomplete without these):
  ```
  Scenario: Call-temp metric selected
    Tool: Bash
    Steps: Inspect `.sisyphus/evidence/task-5-call-temp-measurement.md` for `selected metric` and `pre-fix signal`.
    Expected: Evidence names a deterministic primary metric and states why RSS alone is not used.
    Evidence: .sisyphus/evidence/task-5-call-temp-measurement.md

  Scenario: Existing string/scope tests still pass before new changes
    Tool: Bash
    Steps: Run `cargo test --features integration --test integration_e2e scope_leak_counters -- --nocapture --test-threads=1` if that selector exists; otherwise run `cargo test --features integration --test integration_e2e string_ -- --nocapture --test-threads=1`.
    Expected: Command exits 0 or missing selector is documented; no production changes are made.
    Evidence: .sisyphus/evidence/task-5-existing-string-tests.txt
  ```

  **Commit**: NO | Message: N/A | Files: [.sisyphus/evidence/task-5-call-temp-measurement.md]

- [x] 6. Add call-argument temporary RED regressions

  **What to do**: Add tests that fail before the call-temp cleanup fix. Create `tests/integration_e2e/call_temp_leak_regressions.rs`, register it in `tests/integration_e2e/tests.rs` as `mod call_temp_leak_regressions;`, and define these exact Rust test functions: `call_temp_owned_arg_freed_on_return`, `call_temp_owned_arg_freed_on_propagate`, `call_temp_mixed_disposition`, `call_temp_nested_later_failure_cleanup`, and `call_temp_take_owned_no_double_free`. Cover direct interpolation passed to `writer_write_sync`, mixed borrowed and owned arguments, nested/direct calls where a later failure must clean earlier temporaries, propagate/early return, and a transferred/take-owned argument case that must not double-free. Use the measurement strategy from Task 5. Prefer integration tests under `tests/integration_e2e/` with fixtures or inline `.op` sources compiled by `compile_program_for_tests`, matching existing helper style.
  **Must NOT do**: Do not implement cleanup in this task; do not double-count let-bound strings that `scope_tracker.rs` already owns; do not add flaky RSS-only assertions.

  **Recommended Agent Profile**:
  - Category: `unspecified-high` - Test harness work with failure-path coverage.
  - Skills: [] - No special skill needed.
  - Omitted: [`playwright`] - No browser testing.

  **Parallelization**: Can Parallel: NO | Wave 2 | Blocks: [7] | Blocked By: [5]

  **References** (executor has NO interview context - be exhaustive):
  - Bug path: `src/codegen/functions_call.rs` - Argument lowering currently lacks a temporary cleanup scope and propagate early-return cleanup.
  - Bug path: `src/codegen/expressions_string.rs` - Interpolation output buffer is allocated and returned to caller.
  - Bug path: `test-projects/game-of-life-full/src/render.op` - Uses direct interpolation argument to `writer_write_sync`.
  - Pattern: `tests/integration_e2e.rs` - `compile_program_for_tests`, `run_binary_output_with_timeout`, and command timeout helpers.
  - Pattern: `tests/integration_e2e/tests.rs` - Register new test module.

  **Acceptance Criteria** (agent-executable only):
  - [ ] Pre-fix RED run of `cargo test --features integration --test integration_e2e tests::call_temp_leak_regressions::call_temp_owned_arg_freed_on_return -- --exact --nocapture --test-threads=1` exits non-zero and output is saved to `.sisyphus/evidence/task-6-direct-interpolation-red.txt`.
  - [ ] Pre-fix RED run of `cargo test --features integration --test integration_e2e tests::call_temp_leak_regressions::call_temp_owned_arg_freed_on_propagate -- --exact --nocapture --test-threads=1` exits non-zero and output is saved to `.sisyphus/evidence/task-6-propagate-red.txt`.
  - [ ] Test functions exactly include `call_temp_owned_arg_freed_on_return`, `call_temp_owned_arg_freed_on_propagate`, `call_temp_mixed_disposition`, `call_temp_nested_later_failure_cleanup`, and `call_temp_take_owned_no_double_free`.
  - [ ] Each test has an explicit timeout using existing helper patterns.

  **QA Scenarios** (MANDATORY - task incomplete without these):
  ```
  Scenario: Direct interpolation call RED proof
    Tool: Bash
    Steps: Run `cargo test --features integration --test integration_e2e tests::call_temp_leak_regressions::call_temp_owned_arg_freed_on_return -- --exact --nocapture --test-threads=1` before cleanup fix.
    Expected: Command exits non-zero due to leaked owned temporary metric or sanitizer leak marker.
    Evidence: .sisyphus/evidence/task-6-direct-interpolation-red.txt

  Scenario: Propagate early-return RED proof
    Tool: Bash
    Steps: Run `cargo test --features integration --test integration_e2e tests::call_temp_leak_regressions::call_temp_owned_arg_freed_on_propagate -- --exact --nocapture --test-threads=1` before cleanup fix.
    Expected: Command exits non-zero due to leaked owned temporary on the error path, not due to compile failure.
    Evidence: .sisyphus/evidence/task-6-propagate-red.txt
  ```

  **Commit**: CONDITIONAL | Message: `test(memory): add call argument temporary regressions` | Files: [tests/integration_e2e/tests.rs, tests/integration_e2e/call_temp_leak_regressions.rs, optional tests/integration_e2e/fixtures/call_temp_leak_regressions.c]

- [x] 7. Implement generic ephemeral owned call-argument cleanup on all exits

  **What to do**: In `src/codegen/functions_call.rs`, introduce a generic cleanup scope/list for ephemeral owned argument temporaries produced during argument lowering. Track disposition per argument: borrowed/no cleanup, owned-cleanup-by-caller, or transferred/moved so cleanup is disarmed. Include `Expr::StringInterpolation` and known malloc-returning runtime calls such as `*_to_string` only when they are ephemeral direct arguments, not identifiers/bindings already owned by `scope_tracker.rs`. Emit cleanup on normal return and before propagate early return; also clean already-created temporaries if later argument evaluation or callee setup fails in generated control flow. Use existing `free` declaration/helper patterns rather than duplicating ad hoc declarations. The GREEN proof must rerun the exact Task 6 test functions under module path `tests::call_temp_leak_regressions::*`.
  **Must NOT do**: Do not double-free let-bound strings; do not special-case only Game of Life; do not leak earlier temporaries when a later argument or propagated call fails; do not transfer ownership to callees unless the callee ABI explicitly requires it.

  **Recommended Agent Profile**:
  - Category: `deep` - Complex codegen control-flow/lifetime correctness.
  - Skills: [] - No special skill needed.
  - Omitted: [`frontend-ui-ux`] - No UI work.

  **Parallelization**: Can Parallel: NO | Wave 3 | Blocks: [8, 9] | Blocked By: [6]

  **References** (executor has NO interview context - be exhaustive):
  - Bug path: `src/codegen/functions_call.rs` - Argument lowering around call construction and propagate lowering must run cleanup on every exit edge.
  - Bug path: `src/codegen/expressions_string.rs` - `codegen_string_interpolation` returns a malloc-backed buffer for the interpolation result.
  - API/Type: `src/codegen/scope_tracker.rs` - Existing binding-based malloc cleanup and predicates for owned strings; avoid double-freeing tracked bindings.
  - API/Type: `runtime/opal_string.c` - `int64_to_string` returns caller-owned malloc string.
  - Source: `test-projects/game-of-life-full/src/render.op` - Direct interpolation argument to `writer_write_sync` reproduces per-frame leak.

  **Acceptance Criteria** (agent-executable only):
  - [ ] Pre-fix Task 6 RED evidence exists for direct interpolation and propagate early-return leaks.
  - [ ] Post-fix `cargo test --features integration --test integration_e2e tests::call_temp_leak_regressions::call_temp_owned_arg_freed_on_return -- --exact --nocapture --test-threads=1` exits 0.
  - [ ] Post-fix `cargo test --features integration --test integration_e2e tests::call_temp_leak_regressions::call_temp_owned_arg_freed_on_propagate -- --exact --nocapture --test-threads=1` exits 0.
  - [ ] Post-fix `cargo test --features integration --test integration_e2e tests::call_temp_leak_regressions::call_temp_mixed_disposition -- --exact --nocapture --test-threads=1` exits 0.
  - [ ] Post-fix `cargo test --features integration --test integration_e2e tests::call_temp_leak_regressions::call_temp_nested_later_failure_cleanup -- --exact --nocapture --test-threads=1` exits 0.
  - [ ] Post-fix `cargo test --features integration --test integration_e2e tests::call_temp_leak_regressions::call_temp_take_owned_no_double_free -- --exact --nocapture --test-threads=1` exits 0.
  - [ ] Sanitizer or selected malloc metric reports no leaked ephemeral temporaries and no double-free.
  - [ ] `cargo test --workspace` exits 0.

  **QA Scenarios** (MANDATORY - task incomplete without these):
  ```
  Scenario: Call-temp leak tests go GREEN
    Tool: Bash
    Steps: Run `cargo test --features integration --test integration_e2e tests::call_temp_leak_regressions::call_temp_owned_arg_freed_on_return -- --exact --nocapture --test-threads=1`, `cargo test --features integration --test integration_e2e tests::call_temp_leak_regressions::call_temp_owned_arg_freed_on_propagate -- --exact --nocapture --test-threads=1`, `cargo test --features integration --test integration_e2e tests::call_temp_leak_regressions::call_temp_mixed_disposition -- --exact --nocapture --test-threads=1`, `cargo test --features integration --test integration_e2e tests::call_temp_leak_regressions::call_temp_nested_later_failure_cleanup -- --exact --nocapture --test-threads=1`, and `cargo test --features integration --test integration_e2e tests::call_temp_leak_regressions::call_temp_take_owned_no_double_free -- --exact --nocapture --test-threads=1` after implementing cleanup.
    Expected: Command exits 0 for direct interpolation, mixed borrowed/owned args, nested later-failure cleanup, propagate early return, and transferred/no-double-free cases.
    Evidence: .sisyphus/evidence/task-7-call-temp-green.txt

  Scenario: Workspace remains stable
    Tool: Bash
    Steps: Run `cargo test --workspace`.
    Expected: Command exits 0; no unrelated regressions from call lowering changes.
    Evidence: .sisyphus/evidence/task-7-workspace-tests.txt
  ```

  **Commit**: YES | Message: `fix(codegen): clean owned call argument temporaries on all exits` | Files: [src/codegen/functions_call.rs, src/codegen/expressions_string.rs if ownership marker is needed, src/codegen/scope_tracker.rs if predicate exposure is needed, tests/integration_e2e/call_temp_leak_regressions.rs, optional tests/integration_e2e/fixtures/call_temp_leak_regressions.c]

- [x] 8. Add timeout-bounded Game of Life Full executable memory stress test

  **What to do**: Add an ignored/gated integration stress test, e.g. `tests/integration_e2e/game_of_life_full_memory_stress.rs`, and register it in `tests/integration_e2e/tests.rs`. The test must compile `test-projects/game-of-life-full` with `compile_project_for_tests`, spawn the actual compiled executable, sample process memory or an exported test metric at fixed intervals, kill the process at the end, and enforce both a sample/iteration target and wall-clock timeout. Use `OPAL_RUN_STRESS=1` plus `#[ignore]` so default `cargo test` does not run the ~15s test. Use warm-up samples and assert bounded post-warm-up slope/spread; print all samples and threshold on failure. Run it before fixes if feasible to capture failing evidence, then after fixes to capture GREEN evidence.
  **Must NOT do**: Do not allow the infinite Game of Life executable to run without timeout/kill; do not use raw absolute RSS peak alone; do not run this ignored stress test in default test suite.

  **Recommended Agent Profile**:
  - Category: `unspecified-high` - Integration harness, process management, and stress verification.
  - Skills: [] - No special skill needed.
  - Omitted: [`playwright`] - No browser testing.

  **Parallelization**: Can Parallel: NO | Wave 4 | Blocks: [9] | Blocked By: [4, 7]

  **References** (executor has NO interview context - be exhaustive):
  - Pattern: `tests/integration_e2e.rs` - `compile_project_for_tests`, `run_command_output_with_timeout`, and `run_binary_output_with_timeout` helper patterns.
  - Pattern: `tests/integration_e2e/game_of_life.rs` - Existing Game of Life integration module style.
  - Source: `test-projects/game-of-life-full/opal.toml` - Project manifest to compile.
  - Source: `test-projects/game-of-life-full/src/main.op` - Infinite loop; stress harness must timeout/kill.
  - Source: `test-projects/game-of-life-full/src/rules.op` - Array churn hot path.
  - Source: `test-projects/game-of-life-full/src/render.op` - String interpolation hot path.
  - Test: `scripts/array_memory_sanitizer.sh` - Optional explicit invocation point for ignored stress.

  **Acceptance Criteria** (agent-executable only):
  - [ ] `OPAL_RUN_STRESS=1 cargo test --features integration --test integration_e2e tests::game_of_life_full_memory_stress::game_of_life_full_memory_stress -- --ignored --exact --nocapture --test-threads=1` exits 0 after both fixes.
  - [ ] The stress test has an explicit wall-clock timeout no greater than 20 seconds for a nominal 15-second sampling window.
  - [ ] The stress test fails explicitly if fewer than `MIN_SAMPLES` are collected.
  - [ ] Failure output includes sample series, warm-up count, slope/spread threshold, and final child process status/kill result.
  - [ ] Pre-fix failure evidence is saved if feasible; if not feasible because fixes are already present, evidence notes why targeted RED tests are the RED drivers.

  **QA Scenarios** (MANDATORY - task incomplete without these):
  ```
  Scenario: Stress test cannot run forever
    Tool: Bash
    Steps: Run `OPAL_RUN_STRESS=1 cargo test --features integration --test integration_e2e tests::game_of_life_full_memory_stress::game_of_life_full_memory_stress -- --ignored --exact --nocapture --test-threads=1`.
    Expected: Command exits within 20 seconds after test body starts sampling; child process is killed/collected even on assertion failure.
    Evidence: .sisyphus/evidence/task-8-stress-timeout.txt

  Scenario: Game of Life Full bounded memory after fixes
    Tool: Bash
    Steps: Run the same ignored stress command after Tasks 4 and 7.
    Expected: Command exits 0; post-warm-up memory slope/spread is within documented threshold and sample count >= MIN_SAMPLES.
    Evidence: .sisyphus/evidence/task-8-stress-green.txt
  ```

  **Commit**: YES | Message: `test(gol): add bounded memory stress for full executable` | Files: [tests/integration_e2e/tests.rs, tests/integration_e2e/game_of_life_full_memory_stress.rs, optional tests/integration_e2e/fixtures/gol_memory_probe.c]

- [x] 9. Wire memory verification, verify atomic commits, and finalize evidence

  **What to do**: Add deterministic new memory verification selectors to `scripts/array_memory_sanitizer.sh` `MEMORY_VERIFICATION_TESTS`. Keep the ignored stress test out of default hooks unless explicitly invoked with `OPAL_RUN_STRESS=1`; if CI should run it, add a clearly named separate step or documented command. Run final commands, capture evidence, and inspect git history/diffs for atomicity. Ensure every planned commit has a focused scope and no unintended files are staged.
  **Must NOT do**: Do not make default CI flaky by silently running the 15s stress in every `cargo test`; do not stage unrelated files; do not rewrite history unless explicitly requested.

  **Recommended Agent Profile**:
  - Category: `deep` - Cross-cutting verification and git/CI discipline.
  - Skills: [`git-master`] - Required for git operations/atomic commit verification if commits are requested.
  - Omitted: [`frontend-ui-ux`] - No UI work.

  **Parallelization**: Can Parallel: NO | Wave 4 | Blocks: [Final] | Blocked By: [8]

  **References** (executor has NO interview context - be exhaustive):
  - Test: `scripts/array_memory_sanitizer.sh` - Add deterministic selectors to `MEMORY_VERIFICATION_TESTS`; preserve ASAN/Valgrind logic.
  - Test: `.github/workflows/ci.yml` - Existing CI invokes memory sanitizer script; edit only if a separate ignored stress step is intentionally added.
  - Command: `cargo test --workspace` - Final non-ignored regression suite.
  - Command: `bash scripts/array_memory_sanitizer.sh` - Final memory/sanitizer verification.
  - Command: `OPAL_RUN_STRESS=1 cargo test --features integration --test integration_e2e tests::game_of_life_full_memory_stress::game_of_life_full_memory_stress -- --ignored --exact --nocapture --test-threads=1` - Final executable stress verification.

  **Acceptance Criteria** (agent-executable only):
  - [ ] `cargo test --workspace` exits 0.
  - [ ] `bash scripts/array_memory_sanitizer.sh` exits 0 and includes deterministic new memory selectors.
  - [ ] Ignored stress command exits 0 when explicitly invoked with `OPAL_RUN_STRESS=1`.
  - [ ] `git status`, `git diff`, and `git log --oneline -10` have been inspected before committing.
  - [ ] Commit boundaries match the atomic commit strategy or deviations are documented in `.sisyphus/evidence/task-9-atomicity.md`.

  **QA Scenarios** (MANDATORY - task incomplete without these):
  ```
  Scenario: Final automated verification
    Tool: Bash
    Steps: Run `cargo test --workspace`, `bash scripts/array_memory_sanitizer.sh`, and the explicit ignored stress command.
    Expected: All commands exit 0; outputs saved separately.
    Evidence: .sisyphus/evidence/task-9-final-verification.txt

  Scenario: Atomic commit review
    Tool: Bash
    Steps: Inspect `git status`, `git diff`, and `git log --oneline -10`; verify staged files match planned commit scope.
    Expected: No unrelated changes; commit messages and file sets match the strategy or documented deviation.
    Evidence: .sisyphus/evidence/task-9-atomicity.md
  ```

  **Commit**: YES | Message: `ci(memory): wire leak regressions into sanitizer verification` | Files: [scripts/array_memory_sanitizer.sh, .github/workflows/ci.yml if separate stress step is added]

## Final Verification Wave (MANDATORY — after ALL implementation tasks)
> 4 review agents run in PARALLEL. ALL must APPROVE. Present consolidated results to user and get explicit "okay" before completing.
> **Do NOT auto-proceed after verification. Wait for user's explicit approval before marking work complete.**
> **Never mark F1-F4 as checked before getting user's okay.** Rejection or user feedback -> fix -> re-run -> present again -> wait for okay.
- [x] F1. Plan Compliance Audit — oracle
- [x] F2. Code Quality Review — unspecified-high
- [x] F3. Real Manual QA — unspecified-high
- [x] F4. Scope Fidelity Check — deep

## Commit Strategy
- Use atomic commits. Recommended sequence when tests can be committed ignored or non-default before fixes:
  1. `test(memory): add rc store leak regressions`
  2. `fix(codegen): take owned rc values on proven fresh stores`
  3. `test(memory): add call argument temporary regressions`
  4. `fix(codegen): clean owned call argument temporaries on all exits`
  5. `test(gol): add timeout bounded memory stress for full executable`
  6. `ci(memory): wire leak regressions into sanitizer verification`
- If repository policy forbids committing failing tests, combine each RED test with its fix in one bisectable commit and attach pre-fix failure output in `.sisyphus/evidence/` before committing. Do not commit `.sisyphus/evidence/` unless the repository already tracks evidence files.
- Before any commit, inspect `git status`, `git diff`, and `git log --oneline -10`; stage only intended files.

## Success Criteria
- The RC-array leak cannot reappear without failing deterministic RC store tests.
- The direct interpolation/call-temp leak cannot reappear without failing deterministic call-temp tests or sanitizer probes.
- The actual Game of Life Full executable runs under the stress harness for the configured sample window without unbounded post-warm-up memory growth and cannot hang indefinitely.
- Existing test and sanitizer suites remain green.
- The implementation remains aligned with Perceus-style ownership transfer and Second-Class Reference direction.
