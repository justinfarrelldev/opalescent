# Fix RC Scope-Exit Memory Leaks

## TL;DR
> **Summary**: Add lexical scope cleanup to codegen so RC-bearing locals are decremented on normal block exit and structured exits, then fix string elements in arrays so dropped arrays release child strings.
> **Deliverables**:
> - Runtime-counter integration tests proving block, loop, return, break, continue, if/else, and string-array cleanup.
> - Scope tracking in `CodegenEnv` with explicit cleanup methods and loop-depth metadata.
> - Shared binding-release helper and string-array RC classification fix.
> - Three independent final verifiers that each state exactly `PASS` for leak-free Game of Life implemented entirely in Opalescent.
> **Effort**: Large
> **Parallel**: YES - 4 waves
> **Critical Path**: Task 1 → Task 2 → Tasks 3-6 → Task 7 → Task 8 → F1-F4 → 3-verifier Game of Life validation

## Context
### Original Request
Execute the compacted plan for fixing memory leaks in scope-exit RC cleanup.

### Interview Summary
User supplied a compacted prior investigation with root cause and implementation direction. No user-preference questions remain; unresolved technical edge cases from review are resolved below as defaults and guardrails.

### Metis Review (gaps addressed)
- Ownership transfer is explicit: returned values and break payloads must not be decremented before transfer; all other live locals must be decremented.
- All known scope-introducing constructs are classified: blocks, if/else branches, while/loop bodies, for iteration bindings, return, break, continue are in scope; panic/non-local unwind safety and match-arm local bindings are out of scope unless already represented by `Stmt::Block`.
- Tests must use runtime counters as primary evidence, not LLVM IR string matching.
- Refactor must happen after green behavior, not before.
- Binding cleanup must be live-binding aware enough to avoid double decrement for values explicitly transferred by return/break payloads.

## Work Objectives
### Core Objective
Ensure every RC-bearing local binding introduced by codegen is released exactly once when its lexical scope exits through normal completion, return, break, or continue, while preserving ownership of transferred return/break payload values.

### Deliverables
- `tests/integration_e2e/scope_leak_counters.rs`
- `mod scope_leak_counters;` in `tests/integration_e2e/tests.rs`
- Scope tracking extensions in `src/codegen/expressions.rs` or `src/codegen/scope_tracker.rs` if line count requires a split.
- Cleanup wiring in `src/codegen/statements.rs` and `src/codegen/control_flow.rs`.
- Binding-release helper in `src/codegen/binding_store.rs`.
- `CoreType::String` array-element RC fix in `src/codegen/expressions_array.rs` with updated unit test.

### Definition of Done (verifiable conditions with commands)
- `cargo test --features integration --test integration_e2e scope_leak` passes.
- `cargo test` passes.
- `bash scripts/check-line-count.sh` passes.
- `cargo clippy --all-targets --all-features -- -D warnings -D clippy::all -D clippy::pedantic -D clippy::nursery -A clippy::cargo` passes.

### Must Have
- Red tests must be introduced before behavior changes and fail with current leak behavior.
- Cleanup must be emitted before `build_return`, before break branch, before continue branch, and before normal block/loop scope exit branches.
- For-loop iteration variable cleanup must emit RC dec for the iteration binding without corrupting existing previous-binding restore logic.
- Return/break payload ownership transfer must be preserved.
- Runtime counters must assert exact `live=0` for strings and arrays unless a test intentionally validates transferred ownership.
- Final Game of Life validation must use an implementation created entirely with Opalescent, with no hardcoded Game of Life outputs/shortcuts and no memory cap.
- Any test or lint failure is in scope and must be fixed because baseline tests and lint currently pass; treat such failures as regressions introduced by this initiative.

### Must NOT Have
- No runtime C ownership redesign.
- No panic/unwind safety work.
- No LLVM IR string matching as a required pass/fail gate.
- No broad memory-model redesign or user-defined type drop semantics beyond current RC-bearing classification.
- No source changes outside listed implementation/test files unless required by line-count split or module registration.
- No hardcoding Game of Life behavior, expected frames, leak outcomes, or counter results to satisfy final validation.
- No memory cap as a substitute for correct ownership cleanup.

## Verification Strategy
> ZERO HUMAN INTERVENTION - all verification is agent-executed.
- Test decision: TDD, using integration tests with `integration` feature and runtime counter assertions.
- QA policy: Every task has agent-executed scenarios.
- Evidence: `.sisyphus/evidence/task-{N}-{slug}.{ext}`.

## Execution Strategy
### Parallel Execution Waves
> Target: 5-8 tasks per wave. <3 per wave (except final) = under-splitting.
> Extract shared dependencies as Wave-1 tasks for max parallelism.

Wave 1: Task 1 tests and Task 2 scope infrastructure.
Wave 2: Tasks 3-6 cleanup wiring and string-array fix.
Wave 3: Task 7 refactor and line-count split.
Wave 4: Task 8 full verification. After F1-F4 approval, run the mandatory 3-verifier Game of Life leak validation.

