# Strict Guard Error Handling

## TL;DR
> **Summary**: Enforce propagation-only guard error handling by making named guard `else err =>` clauses compile only when they end in an approved terminal propagation form, and replace current generic guard failures with structured Miette diagnostics. Execute with red-green-refactor, including project fixtures that first fail (`delete-downloads`, `delete-downloads-strict`) and then are fixed.
> **Deliverables**:
> - Strict type-checker validator for named guard error clauses.
> - Dedicated Miette `TypeError` diagnostics for invalid guard handling shapes.
> - Unit/integration tests plus multiple test-project fixtures for valid and invalid forms.
> - Fixed `delete-downloads` and `delete-downloads-strict` projects after failure confirmation.
> - Atomic commits and final clean `git status`.
> **Effort**: Large
> **Parallel**: YES - 4 waves plus final verification
> **Critical Path**: Task 1 → Task 2 → Task 4 → Task 5 → Task 7 → Task 10 → Final Verification

## Context
### Original Request
Implement strict guard error handling for items 1-3 only. Use TDD red-green-refactor, create multiple test-projects, confirm existing projects such as `delete-downloads` and `delete-downloads-strict` fail before fixing them, use atomic commits, finish with clean `git status`, research current guard compiler usage using GitHub Copilot-backed subagents only, and replace the current compilation failure with a beautiful Miette diagnostic.

### Interview Summary
No extra user interview was required after repository research. The plan applies these defaults:
- Shorthand `propagate <fallible_call>()` is the no-guard alternative, not an in-clause handler for a bound guard error.
- Inside `guard ... else err =>`, valid terminal forms are exactly final top-level `propagate err` and direct typed wrapper return with exact `source: err`.
- Non-terminal prelude statements are allowed before a valid terminal, but do not themselves count as handling.
- Strict rules apply only to named guard error clauses with an active error binding. Do not tighten non-error `else =>` clauses.

### Metis Review (gaps addressed)
Metis identified risks around shorthand scope, typed wrapper ABI/codegen, wrapper-shape precision, pre-terminal statements, fixture red-phase verifiability, diagnostic snapshot evidence, and avoiding parser/AST drift. This plan resolves them by locking the strict-rule spec in Task 1, gating wrapper-return codegen in Task 8, requiring diagnostic-code assertions and rendered diagnostic evidence, and keeping parser/AST unchanged unless implementation proves impossible.

## Work Objectives
### Core Objective
Make the compiler reject every named guard `else err =>` body that does not terminate in one of the approved propagation-only forms, and present those rejections as structured, source-labeled Miette diagnostics.

### Deliverables
- Strict guard error clause specification implemented in `src/type_system/checker/expressions_guard.rs`.
- New `TypeError` variants in `src/type_system/errors.rs` for invalid guard error handling.
- Updated tests in `src/type_system/tests.rs`, `tests/integration_e2e/guard_stmt.rs`, and related guard integration suites.
- New/updated `test-projects/*` fixtures covering valid terminal propagate, valid wrapper return, ignored alias failure, print-only failure, non-final propagate failure, and delete-downloads migrations.
- Updated existing `delete-downloads` and `delete-downloads-strict` sources to comply after red-phase confirmation.
- Atomic commits for fixture red tests, diagnostics, validator, wrapper support, fixture fixes, refactor, and final verification.

### Definition of Done (verifiable conditions with commands)
- `cargo build` succeeds.
- `cargo test` succeeds.
- `cargo test --features integration` succeeds.
- Focused guard tests pass: `cargo test --features integration guard`.
- `./target/release/opalescent run test-projects/delete-downloads/src/main.op` succeeds after fixes.
- `./target/release/opalescent run test-projects/delete-downloads-strict/src/main.op` succeeds after fixes and emits expected markers asserted by integration tests.
- `git status --porcelain` prints no output after the final commit.

### Must Have
- Strict named guard error clause validator with no broad “statement counted as handled” heuristic.
- Dedicated Miette diagnostics with stable codes, labels, and help text.
- TDD red phase that proves current `delete-downloads` and `delete-downloads-strict` behavior is invalid before the compiler/fix changes make tests green.
- Multiple project fixtures with compile-pass and compile-fail assertions via `compile_project(...)`.
- Wrapper return validation that requires direct literal return shape and exact source binding identity.
- GitHub Copilot-only subagent usage during execution review; no OpenRouter-backed delegation.

### Must NOT Have (guardrails, AI slop patterns, scope boundaries)
- Do not add a fourth valid guard error handling form.
- Do not treat logging, printing, assignment, `_ignored_*`, `return void`, `return ''`, `break`, `continue`, or nested successful cleanup as handling the outer guard error.
- Do not tighten unnamed/non-error guard `else =>` clauses.
- Do not change parser or AST guard shapes unless explicitly justified by failing implementation evidence.
- Do not rely on generic `ConstraintSolvingFailed.reason` strings for the new guard diagnostics.
- Do not leave red-phase-only failing tests in the final state.
- Do not finish with uncommitted or staged changes.

## Verification Strategy
> ZERO HUMAN INTERVENTION - all verification is agent-executed.
- Test decision: TDD RED-GREEN-REFACTOR using Rust unit tests, integration tests, compile-fail project fixtures, and CLI project runs.
- QA policy: Every task has agent-executed scenarios.
- Evidence: `.sisyphus/evidence/task-{N}-{slug}.{ext}`

