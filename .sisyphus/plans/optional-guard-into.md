# Optional Guard Into Shorthand

## TL;DR
> **Summary**: Add statement-form shorthand `guard <expr> else <err> =>` as the no-success-binding equivalent of `guard <expr> into _ else <err> =>`, with TDD-first parser/diagnostic coverage and Windows/Wine validation. Keep expression guards and named statement guards unchanged, and emit a dedicated miette diagnostic for ambiguous bare guarded `if` subjects.
> **Deliverables**:
> - Statement guard AST/type/codegen/formatter support for omitted success binding.
> - Dedicated parser diagnostic `opalescent::parser::guard_ambiguous_if_else` with labeled miette output and LSP-compatible spans.
> - Parser, type-system, codegen, formatter, compile-failure, test-project, and Wine validation tests.
> - Final cleanup rewriting `guard ... into _ else ... =>` occurrences in `test-projects/` to shorthand only after green validation.
> **Effort**: Medium
> **Parallel**: YES - 4 waves
> **Critical Path**: Task 1 → Task 2 → Task 3 → Task 5 → Task 9 → Final Verification

## Context
### Original Request
Make the `into` part of guard clauses optional so code like:

```opal
guard delete_file_sync(child) into _ else delete_err =>
    print('DELETE_ERR={path_to_string(child)}: {delete_err}')
```

can be written as:

```opal
guard delete_file_sync(child) else delete_err =>
    print('DELETE_ERR={path_to_string(child)}: {delete_err}')
```

Use extensive TDD, add a helpful miette build error for syntactically ambiguous guarded if-statements such as `guard if some_operation() else err`, create a test project for ambiguous cases, validate Windows/Wine behavior, and finally rewrite all `into _` guard statements in test projects to the shorthand.

### Interview Summary
- Prior discussion already decided the feature should be implemented.
- Shorthand applies to statement guards only.
- Explicit `guard ... into value else ...` remains the named-binding form.
- Omitted `into` must not create a fake `_` binding in AST, type checker, or codegen environment.
- Ambiguous bare `if` subjects should fail with a targeted diagnostic telling users to wrap the if-expression in parentheses.
- Existing `guard ... into _ else ...` remains valid for compatibility; only test projects are rewritten as cleanup.
- Test strategy is RED-GREEN-REFACTOR TDD because the user explicitly requested extensive TDD.

### Metis Review (gaps addressed)
- Metis identified two surfaces that must not be conflated: statement guards vs expression guards. The plan explicitly keeps expression guard shorthand out of scope and adds a negative test.
- Metis warned that desugaring to `_` leaks semantics. The plan changes statement AST to `Option<String>` and requires omitted binding to introduce no symbol.
- Metis flagged formatter policy ambiguity. Decision: formatter preserves AST shape; shorthand stays shorthand and legacy `into _` stays legacy. The requested test-project rewrite is a one-shot cleanup, not a global formatter auto-rewrite.
- Metis flagged diagnostic scope. Decision: v1 diagnostic targets bare guarded `if` subjects and nested/top-level `else` confusion; other expressions are covered by parser regression tests but not expanded into a broad grammar redesign.
- Metis required Wine pass criteria. The plan uses existing prereq script and `windows-wine` harness with explicit commands; if prereqs report `SKIP`, executor records that as environment evidence and still runs the non-Wine cross-target compile test where possible.
- Metis required cleanup ordering. The cleanup task is last and blocked by green Linux/integration/Wine validation.

## Work Objectives
### Core Objective
Implement optional `into` for statement-form guard clauses as a safe shorthand for discarded success values, while preserving existing guard semantics and improving parser diagnostics for ambiguous if-expression cases.

### Deliverables
- `Stmt::Guard.success_binding` changed from `String` to `Option<String>` or an equivalent absent-marker.
- `src/parser/statements.rs` accepts both `guard expr into success else err =>` and `guard expr else err =>`.
- `src/parser/expressions.rs` continues requiring `into` for expression-form guards.
- `src/parser/errors.rs` has a dedicated miette diagnostic for ambiguous bare guarded if-expressions.
- Type checker and codegen skip success-symbol registration when the binding is omitted.
- Formatter prints shorthand for omitted-binding AST nodes and preserves explicit `into _` for explicit AST nodes.
- A dedicated ambiguous-guard test project exists under `test-projects/` and compile-failure integration tests assert its parser error.
- Windows/Wine path validates a shorthand-guard program can build/run as a Windows target when prereqs are available.
- All `into _` guard occurrences in `test-projects/` are rewritten to shorthand after stability.

### Definition of Done (verifiable conditions with commands)
- `cargo test --lib guard` exits 0.
- `cargo test --lib errors::tests::guard_ambiguous_if_else_diagnostic` exits 0.
- `cargo test --features integration compile_failures::ambiguous_guard_if_project_fails_with_miette_help` exits 0.
- `cargo test --features integration guard_shorthand_project_compiles_links_and_runs -- --nocapture` exits 0.
- `bash scripts/verify-wine-prereqs.sh` exits 0 or prints a `SKIP:` reason that is captured in `.sisyphus/evidence/task-9-wine-guard-shorthand.txt`.
- If Wine prereqs are OK: `cargo build --release && cargo test --features "integration windows-wine" --test integration_e2e -- --nocapture wine_msvc_guard_shorthand` exits 0.
- `cargo build --release && cargo test --all-features` exits 0, or environment-gated Wine skips are captured with exact evidence.
- `grep -R "into _ else" test-projects/` returns no matches after cleanup.

### Must Have
- Existing explicit named guard syntax remains valid and behaviorally unchanged.
- Existing explicit `into _` statement guards remain valid, even though test projects are migrated to shorthand.
- Omitted success binding creates no symbol in the type symbol table and no user-visible variable in codegen env.
- Ambiguous guarded `if` diagnostic includes: stable diagnostic code, message, primary label, help line mentioning parentheses, and rendered miette output test coverage.
- Parser recovery emits one primary ambiguity error for the ambiguous test fixtures and does not cascade into unrelated declarations.
- Every implementation task captures evidence in `.sisyphus/evidence/`.

