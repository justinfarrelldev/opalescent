# Guard Error Propagation Semantics

## TL;DR
> **Summary**: Unify statement guard typing with expression guard typing, fix guard binding scope, and implement guard-clause-only `propagate err` semantics with strict TDD and green atomic commits.
> **Deliverables**:
> - Statement guards parse typed/mutable success bindings like expression guards.
> - Statement guards type-check through the shared expression-guard path.
> - Success bindings are unavailable inside guard error clauses and available only after successful guard completion.
> - Guard error bindings are must-handle values scoped only to the guard error clause.
> - `return err` is rejected in guard error clauses; `propagate err` is accepted only as a guard-error-clause terminal form.
> - Guard clauses that only contain `propagate err` fail with shorthand guidance for `propagate <fallible_call>()`.
> - Multiple end-to-end test projects and all existing broken tests/fixtures migrated without skips/deletions.
> - Atomic commits after each green/refactor slice.
> **Effort**: Large
> **Parallel**: YES - 5 waves
> **Critical Path**: Baseline tests → AST/parser representation → shared guard checker → diagnostics/must-handle → codegen/test-project migration → final verification

## Context
### Original Request
Implement guard semantics changes: statement guards should use the same typed path as expression guards; success bindings must not exist inside else/error branches; guard error bindings must be must-handle; implement propagation-only guard handling without direct `return err`; introduce `propagate err` as the terminal guard error-clause propagation form; reject guard clauses that only propagate; use strict RED-GREEN-REFACTOR TDD; create multiple test-projects; fix existing broken test projects; use atomic commits frequently.

### Interview Summary
- Confirmed `propagate err` is a guard-error-clause-only terminal form.
- Confirmed ordinary `propagate fallible_call()` remains the shorthand/general propagation expression.
- Confirmed a long-form guard error clause must perform at least one real handling action before final `propagate err`; a clause containing only `propagate err` fails.
- Confirmed execution should commit after each green/refactor slice.
- Test strategy is strict TDD: every slice writes/observes failing tests first, implements to green, refactors, reruns gates, then commits.

### Metis Review (gaps addressed)
- **Representation decision**: Use a new statement-only AST form for guard error propagation, named `Stmt::PropagateGuardError` or the closest repository-style equivalent, carrying the error binding identifier and span/diagnostic location fields matching nearby `Stmt` variants. Do not reuse or broaden ordinary `Expr::Propagate`.
- **Error binding decision**: Replace statement-guard `CoreType::String` error binding with compiler-tracked guard error context based on the guarded expression's actual error set. The binding is only valid inside the guard error clause and can only be consumed by guard-approved handling, especially terminal `propagate err`.
- **Must-handle decision**: The bound guard error is considered handled when the error clause reaches a valid terminal handler consuming the active guard error binding. This change does not require an extra pre-terminal textual reference to `err`; it does require at least one non-propagate statement before final `propagate err` in long form.
- **Terminal-set decision**: For this proposal, accepted terminal for the original guard error in long-form statement guard clauses is final top-level `propagate <active_error_binding>`. Do not implement wrapper-constructor returns, `return err`, or branch-sensitive divergent-flow analysis in this change.
- **Shadowing decision**: If an outer variable has the same name as a guard success binding, the outer variable remains accessible in the error clause according to normal lexical scoping; the not-yet-bound guard success value itself must not be visible there. Add a regression case for this.
- **Commit policy decision**: Do not commit red states. For each slice, record RED evidence, implement GREEN, optionally refactor, run gates, then commit the green slice.
- **Diagnostic strategy decision**: Reuse existing type/parser test patterns and compile-fail assertion helpers discovered in the repo; do not introduce a new snapshot framework unless the existing tests already use one.

## Work Objectives
### Core Objective
Make statement guards semantically consistent with expression guards while adding narrow guard-error propagation semantics and diagnostics requested by the propagation-only proposal caveat.

### Deliverables
- Parser/AST support for typed/mutable statement guard success bindings.
- Parser/AST support for statement-only guard terminal `propagate err` inside guard error clauses.
- Shared type-checking path for statement and expression guards.
- Guard error binding scope and must-handle enforcement.
- Diagnostics for success-binding leaks, invalid `return err`, invalid/out-of-context `propagate err`, non-terminal guard error clauses, and propagate-only long-form guard clauses.
- Codegen for the guard-only propagation terminal without using the current `return err` hack.
- Updated proposal/examples/tests to reflect no direct return handling.
- New and migrated end-to-end test projects.
- Green full CI-equivalent command set.

### Definition of Done (verifiable conditions with commands)
- `cargo fmt --all -- --check` exits 0.
- `cargo clippy --all-targets --all-features -- -D warnings` exits 0.
- `cargo test --all-features` exits 0.
- Existing guard tests still pass after being migrated to new semantics where necessary.
- New compile-pass test projects build/run and produce expected stdout.
- New compile-fail test projects fail with exact diagnostic substrings listed in this plan.
- No existing tests are skipped, ignored, commented out, or deleted.
- Git history contains green atomic commits for each completed slice.

### Must Have
- Strict RED-GREEN-REFACTOR evidence for every implementation task.
- New tests before implementation in each slice.
- Statement guards reusing expression-guard validation for guarded expression/error-set compatibility.
- Error clause does not see the not-yet-successful guard success binding.
- `return err` is a type error inside guard error clauses.
- `propagate err` only works as final top-level statement in the active guard error clause.
- Long-form guard error clause with only `propagate err` is rejected.
- Shorthand `propagate fallible_call()` remains valid and unchanged outside this guard-specific terminal form.