### Dependency Matrix (full, all tasks)
- Task 1: no blockers; blocks final proof in Tasks 3-8.
- Task 2: no blockers; blocks Tasks 3-6.
- Task 3: blocked by Tasks 1-2; blocks Task 7.
- Task 4: blocked by Tasks 1-2; blocks Task 7.
- Task 5: blocked by Tasks 1-2; blocks Task 7.
- Task 6: blocked by Task 1; independent of scope infrastructure except final integration.
- Task 7: blocked by Tasks 3-6.
- Task 8: blocked by Tasks 1-7.
- Mandatory Game of Life validation: blocked by Task 8 and F1-F4; blocks completion.

### Agent Dispatch Summary (wave → task count → categories)
- Wave 1 → 2 tasks → unspecified-high, deep.
- Wave 2 → 4 tasks → deep, unspecified-high.
- Wave 3 → 1 task → deep.
- Wave 4 → 1 task → unspecified-high; post-F1-F4 mandatory validation → 3 independent verifier agents.

## TODOs
> Implementation + Test = ONE task. Never separate.
> EVERY task MUST have: Agent Profile + Parallelization + QA Scenarios.

- [x] 1. Add red integration tests for scope leak counters

  **What to do**: Create `tests/integration_e2e/scope_leak_counters.rs` and register it in `tests/integration_e2e/tests.rs` near `mod memory_model_counters;`. Follow the structure of `tests/integration_e2e/memory_model_counters.rs:25-166` and `tests/integration_e2e/tests.rs:23-76`, but do **not** rely on ordinary `compile_program_for_tests` binaries printing counters because compiled Opalescent programs do not currently expose `counter:*` lines. Instead, add a concrete test-only counter harness path in the integration test: compile Opalescent source to an object using existing lower-level test-accessible compiler helpers (`compile_to_module`, `emit_object_file`, `link_object_file_for_tests`) and link it with a small C harness compiled with `-DOPAL_ENABLE_INTERNAL_TESTING` that registers `atexit(report_counters)`, calls `opal_rc_debug_reset_counters_for_test()`, invokes the generated Opalescent entry point, and prints the same `counter:strings alloc=... free=... live=...` / `counter:arrays ...` lines as `tests/integration_e2e/fixtures/memory_model_counters.c:40-60,255-270`. Include tests named exactly: `scope_leak_block_exit`, `scope_leak_for_iter_var`, `scope_leak_early_return`, `scope_leak_break`, `scope_leak_continue`, `scope_leak_string_array_drop`, and `scope_leak_nested_if_else`.

  **Must NOT do**: Do not modify production compiler/runtime code in this task. Do not assume `src/compiler.rs:55-76` or `src/compiler.rs:586-621` already enables `OPAL_ENABLE_INTERNAL_TESTING`; it does not. Do not use LLVM IR text matching as the red criterion. Do not add placeholder tests that only assert compilation.

  **Recommended Agent Profile**:
  - Category: `unspecified-high` - Reason: test harness integration requires careful repo-specific patterns.
  - Skills: [] - No special skill required.
  - Omitted: [`playwright`] - No browser/UI work.

  **Parallelization**: Can Parallel: YES | Wave 1 | Blocks: [3, 4, 5, 6, 8] | Blocked By: []

  **References** (executor has NO interview context - be exhaustive):
  - Pattern: `tests/integration_e2e/memory_model_counters.rs:25-166` - command execution, counter line parsing, temp directory cleanup, and `live=0` assertions.
  - Pattern: `tests/integration_e2e/tests.rs:23-76` - module registration location.
  - Pattern: `tests/integration_e2e/fixtures/memory_model_counters.c:40-60` - exact counter report line format.
  - Pattern: `tests/integration_e2e/fixtures/memory_model_counters.c:255-270` - reset counters, register reporter, run scenario.
  - API/Type: `tests/integration_e2e.rs:41-79` - existing compile/link helper signatures available to child modules.
  - API/Type: `tests/integration_e2e.rs:97-104` - binary execution helper signature.
  - Existing fixtures: `test-projects/game-of-life/src/main.op` - source pattern using `string_join`, string builders, loops, and `propagate`.

  **Acceptance Criteria** (agent-executable only):
  - [ ] `cargo test --features integration --test integration_e2e scope_leak -- --nocapture` runs all seven new tests.
  - [ ] Before implementation tasks, at least `scope_leak_block_exit`, `scope_leak_for_iter_var`, `scope_leak_early_return`, `scope_leak_break`, `scope_leak_continue`, and `scope_leak_string_array_drop` fail because `live` counters are non-zero or expected output is missing.
  - [ ] Every test failure message includes the full stdout for diagnosis.
  - [ ] `bash scripts/check-line-count.sh` passes after adding tests.

  **QA Scenarios** (MANDATORY - task incomplete without these):
  ```
  Scenario: Red leak tests execute
    Tool: Bash
    Steps: set -o pipefail; cargo test --features integration --test integration_e2e scope_leak -- --nocapture 2>&1 | tee .sisyphus/evidence/task-1-scope-leak-red.txt
    Expected: Command exits non-zero before implementation and output lists the new scope_leak_* tests as failed or exposes non-zero live counters.
    Evidence: .sisyphus/evidence/task-1-scope-leak-red.txt

  Scenario: Test module registration is correct
    Tool: Bash
    Steps: set -o pipefail; cargo test --features integration --test integration_e2e scope_leak_string_array_drop -- --list 2>&1 | tee .sisyphus/evidence/task-1-scope-leak-list.txt
    Expected: Output includes `scope_leak_string_array_drop` exactly once.
    Evidence: .sisyphus/evidence/task-1-scope-leak-list.txt
  ```

  **Commit**: NO | Message: `test(codegen): cover rc scope leaks` | Files: [`tests/integration_e2e/scope_leak_counters.rs`, `tests/integration_e2e/tests.rs`]

