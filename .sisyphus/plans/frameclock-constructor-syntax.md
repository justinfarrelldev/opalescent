# Generalized Fallible Constructor Expressions

## TL;DR
> **Summary**: Introduce an extensible fallible-constructor language feature so `propagate new FrameClock:` works through a generalized registry-backed fallible expression path, not a FrameClock special case. FrameClock is the first runtime-backed fallible constructor entry; future language/runtime elements can opt in by adding registry entries and stdlib declarations.
> **Deliverables**:
> - Parser accepts `propagate new <Type>:` while still rejecting ordinary non-call/non-constructor propagation targets.
> - Parser explicitly verifies `guard new <Type>:` constructor field-block disambiguation: guard `else` must appear after the constructor block dedents to guard level.
> - Typechecker resolves fallible calls and registered fallible constructors through a shared `FallibleExprInfo`-style helper.
> - Registry maps resolved canonical constructor result types to runtime symbols, named field order/types, return type, error types, and ABI lowering descriptor.
> - Codegen lowers registered fallible constructors through existing error ABI and runtime stdlib declarations.
> - FrameClock test projects use `propagate new FrameClock:` / `guard new FrameClock:` and all current test projects pass.
> - Imported/aliased ordinary constructors such as `new Account:` remain unchanged.
> **Effort**: Large
> **Parallel**: YES - 6 waves
> **Critical Path**: Task 1 → Task 2 → Task 3 → Task 4 → Task 5 → Tasks 6-10 → Task 12 → Task 14

## Context
### Original Request
- User dislikes source-level `frame_clock_new` usage and requires:
  ```opalescent
  let frameclock = propagate new FrameClock:
      frames_per_second: 10
  ```
- Use TDD with RED-GREEN-REFACTOR.
- All current test projects must pass by the end.
- Commit final changes and fix all issues caught by pre-commit.
- User later clarified this must be generalized and extensible, not FrameClock-only: “this is an entirely new dedicated language feature.”
- User explicitly requested Momus review after plan rewrite.

### Interview Summary
- User rejected the prior FrameClock-only design.
- Decision: implement a generalized, registry-backed fallible constructor feature.
- Decision: FrameClock is the first registered entry and must reuse existing `frame_clock_new` runtime ABI.
- Decision: preserve existing ordinary constructor semantics, especially imported/aliased type constructor behavior from `test-projects/import-types-aliased/src/main.op`:
  ```opalescent
  import type User as Account from ./models.types
  let bob: Account = new Account:
      id: 42
      display_name: 'Bob'
  ```

### Research Findings
- `src/parser/new_expression.rs:21-139` parses `new Type:` field-block constructor expressions into `Expr::Constructor`.
- `src/parser/expressions.rs:241-263` rejects `propagate` targets that are not `Expr::Call`; this parser gate must become `Expr::Call | Expr::Constructor`.
- `src/parser/tests.rs:376-394` pins call propagation and ordinary non-call rejection; add constructor acceptance while preserving `propagate 1 + 2` rejection.
- `src/ast.rs` stores `Expr::Propagate { call: Box<Expr>, ... }`; field name is call-shaped but can already hold any `Expr`, so no AST enum change is required unless executor finds docs needing update.
- `src/type_system/checker/expressions.rs:364-470` typechecks propagation by extracting `CoreType::Function` metadata from a call callee; this must delegate to a shared fallible-expression classifier.
- `src/type_system/checker/expressions_guard.rs:77-99` and `164-180` typecheck guard subjects as calls; this must delegate to the same classifier.
- `src/type_system/checker/constructors.rs:19-203` validates ordinary product/sum constructors; generalized fallible constructors must not break this path.
- `src/type_system/propertyless_constructors.rs` provides a small static registry pattern for propertyless runtime constructors; use as style reference, but introduce a separate fallible-constructor registry because fallible constructors have field schemas, error types, runtime symbols, and ABI descriptors.
- `src/type_system/checker/time_builtins.rs:22-54` registers `FrameClock`, `InvalidFrameRateError`, `frame_clock_new`, and `frame_clock_wait_next_sync`; registry should reference these canonical names.
- `src/codegen/error_abi.rs` defines canonical error ABI and error-field index rules.
- `src/codegen/functions_call.rs:410-504` already lowers propagation by inspecting error-bearing aggregate values; keep this generic behavior.
- `src/codegen/adts.rs:28-77` lowers constructors and currently has propertyless constructor runtime calls; add registry-backed fallible constructor lowering here.
- `src/codegen/functions_stdlib.rs` already declares `frame_clock_new`; do not change runtime ABI.
- `tests/integration_e2e/project_execution.rs` contains `import_types_aliased_compiles_and_runs`; make it a regression gate.
- `tests/integration_e2e/time_stdlib.rs:155-309` covers frame-clock integration behavior; split invalid-fps tests into direct per-project gates.

### Oracle Design Review (incorporated)
- Keep `new Type:` as constructor syntax, not disguised function calls.
- Add a small runtime-backed fallible constructor registry plus fallible-expression classifier.
- Keep `CoreType` as value type only; fallibility belongs in expression-analysis metadata.
- Key registry lookups off resolved canonical type identity, not raw syntax, to avoid alias bugs.
- Typechecker should be nominal/registered; do not bless arbitrary two-field structs as fallible just because codegen could branch on them.
- Lower registered constructors through existing runtime functions and error ABI.

### Metis Review (incorporated)
- Treat as Build-from-Scratch/extensible language feature with parser/typechecker/codegen refactoring overlay.
- Required abstraction: `FallibleConstructorRegistry` or equivalent; FrameClock is first entry, not a hardcoded branch.
- Must include test-only second registered constructor to prove extensibility and catch FrameClock coupling.
- Must include alias/import regression for `new Account:`.
- Must use per-layer RED-GREEN-REFACTOR.

## Work Objectives
### Core Objective
Implement an extensible language-level mechanism for registered fallible constructor expressions, initially enabling `propagate new FrameClock:` and `guard new FrameClock:` while preserving ordinary constructors and alias/import behavior.

### Deliverables
- Parser support for `propagate` wrapping constructor expressions.
- Parser/formatting tests for `guard new <Type>:` with constructor-block colon followed by aligned guard `else`.
- `FallibleConstructorRegistry` (or equivalent name) with FrameClock and test-only second entry.
- Shared fallible-expression classifier returning success type, error types, and lowering metadata for calls and registered constructors.
- Typechecker support for `propagate`/`guard` over any registered fallible expression.
- Codegen lowering for registered fallible constructors via runtime symbols and existing error ABI.
- Migrated frame-clock test projects using new syntax.
- Regression coverage for imported type alias constructor fixture.
- Full test/check/pre-commit/commit task.

