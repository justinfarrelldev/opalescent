# RC Return Assignment Memory Leak Fix

## TL;DR
> **Summary**: Fix the Game of Life leak by aligning mutable assignment with existing `let` initialization ownership semantics: RC-typed call RHS values are treated as owned/fresh and stored with `StoreMode::TakeOwned` instead of being retained again. This avoids the metadata redesign; alias-return ownership remains a documented pre-existing limitation.
> **Deliverables**:
> - Failing-first regression for `board = next_generation(...)`-style mutable reassignment from a fresh user-function call
> - 2-minute ignored, opt-in stress test with hard timeout and deterministic leak assertion
> - Narrow `src/codegen/statements.rs` fix: `assignment_store_mode(value, binding_type)` returns `TakeOwned` for RC-typed `Expr::Call`
> - Ignored characterization test/documentation for pre-existing alias-return unsoundness
> - Atomic test-first commits and verification evidence
> **Effort**: Short
> **Parallel**: YES - 3 waves
> **Critical Path**: Task 1 → Task 3 → Task 4 → Task 6

## Context
### Original Request
Fix a remaining Game of Life memory leak unrelated to `.sisyphus/plans/game-of-life-memory-leaks.md`. The user requires the real underlying issue be fixed, not the source workaround, and requires tests first, including a 2-minute bounded stress test. Execution must use atomic commits.

### Interview Summary
- Explicitly forbidden: changing Game of Life source to `let next_board = next_generation(...); board = next_board` as a workaround.
- Required: maintainable compiler fix with extreme care.
- Required: add failing tests before implementation.
- Required: add stress test similar to the existing one, but long enough to catch this leak and bounded by timeout.
- User supplied a second developer's narrower diagnosis; direct verification confirmed it is true for the current code.

### Metis Review (gaps addressed)
- The actual bug is an assignment/let asymmetry, not a need for return-ownership metadata.
- `codegen_let_statement` already treats non-identifier initializers, including calls, as owned by not retaining them.
- `assignment_store_mode` currently retains ordinary calls, causing the one-board-per-generation leak.
- Fix scope should be narrow: `assignment_store_mode` signature/body plus its caller in `codegen_assignment`.
- Alias/parameter-returning functions are a pre-existing limitation already present in the `let` path; document it, do not solve it in this PR.
- Keep runtime RC semantics unchanged and avoid provenance/metadata/escape-analysis scope creep.

### Verified Code Facts
- `src/codegen/statements.rs:237`: `let retain_new_value = matches!(*initializer_expr, Expr::Identifier { .. });`; call initializers are not retained.
- `src/codegen/statements.rs:632-649`: assignment lowers RHS with `Some(&binding_type)` then calls `assignment_store_mode(value)`.
- `src/codegen/statements.rs:933-944`: `assignment_store_mode` returns `TakeOwned` for array literals and `reserve(...)`, otherwise `Retain`.
- `src/codegen/binding_store.rs:19-24`: `StoreMode::TakeOwned` is intended only when the lowering site proves the stored RC value is fresh/linear.
- `src/codegen/binding_store.rs:92-108`: assignment store prepares new value according to mode, stores it, then releases old value.
- `src/codegen/control_flow.rs:559-577`: return lowering preserves returned identifier names so local owned values can transfer out.

## Work Objectives
### Core Objective
Eliminate the per-generation RC array leak caused by mutable assignment retaining an owned call result, by making assignment follow the existing let-initializer call ownership rule for RC bindings.

### Deliverables
- Tests-first regression coverage for `board = next_generation(...)`-style returned fresh array assignment.
- 2-minute ignored, opt-in stress test with timeout and deterministic counter/RSS assertion.
- Narrow compiler fix in `src/codegen/statements.rs`.
- Caller audit proving `assignment_store_mode` and `initialize_binding_value` paths are understood.
- Known limitation documentation/ignored characterization for alias-returning functions.
- Evidence files under `.sisyphus/evidence/` produced during execution.