- [x] 2. Add scope tracking infrastructure to `CodegenEnv`

  **What to do**: Extend `src/codegen/expressions.rs` so `LoopContext` has `scope_depth: usize` and `CodegenEnv` has a documented `scope_stack` field. Add methods with these exact responsibilities: `enter_scope() -> usize`, `current_scope_depth() -> usize`, `register_scope_binding(&mut self, name: &str)`, `release_scope_binding_value(&mut self, codegen_context, name, transferred_names)`, `exit_scope_cleanup(codegen_context, transferred_names)`, `cleanup_all_scopes_for_return(codegen_context, transferred_names)`, and `cleanup_scopes_to_depth(codegen_context, target_depth, transferred_names)`. The cleanup methods must emit decrements for RC-bearing live bindings, remove cleaned names from `env.variables`, and skip names present in `transferred_names` so return/break payloads are not decremented before transfer.

  **Must NOT do**: Do not wire behavior into statements or control flow in this task except compile-driven minimal imports. Do not make `ScopeExitMode` public; prefer helper parameters and transferred-name sets. Do not duplicate heap classification logic if `binding_store.rs` can expose a helper.

  **Recommended Agent Profile**:
  - Category: `deep` - Reason: central ownership state and lifetime semantics.
  - Skills: [] - No special skill required.
  - Omitted: [`frontend-ui-ux`] - No UI work.

  **Parallelization**: Can Parallel: YES | Wave 1 | Blocks: [3, 4, 5, 7] | Blocked By: []

  **References** (executor has NO interview context - be exhaustive):
  - Pattern: `src/codegen/expressions.rs:52-72` - existing `LoopContext` and `CodegenEnv` fields.
  - Pattern: `src/codegen/expressions.rs:74-96` - `CodegenEnv::new` and method style.
  - API/Type: `src/codegen/binding_store.rs:90-132` - current heap classification and private RC inc/dec helpers.
  - API/Type: `src/codegen/rc_emitter.rs` - `RcEmitter::emit_dec` behavior.

  **Acceptance Criteria** (agent-executable only):
  - [ ] `cargo check` passes after infrastructure addition.
  - [ ] `LoopContext` construction sites fail no compilation; each construction receives `scope_depth: env.current_scope_depth()` or the intended loop-entry depth.
  - [ ] The `scope_stack` field has a doc comment stating every let/destructure/guard binding inserted into `env.variables` must be registered or explicitly documented as function/global/non-owned.
  - [ ] Cleanup methods accept a transferred-name set or slice and skip those names.

  **QA Scenarios** (MANDATORY - task incomplete without these):
  ```
  Scenario: Scope infrastructure compiles
    Tool: Bash
    Steps: set -o pipefail; cargo check 2>&1 | tee .sisyphus/evidence/task-2-scope-infra-check.txt
    Expected: Exit code 0.
    Evidence: .sisyphus/evidence/task-2-scope-infra-check.txt

  Scenario: No premature behavior change claimed
    Tool: Bash
    Steps: set -o pipefail; cargo test --features integration --test integration_e2e scope_leak_block_exit -- --nocapture 2>&1 | tee .sisyphus/evidence/task-2-still-red.txt
    Expected: Test remains red unless later wiring tasks have already run; if already green due to parallel execution, evidence must state which later task made it green.
    Evidence: .sisyphus/evidence/task-2-still-red.txt
  ```

  **Commit**: NO | Message: `refactor(codegen): track lexical rc scopes` | Files: [`src/codegen/expressions.rs`, optionally `src/codegen/scope_tracker.rs`, module declaration file if new module is needed]