### Must NOT Have (guardrails, AI slop patterns, scope boundaries)
- Do not implement direct `return err` support.
- Do not generalize `propagate err` into a normal expression or non-guard syntax.
- Do not change expression-guard surface syntax/semantics except through shared checker extraction needed for statement parity.
- Do not add wrapper-constructor returns, match-based error handlers, `?`/`try!` syntax, mappers, `with` clauses, deprecation warnings, IDE hints, or broad error-value redesign.
- Do not add unrelated refactors, new test frameworks, or performance optimizations.
- Do not skip/comment/delete failing tests or test projects.
- Do not commit failing red states.

## Verification Strategy
> ZERO HUMAN INTERVENTION - all verification is agent-executed.
- Test decision: TDD / RED-GREEN-REFACTOR using Rust unit/integration tests and Opalescent test-project E2E fixtures.
- QA policy: Every task has agent-executed scenarios.
- Evidence: `.sisyphus/evidence/task-{N}-{slug}.{ext}`.
- Per-slice full gate:
  ```bash
  cargo fmt --all -- --check && cargo clippy --all-targets --all-features -- -D warnings && cargo test --all-features
  ```

## Execution Strategy
### Parallel Execution Waves
> Target: 5-8 tasks per wave. <3 per wave (except final) = under-splitting.
> Extract shared dependencies as Wave-1 tasks for max parallelism.

Wave 1: Task 1 baseline impact map and characterization tests; Task 2 AST/parser representation red tests; Task 3 typechecker red tests.
Wave 2: Task 4 parser/AST implementation; Task 5 shared checker refactor; Task 6 guard error binding and scope enforcement.
Wave 3: Task 7 propagation diagnostics/must-handle rules; Task 8 codegen lowering; Task 9 proposal/docs/test fixture migration.
Wave 4: Task 10 multiple E2E test projects; Task 11 full test-project migration and regression sweep; Task 12 atomic commit audit and CI gate.
Wave 5: Final Verification Wave F1-F4.

### Dependency Matrix (full, all tasks)
- Task 1 blocks Tasks 4-12.
- Task 2 blocks Task 4.
- Task 3 blocks Tasks 5-7.
- Task 4 blocks Tasks 5, 8, 10.
- Task 5 blocks Tasks 6-7.
- Task 6 blocks Tasks 7-8.
- Task 7 blocks Tasks 8, 10, 11.
- Task 8 blocks Tasks 10-12.
- Task 9 can run after Task 7 semantics are fixed; blocks Task 12.
- Task 10 blocks Task 11.
- Task 11 blocks Task 12.
- Task 12 blocks Final Verification Wave.

### Agent Dispatch Summary (wave → task count → categories)
- Wave 1 → 3 tasks → `deep`, `quick`, `quick`.
- Wave 2 → 3 tasks → `deep`, `deep`, `deep`.
- Wave 3 → 3 tasks → `deep`, `deep`, `writing`.
- Wave 4 → 3 tasks → `deep`, `unspecified-high`, `quick`.
- Wave 5 → 4 review tasks → oracle / unspecified-high / unspecified-high / deep.

## TODOs
> Implementation + Test = ONE task. Never separate.
> EVERY task MUST have: Agent Profile + Parallelization + QA Scenarios.

- [x] 1. Baseline impact map and characterization tests

  **What to do**: Use read-only LSP/search before modifying implementation: map references to `Stmt::Guard`, `type_check_guard_statement`, `codegen_guard_statement`, `Expr::Propagate`, `parse_guard_statement`, `parse_guard_expression`, and existing guard diagnostics. Add characterization tests only where existing coverage is absent for current statement guard behavior that must intentionally change: success binding visible in else, error binding typed as string, `return err` behavior if present, and ordinary `propagate <call>` behavior. RED evidence must show new tests fail under current behavior for the new expected semantics; GREEN for this task may be achieved only by marking the tests as expected compile-fail/pass using existing repository helpers, not by changing compiler semantics yet.
  **Must NOT do**: Do not change parser/typechecker/codegen semantics. Do not add a new test framework. Do not skip/comment/delete existing tests.

  **Recommended Agent Profile**:
  - Category: `deep` - Requires repo-wide impact mapping and test convention discovery without implementation drift.
  - Skills: [] - No external skill needed.
  - Omitted: [`git-master`] - The plan itself gives commit protocol; no history rewrite needed.

  **Parallelization**: Can Parallel: YES | Wave 1 | Blocks: Tasks 4-12 | Blocked By: none

  **References** (executor has NO interview context - be exhaustive):
  - Pattern: `src/parser/statements_guard.rs` - statement guard parsing entrypoint.
  - Pattern: `src/parser/expressions.rs` - expression guard and ordinary `propagate <call>` parsing.
  - Pattern: `src/type_system/checker/statements.rs` - current divergent statement guard checker.
  - Pattern: `src/type_system/checker/expressions_guard.rs` - target shared guard typed path.
  - Pattern: `src/type_system/checker/control_flow.rs` - `GuardUsage`, `GuardBindingInfo`, wrapper seam.
  - Pattern: `src/codegen/statements.rs` - statement guard lowering.
  - Test: `tests/integration_e2e/guard_shorthand.rs` - E2E compile/run fixture pattern.
  - Test: `tests/integration_e2e/guard_optional_binding.rs` - inline source guard integration pattern.

  **Acceptance Criteria** (agent-executable only):
  - [ ] LSP/reference output is saved to `.sisyphus/evidence/task-1-impact-map.md` and includes all files above.
  - [ ] New characterization tests exist for success binding in else, error binding current/string assumption, ordinary `propagate <call>`, and any current `return err` guard behavior found.
  - [ ] RED evidence saved at `.sisyphus/evidence/task-1-red.txt` shows at least one new expected-new-semantics test failing before implementation.
  - [ ] Full gate passes after characterization-only adjustments: `cargo fmt --all -- --check && cargo clippy --all-targets --all-features -- -D warnings && cargo test --all-features`.

  **QA Scenarios** (MANDATORY - task incomplete without these):
  ```
  Scenario: Baseline mapping complete
    Tool: Bash
    Steps: Run `cargo test --all-features` after adding characterization tests; write referenced files and command output to `.sisyphus/evidence/task-1-impact-map.md` and `.sisyphus/evidence/task-1-green.txt`.
    Expected: Command exits 0 and evidence lists parser, AST, checker, codegen, diagnostics, and test files.
    Evidence: .sisyphus/evidence/task-1-impact-map.md

  Scenario: New semantics currently fail
    Tool: Bash
    Steps: Run the narrow new-semantics characterization test(s) before implementation and capture output.
    Expected: Output shows failure caused by current compiler behavior, not test harness errors.
    Evidence: .sisyphus/evidence/task-1-red.txt
  ```

  **Commit**: YES | Message: `test(guards): slice 1 - capture guard baseline behavior` | Files: [`tests/**`, `.sisyphus/evidence/**` if evidence is committed by repo convention]