### Must NOT Have (guardrails, AI slop patterns, scope boundaries)
- Must NOT make expression-form `guard expr else handler` valid.
- Must NOT synthesize `_` as a real success binding for omitted `into`.
- Must NOT auto-rewrite user code via formatter; only rewrite `test-projects/` in the final cleanup task.
- Must NOT broaden this into a full parser redesign or support parenthesis-free `guard if ... else ... else err =>`.
- Must NOT change runtime error encoding, propagation semantics, or non-guard syntax.
- Must NOT alter unrelated diagnostics or refactor unrelated parser/type/codegen modules opportunistically.

## Verification Strategy
> ZERO HUMAN INTERVENTION - all verification is agent-executed.
- Test decision: RED-GREEN-REFACTOR TDD + Rust unit/integration tests + existing Wine harness.
- QA policy: Every task has agent-executed scenarios.
- Evidence: `.sisyphus/evidence/task-{N}-{slug}.{ext}`.

## Execution Strategy
### Parallel Execution Waves
> Target: 5-8 tasks per wave. <3 per wave (except final) = under-splitting.
> Extract shared dependencies as Wave-1 tasks for max parallelism.

Wave 1: Tasks 1-3 — parser/AST/diagnostic foundation (sequential within wave: 1 → 2 → 3)
Wave 2: Tasks 4-7 — semantic consumers and formatter/test-project fixture (Task 4 after Task 2; Tasks 5-7 after Task 3/4 as noted)
Wave 3: Tasks 8-10 — integration, Windows/Wine, cleanup (sequential: 8 → 9 → 10)
Wave 4: Task 11 — full regression and documentation/spec alignment

### Dependency Matrix (full, all tasks)
- Task 1: no blockers; blocks Tasks 2, 3.
- Task 2: blocked by Task 1; blocks Tasks 4, 5, 6, 7.
- Task 3: blocked by Tasks 1-2; blocks Tasks 5, 8.
- Task 4: blocked by Task 2; blocks Tasks 5, 8, 9.
- Task 5: blocked by Tasks 3-4; blocks Tasks 8, 9.
- Task 6: blocked by Task 2; blocks Task 10.
- Task 7: blocked by Tasks 2-3; blocks Task 8.
- Task 8: blocked by Tasks 3, 5, 7; blocks Tasks 9, 10.
- Task 9: blocked by Tasks 5, 8; blocks Task 10.
- Task 10: blocked by Tasks 6, 8, 9; blocks Task 11.
- Task 11: blocked by Tasks 1-10.

### Agent Dispatch Summary (wave → task count → categories)
- Wave 1 → 3 tasks → quick, unspecified-high
- Wave 2 → 4 tasks → quick, unspecified-high
- Wave 3 → 3 tasks → unspecified-high
- Wave 4 → 1 task → deep

## TODOs
> Implementation + Test = ONE task. Never separate.
> EVERY task MUST have: Agent Profile + Parallelization + QA Scenarios.

- [x] 1. Add RED parser and diagnostic tests for the new guard grammar

  **What to do**: Add failing tests before implementation. In `src/parser/tests.rs`, add statement-parser tests for: `guard foo() else err =>` producing `Stmt::Guard { success_binding: None, error_binding: "err" }`; explicit `guard foo() into ok else err =>` remaining `Some("ok")`; legacy `guard foo() into _ else err =>` remaining explicit `Some("_")`; expression-position shorthand such as `let x = guard foo() else fallback` still rejected via `GuardMissingIntoClause` or another expression-guard-specific error. In `src/errors/tests.rs`, add a failing diagnostic render test for the new ambiguous-if parser error variant with expected code/message/help text. Do not change parser behavior yet except minimal test helpers if required.
  **Must NOT do**: Do not implement optional parsing in this task. Do not update production AST fields in this task unless tests cannot compile; if compilation requires AST field changes, stop after the smallest compile-facing test adjustment and leave behavior failing.

  **Recommended Agent Profile**:
  - Category: `quick` - Reason: targeted test additions in existing test files.
  - Skills: [] - no specialized skill needed.
  - Omitted: [`frontend-ui-ux`] - no UI/browser work.

  **Parallelization**: Can Parallel: NO | Wave 1 | Blocks: [2, 3] | Blocked By: []

  **References** (executor has NO interview context - be exhaustive):
  - Pattern: `src/parser/tests.rs:324-405` - existing guard expression parse tests and guard-specific missing-into/missing-else error tests.
  - Pattern: `src/parser/tests.rs:5639-5681` - tests for invalid guard statement tokens that should return errors, not panic.
  - Pattern: `src/errors/tests.rs:65-77` - diagnostic formatting assertion style for code/help/docs.
  - API/Type: `src/ast.rs:682-689` - current `Stmt::Guard` shape; expected to become optional in Task 2.
  - API/Type: `src/parser/errors.rs:44-68` - existing `GuardMissingIntoClause` and `GuardMissingElseClause` diagnostics.

  **Acceptance Criteria** (agent-executable only):
  - [ ] `cargo test --lib guard_shorthand_without_into_parses_as_statement -- --nocapture` fails for the expected reason before implementation: shorthand not yet accepted or AST lacks `None` binding.
  - [ ] `cargo test --lib guard_expression_shorthand_still_requires_into -- --nocapture` exists and fails only if expression shorthand is incorrectly accepted.
  - [ ] `cargo test --lib guard_ambiguous_if_else_diagnostic -- --nocapture` exists and fails because the diagnostic variant/renderer is not implemented yet.
  - [ ] Evidence written to `.sisyphus/evidence/task-1-red-parser-diagnostics.txt` with command outputs.

  **QA Scenarios** (MANDATORY - task incomplete without these):
  ```
  Scenario: RED shorthand parser test exists
    Tool: Bash
    Steps: cargo test --lib guard_shorthand_without_into_parses_as_statement -- --nocapture
    Expected: Test is discovered and fails for missing feature, not because of syntax/test harness errors unrelated to guard parsing.
    Evidence: .sisyphus/evidence/task-1-red-parser-diagnostics.txt

  Scenario: RED ambiguity diagnostic test exists
    Tool: Bash
    Steps: cargo test --lib guard_ambiguous_if_else_diagnostic -- --nocapture
    Expected: Test is discovered and fails because `GuardAmbiguousIfElse` diagnostic support is not implemented.
    Evidence: .sisyphus/evidence/task-1-red-parser-diagnostics-error.txt
  ```

  **Commit**: NO | Message: `test(parser): capture optional guard into expectations` | Files: [`src/parser/tests.rs`, `src/errors/tests.rs`]