## Strict Rule Specification
1. Shorthand fallible-call propagation (`propagate some_call(...)`) remains valid where `Expr::Propagate` is already legal and must be recommended when a guard clause would only rethrow the bound error.
2. In a named guard error clause (`guard call() into value else err =>` or `guard call() else err =>`), the body must be `Prelude* + Terminal`, where:
   - `Prelude*` may contain side effects, local bindings, logging/printing, cleanup attempts, or nested guards, but none of these count as handling the active outer `err`.
   - `Terminal` must be the final top-level statement.
   - Valid terminal A: `propagate err`, represented by `Stmt::PropagateGuardError`, matching the active guard error binding identity and compatible with the function declared errors.
   - Valid terminal B: direct typed wrapper return, e.g. `return new ConfigLoadError.ReadingConfig: source: err ...`, whose returned error type/variant belongs to the current function's declared errors and whose `source` field is exactly the active guard error binding, not an alias or shadowed binding.
3. Invalid examples must be rejected: print/log-only body, `_ignored_*` assignment, `return err`, non-final `propagate err`, terminal return without `source: err`, wrapper return with aliased/shadowed source, fallback success return, `break`, `continue`, `return void`, and `propagate <call>()` inside the named guard error clause as a substitute for handling the active `err`.

## Execution Strategy
### Parallel Execution Waves
> Target: 5-8 tasks per wave. <3 per wave (except final) = under-splitting.
> Extract shared dependencies as Wave-1 tasks for max parallelism.

Wave 1: Tasks 1-3 (spec/test inventory, red-phase fixtures, diagnostic variant scaffolding)
Wave 2: Tasks 4-6 (structured diagnostics, strict validator, migrated assertions)
Wave 3: Tasks 7-9 (project fixture fixes, wrapper/codegen support if needed, refactor)
Wave 4: Tasks 10-11 (atomic commits/final clean verification, documentation of evidence)

### Dependency Matrix (full, all tasks)
- Task 1 blocks Tasks 2, 4, 5, 6, 7, 8, 9.
- Task 2 blocks Tasks 5, 7, 10.
- Task 3 blocks Tasks 4, 6.
- Task 4 blocks Tasks 5, 6, 10.
- Task 5 blocks Tasks 6, 7, 8, 9, 10.
- Task 6 blocks Task 10.
- Task 7 blocks Task 10.
- Task 8 blocks Task 10 if wrapper tests expose codegen gaps.
- Task 9 blocks Task 10.
- Task 10 blocks Task 11.
- Task 11 blocks Final Verification and final `git status --porcelain`.

### Agent Dispatch Summary (wave → task count → categories)
- Wave 1 → 3 tasks → deep, quick, quick
- Wave 2 → 3 tasks → unspecified-high, deep, quick
- Wave 3 → 3 tasks → unspecified-high, deep, quick
- Wave 4 → 2 tasks → unspecified-high, writing

## TODOs
> Implementation + Test = ONE task. Never separate.
> EVERY task MUST have: Agent Profile + Parallelization + QA Scenarios.

- [ ] 1. Lock strict guard semantics and test inventory

  **What to do**: Create an implementation-local spec comment/test helper plan before changing behavior. Inspect `src/type_system/checker/expressions_guard.rs`, `src/type_system/errors.rs`, `src/type_system/tests.rs`, `tests/integration_e2e/guard_stmt.rs`, `tests/integration_e2e/guard_shorthand.rs`, `test-projects/delete-downloads/src/main.op`, and `test-projects/delete-downloads-strict/src/main.op`. Record the exact existing tests that must be migrated and the fixture names to add. Do not change parser or codegen in this task.
  **Must NOT do**: Do not implement the validator, add new syntax, or modify parser/AST shapes.

  **Recommended Agent Profile**:
  - Category: `deep` - Reason: Requires cross-cutting compiler/test understanding before edits.
  - Skills: `[]` - No special skill required.
  - Omitted: [`frontend-ui-ux`] - Compiler semantics only.

  **Parallelization**: Can Parallel: YES | Wave 1 | Blocks: Tasks 2, 4, 5, 6, 7, 8, 9 | Blocked By: none

  **References** (executor has NO interview context - be exhaustive):
  - Pattern: `src/type_system/checker/expressions_guard.rs:462-703` - Existing terminal propagate, `return err`, and guard else validation logic.
  - Pattern: `src/type_system/errors.rs:370-572` - Existing `TypeError` variants and Miette diagnostic style.
  - Test: `src/type_system/tests.rs:2360-3665` - Existing type-checker guard behavior expectations.
  - Test: `tests/integration_e2e/guard_stmt.rs` - Existing integration guard compile-fail/runtime tests.
  - Fixture: `test-projects/delete-downloads/src/main.op:16-18` - `_ignored_rmdir_err` invalid sentinel.
  - Fixture: `test-projects/delete-downloads-strict/src/main.op:37-43` - Current print/nested-cleanup behavior that should fail before being fixed.

  **Acceptance Criteria** (agent-executable only):
  - [ ] `cargo test guard --lib -- --nocapture` runs and its output is saved to `.sisyphus/evidence/task-1-inventory.txt`.
  - [ ] `cargo test --features integration guard -- --nocapture` runs and its output is saved to `.sisyphus/evidence/task-1-integration-inventory.txt`.
  - [ ] A checklist in `.sisyphus/evidence/task-1-strict-spec.md` lists every invalid/valid form to cover and every existing test file that will change.

  **QA Scenarios** (MANDATORY - task incomplete without these):
  ```
  Scenario: Existing guard test inventory captured
    Tool: Bash
    Steps: Run `cargo test guard --lib -- --nocapture` and `cargo test --features integration guard -- --nocapture`; write complete outputs to the evidence files.
    Expected: Commands complete with current baseline results, and evidence files contain test names for guard-related tests.
    Evidence: .sisyphus/evidence/task-1-inventory.txt

  Scenario: Spec checklist rejects broad handling forms
    Tool: Bash
    Steps: Inspect `.sisyphus/evidence/task-1-strict-spec.md` for entries covering `_ignored_*`, print-only, non-final propagate, `return err`, invalid wrapper source, and valid wrapper source.
    Expected: All listed forms appear exactly once with expected pass/fail classification.
    Evidence: .sisyphus/evidence/task-1-strict-spec.md
  ```

  **Commit**: NO | Message: n/a | Files: `.sisyphus/evidence/task-1-*`