- [x] 2. Add RED parser and AST tests for typed statement guards and guard-only propagation terminal

  **What to do**: Add failing parser/AST tests for statement guard syntax parity with expression guards: `guard fallible() into value: Type mutable else err => ...`. Add failing parser/AST tests for `propagate err` accepted only as a final statement inside a statement guard error clause. Add failing parser/AST test that bare `propagate err` outside guard error context remains invalid or is parsed then rejected by type checker according to existing parser-test convention; do not make ordinary `Expr::Propagate` accept identifiers globally.
  **Must NOT do**: Do not implement parser changes in this task except minimal test scaffolding. Do not change expression guard grammar.

  **Recommended Agent Profile**:
  - Category: `quick` - Focused test additions in parser/AST layer.
  - Skills: [] - No special skill.
  - Omitted: [`frontend-ui-ux`] - No UI.

  **Parallelization**: Can Parallel: YES | Wave 1 | Blocks: Task 4 | Blocked By: none

  **References**:
  - Pattern: `src/parser/statements_guard.rs` - current statement guard grammar to test against.
  - Pattern: `src/parser/expressions.rs` - mirror typed/mutable binding parsing from expression guard.
  - API/Type: `src/ast.rs` - `Stmt::Guard`, `Expr::Guard`, `Expr::Propagate` shapes.
  - Test: `src/parser/tests.rs` - parser unit-test style if present; otherwise use closest existing parser test module found in Task 1.

  **Acceptance Criteria**:
  - [ ] Parser tests assert statement guards can represent success binding annotation and mutability.
  - [ ] Parser tests assert guard-only `propagate err` is represented as statement-only terminal, not `Expr::Propagate`.
  - [ ] RED evidence saved to `.sisyphus/evidence/task-2-red.txt` shows tests fail before implementation.
  - [ ] No compiler implementation files beyond tests are semantically changed in this task unless needed for test compilation placeholders.

  **QA Scenarios**:
  ```
  Scenario: Parser RED for typed statement guard
    Tool: Bash
    Steps: Run the exact parser test name added for `guard fallible() into value: int32 mutable else err => ...`.
    Expected: Test fails because current `Stmt::Guard` lacks typed/mutable representation or parser support.
    Evidence: .sisyphus/evidence/task-2-red.txt

  Scenario: Parser RED for guard-only propagate terminal
    Tool: Bash
    Steps: Run the parser test for `propagate err` inside and outside guard error clause.
    Expected: Inside-guard test fails until new statement-only AST form exists; outside-guard behavior remains invalid or type-rejected per chosen implementation.
    Evidence: .sisyphus/evidence/task-2-propagate-red.txt
  ```

  **Commit**: YES | Message: `test(parser): slice 2 - specify statement guard syntax parity` | Files: [`src/parser/**`, `src/ast.rs` only if placeholder compile updates are unavoidable]

- [x] 3. Add RED typechecker tests for guard scope and must-handle semantics

  **What to do**: Add failing typechecker tests for: success binding unavailable in guard error clause; guard success binding available after guard; outer variable shadowing remains accessible in error clause when guard success binding uses the same name; `return err` rejected in guard error clause; `propagate err` accepted only final top-level in active guard error clause; only `propagate err` rejected with exact diagnostic; non-terminal/fallthrough error clause rejected; ordinary `propagate <call>` unchanged.
  **Must NOT do**: Do not implement checker changes. Do not weaken existing expression guard tests.

  **Recommended Agent Profile**:
  - Category: `quick` - Focused type-system test design.
  - Skills: [] - No special skill.
  - Omitted: [`librarian`] - No external docs needed.

  **Parallelization**: Can Parallel: YES | Wave 1 | Blocks: Tasks 5-7 | Blocked By: none

  **References**:
  - Pattern: `src/type_system/tests.rs` - existing type-system test style and likely current string-error assumptions.
  - Pattern: `src/type_system/checker/expressions_guard.rs` - desired expression guard checker behavior.
  - Pattern: `src/type_system/errors.rs` - diagnostic variants/messages.
  - Pattern: `language-spec/error_handling_samples.op` - error handling examples.

  **Acceptance Criteria**:
  - [ ] Tests require exact diagnostic substrings:
    - `success binding is not available inside guard error clause`
    - `return err is not valid in a guard error clause; use propagate err to forward the guard error`
    - `guard error clause must perform handling before propagating; replace this guard with shorthand propagate <call>() when no handling is needed`
    - `propagate err is only valid as the final statement of a guard error clause`
    - `guard error clause must handle or propagate the bound error`
  - [ ] RED evidence saved to `.sisyphus/evidence/task-3-red.txt` shows all new checker tests fail for the intended reasons.

  **QA Scenarios**:
  ```
  Scenario: Typechecker RED for success binding leak
    Tool: Bash
    Steps: Run the added typechecker test compiling a guard error clause that references the guard success binding.
    Expected: Test fails before implementation because current statement checker exposes success binding.
    Evidence: .sisyphus/evidence/task-3-scope-red.txt

  Scenario: Typechecker RED for only-propagate clause
    Tool: Bash
    Steps: Run the added typechecker test with `else err => propagate err`.
    Expected: Test fails before implementation because current compiler does not emit the required shorthand diagnostic.
    Evidence: .sisyphus/evidence/task-3-propagate-red.txt
  ```

  **Commit**: YES | Message: `test(typeck): slice 3 - specify guard error handling rules` | Files: [`src/type_system/**`, `tests/**`]