- [x] 2. Change statement guard AST and parser to support omitted success binding

  **What to do**: Update `Stmt::Guard.success_binding` in `src/ast.rs` from `String` to `Option<String>`. Update `src/parser/statements.rs` so `parse_guard_statement` parses either `into <identifier>` or no `into`; when no `into`, it must immediately require `else <identifier> =>`. Update all direct pattern matches to compile with `Option<String>` but leave semantic behavior for omitted binding minimal until Task 4/5. Preserve existing explicit `into _` as `Some("_")`. Update tests from Task 1 to assert AST shape. Avoid touching `Expr::Guard` fields.
  **Must NOT do**: Do not allow `guard expr else handler` in expression position. Do not replace missing binding with `_`. Do not remove `GuardMissingIntoClause` because expression guards still use it.

  **Recommended Agent Profile**:
  - Category: `unspecified-high` - Reason: AST type change touches multiple consumers and requires careful compile fixes.
  - Skills: [] - no specialized skill needed.
  - Omitted: [`git-master`] - no commit requested during implementation task.

  **Parallelization**: Can Parallel: NO | Wave 1 | Blocks: [3, 4, 5, 6, 7] | Blocked By: [1]

  **References** (executor has NO interview context - be exhaustive):
  - API/Type: `src/ast.rs:682-689` - `Stmt::Guard { expression, success_binding, error_binding, else_body, span, id }` current AST.
  - Pattern: `src/parser/statements.rs:127-133` - guard statement/expression dispatch.
  - Pattern: `src/parser/statements.rs:141-166` - current `is_guard_statement_form()` first-`else` lookahead; update comments and logic for shorthand.
  - Pattern: `src/parser/statements.rs:669-734` - current `parse_guard_statement` requiring `Into`.
  - Consumer: `src/parser/captures.rs:194-201` - statement guard capture traversal ignores binding and should keep compiling.
  - Consumer: `src/type_system/rc_analysis.rs:277-284` - statement guard variable-use traversal ignores binding and should keep compiling.

  **Acceptance Criteria** (agent-executable only):
  - [ ] `cargo test --lib guard_shorthand_without_into_parses_as_statement -- --nocapture` exits 0.
  - [ ] `cargo test --lib guard_into_underscore_still_parses_as_explicit_binding -- --nocapture` exits 0.
  - [ ] `cargo test --lib guard_expression_shorthand_still_requires_into -- --nocapture` exits 0.
  - [ ] `cargo test --lib test_guard_success_binding_with_invalid_token -- --nocapture` exits 0 to prove legacy invalid-token behavior still reports errors.
  - [ ] Evidence written to `.sisyphus/evidence/task-2-ast-parser-shorthand.txt`.

  **QA Scenarios** (MANDATORY - task incomplete without these):
  ```
  Scenario: Shorthand parses as omitted binding
    Tool: Bash
    Steps: cargo test --lib guard_shorthand_without_into_parses_as_statement -- --nocapture
    Expected: Test passes and AST assertion confirms `success_binding == None`, `error_binding == "err"`.
    Evidence: .sisyphus/evidence/task-2-ast-parser-shorthand.txt

  Scenario: Expression shorthand remains invalid
    Tool: Bash
    Steps: cargo test --lib guard_expression_shorthand_still_requires_into -- --nocapture
    Expected: Test passes and confirms expression guard still rejects missing `into` with an expression-guard-specific parse error.
    Evidence: .sisyphus/evidence/task-2-ast-parser-shorthand-error.txt
  ```

  **Commit**: NO | Message: `feat(parser): support omitted guard success binding` | Files: [`src/ast.rs`, `src/parser/statements.rs`, `src/parser/tests.rs`, compile-fix consumers]

- [x] 3. Implement dedicated ambiguous guarded-if miette diagnostic and parser recovery

  **What to do**: Add `ParseError::GuardAmbiguousIfElse` or similarly named variant in `src/parser/errors.rs` with code `opalescent::parser::guard_ambiguous_if_else`. Include fields for at least the `if` span and the guard `else` span if miette supports multiple labels in the current style; if derive constraints make two labels awkward, use a primary label on the confusing `else` and include the `if` in the message/help. Update statement-guard parsing to detect bare guarded `if` subjects whose `else` can be consumed as part of the if-expression before the guard's `else <err> =>`, and emit exactly one targeted error with recovery to newline/block boundary. The help must instruct: wrap the if-expression in parentheses, e.g. `guard (if ... else ...) else err =>`.
  **Must NOT do**: Do not try to make parenthesis-free `guard if ... else ... else err =>` valid. Do not emit generic `Expected 'else'`/`MissingToken` for the known ambiguous pattern.

  **Recommended Agent Profile**:
  - Category: `unspecified-high` - Reason: parser ambiguity/recovery and diagnostic quality require careful handling.
  - Skills: [] - no specialized skill needed.
  - Omitted: [`frontend-ui-ux`] - diagnostics are terminal/miette, not browser UI.

  **Parallelization**: Can Parallel: NO | Wave 1 | Blocks: [5, 7, 8] | Blocked By: [1, 2]

  **References** (executor has NO interview context - be exhaustive):
  - Pattern: `src/parser/errors.rs:44-68` - guard-specific miette diagnostic variants and `#[diagnostic(code(...), help(...))]` style.
  - Pattern: `src/parser/expressions.rs:92-118` - `parse_if_expression` consumes optional `else` as an if branch.
  - Pattern: `src/parser/expressions.rs:224-234` - `recover_guard_clause()` recovery boundaries for malformed guard expressions; mirror or reuse concept for statement guard recovery.
  - Pattern: `src/errors/tests.rs:65-77` - renderer assertions for diagnostic code/help/docs.
  - Pattern: `src/lsp/diagnostics.rs` - maps miette labels to LSP ranges; new diagnostic labels must be compatible.

  **Acceptance Criteria** (agent-executable only):
  - [ ] `cargo test --lib guard_ambiguous_if_else_diagnostic -- --nocapture` exits 0 and rendered output contains `opalescent::parser::guard_ambiguous_if_else`, `ambiguous`, `guard`, `parentheses`, and `guard (`.
  - [ ] `cargo test --lib guard_ambiguous_if_else_recovers_without_cascading_errors -- --nocapture` exits 0 and asserts exactly one parser error for a source with a following valid declaration.
  - [ ] `cargo test --lib guard_parenthesized_if_subject_parses -- --nocapture` exits 0.
  - [ ] Evidence written to `.sisyphus/evidence/task-3-ambiguous-if-diagnostic.txt`.

  **QA Scenarios** (MANDATORY - task incomplete without these):
  ```
  Scenario: Ambiguous bare if gets targeted miette diagnostic
    Tool: Bash
    Steps: cargo test --lib guard_ambiguous_if_else_diagnostic -- --nocapture
    Expected: Rendered diagnostic has stable parser code and help telling the user to wrap the if-expression in parentheses.
    Evidence: .sisyphus/evidence/task-3-ambiguous-if-diagnostic.txt

  Scenario: Parenthesized if remains valid
    Tool: Bash
    Steps: cargo test --lib guard_parenthesized_if_subject_parses -- --nocapture
    Expected: Test passes; parser accepts `guard (if ... else ...) else err =>` where the guarded expression is parenthesized.
    Evidence: .sisyphus/evidence/task-3-ambiguous-if-diagnostic-error.txt
  ```

  **Commit**: NO | Message: `fix(parser): explain ambiguous guard if subjects` | Files: [`src/parser/errors.rs`, `src/parser/statements.rs`, `src/parser/tests.rs`, `src/errors/tests.rs`]