- [ ] 2. Add red-phase compile-fail/pass fixtures and prove current failures

  **What to do**: Add or update multiple minimal test projects under `test-projects/` and integration tests under `tests/integration_e2e/guard_stmt.rs` (or a new guard strict integration file registered by the existing harness) to cover invalid print-only, invalid `_ignored_*` alias, invalid non-final `propagate err`, invalid `return err`, valid terminal `propagate err`, valid shorthand `propagate <call>()`, valid wrapper return with `source: err`, and invalid wrapper return with alias/shadowed source. Add focused tests that assert `delete-downloads` and `delete-downloads-strict` fail before fixes. Run these tests before implementation and capture RED evidence.
  **Must NOT do**: Do not change `src/type_system/**` yet. Do not fix `delete-downloads` or `delete-downloads-strict` yet. Do not commit a permanently failing test state unless the commit also includes the green implementation.

  **Recommended Agent Profile**:
  - Category: `quick` - Reason: Fixture/test additions follow existing compile_project patterns.
  - Skills: `[]` - No special skill required.
  - Omitted: [`git-master`] - Commit occurs only after green slice unless explicitly invoking git workflow.

  **Parallelization**: Can Parallel: YES | Wave 1 | Blocks: Tasks 5, 7, 10 | Blocked By: Task 1

  **References**:
  - Pattern: `tests/integration_e2e/compile_failures.rs:94-106` - Pattern for `CompileError::Report` assertions.
  - Pattern: `tests/integration_e2e/compile_failures.rs:245-252` - Pattern for matching specific `TypeError` in report entries.
  - Pattern: `tests/integration_e2e/guard_stmt.rs:201-314` - Existing guard diagnostic/runtime assertions and delete-downloads tests.
  - Fixture: `test-projects/delete-downloads/opal.toml` and `src/main.op` - Existing loose fixture.
  - Fixture: `test-projects/delete-downloads-strict/README.md` and `src/main.op` - Existing strict fixture markers.

  **Acceptance Criteria**:
  - [ ] New integration tests fail on current code for the intended strict-rule gaps; output is saved to `.sisyphus/evidence/task-2-red-tests.txt`.
  - [ ] `delete-downloads` compile-fail assertion fails or reports the current non-strict behavior as expected in red evidence.
  - [ ] `delete-downloads-strict` compile-fail assertion fails on current code because it currently compiles; evidence explicitly names this expected RED failure.
  - [ ] Existing unrelated tests are not edited to mask failures.

  **QA Scenarios**:
  ```
  Scenario: Red tests fail for strict guard gaps
    Tool: Bash
    Steps: Run `cargo test --features integration strict_guard -- --nocapture` or the exact focused test filter added in this task.
    Expected: Command fails before implementation; failure output names invalid guard handling cases and delete-downloads-strict current compile success.
    Evidence: .sisyphus/evidence/task-2-red-tests.txt

  Scenario: Existing fixtures are not prematurely fixed
    Tool: Bash
    Steps: Run `git diff -- test-projects/delete-downloads test-projects/delete-downloads-strict` immediately after red tests are added.
    Expected: Diff shows no source fixes to existing fixture bodies beyond test harness references, unless a minimal fixture copy was created separately.
    Evidence: .sisyphus/evidence/task-2-fixture-diff.txt
  ```

  **Commit**: NO | Message: n/a | Files: test files and new fixtures remain uncommitted until green implementation lands

- [ ] 3. Add structured Miette guard diagnostics

  **What to do**: Add dedicated `TypeError` variants in `src/type_system/errors.rs` for strict guard failures. Required variants/codes: `GuardErrorClauseMissingTerminal` (`opalescent::guard::missing_terminal`), `GuardPropagateErrNotFinal` (`opalescent::guard::propagate_not_final`), `GuardReturnErrInvalid` (`opalescent::guard::return_err_invalid`), `GuardWrapperSourceInvalid` (`opalescent::guard::wrapper_source_invalid`), and `GuardShorthandRequired` (`opalescent::guard::shorthand_required`). Each must derive/render through existing `miette::Diagnostic` enum style with `#[diagnostic(code(...), help(...))]` and at least one `#[label(...)] SourceSpan` field. Add targeted unit tests for `Diagnostic::code()` and non-empty label spans using existing error-test style.
  **Must NOT do**: Do not use generic `ConstraintSolvingFailed.reason` for new strict guard errors. Do not add a new dependency unless an existing diagnostic rendering helper requires it.

  **Recommended Agent Profile**:
  - Category: `quick` - Reason: Local diagnostic enum/test additions.
  - Skills: `[]` - No special skill required.
  - Omitted: [`frontend-ui-ux`] - CLI diagnostics only, not UI.

  **Parallelization**: Can Parallel: YES | Wave 1 | Blocks: Tasks 4, 6 | Blocked By: Task 1

  **References**:
  - Pattern: `src/type_system/errors.rs:370-408` - Existing `ConstraintSolvingFailed` diagnostic shape.
  - Pattern: `src/type_system/errors.rs:550-572` - Existing guard-related diagnostic variants.
  - API: `src/type_system/errors.rs:950-967` - Existing `span_from_span` conversion helper.
  - External: `https://docs.rs/miette/latest/index.html` - `#[derive(Diagnostic)]`, `#[diagnostic(code, help)]`, `#[label]`, `SourceSpan` patterns.

  **Acceptance Criteria**:
  - [ ] `cargo test --lib type_system::tests::guard` or exact relevant filter passes for new diagnostic construction tests.
  - [ ] Each new variant exposes the exact diagnostic code via `miette::Diagnostic::code()`.
  - [ ] Each new variant has at least one non-empty source label span.

  **QA Scenarios**:
  ```
  Scenario: Diagnostic codes are stable
    Tool: Bash
    Steps: Run the focused diagnostic unit tests added in this task.
    Expected: Tests assert exact codes `opalescent::guard::*` and pass.
    Evidence: .sisyphus/evidence/task-3-diagnostic-codes.txt

  Scenario: Rendered diagnostic includes help and label text
    Tool: Bash
    Steps: Run focused test with `--nocapture` that renders one representative Miette report.
    Expected: Output contains the diagnostic code, a help line, and a source label for the offending guard clause.
    Evidence: .sisyphus/evidence/task-3-miette-render.txt
  ```

  **Commit**: YES | Message: `feat(diagnostics): add structured guard errors` | Files: `src/type_system/errors.rs`, focused diagnostic tests