- [x] 4. Implement statement guard AST/parser parity and guard-only terminal representation

  **What to do**: Extend `Stmt::Guard` or parser output so statement guards carry the same success binding metadata as expression guards: binding name, optional type annotation, and mutability. Add a new statement-only AST variant named `Stmt::PropagateGuardError` (or exact repo-style name chosen during implementation) for `propagate <identifier>` in guard error clauses. Wire `src/parser/statements_guard.rs` to parse optional `: Type` and `mutable` after `into <identifier>` by mirroring `parse_guard_expression`. Parse `propagate err` inside guard error clause into the new statement-only form without widening `src/parser/expressions.rs::parse_propagate_expression`. Update exhaustiveness in statement visitors/formatters/codegen/checker with explicit TODO-type errors until later tasks implement semantics, but keep build green.
  **Must NOT do**: Do not alter expression guard grammar. Do not make ordinary `propagate identifier` legal globally. Do not implement checker/codegen semantics beyond exhaustiveness stubs needed for green parser tests.

  **Recommended Agent Profile**:
  - Category: `deep` - Cross-layer AST/parser updates require careful exhaustiveness handling.
  - Skills: [] - No special skill.
  - Omitted: [`writing`] - Not primarily docs.

  **Parallelization**: Can Parallel: NO | Wave 2 | Blocks: Tasks 5, 8, 10 | Blocked By: Tasks 1-2

  **References**:
  - Pattern: `src/parser/statements_guard.rs` - add optional annotation/mutable parsing.
  - Pattern: `src/parser/expressions.rs` - mirror expression guard binding parsing; leave ordinary propagate unchanged.
  - API/Type: `src/ast.rs` - add metadata and statement-only terminal variant.
  - Test: parser tests from Task 2.

  **Acceptance Criteria**:
  - [ ] Task 2 parser tests pass.
  - [ ] Existing expression guard tests pass unchanged.
  - [ ] `cargo test --all-features` passes after implementation.
  - [ ] Evidence saved to `.sisyphus/evidence/task-4-green.txt`.

  **QA Scenarios**:
  ```
  Scenario: Typed mutable statement guard parses
    Tool: Bash
    Steps: Run parser tests for `guard f() into value: int32 mutable else err => ...`.
    Expected: AST includes binding name `value`, annotation `int32`, and mutable flag true.
    Evidence: .sisyphus/evidence/task-4-parser-green.txt

  Scenario: Ordinary propagate remains call-only
    Tool: Bash
    Steps: Run parser tests for `propagate err` outside guard context and `propagate fallible()` in ordinary context.
    Expected: Outside bare identifier remains invalid or not checker-valid; call form remains accepted.
    Evidence: .sisyphus/evidence/task-4-propagate-green.txt
  ```

  **Commit**: YES | Message: `feat(parser): slice 4 - parse typed statement guards` | Files: [`src/ast.rs`, `src/parser/**`, affected exhaustiveness files, parser tests]

- [x] 5. Refactor statement guards onto shared expression-guard checker path

  **What to do**: Refactor `src/type_system/checker/statements.rs::type_check_guard_statement` so statement guards construct `GuardBindingInfo` with parsed annotation/mutability and call the shared guard checker with `GuardUsage::Statement`. Preserve expression guard behavior. Remove duplicate statement-specific success binding registration before/inside the error clause.
  **Must NOT do**: Do not add must-handle diagnostics in this task except what is necessary to keep tests compiling. Do not change expression guard surface syntax.

  **Recommended Agent Profile**:
  - Category: `deep` - Semantic checker refactor across scopes and diagnostics.
  - Skills: [] - No special skill.
  - Omitted: [`quick`] - Too risky for trivial execution.

  **Parallelization**: Can Parallel: NO | Wave 2 | Blocks: Tasks 6-7 | Blocked By: Tasks 1, 3, 4

  **References**:
  - Pattern: `src/type_system/checker/statements.rs` - replace divergent statement guard checker.
  - Pattern: `src/type_system/checker/expressions_guard.rs` - shared target validation.
  - API/Type: `src/type_system/checker/control_flow.rs` - `GuardUsage::Statement`, `GuardBindingInfo`.
  - Test: Task 3 typechecker tests.

  **Acceptance Criteria**:
  - [ ] Statement guard subject/error-set tests match expression guard behavior.
  - [ ] No success binding is registered before checking the error clause.
  - [ ] Diagnostic drift from Task 1 baseline is documented in `.sisyphus/evidence/task-5-diagnostic-diff.md`; only intentional changed-semantics tests differ.
  - [ ] Full gate passes.

  **QA Scenarios**:
  ```
  Scenario: Statement and expression guards reject same invalid subject
    Tool: Bash
    Steps: Run paired typechecker tests for invalid guard subject in expression and statement forms.
    Expected: Both forms fail with equivalent guard-on-non-error diagnostic.
    Evidence: .sisyphus/evidence/task-5-shared-path.txt

  Scenario: Refactor preserves expression guard behavior
    Tool: Bash
    Steps: Run all existing expression guard tests plus `cargo test --all-features`.
    Expected: Existing expression guard tests pass without expected-output changes.
    Evidence: .sisyphus/evidence/task-5-green.txt
  ```

  **Commit**: YES | Message: `refactor(typeck): slice 5 - share guard checker path` | Files: [`src/type_system/checker/**`, typechecker tests]