### Definition of Done (verifiable conditions with commands)
- `cargo test --features integration board_reassignment_from_user_fn_no_leak -- --nocapture` fails before the fix and exits 0 after the fix.
- `OPAL_RUN_STRESS=1 cargo test --features integration game_of_life_rc_return_stress -- --ignored --nocapture --test-threads=1` fails before the fix, exits 0 after the fix, and never runs longer than 130 seconds.
- `cargo test --features integration rc_store_leak_regressions -- --nocapture` exits 0 after the fix.
- `cargo test` exits 0.
- `cargo test --features integration` exits 0.
- `bash scripts/array_memory_sanitizer.sh` exits 0 if present and executable.

### Must Have
- Tests committed before production code changes.
- `assignment_store_mode` receives `binding_type: &CoreType` or equivalent target type context.
- Existing array literal and `reserve(...)` `TakeOwned` behavior preserved.
- New call arm gated on binding RC cleanup predicate, not on calls alone.
- No changes to `codegen_let_statement`, `initialize_binding_value`, return lowering, or RC runtime semantics.
- Alias-return limitation explicitly documented as pre-existing and out of scope.
- Stress test has both `#[ignore]` and `OPAL_RUN_STRESS=1` gating plus hard timeout.
- Atomic commits as specified in each task.

### Must NOT Have (guardrails, AI slop patterns, scope boundaries)
- Must not use the Game of Life source workaround.
- Must not introduce return-ownership metadata, callee-side annotations, provenance tracking, escape analysis, or borrow checking.
- Must not change `runtime/opal_rc.c` or `runtime/opal_rc.h` semantics.
- Must not modify `codegen_let_statement`, `initialize_binding_value`, or `src/codegen/control_flow.rs` return lowering.
- Must not treat calls into non-RC bindings as `TakeOwned`.
- Must not rely on RSS-only stress-test pass criteria if runtime counters are available.

### Known Limitations
- Functions returning borrowed aliases/parameters are already unsound under current `let` initialization semantics because call initializers are not retained.
- This plan intentionally does not solve alias-return provenance. It aligns assignment with existing let behavior to fix the Game of Life fresh-return leak.
- Add an ignored characterization test documenting this limitation so it is discoverable and can seed a separate future plan.

## Verification Strategy
> ZERO HUMAN INTERVENTION - all verification is agent-executed.
- Test decision: TDD / RED-GREEN-REFACTOR using Rust `cargo test` and integration feature tests.
- QA policy: Every task has agent-executed scenarios.
- Evidence: `.sisyphus/evidence/task-{N}-{slug}.{ext}`

## Execution Strategy
### Parallel Execution Waves
> Target: 5-8 tasks per wave. <3 per wave (except final) = under-splitting.
> Extract shared dependencies as Wave-1 tasks for max parallelism.

Wave 1: Task 1 (focused RED regression), Task 2 (2-minute stress)
Wave 2: Task 3 (caller/path audit), Task 4 (narrow compiler fix after Task 3)
Wave 3: Task 5 (stress/sanitizer verification), Task 6 (full verification and atomicity audit after Task 5)

### Dependency Matrix (full, all tasks)
- Task 1: blocks Tasks 3, 4, and 6; independent of Task 2.
- Task 2: blocks Task 5 and Task 6; independent of Task 1.
- Task 3: blocked by Task 1; blocks Task 4.
- Task 4: blocked by Tasks 1 and 3; blocks Tasks 5 and 6.
- Task 5: blocked by Tasks 2 and 4; blocks Task 6.
- Task 6: blocked by all prior tasks.

### Agent Dispatch Summary (wave → task count → categories)
- Wave 1 → 2 tasks → `deep`, `unspecified-high`
- Wave 2 → 2 tasks → `quick`, `quick`
- Wave 3 → 2 tasks → `quick`, `unspecified-high`

## TODOs
> Implementation + Test = ONE task. Never separate.
> EVERY task MUST have: Agent Profile + Parallelization + QA Scenarios.