### Definition of Done (verifiable conditions with commands)
- `cargo test --lib parse_propagate_accepts_new_constructor_expression` exits 0.
- `cargo test --lib parse_statement_guard_accepts_new_constructor_with_aligned_else` exits 0.
- `cargo test --lib parse_expression_guard_accepts_new_constructor_with_into_after_block` exits 0.
- `cargo test --lib parse_statement_guard_rejects_misindented_constructor_else` exits 0.
- `cargo test --lib test_parse_propagate_rejects_non_call` exits 0.
- `cargo test --lib fallible_constructor_registry_registers_frameclock` exits 0.
- `cargo test --lib fallible_constructor_registry_supports_test_second_entry` exits 0.
- `cargo test --lib propagate_new_frameclock_typechecks_via_registry` exits 0.
- `cargo test --lib guard_new_frameclock_typechecks_via_registry` exits 0.
- `cargo test --lib propagate_new_nonfallible_constructor_reports_diagnostic` exits 0.
- `cargo test --lib codegen_new_frameclock_uses_registered_runtime_symbol` exits 0.
- `cargo test --features integration --test integration_e2e -- --nocapture frame_clock_30fps_ten_waits_timing` exits 0.
- `cargo test --features integration --test integration_e2e -- --nocapture frame_clock_rejects_zero_fps` exits 0.
- `cargo test --features integration --test integration_e2e -- --nocapture frame_clock_rejects_negative_fps` exits 0.
- `cargo test --features integration --test integration_e2e -- --nocapture frame_clock_rejects_invalid_fps` exits 0.
- `cargo test --features integration --test integration_e2e -- --nocapture import_types_aliased_compiles_and_runs` exits 0.
- `timeout 900 cargo test --all-features` exits 0.
- `cargo clippy --all-targets --all-features -- -D warnings` exits 0.
- `cargo fmt --all -- --check` exits 0.
- `grep -R "frame_clock_new(" test-projects/frame-clock-*/src --include='*.op'` exits 1.
- Final commit exists and `git status --porcelain --untracked-files=no` is empty.

### Must Have
- Generalized registry-backed design; no FrameClock-only implementation logic outside registry entries and tests.
- Parser accepts `propagate new <Type>:` for constructor AST targets.
- Parser accepts `guard new <Type>:\n    field: value\nelse err => ...` only when `else` is dedented to guard level after the constructor field block.
- Parser rejects or clearly errors on `guard new <Type>:` where `else` remains indented inside the constructor field block.
- Typechecker rejects `propagate new <NonRegisteredType>:` with exact diagnostic substring `does not have a fallible constructor`.
- Registered fallible constructors use resolved canonical type identity for lookup; raw aliases must not cause false positives/negatives.
- Ordinary ADT/product constructors and imported/aliased constructors continue to typecheck and run.
- FrameClock registry entry maps `FrameClock` + `frames_per_second: int32` to runtime `frame_clock_new` + `InvalidFrameRateError`.
- `guard new FrameClock:` works for invalid-fps tests so errors can be handled locally.
- Test-only second fallible constructor entry proves extensibility without adding a production runtime API.
- Existing runtime `frame_clock_new` ABI remains unchanged.

### Must NOT Have
- No hardcoded FrameClock logic in parser, shared fallibility classifier, or codegen lowering outside the registry entry/test assertions.
- No change to `runtime/opal_io.c` or `runtime/opal_runtime.h` unless a test proves existing runtime is broken.
- No change to ordinary `new Type:` constructor field-block grammar in `src/parser/new_expression.rs`.
- No acceptance of arbitrary aggregate structs as fallible expressions.
- No broad constructor-system rewrite beyond introducing the registry/classifier needed for this feature.
- No removal of source compatibility for callable `frame_clock_new`; only frame-clock test projects are migrated away from using it.
- No unrelated lint/refactor cleanup beyond changed files.

## Verification Strategy
> ZERO HUMAN INTERVENTION - all verification is agent-executed.
- Test decision: TDD RED-GREEN-REFACTOR per layer: parser → registry → typechecker → codegen → integration migration → full regression.
- QA policy: Every task has executable commands and evidence paths.
- Evidence: `.sisyphus/evidence/task-{N}-{slug}.{ext}`. Remove or ignore generated evidence before final commit unless already tracked.
- Full regression includes all current Rust tests and integration test projects through `--all-features`.
- Commit task must run hooks normally; if hooks fail, fix introduced issues and rerun relevant gates.

## Execution Strategy
### Parallel Execution Waves
> Target: 5-8 tasks per wave. <3 per wave (except final) = under-splitting.
> Extract shared dependencies as Wave-1 tasks for max parallelism.

Wave 1: Task 1 baseline + RED tests.
Wave 2: Tasks 2-5 parser, registry, typechecker, codegen GREEN/REFACTOR on critical path.
Wave 3: Tasks 6-11 integration migrations, alias regression, diagnostics, extensibility proof.
Wave 4: Tasks 12-14 full regression, cleanup, commit.
Wave 5: Final Verification Wave F1-F4.

### Dependency Matrix (full, all tasks)
| Task | Depends On | Blocks |
|---|---|---|
| 1 | None | 2, 3, 4, 5 |
| 2 | 1 | 4, 5, 6, 7 |
| 3 | 1 | 4, 5, 9, 10 |
| 4 | 2, 3 | 5, 6, 7, 8, 9, 10 |
| 5 | 3, 4 | 6, 7, 8, 12 |
| 6 | 5 | 11, 12 |
| 7 | 5 | 11, 12 |
| 8 | 5 | 11, 12 |
| 9 | 4 | 12 |
| 10 | 3, 4, 5 | 12 |
| 11 | 6, 7, 8 | 12 |
| 12 | 9, 10, 11 | 13 |
| 13 | 12 | 14 |
| 14 | 13 | F1-F4 |
| F1-F4 | 14 | Completion |

### Agent Dispatch Summary (wave → task count → categories)
| Wave | Task Count | Categories |
|---|---:|---|
| 1 | 1 | deep |
| 2 | 4 | deep, unspecified-high |
| 3 | 6 | unspecified-high, quick |
| 4 | 3 | unspecified-high, quick |
| 5 | 4 | oracle, Bash |

## TODOs
> Implementation + Test = ONE task. Never separate.
> EVERY task MUST have: Agent Profile + Parallelization + QA Scenarios.

- [x] 1. Capture baseline and add per-layer RED tests

  **What to do**: Capture baseline git/test state, then add failing RED tests for parser, registry, typechecker, codegen, integration migration, alias regression, and diagnostics before implementing feature code. The RED tests must demonstrate current limitations: parser rejects `propagate new`, guard-new constructor block `else` disambiguation is untested, no registry exists, typechecker/guard only understand call-shaped fallible expressions, codegen cannot lower registered fallible constructors, and frame-clock projects still use `frame_clock_new`.
  **Must NOT do**: Do not implement parser/typechecker/codegen/registry support in this task. Do not weaken existing tests to make RED pass.

  **Recommended Agent Profile**:
  - Category: `deep` - Reason: Establishes TDD anchors across compiler pipeline and integration fixtures.
  - Skills: [] - No specialized skill required.
  - Omitted: [`frontend-ui-ux`] - No UI work.

  **Parallelization**: Can Parallel: NO | Wave 1 | Blocks: [2, 3, 4, 5] | Blocked By: []

  **References**:
  - Parser: `src/parser/expressions.rs:241-263` - current call-only `propagate` gate.
  - Parser tests: `src/parser/tests.rs:376-394` - current propagate call/non-call tests.
  - Registry pattern: `src/type_system/propertyless_constructors.rs` - small static constructor registry style.
  - Typechecker: `src/type_system/checker/expressions.rs:364-470` - call-shaped propagate typechecking.
  - Guard checker: `src/type_system/checker/expressions_guard.rs:77-99` - guard subject call shape.
  - Codegen: `src/codegen/adts.rs:28-77` - constructor lowering entry.
  - Integration: `tests/integration_e2e/time_stdlib.rs:155-309`, `tests/integration_e2e/project_execution.rs`.

  **Acceptance Criteria**:
  - [ ] `git status --porcelain > .sisyphus/evidence/task-1-baseline-status.txt` captures baseline state.
  - [ ] `cargo test --lib parse_propagate_accepts_new_constructor_expression 2>&1 | tee .sisyphus/evidence/task-1-parser-red.log` exits non-zero.