- [x] 6. Enforce guard binding scope and typed guard error context

  **What to do**: Make the guard success binding unavailable inside the guard error clause and available only after the guard succeeds. Preserve normal outer lexical variables with the same name inside the error clause. Replace statement-guard `CoreType::String` error binding with guard-error context based on the guarded expression's actual error set/type information. The error binding exists only inside the guard error clause and cannot be used after it.
  **Must NOT do**: Do not expose guard error binding as a general first-class error value outside the guard error clause. Do not change ordinary variable scoping beyond the guard success binding bug.

  **Recommended Agent Profile**:
  - Category: `deep` - Scope/type changes with regression risk.
  - Skills: [] - No special skill.
  - Omitted: [`writing`] - Not docs-first.

  **Parallelization**: Can Parallel: NO | Wave 2 | Blocks: Tasks 7-8 | Blocked By: Task 5

  **References**:
  - Pattern: `src/type_system/checker/statements.rs` - remove old `CoreType::String` error binding behavior.
  - Pattern: `src/type_system/checker/expressions_guard.rs` - guard error stack and scoped else handling.
  - Pattern: `src/type_system/errors.rs` - add/reuse diagnostic variant.
  - Test: Task 3 tests for scope, shadowing, and binding lifetime.

  **Acceptance Criteria**:
  - [ ] Guard error clause referencing the guard success binding fails with exact diagnostic substring: `success binding is not available inside guard error clause`.
  - [ ] Outer variable shadowed by a guard success binding remains accessible in the error clause as the outer variable, not as the guard success value.
  - [ ] Guard success binding is available after the guard in success path.
  - [ ] Guard error binding cannot be referenced after the guard clause.
  - [ ] Full gate passes.

  **QA Scenarios**:
  ```
  Scenario: Success binding unavailable in error clause
    Tool: Bash
    Steps: Compile test source where `guard fallible() into value else err => use(value); propagate err`.
    Expected: Non-zero compile failure with `success binding is not available inside guard error clause`.
    Evidence: .sisyphus/evidence/task-6-success-scope.txt

  Scenario: Outer shadowing remains lexical
    Tool: Bash
    Steps: Compile/run source where outer `value` exists, guard binds `value`, and error clause uses outer `value` before `propagate err`.
    Expected: Compile succeeds or fails only for unrelated propagation compatibility; no success-binding leak diagnostic.
    Evidence: .sisyphus/evidence/task-6-shadowing.txt
  ```

  **Commit**: YES | Message: `fix(typeck): slice 6 - scope guard bindings correctly` | Files: [`src/type_system/**`, tests]

- [x] 7. Implement guard error must-handle diagnostics and `propagate err` rules

  **What to do**: Implement type-checking rules for statement-only guard terminal: `propagate <active_error_binding>` is valid only as the final top-level statement in the active guard error clause, only when the surrounding function can propagate the guarded error set, and only when the clause contains at least one prior non-propagate handling statement. Reject `return err` inside guard error clauses with exact diagnostic. Reject missing/fallthrough handlers. Reject only-propagate clauses with shorthand guidance. Preserve ordinary `propagate <fallible_call>()` unchanged.
  **Must NOT do**: Do not implement branch-sensitive terminal analysis, wrapper-constructor returns, or normal `return err` special casing. Do not require textual use of `err` before terminal propagation beyond final `propagate err` consumption.

  **Recommended Agent Profile**:
  - Category: `deep` - Core semantic diagnostics and error-set compatibility.
  - Skills: [] - No special skill.
  - Omitted: [`artistry`] - Standard compiler semantics, not creative exploration.

  **Parallelization**: Can Parallel: NO | Wave 3 | Blocks: Tasks 8, 10, 11 | Blocked By: Task 6

  **References**:
  - Pattern: `src/type_system/checker/expressions_guard.rs` - statement guard else-branch validation and error stack.
  - Pattern: `src/type_system/errors.rs` - add exact diagnostics.
  - Pattern: `src/parser/expressions.rs` - ordinary propagate call remains unchanged.
  - External: `error-handler-proposals/propagation-only/proposal.md` - proposal source, but implementation intentionally excludes direct return.

  **Acceptance Criteria**:
  - [ ] `return err` in guard error clause fails with exact substring: `return err is not valid in a guard error clause; use propagate err to forward the guard error`.
  - [ ] Only `propagate err` fails with exact substring: `guard error clause must perform handling before propagating; replace this guard with shorthand propagate <call>() when no handling is needed`.
  - [ ] `propagate err` outside active guard error clause fails with exact substring: `propagate err is only valid as the final statement of a guard error clause`.
  - [ ] Error clause without valid terminal handler fails with exact substring: `guard error clause must handle or propagate the bound error`.
  - [ ] Clause with real handling statement followed by final `propagate err` passes when surrounding function errors are compatible.
  - [ ] Full gate passes.

  **QA Scenarios**:
  ```
  Scenario: Side-effect then propagate accepted
    Tool: Bash
    Steps: Compile source with `else err => log_guard_error(err); propagate err` in a compatible error-returning function.
    Expected: Compile exits 0 and test asserts emitted behavior if run.
    Evidence: .sisyphus/evidence/task-7-side-effect-propagate.txt

  Scenario: Only propagate rejected
    Tool: Bash
    Steps: Compile source with `else err => propagate err`.
    Expected: Compile exits non-zero and output contains shorthand diagnostic exactly.
    Evidence: .sisyphus/evidence/task-7-only-propagate.txt
  ```

  **Commit**: YES | Message: `feat(typeck): slice 7 - enforce guard error propagation rules` | Files: [`src/type_system/**`, parser/typechecker tests]