- [x] 1. Add failing focused reassignment regression

  **What to do**: Add a test-first regression in `tests/integration_e2e/rc_store_leak_regressions.rs` named `board_reassignment_from_user_fn_no_leak`. Use the existing RC-counter fixture style in `tests/integration_e2e/fixtures/rc_store_leak_regressions.c`. The Opalescent snippet must initialize a mutable `int8[]` board, run at least 100 reassignment iterations, assign `board = next_generation(board, width, height)`, and have `next_generation` build and return a fresh local array. Assert live RC allocations/bytes/counter delta returns to steady state after final cleanup. Add a separate ignored characterization test named `alias_return_assignment_known_limitation` that documents the pre-existing alias-return unsoundness and is not part of the required green suite.
  **Must NOT do**: Do not change production code in this task. Do not alter Game of Life app source. Do not make the fresh-return regression ignored. Do not try to fix alias-return behavior in this task.

  **Recommended Agent Profile**:
  - Category: `deep` - Reason: Requires careful fixture-based RC-counter regression design.
  - Skills: [] - No special skill required.
  - Omitted: [`git-master`] - Normal atomic commit only; no history rewriting.

  **Parallelization**: Can Parallel: YES | Wave 1 | Blocks: Tasks 4, 6 | Blocked By: none

  **References** (executor has NO interview context - be exhaustive):
  - Pattern: `tests/integration_e2e/rc_store_leak_regressions.rs` - Existing RC store leak regression harness.
  - Pattern: `tests/integration_e2e/fixtures/rc_store_leak_regressions.c` - Existing C fixture for generated object/runtime counter checks.
  - API/Type: `src/codegen/statements.rs:237` - Let path treats call initializers as owned by not retaining them.
  - API/Type: `src/codegen/statements.rs:933-944` - Current assignment mode misses ordinary calls.
  - API/Type: `src/codegen/binding_store.rs:19-24` - `TakeOwned` invariant for fresh/linear RC values.
  - Context: `test-projects/game-of-life-full/src/main.op` and `test-projects/game-of-life-full/src/rules.op` - Real leak shape.

  **Acceptance Criteria** (agent-executable only):
  - [ ] `cargo test --features integration board_reassignment_from_user_fn_no_leak -- --nocapture` fails before the fix due to non-zero live allocation/counter delta, not compile error or timeout.
  - [ ] `cargo test --features integration rc_store_leak_regressions -- --nocapture` shows only the new positive regression failing before the fix; existing tests remain in their prior state.
  - [ ] `cargo test --features integration alias_return_assignment_known_limitation -- --ignored --nocapture` demonstrates/document the limitation and is not required to pass as part of normal suite.
  - [ ] `.sisyphus/evidence/task-1-reassignment-red.txt` captures the RED output.

  **QA Scenarios** (MANDATORY - task incomplete without these):
  ```
  Scenario: Fresh call reassignment fails before fix
    Tool: Bash
    Steps: Run `cargo test --features integration board_reassignment_from_user_fn_no_leak -- --nocapture`.
    Expected: Command fails before production fix with explicit non-zero live allocation/counter delta.
    Evidence: .sisyphus/evidence/task-1-reassignment-red.txt

  Scenario: Alias limitation characterized
    Tool: Bash
    Steps: Run `cargo test --features integration alias_return_assignment_known_limitation -- --ignored --nocapture`.
    Expected: Output documents expected correct vs current buggy alias-return behavior; test remains ignored and does not gate this fix.
    Evidence: .sisyphus/evidence/task-1-alias-known-limitation.txt
  ```

  **Commit**: YES | Message: `test(rc): add reassignment return leak regression` | Files: [`tests/integration_e2e/rc_store_leak_regressions.rs`, `tests/integration_e2e/fixtures/rc_store_leak_regressions.c`, `.sisyphus/evidence/task-1-*.txt`]