- [x] 4. Update type checker and scope handling for optional statement guard bindings

  **What to do**: Update `src/type_system/checker/statements.rs` so `type_check_guard_stmt_with_return` and `type_check_guard_statement` accept `Option<&str>` for the success binding. When `Some(name)`, preserve current behavior including registering the success value outside the else scope and making it visible after the guard. When `None`, type-check the guarded expression for its success type but register no success symbol in the outer scope or the else scope. Always register only the error binding inside the else scope. Add tests proving shorthand guards type-check, named guards still expose their success binding afterward, omitted bindings cannot be referenced afterward, and explicit `into _` compatibility remains valid.
  **Must NOT do**: Do not make the success binding visible inside the `else` body for omitted bindings. Do not change expression guard typing in `src/type_system/checker/expressions_guard.rs` except compile fixes caused by shared AST imports.

  **Recommended Agent Profile**:
  - Category: `unspecified-high` - Reason: scope and type behavior is semantically sensitive.
  - Skills: [] - no specialized skill needed.
  - Omitted: [`visual-engineering`] - no UI work.

  **Parallelization**: Can Parallel: NO | Wave 2 | Blocks: [5, 8, 9] | Blocked By: [2]

  **References** (executor has NO interview context - be exhaustive):
  - Pattern: `src/type_system/checker/statements.rs:151-165` - statement dispatch passes `success_binding.as_str()` today.
  - Pattern: `src/type_system/checker/statements.rs:600-657` - `type_check_guard_statement` currently registers success binding unconditionally, then registers success and error binding inside else scope.
  - Pattern: `src/type_system/checker/statements.rs:787-805` - wrapper currently accepts `&str` success binding.
  - Test helper: `src/type_system/tests.rs:46-130` - parser/typechecker source helpers for integration-style type tests.
  - Guardrail: `src/type_system/checker/expressions_guard.rs` - expression guard typing must continue requiring explicit `binding_name`.

  **Acceptance Criteria** (agent-executable only):
  - [ ] `cargo test --lib type_check_guard_shorthand_discards_success_binding -- --nocapture` exits 0.
  - [ ] `cargo test --lib type_check_guard_shorthand_success_binding_not_in_scope -- --nocapture` exits 0 and asserts a `SymbolNotFound` or equivalent for referencing the omitted binding.
  - [ ] `cargo test --lib type_check_named_guard_binding_still_available_after_guard -- --nocapture` exits 0.
  - [ ] `cargo test --lib type_check_guard_into_underscore_still_valid -- --nocapture` exits 0.
  - [ ] Evidence written to `.sisyphus/evidence/task-4-typecheck-optional-guard-binding.txt`.

  **QA Scenarios** (MANDATORY - task incomplete without these):
  ```
  Scenario: Shorthand guard type-checks without success symbol
    Tool: Bash
    Steps: cargo test --lib type_check_guard_shorthand_discards_success_binding -- --nocapture
    Expected: Test passes and confirms `guard foo() else err =>` type-checks in a function with declared errors/returns.
    Evidence: .sisyphus/evidence/task-4-typecheck-optional-guard-binding.txt

  Scenario: Omitted binding cannot be referenced
    Tool: Bash
    Steps: cargo test --lib type_check_guard_shorthand_success_binding_not_in_scope -- --nocapture
    Expected: Test passes by asserting the compiler rejects a later reference to a success binding that was never declared.
    Evidence: .sisyphus/evidence/task-4-typecheck-optional-guard-binding-error.txt
  ```

  **Commit**: NO | Message: `feat(typecheck): handle omitted guard success bindings` | Files: [`src/type_system/checker/statements.rs`, `src/type_system/tests.rs`, compile-fix consumers]