- [x] 3. Wire block, let, destructure, guard, and if/else scope cleanup

  **What to do**: In `src/codegen/statements.rs`, wrap `Stmt::Block` lowering with `env.enter_scope()` and `env.exit_scope_cleanup(codegen_context, &[])` when the current block has no terminator. After every successful `env.variables.insert` for a lexical binding, call `env.register_scope_binding(...)`: normal let at `src/codegen/statements.rs:173-182`, destructuring let at `src/codegen/statements.rs:244-253`, guard error binding at `src/codegen/statements.rs:499-508`, and guard success binding at `src/codegen/statements.rs:540-549`. In `src/codegen/control_flow.rs`, ensure `codegen_if_statement` and `codegen_if_expression` clean each branch-local scope before branching to merge when not terminated. Branch cleanup must not remove bindings from the opposite branch.

  **Must NOT do**: Do not decrement values after a terminator already exists. Do not clean branch scopes after positioning at merge. Do not make `if` branch locals visible after merge unless they were already intentionally inserted by existing code.

  **Recommended Agent Profile**:
  - Category: `deep` - Reason: branch-local ownership cleanup is easy to over-release or under-release.
  - Skills: [] - No special skill required.
  - Omitted: [`playwright`] - No UI/browser work.

  **Parallelization**: Can Parallel: YES | Wave 2 | Blocks: [7, 8] | Blocked By: [1, 2]

  **References**:
  - Pattern: `src/codegen/statements.rs:103-108` - current block statement lowering with no cleanup.
  - Pattern: `src/codegen/statements.rs:173-182` - normal let binding insert.
  - Pattern: `src/codegen/statements.rs:244-253` - destructuring let binding insert.
  - Pattern: `src/codegen/statements.rs:390-576` - guard statement bindings.
  - Pattern: `src/codegen/control_flow.rs:29-147` - if statement/expression branch and merge blocks.

  **Acceptance Criteria**:
  - [ ] `cargo test --features integration --test integration_e2e scope_leak_block_exit -- --nocapture` passes.
  - [ ] `cargo test --features integration --test integration_e2e scope_leak_nested_if_else -- --nocapture` passes.
  - [ ] Branch cleanup occurs before `build_unconditional_branch(merge_block)` in non-terminated branch blocks.
  - [ ] Existing guard tests still pass: `cargo test --features integration --test integration_e2e guard`.

  **QA Scenarios**:
  ```
  Scenario: Block and branch leaks are fixed
    Tool: Bash
    Steps: set -o pipefail; cargo test --features integration --test integration_e2e scope_leak_block_exit -- --nocapture 2>&1 | tee .sisyphus/evidence/task-3-block-cleanup.txt && cargo test --features integration --test integration_e2e scope_leak_nested_if_else -- --nocapture 2>&1 | tee .sisyphus/evidence/task-3-if-cleanup.txt
    Expected: Exit code 0 and counter lines show live=0 for exercised RC categories.
    Evidence: .sisyphus/evidence/task-3-block-cleanup.txt, .sisyphus/evidence/task-3-if-cleanup.txt

  Scenario: Guard regressions absent
    Tool: Bash
    Steps: set -o pipefail; cargo test --features integration --test integration_e2e guard -- --nocapture 2>&1 | tee .sisyphus/evidence/task-3-guard-regression.txt
    Expected: Exit code 0.
    Evidence: .sisyphus/evidence/task-3-guard-regression.txt
  ```

  **Commit**: NO | Message: `fix(codegen): release rc bindings on block exit` | Files: [`src/codegen/statements.rs`, `src/codegen/control_flow.rs`]