- [x] 2. Add 2-minute bounded Game of Life reassignment stress test

  **What to do**: Add a new ignored opt-in stress test named `game_of_life_rc_return_stress`, preferably in `tests/integration_e2e/game_of_life_full_memory_stress.rs` unless a separate module is cleaner. It must run for 120 seconds with hard timeout of 130 seconds or less, use `OPAL_RUN_STRESS=1`, and assert deterministic steady-state behavior. Primary pass criterion should be runtime live allocations/bytes/counters if available; RSS may be secondary with a bounded post-warmup threshold. The test should exercise the actual Game of Life project path when feasible; if counters are not exposed from the actual binary, use a generated Game-of-Life-equivalent loop and state that RSS on the actual binary is secondary evidence.
  **Must NOT do**: Do not rely on human observation. Do not leave test unbounded. Do not run it by default without `--ignored` and `OPAL_RUN_STRESS=1`.

  **Recommended Agent Profile**:
  - Category: `unspecified-high` - Reason: Long-running stress tests need robust gating, timeout, and cleanup.
  - Skills: [] - No special skill required.
  - Omitted: [`frontend-ui-ux`] - No UI work.

  **Parallelization**: Can Parallel: YES | Wave 1 | Blocks: Tasks 5, 6 | Blocked By: none

  **References** (executor has NO interview context - be exhaustive):
  - Pattern: `tests/integration_e2e/game_of_life_full_memory_stress.rs` - Existing ignored stress test, `OPAL_RUN_STRESS=1`, sampling loop, timeout constants.
  - Pattern: `tests/integration_e2e/tests.rs` - Integration test module registration.
  - Pattern: `test-projects/game-of-life-full/README.md` - Existing app is infinite; timeout is mandatory.

  **Acceptance Criteria** (agent-executable only):
  - [ ] `cargo test --features integration game_of_life_rc_return_stress -- --ignored --nocapture --test-threads=1` exits quickly or skips unless `OPAL_RUN_STRESS=1` is set.
  - [ ] `OPAL_RUN_STRESS=1 cargo test --features integration game_of_life_rc_return_stress -- --ignored --nocapture --test-threads=1` fails before the fix due to counter/RSS growth and finishes within 130 seconds.
  - [ ] `.sisyphus/evidence/task-2-stress-red.txt` captures skip and RED/stress output.

  **QA Scenarios** (MANDATORY - task incomplete without these):
  ```
  Scenario: Stress opt-in gate works
    Tool: Bash
    Steps: Run `cargo test --features integration game_of_life_rc_return_stress -- --ignored --nocapture --test-threads=1` without `OPAL_RUN_STRESS`.
    Expected: Exits without a 120-second run; output indicates opt-in requirement.
    Evidence: .sisyphus/evidence/task-2-stress-skip.txt

  Scenario: Stress catches leak before fix
    Tool: Bash
    Steps: Run `OPAL_RUN_STRESS=1 cargo test --features integration game_of_life_rc_return_stress -- --ignored --nocapture --test-threads=1`.
    Expected: Fails before production fix due to positive steady-state growth and exits within 130 seconds.
    Evidence: .sisyphus/evidence/task-2-stress-red.txt
  ```

  **Commit**: YES | Message: `test(rc): add game of life reassignment stress` | Files: [`tests/integration_e2e/game_of_life_full_memory_stress.rs`, `tests/integration_e2e/tests.rs` if changed, `.sisyphus/evidence/task-2-*.txt`]