- [x] 4. Wire new diagnostics into existing guard error checks

  **What to do**: Replace current generic `ConstraintSolvingFailed` returns for known guard error clause violations with the new structured variants. Target non-final `propagate err`, `return err`, only-propagate shorthand recommendation, invalid/missing terminal, and invalid wrapper source. Preserve existing `PropagateOutsideErrorFunction` and `PropagateErrorMismatch` behavior for declared-error compatibility unless tests show they need label upgrades.
  **Must NOT do**: Do not yet replace the broad handled-statement heuristic wholesale; this task only changes diagnostic surfaces for already-detected errors and necessary call-site plumbing.

  **Recommended Agent Profile**:
  - Category: `unspecified-high` - Reason: Requires careful call-site edits in compiler type-checking.
  - Skills: `[]` - No special skill required.
  - Omitted: [`frontend-ui-ux`] - Compiler diagnostics only.

  **Parallelization**: Can Parallel: YES | Wave 2 | Blocks: Tasks 5, 6, 10 | Blocked By: Tasks 1, 3

  **References**:
  - Pattern: `src/type_system/checker/expressions_guard.rs:470-486` - Current non-final propagate and shorthand guidance errors.
  - Pattern: `src/type_system/checker/expressions_guard.rs:531-540` - Current `return err` rejection.
  - Pattern: `src/type_system/checker/expressions_guard.rs:644-703` - Current terminal propagate compatibility checks.
  - Diagnostic: `src/type_system/errors.rs` - New variants from Task 3.

  **Acceptance Criteria**:
  - [ ] Existing tests that expected generic guard reason strings are updated or temporarily dual-asserted to pass with new structured variants.
  - [ ] Focused tests for non-final `propagate err`, `return err`, and shorthand-required cases pass and match exact new `TypeError` variants.
  - [ ] Miette rendered output for at least one guard error contains code, label, and help.

  **QA Scenarios**:
  ```
  Scenario: Known guard violations use structured variants
    Tool: Bash
    Steps: Run focused unit tests for non-final `propagate err`, `return err`, and shorthand-only guard clause.
    Expected: Tests pass by matching new `TypeError::Guard*` variants rather than `ConstraintSolvingFailed.reason`.
    Evidence: .sisyphus/evidence/task-4-structured-variants.txt

  Scenario: Compatibility errors remain stable
    Tool: Bash
    Steps: Run existing tests covering `PropagateOutsideErrorFunction` and `PropagateErrorMismatch`.
    Expected: Tests still pass; no compatibility regression from diagnostic refactor.
    Evidence: .sisyphus/evidence/task-4-compatibility.txt
  ```

  **Commit**: YES | Message: `feat(diagnostics): report guard clauses with miette` | Files: `src/type_system/checker/expressions_guard.rs`, `src/type_system/errors.rs`, related tests

- [x] 5. Implement structural strict guard error clause validator

  **What to do**: Replace the broad “handled” heuristic in `src/type_system/checker/expressions_guard.rs` for named guard error clauses with a single structural validator, e.g. `validate_strict_guard_error_clause`. The validator must inspect top-level else-body statements as `Prelude* + Terminal`; allow arbitrary prelude statements after normal type-checking, but require final top-level terminal to be either `Stmt::PropagateGuardError` for the active binding or direct typed wrapper return with exact `source: err`. Reject print/log-only, `_ignored_*` aliases, fallback success returns, `return void`, `break`, `continue`, `return err`, `propagate <call>()` in-clause, non-final `propagate err`, alias source, and shadowed source. Preserve error-set subset checks in `type_check_guard_error_propagate_terminal`.
  **Must NOT do**: Do not change parser active binding behavior. Do not make nested guards consume the outer error unless the nested terminal explicitly references the outer active binding. Do not tighten unnamed guard `else =>` clauses.

  **Recommended Agent Profile**:
  - Category: `deep` - Reason: Central semantic change with nested scope and type identity risks.
  - Skills: `[]` - No special skill required.
  - Omitted: [`frontend-ui-ux`] - Not UI work.

  **Parallelization**: Can Parallel: NO | Wave 2 | Blocks: Tasks 6, 7, 8, 9, 10 | Blocked By: Tasks 1, 2, 4

  **References**:
  - Pattern: `src/type_system/checker/expressions_guard.rs:307-440` - Guard else branch validation flow.
  - Pattern: `src/type_system/checker/expressions_guard.rs:442-642` - Current statement-level handled heuristic.
  - Pattern: `src/type_system/checker/expressions_guard.rs:723` - Current ignored binding naming helper; `_ignored_*` must not count as handling.
  - AST/API: `src/ast.rs:671-690` - `Stmt::Guard` and `Stmt::PropagateGuardError` shapes.
  - AST/API: `src/ast.rs:449-523` - `Expr::Guard` and `Expr::Propagate` shapes.

  **Acceptance Criteria**:
  - [ ] Red-phase strict guard tests from Task 2 turn green for invalid terminal enforcement.
  - [ ] Existing valid terminal `propagate err` tests still pass.
  - [ ] Print/log-only and `_ignored_*` alias fixtures fail with `opalescent::guard::missing_terminal` or a more specific planned code.
  - [ ] Non-final `propagate err` fails with `opalescent::guard::propagate_not_final`.
  - [ ] `return err` fails with `opalescent::guard::return_err_invalid`.

  **QA Scenarios**:
  ```
  Scenario: Invalid broad handling no longer passes
    Tool: Bash
    Steps: Run focused integration tests for print-only, `_ignored_*`, fallback return, and nested-cleanup-without-terminal fixtures.
    Expected: Each compile fails with the expected `opalescent::guard::*` diagnostic code.
    Evidence: .sisyphus/evidence/task-5-invalid-handling.txt

  Scenario: Valid terminal propagation still works
    Tool: Bash
    Steps: Run focused unit/integration tests containing side-effect prelude followed by final `propagate err`.
    Expected: Each compile succeeds when function declared errors are compatible.
    Evidence: .sisyphus/evidence/task-5-valid-propagate.txt
  ```

  **Commit**: YES | Message: `feat(typecheck): enforce strict guard error terminals` | Files: `src/type_system/checker/expressions_guard.rs`, strict guard tests/fixtures