- [ ] `cargo test --lib parse_statement_guard_accepts_new_constructor_with_aligned_else 2>&1 | tee .sisyphus/evidence/task-1-guard-new-red.log` exits non-zero or is newly added as RED.
- [ ] `cargo test --lib parse_statement_guard_rejects_misindented_constructor_else 2>&1 | tee .sisyphus/evidence/task-1-guard-new-negative-red.log` exits non-zero or is newly added as RED.
  - [ ] `cargo test --lib fallible_constructor_registry_registers_frameclock 2>&1 | tee .sisyphus/evidence/task-1-registry-red.log` exits non-zero.
  - [ ] `cargo test --lib propagate_new_frameclock_typechecks_via_registry 2>&1 | tee .sisyphus/evidence/task-1-typecheck-red.log` exits non-zero.
  - [ ] `cargo test --lib codegen_new_frameclock_uses_registered_runtime_symbol 2>&1 | tee .sisyphus/evidence/task-1-codegen-red.log` exits non-zero.
  - [ ] `cargo test --features integration --test integration_e2e -- --nocapture import_types_aliased_compiles_and_runs 2>&1 | tee .sisyphus/evidence/task-1-alias-baseline.log` exits 0 before feature work.

  **QA Scenarios**:
  ```
  Scenario: Parser RED proves current syntax gate
    Tool: Bash
    Steps: Run `cargo test --lib parse_propagate_accepts_new_constructor_expression 2>&1 | tee .sisyphus/evidence/task-1-parser-red.log` and `cargo test --lib parse_statement_guard_accepts_new_constructor_with_aligned_else 2>&1 | tee .sisyphus/evidence/task-1-guard-new-red.log`.
    Expected: Commands exit non-zero or newly added REDs fail because `propagate new` and guard-new constructor disambiguation are not yet supported/tested.
    Evidence: .sisyphus/evidence/task-1-parser-red.log and .sisyphus/evidence/task-1-guard-new-red.log

  Scenario: Alias fixture baseline still passes before feature work
    Tool: Bash
    Steps: Run `cargo test --features integration --test integration_e2e -- --nocapture import_types_aliased_compiles_and_runs 2>&1 | tee .sisyphus/evidence/task-1-alias-baseline.log`.
    Expected: Command exits 0, proving `new Account:` baseline works before changes.
    Evidence: .sisyphus/evidence/task-1-alias-baseline.log
  ```

  **Commit**: NO | Message: N/A | Files: [N/A]

- [x] 2. Parser GREEN/REFACTOR for constructor propagation

  **What to do**: Update `src/parser/expressions.rs:241-263` so `parse_propagate_expression` accepts `Expr::Call` and `Expr::Constructor` targets. Keep ordinary non-call rejection for expressions like `propagate 1 + 2`. Add/update parser tests in `src/parser/tests.rs` with exact AST assertions: `Expr::Propagate` wrapping `Expr::Constructor` whose callee is `FrameClock` and field `frames_per_second` is integer literal `10`. Add explicit guard-new parser tests for the constructor-colon/guard-else ambiguity: statement guard shorthand accepts `guard new FrameClock:\n    frames_per_second: 0\nelse err =>\n    return void`, expression guard accepts `guard new FrameClock:\n    frames_per_second: 0\ninto clock else fallback`, and misindented `else` inside the constructor block is rejected with a deterministic parse error. Update AST docs/comments if they claim `Propagate.call` must be call-only, but do not change AST shape unless compilation requires it.
  **Must NOT do**: Do not parse arbitrary statements after `propagate`. Do not change `src/parser/new_expression.rs` constructor field-block grammar. Do not require `guard new` users to parenthesize the constructor; use indentation/dedent tests to disambiguate.

  **Recommended Agent Profile**:
  - Category: `unspecified-high` - Reason: Focused parser grammar expansion with AST regression coverage.
  - Skills: []
  - Omitted: [`frontend-ui-ux`] - No UI work.

  **Parallelization**: Can Parallel: NO | Wave 2 | Blocks: [4, 5, 6, 7] | Blocked By: [1]

  **References**:
  - `src/parser/expressions.rs:241-263` - change call-only acceptance to call-or-constructor.
  - `src/parser/new_expression.rs:21-139` - constructor parser already exists.
  - `src/parser/tests.rs:376-394` - existing propagate parser tests to extend.
  - `src/parser/tests.rs:534-641` - statement guard parse patterns to extend for constructor subjects.
  - `src/parser/tests.rs:326-374` - expression guard parse patterns to extend for constructor subjects with `into`.
  - `src/parser/tests.rs:6004-6338` - constructor AST assertion patterns.

  **Acceptance Criteria**:
  - [ ] `cargo test --lib parse_propagate_accepts_new_constructor_expression 2>&1 | tee .sisyphus/evidence/task-2-parser-green.log` exits 0.
- [ ] `cargo test --lib parse_statement_guard_accepts_new_constructor_with_aligned_else 2>&1 | tee .sisyphus/evidence/task-2-guard-statement-green.log` exits 0.
- [ ] `cargo test --lib parse_expression_guard_accepts_new_constructor_with_into_after_block 2>&1 | tee .sisyphus/evidence/task-2-guard-expression-green.log` exits 0.
- [ ] `cargo test --lib parse_statement_guard_rejects_misindented_constructor_else 2>&1 | tee .sisyphus/evidence/task-2-guard-misindent.log` exits 0.
- [ ] `cargo test --lib test_parse_propagate_with_call 2>&1 | tee .sisyphus/evidence/task-2-call-regression.log` exits 0.
- [ ] `cargo test --lib test_parse_propagate_rejects_non_call 2>&1 | tee .sisyphus/evidence/task-2-non-call-regression.log` exits 0.

  **QA Scenarios**:
  ```
  Scenario: Constructor propagation parses
    Tool: Bash
    Steps: Run `cargo test --lib parse_propagate_accepts_new_constructor_expression 2>&1 | tee .sisyphus/evidence/task-2-parser-green.log`.
    Expected: Command exits 0 and AST assertion confirms `Propagate(Constructor(FrameClock, frames_per_second=10))`.
    Evidence: .sisyphus/evidence/task-2-parser-green.log

  Scenario: Guard-new constructor block disambiguates else by dedent
    Tool: Bash
    Steps: Run `cargo test --lib parse_statement_guard_accepts_new_constructor_with_aligned_else 2>&1 | tee .sisyphus/evidence/task-2-guard-statement-green.log`, `cargo test --lib parse_expression_guard_accepts_new_constructor_with_into_after_block 2>&1 | tee .sisyphus/evidence/task-2-guard-expression-green.log`, and `cargo test --lib parse_statement_guard_rejects_misindented_constructor_else 2>&1 | tee .sisyphus/evidence/task-2-guard-misindent.log`.
    Expected: Aligned `else` parses after the constructor block dedents; expression guard with `into` after constructor block parses; misindented `else` is rejected deterministically.
    Evidence: .sisyphus/evidence/task-2-guard-statement-green.log, .sisyphus/evidence/task-2-guard-expression-green.log, .sisyphus/evidence/task-2-guard-misindent.log

  Scenario: Existing propagate parsing remains compatible
    Tool: Bash
    Steps: Run call and non-call parser regression commands.
    Expected: Call propagation still parses; ordinary non-call propagation still rejects.
    Evidence: .sisyphus/evidence/task-2-call-regression.log and .sisyphus/evidence/task-2-non-call-regression.log
  ```

  **Commit**: NO | Message: N/A | Files: [N/A]