- [x] 3. Audit assignment/let ownership callers and predicates

  **What to do**: Before changing behavior, audit callers/references for `assignment_store_mode`, `initialize_binding_value`, and `binding_requires_rc_cleanup`. Confirm `assignment_store_mode` has only the assignment caller or document every caller. Confirm `initialize_binding_value` let semantics are intentionally untouched. Confirm the correct predicate to gate the new call arm is `binding_requires_rc_cleanup(&binding_type)` from `src/codegen/binding_store.rs`, or use an existing equivalent if more appropriate. Record findings in `.sisyphus/evidence/task-3-assignment-let-audit.md`.
  **Must NOT do**: Do not change production behavior. Do not expand into general sink/provenance audit.

  **Recommended Agent Profile**:
  - Category: `quick` - Reason: Small read-only caller/predicate audit.
  - Skills: [] - No special skill required.
  - Omitted: [`ai-slop-remover`] - No refactor/cleanup.

  **Parallelization**: Can Parallel: NO | Wave 2 | Blocks: Task 4 | Blocked By: Task 1

  **References** (executor has NO interview context - be exhaustive):
  - Pattern: `src/codegen/statements.rs:632-649` - Assignment caller to update.
  - Pattern: `src/codegen/statements.rs:933-944` - `assignment_store_mode` body to update.
  - Pattern: `src/codegen/binding_store.rs:127-129` - `binding_requires_rc_cleanup` predicate.
  - Pattern: `src/codegen/binding_store.rs:26-49` - `initialize_binding_value` let-init helper.

  **Acceptance Criteria** (agent-executable only):
  - [ ] `test -s .sisyphus/evidence/task-3-assignment-let-audit.md` exits 0.
  - [ ] Audit states exact caller count for `assignment_store_mode` and exact decision to leave `codegen_let_statement` unchanged.
  - [ ] `cargo test --features integration board_reassignment_from_user_fn_no_leak -- --nocapture` remains RED after audit-only work.

  **QA Scenarios** (MANDATORY - task incomplete without these):
  ```
  Scenario: Audit complete
    Tool: Bash
    Steps: Run `test -s .sisyphus/evidence/task-3-assignment-let-audit.md` and inspect it for `assignment_store_mode`, `initialize_binding_value`, and `binding_requires_rc_cleanup` entries.
    Expected: File exists and documents exact symbols/callers and no behavior changes.
    Evidence: .sisyphus/evidence/task-3-assignment-let-audit.md

  Scenario: Audit did not change behavior
    Tool: Bash
    Steps: Run `cargo test --features integration board_reassignment_from_user_fn_no_leak -- --nocapture`.
    Expected: Still fails with same leak assertion as Task 1.
    Evidence: .sisyphus/evidence/task-3-no-behavior-change.txt
  ```

  **Commit**: YES | Message: `test(rc): document reassignment ownership audit` | Files: [`.sisyphus/evidence/task-3-*.md`, `.sisyphus/evidence/task-3-*.txt`]

- [x] 4. Fix assignment_store_mode call ownership asymmetry

  **What to do**: Modify only `src/codegen/statements.rs` for the production behavior. Change `assignment_store_mode(value: &Expr)` to `assignment_store_mode(value: &Expr, binding_type: &CoreType)` or equivalent. Update the caller in `codegen_assignment` to pass `&binding_type`. Preserve existing array literal and `reserve(...)` arms semantically. Add a new `Expr::Call { .. }` arm after the `reserve(...)` arm that returns `StoreMode::TakeOwned` only when `binding_requires_rc_cleanup(binding_type)` is true. Otherwise return `StoreMode::Retain`. Add a short comment referencing the existing let initializer rule: call RHS values for RC bindings follow `codegen_let_statement` semantics and consume the call-owned result rather than retaining it again.
  **Must NOT do**: Do not change `codegen_let_statement`, `initialize_binding_value`, `control_flow.rs`, runtime files, or add return-ownership metadata. Do not alter array literal or reserve behavior.

  **Recommended Agent Profile**:
  - Category: `quick` - Reason: Narrow production code change after tests are in place.
  - Skills: [] - No special skill required.
  - Omitted: [`refactor`] - Avoid broad refactor; surgical change only.

  **Parallelization**: Can Parallel: NO | Wave 2 | Blocks: Tasks 5, 6 | Blocked By: Tasks 1, 3

  **References** (executor has NO interview context - be exhaustive):
  - Pattern: `src/codegen/statements.rs:237` - Existing let initializer no-retain behavior for calls.
  - Pattern: `src/codegen/statements.rs:632-649` - Assignment caller passing `binding_type`.
  - Pattern: `src/codegen/statements.rs:933-944` - Current `assignment_store_mode` match arms.
  - API/Type: `src/codegen/binding_store.rs:127-129` - `binding_requires_rc_cleanup` predicate.
  - API/Type: `src/codegen/binding_store.rs:19-24` - `StoreMode::TakeOwned` invariant.

  **Acceptance Criteria** (agent-executable only):
  - [ ] `cargo test --features integration board_reassignment_from_user_fn_no_leak -- --nocapture` exits 0 after the fix.
  - [ ] `cargo test --features integration rc_store_leak_regressions -- --nocapture` exits 0.
  - [ ] `cargo test` exits 0.
  - [ ] `git diff -- src/codegen/statements.rs` shows only the signature/caller/new-call-arm/comment change; no broad refactor.

  **QA Scenarios** (MANDATORY - task incomplete without these):
  ```
  Scenario: Focused leak regression green
    Tool: Bash
    Steps: Run `cargo test --features integration board_reassignment_from_user_fn_no_leak -- --nocapture`.
    Expected: Test exits 0 and reports steady live allocation/counter delta.
    Evidence: .sisyphus/evidence/task-4-reassignment-green.txt

  Scenario: Existing RC store regressions remain green
    Tool: Bash
    Steps: Run `cargo test --features integration rc_store_leak_regressions -- --nocapture`.
    Expected: Existing RC store tests and new fresh-return test exit 0; ignored alias limitation remains ignored.
    Evidence: .sisyphus/evidence/task-4-rc-store-green.txt
  ```

  **Commit**: YES | Message: `fix(codegen): take ownership of rc call assignment results` | Files: [`src/codegen/statements.rs`, `.sisyphus/evidence/task-4-*.txt`]