- [x] 6. Migrate guard diagnostic tests to structured assertions and rendered output checks

  **What to do**: Update `src/type_system/tests.rs`, `tests/integration_e2e/guard_stmt.rs`, and any guard shorthand integration tests that assert literal `ConstraintSolvingFailed.reason` strings. Prefer matching concrete `TypeError::Guard*` variants in report entries; additionally assert rendered diagnostic text contains the stable diagnostic code, help, and source label for representative cases. Add snapshot-style or text fixture output only if the repository already uses a snapshot crate; otherwise use normal string assertions and evidence files.
  **Must NOT do**: Do not remove coverage for old semantic cases; migrate it to structured assertions.

  **Recommended Agent Profile**:
  - Category: `quick` - Reason: Test assertion migration after compiler behavior exists.
  - Skills: `[]` - No special skill required.
  - Omitted: [`frontend-ui-ux`] - Diagnostics are textual.

  **Parallelization**: Can Parallel: YES | Wave 2 | Blocks: Task 10 | Blocked By: Tasks 3, 4, 5

  **References**:
  - Test: `src/type_system/tests.rs:2360-3665` - Existing unit tests with guard diagnostic strings.
  - Test: `tests/integration_e2e/guard_stmt.rs:83-117` - Helper that searches report entries/reasons.
  - Test: `tests/integration_e2e/guard_stmt.rs:201-314` - Current guard expected diagnostic tests.
  - Diagnostic: `src/type_system/errors.rs` - New variant names/codes.

  **Acceptance Criteria**:
  - [ ] No guard strict test depends only on a free-form `ConstraintSolvingFailed.reason` string for new diagnostics.
  - [ ] At least one representative Miette rendered diagnostic is asserted for code/help/label text.
  - [ ] `cargo test guard --lib` passes.
  - [ ] `cargo test --features integration guard` passes, except fixture fix tests intentionally pending until Task 7 if separated.

  **QA Scenarios**:
  ```
  Scenario: Structured assertions replace brittle reason strings
    Tool: Bash
    Steps: Search tests for the old exact strings `return err is not valid in a guard error clause` and `propagate err is only valid as the final statement`.
    Expected: Either no matches remain for new diagnostics, or matches are only compatibility text inside new diagnostic help/message assertions.
    Evidence: .sisyphus/evidence/task-6-string-search.txt

  Scenario: Guard diagnostic tests pass
    Tool: Bash
    Steps: Run `cargo test guard --lib` and `cargo test --features integration guard`.
    Expected: Both commands pass after assertion migration and validator implementation.
    Evidence: .sisyphus/evidence/task-6-tests.txt
  ```

  **Commit**: YES | Message: `test(guard): assert structured guard diagnostics` | Files: `src/type_system/tests.rs`, `tests/integration_e2e/guard_stmt.rs`, related guard tests

- [x] 7. Fix `delete-downloads` and `delete-downloads-strict` after confirmed failures

  **What to do**: After Task 2 and Task 5 prove both fixtures fail under strict rules, update `test-projects/delete-downloads/src/main.op` and `test-projects/delete-downloads-strict/src/main.op` to end each named guard error clause with a valid terminal. For errors that should be forwarded, use final `propagate err` and ensure enclosing functions declare compatible errors. For contextual errors, use the direct wrapper return form with exact `source: err` only if Task 8 support is green. Preserve existing intended runtime behavior and README markers for strict fixture where possible by moving print/log cleanup statements before the terminal.
  **Must NOT do**: Do not suppress errors with `_ignored_*`, print-only clauses, or fallback success returns. Do not change tests to expect silent ignoring.

  **Recommended Agent Profile**:
  - Category: `unspecified-high` - Reason: Fixture migration must preserve semantics while satisfying new compiler rules.
  - Skills: `[]` - No special skill required.
  - Omitted: [`frontend-ui-ux`] - CLI fixture only.

  **Parallelization**: Can Parallel: YES | Wave 3 | Blocks: Task 10 | Blocked By: Tasks 2, 5

  **References**:
  - Fixture: `test-projects/delete-downloads/src/main.op:9-18` - Existing list/delete guard clauses and invalid ignored rmdir error.
  - Fixture: `test-projects/delete-downloads-strict/src/main.op:6-43` - Existing reset, rmdir, fallback delete, marker output behavior.
  - Test: `tests/integration_e2e/guard_stmt.rs:294-314` - Existing delete-downloads related tests.
  - Pattern: `src/type_system/checker/expressions_guard.rs:644-703` - Declared-error compatibility for final `propagate err`.

  **Acceptance Criteria**:
  - [ ] Before fixture edits, focused tests show both projects fail with strict guard diagnostics; evidence saved.
  - [ ] After fixture edits, both projects compile through `compile_project(...)` integration tests.
  - [ ] CLI runs for both projects succeed after `cargo build --release`.
  - [ ] Strict fixture marker assertions are updated only if terminal propagation makes old marker semantics impossible; any update must be justified in test comments.

  **QA Scenarios**:
  ```
  Scenario: Existing projects fail before fixes
    Tool: Bash
    Steps: Run focused compile_project tests for `delete-downloads` and `delete-downloads-strict` before editing fixtures.
    Expected: Both fail with planned strict guard diagnostic codes.
    Evidence: .sisyphus/evidence/task-7-before-fix.txt

  Scenario: Existing projects pass after fixes
    Tool: Bash
    Steps: Run focused compile_project tests and CLI runs for both projects after fixture edits.
    Expected: Both compile and run successfully; output matches asserted markers.
    Evidence: .sisyphus/evidence/task-7-after-fix.txt
  ```

  **Commit**: YES | Message: `fix(fixtures): propagate delete downloads errors strictly` | Files: `test-projects/delete-downloads/**`, `test-projects/delete-downloads-strict/**`, related integration tests