- [x] 3. Registry GREEN/REFACTOR for extensible fallible constructors

  **What to do**: Introduce `src/type_system/fallible_constructors.rs` or equivalent module. Define a registry entry type containing canonical result type name, runtime function symbol, required named fields in ABI order, expected field `CoreType`s, return/success type, error types, and ABI/lowering descriptor. Add production entry for `FrameClock` mapping `frames_per_second: int32` to `frame_clock_new` and `InvalidFrameRateError`. Add a `#[cfg(test)]` test-only second entry to prove extensibility and prevent FrameClock coupling. Export lookup by resolved canonical type name. Wire module exports wherever project module structure requires.
  **Must NOT do**: Do not put FrameClock-specific branches in checker/codegen helpers outside this registry. Do not merge with `PROPERTYLESS_CONSTRUCTORS` unless that can be done without destabilizing existing Bytes/StringBuilder behavior.

  **Recommended Agent Profile**:
  - Category: `deep` - Reason: Establishes core extensibility contract and future feature surface.
  - Skills: []
  - Omitted: [`frontend-ui-ux`] - No UI work.

  **Parallelization**: Can Parallel: NO | Wave 2 | Blocks: [4, 5, 9, 10] | Blocked By: [1]

  **References**:
  - `src/type_system/propertyless_constructors.rs` - registry style reference.
  - `src/type_system/checker/time_builtins.rs:15-20` - canonical names for FrameClock and InvalidFrameRateError.
  - `src/type_system/types.rs` - `CoreType` constructors for field/return/error type metadata.
  - `src/codegen/error_abi.rs` - ABI descriptor must align with existing error aggregate rules.

  **Acceptance Criteria**:
  - [ ] `cargo test --lib fallible_constructor_registry_registers_frameclock 2>&1 | tee .sisyphus/evidence/task-3-frameclock-registry.log` exits 0.
  - [ ] `cargo test --lib fallible_constructor_registry_supports_test_second_entry 2>&1 | tee .sisyphus/evidence/task-3-second-entry.log` exits 0.
  - [ ] `cargo test --lib propertyless_constructor_accepts_bytes 2>&1 | tee .sisyphus/evidence/task-3-propertyless-regression.log` exits 0.

  **QA Scenarios**:
  ```
  Scenario: FrameClock is a registry entry, not hardcoded logic
    Tool: Bash
    Steps: Run `cargo test --lib fallible_constructor_registry_registers_frameclock 2>&1 | tee .sisyphus/evidence/task-3-frameclock-registry.log`.
    Expected: Test verifies FrameClock entry fields, runtime symbol `frame_clock_new`, success type `FrameClock`, and error `InvalidFrameRateError`.
    Evidence: .sisyphus/evidence/task-3-frameclock-registry.log

  Scenario: Registry proves extensibility
    Tool: Bash
    Steps: Run `cargo test --lib fallible_constructor_registry_supports_test_second_entry 2>&1 | tee .sisyphus/evidence/task-3-second-entry.log`.
    Expected: Test-only second entry resolves through same lookup API without FrameClock branches.
    Evidence: .sisyphus/evidence/task-3-second-entry.log
  ```

  **Commit**: NO | Message: N/A | Files: [N/A]

- [x] 4. Typechecker GREEN/REFACTOR for shared fallible expressions

  **What to do**: Add a shared fallible-expression classifier/helper that returns success type, error types, expression kind, and optional fallible-constructor registry metadata. It must handle existing call-shaped fallible expressions and registered constructor expressions. Refactor `type_check_propagate_expr` in `src/type_system/checker/expressions.rs` and guard checking in `src/type_system/checker/expressions_guard.rs` to use the helper. For constructors, validate fields against registry metadata, type-check field expressions, coerce integer literals where existing rules allow, and perform the same enclosing-function error subset checks as call propagation. Lookup registry after resolving the constructor target to canonical type identity, not raw alias syntax. Add exact diagnostic substring `does not have a fallible constructor` for `propagate new NonFallibleType:`.
  **Must NOT do**: Do not encode fallibility into `CoreType`. Do not accept arbitrary aggregate shapes as fallible. Do not bypass existing ordinary constructor validation for non-registered constructors.

  **Recommended Agent Profile**:
  - Category: `deep` - Reason: Core semantics across propagation, guard, aliases, diagnostics, and compatibility.
  - Skills: []
  - Omitted: [`frontend-ui-ux`] - No UI work.

  **Parallelization**: Can Parallel: NO | Wave 2 | Blocks: [5, 6, 7, 8, 9, 10] | Blocked By: [2, 3]

  **References**:
  - `src/type_system/checker/expressions.rs:364-470` - propagation error subset logic to reuse.
  - `src/type_system/checker/expressions_guard.rs:77-180` - guard subject signature logic to generalize.
  - `src/type_system/checker/constructors.rs:127-194` - field validation patterns.
  - `src/type_system/checker/call_resolution.rs` - call signature instantiation and impurity rules.
  - `src/type_system/errors.rs` - add/reuse diagnostic variant with exact substring.
  - `test-projects/import-types-aliased/src/main.op` - alias constructor regression.

  **Acceptance Criteria**:
  - [ ] `cargo test --lib propagate_new_frameclock_typechecks_via_registry 2>&1 | tee .sisyphus/evidence/task-4-propagate-frameclock.log` exits 0.
  - [ ] `cargo test --lib guard_new_frameclock_typechecks_via_registry 2>&1 | tee .sisyphus/evidence/task-4-guard-frameclock.log` exits 0.
  - [ ] `cargo test --lib propagate_new_nonfallible_constructor_reports_diagnostic 2>&1 | tee .sisyphus/evidence/task-4-nonfallible-diagnostic.log` exits 0.
  - [ ] `cargo test --lib propagate_new_constructor_error_mismatch_reports_existing_rule 2>&1 | tee .sisyphus/evidence/task-4-error-mismatch.log` exits 0.
  - [ ] `cargo test --features integration --test integration_e2e -- --nocapture import_types_aliased_compiles_and_runs 2>&1 | tee .sisyphus/evidence/task-4-alias-regression.log` exits 0.

  **QA Scenarios**:
  ```
  Scenario: Propagate and guard use shared fallible constructor semantics
    Tool: Bash
    Steps: Run `cargo test --lib propagate_new_frameclock_typechecks_via_registry` and `cargo test --lib guard_new_frameclock_typechecks_via_registry`, teeing logs to task-4 evidence files.
    Expected: Both commands exit 0 and tests assert success type FrameClock plus InvalidFrameRateError handling.
    Evidence: .sisyphus/evidence/task-4-propagate-frameclock.log and .sisyphus/evidence/task-4-guard-frameclock.log

  Scenario: Non-registered constructor is rejected clearly
    Tool: Bash
    Steps: Run `cargo test --lib propagate_new_nonfallible_constructor_reports_diagnostic 2>&1 | tee .sisyphus/evidence/task-4-nonfallible-diagnostic.log`.
    Expected: Command exits 0 and diagnostic contains `does not have a fallible constructor`.
    Evidence: .sisyphus/evidence/task-4-nonfallible-diagnostic.log
  ```

  **Commit**: NO | Message: N/A | Files: [N/A]