- [x] 5. Verify stress and sanitizer workflow after narrow fix

  **What to do**: Run the new 2-minute stress test after Task 4 and ensure it passes. Run the existing Game of Life memory stress test if distinct. Run `scripts/array_memory_sanitizer.sh` if present/executable. If stress results are flaky, do not relax to vague criteria; prefer counter-based steady-state assertion and document RSS only as secondary diagnostics.
  **Must NOT do**: Do not make ignored stress tests unconditional in CI. Do not change thresholds without evidence.

  **Recommended Agent Profile**:
  - Category: `quick` - Reason: Verification task after surgical fix.
  - Skills: [] - No special skill required.
  - Omitted: [`frontend-ui-ux`] - No UI.

  **Parallelization**: Can Parallel: NO | Wave 2 | Blocks: Task 6 | Blocked By: Tasks 2, 4

  **References** (executor has NO interview context - be exhaustive):
  - Pattern: `tests/integration_e2e/game_of_life_full_memory_stress.rs` - Stress test patterns and existing test.
  - Pattern: `scripts/array_memory_sanitizer.sh` - Sanitizer workflow.
  - Pattern: `.github/workflows/ci.yml` - CI timeout context.

  **Acceptance Criteria** (agent-executable only):
  - [ ] `OPAL_RUN_STRESS=1 cargo test --features integration game_of_life_rc_return_stress -- --ignored --nocapture --test-threads=1` exits 0 within 130 seconds.
  - [ ] `OPAL_RUN_STRESS=1 cargo test --features integration game_of_life_full_memory_stress -- --ignored --nocapture --test-threads=1` exits 0 if the existing test is separate and still present.
  - [ ] `bash scripts/array_memory_sanitizer.sh` exits 0 if present and executable.

  **QA Scenarios** (MANDATORY - task incomplete without these):
  ```
  Scenario: New stress green
    Tool: Bash
    Steps: Run `OPAL_RUN_STRESS=1 cargo test --features integration game_of_life_rc_return_stress -- --ignored --nocapture --test-threads=1`.
    Expected: Exits 0 within 130 seconds with steady-state counter/RSS assertion passing.
    Evidence: .sisyphus/evidence/task-5-stress-green.txt

  Scenario: Sanitizer workflow green
    Tool: Bash
    Steps: Run `bash scripts/array_memory_sanitizer.sh` if present/executable.
    Expected: Exit code 0; no `LeakSanitizer`, `AddressSanitizer`, Valgrind leak failure, or timeout.
    Evidence: .sisyphus/evidence/task-5-sanitizer-green.txt
  ```

  **Commit**: YES | Message: `test(rc): verify reassignment stress workflow` | Files: [`tests/integration_e2e/game_of_life_full_memory_stress.rs`, `scripts/array_memory_sanitizer.sh` if changed, `.sisyphus/evidence/task-5-*.txt`]