- [x] 4. Wire loop, for-iteration, break, and continue cleanup

  **What to do**: In `src/codegen/control_flow.rs`, add `scope_depth: env.current_scope_depth()` to every `LoopContext` construction in `emit_loop_body_with_targets`. For `while` and `loop`, ensure body scopes are cleaned before back-edge branches when the current block is unterminated. For `for`, create an iteration scope around the per-iteration variable at `src/codegen/control_flow.rs:349-365`, register it, and call a dec-only/skip-restore-safe cleanup before existing previous-binding restore at `src/codegen/control_flow.rs:377-381`; preserve previous binding restoration exactly. In `src/codegen/statements.rs`, update `codegen_break_statement` and `codegen_continue_statement` so cleanup to `loop_context.scope_depth` happens before building the branch. For break payloads, lower/store break values first, mark any source identifiers transferred, then cleanup all non-transferred nested bindings.

  **Must NOT do**: Do not remove or overwrite a shadowed previous binding before existing restore logic runs. Do not decrement break payload values before storing them into break slots. Do not ignore `Stmt::Continue { values }`; if continue payloads remain unsupported, return an explicit codegen error or document existing semantics, but do not leak scopes.

  **Recommended Agent Profile**:
  - Category: `deep` - Reason: loop control-flow exits interact with ownership transfer and shadowing.
  - Skills: [] - No special skill required.
  - Omitted: [`frontend-ui-ux`] - No UI work.

  **Parallelization**: Can Parallel: YES | Wave 2 | Blocks: [7, 8] | Blocked By: [1, 2]

  **References**:
  - Pattern: `src/codegen/control_flow.rs:149-408` - while/loop/for lowering.
  - Pattern: `src/codegen/control_flow.rs:349-381` - for iteration binding insert and previous-binding restore.
  - Pattern: `src/codegen/control_flow.rs:708-738` - loop context push/pop.
  - Pattern: `src/codegen/statements.rs:266-309` - break and continue lowering.
  - API/Type: `src/ast.rs:702-720` - break and continue payload fields.

  **Acceptance Criteria**:
  - [ ] `cargo test --features integration --test integration_e2e scope_leak_for_iter_var -- --nocapture` passes.
  - [ ] `cargo test --features integration --test integration_e2e scope_leak_break -- --nocapture` passes.
  - [ ] `cargo test --features integration --test integration_e2e scope_leak_continue -- --nocapture` passes.
  - [ ] `cargo test --features integration --test integration_e2e game_of_life_ten_frames -- --nocapture` passes.

  **QA Scenarios**:
  ```
  Scenario: Loop exit leaks fixed
    Tool: Bash
    Steps: set -o pipefail; cargo test --features integration --test integration_e2e scope_leak_for_iter_var -- --nocapture 2>&1 | tee .sisyphus/evidence/task-4-for-iter-var.txt && cargo test --features integration --test integration_e2e scope_leak_break -- --nocapture 2>&1 | tee .sisyphus/evidence/task-4-break.txt && cargo test --features integration --test integration_e2e scope_leak_continue -- --nocapture 2>&1 | tee .sisyphus/evidence/task-4-continue.txt
    Expected: Exit code 0 and counter lines show live=0 for loop-exit cases.
    Evidence: .sisyphus/evidence/task-4-for-iter-var.txt, .sisyphus/evidence/task-4-break.txt, .sisyphus/evidence/task-4-continue.txt

  Scenario: Existing Game of Life still runs
    Tool: Bash
    Steps: set -o pipefail; cargo test --features integration --test integration_e2e game_of_life_ten_frames -- --nocapture 2>&1 | tee .sisyphus/evidence/task-4-game-of-life-regression.txt
    Expected: Exit code 0 with no changed expected frame behavior.
    Evidence: .sisyphus/evidence/task-4-game-of-life-regression.txt
  ```

  **Commit**: NO | Message: `fix(codegen): release rc bindings on loop exits` | Files: [`src/codegen/control_flow.rs`, `src/codegen/statements.rs`]

- [x] 5. Wire return and guard-propagation cleanup with ownership transfer

  **What to do**: In `src/codegen/control_flow.rs:463-588`, lower return payload values first, identify transferred source binding names for simple identifier returns and break-compatible payload patterns, call `env.cleanup_all_scopes_for_return(codegen_context, transferred_names)` before every `build_return`, then build the return using already-lowered values. Apply the same cleanup-before-return rule to error-aware returns and void returns. In `src/codegen/statements.rs:580-627`, add cleanup before guard error propagation `build_return`. Transfer rule: the value being returned remains owned by the caller; every other live RC local is decremented.

  **Must NOT do**: Do not cleanup before lowering return expressions, because expressions may depend on live locals. Do not decrement a simple returned identifier before returning it. Do not skip cleanup for void or error returns.

  **Recommended Agent Profile**:
  - Category: `deep` - Reason: return ownership transfer is a critical double-free/leak boundary.
  - Skills: [] - No special skill required.
  - Omitted: [`playwright`] - No UI/browser work.

  **Parallelization**: Can Parallel: YES | Wave 2 | Blocks: [7, 8] | Blocked By: [1, 2]

  **References**:
  - Pattern: `src/codegen/control_flow.rs:463-588` - normal and error-aware return lowering.
  - Pattern: `src/codegen/statements.rs:580-627` - guard error propagation return.
  - API/Type: `src/ast.rs:598-609` - return payload structure.
  - Metis guardrail: returned value must not be decremented; all other live bindings must be decremented.

  **Acceptance Criteria**:
  - [ ] `cargo test --features integration --test integration_e2e scope_leak_early_return -- --nocapture` passes.
  - [ ] A transfer-specific test confirms a returned RC value is usable by caller and not double-freed.
  - [ ] `cargo test --features integration --test integration_e2e guard -- --nocapture` passes.

  **QA Scenarios**:
  ```
  Scenario: Early return cleanup is leak-free
    Tool: Bash
    Steps: set -o pipefail; cargo test --features integration --test integration_e2e scope_leak_early_return -- --nocapture 2>&1 | tee .sisyphus/evidence/task-5-return-cleanup.txt
    Expected: Exit code 0 and live counters return to zero after early return.
    Evidence: .sisyphus/evidence/task-5-return-cleanup.txt

  Scenario: Return transfer remains valid
    Tool: Bash
    Steps: set -o pipefail; cargo test --features integration --test integration_e2e scope_leak_return_transfer -- --nocapture 2>&1 | tee .sisyphus/evidence/task-5-return-transfer.txt
    Expected: Exit code 0; caller observes returned value correctly and final live counters are zero after caller scope exits.
    Evidence: .sisyphus/evidence/task-5-return-transfer.txt
  ```

  **Commit**: NO | Message: `fix(codegen): release rc bindings before return` | Files: [`src/codegen/control_flow.rs`, `src/codegen/statements.rs`, `tests/integration_e2e/scope_leak_counters.rs`]