- [x] 5. Codegen GREEN/REFACTOR for registry-backed fallible constructors

  **What to do**: Update constructor codegen in `src/codegen/adts.rs` to consult the fallible-constructor registry. For registered constructors, evaluate field expressions in registry field order, declare the runtime function via `src/codegen/functions_stdlib.rs::declare_stdlib_function`, build the call, and return the error-bearing aggregate. Let existing `codegen_propagate_expression` / guard lowering consume the aggregate. Add no FrameClock-specific branch outside registry lookup. Keep existing product/sum/propertyless constructor lowering as fallback.
  **Must NOT do**: Do not change runtime C ABI or stdlib function declaration shape. Do not duplicate propagate early-return logic in constructor lowering.

  **Recommended Agent Profile**:
  - Category: `unspecified-high` - Reason: ABI-sensitive lowering with compatibility constraints.
  - Skills: []
  - Omitted: [`frontend-ui-ux`] - No UI work.

  **Parallelization**: Can Parallel: NO | Wave 2 | Blocks: [6, 7, 8, 12] | Blocked By: [3, 4]

  **References**:
  - `src/codegen/adts.rs:28-77` - constructor lowering entry.
  - `src/codegen/functions_stdlib.rs` - declare `frame_clock_new` runtime symbol.
  - `src/codegen/functions_call.rs:410-504` - existing propagation over error aggregate.
  - `src/codegen/error_abi.rs` - aggregate shape and error field rules.
  - `src/codegen/tests.rs:2792-2834` - existing frame-clock ABI assertion pattern.

  **Acceptance Criteria**:
  - [ ] `cargo test --lib codegen_new_frameclock_uses_registered_runtime_symbol 2>&1 | tee .sisyphus/evidence/task-5-codegen-frameclock.log` exits 0.
  - [ ] `cargo test --lib codegen_fallible_constructor_test_second_entry_uses_registry_path 2>&1 | tee .sisyphus/evidence/task-5-codegen-second-entry.log` exits 0.
  - [ ] `git diff -- runtime/opal_io.c runtime/opal_runtime.h > .sisyphus/evidence/task-5-runtime-diff.txt` produces an empty file.

  **QA Scenarios**:
  ```
  Scenario: FrameClock constructor lowers through registry to existing runtime symbol
    Tool: Bash
    Steps: Run `cargo test --lib codegen_new_frameclock_uses_registered_runtime_symbol 2>&1 | tee .sisyphus/evidence/task-5-codegen-frameclock.log`.
    Expected: IR contains `@frame_clock_new` declaration/call via registry-backed constructor lowering and command exits 0.
    Evidence: .sisyphus/evidence/task-5-codegen-frameclock.log

  Scenario: Runtime ABI remains untouched
    Tool: Bash
    Steps: Run `git diff -- runtime/opal_io.c runtime/opal_runtime.h > .sisyphus/evidence/task-5-runtime-diff.txt`.
    Expected: Evidence file is empty.
    Evidence: .sisyphus/evidence/task-5-runtime-diff.txt
  ```

  **Commit**: NO | Message: N/A | Files: [N/A]

- [x] 6. Migrate 30fps FrameClock project to new constructor syntax

  **What to do**: Update `test-projects/frame-clock-30fps-ten-waits-timing/src/main.op` to use `propagate new FrameClock:` with `frames_per_second: 30`. Remove source-level `frame_clock_new` import/use. Keep `frame_clock_wait_next_sync` behavior unchanged. Use correct import style based on existing standard type visibility; if `FrameClock` does not need importing, do not add unnecessary import.
  **Must NOT do**: Do not change timing assertions or runtime behavior.

  **Recommended Agent Profile**:
  - Category: `quick` - Reason: Single fixture migration with one integration gate.
  - Skills: []
  - Omitted: [`frontend-ui-ux`] - No UI work.

  **Parallelization**: Can Parallel: YES | Wave 3 | Blocks: [11, 12] | Blocked By: [5]

  **References**:
  - `test-projects/frame-clock-30fps-ten-waits-timing/src/main.op` - current source.
  - `tests/integration_e2e/time_stdlib.rs:237-309` - timing test.

  **Acceptance Criteria**:
  - [ ] `cargo test --features integration --test integration_e2e -- --nocapture frame_clock_30fps_ten_waits_timing 2>&1 | tee .sisyphus/evidence/task-6-30fps.log` exits 0.
  - [ ] `grep -n "frame_clock_new" test-projects/frame-clock-30fps-ten-waits-timing/src/main.op` exits 1.
  - [ ] `grep -n "propagate new FrameClock" test-projects/frame-clock-30fps-ten-waits-timing/src/main.op` exits 0.

  **QA Scenarios**:
  ```
  Scenario: Timing project runs via generalized constructor syntax
    Tool: Bash
    Steps: Run `cargo test --features integration --test integration_e2e -- --nocapture frame_clock_30fps_ten_waits_timing 2>&1 | tee .sisyphus/evidence/task-6-30fps.log`.
    Expected: Command exits 0 and existing timing bounds pass.
    Evidence: .sisyphus/evidence/task-6-30fps.log
  ```

  **Commit**: NO | Message: N/A | Files: [N/A]

- [x] 7. Split and migrate zero/negative invalid-fps projects

  **What to do**: Refactor `tests/integration_e2e/time_stdlib.rs:155-234` to expose separate tests `frame_clock_rejects_zero_fps` and `frame_clock_rejects_negative_fps`, using shared helper code if helpful. Update `test-projects/frame-clock-rejects-zero-fps/src/main.op` and `test-projects/frame-clock-rejects-negative-fps/src/main.op` to use `guard new FrameClock:` with `frames_per_second: 0` and `frames_per_second: -1`. Preserve output labels and `InvalidFrameRateError` assertions.
  **Must NOT do**: Do not convert handled-error tests into propagation-only tests that lose error-output validation.

  **Recommended Agent Profile**:
  - Category: `unspecified-high` - Reason: Integration harness split plus two fixture migrations.
  - Skills: []
  - Omitted: [`frontend-ui-ux`] - No UI work.

  **Parallelization**: Can Parallel: YES | Wave 3 | Blocks: [11, 12] | Blocked By: [5]

  **References**:
  - `tests/integration_e2e/time_stdlib.rs:155-234` - existing combined invalid-fps test.
  - `test-projects/frame-clock-rejects-zero-fps/src/main.op` - zero case.
  - `test-projects/frame-clock-rejects-negative-fps/src/main.op` - negative case.

  **Acceptance Criteria**:
  - [ ] `cargo test --features integration --test integration_e2e -- --nocapture frame_clock_rejects_zero_fps 2>&1 | tee .sisyphus/evidence/task-7-zero.log` exits 0.
  - [ ] `cargo test --features integration --test integration_e2e -- --nocapture frame_clock_rejects_negative_fps 2>&1 | tee .sisyphus/evidence/task-7-negative.log` exits 0.
  - [ ] `grep -n "frame_clock_new" test-projects/frame-clock-rejects-zero-fps/src/main.op` exits 1.
  - [ ] `grep -n "frame_clock_new" test-projects/frame-clock-rejects-negative-fps/src/main.op` exits 1.

  **QA Scenarios**:
  ```
  Scenario: Invalid fps projects still handle runtime error locally
    Tool: Bash
    Steps: Run the zero and negative fps cargo test commands, saving logs to task-7 evidence files.
    Expected: Both commands exit 0 and tests validate `InvalidFrameRateError` output.
    Evidence: .sisyphus/evidence/task-7-zero.log and .sisyphus/evidence/task-7-negative.log
  ```

  **Commit**: NO | Message: N/A | Files: [N/A]