- [x] 6. Run full verification and atomic commit audit

  **What to do**: Run full verification, inspect git diff/log for atomic tests-first commits, and confirm guardrails. Ensure no source workaround in Game of Life, no runtime semantic changes, no metadata/provenance redesign, and no broad unrelated formatting churn. If any verification fails, create a focused follow-up commit and rerun affected checks.
  **Must NOT do**: Do not amend commits unless explicitly instructed. Do not mark final verification tasks complete until review agents approve and user gives explicit okay.

  **Recommended Agent Profile**:
  - Category: `unspecified-high` - Reason: Full QA plus guardrail/atomicity audit.
  - Skills: [] - No special skill required; load `/git-master` only for complex git history operations, not expected here.
  - Omitted: [`frontend-ui-ux`] - No UI.

  **Parallelization**: Can Parallel: NO | Wave 3 | Blocks: Final Verification Wave | Blocked By: Tasks 1-5

  **References** (executor has NO interview context - be exhaustive):
  - Guardrail: User required atomic commits and tests first.
  - Guardrail: `test-projects/game-of-life-full/src/main.op` source workaround is forbidden.
  - Guardrail: `runtime/opal_rc.c`, `runtime/opal_rc.h`, `src/codegen/control_flow.rs`, and `codegen_let_statement` should not be changed for this narrow fix.
  - Commands: `cargo test`, `cargo test --features integration`, stress test command, sanitizer script.

  **Acceptance Criteria** (agent-executable only):
  - [ ] `cargo test` exits 0.
  - [ ] `cargo test --features integration` exits 0.
  - [ ] `OPAL_RUN_STRESS=1 cargo test --features integration game_of_life_rc_return_stress -- --ignored --nocapture --test-threads=1` exits 0 within 130 seconds.
  - [ ] `bash scripts/array_memory_sanitizer.sh` exits 0 if present and executable.
  - [ ] `git diff -- test-projects/game-of-life-full/src/main.op` shows no source workaround.
  - [ ] `git diff -- runtime/opal_rc.c runtime/opal_rc.h src/codegen/control_flow.rs` shows no semantic changes.
  - [ ] `git diff` shows no return-ownership metadata/provenance redesign.
  - [ ] `git log --oneline -10` shows tests-first atomic commit ordering.

  **QA Scenarios** (MANDATORY - task incomplete without these):
  ```
  Scenario: Full suite green
    Tool: Bash
    Steps: Run `cargo test` then `cargo test --features integration`.
    Expected: Both commands exit 0 with zero failures.
    Evidence: .sisyphus/evidence/task-6-full-suite-green.txt

  Scenario: Atomicity and narrow-scope guardrails preserved
    Tool: Bash
    Steps: Run `git status --short`, `git diff -- test-projects/game-of-life-full/src/main.op`, `git diff -- runtime/opal_rc.c runtime/opal_rc.h src/codegen/control_flow.rs`, `git diff`, and `git log --oneline -10`.
    Expected: Tests-first atomic commits; no source workaround; no runtime/return-lowering changes; no metadata/provenance redesign.
    Evidence: .sisyphus/evidence/task-6-atomicity-guardrails.txt
  ```

  **Commit**: YES | Message: `chore(verify): record reassignment leak verification` | Files: [`.sisyphus/evidence/task-6-*.txt`]

## Final Verification Wave (MANDATORY — after ALL implementation tasks)
> 4 review agents run in PARALLEL. ALL must APPROVE. Present consolidated results to user and get explicit "okay" before completing.
> **Do NOT auto-proceed after verification. Wait for user's explicit approval before marking work complete.**
> **Never mark F1-F4 as checked before getting user's okay.** Rejection or user feedback -> fix -> re-run -> present again -> wait for okay.