- [x] 8. Lower guard-only `propagate err` without `return err` codegen hack

  **What to do**: Implement codegen for statement-only `propagate err` terminal in guard error clauses. Reuse existing propagation/error-return lowering concepts where safe, but do not lower by manufacturing or accepting `return err`. Ensure the active guard error payload/set propagates to the surrounding function's error return ABI consistently with ordinary `propagate <call>()`. Confirm codegen still scopes error binding only during else body and success binding only after merge.
  **Must NOT do**: Do not duplicate large chunks of `Expr::Propagate` lowering if helper extraction can safely share code. Do not make `return err` work. Do not change unrelated function return codegen.

  **Recommended Agent Profile**:
  - Category: `deep` - LLVM/codegen propagation semantics with ABI risk.
  - Skills: [] - No special skill.
  - Omitted: [`quick`] - Not trivial.

  **Parallelization**: Can Parallel: NO | Wave 3 | Blocks: Tasks 10-12 | Blocked By: Tasks 4, 6, 7

  **References**:
  - Pattern: `src/codegen/statements.rs` - `codegen_guard_statement` and binding lifetime.
  - Pattern: `src/codegen/functions_call.rs` - existing ordinary propagate expression lowering; use as reference, do not blindly reuse for identifier error.
  - Pattern: `src/codegen/control_flow.rs` - existing return/control-flow helpers if present.
  - Test: E2E guard tests and new codegen/run tests from Tasks 7 and 10.

  **Acceptance Criteria**:
  - [ ] Compile-pass source with side effect then `propagate err` builds and runs, propagating guarded error to surrounding function.
  - [ ] `return err` remains invalid in guard clause after codegen changes.
  - [ ] Existing guard shorthand/named-binding E2E tests pass after migration.
  - [ ] Full gate passes.

  **QA Scenarios**:
  ```
  Scenario: Runtime propagation reaches surrounding function
    Tool: Bash
    Steps: Build/run a test project where inner guard error clause logs marker then `propagate err`, and outer function handles/prints propagated error marker.
    Expected: Exit 0 and stdout contains both handling marker and outer propagated-error marker.
    Evidence: .sisyphus/evidence/task-8-runtime-propagation.txt

  Scenario: No return-err lowering fallback
    Tool: Bash
    Steps: Compile source using `return err` in guard error clause after codegen implementation.
    Expected: Compile fails in type checking before codegen with exact return-err diagnostic.
    Evidence: .sisyphus/evidence/task-8-return-err-rejected.txt
  ```

  **Commit**: YES | Message: `feat(codegen): slice 8 - lower guard error propagation` | Files: [`src/codegen/**`, codegen/E2E tests]

- [x] 9. Update propagation-only proposal and examples to match implemented caveat

  **What to do**: Update `error-handler-proposals/propagation-only/proposal.md` and any local examples/tests that document `return err` in guard error clauses so they instead describe `propagate err` as the guard-only terminal form and explicitly state that direct return handling is excluded. Keep docs changes minimal and scoped to changed semantics.
  **Must NOT do**: Do not perform a broad documentation rewrite. Do not add unimplemented future features.

  **Recommended Agent Profile**:
  - Category: `writing` - Focused technical prose migration.
  - Skills: [] - No special skill.
  - Omitted: [`deep`] - Semantics already decided by implementation tasks.

  **Parallelization**: Can Parallel: YES | Wave 3 | Blocks: Task 12 | Blocked By: Task 7

  **References**:
  - External: `error-handler-proposals/propagation-only/proposal.md` - primary proposal to update.
  - Pattern: `language-spec/error_handling_samples.op` - update only if it contains now-invalid guard examples.
  - Pattern: `README.md` - update only if guard examples become invalid under new semantics.

  **Acceptance Criteria**:
  - [ ] Proposal no longer presents direct `return err` as an implemented guard handler.
  - [ ] Proposal includes `propagate err` terminal caveat and only-propagate shorthand rule.
  - [ ] Documentation examples compile or are explicitly marked proposal-only/non-compiling according to existing doc conventions.
  - [ ] Full gate passes if docs are checked by tests; otherwise `cargo test --all-features` passes.

  **QA Scenarios**:
  ```
  Scenario: Proposal text matches semantics
    Tool: Bash
    Steps: Search docs/proposals for `return err` and inspect each occurrence for validity.
    Expected: No guard error-clause documentation recommends direct `return err`.
    Evidence: .sisyphus/evidence/task-9-doc-search.txt

  Scenario: Examples remain coherent
    Tool: Bash
    Steps: Run doc-related tests if present, otherwise run `cargo test --all-features`.
    Expected: Exit 0.
    Evidence: .sisyphus/evidence/task-9-green.txt
  ```

  **Commit**: YES | Message: `docs(guards): slice 9 - document propagate error handlers` | Files: [`error-handler-proposals/propagation-only/proposal.md`, related docs/examples if needed]