- [x] 6. Fix string elements in array RC classification

  **What to do**: In `src/codegen/expressions_array.rs:1295-1301`, change `is_rc_bearing_element_type` so `HeapClass::ReferenceCounted` returns true for `CoreType::String` and `CoreType::Array(_)`, while preserving generic and caller-owned behavior. Update the unit test at `src/codegen/expressions_array.rs:1303-1321` so it asserts `is_rc_bearing_element_type(&CoreType::String)` is true. Ensure the integration test `scope_leak_string_array_drop` proves array drop releases child strings.

  **Must NOT do**: Do not change `classify_core_type(&CoreType::String)`; it already returns `HeapClass::ReferenceCounted`. Do not special-case Game of Life or string_join.

  **Recommended Agent Profile**:
  - Category: `unspecified-high` - Reason: narrow but memory-sensitive behavior change.
  - Skills: [] - No special skill required.
  - Omitted: [`playwright`] - No UI work.

  **Parallelization**: Can Parallel: YES | Wave 2 | Blocks: [7, 8] | Blocked By: [1]

  **References**:
  - Pattern: `src/codegen/expressions_array.rs:1252-1284` - per-element retain/release helpers.
  - Bug: `src/codegen/expressions_array.rs:1295-1301` - current false for string elements.
  - Test: `src/codegen/expressions_array.rs:1303-1321` - update expected string semantics.

  **Acceptance Criteria**:
  - [ ] `cargo test expressions_array::tests::heap_class_array_children_preserves_string_representation_semantics` passes with string element RC expected true.
  - [ ] `cargo test --features integration --test integration_e2e scope_leak_string_array_drop -- --nocapture` passes.
  - [ ] No Game of Life-specific or string_join-specific logic is added.

  **QA Scenarios**:
  ```
  Scenario: String array child releases are fixed
    Tool: Bash
    Steps: set -o pipefail; cargo test --features integration --test integration_e2e scope_leak_string_array_drop -- --nocapture 2>&1 | tee .sisyphus/evidence/task-6-string-array-drop.txt
    Expected: Exit code 0 and string/array live counters are zero.
    Evidence: .sisyphus/evidence/task-6-string-array-drop.txt

  Scenario: Unit classification updated
    Tool: Bash
    Steps: set -o pipefail; cargo test heap_class_array_children_preserves_string_representation_semantics 2>&1 | tee .sisyphus/evidence/task-6-array-classification-unit.txt
    Expected: Exit code 0.
    Evidence: .sisyphus/evidence/task-6-array-classification-unit.txt
  ```

  **Commit**: NO | Message: `fix(codegen): release string array elements` | Files: [`src/codegen/expressions_array.rs`, `tests/integration_e2e/scope_leak_counters.rs`]