- [x] F1. Plan Compliance Audit — oracle

  **Tool/Agent**: `task(subagent_type="oracle", run_in_background=true, load_skills=[], prompt="Audit implementation against .sisyphus/plans/rc-return-assignment-memory-leak.md. Verify tests-first ordering, no Game of Life source workaround, no runtime semantic changes, no return-ownership metadata/provenance redesign, and all acceptance criteria evidence exists. Return APPROVE or REJECT with exact blockers.")`
  **Expected**: Oracle returns `APPROVE`; any `REJECT` must be fixed before completion.
  **Evidence**: `.sisyphus/evidence/final-f1-plan-compliance.md`

- [x] F2. Code Quality Review — unspecified-high

  **Tool/Agent**: `task(category="unspecified-high", run_in_background=true, load_skills=[], prompt="Review the final diff for code quality and maintainability. Confirm the production fix is surgical in src/codegen/statements.rs, existing array literal/reserve behavior is preserved, no broad refactor or unrelated formatting occurred, and comments accurately explain let/assignment ownership alignment. Return APPROVE or REJECT with exact blockers.")`
  **Expected**: Reviewer returns `APPROVE`; any `REJECT` must be fixed before completion.
  **Evidence**: `.sisyphus/evidence/final-f2-code-quality.md`

- [x] F3. Real Manual QA — unspecified-high

  **Tool/Agent**: `task(category="unspecified-high", run_in_background=true, load_skills=[], prompt="Execute real QA for the reassignment leak fix. Run cargo test, cargo test --features integration, OPAL_RUN_STRESS=1 cargo test --features integration game_of_life_rc_return_stress -- --ignored --nocapture --test-threads=1, and bash scripts/array_memory_sanitizer.sh if present/executable. Verify outputs match plan acceptance criteria. Return APPROVE or REJECT with command outputs and exact blockers.")`
  **Expected**: Reviewer returns `APPROVE` with passing command evidence; any failure must be fixed before completion.
  **Evidence**: `.sisyphus/evidence/final-f3-real-qa.md`

- [x] F4. Scope Fidelity Check — deep

  **Tool/Agent**: `task(category="deep", run_in_background=true, load_skills=[], prompt="Check scope fidelity for the reassignment leak fix. Confirm only the intended narrow compiler behavior changed, alias-return limitation is documented but not claimed fixed, no source workaround was used, no runtime/return-lowering semantics changed, and atomic commits match the plan. Return APPROVE or REJECT with exact blockers.")`
  **Expected**: Reviewer returns `APPROVE`; any `REJECT` must be fixed before completion.
  **Evidence**: `.sisyphus/evidence/final-f4-scope-fidelity.md`

## Commit Strategy
1. `test(rc): add reassignment return leak regression` — Task 1 focused regression and ignored limitation characterization; expected focused positive test fails before fix.
2. `test(rc): add game of life reassignment stress` — Task 2 ignored 2-minute stress test; expected opt-in stress fails before fix.
3. `test(rc): document reassignment ownership audit` — Task 3 evidence only, or leave evidence unstaged if repo convention excludes evidence commits.
4. `fix(codegen): take ownership of rc call assignment results` — Task 4 surgical production fix in `src/codegen/statements.rs`.
5. `test(rc): verify reassignment stress workflow` — Task 5 if sanitizer script or stress wiring changed.
6. `chore(verify): record reassignment leak verification` — Task 6 evidence only if repo convention allows evidence commits; otherwise leave evidence unstaged and report paths.

## Success Criteria
- Mutable assignment from fresh RC-returning user function calls no longer leaks one object per iteration.
- Existing let initialization behavior remains unchanged.
- Existing array literal and `reserve(...)` assignment ownership behavior remains unchanged.
- Alias-return unsoundness is documented as a pre-existing known limitation, not silently treated as solved.
- Stress test catches the pre-fix leak and passes after the fix.
- Runtime RC implementation and return lowering remain unchanged.
- Atomic commits are small, reviewable, and ordered tests-first.