- [x] 5. Update statement guard codegen to discard omitted success values safely

  **What to do**: Update `src/codegen/statements.rs` so `codegen_guard_statement` accepts `Option<&str>`. Preserve current alloca/env insertion behavior for `Some(success_binding)`. For `None`, still evaluate the guarded expression and branch on error pointer exactly as before, but do not allocate/register the success binding or array metadata variables named from it. For aggregate/error-returning calls, extract the success value only if needed for control-flow correctness; otherwise do not expose it. Add codegen/integration tests proving shorthand guards compile/link/run identically to `into _` for a side-effectful operation and that named bindings still work.
  **Must NOT do**: Do not modify expression guard codegen in `src/codegen/functions_call.rs` except if compile changes are mechanically required. Do not insert env variables with names `_`, `_len`, or `_cap` for omitted bindings.

  **Recommended Agent Profile**:
  - Category: `unspecified-high` - Reason: LLVM/codegen control flow must preserve runtime semantics.
  - Skills: [] - no specialized skill needed.
  - Omitted: [`frontend-ui-ux`] - no UI/browser work.

  **Parallelization**: Can Parallel: NO | Wave 2 | Blocks: [8, 9] | Blocked By: [3, 4]

  **References** (executor has NO interview context - be exhaustive):
  - Pattern: `src/codegen/statements.rs:69-82` - statement dispatch passes `success_binding.as_str()` today.
  - Pattern: `src/codegen/statements.rs:428-633` - `codegen_guard_statement` currently allocates/registers success binding unconditionally and adds array metadata vars.
  - Pattern: `src/codegen/functions_call.rs:507-611` - expression guard lowering, intentionally out of scope for shorthand.
  - Test pattern: `tests/integration_e2e/tests.rs:51-92` - inline compile/link/run smoke test style.
  - Test pattern: `tests/integration_e2e/project_execution.rs` - project execution patterns if a fixture project is preferred.

  **Acceptance Criteria** (agent-executable only):
  - [ ] `cargo test --features integration guard_shorthand_compiles_links_and_runs -- --nocapture` exits 0.
  - [ ] `cargo test --features integration guard_named_binding_still_compiles_links_and_runs -- --nocapture` exits 0.
  - [ ] `cargo test --lib codegen_guard_shorthand_does_not_register_underscore_metadata -- --nocapture` exits 0 if a suitable unit-level env inspection pattern exists; otherwise record why integration coverage is the available executable check.
  - [ ] Evidence written to `.sisyphus/evidence/task-5-codegen-guard-shorthand.txt`.

  **QA Scenarios** (MANDATORY - task incomplete without these):
  ```
  Scenario: Shorthand guard compiles and runs on host
    Tool: Bash
    Steps: cargo test --features integration guard_shorthand_compiles_links_and_runs -- --nocapture
    Expected: Test passes; executable exits 0 and expected stdout marker confirms the guard success path and else path behavior.
    Evidence: .sisyphus/evidence/task-5-codegen-guard-shorthand.txt

  Scenario: Named guard codegen remains valid
    Tool: Bash
    Steps: cargo test --features integration guard_named_binding_still_compiles_links_and_runs -- --nocapture
    Expected: Test passes and named success binding remains usable after guard.
    Evidence: .sisyphus/evidence/task-5-codegen-guard-shorthand-error.txt
  ```

  **Commit**: NO | Message: `feat(codegen): discard omitted guard success values` | Files: [`src/codegen/statements.rs`, `tests/integration_e2e/tests.rs` or new integration module]

- [x] 6. Update formatter and naming checks for optional guard success bindings

  **What to do**: Update `src/formatter/printer.rs` so `Stmt::Guard { success_binding: None, ... }` prints `guard <expr> else <err> =>`, while `Some(name)` prints `guard <expr> into <name> else <err> =>`. Update `src/formatter/naming.rs` so success-binding snake_case checks run only for `Some(name)`; error binding checks remain unchanged. Add formatter round-trip tests for shorthand and explicit `into _`, and naming tests proving no violation is emitted for omitted success binding.
  **Must NOT do**: Do not make the formatter auto-rewrite explicit `into _` to shorthand; the requested rewrite is a final test-project cleanup, not formatter policy.

  **Recommended Agent Profile**:
  - Category: `quick` - Reason: localized formatting/naming consumer updates once AST is optional.
  - Skills: [] - no specialized skill needed.
  - Omitted: [`artistry`] - no creative design required.

  **Parallelization**: Can Parallel: YES | Wave 2 | Blocks: [10] | Blocked By: [2]

  **References** (executor has NO interview context - be exhaustive):
  - Pattern: `src/formatter/printer.rs:687-710` - statement guard printer currently always emits `into {success_binding}`.
  - Pattern: `src/formatter/printer.rs:943-958` - expression guard printer must remain unchanged and always emit `into`.
  - Pattern: `src/formatter/naming.rs:253-297` - guard statement naming check currently validates success and error bindings unconditionally.
  - Existing formatter tests: search in `src/formatter` and `tests/fmt_integration.rs` for round-trip/check patterns before editing.

  **Acceptance Criteria** (agent-executable only):
  - [ ] `cargo test --lib formatter_guard_shorthand_roundtrip -- --nocapture` exits 0 and shorthand output is byte-identical to expected shorthand.
  - [ ] `cargo test --lib formatter_guard_into_underscore_preserved -- --nocapture` exits 0 and explicit `into _` remains explicit.
  - [ ] `cargo test --lib naming_guard_omitted_success_binding_has_no_violation -- --nocapture` exits 0.
  - [ ] Evidence written to `.sisyphus/evidence/task-6-formatter-naming-guard-shorthand.txt`.

  **QA Scenarios** (MANDATORY - task incomplete without these):
  ```
  Scenario: Formatter preserves shorthand
    Tool: Bash
    Steps: cargo test --lib formatter_guard_shorthand_roundtrip -- --nocapture
    Expected: Test passes; formatted output contains `guard foo() else err =>` and does not contain `into _`.
    Evidence: .sisyphus/evidence/task-6-formatter-naming-guard-shorthand.txt

  Scenario: Formatter preserves explicit legacy syntax
    Tool: Bash
    Steps: cargo test --lib formatter_guard_into_underscore_preserved -- --nocapture
    Expected: Test passes; formatted output still contains `guard foo() into _ else err =>`.
    Evidence: .sisyphus/evidence/task-6-formatter-naming-guard-shorthand-error.txt
  ```

  **Commit**: NO | Message: `fix(formatter): print omitted guard bindings as shorthand` | Files: [`src/formatter/printer.rs`, `src/formatter/naming.rs`, formatter/naming tests]