- [x] 8. Migrate invalid-fps harness project to new constructor syntax

  **What to do**: Update `test-projects/frame-clock-rejects-invalid-fps/src/main.op` to use `guard new FrameClock:` in its helper flow while preserving both `fps=0` and `fps=-1` behavior. Keep or refactor existing combined `frame_clock_rejects_invalid_fps` integration test after Task 7 split exists.
  **Must NOT do**: Do not leave `frame_clock_new` in imports, comments, or executable source for this project.

  **Recommended Agent Profile**:
  - Category: `quick` - Reason: Single harness migration after shared guard path exists.
  - Skills: []
  - Omitted: [`frontend-ui-ux`] - No UI work.

  **Parallelization**: Can Parallel: YES | Wave 3 | Blocks: [11, 12] | Blocked By: [5]

  **References**:
  - `test-projects/frame-clock-rejects-invalid-fps/src/main.op` - current harness source.
  - `tests/integration_e2e/time_stdlib.rs:155-234` - combined invalid-fps expectations.

  **Acceptance Criteria**:
  - [ ] `cargo test --features integration --test integration_e2e -- --nocapture frame_clock_rejects_invalid_fps 2>&1 | tee .sisyphus/evidence/task-8-invalid.log` exits 0.
  - [ ] `grep -n "frame_clock_new" test-projects/frame-clock-rejects-invalid-fps/src/main.op` exits 1.

  **QA Scenarios**:
  ```
  Scenario: Invalid-fps harness validates both bad values with new constructor syntax
    Tool: Bash
    Steps: Run `cargo test --features integration --test integration_e2e -- --nocapture frame_clock_rejects_invalid_fps 2>&1 | tee .sisyphus/evidence/task-8-invalid.log`.
    Expected: Command exits 0 and validates both zero and negative fps error behavior.
    Evidence: .sisyphus/evidence/task-8-invalid.log
  ```

  **Commit**: NO | Message: N/A | Files: [N/A]

- [x] 9. Add alias/import and ordinary constructor regression gates

  **What to do**: Keep `test-projects/import-types-aliased/src/main.op` behavior unchanged and add or strengthen regression tests proving `new Account:` still resolves imported type alias constructors after fallible-constructor registry work. Also add a negative test proving registry lookup uses resolved canonical identity and does not falsely treat ordinary aliases as fallible unless their canonical type is registered. If adding a test-only registered alias target, keep it in Rust unit tests only; do not change production fixture semantics.
  **Must NOT do**: Do not modify the import-types-aliased fixture unless needed to add a separate new fixture; preserve its existing output.

  **Recommended Agent Profile**:
  - Category: `unspecified-high` - Reason: Alias/import regression is a critical user-selected guardrail.
  - Skills: []
  - Omitted: [`frontend-ui-ux`] - No UI work.

  **Parallelization**: Can Parallel: YES | Wave 3 | Blocks: [12] | Blocked By: [4]

  **References**:
  - `test-projects/import-types-aliased/src/main.op` - selected alias fixture.
  - `tests/integration_e2e/project_execution.rs` - integration test runner.
  - `.sisyphus/plans/types-file-imports.md` - prior import-type plan context.

  **Acceptance Criteria**:
  - [ ] `cargo test --features integration --test integration_e2e -- --nocapture import_types_aliased_compiles_and_runs 2>&1 | tee .sisyphus/evidence/task-9-import-alias.log` exits 0.
  - [ ] `cargo test --lib fallible_constructor_lookup_uses_resolved_canonical_type 2>&1 | tee .sisyphus/evidence/task-9-canonical-lookup.log` exits 0.
  - [ ] `cargo test --lib ordinary_aliased_constructor_not_treated_as_fallible 2>&1 | tee .sisyphus/evidence/task-9-ordinary-alias.log` exits 0.

  **QA Scenarios**:
  ```
  Scenario: User-selected alias fixture remains green
    Tool: Bash
    Steps: Run `cargo test --features integration --test integration_e2e -- --nocapture import_types_aliased_compiles_and_runs 2>&1 | tee .sisyphus/evidence/task-9-import-alias.log`.
    Expected: Command exits 0 and existing output remains valid.
    Evidence: .sisyphus/evidence/task-9-import-alias.log
  ```

  **Commit**: NO | Message: N/A | Files: [N/A]

- [x] 10. Add malformed constructor and diagnostic coverage

  **What to do**: Add tests for missing required field, extra field, wrong field type, non-registered fallible constructor, and caller error mismatch. Use existing `TypeError` variants where possible; if a new variant is required, ensure rendered diagnostic includes exact substring `does not have a fallible constructor` for non-registered constructor propagation. Include both `propagate new` and `guard new` where behavior differs.
  **Must NOT do**: Do not add broad diagnostic infrastructure or change unrelated diagnostics.

  **Recommended Agent Profile**:
  - Category: `unspecified-high` - Reason: Diagnostic surface must be stable and precise.
  - Skills: []
  - Omitted: [`frontend-ui-ux`] - No UI work.

  **Parallelization**: Can Parallel: YES | Wave 3 | Blocks: [12] | Blocked By: [3, 4]

  **References**:
  - `tests/integration_e2e/compile_failures.rs` - compile-failure assertion pattern.
  - `src/type_system/errors.rs` - diagnostic variants.
  - `src/type_system/checker/constructors.rs:127-194` - field validation patterns.

  **Acceptance Criteria**:
  - [ ] `cargo test --features integration --test integration_e2e -- --nocapture new_frameclock_missing_field 2>&1 | tee .sisyphus/evidence/task-10-missing.log` exits 0.
  - [ ] `cargo test --features integration --test integration_e2e -- --nocapture new_frameclock_extra_field 2>&1 | tee .sisyphus/evidence/task-10-extra.log` exits 0.
  - [ ] `cargo test --features integration --test integration_e2e -- --nocapture new_frameclock_wrong_type 2>&1 | tee .sisyphus/evidence/task-10-wrong-type.log` exits 0.
  - [ ] `cargo test --lib propagate_new_nonfallible_constructor_reports_diagnostic 2>&1 | tee .sisyphus/evidence/task-10-nonfallible.log` exits 0.

  **QA Scenarios**:
  ```
  Scenario: Malformed registered constructor fields are rejected
    Tool: Bash
    Steps: Run missing, extra, and wrong-type integration test commands.
    Expected: All commands exit 0 and assert deterministic typechecker diagnostics.
    Evidence: .sisyphus/evidence/task-10-missing.log, .sisyphus/evidence/task-10-extra.log, .sisyphus/evidence/task-10-wrong-type.log
  ```

  **Commit**: NO | Message: N/A | Files: [N/A]