- [x] 10. Add multiple end-to-end test projects for guard propagation semantics

  **What to do**: Create multiple test projects under `test-projects/` following existing conventions (`opal.toml`, `.gitignore`, `README.md`, `src/main.op`, expected files if used). Minimum projects: `guard-stmt-typed-binding` (typed/mutable success binding pass), `guard-stmt-propagate-err` (side-effect + final propagate pass), `guard-stmt-success-binding-leak` (compile-fail), `guard-stmt-only-propagate` (compile-fail), and `guard-stmt-return-err-banned` (compile-fail). Add Rust integration tests in `tests/integration_e2e/` that build/run pass projects and assert compile-fail diagnostics for fail projects using existing helpers.
  **Must NOT do**: Do not consolidate away required semantic coverage. Do not skip projects because unit tests exist.

  **Recommended Agent Profile**:
  - Category: `deep` - E2E fixture design across compiler/link/run and diagnostics.
  - Skills: [] - No special skill.
  - Omitted: [`quick`] - Multiple fixtures and integration harness changes.

  **Parallelization**: Can Parallel: NO | Wave 4 | Blocks: Task 11 | Blocked By: Tasks 4, 7, 8

  **References**:
  - Test: `tests/integration_e2e/guard_shorthand.rs` - compile/run E2E pattern.
  - Test: `tests/integration_e2e/guard_optional_binding.rs` - inline program compilation pattern.
  - Pattern: `test-projects/guard-shorthand/opal.toml` - manifest convention.
  - Pattern: `test-projects/guard-shorthand/src/main.op` - guard fixture style.
  - Pattern: `test-projects/array-map/expected/stdout.txt` - expected stdout convention.

  **Acceptance Criteria**:
  - [ ] At least five new guard test projects exist with names listed above or stricter equivalents.
  - [ ] Compile-pass projects build/run and assert deterministic stdout markers.
  - [ ] Compile-fail projects assert exact diagnostic substrings from Tasks 6-7.
  - [ ] E2E command `cargo test --features integration guard_stmt` or exact discovered filter passes.
  - [ ] Full gate passes.

  **QA Scenarios**:
  ```
  Scenario: Compile-pass projects run end-to-end
    Tool: Bash
    Steps: Run integration tests for `guard-stmt-typed-binding` and `guard-stmt-propagate-err`.
    Expected: Exit 0; stdout contains declared markers such as `typed-binding-ok` and `propagated-after-handling-ok`.
    Evidence: .sisyphus/evidence/task-10-pass-projects.txt

  Scenario: Compile-fail projects emit exact diagnostics
    Tool: Bash
    Steps: Run integration tests for `guard-stmt-success-binding-leak`, `guard-stmt-only-propagate`, and `guard-stmt-return-err-banned`.
    Expected: Each compile exits non-zero and output contains its exact expected diagnostic substring.
    Evidence: .sisyphus/evidence/task-10-fail-projects.txt
  ```

  **Commit**: YES | Message: `test(e2e): slice 10 - cover guard propagation projects` | Files: [`test-projects/guard-stmt-*/**`, `tests/integration_e2e/**`]