- [x] 7. Add dedicated ambiguous-guard test project and compile-failure integration coverage

  **What to do**: Create `test-projects/ambiguous-guard-if/` following README conventions with `opal.toml`, `.gitignore`, `README.md`, and `src/main.op`. The source must intentionally contain a bare ambiguous guarded if-expression that should fail with `GuardAmbiguousIfElse`, plus a nearby valid declaration to prove recovery. Add a test in `tests/integration_e2e/compile_failures.rs` and register any new module only if needed in `tests/integration_e2e/tests.rs`. Assert the compilation report contains `CompilerError::Parser(ParseError::GuardAmbiguousIfElse { .. })` and rendered output contains the miette help. Add a second valid project or inline integration source for the parenthesized-if form if not already covered by unit tests.
  **Must NOT do**: Do not make this a manual screenshot/snapshot-only test. Do not assert only that compilation failed; assert the specific parser diagnostic.

  **Recommended Agent Profile**:
  - Category: `unspecified-high` - Reason: integration fixture + diagnostic assertions require accurate project conventions.
  - Skills: [] - no specialized skill needed.
  - Omitted: [`frontend-ui-ux`] - no browser/UI work.

  **Parallelization**: Can Parallel: YES | Wave 2 | Blocks: [8] | Blocked By: [2, 3]

  **References** (executor has NO interview context - be exhaustive):
  - Pattern: `README.md` Test Project Conventions - project layout: `opal.toml`, `.gitignore`, `README.md`, `src/main.op`.
  - Pattern: `tests/integration_e2e/compile_failures.rs:57-135` - compile-failure test that inspects `CompileError::Report` entries.
  - Pattern: `tests/integration_e2e/tests.rs:5-49` - integration module registration.
  - API/Type: `tests/integration_e2e.rs:3-8` - imported `CompileError`, `CompilerError`, and compile helpers available to modules.

  **Acceptance Criteria** (agent-executable only):
  - [ ] `cargo test --features integration ambiguous_guard_if_project_fails_with_miette_help -- --nocapture` exits 0.
  - [ ] The test asserts a parser diagnostic variant, not just any compile failure.
  - [ ] Rendered diagnostic assertions include `opalescent::parser::guard_ambiguous_if_else` and `parentheses`.
  - [ ] Evidence written to `.sisyphus/evidence/task-7-ambiguous-guard-test-project.txt`.

  **QA Scenarios** (MANDATORY - task incomplete without these):
  ```
  Scenario: Ambiguous guard test project fails with correct diagnostic
    Tool: Bash
    Steps: cargo test --features integration ambiguous_guard_if_project_fails_with_miette_help -- --nocapture
    Expected: Test passes and confirms the compile report contains the dedicated parser diagnostic with parentheses help.
    Evidence: .sisyphus/evidence/task-7-ambiguous-guard-test-project.txt

  Scenario: Parser recovery avoids cascading integration failures
    Tool: Bash
    Steps: cargo test --features integration ambiguous_guard_if_project_fails_with_miette_help -- --nocapture
    Expected: Test asserts exactly one relevant parser error for the fixture or records any additional errors as a failure.
    Evidence: .sisyphus/evidence/task-7-ambiguous-guard-test-project-error.txt
  ```

  **Commit**: NO | Message: `test(guard): cover ambiguous guard if project diagnostics` | Files: [`test-projects/ambiguous-guard-if/**`, `tests/integration_e2e/compile_failures.rs`, `tests/integration_e2e/tests.rs` if needed]

- [x] 8. Add host integration project coverage for shorthand guard success and error paths

  **What to do**: Add a valid `test-projects/guard-shorthand/` fixture or inline integration source that uses `guard <call> else <err> =>` for discarded success values and also keeps one explicit `guard <call> into value else <err> =>` to prove named success binding still works. The project must exercise both a success path and an error path with deterministic stdout markers. Add an integration test in `tests/integration_e2e/` that compiles, links, runs, captures stdout, and asserts exact markers. If a new integration module file is added, register it in `tests/integration_e2e/tests.rs`.
  **Must NOT do**: Do not depend on interactive input. Do not use nondeterministic filesystem paths without cleanup. Do not skip asserting the runtime output.

  **Recommended Agent Profile**:
  - Category: `unspecified-high` - Reason: project fixture + compile/link/run integration coverage.
  - Skills: [] - no specialized skill needed.
  - Omitted: [`playwright`] - no browser work.

  **Parallelization**: Can Parallel: NO | Wave 3 | Blocks: [9, 10] | Blocked By: [3, 5, 7]

  **References** (executor has NO interview context - be exhaustive):
  - Pattern: `tests/integration_e2e/tests.rs:51-92` - compile/link/run smoke test style.
  - Pattern: `tests/integration_e2e/project_execution.rs` - project execution assertions and stdout capture style.
  - Pattern: `test-projects/hello-world/` and README Test Project Conventions - fixture structure.
  - Pattern: `test-projects/fs-directory-operations/src/main.op` - existing real-world `guard ... into _ else ...` usage to mirror with shorthand.

  **Acceptance Criteria** (agent-executable only):
  - [ ] `cargo test --features integration guard_shorthand_project_compiles_links_and_runs -- --nocapture` exits 0.
  - [ ] Runtime stdout contains `GUARD_SHORTHAND_SUCCESS=ok` and `GUARD_SHORTHAND_ERROR=handled` or equivalent exact markers defined in the test.
  - [ ] The fixture contains at least one shorthand guard and one named-binding guard.
  - [ ] Evidence written to `.sisyphus/evidence/task-8-host-guard-shorthand-project.txt`.

  **QA Scenarios** (MANDATORY - task incomplete without these):
  ```
  Scenario: Host integration success and error paths
    Tool: Bash
    Steps: cargo test --features integration guard_shorthand_project_compiles_links_and_runs -- --nocapture
    Expected: Test passes; stdout markers prove shorthand guard success path continues and error path enters the else handler.
    Evidence: .sisyphus/evidence/task-8-host-guard-shorthand-project.txt

  Scenario: Named guard remains usable in integration fixture
    Tool: Bash
    Steps: cargo test --features integration guard_shorthand_project_compiles_links_and_runs -- --nocapture
    Expected: Test passes and asserts a marker derived from a named success binding after an explicit `into value` guard.
    Evidence: .sisyphus/evidence/task-8-host-guard-shorthand-project-error.txt
  ```

  **Commit**: NO | Message: `test(guard): add host integration coverage for shorthand guards` | Files: [`test-projects/guard-shorthand/**`, `tests/integration_e2e/guard_shorthand.rs`, `tests/integration_e2e/tests.rs`]