- [x] 7. Refactor binding decrement helper and split files if line count requires

  **What to do**: After Tasks 3-6 are green, expose a single helper in `src/codegen/binding_store.rs` such as `emit_dec_for_binding(codegen_context, binding_name, binding)` or `release_binding_value_if_needed(codegen_context, core_type, value)`. Update scope cleanup code to use it instead of duplicating RC classification and `RcEmitter` setup. Run `bash scripts/check-line-count.sh`; if `src/codegen/expressions.rs` or `src/codegen/control_flow.rs` exceeds limits, move scope methods into `src/codegen/scope_tracker.rs` as an `impl CodegenEnv` block and register the module consistently with existing `src/codegen` module layout.

  **Must NOT do**: Do not refactor before behavioral tests are green. Do not alter RC semantics while extracting helper. Do not silence clippy with broad new allowances unless a pre-existing module-level policy already applies.

  **Recommended Agent Profile**:
  - Category: `deep` - Reason: post-green consolidation must preserve memory semantics exactly.
  - Skills: [] - No special skill required.
  - Omitted: [`frontend-ui-ux`] - No UI work.

  **Parallelization**: Can Parallel: NO | Wave 3 | Blocks: [8, 9] | Blocked By: [3, 4, 5, 6]

  **References**:
  - Pattern: `src/codegen/binding_store.rs:18-132` - current binding store and private RC helpers.
  - Pattern: `src/codegen/expressions.rs:60-96` - current env impl location.
  - Command: `bash scripts/check-line-count.sh` - file size guardrail.

  **Acceptance Criteria**:
  - [ ] `cargo test --features integration --test integration_e2e scope_leak -- --nocapture` passes after refactor.
  - [ ] `bash scripts/check-line-count.sh` passes.
  - [ ] No duplicate direct `RcEmitter::new(...).emit_dec(...)` logic exists outside helper except legitimate array-element release paths.

  **QA Scenarios**:
  ```
  Scenario: Refactor preserves leak fixes
    Tool: Bash
    Steps: set -o pipefail; cargo test --features integration --test integration_e2e scope_leak -- --nocapture 2>&1 | tee .sisyphus/evidence/task-7-refactor-scope-leak.txt
    Expected: Exit code 0.
    Evidence: .sisyphus/evidence/task-7-refactor-scope-leak.txt

  Scenario: Line count remains valid
    Tool: Bash
    Steps: set -o pipefail; bash scripts/check-line-count.sh 2>&1 | tee .sisyphus/evidence/task-7-line-count.txt
    Expected: Exit code 0.
    Evidence: .sisyphus/evidence/task-7-line-count.txt
  ```

  **Commit**: NO | Message: `refactor(codegen): centralize rc binding release` | Files: [`src/codegen/binding_store.rs`, `src/codegen/expressions.rs`, optionally `src/codegen/scope_tracker.rs`]

- [x] 8. Run full regression, lint, and baseline-preserving verification

  **What to do**: Run the complete verification sequence. Because tests and linting currently pass at baseline, any failure is in scope and must be fixed, even if it appears outside the new leak tests. Run `cargo test`, `cargo test --features integration --test integration_e2e scope_leak -- --nocapture`, `cargo test --features integration --test integration_e2e game_of_life_ten_frames -- --nocapture`, `bash scripts/check-line-count.sh`, and strict clippy. Capture all output to `.sisyphus/evidence/`. If anything fails, fix the root cause and rerun the entire sequence.

  **Must NOT do**: Do not mark known failures as unrelated. Do not weaken lint flags. Do not add memory caps. Do not hardcode Game of Life behavior to satisfy tests.

  **Recommended Agent Profile**:
  - Category: `unspecified-high` - Reason: full-suite QA and regression fixing.
  - Skills: [] - No special skill required.
  - Omitted: [`playwright`] - No UI work.

  **Parallelization**: Can Parallel: NO | Wave 4 | Blocks: [9, F1, F2, F3, F4] | Blocked By: [7]

  **References**:
  - Command: `cargo test` - full Rust suite.
  - Command: `cargo test --features integration --test integration_e2e scope_leak -- --nocapture` - new leak suite.
  - Command: `cargo test --features integration --test integration_e2e game_of_life_ten_frames -- --nocapture` - existing non-trivial Opalescent program regression.
  - Command: `bash scripts/check-line-count.sh` - repo line-count gate.
  - Command: `cargo clippy --all-targets --all-features -- -D warnings -D clippy::all -D clippy::pedantic -D clippy::nursery -A clippy::cargo` - strict lint gate from original plan.

  **Acceptance Criteria**:
  - [ ] `cargo test` passes.
  - [ ] `cargo test --features integration --test integration_e2e scope_leak -- --nocapture` passes.
  - [ ] `cargo test --features integration --test integration_e2e game_of_life_ten_frames -- --nocapture` passes.
  - [ ] `bash scripts/check-line-count.sh` passes.
  - [ ] Strict clippy command passes.

  **QA Scenarios**:
  ```
  Scenario: Full tests and lint pass
    Tool: Bash
    Steps: set -o pipefail; cargo test 2>&1 | tee .sisyphus/evidence/task-8-cargo-test.txt && cargo test --features integration --test integration_e2e scope_leak -- --nocapture 2>&1 | tee .sisyphus/evidence/task-8-scope-leak.txt && cargo test --features integration --test integration_e2e game_of_life_ten_frames -- --nocapture 2>&1 | tee .sisyphus/evidence/task-8-game-of-life.txt && bash scripts/check-line-count.sh 2>&1 | tee .sisyphus/evidence/task-8-line-count.txt && cargo clippy --all-targets --all-features -- -D warnings -D clippy::all -D clippy::pedantic -D clippy::nursery -A clippy::cargo 2>&1 | tee .sisyphus/evidence/task-8-clippy.txt
    Expected: Exit code 0 for every command.
    Evidence: .sisyphus/evidence/task-8-cargo-test.txt, .sisyphus/evidence/task-8-scope-leak.txt, .sisyphus/evidence/task-8-game-of-life.txt, .sisyphus/evidence/task-8-line-count.txt, .sisyphus/evidence/task-8-clippy.txt

  Scenario: No hardcoding or memory caps introduced
    Tool: Bash
    Steps: set -o pipefail; git diff -- src runtime tests test-projects | tee .sisyphus/evidence/task-8-no-hardcoding-diff.txt
    Expected: Diff contains no Game of Life-specific shortcuts and no memory cap logic.
    Evidence: .sisyphus/evidence/task-8-no-hardcoding-diff.txt
  ```

  **Commit**: NO | Message: `fix(codegen): release rc locals on scope exit` | Files: [all changed implementation and test files]