- [x] 11. Remove source-level frame_clock_new from frame-clock projects and refactor

  **What to do**: Search frame-clock test project sources for `frame_clock_new(` and remove any remaining source-level usage. Refactor new registry/classifier/codegen code for clarity only after all targeted GREEN tests pass. Keep runtime/internal Rust declarations and tests that intentionally assert `frame_clock_new` ABI.
  **Must NOT do**: Do not remove `frame_clock_new` builtin/runtime compatibility. Do not refactor unrelated constructors.

  **Recommended Agent Profile**:
  - Category: `unspecified-high` - Reason: Cross-cutting cleanup and scope verification.
  - Skills: []
  - Omitted: [`frontend-ui-ux`] - No UI work.

  **Parallelization**: Can Parallel: NO | Wave 3 | Blocks: [12] | Blocked By: [6, 7, 8]

  **References**:
  - `test-projects/frame-clock-*/src/*.op` - migration target.
  - `src/codegen/tests.rs` - keep ABI assertions.
  - `src/type_system/checker/time_builtins.rs` - keep builtin registration.

  **Acceptance Criteria**:
  - [ ] `grep -R "frame_clock_new(" test-projects/frame-clock-*/src --include='*.op' > .sisyphus/evidence/task-11-remaining-frame-clock-new.txt; test $? -eq 1` succeeds.
  - [ ] `cargo test --lib fallible_constructor_registry_registers_frameclock fallible_constructor_registry_supports_test_second_entry 2>&1 | tee .sisyphus/evidence/task-11-registry-regression.log` exits 0.
  - [ ] `git diff -- runtime/opal_io.c runtime/opal_runtime.h > .sisyphus/evidence/task-11-runtime-diff.txt` is empty.

  **QA Scenarios**:
  ```
  Scenario: No frame-clock project source uses old function
    Tool: Bash
    Steps: Run `grep -R "frame_clock_new(" test-projects/frame-clock-*/src --include='*.op' > .sisyphus/evidence/task-11-remaining-frame-clock-new.txt; test $? -eq 1`.
    Expected: No source-level frame-clock project usage remains.
    Evidence: .sisyphus/evidence/task-11-remaining-frame-clock-new.txt
  ```

  **Commit**: NO | Message: N/A | Files: [N/A]

- [x] 12. Run full regression and all current test projects

  **What to do**: Run the full verification suite and fix failures introduced by this change. Include unit tests, integration tests, all features, clippy, fmt, and any current test-project coverage wired into integration tests. If a pre-existing failure is discovered, record it separately and do not expand scope unless directly caused by changed files.
  **Must NOT do**: Do not skip slow tests. Do not mark complete with known failures caused by this change.

  **Recommended Agent Profile**:
  - Category: `unspecified-high` - Reason: Full regression with failure triage.
  - Skills: []
  - Omitted: [`frontend-ui-ux`] - No UI work.

  **Parallelization**: Can Parallel: NO | Wave 4 | Blocks: [13] | Blocked By: [9, 10, 11]

  **References**:
  - `.github/workflows/ci.yml` - CI-equivalent checks.
  - `Makefile.toml` - local test task patterns.
  - `README.md` Testing section - integration test conventions.

  **Acceptance Criteria**:
  - [ ] `cargo test --lib 2>&1 | tee .sisyphus/evidence/task-12-lib.log` exits 0.
  - [ ] `cargo test --features integration --test integration_e2e -- --nocapture 2>&1 | tee .sisyphus/evidence/task-12-integration.log` exits 0.
  - [ ] `timeout 900 cargo test --all-features 2>&1 | tee .sisyphus/evidence/task-12-all-features.log` exits 0.
  - [ ] `cargo clippy --all-targets --all-features -- -D warnings 2>&1 | tee .sisyphus/evidence/task-12-clippy.log` exits 0.
  - [ ] `cargo fmt --all -- --check 2>&1 | tee .sisyphus/evidence/task-12-fmt.log` exits 0.

  **QA Scenarios**:
  ```
  Scenario: Full CI-equivalent regression passes
    Tool: Bash
    Steps: Run `timeout 900 cargo test --all-features 2>&1 | tee .sisyphus/evidence/task-12-all-features.log`.
    Expected: Command exits 0.
    Evidence: .sisyphus/evidence/task-12-all-features.log

  Scenario: Lint and format gates pass
    Tool: Bash
    Steps: Run clippy and fmt check commands, saving logs to task-12 evidence.
    Expected: Both commands exit 0.
    Evidence: .sisyphus/evidence/task-12-clippy.log and .sisyphus/evidence/task-12-fmt.log
  ```

  **Commit**: NO | Message: N/A | Files: [N/A]

- [x] 13. Pre-commit cleanup and evidence hygiene

  **What to do**: Fix only issues introduced by this change or reported by pre-commit hooks. Remove or ignore generated `.sisyphus/evidence/*` artifacts before final staging unless already tracked. Rerun affected Task 12 gates after any cleanup.
  **Must NOT do**: Do not fix unrelated pre-existing lints outside changed files unless required to unblock commit and documented.

  **Recommended Agent Profile**:
  - Category: `quick` - Reason: Focused cleanup after full regression.
  - Skills: []
  - Omitted: [`frontend-ui-ux`] - No UI work.

  **Parallelization**: Can Parallel: NO | Wave 4 | Blocks: [14] | Blocked By: [12]

  **References**:
  - Task 12 evidence logs.
  - `git status --porcelain` output.

  **Acceptance Criteria**:
  - [ ] `git status --porcelain > .sisyphus/evidence/task-13-status-before-commit.txt` reviewed for intended files only.
  - [ ] No untracked `.sisyphus/evidence` files are staged.
  - [ ] Any cleanup reruns the relevant failed command from Task 12 and exits 0.

  **QA Scenarios**:
  ```
  Scenario: Only intended tracked files remain before commit
    Tool: Bash
    Steps: Run `git status --porcelain > .sisyphus/evidence/task-13-status-before-commit.txt` and inspect for changed compiler/test files only; remove/ignore untracked evidence before staging.
    Expected: Only intended source/test changes remain eligible for staging.
    Evidence: .sisyphus/evidence/task-13-status-before-commit.txt
  ```

  **Commit**: NO | Message: N/A | Files: [N/A]