- [x] 9. Add Windows/Wine validation for guard shorthand

  **What to do**: Extend the existing Windows/Wine integration harness with a new `wine_msvc_guard_shorthand` test that builds the valid `test-projects/guard-shorthand/` project for `x86_64-pc-windows-msvc`, runs it under Wine, and asserts the same stdout markers as host integration. Use `scripts/verify-wine-prereqs.sh`/`skip_if_prereqs_missing` style already present. Capture deterministic evidence whether Wine runs or prereqs skip. Ensure `cargo build --release` prerequisite is included in the documented command because the harness expects `target/release/opalescent`.
  **Must NOT do**: Do not weaken existing Wine tests. Do not treat a Wine host limitation as success unless the existing harness records it as a skip with evidence. Do not require manual Wine interaction.

  **Recommended Agent Profile**:
  - Category: `unspecified-high` - Reason: cross-target validation and environment-gated integration test.
  - Skills: [] - no specialized skill needed.
  - Omitted: [`frontend-ui-ux`] - no UI work.

  **Parallelization**: Can Parallel: NO | Wave 3 | Blocks: [10] | Blocked By: [5, 8]

  **References** (executor has NO interview context - be exhaustive):
  - Pattern: `tests/integration_e2e/windows_wine.rs:44-70` - prereq check via `scripts/verify-wine-prereqs.sh`.
  - Pattern: `tests/integration_e2e/windows_wine.rs:72-123` - `build_opal_project(project, target)` uses `target/release/opalescent build --target ...`.
  - Pattern: `tests/integration_e2e/windows_wine.rs:667-734` - `wine_msvc_file_ops` test structure, skip handling, build, run, evidence capture, marker assertions.
  - Command docs: `README.md` Building Windows Programs and Testing with Wine sections.

  **Acceptance Criteria** (agent-executable only):
  - [ ] `bash scripts/verify-wine-prereqs.sh | tee .sisyphus/evidence/task-9-wine-prereqs.txt` executed and evidence captured.
  - [ ] If prereqs output `OK:`, `cargo build --release && cargo test --features "integration windows-wine" --test integration_e2e -- --nocapture wine_msvc_guard_shorthand` exits 0.
  - [ ] If prereqs output `SKIP:`, `.sisyphus/evidence/task-9-wine-guard-shorthand.txt` records the exact skip reason and `cargo test --features integration guard_shorthand_project_compiles_links_and_runs -- --nocapture` still exits 0.
  - [ ] Evidence written to `.sisyphus/evidence/task-9-wine-guard-shorthand.txt`.

  **QA Scenarios** (MANDATORY - task incomplete without these):
  ```
  Scenario: Wine prereq-gated Windows build/run
    Tool: Bash
    Steps: bash scripts/verify-wine-prereqs.sh; if OK, cargo build --release && cargo test --features "integration windows-wine" --test integration_e2e -- --nocapture wine_msvc_guard_shorthand
    Expected: If prereqs are OK, test passes and stdout markers match host integration. If prereqs are SKIP, exact skip reason is recorded as evidence.
    Evidence: .sisyphus/evidence/task-9-wine-guard-shorthand.txt

  Scenario: Host fallback remains green when Wine unavailable
    Tool: Bash
    Steps: cargo test --features integration guard_shorthand_project_compiles_links_and_runs -- --nocapture
    Expected: Test passes even if Wine prereqs are unavailable.
    Evidence: .sisyphus/evidence/task-9-wine-guard-shorthand-error.txt
  ```

  **Commit**: NO | Message: `test(windows): validate guard shorthand under wine` | Files: [`tests/integration_e2e/windows_wine.rs`, `.sisyphus/evidence/task-9-*`]

- [x] 10. Rewrite test-project `guard ... into _ else ... =>` statements to shorthand after green validation

  **What to do**: After Tasks 1-9 are green, rewrite only `test-projects/` occurrences of `guard <expr> into _ else <err> =>` to `guard <expr> else <err> =>`. Initial grep found 55 matches across 8 files: `test-projects/fs-directory-operations/src/main.op` (30), `test-projects/windows-file-ops/src/main.op` (7), `test-projects/_fs_dir_inventory/src/main.op` (6), `test-projects/_fs_write_text_atomic/src/main.op` (6), `test-projects/delete-downloads/src/main.op` (2), `test-projects/move-downloads/src/main.op` (2), `test-projects/fs-markdown-roundtrip/src/main.op` (1), `test-projects/op-cat/src/main.op` (1). Use a parser-aware or carefully reviewed textual replacement; verify no `into _ else` remains under `test-projects/`. Run integration tests for every touched fixture plus full integration suite.
  **Must NOT do**: Do not rewrite `language-spec/`, `README.md`, source code examples, or non-test-project files in this task. Do not rewrite named bindings or `into _` outside statement guards.

  **Recommended Agent Profile**:
  - Category: `unspecified-high` - Reason: broad fixture rewrite with many integration tests.
  - Skills: [] - no specialized skill needed.
  - Omitted: [`ai-slop-remover`] - this is not AI slop cleanup.

  **Parallelization**: Can Parallel: NO | Wave 3 | Blocks: [11] | Blocked By: [6, 8, 9]

  **References** (executor has NO interview context - be exhaustive):
  - Search result: `grep -R "into _ else" test-projects/` found 55 matches in the 8 files listed above.
  - Pattern: `test-projects/fs-directory-operations/src/main.op` - largest cluster of old syntax.
  - Pattern: `test-projects/windows-file-ops/src/main.op` - must remain Windows-compatible because Wine test uses this fixture.
  - Verification: `tests/integration_e2e/tests.rs:5-49` lists integration modules for many touched fixtures.

  **Acceptance Criteria** (agent-executable only):
  - [ ] `grep -R "into _ else" test-projects/` returns no matches; command output recorded.
  - [ ] `cargo test --features integration fs_directory_operations -- --nocapture` exits 0.
  - [ ] Each touched-fixture filter exits 0 when run separately, e.g. `cargo test --features integration fs_write_text_atomic -- --nocapture`, `cargo test --features integration fs_dir_inventory -- --nocapture`, `cargo test --features integration fs_markdown_roundtrip -- --nocapture`, and `cargo test --features integration op_cat -- --nocapture`.
  - [ ] `cargo test --features integration -- --nocapture` exits 0.
  - [ ] Evidence written to `.sisyphus/evidence/task-10-rewrite-test-project-into-underscore.txt`.

  **QA Scenarios** (MANDATORY - task incomplete without these):
  ```
  Scenario: No test-project legacy discard guards remain
    Tool: Bash
    Steps: grep -R "into _ else" test-projects/
    Expected: Command returns non-zero/no matches; evidence records no remaining occurrences.
    Evidence: .sisyphus/evidence/task-10-rewrite-test-project-into-underscore.txt

  Scenario: Touched fixtures still pass integration tests
    Tool: Bash
    Steps: cargo test --features integration -- --nocapture
    Expected: Full integration suite passes after rewriting test projects.
    Evidence: .sisyphus/evidence/task-10-rewrite-test-project-into-underscore-error.txt
  ```

  **Commit**: NO | Message: `refactor(test-projects): use guard shorthand for discarded success values` | Files: [`test-projects/fs-directory-operations/src/main.op`, `test-projects/windows-file-ops/src/main.op`, `test-projects/_fs_dir_inventory/src/main.op`, `test-projects/_fs_write_text_atomic/src/main.op`, `test-projects/delete-downloads/src/main.op`, `test-projects/move-downloads/src/main.op`, `test-projects/fs-markdown-roundtrip/src/main.op`, `test-projects/op-cat/src/main.op`]