## Final Verification Wave (MANDATORY — after ALL implementation tasks)
> 4 review agents run in PARALLEL. ALL must APPROVE. Present consolidated results to user and get explicit "okay" before completing.
> **Do NOT auto-proceed after verification. Wait for user's explicit approval before marking work complete.**
> **Never mark F1-F4 as checked before getting user's okay.** Rejection or user feedback -> fix -> re-run -> present again -> wait for okay.
> **Baseline assumption**: tests and linting currently completely pass. Any test failure, lint failure, or check-line-count failure encountered during this initiative is in scope and must be fixed as an introduced regression.
- [x] F1. Plan Compliance Audit — oracle
- [x] F2. Code Quality Review — unspecified-high
- [x] F3. Real Manual QA — unspecified-high
- [x] F4. Scope Fidelity Check — deep

## Commit Strategy
- Commit after all verification passes.
- Suggested message: `fix(codegen): release rc locals on scope exit`
- Single commit is acceptable because tests and implementation prove one root bug family; do not split string-array fix into a separate commit unless repository maintainers require commit-level isolation.

## Success Criteria
- New scope leak tests fail before implementation and pass after implementation.
- Full Rust and integration test suites pass.
- Strict clippy passes.
- Line-count script passes, with `src/codegen/scope_tracker.rs` split if needed.

## Mandatory Final Game of Life Leak Validation
After all implementation tasks, all tests, linting, line-count checks, and F1-F4 verification pass, run a separate final validation with **3 individual independent verifiers**. Each verifier must evaluate whether a Game of Life implementation created entirely with Opalescent would have no memory leaks under the completed ownership cleanup.

Hard requirements:
- Each of the 3 verifiers must state exactly `PASS` for this validation to pass.
- If any verifier rejects, reports uncertainty, reports an issue, or fails to state exactly `PASS`, the validation fails.
- On validation failure, fix every reported issue, rerun the full verification sequence, then rerun all 3 Game of Life leak verifiers.
- Do **not** hardcode anything for Game of Life: no hardcoded output frames, no hardcoded counter values, no special-case compiler/runtime behavior, no fixture-specific shortcuts.
- Do **not** add a memory cap or use a memory cap as a pass condition. Correctness must come from ownership cleanup and leak-free RC behavior.
- Evidence files must include each verifier transcript: `.sisyphus/evidence/game-of-life-verifier-1.txt`, `.sisyphus/evidence/game-of-life-verifier-2.txt`, `.sisyphus/evidence/game-of-life-verifier-3.txt`.

Verifier dispatch instructions:
- Launch 3 separate agents in parallel after F1-F4 approval.
- Verifier 1 profile: oracle. Prompt must ask only whether the completed implementation can run a Game of Life implementation created entirely with Opalescent without memory leaks, and must require final line exactly `PASS` or a concrete rejection.
- Verifier 2 profile: deep. Prompt must independently inspect implementation/evidence for leak paths in Game of Life-style loops, string joins, arrays, builders, returns, break/continue, and must require final line exactly `PASS` or concrete rejection.
- Verifier 3 profile: unspecified-high. Prompt must independently review test/lint evidence and diffs for hardcoding/memory caps and must require final line exactly `PASS` or concrete rejection.

Final validation QA scenarios:
```
Scenario: All three verifiers pass exactly
  Tool: Bash
  Steps: test "$(tail -n 1 .sisyphus/evidence/game-of-life-verifier-1.txt)" = "PASS" && test "$(tail -n 1 .sisyphus/evidence/game-of-life-verifier-2.txt)" = "PASS" && test "$(tail -n 1 .sisyphus/evidence/game-of-life-verifier-3.txt)" = "PASS"
  Expected: Exit code 0.
  Evidence: .sisyphus/evidence/game-of-life-verifier-1.txt, .sisyphus/evidence/game-of-life-verifier-2.txt, .sisyphus/evidence/game-of-life-verifier-3.txt

Scenario: No Game of Life hardcoding or memory cap
  Tool: Bash
  Steps: set -o pipefail; git diff -- src runtime tests test-projects | tee .sisyphus/evidence/final-no-hardcoding-diff.txt
  Expected: Diff contains no Game of Life-specific compiler/runtime shortcuts, no hardcoded expected frames/counter values used as implementation logic, and no memory cap logic.
  Evidence: .sisyphus/evidence/final-no-hardcoding-diff.txt
```