- [x] 8. Implement and verify direct typed wrapper return with `source: err`

  **What to do**: Implement the allowed wrapper terminal shape only if not already fully supported. The validator must recognize a direct top-level `return new ErrorType.Variant: source: err ...` (or exact existing AST equivalent) whose type/variant is declared in the current function errors and whose `source` field is exactly the active guard binding. If type checking accepts wrapper return but codegen fails due payload-bearing error ABI, implement the minimal codegen support in `src/codegen/control_flow.rs`, `src/codegen/error_abi.rs`, and/or `src/codegen/statements.rs`; otherwise document that no codegen change was needed in evidence.
  **Must NOT do**: Do not allow alias source, shadowed `err`, missing `source`, renamed source field, wrapper not in declared errors, arbitrary return expressions, or success-value fallbacks.

  **Recommended Agent Profile**:
  - Category: `deep` - Reason: Highest-risk typechecker/codegen boundary and ABI validation.
  - Skills: `[]` - No special skill required.
  - Omitted: [`frontend-ui-ux`] - Compiler/backend only.

  **Parallelization**: Can Parallel: YES | Wave 3 | Blocks: Task 10 | Blocked By: Tasks 1, 5

  **References**:
  - Pattern: `src/type_system/checker/expressions_guard.rs:515-642` - Current return statement handling in guard error clauses.
  - Pattern: `src/codegen/control_flow.rs:478-631` - Return lowering and error variant extraction; payload-bearing risk.
  - Pattern: `src/codegen/statements.rs:902-951` - Guard error propagation statement lowering.
  - Pattern: `src/codegen/error_abi.rs:79-127` - Error aggregate builders.

  **Acceptance Criteria**:
  - [ ] Valid wrapper return fixture compiles and runs or compile-checks successfully.
  - [ ] Invalid wrapper source fixtures fail with `opalescent::guard::wrapper_source_invalid`.
  - [ ] If codegen changes are needed, existing return/error ABI tests still pass.
  - [ ] Evidence states whether wrapper support was typechecker-only or required backend changes.

  **QA Scenarios**:
  ```
  Scenario: Valid wrapper return is accepted
    Tool: Bash
    Steps: Run focused compile_project test for the valid wrapper return fixture.
    Expected: Compile succeeds and any runtime assertion passes.
    Evidence: .sisyphus/evidence/task-8-wrapper-valid.txt

  Scenario: Invalid wrapper source is rejected
    Tool: Bash
    Steps: Run focused compile_project tests for alias, shadowed, and missing-source wrapper fixtures.
    Expected: Each fails with `opalescent::guard::wrapper_source_invalid`.
    Evidence: .sisyphus/evidence/task-8-wrapper-invalid.txt
  ```

  **Commit**: YES | Message: `feat(guard): support wrapper source returns` | Files: `src/type_system/checker/expressions_guard.rs`, `src/codegen/**` if needed, wrapper tests/fixtures

- [x] 9. Refactor strict guard validator and remove obsolete heuristics

  **What to do**: After tests are green, refactor `expressions_guard.rs` to make the strict validator readable and isolated. Remove unused `handled_bound_error`, `handling_statement_count`, or old helper logic only if no remaining tests or non-error guard paths need it. Keep helper names explicit: terminal classification, active binding identity check, wrapper source check, and diagnostic construction. Run format/checks after refactor.
  **Must NOT do**: Do not change behavior during refactor. Do not remove support for unnamed guard else clauses or expression guards.

  **Recommended Agent Profile**:
  - Category: `quick` - Reason: Local cleanup after green tests.
  - Skills: `[]` - No special skill required.
  - Omitted: [`git-master`] - Commit is straightforward after tests pass.

  **Parallelization**: Can Parallel: YES | Wave 3 | Blocks: Task 10 | Blocked By: Tasks 5, 6, 8

  **References**:
  - Pattern: `src/type_system/checker/expressions_guard.rs` - Final strict validator and obsolete heuristic fields/helpers.
  - Test: `cargo test guard --lib` - Fast regression target.
  - Test: `cargo test --features integration guard` - Integration regression target.

  **Acceptance Criteria**:
  - [ ] `cargo fmt --all -- --check` passes after refactor.
  - [ ] `cargo test guard --lib` passes.
  - [ ] `cargo test --features integration guard` passes.
  - [ ] Diff shows behavior-neutral helper extraction/removal only.

  **QA Scenarios**:
  ```
  Scenario: Refactor preserves strict behavior
    Tool: Bash
    Steps: Run focused guard unit and integration tests after refactor.
    Expected: Same tests pass as before refactor.
    Evidence: .sisyphus/evidence/task-9-refactor-tests.txt

  Scenario: Formatting remains clean
    Tool: Bash
    Steps: Run `cargo fmt --all -- --check`.
    Expected: Command succeeds.
    Evidence: .sisyphus/evidence/task-9-fmt.txt
  ```

  **Commit**: YES | Message: `refactor(guard): simplify strict terminal validation` | Files: `src/type_system/checker/expressions_guard.rs`, related tests only if needed