- [x] 11. Run full regression, update narrow docs/spec examples, and capture final evidence

  **What to do**: Run full regression after all implementation and cleanup tasks. Update only narrow syntax documentation/examples if present in README/language-spec so the language docs mention both explicit named guards and shorthand discarded-success guards, plus the parentheses requirement for ambiguous guarded if-expressions. Keep doc edits minimal and factual. Run formatting/check/test commands and capture outputs. If `cargo test --all-features` is too broad or environment-dependent, still run it and record exact failures; fix code/test failures, but record environment-only Wine skips separately.
  **Must NOT do**: Do not broaden docs into a full language redesign narrative. Do not mark final verification tasks complete before user approval after review agents run.

  **Recommended Agent Profile**:
  - Category: `deep` - Reason: final cross-cutting regression and documentation consistency requires broad verification.
  - Skills: [] - no specialized skill needed.
  - Omitted: [`frontend-ui-ux`] - no UI/browser work.

  **Parallelization**: Can Parallel: NO | Wave 4 | Blocks: [Final Verification] | Blocked By: [10]

  **References** (executor has NO interview context - be exhaustive):
  - Docs: `README.md` Error Handling section currently documents `guard ... into ... else`.
  - Docs: `language-spec/error_handling_samples.op` likely contains canonical guard examples.
  - Commands: `Cargo.toml` feature flags `integration`, `windows-wine`; `Makefile.toml` test task patterns; `.github/workflows/ci.yml` CI expectations.

  **Acceptance Criteria** (agent-executable only):
  - [ ] `cargo fmt --check` exits 0.
  - [ ] `cargo test --lib` exits 0.
  - [ ] `cargo test --features integration` exits 0.
  - [ ] `cargo build --release && cargo test --all-features` exits 0 or produces only documented environment-gated Wine skips with evidence.
  - [ ] `grep -R "guard .*into _ else" test-projects/` returns no matches.
  - [ ] Evidence written to `.sisyphus/evidence/task-11-final-regression-guard-shorthand.txt`.

  **QA Scenarios** (MANDATORY - task incomplete without these):
  ```
  Scenario: Full Rust regression
    Tool: Bash
    Steps: cargo fmt --check && cargo test --lib && cargo test --features integration && cargo build --release && cargo test --all-features
    Expected: Commands exit 0, except documented environment-gated Wine skips are captured with exact skip reason.
    Evidence: .sisyphus/evidence/task-11-final-regression-guard-shorthand.txt

  Scenario: Documentation/examples mention shorthand accurately
    Tool: Bash
    Steps: grep -R "guard .* else .*=>" README.md language-spec/ test-projects/guard-shorthand test-projects/ambiguous-guard-if
    Expected: Output includes shorthand examples and no stale claim that statement guards always require `into` for discarded success values.
    Evidence: .sisyphus/evidence/task-11-final-regression-guard-shorthand-error.txt
  ```

  **Commit**: NO | Message: `docs(guard): document shorthand guard syntax` | Files: [`README.md`, `language-spec/error_handling_samples.op` if needed, `.sisyphus/evidence/task-11-*`]

## Final Verification Wave (MANDATORY — after ALL implementation tasks)
> 4 review agents run in PARALLEL. ALL must APPROVE. Present consolidated results to user and get explicit "okay" before completing.
> **Do NOT auto-proceed after verification. Wait for user's explicit approval before marking work complete.**
> **Never mark F1-F4 as checked before getting user's okay.** Rejection or user feedback -> fix -> re-run -> present again -> wait for okay.
- [x] F1. Plan Compliance Audit — oracle
- [x] F2. Code Quality Review — unspecified-high
- [x] F3. Real Manual QA — unspecified-high (+ playwright if UI)
- [x] F4. Scope Fidelity Check — deep
## Commit Strategy
- Commit 1: `test(parser): capture optional guard into expectations` — parser/diagnostic failing tests and fixtures only, if repository workflow permits committing failing TDD tests separately.
- Commit 2: `feat(parser): support omitted guard success binding` — AST/parser/diagnostic implementation and parser tests.
- Commit 3: `feat(guard): propagate optional guard bindings through semantics` — type checker, codegen, formatter, naming, LSP updates and unit tests.
- Commit 4: `test(guard): add integration and wine coverage` — test projects and integration/Wine harness.
- Commit 5: `refactor(test-projects): use guard shorthand for discarded success values` — final `test-projects/` cleanup only.
- If the repository expects all tests green per commit, squash Commit 1 into Commit 2 before committing; do not leave failing tests on a shared branch.

## Success Criteria
- New shorthand works in statement guards for success-discarding cases.
- Named success-binding guards remain unchanged.
- Expression guards still require `into`.
- Ambiguous guarded `if` forms produce one helpful parser/miette error with parentheses guidance.
- Linux, integration, and Windows/Wine validation are green or environment-gated with recorded evidence.
- `test-projects/` no longer contains `into _ else` after final cleanup.