- [x] 11. Migrate existing broken guard/error test projects and tests without skips

  **What to do**: Run a complete guard/error regression sweep. Fix every existing broken test, fixture, README example, expected stdout/stderr, or type-system expectation caused by the semantic change. Use AST-aware/content search to find all `return err`, `propagate err`, guard `else err`, and current string-error assumptions in `test-projects/`, `tests/`, `src/type_system/tests.rs`, and docs. For each failure, migrate to the new semantics: use `propagate <fallible_call>()` for pure propagation, side-effect + `propagate err` for long form, or alternative local handling that does not return the raw error.
  **Must NOT do**: Do not skip, ignore, comment out, delete, or weaken tests. Do not accept unrelated fixture output changes.

  **Recommended Agent Profile**:
  - Category: `unspecified-high` - Broad hands-on migration and regression fixing.
  - Skills: [] - No special skill.
  - Omitted: [`writing`] - Includes code/test migration, not prose only.

  **Parallelization**: Can Parallel: NO | Wave 4 | Blocks: Task 12 | Blocked By: Tasks 7, 8, 10

  **References**:
  - Test: `tests/integration_e2e/guard_shorthand.rs` - existing guard E2E test likely affected.
  - Test: `tests/integration_e2e/guard_optional_binding.rs` - existing guard optional binding tests likely affected.
  - Pattern: `test-projects/guard-shorthand/src/main.op` - existing fixture likely affected.
  - Pattern: `src/type_system/tests.rs` - likely current string-error assumptions.

  **Acceptance Criteria**:
  - [ ] Search/migration checklist saved to `.sisyphus/evidence/task-11-migration-checklist.md` with every touched occurrence and resolution.
  - [ ] No `#[ignore]`, commented-out test bodies, deleted guard tests, or removed test projects are introduced.
  - [ ] `cargo test --features integration` passes.
  - [ ] `cargo test --all-features` passes.

  **QA Scenarios**:
  ```
  Scenario: Existing integration tests migrated
    Tool: Bash
    Steps: Run `cargo test --features integration` after migration.
    Expected: Exit 0; existing guard integration tests pass under new semantics.
    Evidence: .sisyphus/evidence/task-11-integration-green.txt

  Scenario: No skip/delete shortcuts
    Tool: Bash
    Steps: Inspect git diff for `#[ignore]`, deleted guard tests/projects, or commented-out failing tests.
    Expected: No prohibited shortcuts appear.
    Evidence: .sisyphus/evidence/task-11-no-skips.txt
  ```

  **Commit**: YES | Message: `test(guards): slice 11 - migrate guard regressions` | Files: [`tests/**`, `test-projects/**`, related docs/examples]

- [x] 12. Final green gate, atomic commit audit, and evidence consolidation

  **What to do**: Run final CI-equivalent gate, inspect git log/diff/status to verify atomic green commits exist for every slice, ensure all evidence files referenced by tasks exist, and verify no implementation changes remain uncommitted. If hooks or tests fail, fix via the smallest follow-up green commit and rerun the gate.
  **Must NOT do**: Do not amend unless allowed by git safety rules and the failed hook auto-modified files in the just-created commit. Do not push.

  **Recommended Agent Profile**:
  - Category: `quick` - Final verification and commit hygiene.
  - Skills: [] - No special skill.
  - Omitted: [`deep`] - No new implementation intended.

  **Parallelization**: Can Parallel: NO | Wave 4 | Blocks: Final Verification Wave | Blocked By: Tasks 9-11

  **References**:
  - Command: `cargo fmt --all -- --check && cargo clippy --all-targets --all-features -- -D warnings && cargo test --all-features` - final gate.
  - Command: `git status`, `git log --oneline -n 20`, `git diff --stat` - audit only.
  - Evidence: `.sisyphus/evidence/task-*` - task proof artifacts.

  **Acceptance Criteria**:
  - [ ] Final gate exits 0.
  - [ ] Git status shows no unstaged/staged implementation changes after final commit, except intentionally untracked evidence if repo convention excludes it.
  - [ ] Git log shows atomic commits corresponding to slices completed.
  - [ ] Evidence index saved to `.sisyphus/evidence/task-12-final-index.md`.

  **QA Scenarios**:
  ```
  Scenario: CI parity gate passes
    Tool: Bash
    Steps: Run `cargo fmt --all -- --check && cargo clippy --all-targets --all-features -- -D warnings && cargo test --all-features`.
    Expected: Exit 0.
    Evidence: .sisyphus/evidence/task-12-final-gate.txt

  Scenario: Atomic commit audit passes
    Tool: Bash
    Steps: Run git status/log/diff audit and record slice commits.
    Expected: No uncommitted implementation changes; slice commits are present and green.
    Evidence: .sisyphus/evidence/task-12-commit-audit.txt
  ```

  **Commit**: YES | Message: `chore(guards): slice 12 - finalize guard propagation work` | Files: [`.sisyphus/evidence/**` if evidence committed, any final tiny fixes]

## Final Verification Wave (MANDATORY — after ALL implementation tasks)
> 4 review agents run in PARALLEL. ALL must APPROVE. Present consolidated results to user and get explicit "okay" before completing.
> **Do NOT auto-proceed after verification. Wait for user's explicit approval before marking work complete.**
> **Never mark F1-F4 as checked before getting user's okay.** Rejection or user feedback -> fix -> re-run -> present again -> wait for okay.
- [x] F1. Plan Compliance Audit — oracle

  **What to do**: Review the completed implementation against this plan line-by-line. Verify every numbered task's acceptance criteria, QA evidence, commit expectation, and guardrail was satisfied. Produce a pass/fail report that lists any missing requirement by task number and file path.

  **Recommended Agent Profile**:
  - Category: N/A - Direct reviewer: `oracle`.
  - Skills: [] - No special skill.

  **QA Scenarios**:
  ```
  Scenario: Plan compliance audit
    Tool: oracle review + Bash
    Steps: Inspect `.sisyphus/plans/guard-error-propagation.md`, `.sisyphus/evidence/task-*`, `git log --oneline -n 30`, and `git status`. Compare implementation evidence against every task acceptance criterion.
    Expected: Report says APPROVED only if every required task, evidence artifact, and green atomic commit is present; otherwise report REJECTED with exact missing items.
    Evidence: .sisyphus/evidence/final-f1-plan-compliance.md
  ```

- [x] F2. Code Quality Review — unspecified-high

  **What to do**: Review changed compiler code for maintainability, minimal scope, correct shared checker design, no expression-guard semantic drift, no broad error-value redesign, no `return err` hack, and no unrelated refactors.

  **Recommended Agent Profile**:
  - Category: `unspecified-high` - Hands-on code review across parser/checker/codegen/tests.
  - Skills: [] - No special skill.

  **QA Scenarios**:
  ```
  Scenario: Code quality and scope review
    Tool: Bash + code inspection
    Steps: Run `git diff --stat` and inspect changed parser, AST, checker, diagnostics, codegen, tests, and docs. Run `cargo clippy --all-targets --all-features -- -D warnings`.
    Expected: Clippy exits 0; report APPROVED only if changes are minimal, readable, scoped to guard semantics, and contain no direct-return propagation hack or broad propagate expression generalization.
    Evidence: .sisyphus/evidence/final-f2-code-quality.md
  ```

- [x] F3. Real Manual QA — unspecified-high

  **What to do**: Execute real end-to-end verification of pass and fail test projects, including runtime output for compile-pass projects and diagnostic output for compile-fail projects. This is agent-executed manual QA, not human inspection.

  **Recommended Agent Profile**:
  - Category: `unspecified-high` - Hands-on QA with build/run commands.
  - Skills: [] - No UI/browser skill needed.

  **QA Scenarios**:
  ```
  Scenario: E2E pass/fail project QA
    Tool: Bash
    Steps: Run `cargo test --features integration guard_stmt` or the exact discovered integration filter for the guard statement projects; then run `cargo test --all-features`.
    Expected: Commands exit 0; pass projects produce expected stdout markers; fail projects assert exact diagnostic substrings.
    Evidence: .sisyphus/evidence/final-f3-e2e-qa.txt
  ```

- [x] F4. Scope Fidelity Check — deep

  **What to do**: Verify the implementation matches the user's requested semantics and excludes explicitly out-of-scope work. Check that direct `return err` is not implemented, `propagate err` is guard-clause-only terminal syntax, ordinary `propagate <call>` remains unchanged, only-propagate long form is rejected, and existing tests were migrated rather than skipped/deleted.

  **Recommended Agent Profile**:
  - Category: `deep` - Requires semantic comparison against original request and final code state.
  - Skills: [] - No special skill.

  **QA Scenarios**:
  ```
  Scenario: Scope fidelity and regression check
    Tool: Bash + code inspection
    Steps: Inspect tests and code for `return err`, `propagate err`, `Expr::Propagate`, and guard error diagnostics; run `cargo fmt --all -- --check && cargo clippy --all-targets --all-features -- -D warnings && cargo test --all-features`.
    Expected: Gate exits 0; report APPROVED only if semantics match the original request and no prohibited scope expansion or test skipping/deletion is present.
    Evidence: .sisyphus/evidence/final-f4-scope-fidelity.md
  ```

## Commit Strategy
- Every numbered implementation task records RED evidence, reaches GREEN, runs the slice gate, then commits the green state.
- Do not commit known-failing red test states.
- Commit message format: `type(scope): slice N - imperative summary`.
- Recommended messages are listed per task and may be used exactly unless changed files make a narrower scope more accurate.
- Do not push unless the user explicitly requests it after `/start-work`.

## Success Criteria
- All TODO acceptance criteria are checked by agents.
- Final verification agents F1-F4 all approve.
- User explicitly approves the consolidated final verification report.
- Plan execution leaves no skipped/deleted/commented tests and no uncommitted implementation changes except generated evidence if intentionally untracked.