- [x] 10. Run full verification and create atomic commits in order

  **What to do**: Ensure all implementation work is split into atomic commits in the commit strategy order. Use normal hooks; do not skip verification. Run full build/test/integration commands and save outputs. If any red-phase evidence files are untracked and not meant to be committed, either commit them only if project convention allows `.sisyphus/evidence`, or remove generated evidence before final status check according to repository policy. Final repository state must have all intended source/test changes committed and no staged/unstaged changes.
  **Must NOT do**: Do not use `--no-verify`, do not amend unless hook-modified files require it and git safety rules allow it, do not push unless explicitly requested.

  **Recommended Agent Profile**:
  - Category: `unspecified-high` - Reason: Requires disciplined git/test sequencing.
  - Skills: [`git-master`] - Atomic commits and final clean status.
  - Omitted: [`frontend-ui-ux`] - Not UI work.

  **Parallelization**: Can Parallel: NO | Wave 4 | Blocks: Task 11 | Blocked By: Tasks 6, 7, 8, 9

  **References**:
  - CI: `.github/workflows/ci.yml:30-37` - Linux CI commands: `cargo test --all-features`, clippy, fmt.
  - CI: `.github/workflows/ci.yml:61-66` - Windows build/test shape.
  - Pattern: `src/compiler.rs:585+` - `compile_project(...)` pipeline used by fixtures.
  - Test: `tests/integration_e2e/project_execution.rs` - Existing project execution assertions.

  **Acceptance Criteria**:
  - [ ] `cargo build` succeeds.
  - [ ] `cargo test` succeeds.
  - [ ] `cargo test --features integration` succeeds.
  - [ ] `cargo clippy --all-targets --all-features -- -D warnings` succeeds, or any pre-existing unrelated clippy failure is documented and approved before final.
  - [ ] `cargo fmt --all -- --check` succeeds.
  - [ ] `git log --oneline -8` shows atomic commits matching the planned sequence.
  - [ ] `git status --porcelain` prints no output.

  **QA Scenarios**:
  ```
  Scenario: Full Rust verification passes
    Tool: Bash
    Steps: Run `cargo build`, `cargo test`, `cargo test --features integration`, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo fmt --all -- --check`.
    Expected: Every command succeeds.
    Evidence: .sisyphus/evidence/task-10-full-verification.txt

  Scenario: Git status is clean after atomic commits
    Tool: Bash
    Steps: Run `git log --oneline -8` and `git status --porcelain`.
    Expected: Commit history shows atomic slices; status output is empty.
    Evidence: .sisyphus/evidence/task-10-git-clean.txt
  ```

  **Commit**: YES | Message: multiple atomic green commits per strategy | Files: all intended implementation/test/fixture changes

- [x] 11. Record execution evidence summary, clean evidence files, and verify final status

  **What to do**: Create a concise execution summary during the task listing final diagnostic codes, fixtures added/updated, red-phase confirmation results, final verification commands, commit hashes, and any deviations from the plan. If repository convention allows versioned `.sisyphus/evidence`, commit `.sisyphus/evidence/strict-guard-error-handling-summary.md` as the final evidence commit. Otherwise, include the same summary in the final assistant response, remove untracked `.sisyphus/evidence/*` files before final status verification, and record the cleanup decision in the response. Run `git status --porcelain` after the summary commit/removal decision.
  **Must NOT do**: Do not leave untracked evidence files behind. Do not modify source docs or README files unless required by tests. Do not include secrets or environment-specific paths beyond repository-relative paths.

  **Recommended Agent Profile**:
  - Category: `writing` - Reason: Evidence summary only.
  - Skills: `[]` - No special skill required.
  - Omitted: [`git-master`] - No git operations unless committing this evidence is approved by repository convention.

  **Parallelization**: Can Parallel: NO | Wave 4 | Blocks: Final Verification | Blocked By: Task 10

  **References**:
  - Evidence: `.sisyphus/evidence/task-*` - Outputs captured by each task.
  - Git: `git log --oneline -8` - Atomic commit references.
  - Plan: `.sisyphus/plans/strict-guard-error-handling.md` - Acceptance criteria to summarize.

  **Acceptance Criteria**:
  - [ ] Summary lists every new diagnostic code and one representative fixture/test for it.
  - [ ] Summary lists before/after status for `delete-downloads` and `delete-downloads-strict`.
  - [ ] Summary lists final verification commands and pass/fail status.
  - [ ] Evidence policy is resolved explicitly: either summary/evidence is committed, or untracked evidence files are removed before final status.
  - [ ] `git status --porcelain` prints no output after the evidence policy decision.

  **QA Scenarios**:
  ```
  Scenario: Evidence summary is complete
    Tool: Bash
    Steps: Inspect the committed `.sisyphus/evidence/strict-guard-error-handling-summary.md` if evidence is versioned; otherwise inspect the final response draft generated from the evidence files before cleanup.
    Expected: Diagnostic codes, fixture list, red/green confirmation, final commands, and commit hashes are all present.
    Evidence: .sisyphus/evidence/strict-guard-error-handling-summary.md or final response summary

  Scenario: Final status remains clean after evidence policy decision
    Tool: Bash
    Steps: Compare summary commit hashes with `git log --oneline -8`, then run `git status --porcelain` after either committing or removing evidence files.
    Expected: Every listed commit exists in git history and `git status --porcelain` prints no output.
    Evidence: .sisyphus/evidence/task-11-summary-git-check.txt or final response summary
  ```

  **Commit**: CONDITIONAL | Message: `test(guard): record strict guard evidence` only if `.sisyphus/evidence` is versioned; otherwise remove evidence files before final status | Files: `.sisyphus/evidence/strict-guard-error-handling-summary.md` if versioned

## Final Verification Wave (MANDATORY — after ALL implementation tasks)
> 4 review agents run in PARALLEL after Task 11. ALL must APPROVE. Present consolidated results to user and get explicit "okay" before completing.
> **Do NOT auto-proceed after verification. Wait for user's explicit approval before marking work complete.**
> **Never mark F1-F4 as checked before getting user's okay.** Rejection or user feedback -> fix -> re-run -> present again -> wait for okay.

- [ ] F1. Plan Compliance Audit — oracle

  **Tool/Agent**: `task(subagent_type="oracle", run_in_background=true)`
  **Prompt**: Review `.sisyphus/plans/strict-guard-error-handling.md` against the final git diff and evidence summary. Verify every plan task was completed, every acceptance criterion has evidence, red-phase failures were captured before fixes, no OpenRouter-backed subagents were used during execution, and final `git status --porcelain` is empty. Return APPROVE or BLOCK with exact missing items.
  **Expected Pass Condition**: Oracle returns APPROVE with no critical missing plan requirements.
  **Evidence**: `.sisyphus/evidence/final-f1-plan-compliance.md` if evidence is versioned; otherwise include full oracle verdict in final response summary before evidence cleanup.

- [ ] F2. Code Quality Review — unspecified-high

  **Tool/Agent**: `task(category="unspecified-high", run_in_background=true)`
  **Prompt**: Review the final implementation diff for maintainability, compiler architecture fit, parser/typechecker/codegen boundaries, clear helper names, absence of broad handling heuristics, and no AI-slop patterns. Confirm new code is minimal, readable, and preserves existing non-error guard behavior. Return APPROVE or BLOCK with exact required fixes.
  **Expected Pass Condition**: Reviewer returns APPROVE with no required code-quality fixes.
  **Evidence**: `.sisyphus/evidence/final-f2-code-quality.md` if evidence is versioned; otherwise include full verdict in final response summary before evidence cleanup.

- [ ] F3. Real Manual QA — unspecified-high

  **Tool/Agent**: `task(category="unspecified-high", run_in_background=true)`
  **Prompt**: Execute real QA from a clean working tree: `cargo build`, `cargo test`, `cargo test --features integration`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo fmt --all -- --check`, CLI runs for `test-projects/delete-downloads/src/main.op` and `test-projects/delete-downloads-strict/src/main.op`, and focused compile-fail checks for invalid guard fixtures. Return APPROVE only if all commands pass and diagnostic outputs contain the planned `opalescent::guard::*` codes.
  **Expected Pass Condition**: QA agent returns APPROVE and includes command outputs or exact pass summaries for every command.
  **Evidence**: `.sisyphus/evidence/final-f3-real-qa.md` if evidence is versioned; otherwise include full QA summary in final response before evidence cleanup.

- [ ] F4. Scope Fidelity Check — deep

  **Tool/Agent**: `task(category="deep", run_in_background=true)`
  **Prompt**: Compare the final behavior to the original user request. Confirm only the three approved propagation-only forms are accepted, invalid guard else handling fails at compile time, `delete-downloads` and `delete-downloads-strict` failed before fixes and pass after fixes, diagnostics are structured Miette diagnostics, commits are atomic, and final status is clean. Return APPROVE or BLOCK.
  **Expected Pass Condition**: Scope reviewer returns APPROVE with no deviations from the user request.
  **Evidence**: `.sisyphus/evidence/final-f4-scope-fidelity.md` if evidence is versioned; otherwise include full verdict in final response before evidence cleanup.

## Commit Strategy
- Red-phase tests and fixture failures are created and run in the working tree first, with evidence captured, but **must not be committed as a red-only commit**.
- Commit 1: `feat(diagnostics): add structured guard errors` — green diagnostic scaffolding and tests.
- Commit 2: `feat(typecheck): enforce strict guard error terminals` — strict validator plus the formerly-red fixtures/tests now passing for invalid cases.
- Commit 3: `test(guard): assert structured guard diagnostics` — assertion migration, only if not already included in Commit 2.
- Commit 4: `fix(fixtures): propagate delete downloads errors strictly` — existing fixture source fixes after failure evidence is captured.
- Commit 5: `feat(guard): support wrapper source returns` — only if wrapper-return implementation requires separate codegen/typechecker work.
- Commit 6: `refactor(guard): simplify strict terminal validation` — behavior-neutral cleanup.
- Commit 7: `test(guard): verify strict guard projects end to end` — final fixture/runtime verification, only if not already included in earlier commits.
- Commit 8: `test(guard): record strict guard evidence` — only if `.sisyphus/evidence` is intentionally versioned; otherwise delete evidence files before final status.
- After Task 11 and final verification review, run `git status --porcelain` and require empty output.

## Success Criteria
- All strict guard invalid forms produce structured Miette diagnostics with stable codes and source labels.
- All valid forms compile and run where applicable.
- `delete-downloads` and `delete-downloads-strict` fail in red-phase tests before fixes and pass after fixes.
- No OpenRouter-backed subagents are used during execution.
- All tests/builds pass and repository status is clean.