- [x] 14. Commit generalized fallible constructor feature

  **What to do**: Inspect `git status`, `git diff`, and `git log --oneline -10`. Stage only intended files. Commit with subject `feat(language): generalize fallible constructor expressions` and body summarizing registry-backed constructors, FrameClock migration, alias regression, ABI preservation, and tests/checks run. If hooks fail, fix introduced issues, rerun relevant gates, and retry without skipping hooks.
  **Must NOT do**: Do not amend unrelated work. Do not skip hooks. Do not commit generated evidence unless already tracked or explicitly required.

  **Recommended Agent Profile**:
  - Category: `quick` - Reason: Focused git operation after verification.
  - Skills: [`git-master`] - Required for git operations.
  - Omitted: [`frontend-ui-ux`] - No UI work.

  **Parallelization**: Can Parallel: NO | Wave 4 | Blocks: [F1, F2, F3, F4] | Blocked By: [13]

  **References**:
  - Commit subject: `feat(language): generalize fallible constructor expressions`.
  - Task 12 verification logs.
  - Task 13 intended file status.

  **Acceptance Criteria**:
  - [ ] `git log -1 --format=%s%n%b > .sisyphus/evidence/task-14-commit-message.txt` shows correct subject/body.
  - [ ] `git status --porcelain --untracked-files=no > /tmp/task-14-final-status.txt && cp /tmp/task-14-final-status.txt .sisyphus/evidence/task-14-final-status.txt` records empty tracked-file status after commit.

  **QA Scenarios**:
  ```
  Scenario: Final commit documents generalized feature
    Tool: Bash
    Steps: Run `git log -1 --format=%s%n%b > .sisyphus/evidence/task-14-commit-message.txt`.
    Expected: Subject is `feat(language): generalize fallible constructor expressions` and body mentions registry, FrameClock migration, alias regression, ABI preservation, and tests.
    Evidence: .sisyphus/evidence/task-14-commit-message.txt

  Scenario: Tracked working tree clean after commit
    Tool: Bash
    Steps: Run `git status --porcelain --untracked-files=no > /tmp/task-14-final-status.txt && cp /tmp/task-14-final-status.txt .sisyphus/evidence/task-14-final-status.txt`.
    Expected: Status file is empty.
    Evidence: .sisyphus/evidence/task-14-final-status.txt
  ```

  **Commit**: YES | Message: `feat(language): generalize fallible constructor expressions` | Files: [intended compiler/test-project/test files only]

## Final Verification Wave (MANDATORY — after ALL implementation tasks)
> 4 review agents/checks run in PARALLEL where possible. ALL must APPROVE. Present consolidated results to user and get explicit "okay" before completing.
> **Do NOT auto-proceed after verification. Wait for user's explicit approval before marking work complete.**
- [x] F1. Generalization Compliance Audit — oracle
  - Agent Profile: `oracle` read-only review.
  - Parallelization: Can Parallel: YES | Final Wave | Blocks: Completion | Blocked By: [14]
  - QA Scenario:
    ```
    Scenario: Generalization audit approves final diff
      Tool: `call_omo_agent(agent="oracle", prompt="Review .sisyphus/plans/frameclock-constructor-syntax.md against git show HEAD. Verify generalized fallible-constructor design, no hardcoded FrameClock outside registry/tests, alias regression, test-only second entry, and every Must Have/Must NOT Have. Return APPROVE only if no blockers; otherwise REJECT with blockers.")`
      Steps: Launch after Task 14 and save result to `.sisyphus/evidence/f1-generalization-audit.md`.
      Expected: Oracle returns APPROVE/OKAY with no blocking findings.
      Evidence: .sisyphus/evidence/f1-generalization-audit.md
    ```
- [x] F2. Code Quality Review — oracle
  - Agent Profile: `oracle` code reviewer.
  - Parallelization: Can Parallel: YES | Final Wave | Blocks: Completion | Blocked By: [14]
  - QA Scenario:
    ```
    Scenario: Code quality review approves implementation
      Tool: `call_omo_agent(agent="oracle", prompt="Inspect git show HEAD for the generalized fallible-constructor implementation. Check minimality, idiomatic parser/typechecker/codegen structure, no unrelated refactors, no AI slop, and clean extension points. Return APPROVE only if no blockers; otherwise REJECT with blockers.")`
      Steps: Launch after Task 14 and save result to `.sisyphus/evidence/f2-code-quality.md`.
      Expected: Oracle returns APPROVE/OKAY with no blocking findings.
      Evidence: .sisyphus/evidence/f2-code-quality.md
    ```
- [x] F3. Real QA Rerun — Bash
  - Agent Profile: direct command QA runner.
  - Parallelization: Can Parallel: YES | Final Wave | Blocks: Completion | Blocked By: [14]
  - QA Scenario:
    ```
    Scenario: Critical integration gates rerun
      Tool: Bash
      Steps: Run `cargo test --features integration --test integration_e2e -- --nocapture frame_clock_30fps_ten_waits_timing 2>&1 | tee .sisyphus/evidence/f3-30fps.log`, `cargo test --features integration --test integration_e2e -- --nocapture frame_clock_rejects_zero_fps 2>&1 | tee .sisyphus/evidence/f3-zero-fps.log`, `cargo test --features integration --test integration_e2e -- --nocapture frame_clock_rejects_negative_fps 2>&1 | tee .sisyphus/evidence/f3-negative-fps.log`, `cargo test --features integration --test integration_e2e -- --nocapture frame_clock_rejects_invalid_fps 2>&1 | tee .sisyphus/evidence/f3-invalid-fps.log`, and `cargo test --features integration --test integration_e2e -- --nocapture import_types_aliased_compiles_and_runs 2>&1 | tee .sisyphus/evidence/f3-import-alias.log`.
      Expected: All commands exit 0.
      Evidence: .sisyphus/evidence/f3-30fps.log, .sisyphus/evidence/f3-zero-fps.log, .sisyphus/evidence/f3-negative-fps.log, .sisyphus/evidence/f3-invalid-fps.log, .sisyphus/evidence/f3-import-alias.log
    ```
- [x] F4. Scope Fidelity Check — oracle
  - Agent Profile: `oracle` scope reviewer.
  - Parallelization: Can Parallel: YES | Final Wave | Blocks: Completion | Blocked By: [14]
  - QA Scenario:
    ```
    Scenario: Scope fidelity review approves boundaries
      Tool: `call_omo_agent(agent="oracle", prompt="Review git show HEAD, remaining source usage of frame_clock_new in test-projects/frame-clock-*/src, runtime diffs, and registry usage. Confirm no runtime ABI change, no arbitrary aggregate fallibility, no ordinary constructor/alias regression, and no unrelated cleanup. Return APPROVE only if no blockers; otherwise REJECT with blockers.")`
      Steps: Launch after Task 14 and save result to `.sisyphus/evidence/f4-scope-fidelity.md`.
      Expected: Oracle returns APPROVE/OKAY with no blocking findings.
      Evidence: .sisyphus/evidence/f4-scope-fidelity.md
    ```

## Commit Strategy
- Make one final commit only after implementation tasks and required verification pass.
- Commit message: `feat(language): generalize fallible constructor expressions`
- Commit body must mention:
  - Registry-backed fallible constructors.
  - FrameClock migrated to `propagate new FrameClock:` / `guard new FrameClock:`.
  - Alias/import constructor regression preserved.
  - Runtime ABI preserved.
  - Tests/checks run.
- Stage only intended files. Do not commit `.sisyphus/evidence` unless already tracked or explicitly required.

## Success Criteria
- `propagate new FrameClock:` and `guard new FrameClock:` work through generalized registry-backed fallible expression handling.
- Future runtime-backed fallible constructors can be added by registry entry + stdlib declaration + tests, without parser or hardcoded codegen branches.
- Ordinary constructors, imported aliases, propertyless constructors, and call-shaped `propagate`/`guard` remain compatible.
- All current tests/test projects/checks pass.
- Final commit is created and working tree is clean for tracked files.
