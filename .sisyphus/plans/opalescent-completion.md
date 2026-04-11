# Opalescent Language — Full Completion Work Plan

## TL;DR

> **Quick Summary**: Complete the Opalescent programming language from current state (~80% Phase 1, 259 tests passing) through all 10 phases to production readiness. The work follows the dependency chain established in PLAN.md: finish Phase 2 blockers → Phase 2 features → Advanced Types → Module System → Code Generation → Hot Reloading → DX/Stdlib/Quality/Production.
> 
> **Deliverables**:
> - Fully functional compiler for the Opalescent language
> - LLVM-based code generation producing native executables
> - Hot reloading system with ABI guards and versioned dynamic library swap
> - Standard library with core/collections/system modules
> - Language Server Protocol implementation
> - Code formatter
> - Package manager and build system
> - All language-spec/*.op files compiling and executing correctly
> 
> **Estimated Effort**: XL (multi-week)
> **Parallel Execution**: YES - within each wave, tasks run in parallel
> **Critical Path**: Phase 2 Blockers → Phase 2 Features → Phase 3 Types → Phase 4 Modules → Phase 5 CodeGen → Phase 6 Hot Reload → Phases 7-10

---

## Context

### Original Request
"Please continue and fully finish this project. Make sure that all agents follow the guidelines in .github/chatmodes/principal-engineer.chatmode.md. Always have your workers check linting and tests for every change, and all commits should be atomic units of work that NEVER EVER have broken tests, lint runs, etc."

### Interview Summary
**Key Discussions**:
- Project is a compiled, statically-typed programming language with hot reloading
- Follows strict TDD (red-green-refactor), no exceptions
- Extremely strict Clippy linting (pedantic + nursery + 60+ restriction lints)
- no_std compatible core modules (alloc/core over std)
- All files under 500 lines (test files under 1000)
- Never use --no-verify on git commits
- Use Serena tools extensively for development
- Comprehensive documentation on every item (public and private)
- All tests must be mocked/stubbed — no actual file IO

**Research Findings**:
- 259 tests passing, lint clean, clean working tree
- Lexer and Parser are 100% complete
- Type System Core is ~80% done — error handling (guard/propagate/errors) fully implemented
- Phase 2 Blockers #2-#9 are NOT STARTED
- Language spec defined in overview.md, math.md, modules.md
- 9 example .op files serve as integration benchmarks
- Hot reload architecture documented in HOT_RELOAD_ARCHITECTURE.md
- Error handling patterns in ERROR_HANDLING_STANDARDS.md
- Integration dependencies in INTEGRATION_DEPENDENCIES.md

### Metis Review
**Identified Gaps** (addressed):
- Metis consulted; findings incorporated into plan structure
- Key risks: LLVM integration complexity, no_std constraints in later phases, hot reload system scope

---

## Work Objectives

### Core Objective
Build a complete, production-quality compiled programming language with hot reloading, from the current 80%-Phase-1 state through all 10 phases to production readiness.

### Concrete Deliverables
- Complete type system with generics, ADTs, pattern matching
- Full module system with imports/exports and circular dependency detection
- LLVM-based code generation targeting native executables
- Hot reloading system with versioned dynamic library swap
- Standard library (core, collections, system)
- Language Server Protocol implementation
- Code formatter
- Build system with dependency management
- All `language-spec/*.op` files parsing, type-checking, and executing correctly

### Definition of Done
- [ ] All `language-spec/*.op` example files compile and produce correct output
- [ ] `cargo make test` passes with comprehensive coverage
- [ ] `cargo make lint` passes with zero warnings
- [ ] `cargo make build-all` succeeds
- [ ] Hot reload demo works (function body change → live reload without restart)
- [ ] LSP provides errors, completion, hover in editors

### Must Have
- TDD for ALL new features (red-green-refactor)
- Comprehensive in-code documentation on every function, struct, enum, trait, module
- no_std compatibility in core modules (lexer, parser, type system, AST)
- miette for all user-facing error reporting
- `#[expect(...)]` instead of `#[allow(...)]` for lint suppressions
- All commits atomic with passing tests + lint
- `cargo make lint-fix` before each commit
- Check `scripts/check-line-count.sh` before each commit
- Output test/lint results to `temp.log` via tee

### Must NOT Have (Guardrails)
- No `unwrap()`, `expect()`, `panic!()`, `todo!()`, `unimplemented!()`, `unreachable!()`
- No `--no-verify` on any git command
- No `as` conversions (use TryFrom/TryInto)
- No `str.to_string()` (use `to_owned()` or `String::from()`)
- No `std::collections::HashMap` in core (use `alloc::collections::BTreeMap`)
- No `#[allow(...)]` without `reason` (use `#[expect(...)]`)
- No single-char lifetime names
- No modifying .git, AGENTS.md, target, scripts, Makefile.toml, or lint rules
- No Python for file editing
- No tests that touch the actual file system
- No files exceeding 500 lines (1000 for test files)
- No `dbg!()` macros in committed code
- No `mem::forget()`, no `Arc<Mutex<T>>` when `Mutex<T>` suffices
- No format strings just to push them (use direct concatenation or `write!`)

---

## Verification Strategy (MANDATORY)

> **ZERO HUMAN INTERVENTION** - ALL verification is agent-executed. No exceptions.

### Test Decision
- **Infrastructure exists**: YES (cargo test with cargo-make)
- **Automated tests**: YES (TDD - mandated by chatmode)
- **Framework**: cargo test (Rust built-in) via `cargo make test`
- **TDD**: Each task follows RED (failing test) → GREEN (minimal impl) → REFACTOR

### QA Policy
Every task MUST:
1. Run `cargo make test 2>&1 | tee temp.log` — ALL tests pass
2. Run `cargo make lint 2>&1 | tee temp.log` — ZERO warnings
3. Run `scripts/check-line-count.sh 2>&1 | tee temp.log` — all files compliant
4. Run `cargo make lint-fix` before committing
5. Commit with `git commit -m "message"` (hooks run automatically)
6. Read temp.log to verify results

  ### Pre-Task Mandatory Reads
  Every worker MUST read these files before starting ANY task:
  - `.github/chatmodes/principal-engineer.chatmode.md` (full file — primary development rules)
  - `.github/chatmodes/refactor-lint.chatmode.md` (full file — detailed lint rule explanations and patterns)
  - `language-spec/requirements/overview.md` (language design specification)
  - `language-spec/requirements/math.md` (numeric type and arithmetic specification)
  - `language-spec/requirements/modules.md` (module system specification)
  - `PLAN.md` (full file — master project plan with phase structure)
  - The relevant plan file in `plan/` folder for the specific task
  - `ERROR_HANDLING_STANDARDS.md` (error handling patterns — read for ALL tasks)
  - `HOT_RELOAD_ARCHITECTURE.md` (hot-reload metadata requirements — read for ALL type system/codegen tasks)
  - `INTEGRATION_DEPENDENCIES.md` (phase dependency matrix — read for ALL tasks)
  - `REFACTORING_GUIDE.md` (module organization patterns — read when creating new modules)
  - `REFACTORING_STATUS.md` (current refactoring state — read to understand module structure)
  - `FIXES.txt` (known bug fixes — read to avoid reintroducing fixed bugs)
  - `Makefile.toml` (to understand lint rules — READ-ONLY, never modify)

---

## Execution Strategy

### Parallel Execution Waves

> Tasks within each wave can run in parallel. Each wave completes before the next begins.
> The wave structure follows PLAN.md's dependency chain exactly.

```
Wave 1 (Phase 2 Blockers - Independent Foundation):
├── Task 1: Multiple Return Values (#2) [deep]
├── Task 2: Standard Library Built-ins (#3) [deep]
├── Task 3: If Expression Semantics (#5) [deep]
├── Task 4: Warning System Infrastructure (#9) [deep]
├── Task 5: Documentation Sync (#10, #11) [quick]

Wave 2 (Phase 2 Blockers - Dependent on Wave 1):
├── Task 6: Generic Type Parameter Constraints (#4) [deep]
├── Task 7: Member Access Type Checking (#6) [deep]
├── Task 8: Arithmetic Overflow Detection (#7) [deep]
├── Task 9: Division by Zero Detection (#8) [deep]
├── Task 10: Phase 1 Remaining - Integration Tests [unspecified-high]

Wave 3 (Phase 2 Language Features - After All Blockers):
├── Task 11: Function System Completion [deep]
├── Task 12: Variable System Completion [deep]
├── Task 13: Control Flow Completion [deep]
├── Task 14: Arithmetic & Logic Completion [deep]

Wave 4 (Phase 3 - Advanced Type Features):
├── Task 15: ADT Implementation - Pattern Matching [deep]
├── Task 16: ADT Implementation - Constructors & Fields [deep]
├── Task 17: Array & Collection Support [deep]
├── Task 18: Generic System Completion [deep]

Wave 5 (Phase 4 - Module System):
├── Task 19: Import/Export Resolution [deep]
├── Task 20: Module Validation [deep]

Wave 6 (Phase 5 - Code Generation):
├── Task 21: LLVM Backend Setup [deep]
├── Task 22: Code Generation - Expressions & Statements [deep]
├── Task 23: Code Generation - Functions & Control Flow [deep]
├── Task 24: Runtime System Foundation [deep]

Wave 7 (Phase 5 continued + Phase 6 Start):
├── Task 25: Code Generation - ADTs & Generics [deep]
├── Task 26: Runtime System - Memory & Stdlib [deep]
├── Task 27: Basic Optimization Passes [deep]
├── Task 28: Hot Reload Infrastructure [deep]

Wave 8 (Phase 6 - Hot Reloading):
├── Task 29: Change Detection System [deep]
├── Task 30: Hot Reload Safety & ABI Guards [deep]

Wave 9 (Phase 7 - Developer Experience):
├── Task 31: Enhanced Error Reporting [deep]
├── Task 32: Documentation Generation System [unspecified-high]
├── Task 33: Build System [deep]

Wave 10 (Phase 8 - Standard Library):
├── Task 34: Core Library Implementation [deep]
├── Task 35: Collections Library [deep]
├── Task 36: System Library [deep]

Wave 11 (Phase 9 - Testing & Quality):
├── Task 37: Test Framework for Opalescent Programs [deep]
├── Task 38: Language Server Protocol [deep]
├── Task 39: Code Formatter [deep]

Wave 12 (Phase 10 - Production Readiness):
├── Task 40: Performance Optimization [deep]
├── Task 41: Platform Support & Cross-Compilation [deep]
├── Task 42: Ecosystem (Package Manager, Registry) [deep]

Wave FINAL (After ALL tasks — verification):
├── Task F1: Plan compliance audit (oracle)
├── Task F2: Code quality review (unspecified-high)
├── Task F3: Real manual QA - all .op files (unspecified-high)
├── Task F4: Scope fidelity check (deep)
-> Present results -> Get explicit user okay
```

### Dependency Matrix

- **1-5**: None → 6-10, Wave 1
- **6-10**: 1-5 → 11-14, Wave 2
- **11-14**: 6-10 → 15-18, Wave 3
- **15-18**: 11-14 → 19-20, Wave 4
- **19-20**: 15-18 → 21-24, Wave 5
- **21-24**: 19-20 → 25-28, Wave 6
- **25-28**: 21-24 → 29-30, Wave 7
- **29-30**: 28 → 31-33, Wave 8
- **31-33**: 29-30 → 34-36, Wave 9
- **34-36**: 31-33 → 37-39, Wave 10
- **37-39**: 34-36 → 40-42, Wave 11
- **40-42**: 37-39 → F1-F4, Wave 12

### Agent Dispatch Summary

- **Wave 1**: 5 tasks — T1-T4 → `deep`, T5 → `quick`
- **Wave 2**: 5 tasks — T6-T9 → `deep`, T10 → `unspecified-high`
- **Wave 3**: 4 tasks — T11-T14 → `deep`
- **Wave 4**: 4 tasks — T15-T18 → `deep`
- **Wave 5**: 2 tasks — T19-T20 → `deep`
- **Wave 6**: 4 tasks — T21-T24 → `deep`
- **Wave 7**: 4 tasks — T25-T28 → `deep`
- **Wave 8**: 2 tasks — T29-T30 → `deep`
- **Wave 9**: 3 tasks — T31 → `unspecified-high`, T32 → `unspecified-high`, T33 → `deep`
- **Wave 10**: 3 tasks — T34-T36 → `unspecified-high`
- **Wave 11**: 3 tasks — T37 → `deep`, T38 → `deep`, T39 → `unspecified-high`
- **Wave 12**: 3 tasks — T40 → `deep`, T41 → `deep`, T42 → `deep`
- **FINAL**: 4 tasks — F1 → `oracle`, F2 → `unspecified-high`, F3 → `unspecified-high`, F4 → `deep`

---

## TODOs

> Implementation + Test = ONE Task. Never separate.
> EVERY task follows the chatmode workflow: Read requirements → Read PLAN.md → Read plan file → Write tests (TDD) → Implement → Lint → Commit
> **A task WITHOUT QA Scenarios is INCOMPLETE. No exceptions.**

- [ ] 1. Multiple Return Values (Phase 2 Blocker #2)

  **What to do**:
  - Read `language-spec/requirements/overview.md`, `language-spec/requirements/modules.md`, `PLAN.md`, `plan/phase-2-blockers-plan.md`
  - Read `Makefile.toml` to understand all lint rules before writing code
  - Modify `Type::Function` AST node to support `return_types: Vec<Type>` (replace single `return_type`)
  - Modify `Decl::Function` and `Expr::Lambda` to use `return_types: Vec<Type>`
  - Maintain backward compatibility: single return = `Vec` of 1 element
  - Add `LabeledValue` struct for labeled return values
  - Modify `Stmt::Return` to support `values: Vec<LabeledValue>`
  - Parse `f(...): Type1, Type2, Type3` function return signatures
  - Parse `return label1: expr1, label2: expr2` syntax
  - Validate label uniqueness in return statements
  - Update `CoreType::Function` to handle `return_types: Vec<CoreType>`
  - Type check multiple return values match signature (arity + types)
  - Validate labeled return values against function signature labels
  - Update constraint solving for multi-return functions
  - Ensure all return statements in a function match its signature
  - Write TDD tests FIRST (minimum 3 per sub-feature): arity mismatch, label mismatch, single-return back-compat, multiple types parsing, labeled returns parsing
  - Run `cargo make lint-fix && cargo make lint 2>&1 | tee temp.log` — fix ALL warnings
  - Run `cargo make test 2>&1 | tee temp.log` — ALL tests pass
  - Run `scripts/check-line-count.sh` — all files compliant
  - Update `plan/phase-2-blockers-plan.md` and check off items in `PLAN.md`
  - Commit with descriptive message

  **Must NOT do**:
  - Do not use `unwrap()`, `expect()`, `panic!()`, `todo!()`, `unimplemented!()`
  - Do not use `as` conversions, `str.to_string()`, or `HashMap`
  - Do not use `#[allow(...)]` — use `#[expect(..., reason = "...")]`
  - Do not exceed 500 lines per file (refactor if needed)
  - Do not skip documentation on any new item
  - Do not use `--no-verify` on git commands

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: Complex multi-module change spanning AST, parser, and type system with strict lint constraints
  - **Skills**: []
    - No specific skills needed — agent uses Serena and standard tools

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1 (with Tasks 2, 3, 4, 5)
  - **Blocks**: Task 11 (Function System), Task 6 (Generic Constraints)
  - **Blocked By**: None (can start immediately)

  **References**:

  **Pattern References**:
  - `src/ast.rs` and `src/ast/types.rs` — Current `Type::Function` and `Decl::Function` definitions to modify
  - `src/parser/declarations.rs` — Function declaration parsing (extend for multi-return)
  - `src/parser/statements.rs` — Return statement parsing (extend for labeled values)
  - `src/type_system/checker/declarations.rs` — Function declaration type checking
  - `src/type_system/types.rs` — `CoreType::Function` definition to extend

  **API/Type References**:
  - `src/ast.rs:Expr::Lambda` — Lambda expression with return type (change to `return_types`)
  - `src/type_system/checker/expressions.rs` — Lambda type checking
  - `language-spec/simple_quiz.op:50` — `break user_input: s, user_number: n` shows labeled return syntax

  **Test References**:
  - `src/parser/tests.rs` — Existing parser test patterns
  - `src/type_system/tests.rs` — Existing type system test patterns

  **External References**:
  - `language-spec/simple_quiz.op` — Shows multiple return from loop break with labels
  - `plan/phase-2-blockers-plan.md:139-142` — Outline for this task

  **Acceptance Criteria**:
  - [ ] TDD: Tests written FIRST, then implementation
  - [ ] Parse `f(): int32, string` → function with 2 return types
  - [ ] Parse `return label1: expr1, label2: expr2` → labeled return
  - [ ] Type check: return value count matches signature → pass
  - [ ] Type check: return value count mismatch → error with span
  - [ ] Type check: label name mismatch → error with span
  - [ ] Single return backward compatibility preserved
  - [ ] `cargo make test` passes
  - [ ] `cargo make lint` passes with zero warnings
  - [ ] PLAN.md updated with checked boxes

  **QA Scenarios**:

  ```
  Scenario: Multi-return function parses correctly
    Tool: Bash (cargo test)
    Preconditions: Code changes applied
    Steps:
      1. Run `cargo make test 2>&1 | tee temp.log`
      2. Read temp.log and verify all tests pass including new multi-return tests
      3. Verify test count increased from baseline 259
    Expected Result: All tests pass, 0 failures
    Evidence: .sisyphus/evidence/task-1-multi-return-tests.txt

  Scenario: Lint passes with zero warnings
    Tool: Bash (cargo make lint)
    Preconditions: Code changes applied
    Steps:
      1. Run `cargo make lint 2>&1 | tee temp.log`
      2. Read temp.log and verify "Finished" with no warnings
    Expected Result: Zero clippy warnings or errors
    Evidence: .sisyphus/evidence/task-1-lint-clean.txt
  ```

  **Commit**: YES
  - Message: `feat(type-system): implement multiple return values with labeled returns`
  - Pre-commit: `cargo make lint-fix && cargo make test`

- [ ] 2. Standard Library Built-ins (Phase 2 Blocker #3)

  **What to do**:
  - Read all requirements files and PLAN.md before starting
  - Read `Makefile.toml` to understand all lint rules
  - Define built-in function signatures: `print<T>(value: T): unit`, `take_input(): string`, `string_to_int32(s: string): int32 errors ParseError`, `random_int32(min: int32, max: int32): int32`
  - Add `TypeEnvironment::register_builtin()` method
  - Pre-register all built-in functions on `TypeChecker::new()`
  - Ensure built-ins available in all type checking contexts
  - Support generic built-in functions (like `print<T>`)
  - Create `stdlib/prelude.op` with built-in signatures (documentation purposes)
  - Write TDD tests FIRST: type checking with `print()` calls, `take_input()` calls, `string_to_int32()` with error handling, generic built-in instantiation
  - Validate that `language-spec/hello_world.op` can now be type-checked (uses `print()`)
  - Run lint, test, line-count checks
  - Update plan files and PLAN.md
  - Commit

  **Must NOT do**:
  - Same guardrails as Task 1
  - Do not implement runtime behavior for built-ins (just type signatures for now)

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: Type system integration with generic function support requires careful design
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1 (with Tasks 1, 3, 4, 5)
  - **Blocks**: Task 10 (Integration Tests), Task 11 (Function System)
  - **Blocked By**: None

  **References**:

  **Pattern References**:
  - `src/type_system/environment.rs` — TypeEnvironment where built-ins will be registered
  - `src/type_system/checker.rs` — TypeChecker::new() where registration happens
  - `src/type_system/types.rs` — CoreType::Function definition for built-in signatures
  - `src/type_system/checker/expressions.rs` — Function call type checking (must resolve built-ins)

  **API/Type References**:
  - `src/type_system/symbol_table.rs` — SymbolTable, SymbolInfo for registering built-in symbols
  - `language-spec/simple_quiz.op:8-9` — Import of built-ins from `standard` and `math`

  **Test References**:
  - `src/type_system/tests.rs` — Existing type system tests

  **External References**:
  - `language-spec/hello_world.op` — Uses `print()` (simplest built-in test)
  - `language-spec/simple_quiz.op` — Uses `take_input()`, `string_to_int32()`, `random_int32()`
  - `plan/phase-2-blockers-plan.md:146-148` — Outline for this task

  **Acceptance Criteria**:
  - [ ] TDD: Tests written FIRST
  - [ ] `print<T>(value: T): unit` type-checks correctly
  - [ ] `take_input(): string` type-checks correctly
  - [ ] `string_to_int32(s: string): int32 errors ParseError` type-checks with guard/propagate
  - [ ] `random_int32(min: int32, max: int32): int32` type-checks correctly
  - [ ] Generic `print<T>` instantiates correctly for any type argument
  - [ ] `cargo make test` passes, `cargo make lint` passes

  **QA Scenarios**:

  ```
  Scenario: Built-in functions resolve during type checking
    Tool: Bash (cargo test)
    Preconditions: Built-in registration implemented
    Steps:
      1. Run `cargo make test 2>&1 | tee temp.log`
      2. Read temp.log and verify new built-in tests pass
    Expected Result: All tests pass
    Evidence: .sisyphus/evidence/task-2-builtin-tests.txt

  Scenario: Type error on wrong argument to built-in
    Tool: Bash (cargo test)
    Preconditions: Tests include negative cases
    Steps:
      1. Verify test exists for calling print() with wrong arity
      2. Verify test exists for calling string_to_int32() with non-string arg
      3. Run `cargo make test 2>&1 | tee temp.log`
    Expected Result: Error cases detected and reported with proper spans
    Evidence: .sisyphus/evidence/task-2-builtin-errors.txt
  ```

  **Commit**: YES
  - Message: `feat(type-system): register standard library built-in function signatures`
  - Pre-commit: `cargo make lint-fix && cargo make test`

- [ ] 3. If Expression Semantics (Phase 2 Blocker #5)

  **What to do**:
  - Read all requirements files and PLAN.md
  - Review language spec: Opalescent uses Rust-style if expressions (value-returning)
  - Determine if `Stmt::If` should become `Expr::If` or support both forms
  - If as expression: both branches must return compatible types
  - Else-less if expressions return `unit` type
  - Update parser to emit if expression in expression position
  - Update type checker to infer if expression result type from branch types
  - Ensure both if/else branches return compatible types (unify branch types)
  - Type check else-less if (must return unit or be in statement position)
  - Update constraint collection for if expressions
  - Write TDD tests FIRST: if expression type inference, branch type mismatch, else-less if semantics
  - Run lint, test, line-count checks
  - Update plan files and PLAN.md
  - Commit

  **Must NOT do**:
  - Same guardrails as Task 1

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: Requires careful AST design decisions and type inference integration
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1 (with Tasks 1, 2, 4, 5)
  - **Blocks**: Task 13 (Control Flow)
  - **Blocked By**: None

  **References**:

  **Pattern References**:
  - `src/ast.rs` — Current `Stmt::If` definition
  - `src/parser/statements.rs` — If statement parsing
  - `src/type_system/checker/statements.rs` — If statement type checking (currently enforces boolean condition)

  **API/Type References**:
  - `language-spec/fib_recursive.op:6-9` — If used for value-returning logic
  - `language-spec/requirements/overview.md` — "Rust-style if expressions"
  - `language-spec/requirements/math.md:112` — "using Rust-style if expressions instead"

  **Test References**:
  - `src/type_system/tests.rs` — Type checker tests
  - `src/parser/tests.rs` — Parser tests

  **Acceptance Criteria**:
  - [ ] If-else as expression: inferred type is common type of both branches
  - [ ] Branch type mismatch → clear error with spans
  - [ ] Else-less if yields `unit` type
  - [ ] `cargo make test` passes, `cargo make lint` passes

  **QA Scenarios**:

  ```
  Scenario: If expression infers branch type
    Tool: Bash (cargo test)
    Steps:
      1. Run `cargo make test 2>&1 | tee temp.log`
      2. Verify if-expression type inference tests pass
    Expected Result: All tests pass
    Evidence: .sisyphus/evidence/task-3-if-expr-tests.txt
  ```

  **Commit**: YES
  - Message: `feat(type-system): implement if expression semantics with branch type unification`
  - Pre-commit: `cargo make lint-fix && cargo make test`

- [ ] 4. Warning System Infrastructure (Phase 2 Blocker #9)

  **What to do**:
  - Read all requirements files and PLAN.md
  - Add `Warning` type parallel to `TypeError` with miette integration
  - Add warning collection to `TypeChecker` (`warnings: Vec<Warning>`)
  - Convert `UnsafeCast` from error to warning
  - Implement warning display with miette (diagnostic codes, help text)
  - Add warning categories: unsafe cast, unused variable (placeholder), unreachable code (placeholder), exhaustiveness (placeholder)
  - Support future warning suppression annotations
  - Write TDD tests FIRST: warning creation, unsafe cast as warning, warning collection, miette display
  - Run lint, test, line-count checks
  - Update plan files and PLAN.md
  - Commit

  **Must NOT do**:
  - Same guardrails as Task 1
  - Do not implement full unused variable detection yet (just infrastructure)

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: New error infrastructure parallel to existing TypeError system
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1 (with Tasks 1, 2, 3, 5)
  - **Blocks**: Task 8 (Overflow Detection), Task 12 (Variable System - unused warnings)
  - **Blocked By**: None

  **References**:

  **Pattern References**:
  - `src/type_system/errors.rs` — TypeError definition (Warning should mirror this pattern)
  - `src/type_system/checker.rs` — TypeChecker struct (add warnings field)
  - `src/error.rs` — Lexer error patterns with miette

  **API/Type References**:
  - `ERROR_HANDLING_STANDARDS.md` — Error handling patterns to follow
  - `src/type_system/checker/expressions.rs` — UnsafeCast currently raised as error, convert to warning

  **Acceptance Criteria**:
  - [ ] Warning type created with miette Diagnostic derive
  - [ ] TypeChecker collects warnings alongside errors
  - [ ] UnsafeCast is now a warning, not an error
  - [ ] Warnings display correctly via miette
  - [ ] `cargo make test` passes, `cargo make lint` passes

  **QA Scenarios**:

  ```
  Scenario: Warning infrastructure works
    Tool: Bash (cargo test)
    Steps:
      1. Run `cargo make test 2>&1 | tee temp.log`
      2. Verify warning-related tests pass
    Expected Result: All tests pass
    Evidence: .sisyphus/evidence/task-4-warning-tests.txt
  ```

  **Commit**: YES
  - Message: `feat(type-system): add warning system infrastructure with unsafe cast warnings`
  - Pre-commit: `cargo make lint-fix && cargo make test`

- [ ] 5. Documentation Synchronization (Phase 2 Blockers #10, #11)

  **What to do**:
  - Read PLAN.md and plan/phase-2-blockers-plan.md and plan/type-system-core-plan.md
  - Update `plan/type-system-core-plan.md`: mark error handling as complete, add Phase 2 blocker cross-refs, update constraint solver status, document HasField deferral
  - Update PLAN.md: add "Error Handling System" as standalone phase item, document guard/propagate/errors syntax requirements, add dependency notes, cross-reference blocker items
  - Ensure all checked items in PLAN.md match actual implementation status
  - Run lint check (no code changes, but verify no regressions)
  - Commit docs update

  **Must NOT do**:
  - Do not modify any source code in this task
  - Do not modify Makefile.toml, scripts, or restricted files

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Documentation-only changes, no code modifications
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1 (with Tasks 1, 2, 3, 4)
  - **Blocks**: None
  - **Blocked By**: None

  **References**:

  **Pattern References**:
  - `plan/phase-2-blockers-plan.md` — Current blocker plan (Blocker #1 all checked)
  - `plan/type-system-core-plan.md` — Type system plan to update
  - `PLAN.md` — Master plan to synchronize

  **Acceptance Criteria**:
  - [ ] `plan/type-system-core-plan.md` reflects current implementation status
  - [ ] PLAN.md reflects current implementation status
  - [ ] All cross-references are accurate

  **QA Scenarios**:

  ```
  Scenario: Documentation is accurate
    Tool: Bash (grep)
    Steps:
      1. Verify PLAN.md has error handling blocker #1 marked complete
      2. Verify plan/type-system-core-plan.md is updated
    Expected Result: Docs match implementation state
    Evidence: .sisyphus/evidence/task-5-docs-sync.txt
  ```

  **Commit**: YES
  - Message: `docs: synchronize PLAN.md and type-system-core-plan with completed error handling`
  - Pre-commit: `cargo make lint-fix && cargo make test`

- [ ] 6. Generic Type Parameter Constraints (Phase 2 Blocker #4)

  **What to do**:
  - Read all requirements files, PLAN.md, and `plan/phase-2-blockers-plan.md`
  - Extend AST to support type parameter constraints/bounds (e.g., `<T: Constraint>`)
  - Parse constraint syntax in generic type parameters
  - Store constraints in `Decl::Function` and `Expr::Lambda` for generic functions/lambdas
  - Support multiple constraints per type parameter
  - Validate generic parameters satisfy declared constraints at instantiation sites
  - Implement constraint solving algorithm for type inference
  - Infer generic parameters from function call arguments
  - Validate inferred types satisfy all constraints
  - Support explicit generic parameter syntax: `map<int32, string>(...)`
  - Handle constraint conflicts and report clear errors with spans
  - Write TDD tests FIRST: generic functions with constraints, constraint violation detection, constraint inference from call sites, multiple constraints on single type parameter
  - Run lint, test, line-count checks
  - Update plan files and PLAN.md
  - Commit

  **Must NOT do**:
  - Same guardrails as Task 1
  - Do not implement generic ADT with constraints yet (Phase 3)
  - Do not implement monomorphization yet (Phase 5)

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: Complex type system work requiring constraint solving and inference
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2 (with Tasks 7, 8, 9, 10)
  - **Blocks**: Task 11 (Function System), Task 18 (Generic System)
  - **Blocked By**: Task 1 (Multiple Return Values — function type changes)

  **References**:

  **Pattern References**:
  - `src/type_system/constraints.rs` — Existing constraint infrastructure (TypeConstraint enum)
  - `src/type_system/types.rs` — CoreType::Generic, TypeVar
  - `src/type_system/substitution.rs` — Substitution system
  - `src/ast/types.rs` — Type AST nodes for generics

  **API/Type References**:
  - `language-spec/partition.op:4` — `f<T>(xs: T[], pred: f(T): boolean): Pair<T[]>` — generic function
  - `language-spec/array_helpers.op:27` — `f<T, U>(xs: T[], fn: f(T): U): U[]` — multi-generic
  - `language-spec/types_example.types.op:93` — `f<A, B, Err>(...)` — generic with error propagation

  **Test References**:
  - `src/type_system/tests.rs` — Existing type system tests

  **Acceptance Criteria**:
  - [ ] Parse generic parameters with constraints
  - [ ] Constraint satisfaction checked at instantiation
  - [ ] Constraint violations produce clear error with span
  - [ ] Generic type inference from call arguments works
  - [ ] `cargo make test` passes, `cargo make lint` passes

  **QA Scenarios**:

  ```
  Scenario: Generic constraint checking
    Tool: Bash (cargo test)
    Steps:
      1. Run `cargo make test 2>&1 | tee temp.log`
      2. Verify generic constraint tests pass
    Expected Result: All tests pass
    Evidence: .sisyphus/evidence/task-6-generic-constraints.txt
  ```

  **Commit**: YES
  - Message: `feat(type-system): implement generic type parameter constraints and inference`
  - Pre-commit: `cargo make lint-fix && cargo make test`

- [ ] 7. Member Access Type Checking (Phase 2 Blocker #6)

  **What to do**:
  - Read all requirements files and PLAN.md
  - Implement `type_check_expr` for `Expr::Member` (currently returns `NotImplementedYet`)
  - Type check object/receiver expression first
  - Validate member exists on object type
  - Handle module member access (e.g., `math.sqrt` from `import math as m`)
  - Handle struct field access (basic — full ADT support in Phase 3)
  - Return member type for subsequent type analysis
  - Handle chained member access (e.g., `obj.field.method`)
  - Write TDD tests FIRST: module member access, field access errors (member not found), chained member access
  - Run lint, test, line-count checks
  - Update plan files and PLAN.md
  - Commit

  **Must NOT do**:
  - Same guardrails as Task 1
  - Do not implement full ADT field access yet (Phase 3)

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: Type system integration with module system concepts
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2 (with Tasks 6, 8, 9, 10)
  - **Blocks**: Task 15 (ADT - field access)
  - **Blocked By**: None (but logically follows Wave 1)

  **References**:

  **Pattern References**:
  - `src/type_system/checker/expressions.rs` — Expression type checking (find `Expr::Member` match arm)
  - `src/type_system/symbol_table.rs` — Symbol lookup for member resolution

  **API/Type References**:
  - `language-spec/requirements/modules.md:43-46` — `import math as m; m.sqrt(9)` — module member access
  - `src/ast.rs` — Expr::Member definition

  **Acceptance Criteria**:
  - [ ] Module member access type-checks correctly
  - [ ] Missing member produces error with span and suggestion
  - [ ] Chained member access works
  - [ ] `cargo make test` passes, `cargo make lint` passes

  **QA Scenarios**:

  ```
  Scenario: Member access type checking
    Tool: Bash (cargo test)
    Steps:
      1. Run `cargo make test 2>&1 | tee temp.log`
      2. Verify member access tests pass
    Expected Result: All tests pass
    Evidence: .sisyphus/evidence/task-7-member-access.txt
  ```

  **Commit**: YES
  - Message: `feat(type-system): implement member access type checking for modules and fields`
  - Pre-commit: `cargo make lint-fix && cargo make test`

- [ ] 8. Arithmetic Overflow Detection (Phase 2 Blocker #7)

  **What to do**:
  - Read all requirements files (especially `math.md`) and PLAN.md
  - Detect overflow in constant arithmetic expressions at compile-time
  - Emit errors for overflowing constant additions, multiplications
  - Emit errors for overflowing bitwise shifts (negative and out-of-range counts)
  - Document runtime trap behavior for debug mode (per math.md: "Integer + - * and bshl trap on overflow in Debug")
  - Parse `checked_add`, `wrapping_add`, `saturating_add` variants as stdlib intrinsics
  - Validate use of explicit overflow-handling variants
  - Type check checked arithmetic operations
  - Integrate with Warning system (Task 4) for reporting
  - Write TDD tests FIRST: constant overflow detection, checked/wrapping/saturating variants, shift bound checking
  - Run lint, test, line-count checks
  - Update plan files and PLAN.md
  - Commit

  **Must NOT do**:
  - Same guardrails as Task 1
  - Do not implement runtime trap code generation (Phase 5)

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: Numeric safety analysis with multiple edge cases per math.md spec
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2 (with Tasks 6, 7, 9, 10)
  - **Blocks**: Task 14 (Arithmetic & Logic Completion)
  - **Blocked By**: Task 4 (Warning System — for reporting)

  **References**:

  **Pattern References**:
  - `src/type_system/checker/expressions.rs` — Binary operation type checking
  - `src/type_system/types.rs` — Numeric type representations

  **API/Type References**:
  - `language-spec/requirements/math.md:180-182` — "Integer + - * and bshl trap on overflow in Debug; in Release, use explicit variants"
  - `language-spec/requirements/math.md:126-131` — Shift count rules

  **Acceptance Criteria**:
  - [ ] Compile-time overflow in constant expressions detected
  - [ ] Shift bound violations detected
  - [ ] Checked/wrapping/saturating variants type-check
  - [ ] `cargo make test` passes, `cargo make lint` passes

  **QA Scenarios**:

  ```
  Scenario: Overflow detection
    Tool: Bash (cargo test)
    Steps:
      1. Run `cargo make test 2>&1 | tee temp.log`
      2. Verify overflow detection tests pass
    Expected Result: All tests pass
    Evidence: .sisyphus/evidence/task-8-overflow.txt
  ```

  **Commit**: YES
  - Message: `feat(type-system): implement compile-time arithmetic overflow detection`
  - Pre-commit: `cargo make lint-fix && cargo make test`

- [ ] 9. Division by Zero Detection (Phase 2 Blocker #8)

  **What to do**:
  - Read all requirements files (especially `math.md`) and PLAN.md
  - Detect division by zero in constant expressions at compile-time
  - Detect modulo by zero in constant expressions at compile-time
  - Emit compile-time errors for constant division by zero with clear spans
  - Document runtime trap behavior for non-constant division (per math.md: "a / 0 or a % 0 → runtime trap")
  - Write TDD tests FIRST: constant div by zero, constant mod by zero, runtime trap documentation
  - Run lint, test, line-count checks
  - Update plan files and PLAN.md
  - Commit

  **Must NOT do**:
  - Same guardrails as Task 1
  - Do not implement runtime trap code generation (Phase 5)

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: Numeric safety analysis with compile-time evaluation
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2 (with Tasks 6, 7, 8, 10)
  - **Blocks**: Task 14 (Arithmetic & Logic Completion)
  - **Blocked By**: None

  **References**:

  **Pattern References**:
  - `src/type_system/checker/expressions.rs` — Binary operation type checking
  - `language-spec/requirements/math.md:176-179` — Division by zero spec

  **Acceptance Criteria**:
  - [ ] Constant `x / 0` detected at compile time
  - [ ] Constant `x % 0` detected at compile time
  - [ ] Error messages include clear spans
  - [ ] `cargo make test` passes, `cargo make lint` passes

  **QA Scenarios**:

  ```
  Scenario: Division by zero detection
    Tool: Bash (cargo test)
    Steps:
      1. Run `cargo make test 2>&1 | tee temp.log`
      2. Verify division by zero tests pass
    Expected Result: All tests pass
    Evidence: .sisyphus/evidence/task-9-div-zero.txt
  ```

  **Commit**: YES
  - Message: `feat(type-system): implement compile-time division and modulo by zero detection`
  - Pre-commit: `cargo make lint-fix && cargo make test`

- [ ] 10. Phase 1 Remaining — Integration Tests

  **What to do**:
  - Read all requirements files and PLAN.md
  - Create parser + type checker integration tests
  - Test full pipeline: source string → lexer → parser → type checker → result
  - Test error message quality (miette formatting, span accuracy)
  - Test multi-error reporting (multiple errors in one source)
  - Test that `language-spec/hello_world.op` parses and type-checks (after Task 2 built-ins)
  - Test that `language-spec/fib_recursive.op` parses and type-checks
  - Test that `language-spec/fib_iterative.op` parses and type-checks
  - Organize type system tests into separate modules by category if needed
  - Run lint, test, line-count checks
  - Update plan files and PLAN.md
  - Commit

  **Must NOT do**:
  - Same guardrails as Task 1
  - Do not implement features — only test existing ones

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
    - Reason: Test-focused task requiring broad codebase understanding
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2 (with Tasks 6, 7, 8, 9)
  - **Blocks**: Task 11-14 (Phase 2 features)
  - **Blocked By**: Task 2 (Standard Library Built-ins — needed for .op file tests)

  **References**:

  **Pattern References**:
  - `src/parser/tests.rs` — Parser tests
  - `src/type_system/tests.rs` — Type system tests
  - `language-spec/error_handling_samples.op` — Error handling integration sample

  **API/Type References**:
  - `language-spec/hello_world.op` — Simplest integration test
  - `language-spec/fib_recursive.op` — Recursive function integration
  - `language-spec/fib_iterative.op` — Loop and mutable variable integration

  **Acceptance Criteria**:
  - [ ] Integration tests cover parse → type check pipeline
  - [ ] Error message quality tests verify span accuracy
  - [ ] At least hello_world.op parses and type-checks
  - [ ] `cargo make test` passes, `cargo make lint` passes

  **QA Scenarios**:

  ```
  Scenario: Integration test suite
    Tool: Bash (cargo test)
    Steps:
      1. Run `cargo make test 2>&1 | tee temp.log`
      2. Verify integration test count and pass rate
    Expected Result: All tests pass
    Evidence: .sisyphus/evidence/task-10-integration-tests.txt
  ```

  **Commit**: YES
  - Message: `test: add parser-type-checker integration tests with language-spec validation`
  - Pre-commit: `cargo make lint-fix && cargo make test`

- [ ] 11. Function System Completion (Phase 2)

  **What to do**:
  - Read all requirements files, PLAN.md, `plan/function-system-plan.md`
  - Implement function call resolution beyond basic arity checking (overload resolution, generic instantiation at call sites)
  - Implement entry point validation: exactly one `entry` keyword per program, proper signature check
  - Complete scope management for parameters and local variables (nested scopes, closures)
  - Integrate generic function inference with constraint system (from Task 6)
  - Add hot-reload metadata propagation for functions (function signature stability tracking per HOT_RELOAD_ARCHITECTURE.md)
  - Write integration tests for function system with error handling (guard/propagate) and multiple returns
  - Add documentation for all new function system code
  - Run lint, test, line-count checks
  - Update `plan/function-system-plan.md` and PLAN.md
  - Commit

  **Must NOT do**:
  - Same guardrails as Task 1
  - Do not implement code generation for functions (Phase 5)
  - Do not implement monomorphization (Phase 5)

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: Complex integration across type system, AST, and parser with generic inference
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 3 (with Tasks 12, 13, 14)
  - **Blocks**: Task 15 (ADT Pattern Matching), Task 19 (Import/Export)
  - **Blocked By**: Tasks 1, 2, 6 (multi-return, built-ins, generic constraints)

  **References**:

  **Pattern References**:
  - `src/type_system/checker/declarations.rs` — Function declaration type checking (extend)
  - `src/type_system/checker/expressions.rs` — Function call type checking (extend resolution)
  - `src/type_system/constraints.rs` — Constraint system for generic instantiation
  - `src/type_system/symbol_table.rs` — Scope management

  **API/Type References**:
  - `language-spec/requirements/overview.md:22-23` — Explicit return keyword required
  - `HOT_RELOAD_ARCHITECTURE.md:97-100` — Function signature stability tracking
  - `plan/function-system-plan.md` — Full function system plan

  **Acceptance Criteria**:
  - [ ] Entry point validation works (exactly one `entry`)
  - [ ] Function call resolution handles generic instantiation
  - [ ] Scope management correct for nested functions
  - [ ] Hot-reload metadata propagation implemented
  - [ ] Integration tests for functions + error handling + multi-return
  - [ ] `cargo make test` passes, `cargo make lint` passes

  **QA Scenarios**:

  ```
  Scenario: Function system integration
    Tool: Bash (cargo test)
    Steps:
      1. Run `cargo make test 2>&1 | tee temp.log`
      2. Verify function system tests pass including entry point validation
    Expected Result: All tests pass
    Evidence: .sisyphus/evidence/task-11-function-system.txt

  Scenario: Entry point validation rejects duplicates
    Tool: Bash (cargo test)
    Steps:
      1. Verify test exists for program with two `entry` functions
      2. Run `cargo make test 2>&1 | tee temp.log`
    Expected Result: Duplicate entry point produces clear error
    Evidence: .sisyphus/evidence/task-11-entry-validation.txt
  ```

  **Commit**: YES
  - Message: `feat(type-system): complete function system with entry validation and generic call resolution`
  - Pre-commit: `cargo make lint-fix && cargo make test`

- [ ] 12. Variable System Completion (Phase 2)

  **What to do**:
  - Read all requirements files, PLAN.md (Variable System section, lines 437-451)
  - Implement assignment to mutable variables validation (only `mut` variables can be reassigned)
  - Implement mutation of immutable variables error detection with clear error messages
  - Implement unused variable warnings (integrate with Warning system from Task 4)
  - Track variable usage through type checking pass
  - Report unused variables as warnings (not errors)
  - Ensure shadowing still works correctly with mutation tracking
  - Write TDD tests FIRST: mutable assignment, immutable mutation error, unused variable warning
  - Run lint, test, line-count checks
  - Update PLAN.md (Variable System section)
  - Commit

  **Must NOT do**:
  - Same guardrails as Task 1
  - Do not implement move semantics or ownership tracking

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: Variable tracking across scopes with warning system integration
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 3 (with Tasks 11, 13, 14)
  - **Blocks**: Task 19 (Import/Export — variable visibility)
  - **Blocked By**: Task 4 (Warning System)

  **References**:

  **Pattern References**:
  - `src/type_system/checker/statements.rs` — Assignment statement checking (extend for mutability)
  - `src/type_system/symbol_table.rs` — SymbolInfo (add mutability tracking, usage tracking)
  - `src/type_system/checker/declarations.rs` — Let binding handling

  **API/Type References**:
  - `language-spec/requirements/overview.md:28-30` — `let` immutable, `let mut` mutable
  - `PLAN.md:449-451` — Variable system remaining items

  **Acceptance Criteria**:
  - [ ] Assigning to immutable variable → error with span
  - [ ] Assigning to mutable variable → success
  - [ ] Unused variable → warning (not error)
  - [ ] Shadowing still works with mutation tracking
  - [ ] `cargo make test` passes, `cargo make lint` passes

  **QA Scenarios**:

  ```
  Scenario: Immutable variable mutation detected
    Tool: Bash (cargo test)
    Steps:
      1. Run `cargo make test 2>&1 | tee temp.log`
      2. Verify immutable mutation error tests pass
    Expected Result: Clear error message for immutable mutation
    Evidence: .sisyphus/evidence/task-12-immutable-error.txt

  Scenario: Unused variable produces warning
    Tool: Bash (cargo test)
    Steps:
      1. Run `cargo make test 2>&1 | tee temp.log`
      2. Verify unused variable warning tests pass
    Expected Result: Warning (not error) for unused variables
    Evidence: .sisyphus/evidence/task-12-unused-warning.txt
  ```

  **Commit**: YES
  - Message: `feat(type-system): complete variable system with mutability validation and unused warnings`
  - Pre-commit: `cargo make lint-fix && cargo make test`

- [ ] 13. Control Flow Completion (Phase 2)

  **What to do**:
  - Read all requirements files, PLAN.md (Control Flow section, lines 453-473)
  - Implement if expression value-returning semantics (from Task 3 foundation)
  - Implement exhaustiveness checking for if expressions (all paths return a value)
  - Implement control flow analysis for unreachable code detection
  - Implement type narrowing in conditional branches (e.g., after `is` check, narrow type)
  - Emit unreachable code warnings (integrate with Warning system from Task 4)
  - Write TDD tests FIRST: exhaustiveness checking, unreachable code, type narrowing
  - Run lint, test, line-count checks
  - Update PLAN.md (Control Flow section)
  - Commit

  **Must NOT do**:
  - Same guardrails as Task 1
  - Do not implement pattern matching control flow (Phase 3)

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: Control flow analysis with type narrowing requires careful graph traversal
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 3 (with Tasks 11, 12, 14)
  - **Blocks**: Task 15 (ADT Pattern Matching — exhaustiveness patterns)
  - **Blocked By**: Task 3 (If Expression Semantics), Task 4 (Warning System)

  **References**:

  **Pattern References**:
  - `src/type_system/checker/statements.rs` — If/for/while/loop type checking
  - `src/type_system/checker/expressions.rs` — Expression type checking for narrowing

  **API/Type References**:
  - `PLAN.md:469-473` — Control flow remaining items
  - `language-spec/fib_recursive.op:6-9` — If expression for value return
  - `language-spec/fib_iterative.op` — Loop with break returning value

  **Acceptance Criteria**:
  - [ ] Exhaustiveness checking for if expressions works
  - [ ] Unreachable code detected and reported as warning
  - [ ] Type narrowing in conditional branches
  - [ ] `cargo make test` passes, `cargo make lint` passes

  **QA Scenarios**:

  ```
  Scenario: Exhaustiveness and unreachable code
    Tool: Bash (cargo test)
    Steps:
      1. Run `cargo make test 2>&1 | tee temp.log`
      2. Verify exhaustiveness and unreachable code tests pass
    Expected Result: All control flow tests pass
    Evidence: .sisyphus/evidence/task-13-control-flow.txt
  ```

  **Commit**: YES
  - Message: `feat(type-system): complete control flow with exhaustiveness, unreachable code, type narrowing`
  - Pre-commit: `cargo make lint-fix && cargo make test`

- [ ] 14. Arithmetic & Logic Completion (Phase 2)

  **What to do**:
  - Read all requirements files (especially `math.md`), PLAN.md (Arithmetic & Logic section, lines 474-492)
  - Build on overflow detection (Task 8) and div-by-zero detection (Task 9)
  - Implement compile-time constant folding for arithmetic expressions
  - Implement bitwise shift bounds checking (negative and out-of-range shift counts per math.md)
  - Implement masked/wrapping bitwise shift variants
  - Document runtime arithmetic overflow handling strategy (debug trap, release wrap per math.md)
  - Prepare arithmetic operations for code generation (annotate with overflow mode)
  - Write TDD tests FIRST: shift bounds, constant folding, masked shifts
  - Run lint, test, line-count checks
  - Update PLAN.md (Arithmetic & Logic section)
  - Commit

  **Must NOT do**:
  - Same guardrails as Task 1
  - Do not implement actual code generation for arithmetic (Phase 5)

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: Numeric edge cases and spec compliance with math.md
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 3 (with Tasks 11, 12, 13)
  - **Blocks**: Task 22 (Code Generation - Expressions)
  - **Blocked By**: Tasks 8, 9 (Overflow Detection, Div-by-Zero)

  **References**:

  **Pattern References**:
  - `src/type_system/checker/expressions.rs` — Binary operation type checking
  - `language-spec/requirements/math.md` — Complete arithmetic specification

  **API/Type References**:
  - `language-spec/requirements/math.md:126-131` — Shift count rules
  - `language-spec/requirements/math.md:180-182` — Overflow handling modes
  - `PLAN.md:487-492` — Arithmetic remaining items

  **Acceptance Criteria**:
  - [ ] Bitwise shift bounds checking works (negative, out-of-range)
  - [ ] Constant folding for arithmetic expressions
  - [ ] Masked/wrapping shift variants type-check
  - [ ] `cargo make test` passes, `cargo make lint` passes

  **QA Scenarios**:

  ```
  Scenario: Arithmetic completion
    Tool: Bash (cargo test)
    Steps:
      1. Run `cargo make test 2>&1 | tee temp.log`
      2. Verify arithmetic and shift tests pass
    Expected Result: All tests pass
    Evidence: .sisyphus/evidence/task-14-arithmetic.txt
  ```

  **Commit**: YES
  - Message: `feat(type-system): complete arithmetic with shift bounds, constant folding, masked shifts`
  - Pre-commit: `cargo make lint-fix && cargo make test`

- [ ] 15. ADT Implementation — Pattern Matching (Phase 3)

  **What to do**:
  - Read all requirements files, PLAN.md (ADT Implementation section, lines 496-510)
  - Implement pattern matching type checking (each arm receives correct variant types)
  - Implement exhaustiveness checking for match expressions (all variants covered)
  - Support wildcard patterns (`_`)
  - Support nested patterns (patterns within patterns)
  - Support guard clauses on match arms (`if condition`)
  - Ensure match expressions return consistent types across all arms
  - Integrate with control flow exhaustiveness from Task 13
  - Write TDD tests FIRST: basic match, exhaustiveness, wildcard, nested patterns, guard clauses
  - Run lint, test, line-count checks
  - Update PLAN.md (ADT Implementation section)
  - Commit

  **Must NOT do**:
  - Same guardrails as Task 1
  - Do not implement code generation for pattern matching (Phase 5)

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: Pattern matching with exhaustiveness is algorithmically complex
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 4 (with Tasks 16, 17, 18)
  - **Blocks**: Task 19 (Import/Export — ADT exports)
  - **Blocked By**: Tasks 11, 13 (Function System, Control Flow — exhaustiveness patterns)

  **References**:

  **Pattern References**:
  - `src/ast.rs` — Existing Stmt/Expr definitions (add match expression)
  - `src/parser/statements.rs` — Statement parsing (add match parsing)
  - `src/type_system/checker/statements.rs` — Statement type checking

  **API/Type References**:
  - `language-spec/partition.op:14-17` — Pattern matching on list: `match xs { [] => ..., [head, ...rest] => ... }`
  - `language-spec/types_example.types.op:36-42` — Sum type variants for matching
  - `PLAN.md:504-510` — ADT remaining items

  **Acceptance Criteria**:
  - [ ] Match expression parses correctly
  - [ ] Exhaustiveness checking detects missing variants
  - [ ] Wildcard pattern catches remaining cases
  - [ ] Guard clauses on match arms work
  - [ ] All arms return compatible types
  - [ ] `cargo make test` passes, `cargo make lint` passes

  **QA Scenarios**:

  ```
  Scenario: Pattern matching exhaustiveness
    Tool: Bash (cargo test)
    Steps:
      1. Run `cargo make test 2>&1 | tee temp.log`
      2. Verify pattern matching and exhaustiveness tests pass
    Expected Result: All tests pass
    Evidence: .sisyphus/evidence/task-15-pattern-matching.txt

  Scenario: Non-exhaustive match produces error
    Tool: Bash (cargo test)
    Steps:
      1. Verify test exists for match missing a variant
      2. Run `cargo make test 2>&1 | tee temp.log`
    Expected Result: Clear error for non-exhaustive match
    Evidence: .sisyphus/evidence/task-15-non-exhaustive.txt
  ```

  **Commit**: YES
  - Message: `feat(type-system): implement pattern matching with exhaustiveness checking`
  - Pre-commit: `cargo make lint-fix && cargo make test`

- [ ] 16. ADT Implementation — Constructors & Fields (Phase 3)

  **What to do**:
  - Read all requirements files, PLAN.md (ADT Implementation section, lines 496-510)
  - Implement ADT constructor type checking (sum type variant constructors)
  - Implement product type (struct) field access type checking (integrate with member access from Task 7)
  - Implement HasField constraint solving (deferred from Phase 1)
  - Validate field types in product type construction
  - Support named field initialization
  - Support positional variant construction for sum types
  - Validate variant existence in sum type construction
  - Write TDD tests FIRST: constructor validation, field access, HasField constraints
  - Run lint, test, line-count checks
  - Update PLAN.md (ADT Implementation section)
  - Commit

  **Must NOT do**:
  - Same guardrails as Task 1
  - Do not implement generic ADT instantiation yet (Task 18)

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: ADT construction and field access with constraint solving
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 4 (with Tasks 15, 17, 18)
  - **Blocks**: Task 19 (Import/Export — type exports)
  - **Blocked By**: Task 7 (Member Access)

  **References**:

  **Pattern References**:
  - `src/type_system/checker/declarations.rs` — Type declaration checking
  - `src/type_system/checker/expressions.rs` — Member access (extend for field access)
  - `src/type_system/constraints.rs` — HasField constraint (implement solving)

  **API/Type References**:
  - `language-spec/types_example.types.op:1-50` — Full ADT type definitions
  - `language-spec/linked_list.op:1-7` — Generic ADT: `type Node<T> = { value: T, next: Node<T>? }`
  - `PLAN.md:508-510` — ADT constructor and field access items

  **Acceptance Criteria**:
  - [ ] Sum type variant constructors type-check
  - [ ] Product type field access type-checks
  - [ ] HasField constraint solving works
  - [ ] Field type mismatches produce clear errors
  - [ ] `cargo make test` passes, `cargo make lint` passes

  **QA Scenarios**:

  ```
  Scenario: ADT constructor and field access
    Tool: Bash (cargo test)
    Steps:
      1. Run `cargo make test 2>&1 | tee temp.log`
      2. Verify ADT constructor and field tests pass
    Expected Result: All tests pass
    Evidence: .sisyphus/evidence/task-16-adt-constructors.txt
  ```

  **Commit**: YES
  - Message: `feat(type-system): implement ADT constructors, field access, and HasField constraints`
  - Pre-commit: `cargo make lint-fix && cargo make test`

- [ ] 17. Array & Collection Type Completion (Phase 3)

  **What to do**:
  - Read all requirements files, PLAN.md (Array & Collection Support section, lines 512-529)
  - Implement array method type checking (push, pop, length, map, filter, etc.)
  - Implement string manipulation method type checking
  - Define iterator trait/interface for collection iteration
  - Extend for-loop type checking to support any iterable (not just arrays)
  - Support collection method chaining type inference
  - Plan memory management strategy for collections (document for Phase 5)
  - Write TDD tests FIRST: array methods, string methods, iterator interface, for-loop with iterables
  - Run lint, test, line-count checks
  - Update PLAN.md (Collections section)
  - Commit

  **Must NOT do**:
  - Same guardrails as Task 1
  - Do not implement runtime collection operations (Phase 5/8)

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: Method resolution and iterator trait design
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 4 (with Tasks 15, 16, 18)
  - **Blocks**: Task 35 (Collections Library)
  - **Blocked By**: Task 7 (Member Access — method calls)

  **References**:

  **Pattern References**:
  - `src/type_system/checker/expressions.rs` — Array access, member access
  - `src/type_system/checker/statements.rs` — For-loop iteration checking

  **API/Type References**:
  - `language-spec/array_helpers.op` — Array helper functions (map, filter, reduce, zip)
  - `language-spec/partition.op` — Array pattern matching and list operations
  - `PLAN.md:525-529` — Collection remaining items

  **Acceptance Criteria**:
  - [ ] Array methods (push, pop, length, map, filter) type-check
  - [ ] String methods type-check
  - [ ] Iterator interface defined
  - [ ] For-loop works with any iterable
  - [ ] `cargo make test` passes, `cargo make lint` passes

  **QA Scenarios**:

  ```
  Scenario: Collection methods
    Tool: Bash (cargo test)
    Steps:
      1. Run `cargo make test 2>&1 | tee temp.log`
      2. Verify collection and iterator tests pass
    Expected Result: All tests pass
    Evidence: .sisyphus/evidence/task-17-collections.txt
  ```

  **Commit**: YES
  - Message: `feat(type-system): implement array/string methods and iterator interface`
  - Pre-commit: `cargo make lint-fix && cargo make test`

- [ ] 18. Generic System Completion (Phase 3)

  **What to do**:
  - Read all requirements files, PLAN.md (Generic System section, lines 531-546), `plan/generic-type-parsing-plan.md`
  - Build on generic constraints from Task 6
  - Implement generic ADT instantiation (e.g., `Node<int32>` from `type Node<T>`)
  - Implement type parameter bounds/constraints on ADT type parameters
  - Complete type parameter inference at all call sites
  - Validate concrete type arguments satisfy all constraints
  - Prepare monomorphization infrastructure for Phase 5 (type maps, instantiation tracking)
  - Write TDD tests FIRST: generic ADT instantiation, constraint satisfaction on ADTs, inference at call sites
  - Run lint, test, line-count checks
  - Update PLAN.md (Generic System section)
  - Commit

  **Must NOT do**:
  - Same guardrails as Task 1
  - Do not implement actual monomorphization (Phase 5 code generation)

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: Generic instantiation with constraint satisfaction is complex type theory
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 4 (with Tasks 15, 16, 17)
  - **Blocks**: Task 25 (Code Generation — ADTs & Generics)
  - **Blocked By**: Task 6 (Generic Constraints), Task 16 (ADT Constructors)

  **References**:

  **Pattern References**:
  - `src/type_system/constraints.rs` — Constraint system
  - `src/type_system/substitution.rs` — Type substitution for instantiation
  - `src/type_system/types.rs` — CoreType::Generic, TypeVar

  **API/Type References**:
  - `language-spec/linked_list.op:1-7` — `type Node<T>` — generic ADT
  - `language-spec/array_helpers.op:27` — `f<T, U>(xs: T[], fn: f(T): U): U[]` — multi-generic function
  - `PLAN.md:541-546` — Generic system remaining items

  **Acceptance Criteria**:
  - [ ] Generic ADT instantiation works (`Node<int32>`)
  - [ ] Constraint satisfaction on generic ADTs
  - [ ] Type inference at all call sites
  - [ ] Monomorphization infrastructure prepared (type maps)
  - [ ] `cargo make test` passes, `cargo make lint` passes

  **QA Scenarios**:

  ```
  Scenario: Generic system completion
    Tool: Bash (cargo test)
    Steps:
      1. Run `cargo make test 2>&1 | tee temp.log`
      2. Verify generic ADT and inference tests pass
    Expected Result: All tests pass
    Evidence: .sisyphus/evidence/task-18-generics.txt
  ```

  **Commit**: YES
  - Message: `feat(type-system): complete generic system with ADT instantiation and monomorphization prep`
  - Pre-commit: `cargo make lint-fix && cargo make test`

- [ ] 19. Import/Export Resolution (Phase 4)

  **What to do**:
  - Read all requirements files, PLAN.md (Import/Export System section, lines 548-567), `language-spec/requirements/modules.md`
  - Implement standard library import resolution (`import standard`, `import math`)
  - Implement package import resolution (`@scope/name`)
  - Implement local file import resolution (`./path`)
  - Implement import path validation (file exists, module exports match)
  - Implement export validation (no duplicate exports, `public` keyword enforcement)
  - Implement type checking for imported symbols
  - Implement dependency resolution (topological sort of module graph)
  - Implement circular dependency detection with clear error messages
  - Generate module interfaces for cross-module type checking
  - Write TDD tests FIRST: import resolution, export validation, circular dependency detection, cross-module type checking
  - Run lint, test, line-count checks
  - Update PLAN.md (Module System section)
  - Commit

  **Must NOT do**:
  - Same guardrails as Task 1
  - Do not implement module-level code generation (Phase 5)
  - Do not implement actual file system access in tests (mock module contents)

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: Module resolution with dependency graph and circular detection
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 5 (with Task 20)
  - **Blocks**: Task 21 (LLVM Backend — needs module structure)
  - **Blocked By**: Tasks 11, 15, 16 (Function System, ADTs — exportable types)

  **References**:

  **Pattern References**:
  - `src/ast.rs` — Import/export AST nodes
  - `src/parser/declarations.rs` — Import parsing (already complete)
  - `src/type_system/checker/declarations.rs` — Import declaration checking (currently deferred)

  **API/Type References**:
  - `language-spec/requirements/modules.md` — Full module system specification
  - `language-spec/simple_quiz.op:8-9` — `import standard; import math`
  - `language-spec/types_example.types.op:1-3` — Type-only imports
  - `PLAN.md:559-567` — Module system remaining items

  **Acceptance Criteria**:
  - [ ] Standard library imports resolve
  - [ ] Local file imports resolve with path validation
  - [ ] Circular dependencies detected with clear error
  - [ ] Export validation enforces `public` keyword
  - [ ] Cross-module type checking works
  - [ ] `cargo make test` passes, `cargo make lint` passes

  **QA Scenarios**:

  ```
  Scenario: Module import resolution
    Tool: Bash (cargo test)
    Steps:
      1. Run `cargo make test 2>&1 | tee temp.log`
      2. Verify import resolution and circular dependency tests pass
    Expected Result: All tests pass
    Evidence: .sisyphus/evidence/task-19-imports.txt

  Scenario: Circular dependency detection
    Tool: Bash (cargo test)
    Steps:
      1. Verify test exists for A imports B imports A scenario
      2. Run `cargo make test 2>&1 | tee temp.log`
    Expected Result: Clear error message for circular dependency
    Evidence: .sisyphus/evidence/task-19-circular.txt
  ```

  **Commit**: YES
  - Message: `feat(modules): implement import/export resolution with dependency graph and circular detection`
  - Pre-commit: `cargo make lint-fix && cargo make test`

- [ ] 20. Module Validation (Phase 4)

  **What to do**:
  - Read all requirements files, PLAN.md (Module Validation section, lines 568-574)
  - Implement name clash resolution across modules
  - Implement symbol visibility rules (public vs private across module boundaries)
  - Implement module interface generation (public API surface for each module)
  - Implement cross-module type checking (types from imported modules validate correctly)
  - Validate that imported symbols match their declared types
  - Implement aliased imports (`import math as m; m.sqrt()`)
  - Write TDD tests FIRST: name clashes, visibility rules, module interfaces, aliased imports
  - Run lint, test, line-count checks
  - Update PLAN.md (Module Validation section)
  - Commit

  **Must NOT do**:
  - Same guardrails as Task 1

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: Cross-module validation with visibility and name resolution
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 5 (with Task 19)
  - **Blocks**: Task 21 (LLVM Backend)
  - **Blocked By**: Task 19 (Import/Export Resolution)

  **References**:

  **Pattern References**:
  - `src/type_system/symbol_table.rs` — SymbolTable with Visibility enum
  - `src/type_system/checker/declarations.rs` — Declaration checking

  **API/Type References**:
  - `language-spec/requirements/modules.md:43-46` — Aliased imports
  - `PLAN.md:570-575` — Module validation items

  **Acceptance Criteria**:
  - [ ] Name clash resolution across modules works
  - [ ] Visibility rules enforced (private symbols not accessible)
  - [ ] Module interfaces generated
  - [ ] Aliased imports work correctly
  - [ ] `cargo make test` passes, `cargo make lint` passes

  **QA Scenarios**:

  ```
  Scenario: Module validation
    Tool: Bash (cargo test)
    Steps:
      1. Run `cargo make test 2>&1 | tee temp.log`
      2. Verify module validation tests pass
    Expected Result: All tests pass
    Evidence: .sisyphus/evidence/task-20-module-validation.txt
  ```

  **Commit**: YES
  - Message: `feat(modules): implement module validation with visibility rules and interface generation`
  - Pre-commit: `cargo make lint-fix && cargo make test`

- [ ] 21. LLVM Backend Setup (Phase 5)

  **What to do**:
  - Read all requirements files, PLAN.md (LLVM Backend Setup section, lines 576-584)
  - Add `inkwell` (LLVM Rust bindings) as dependency
  - Create `src/codegen/` module structure: `mod.rs`, `context.rs`, `types.rs`, `values.rs`
  - Implement `CodegenContext` struct wrapping LLVM context, module, builder
  - Implement type mapping: `CoreType` → LLVM types (int32→i32, float64→f64, string→i8*, etc.)
  - Implement basic module creation and target triple configuration
  - Support target platform detection (x86_64, aarch64)
  - Create initial compilation pipeline: AST → LLVM IR → object file
  - Write TDD tests FIRST: type mapping, module creation, basic IR generation
  - Run lint, test, line-count checks
  - Update PLAN.md
  - Commit

  **Must NOT do**:
  - Same guardrails as Task 1
  - Do not implement expression/statement codegen yet (Tasks 22-23)
  - Do not implement optimization passes yet (Task 27)

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: LLVM integration is complex infrastructure work
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 6 (with Tasks 22, 23, 24)
  - **Blocks**: Tasks 22, 23, 25 (expression/function/ADT codegen)
  - **Blocked By**: Tasks 19, 20 (Module System — complete type information)

  **References**:

  **Pattern References**:
  - `src/type_system/types.rs` — CoreType definitions (map to LLVM types)
  - `src/ast.rs` — AST node definitions (codegen input)

  **API/Type References**:
  - `PLAN.md:580-585` — LLVM backend items
  - `HOT_RELOAD_ARCHITECTURE.md:9-14` — Dynamic library compilation requirements

  **External References**:
  - inkwell crate documentation — LLVM Rust bindings API

  **Acceptance Criteria**:
  - [ ] `inkwell` dependency added and compiles
  - [ ] `src/codegen/` module structure created
  - [ ] CoreType → LLVM type mapping works for all types
  - [ ] Module creation with target triple works
  - [ ] Basic compilation pipeline produces object file
  - [ ] `cargo make test` passes, `cargo make lint` passes

  **QA Scenarios**:

  ```
  Scenario: LLVM backend setup
    Tool: Bash (cargo test)
    Steps:
      1. Run `cargo make test 2>&1 | tee temp.log`
      2. Verify codegen module tests pass
    Expected Result: All tests pass, LLVM types map correctly
    Evidence: .sisyphus/evidence/task-21-llvm-setup.txt

  Scenario: Build succeeds with LLVM dependency
    Tool: Bash (cargo make build-all)
    Steps:
      1. Run `cargo make build-all 2>&1 | tee temp.log`
      2. Read temp.log and verify successful compilation
    Expected Result: Build succeeds
    Evidence: .sisyphus/evidence/task-21-build.txt
  ```

  **Commit**: YES
  - Message: `feat(codegen): set up LLVM backend with inkwell, type mapping, and compilation pipeline`
  - Pre-commit: `cargo make lint-fix && cargo make test`

- [ ] 22. Code Generation — Expressions & Statements (Phase 5)

  **What to do**:
  - Read all requirements files, PLAN.md
  - Implement codegen for literal expressions (integers, floats, booleans, strings, unit)
  - Implement codegen for binary operations (arithmetic, comparison, logical, bitwise)
  - Implement codegen for unary operations
  - Implement codegen for variable references (load from alloca)
  - Implement codegen for let bindings (alloca + store)
  - Implement codegen for assignment statements
  - Implement codegen for cast expressions (using LLVM conversion instructions)
  - Implement codegen for array literals and array access
  - Implement arithmetic overflow trapping in debug mode (per math.md spec)
  - Implement division by zero runtime trapping
  - Write TDD tests FIRST: each expression type generates correct IR
  - Run lint, test, line-count checks
  - Update PLAN.md
  - Commit

  **Must NOT do**:
  - Same guardrails as Task 1
  - Do not implement function codegen (Task 23)
  - Do not implement ADT codegen (Task 25)

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: Expression-level code generation with numeric safety
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 6 (with Tasks 21, 23, 24)
  - **Blocks**: Task 25 (ADT codegen)
  - **Blocked By**: Task 21 (LLVM Backend Setup)

  **References**:

  **Pattern References**:
  - `src/codegen/` — LLVM backend (from Task 21)
  - `src/type_system/checker/expressions.rs` — Expression types to generate code for

  **API/Type References**:
  - `language-spec/requirements/math.md:180-182` — Overflow trapping in debug mode
  - `language-spec/requirements/overview.md:39-48` — Cast rules

  **Acceptance Criteria**:
  - [ ] All literal types generate correct LLVM IR
  - [ ] Binary operations generate correct instructions
  - [ ] Variable load/store works
  - [ ] Cast expressions use correct LLVM conversions
  - [ ] Overflow trapping in debug mode
  - [ ] `cargo make test` passes, `cargo make lint` passes

  **QA Scenarios**:

  ```
  Scenario: Expression codegen
    Tool: Bash (cargo test)
    Steps:
      1. Run `cargo make test 2>&1 | tee temp.log`
      2. Verify expression codegen tests pass
    Expected Result: All tests pass
    Evidence: .sisyphus/evidence/task-22-expr-codegen.txt
  ```

  **Commit**: YES
  - Message: `feat(codegen): implement expression and statement code generation with overflow trapping`
  - Pre-commit: `cargo make lint-fix && cargo make test`

- [ ] 23. Code Generation — Functions & Control Flow (Phase 5)

  **What to do**:
  - Read all requirements files, PLAN.md
  - Implement function declaration codegen (LLVM function creation, parameter handling)
  - Implement function call codegen (argument passing, return value handling)
  - Implement multiple return value codegen (struct return or multi-value)
  - Implement lambda/closure codegen
  - Implement if/else codegen (conditional branching with phi nodes)
  - Implement for/while/loop codegen (loop headers, back edges, break/continue)
  - Implement return statement codegen
  - Implement entry point codegen (main function wrapping)
  - Implement error handling codegen (guard/propagate → result type lowering)
  - Write TDD tests FIRST: function codegen, control flow branching, error handling lowering
  - Run lint, test, line-count checks
  - Update PLAN.md
  - Commit

  **Must NOT do**:
  - Same guardrails as Task 1
  - Do not implement optimization passes (Task 27)

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: Function and control flow codegen is the core compilation path
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 6 (with Tasks 21, 22, 24)
  - **Blocks**: Task 25 (ADT codegen), Task 28 (Hot Reload)
  - **Blocked By**: Task 21 (LLVM Backend Setup)

  **References**:

  **Pattern References**:
  - `src/codegen/` — LLVM backend infrastructure
  - `src/type_system/checker/declarations.rs` — Function structure
  - `src/type_system/checker/statements.rs` — Control flow structure

  **API/Type References**:
  - `language-spec/requirements/overview.md:22-23` — Explicit return keyword
  - `language-spec/fib_recursive.op` — Recursive function example
  - `language-spec/fib_iterative.op` — Loop with break example

  **Acceptance Criteria**:
  - [ ] Functions compile to LLVM IR with correct calling convention
  - [ ] Control flow generates correct basic blocks
  - [ ] Error handling lowered to result types
  - [ ] Entry point generates main function
  - [ ] `cargo make test` passes, `cargo make lint` passes

  **QA Scenarios**:

  ```
  Scenario: Function and control flow codegen
    Tool: Bash (cargo test)
    Steps:
      1. Run `cargo make test 2>&1 | tee temp.log`
      2. Verify function and control flow codegen tests pass
    Expected Result: All tests pass
    Evidence: .sisyphus/evidence/task-23-function-codegen.txt
  ```

  **Commit**: YES
  - Message: `feat(codegen): implement function and control flow code generation with error handling`
  - Pre-commit: `cargo make lint-fix && cargo make test`

- [ ] 24. Runtime System Foundation (Phase 5)

  **What to do**:
  - Read all requirements files, PLAN.md (Runtime System section, lines 586-592)
  - Create `src/runtime/` module structure
  - Implement runtime library foundation (entry point bootstrap, panic handler)
  - Implement basic memory allocator interface (for string/array heap allocations)
  - Implement string runtime support (allocation, concatenation, comparison)
  - Implement array runtime support (allocation, indexing, bounds checking)
  - Implement `print()` runtime function (stdout output)
  - Implement `take_input()` runtime function (stdin reading)
  - Implement error handling runtime (result type representation, propagation)
  - Document garbage collection strategy decision (reference counting vs tracing vs manual)
  - Write TDD tests FIRST: runtime functions, memory allocation, string/array operations
  - Run lint, test, line-count checks
  - Update PLAN.md
  - Commit

  **Must NOT do**:
  - Same guardrails as Task 1
  - Do not implement GC yet (document decision for future)
  - Tests must mock all I/O — no actual stdin/stdout in tests

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: Runtime system design with memory management
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 6 (with Tasks 21, 22, 23)
  - **Blocks**: Task 26 (Runtime Memory & Stdlib)
  - **Blocked By**: Task 21 (LLVM Backend — runtime links to compiled code)

  **References**:

  **Pattern References**:
  - `src/type_system/types.rs` — CoreType memory layouts (MemoryLayout struct)
  - `HOT_RELOAD_ARCHITECTURE.md` — Host process memory management

  **API/Type References**:
  - `PLAN.md:588-593` — Runtime system items
  - `language-spec/hello_world.op` — Simplest runtime test (`print("Hello, World!")`)

  **Acceptance Criteria**:
  - [ ] Runtime module structure created
  - [ ] print() and take_input() runtime functions implemented
  - [ ] String/array memory allocation works
  - [ ] Error handling runtime representation
  - [ ] GC strategy documented
  - [ ] `cargo make test` passes, `cargo make lint` passes

  **QA Scenarios**:

  ```
  Scenario: Runtime foundation
    Tool: Bash (cargo test)
    Steps:
      1. Run `cargo make test 2>&1 | tee temp.log`
      2. Verify runtime tests pass
    Expected Result: All tests pass
    Evidence: .sisyphus/evidence/task-24-runtime.txt
  ```

  **Commit**: YES
  - Message: `feat(runtime): implement runtime system foundation with memory allocation and built-in functions`
  - Pre-commit: `cargo make lint-fix && cargo make test`

- [ ] 25. Code Generation — ADTs & Generics (Phase 5 continued)

  **What to do**:
  - Read all requirements files, PLAN.md, `REFACTORING_GUIDE.md` (module structure patterns)
  - Implement codegen for ADT construction (sum type variant creation, product type struct creation)
  - Implement codegen for pattern matching (decision tree or switch-based lowering)
  - Implement codegen for field access on product types
  - Implement monomorphization pass (generic functions → concrete instantiations)
  - Implement codegen for generic function calls (dispatch to monomorphized version)
  - Implement codegen for generic ADT instantiation
  - Write TDD tests FIRST: ADT construction IR, pattern match lowering, monomorphization
  - Run lint, test, line-count checks
  - Update PLAN.md
  - Commit

  **Must NOT do**:
  - Same guardrails as Task 1

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: Monomorphization and pattern match lowering are algorithmically complex
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 7 (with Tasks 26, 27, 28)
  - **Blocks**: Task 29 (Change Detection — needs complete codegen)
  - **Blocked By**: Tasks 22, 23 (Expression/Function codegen), Task 18 (Generic System)

  **References**:

  **Pattern References**:
  - `src/codegen/` — LLVM backend infrastructure
  - `src/type_system/substitution.rs` — Type substitution for monomorphization
  - `src/type_system/types.rs` — CoreType::Generic

  **API/Type References**:
  - `language-spec/linked_list.op` — Generic ADT example
  - `language-spec/types_example.types.op` — Complex ADT definitions
  - `PLAN.md:545` — Monomorphization for code generation

  **Acceptance Criteria**:
  - [ ] ADT construction generates correct IR
  - [ ] Pattern matching lowers to efficient decision tree
  - [ ] Monomorphization creates concrete function instances
  - [ ] Generic ADTs instantiate correctly
  - [ ] `cargo make test` passes, `cargo make lint` passes

  **QA Scenarios**:

  ```
  Scenario: ADT and generic codegen
    Tool: Bash (cargo test)
    Steps:
      1. Run `cargo make test 2>&1 | tee temp.log`
      2. Verify ADT codegen and monomorphization tests pass
    Expected Result: All tests pass
    Evidence: .sisyphus/evidence/task-25-adt-codegen.txt
  ```

  **Commit**: YES
  - Message: `feat(codegen): implement ADT code generation and monomorphization`
  - Pre-commit: `cargo make lint-fix && cargo make test`

- [ ] 26. Runtime System — Memory & Standard Library (Phase 5 continued)

  **What to do**:
  - Read all requirements files, PLAN.md, `ERROR_HANDLING_STANDARDS.md`
  - Implement memory management strategy (reference counting or arena allocation)
  - Implement GC if reference counting chosen (cycle detection)
  - Implement `string_to_int32()` runtime function with error result
  - Implement `random_int32()` runtime function
  - Implement string interpolation runtime support
  - Implement array slice operations
  - Implement runtime error reporting with miette-style output
  - Write TDD tests FIRST: memory allocation/deallocation, stdlib functions, error reporting
  - Run lint, test, line-count checks
  - Update PLAN.md
  - Commit

  **Must NOT do**:
  - Same guardrails as Task 1
  - Tests must not perform actual I/O

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: Memory management design with runtime error handling
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 7 (with Tasks 25, 27, 28)
  - **Blocks**: Task 34 (Core Library)
  - **Blocked By**: Task 24 (Runtime Foundation)

  **References**:

  **Pattern References**:
  - `src/runtime/` — Runtime system foundation (from Task 24)
  - `ERROR_HANDLING_STANDARDS.md` — Error reporting patterns

  **API/Type References**:
  - `language-spec/simple_quiz.op:8-9` — Uses `string_to_int32`, `random_int32`
  - `language-spec/requirements/overview.md:52` — miette for output formatting

  **Acceptance Criteria**:
  - [ ] Memory management strategy implemented
  - [ ] `string_to_int32()` and `random_int32()` runtime functions work
  - [ ] String interpolation runtime support
  - [ ] Runtime error reporting works
  - [ ] `cargo make test` passes, `cargo make lint` passes

  **QA Scenarios**:

  ```
  Scenario: Runtime memory and stdlib
    Tool: Bash (cargo test)
    Steps:
      1. Run `cargo make test 2>&1 | tee temp.log`
      2. Verify runtime memory and stdlib tests pass
    Expected Result: All tests pass
    Evidence: .sisyphus/evidence/task-26-runtime-stdlib.txt
  ```

  **Commit**: YES
  - Message: `feat(runtime): implement memory management and standard library runtime functions`
  - Pre-commit: `cargo make lint-fix && cargo make test`

- [ ] 27. Basic Optimization Passes (Phase 5 continued)

  **What to do**:
  - Read all requirements files, PLAN.md
  - Implement dead code elimination pass
  - Implement constant folding/propagation at LLVM IR level
  - Implement basic inline expansion for small functions
  - Implement unused variable/import elimination
  - Configure LLVM optimization levels (O0 for debug, O2 for release)
  - Verify optimized code produces same results as unoptimized
  - Write TDD tests FIRST: dead code removed, constants folded, inlining correct
  - Run lint, test, line-count checks
  - Update PLAN.md
  - Commit

  **Must NOT do**:
  - Same guardrails as Task 1
  - Do not implement loop optimizations (advanced, deferred)

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: Compiler optimization passes require correctness proofs
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 7 (with Tasks 25, 26, 28)
  - **Blocks**: Task 40 (Performance Optimization)
  - **Blocked By**: Tasks 22, 23 (Expression/Function codegen)

  **References**:

  **Pattern References**:
  - `src/codegen/` — LLVM backend
  - `PLAN.md:595-600` — Optimization items

  **External References**:
  - LLVM optimization pass documentation

  **Acceptance Criteria**:
  - [ ] Dead code elimination works
  - [ ] Constant folding at IR level
  - [ ] Basic inlining for small functions
  - [ ] O0/O2 levels configurable
  - [ ] `cargo make test` passes, `cargo make lint` passes

  **QA Scenarios**:

  ```
  Scenario: Optimization passes
    Tool: Bash (cargo test)
    Steps:
      1. Run `cargo make test 2>&1 | tee temp.log`
      2. Verify optimization tests pass
    Expected Result: All tests pass, optimized output matches unoptimized
    Evidence: .sisyphus/evidence/task-27-optimization.txt
  ```

  **Commit**: YES
  - Message: `feat(codegen): implement basic optimization passes with dead code elimination and constant folding`
  - Pre-commit: `cargo make lint-fix && cargo make test`

- [ ] 28. Hot Reload Infrastructure (Phase 6 Start)

  **What to do**:
  - Read all requirements files, PLAN.md, `HOT_RELOAD_ARCHITECTURE.md` (ENTIRE file), `INTEGRATION_DEPENDENCIES.md`
  - Implement dynamic library compilation mode (compile modules to .so/.dylib/.dll)
  - Implement ABI signature generation from type system (function table + POD struct hashes)
  - Implement version management system (versioned filenames: `logic_v0123.so`)
  - Implement host process framework (owns long-lived state and threads)
  - Implement module hot-swap mechanism (load new module, compare ABI signatures, swap if compatible)
  - Create narrow C ABI interface for hot modules
  - Write TDD tests FIRST: ABI signature generation, version management, module loading/swapping
  - Run lint, test, line-count checks
  - Update PLAN.md
  - Commit

  **Must NOT do**:
  - Same guardrails as Task 1
  - Do not implement file watching (Task 29)
  - Do not implement change classification (Task 29)
  - Tests must not create actual .so files — mock the loading interface

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: Dynamic library infrastructure with ABI compatibility checking
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 7 (with Tasks 25, 26, 27)
  - **Blocks**: Tasks 29, 30 (Change Detection, Safety)
  - **Blocked By**: Tasks 21-23 (LLVM Backend — needs compilation pipeline)

  **References**:

  **Pattern References**:
  - `src/type_system/memory.rs` — MemoryLayout for ABI sizing
  - `src/type_system/symbol_table.rs` — SymbolInfo for ABI exports
  - `HOT_RELOAD_ARCHITECTURE.md` — Complete architecture specification

  **API/Type References**:
  - `language-spec/requirements/overview.md:5-19` — Hot reload specification
  - `HOT_RELOAD_ARCHITECTURE.md:76-91` — ABI signature struct definitions
  - `PLAN.md:604-610` — Hot reload infrastructure items

  **Acceptance Criteria**:
  - [ ] Dynamic library compilation mode works
  - [ ] ABI signature generation from type info
  - [ ] Versioned filenames for loaded modules
  - [ ] Host process framework with module swap
  - [ ] Narrow C ABI interface for modules
  - [ ] `cargo make test` passes, `cargo make lint` passes

  **QA Scenarios**:

  ```
  Scenario: Hot reload infrastructure
    Tool: Bash (cargo test)
    Steps:
      1. Run `cargo make test 2>&1 | tee temp.log`
      2. Verify hot reload infrastructure tests pass
    Expected Result: All tests pass
    Evidence: .sisyphus/evidence/task-28-hot-reload-infra.txt

  Scenario: ABI signature generation
    Tool: Bash (cargo test)
    Steps:
      1. Verify test exists for generating ABI signature from type info
      2. Verify test exists for ABI compatibility comparison
      3. Run `cargo make test 2>&1 | tee temp.log`
    Expected Result: ABI signatures generate and compare correctly
    Evidence: .sisyphus/evidence/task-28-abi-signature.txt
  ```

  **Commit**: YES
  - Message: `feat(hot-reload): implement hot reload infrastructure with ABI signatures and module swapping`
  - Pre-commit: `cargo make lint-fix && cargo make test`

- [ ] 29. Change Detection & Hot Swap Classification (Phase 6)

  **What to do**:
  - Read all requirements files, PLAN.md, `HOT_RELOAD_ARCHITECTURE.md` (ENTIRE file — especially lines 250-284 Change Classification), `INTEGRATION_DEPENDENCIES.md`
  - Read `REFACTORING_GUIDE.md`, `refactor-lint.chatmode.md`, `FIXES.txt`, `ERROR_HANDLING_STANDARDS.md`
  - Implement file watching system (inotify/kqueue/ReadDirectoryChanges abstraction)
  - Implement build graph analysis (module dependency graph from Phase 4 module system)
  - Implement `ChangeClassifier` per `HOT_RELOAD_ARCHITECTURE.md:252-283` — analyzes old vs new ABI signatures to determine function/type changes
  - Implement hot-swap vs restart classification algorithm (function body → hot swap, signature change → restart, type layout change → full restart)
  - Implement incremental compilation trigger (only recompile changed modules + dependents)
  - Implement ABI signature caching per `HOT_RELOAD_ARCHITECTURE.md:369-374`
  - Write TDD tests FIRST: change classification for each category (body-only, signature, type layout), build graph traversal, incremental recompilation decisions
  - All tests must mock file system — no actual file watching in tests
  - Run lint, test, line-count checks
  - Update PLAN.md
  - Commit

  **Must NOT do**:
  - Same guardrails as Task 1
  - Do not modify the hot reload infrastructure from Task 28
  - Do not implement state preservation (Task 30)
  - Do not implement error recovery (Task 30)
  - Tests must not create actual files or use real file watchers — mock all I/O

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: Complex classification algorithm with dependency graph analysis
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 8 (with Task 30)
  - **Blocks**: None directly — Tasks 29 and 30 are the last hot reload tasks
  - **Blocked By**: Task 28 (Hot Reload Infrastructure)

  **References**:

  **Pattern References**:
  - `src/codegen/` — Code generation pipeline for incremental compilation context
  - `HOT_RELOAD_ARCHITECTURE.md:252-283` — `ChangeClassifier` implementation strategy
  - `HOT_RELOAD_ARCHITECTURE.md:369-374` — ABI signature caching struct

  **API/Type References**:
  - `HOT_RELOAD_ARCHITECTURE.md:110-122` — `ABIStability` and `HotReloadCategory` enums
  - `HOT_RELOAD_ARCHITECTURE.md:143-158` — `ADTChangeAnalysis` and `ReloadStrategy` structs
  - `HOT_RELOAD_ARCHITECTURE.md:173-187` — `ModuleDependencyGraph` and `ModuleMetadata` structs
  - `PLAN.md:612-618` — Change detection plan items

  **External References**:
  - `language-spec/requirements/overview.md:5-19` — Hot reload specification

  **WHY Each Reference Matters**:
  - `HOT_RELOAD_ARCHITECTURE.md:252-283` — The actual algorithm for classifying changes; copy this pattern
  - `HOT_RELOAD_ARCHITECTURE.md:143-158` — ADT change analysis determines struct/enum reload safety
  - Module dependency graph is used for transitive invalidation — if module A changes, all dependents must be re-checked

  **Acceptance Criteria**:
  - [ ] File watching abstraction created (trait with mock implementation)
  - [ ] Build graph analysis traverses module dependencies
  - [ ] `ChangeClassifier` correctly classifies function body → hot swap
  - [ ] `ChangeClassifier` correctly classifies signature change → restart
  - [ ] `ChangeClassifier` correctly classifies type layout change → full restart
  - [ ] Incremental compilation only targets changed modules + dependents
  - [ ] ABI signature caching avoids recomputation
  - [ ] `cargo make test` passes, `cargo make lint` passes

  **QA Scenarios**:

  ```
  Scenario: Change classification — hot swappable function body change
    Tool: Bash (cargo test)
    Steps:
      1. Run `cargo make test 2>&1 | tee temp.log`
      2. Verify test exists: function body-only change classified as HotSwappable
      3. Verify test exists: same ABI hash before/after body-only change
    Expected Result: Classification is HotSwappable, ABI hash unchanged
    Failure Indicators: Test failure or wrong classification
    Evidence: .sisyphus/evidence/task-29-hot-swap-classify.txt

  Scenario: Change classification — restart-required signature change
    Tool: Bash (cargo test)
    Steps:
      1. Run `cargo make test 2>&1 | tee temp.log`
      2. Verify test exists: function signature change classified as RequiresRestart
      3. Verify test exists: different ABI hash after signature change
    Expected Result: Classification is RequiresRestart, ABI hash differs
    Failure Indicators: Test failure or wrong classification
    Evidence: .sisyphus/evidence/task-29-restart-classify.txt
  ```

  **Commit**: YES
  - Message: `feat(hot-reload): implement change detection, classification, and incremental compilation triggers`
  - Pre-commit: `cargo make lint-fix && cargo make test`

- [ ] 30. Hot Reload Safety & Error Recovery (Phase 6)

  **What to do**:
  - Read all requirements files, PLAN.md, `HOT_RELOAD_ARCHITECTURE.md` (ENTIRE file — especially lines 286-362 Implementation Guidelines and Testing), `INTEGRATION_DEPENDENCIES.md`
  - Read `REFACTORING_GUIDE.md`, `refactor-lint.chatmode.md`, `FIXES.txt`, `ERROR_HANDLING_STANDARDS.md`
  - Implement ABI guard: host loads new module, compares ABI signature hash, rejects incompatible loads
  - Implement automatic fallback restart: when ABI guard rejects, trigger orchestrated full rebuild/restart
  - Implement state preservation across reloads (serialize/deserialize long-lived state in host process)
  - Implement error recovery: handle module load failures, compilation errors during hot reload, partial swap rollback
  - Implement hot reload testing framework per `HOT_RELOAD_ARCHITECTURE.md:314-362` — test categories for ABI compatibility, change classification, integration, and performance
  - Write TDD tests FIRST: ABI guard accept/reject, fallback restart trigger, state preservation round-trip, error recovery from bad module
  - All tests must mock dynamic library loading — no actual .so/.dylib creation
  - Run lint, test, line-count checks
  - Update PLAN.md — check off ALL Phase 6 items
  - Commit

  **Must NOT do**:
  - Same guardrails as Task 1
  - Do not modify change detection (Task 29)
  - Do not modify hot reload infrastructure (Task 28)
  - Tests must not load actual dynamic libraries — mock the loading interface

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: Safety-critical code with error recovery and state management
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 8 (with Task 29)
  - **Blocks**: None directly — last hot reload task
  - **Blocked By**: Task 28 (Hot Reload Infrastructure)

  **References**:

  **Pattern References**:
  - Hot reload infrastructure from Task 28 — ABI signature types, host process, module swap mechanism
  - `HOT_RELOAD_ARCHITECTURE.md:286-362` — Implementation guidelines and testing strategy
  - `ERROR_HANDLING_STANDARDS.md` — Error handling patterns for recovery logic

  **API/Type References**:
  - `HOT_RELOAD_ARCHITECTURE.md:76-91` — `ABISignature` struct for guard comparison
  - `HOT_RELOAD_ARCHITECTURE.md:110-122` — `ABIStability` enum (Stable/Breaking/Compatible)
  - `HOT_RELOAD_ARCHITECTURE.md:143-158` — `ReloadStrategy` enum (HotSwap/StatePreservingRestart/FullRestart)
  - `PLAN.md:620-626` — Hot reload safety items
  - `language-spec/requirements/overview.md:19` — "don't crash—just restart" requirement

  **WHY Each Reference Matters**:
  - ABI guard comparison is the core safety mechanism — without it, incompatible modules crash the host
  - `ReloadStrategy` enum determines whether to hot swap, save state + restart, or full restart
  - The "don't crash—just restart" requirement from overview.md is the user's explicit safety philosophy

  **Acceptance Criteria**:
  - [ ] ABI guard accepts compatible module loads
  - [ ] ABI guard rejects incompatible loads and triggers fallback
  - [ ] Automatic fallback restart works (full rebuild/restart path)
  - [ ] State preservation serializes/deserializes host state
  - [ ] Error recovery handles module load failures gracefully
  - [ ] Error recovery handles compilation errors during hot reload
  - [ ] Partial swap rollback works (if swap fails mid-operation)
  - [ ] Hot reload test framework categories implemented
  - [ ] `cargo make test` passes, `cargo make lint` passes

  **QA Scenarios**:

  ```
  Scenario: ABI guard accepts compatible module
    Tool: Bash (cargo test)
    Steps:
      1. Run `cargo make test 2>&1 | tee temp.log`
      2. Verify test exists: ABI guard accepts module with matching signature hash
      3. Verify test exists: module swap completes successfully after guard passes
    Expected Result: Guard passes, swap succeeds
    Failure Indicators: Guard rejection on compatible module or swap failure
    Evidence: .sisyphus/evidence/task-30-abi-guard-accept.txt

  Scenario: ABI guard rejects incompatible module and triggers restart
    Tool: Bash (cargo test)
    Steps:
      1. Run `cargo make test 2>&1 | tee temp.log`
      2. Verify test exists: ABI guard rejects module with different signature hash
      3. Verify test exists: fallback restart is triggered after guard rejection
    Expected Result: Guard rejects, restart triggered, no crash
    Failure Indicators: Guard accepts incompatible module or crash on rejection
    Evidence: .sisyphus/evidence/task-30-abi-guard-reject.txt

  Scenario: Error recovery from bad module load
    Tool: Bash (cargo test)
    Steps:
      1. Run `cargo make test 2>&1 | tee temp.log`
      2. Verify test exists: module load failure (invalid module) is handled
      3. Verify test exists: host process remains running after load failure
    Expected Result: Error caught, host continues running, error reported
    Evidence: .sisyphus/evidence/task-30-error-recovery.txt
  ```

  **Commit**: YES
  - Message: `feat(hot-reload): implement ABI guard, fallback restart, state preservation, and error recovery`
  - Pre-commit: `cargo make lint-fix && cargo make test`

- [ ] 31. Error Reporting System (Phase 7)

  **What to do**:
  - Read all requirements files, PLAN.md, `ERROR_HANDLING_STANDARDS.md`, `REFACTORING_GUIDE.md`, `refactor-lint.chatmode.md`, `FIXES.txt`
  - Enhance miette integration for beautiful error output across all compiler phases (lexer → parser → type checker → codegen)
  - Implement source location tracking improvements (ensure spans are accurate through all transformations)
  - Implement helpful error messages with context: "expected X, found Y" with surrounding code snippet
  - Implement suggestion system: "did you mean?" for similar identifiers, "consider adding type annotation" for inference failures
  - Implement multi-error reporting: collect ALL errors in a compilation unit, not just the first one
  - Review error messages in `src/parser/errors.rs`, `src/type_system/errors.rs`, `src/error.rs` — ensure consistent miette formatting
  - Write TDD tests FIRST: error message formatting, suggestion accuracy, multi-error collection
  - Run lint, test, line-count checks
  - Update PLAN.md
  - Commit

  **Must NOT do**:
  - Same guardrails as Task 1
  - Do not modify parser logic — only error presentation
  - Do not modify type checker logic — only error presentation
  - Do not add error recovery strategies (that's parser/checker responsibility)

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
    - Reason: Error reporting touches many modules but is focused on presentation layer
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 9 (with Tasks 32, 33)
  - **Blocks**: None directly
  - **Blocked By**: Tasks 21-28 (Code Generation and Hot Reload — need complete pipeline for end-to-end errors)

  **References**:

  **Pattern References**:
  - `src/parser/errors.rs` — Existing parser error types and miette integration
  - `src/type_system/errors.rs` — Existing type checker error types
  - `src/error.rs` — Top-level error types
  - `ERROR_HANDLING_STANDARDS.md` — Error handling patterns and miette usage

  **API/Type References**:
  - `PLAN.md:630-636` — Error reporting plan items
  - `FIXES.txt` — Known bug fixes that inform error message quality

  **External References**:
  - miette crate documentation — diagnostic formatting, labels, suggestions, related errors

  **WHY Each Reference Matters**:
  - `src/parser/errors.rs` and `src/type_system/errors.rs` — These are the existing error types to enhance, not replace
  - `ERROR_HANDLING_STANDARDS.md` — Contains the project's error handling philosophy and patterns
  - `FIXES.txt` — Shows past error handling bugs; new errors must not repeat those patterns

  **Acceptance Criteria**:
  - [ ] Error messages include source code snippets with highlighted spans
  - [ ] "Did you mean?" suggestions for typos in identifiers
  - [ ] "Consider adding type annotation" suggestions for inference failures
  - [ ] Multi-error reporting collects all errors per compilation unit
  - [ ] Consistent miette formatting across lexer, parser, type checker, codegen
  - [ ] `cargo make test` passes, `cargo make lint` passes

  **QA Scenarios**:

  ```
  Scenario: Multi-error reporting
    Tool: Bash (cargo test)
    Steps:
      1. Run `cargo make test 2>&1 | tee temp.log`
      2. Verify test exists: source with 3+ errors reports ALL of them, not just first
      3. Verify test exists: each error includes correct source span
    Expected Result: All errors collected and reported with accurate spans
    Failure Indicators: Only first error reported, or spans point to wrong location
    Evidence: .sisyphus/evidence/task-31-multi-error.txt

  Scenario: Suggestion system accuracy
    Tool: Bash (cargo test)
    Steps:
      1. Run `cargo make test 2>&1 | tee temp.log`
      2. Verify test exists: typo in identifier produces "did you mean?" suggestion
      3. Verify test exists: suggestion is correct (Levenshtein distance or similar)
    Expected Result: Suggestion matches the intended identifier
    Failure Indicators: No suggestion offered, or suggestion is wrong
    Evidence: .sisyphus/evidence/task-31-suggestions.txt
  ```

  **Commit**: YES
  - Message: `feat(errors): enhance error reporting with suggestions, multi-error collection, and beautiful formatting`
  - Pre-commit: `cargo make lint-fix && cargo make test`

- [ ] 32. Documentation Generation System (Phase 7)

  **What to do**:
  - Read all requirements files, PLAN.md, `REFACTORING_GUIDE.md`, `refactor-lint.chatmode.md`, `FIXES.txt`
  - Implement documentation generation from code: extract doc comments (`## Description: ... ##`) and doc attributes (`@description`, `@param`, `@returns`, `@example`)
  - Implement API documentation generation: create structured output from public symbol documentation
  - Implement language reference generation: auto-generate reference docs from type system and grammar
  - Create output format (HTML or Markdown) with navigation, cross-references, and search
  - Write TDD tests FIRST: doc extraction from AST, output format correctness, cross-reference resolution
  - Run lint, test, line-count checks
  - Update PLAN.md
  - Commit

  **Must NOT do**:
  - Same guardrails as Task 1
  - Do not modify the parser's doc comment parsing (already complete)
  - Do not modify AST doc comment preservation (already complete)
  - Do not create actual documentation files in tests — only verify output strings

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
    - Reason: Documentation generation is a standalone feature that reads AST and produces formatted output
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 9 (with Tasks 31, 33)
  - **Blocks**: None directly
  - **Blocked By**: Tasks 21-24 (Code Generation — needs complete AST + type info for full doc generation)

  **References**:

  **Pattern References**:
  - `src/ast.rs` and `src/ast/` — Doc comment nodes in AST (already parsed and preserved)
  - `src/parser/declarations.rs` — How doc comments are parsed and attached to declarations

  **API/Type References**:
  - `PLAN.md:638-646` — Documentation system plan items (note: parsing items already checked off)
  - `language-spec/requirements/overview.md` — Doc comment syntax specification

  **WHY Each Reference Matters**:
  - AST doc comment nodes are the INPUT to this system — the generator reads them and produces output
  - Parser declarations show how doc comments are attached to symbols — this determines traversal order

  **Acceptance Criteria**:
  - [ ] Doc comments extracted correctly from AST
  - [ ] `@param`, `@returns`, `@example` attributes parsed into structured data
  - [ ] API docs generated for public symbols
  - [ ] Cross-references between types resolve correctly
  - [ ] Output format is valid HTML or Markdown
  - [ ] `cargo make test` passes, `cargo make lint` passes

  **QA Scenarios**:

  ```
  Scenario: Documentation generation from source
    Tool: Bash (cargo test)
    Steps:
      1. Run `cargo make test 2>&1 | tee temp.log`
      2. Verify test exists: parse source with doc comments → generate docs → output contains all doc content
      3. Verify test exists: @param/@returns attributes appear in structured output
    Expected Result: Generated docs match source doc comments exactly
    Failure Indicators: Missing doc sections or malformed output
    Evidence: .sisyphus/evidence/task-32-doc-gen.txt

  Scenario: Cross-reference resolution
    Tool: Bash (cargo test)
    Steps:
      1. Run `cargo make test 2>&1 | tee temp.log`
      2. Verify test exists: type reference in docs resolves to correct type's documentation
    Expected Result: Cross-references link to correct symbols
    Evidence: .sisyphus/evidence/task-32-cross-ref.txt
  ```

  **Commit**: YES
  - Message: `feat(docs): implement documentation generation system with API docs and cross-references`
  - Pre-commit: `cargo make lint-fix && cargo make test`

- [ ] 33. Build System & Project Configuration (Phase 7)

  **What to do**:
  - Read all requirements files, PLAN.md, `REFACTORING_GUIDE.md`, `refactor-lint.chatmode.md`, `FIXES.txt`
  - Implement project configuration file format (e.g., `opal.toml` or similar) — defines project name, version, dependencies, build targets
  - Implement dependency management: resolve package dependencies from configuration, check version compatibility
  - Implement build caching: hash source files + dependencies, skip recompilation when unchanged
  - Implement incremental builds: only recompile changed modules and their transitive dependents (leverage module dependency graph from Phase 4)
  - Implement cross-compilation support (target platform selection, no_std compatibility for core modules)
  - Write TDD tests FIRST: config parsing, dependency resolution, cache hit/miss, incremental build decisions
  - All tests must mock file system — no actual file I/O
  - Run lint, test, line-count checks
  - Update PLAN.md — check off ALL Phase 7 items
  - Commit

  **Must NOT do**:
  - Same guardrails as Task 1
  - Do not implement package registry (Task 41)
  - Do not implement network-based dependency resolution (Task 41)
  - Tests must not create actual files — mock all I/O

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: Build system with dependency resolution and caching requires careful algorithm design
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 9 (with Tasks 31, 32)
  - **Blocks**: Tasks 41 (Package Manager depends on build system)
  - **Blocked By**: Tasks 19-20 (Module System — dependency graph needed for incremental builds)

  **References**:

  **Pattern References**:
  - Module system from Tasks 19-20 — Module dependency graph used for incremental builds
  - `Cargo.toml` — Reference for project configuration format (Opalescent's equivalent)

  **API/Type References**:
  - `PLAN.md:648-654` — Build system plan items
  - Module dependency graph types from Phase 4 implementation

  **External References**:
  - Cargo build system design (reference for caching and incremental build strategies)

  **WHY Each Reference Matters**:
  - Module dependency graph is the core data structure for incremental builds — determines what to rebuild
  - Cargo.toml format is a well-understood reference for project config — adapt, don't reinvent

  **Acceptance Criteria**:
  - [ ] Project config file format defined and parseable (e.g., `opal.toml`)
  - [ ] Dependency resolution from config works (name + version constraints)
  - [ ] Build caching detects unchanged source files (hash comparison)
  - [ ] Incremental builds only recompile changed modules + dependents
  - [ ] Cross-compilation target selection works
  - [ ] `cargo make test` passes, `cargo make lint` passes

  **QA Scenarios**:

  ```
  Scenario: Build caching avoids recompilation
    Tool: Bash (cargo test)
    Steps:
      1. Run `cargo make test 2>&1 | tee temp.log`
      2. Verify test exists: same source file hash → cache hit → no recompilation
      3. Verify test exists: different source file hash → cache miss → recompilation triggered
    Expected Result: Cache correctly detects changed vs unchanged files
    Failure Indicators: Cache miss on unchanged file, or cache hit on changed file
    Evidence: .sisyphus/evidence/task-33-build-cache.txt

  Scenario: Incremental build with dependency graph
    Tool: Bash (cargo test)
    Steps:
      1. Run `cargo make test 2>&1 | tee temp.log`
      2. Verify test exists: module A changes → A and its dependents rebuild, unrelated modules skip
    Expected Result: Only affected modules recompile
    Evidence: .sisyphus/evidence/task-33-incremental.txt
  ```

  **Commit**: YES
  - Message: `feat(build): implement build system with project config, caching, incremental builds, and cross-compilation`
  - Pre-commit: `cargo make lint-fix && cargo make test`

- [ ] 34. Core Standard Library (Phase 8)

  **What to do**:
  - Read all requirements files, PLAN.md, `REFACTORING_GUIDE.md`, `refactor-lint.chatmode.md`, `FIXES.txt`, `ERROR_HANDLING_STANDARDS.md`
  - Read `language-spec/requirements/overview.md`, `language-spec/requirements/math.md` for built-in type/function specifications
  - Implement basic data type runtime representations (int8-64, uint8-64, float32/64, bool, string, void)
  - Implement string operations: concatenation, slicing, length, find, replace, split, trim, case conversion
  - Implement math functions per `language-spec/requirements/math.md`: abs, ceil, floor, round, min, max, pow, sqrt, log, sin, cos, tan, pi, e, infinity
  - Implement I/O operations: print, println, read_line (for basic programs)
  - Implement file system access: read_file, write_file, file_exists, list_dir (all behind trait for mockability)
  - Ensure all core modules are `no_std` compatible (use `alloc`/`core` over `std`) per chatmode guidelines
  - Write TDD tests FIRST: string operations, math functions, I/O mocking
  - All tests must mock I/O — no actual file system access
  - Run lint, test, line-count checks
  - Update PLAN.md
  - Commit

  **Must NOT do**:
  - Same guardrails as Task 1
  - Do not implement collections (Task 35)
  - Do not implement system-level operations (Task 36)
  - Do not use `std` directly in core library — use `alloc`/`core`
  - Tests must not perform actual I/O — mock everything

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
    - Reason: Standard library implementation with many small functions following clear specifications
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 10 (with Tasks 35, 36)
  - **Blocks**: Language-spec example execution (need stdlib for programs to run)
  - **Blocked By**: Tasks 21-24 (Code Generation — need runtime to implement stdlib against)

  **References**:

  **Pattern References**:
  - `src/type_system/types.rs` — `CoreType` definitions for built-in types
  - `language-spec/requirements/math.md` — Complete math function specifications

  **API/Type References**:
  - `PLAN.md:658-664` — Core library plan items
  - `language-spec/requirements/overview.md:39-49` — Cast specifications (stdlib must implement checked/saturating APIs)
  - `language-spec/*.op` — Example programs show which stdlib functions are expected

  **External References**:
  - Rust `core`/`alloc` crate documentation — for no_std compatible implementations

  **WHY Each Reference Matters**:
  - `math.md` defines EXACTLY which math functions must exist and their signatures — this is the spec
  - `CoreType` definitions map to runtime representations — stdlib must match these exactly
  - Cast specifications require `checked_` and `saturating_` APIs in stdlib

  **Acceptance Criteria**:
  - [ ] All numeric types have runtime representations
  - [ ] String operations (concat, slice, length, find, replace, split, trim) work
  - [ ] Math functions per `math.md` all implemented (abs, ceil, floor, round, min, max, pow, sqrt, log, trig)
  - [ ] I/O print/println/read_line work
  - [ ] File I/O behind trait abstraction
  - [ ] All core modules are `no_std` compatible
  - [ ] `cargo make test` passes, `cargo make lint` passes

  **QA Scenarios**:

  ```
  Scenario: Math functions match specification
    Tool: Bash (cargo test)
    Steps:
      1. Run `cargo make test 2>&1 | tee temp.log`
      2. Verify tests exist for each math function in math.md
      3. Verify edge cases: infinity, NaN, negative zero
    Expected Result: All math functions return correct values per spec
    Failure Indicators: Wrong results or missing functions
    Evidence: .sisyphus/evidence/task-34-math-stdlib.txt

  Scenario: String operations
    Tool: Bash (cargo test)
    Steps:
      1. Run `cargo make test 2>&1 | tee temp.log`
      2. Verify tests exist for concat, slice, length, find, replace, split, trim
      3. Verify edge cases: empty string, Unicode, out-of-bounds slice
    Expected Result: All string operations return correct results
    Evidence: .sisyphus/evidence/task-34-string-ops.txt
  ```

  **Commit**: YES
  - Message: `feat(stdlib): implement core standard library with types, strings, math, and I/O`
  - Pre-commit: `cargo make lint-fix && cargo make test`

- [x] 35. Collections Standard Library (Phase 8)

  **What to do**:
  - Read all requirements files, PLAN.md, `REFACTORING_GUIDE.md`, `refactor-lint.chatmode.md`, `FIXES.txt`
  - Implement array operations: push, pop, insert, remove, slice, map, filter, reduce, find, sort, reverse, contains, length
  - Implement hash maps: insert, get, remove, contains_key, keys, values, entries, length
  - Implement sets: insert, remove, contains, union, intersection, difference, length
  - Implement lists (linked list if applicable): push_front, push_back, pop_front, pop_back, length
  - Implement iterator trait/interface: next, map, filter, reduce, collect, take, skip, enumerate, zip
  - Ensure all collections are generic (work with type parameters from Phase 3)
  - Write TDD tests FIRST: each collection operation, iterator chaining, edge cases (empty collection, single element)
  - Run lint, test, line-count checks
  - Update PLAN.md
  - Commit

  **Must NOT do**:
  - Same guardrails as Task 1
  - Do not implement core types (Task 34)
  - Do not implement system operations (Task 36)
  - All collections must use `alloc`/`core` not `std`

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
    - Reason: Collection implementations follow well-known patterns with many small methods
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 10 (with Tasks 34, 36)
  - **Blocks**: Language-spec example execution
  - **Blocked By**: Tasks 15-18 (Phase 3 — generic type system needed for generic collections)

  **References**:

  **Pattern References**:
  - Phase 3 ADT and generics implementation — generic collection types
  - `src/type_system/types.rs` — `CoreType::Array` and collection type representations

  **API/Type References**:
  - `PLAN.md:666-672` — Collections library plan items
  - `language-spec/*.op` — Example programs using arrays and collections

  **WHY Each Reference Matters**:
  - Generic type system determines how `Array<T>`, `Map<K, V>` etc. are parameterized
  - Example .op files show which collection operations are expected in real programs

  **Acceptance Criteria**:
  - [ ] Array operations all implemented (push/pop/insert/remove/slice/map/filter/reduce/sort)
  - [ ] Hash map operations all implemented (insert/get/remove/contains_key/keys/values)
  - [ ] Set operations implemented (insert/remove/contains/union/intersection/difference)
  - [ ] Iterator trait with map/filter/reduce/collect/take/skip/enumerate/zip
  - [ ] All collections are generic
  - [ ] Edge cases handled (empty, single element, duplicate keys)
  - [ ] `cargo make test` passes, `cargo make lint` passes

  **QA Scenarios**:

  ```
  Scenario: Array operations
    Tool: Bash (cargo test)
    Steps:
      1. Run `cargo make test 2>&1 | tee temp.log`
      2. Verify tests exist for push, pop, map, filter, reduce, sort
      3. Verify edge cases: empty array, single element
    Expected Result: All array operations return correct results
    Evidence: .sisyphus/evidence/task-35-array-ops.txt

  Scenario: Iterator chaining
    Tool: Bash (cargo test)
    Steps:
      1. Run `cargo make test 2>&1 | tee temp.log`
      2. Verify test exists: array.iter().map().filter().collect() produces correct result
    Expected Result: Chained iterators compose correctly
    Evidence: .sisyphus/evidence/task-35-iterator-chain.txt
  ```

  **Commit**: YES
  - Message: `feat(stdlib): implement collections library with arrays, maps, sets, and iterators`
  - Pre-commit: `cargo make lint-fix && cargo make test`

- [x] 36. System Standard Library (Phase 8)

  **What to do**:
  - Read all requirements files, PLAN.md, `REFACTORING_GUIDE.md`, `refactor-lint.chatmode.md`, `FIXES.txt`
  - Implement operating system interfaces: platform detection, environment variables, command-line arguments
  - Implement network operations: TCP/UDP socket abstraction (trait-based for mockability)
  - Implement threading support: spawn, join, mutex, channel (leveraging Rust's threading primitives)
  - Implement process management: spawn process, exit, signal handling
  - Implement environment access: env vars, working directory, home directory
  - All system interfaces behind traits for testability and platform abstraction
  - Write TDD tests FIRST: platform detection, env var access (mocked), thread spawn/join (mocked)
  - All tests must mock system calls — no actual process spawning, network access, or threading in tests
  - Run lint, test, line-count checks
  - Update PLAN.md — check off ALL Phase 8 items
  - Commit

  **Must NOT do**:
  - Same guardrails as Task 1
  - Do not implement core types (Task 34)
  - Do not implement collections (Task 35)
  - Tests must NOT spawn actual processes, open network connections, or create threads

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
    - Reason: System library wraps OS primitives behind trait abstractions
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 10 (with Tasks 34, 35)
  - **Blocks**: None directly
  - **Blocked By**: Tasks 21-24 (Code Generation — need runtime for system calls)

  **References**:

  **Pattern References**:
  - `src/type_system/memory.rs` — `MemoryLayout` for understanding data representation in system calls
  - Code generation runtime from Tasks 21-24 — runtime library foundation

  **API/Type References**:
  - `PLAN.md:674-680` — System library plan items

  **External References**:
  - Rust `std::process`, `std::net`, `std::thread` documentation — reference implementations

  **WHY Each Reference Matters**:
  - System library wraps Rust's standard library behind Opalescent's abstractions — need to know what Rust provides
  - Memory layout understanding needed for FFI and system call data passing

  **Acceptance Criteria**:
  - [ ] Platform detection works (OS, architecture)
  - [ ] Environment variable access works (mocked in tests)
  - [ ] Command-line argument parsing works
  - [ ] Network socket abstraction defined (trait-based)
  - [ ] Thread spawn/join abstraction works (mocked in tests)
  - [ ] Process spawn/exit works (mocked in tests)
  - [ ] All system interfaces behind traits for mockability
  - [ ] `cargo make test` passes, `cargo make lint` passes

  **QA Scenarios**:

  ```
  Scenario: Platform detection
    Tool: Bash (cargo test)
    Steps:
      1. Run `cargo make test 2>&1 | tee temp.log`
      2. Verify test exists: platform detection returns valid OS/arch
    Expected Result: Platform correctly identified
    Evidence: .sisyphus/evidence/task-36-platform.txt

  Scenario: System interface mockability
    Tool: Bash (cargo test)
    Steps:
      1. Run `cargo make test 2>&1 | tee temp.log`
      2. Verify test exists: env var trait can be mocked with test values
      3. Verify test exists: process trait can be mocked without actual spawning
    Expected Result: All system interfaces testable via mocks
    Evidence: .sisyphus/evidence/task-36-mock-system.txt
  ```

  **Commit**: YES
  - Message: `feat(stdlib): implement system library with OS interfaces, networking, threading, and process management`
  - Pre-commit: `cargo make lint-fix && cargo make test`

- [x] 37. Opalescent Test Framework (Phase 9)

  **What to do**:
  - Read all requirements files, PLAN.md, `REFACTORING_GUIDE.md`, `refactor-lint.chatmode.md`, `FIXES.txt`
  - Implement unit testing support for Opalescent programs: `test` keyword/block syntax, assertion functions (assert, assert_eq, assert_ne, assert_throws)
  - Implement integration testing: multi-module test execution, test discovery, test runner
  - Implement property-based testing: random input generation, shrinking on failure
  - Implement benchmark testing: timing harness, iteration control, statistical reporting
  - Implement coverage reporting: instrument codegen to track which branches/lines execute
  - Integrate test framework with build system (Task 33) — `opal test` command
  - Write TDD tests FIRST: test discovery, assertion correctness, runner execution, coverage tracking
  - Run lint, test, line-count checks
  - Update PLAN.md
  - Commit

  **Must NOT do**:
  - Same guardrails as Task 1
  - Do not modify the Rust test infrastructure (only the Opalescent-level test framework)
  - Do not implement LSP (Task 38) or formatter (Task 39)

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: Test framework design requires careful architecture for test discovery, execution, and reporting
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 11 (with Tasks 38, 39)
  - **Blocks**: None directly
  - **Blocked By**: Tasks 21-24 (Code Generation — tests need to compile and run Opalescent code)

  **References**:

  **Pattern References**:
  - Build system from Task 33 — test command integration
  - Code generation from Tasks 21-24 — instrumentation for coverage

  **API/Type References**:
  - `PLAN.md:684-690` — Test framework plan items
  - `language-spec/*.op` — Example programs as test subjects

  **WHY Each Reference Matters**:
  - Build system provides the `opal test` command — test framework hooks into it
  - Code generation instrumentation is needed for coverage — test framework reads the coverage data

  **Acceptance Criteria**:
  - [ ] Unit test syntax works (`test "name" { ... }` or equivalent)
  - [ ] Assertion functions work (assert, assert_eq, assert_ne, assert_throws)
  - [ ] Test discovery finds all tests in a project
  - [ ] Test runner executes and reports results (pass/fail/skip counts)
  - [ ] Property-based testing generates random inputs and shrinks on failure
  - [ ] Benchmark timing harness runs iterations and reports statistics
  - [ ] Coverage reporting tracks line/branch coverage
  - [ ] `cargo make test` passes, `cargo make lint` passes

  **QA Scenarios**:

  ```
  Scenario: Test framework execution
    Tool: Bash (cargo test)
    Steps:
      1. Run `cargo make test 2>&1 | tee temp.log`
      2. Verify test exists: test discovery finds labeled test blocks
      3. Verify test exists: assertion failures produce clear error messages
    Expected Result: Tests discovered, run, and results reported correctly
    Evidence: .sisyphus/evidence/task-37-test-framework.txt

  Scenario: Property-based testing
    Tool: Bash (cargo test)
    Steps:
      1. Run `cargo make test 2>&1 | tee temp.log`
      2. Verify test exists: property test generates inputs and finds counterexample
      3. Verify test exists: shrinking reduces counterexample to minimal form
    Expected Result: Counterexample found and minimized
    Evidence: .sisyphus/evidence/task-37-property-test.txt
  ```

  **Commit**: YES
  - Message: `feat(testing): implement Opalescent test framework with assertions, property tests, benchmarks, and coverage`
  - Pre-commit: `cargo make lint-fix && cargo make test`

- [x] 38. Language Server Protocol (Phase 9)

  **What to do**:
  - Read all requirements files, PLAN.md, `REFACTORING_GUIDE.md`, `refactor-lint.chatmode.md`, `FIXES.txt`
  - Implement LSP server using `tower-lsp` or similar Rust LSP library
  - Implement syntax highlighting via semantic tokens (respond to `textDocument/semanticTokens`)
  - Implement auto-completion: variable names, function names, type names, keywords (respond to `textDocument/completion`)
  - Implement error reporting: real-time diagnostic publishing from parser and type checker (respond to `textDocument/publishDiagnostics`)
  - Implement hover information: type info and doc comments on hover (respond to `textDocument/hover`)
  - Implement go-to-definition: symbol resolution for jump-to-source (respond to `textDocument/definition`)
  - Implement basic refactoring support: rename symbol (respond to `textDocument/rename`)
  - Create VS Code extension package.json for language configuration (syntax highlighting, bracket matching, comment toggling)
  - Write TDD tests FIRST: LSP message handling, completion results, diagnostic generation, hover info
  - Run lint, test, line-count checks
  - Update PLAN.md
  - Commit

  **Must NOT do**:
  - Same guardrails as Task 1
  - Do not implement test framework (Task 37) or formatter (Task 39)
  - Do not implement complex refactoring (extract function, etc.) — just rename
  - Tests must not open actual TCP connections — mock the LSP transport layer

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: LSP implementation requires understanding protocol messages, parsing pipeline integration, and real-time diagnostics
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 11 (with Tasks 37, 39)
  - **Blocks**: IDE plugins (Phase 10)
  - **Blocked By**: Tasks 21-24 (Code Generation) + Task 31 (Error Reporting — LSP publishes diagnostics)

  **References**:

  **Pattern References**:
  - `src/parser/` — Parser pipeline for real-time parsing in LSP
  - `src/type_system/` — Type checker for diagnostics and hover info
  - Error reporting from Task 31 — diagnostic format

  **API/Type References**:
  - `PLAN.md:692-698` — Language server plan items
  - `src/type_system/symbol_table.rs` — Symbol resolution for go-to-definition

  **External References**:
  - LSP specification: https://microsoft.github.io/language-server-protocol/
  - `tower-lsp` Rust crate documentation

  **WHY Each Reference Matters**:
  - Parser and type checker are called by the LSP on every edit — must understand their APIs
  - Symbol table is used for go-to-definition and rename — need to know how symbols are stored
  - Error reporting diagnostic format must match what LSP publishes

  **Acceptance Criteria**:
  - [ ] LSP server starts and handles initialize/initialized handshake
  - [ ] Semantic tokens provided for syntax highlighting
  - [ ] Auto-completion returns variables, functions, types, keywords
  - [ ] Real-time diagnostics published on file changes
  - [ ] Hover shows type information and doc comments
  - [ ] Go-to-definition navigates to symbol source
  - [ ] Rename refactoring updates all references
  - [ ] VS Code extension package.json created
  - [ ] `cargo make test` passes, `cargo make lint` passes

  **QA Scenarios**:

  ```
  Scenario: LSP diagnostics
    Tool: Bash (cargo test)
    Steps:
      1. Run `cargo make test 2>&1 | tee temp.log`
      2. Verify test exists: source with type error → LSP publishes diagnostic with correct span and message
    Expected Result: Diagnostic published with accurate error location
    Evidence: .sisyphus/evidence/task-38-lsp-diagnostics.txt

  Scenario: Auto-completion
    Tool: Bash (cargo test)
    Steps:
      1. Run `cargo make test 2>&1 | tee temp.log`
      2. Verify test exists: completion request at cursor position → returns relevant symbols
    Expected Result: Completion list contains expected identifiers
    Evidence: .sisyphus/evidence/task-38-completion.txt

  Scenario: Go-to-definition
    Tool: Bash (cargo test)
    Steps:
      1. Run `cargo make test 2>&1 | tee temp.log`
      2. Verify test exists: definition request on variable usage → returns declaration location
    Expected Result: Definition location matches actual declaration
    Evidence: .sisyphus/evidence/task-38-goto-def.txt
  ```

  **Commit**: YES
  - Message: `feat(lsp): implement Language Server Protocol with diagnostics, completion, hover, and go-to-definition`
  - Pre-commit: `cargo make lint-fix && cargo make test`

- [x] 39. Code Formatter (Phase 9)

  **What to do**:
  - Read all requirements files, PLAN.md, `REFACTORING_GUIDE.md`, `refactor-lint.chatmode.md`, `FIXES.txt`
  - Read `language-spec/requirements/overview.md` for naming conventions (snake_case vars/fns, PascalCase types)
  - Implement code formatting rules: indentation (spaces), line width limit, brace placement, operator spacing
  - Implement whitespace enforcement: consistent spacing around operators, after commas, before/after braces
  - Implement style consistency: naming convention enforcement (snake_case for vars/fns, PascalCase for types per overview.md)
  - Implement editor integration: `opal fmt` command that formats files in-place, stdin/stdout mode for editor pipes
  - Implement configuration options: allow users to override default rules via config file
  - Formatter must be idempotent: running twice produces same output
  - Write TDD tests FIRST: formatting rules, idempotency, edge cases (nested expressions, long lines, comments)
  - Run lint, test, line-count checks
  - Update PLAN.md — check off ALL Phase 9 items
  - Commit

  **Must NOT do**:
  - Same guardrails as Task 1
  - Do not implement test framework (Task 37) or LSP (Task 38)
  - Do not modify source code semantics — formatter is syntax-only
  - Tests must not write actual files — verify output strings

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
    - Reason: Formatter is a well-defined transformation of AST to text with configurable rules
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 11 (with Tasks 37, 38)
  - **Blocks**: None directly
  - **Blocked By**: Parser (Phase 1 — complete), AST (Phase 1 — complete). Effectively no blockers.

  **References**:

  **Pattern References**:
  - `src/ast.rs` and `src/ast/` — AST nodes that formatter traverses and reprints
  - `src/lexer.rs` — Token types for whitespace and comment handling

  **API/Type References**:
  - `PLAN.md:700-706` — Formatter plan items
  - `language-spec/requirements/overview.md:22-35` — Naming conventions (snake_case, PascalCase)

  **External References**:
  - `rustfmt` design — reference for formatter architecture (parse → format → print)
  - `prettier` design — reference for configurable formatting

  **WHY Each Reference Matters**:
  - AST nodes define what the formatter traverses — every node type needs a formatting rule
  - Naming conventions from overview.md are the spec — formatter must enforce them
  - Formatter architecture should follow the parse → format → print pattern proven by rustfmt

  **Acceptance Criteria**:
  - [ ] Consistent indentation (configurable spaces/tabs)
  - [ ] Line width enforcement with intelligent line breaking
  - [ ] Operator spacing consistent
  - [ ] Naming convention warnings (snake_case/PascalCase)
  - [ ] `opal fmt` command works (in-place + stdin/stdout modes)
  - [ ] Configuration file support for rule overrides
  - [ ] Formatter is idempotent (format(format(x)) == format(x))
  - [ ] `cargo make test` passes, `cargo make lint` passes

  **QA Scenarios**:

  ```
  Scenario: Formatter idempotency
    Tool: Bash (cargo test)
    Steps:
      1. Run `cargo make test 2>&1 | tee temp.log`
      2. Verify test exists: format(input) → output, format(output) → same output
    Expected Result: Second formatting produces identical output
    Failure Indicators: Output changes on second format pass
    Evidence: .sisyphus/evidence/task-39-idempotent.txt

  Scenario: Naming convention enforcement
    Tool: Bash (cargo test)
    Steps:
      1. Run `cargo make test 2>&1 | tee temp.log`
      2. Verify test exists: camelCase variable name → warning with snake_case suggestion
      3. Verify test exists: snake_case type name → warning with PascalCase suggestion
    Expected Result: Naming violations detected and correct suggestions provided
    Evidence: .sisyphus/evidence/task-39-naming.txt
  ```

  **Commit**: YES
  - Message: `feat(formatter): implement code formatter with style rules, naming enforcement, and configuration`
  - Pre-commit: `cargo make lint-fix && cargo make test`

- [x] 40. Performance Optimization & Benchmark Suite (Phase 10)

  **What to do**:
  - Read all requirements files, PLAN.md, `REFACTORING_GUIDE.md`, `refactor-lint.chatmode.md`, `FIXES.txt`
  - Implement compile time optimization: parser caching, incremental type checking, parallel compilation
  - Implement runtime performance improvements: optimize generated LLVM IR, reduce overhead in stdlib operations
  - Implement memory usage optimization: reduce AST memory footprint, efficient symbol table representation
  - Implement hot reload performance: measure and minimize hot swap latency, optimize ABI signature comparison
  - Implement benchmark suite: compile-time benchmarks (parse/typecheck/codegen time), runtime benchmarks (fibonacci, sorting, matrix ops), memory benchmarks (peak RSS during compilation)
  - Establish baseline performance metrics and regression detection
  - Write TDD tests FIRST: benchmark harness correctness, regression detection thresholds
  - Run lint, test, line-count checks
  - Update PLAN.md
  - Commit

  **Must NOT do**:
  - Same guardrails as Task 1
  - Do not break existing functionality for performance
  - Do not implement platform support (Task 41) or ecosystem (Task 42)
  - Benchmarks must not take >60 seconds each in CI

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: Performance optimization requires profiling, measurement, and careful algorithm tuning
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 12 (with Tasks 41, 42)
  - **Blocks**: None directly
  - **Blocked By**: All prior waves (need complete system to benchmark end-to-end)

  **References**:

  **Pattern References**:
  - `src/parser/` — Parse performance (hot path for compile time)
  - `src/type_system/` — Type checking performance
  - `src/codegen/` — Code generation performance
  - Hot reload infrastructure — swap latency

  **API/Type References**:
  - `PLAN.md:710-716` — Performance optimization plan items

  **External References**:
  - Rust `criterion` crate — benchmark framework

  **Acceptance Criteria**:
  - [ ] Benchmark suite covers compile time, runtime, and memory usage
  - [ ] Baseline performance metrics established
  - [ ] Regression detection threshold configured
  - [ ] Hot reload swap latency measured and optimized
  - [ ] No functionality regressions from optimizations
  - [ ] `cargo make test` passes, `cargo make lint` passes

  **QA Scenarios**:

  ```
  Scenario: Benchmark suite execution
    Tool: Bash (cargo test)
    Steps:
      1. Run `cargo make test 2>&1 | tee temp.log`
      2. Verify benchmark harness tests pass
      3. Verify regression detection works (inject artificial slowdown → detected)
    Expected Result: Benchmarks run and regression detection works
    Evidence: .sisyphus/evidence/task-40-benchmarks.txt
  ```

  **Commit**: YES
  - Message: `feat(perf): implement performance optimization and benchmark suite with regression detection`
  - Pre-commit: `cargo make lint-fix && cargo make test`

- [ ] 41. Platform Support & Package Manager (Phase 10)

  **What to do**:
  - Read all requirements files, PLAN.md, `REFACTORING_GUIDE.md`, `refactor-lint.chatmode.md`, `FIXES.txt`
  - Verify and fix Windows support: path handling (forward/backslash), file locking (versioned filenames already handle this), dynamic library extensions (.dll)
  - Verify and fix macOS support: dynamic library extensions (.dylib), code signing considerations
  - Verify and fix Linux support: dynamic library extensions (.so), ensure all features work
  - Implement cross-compilation: target triple selection, platform-specific codegen flags
  - Implement package manager (`opal pkg` or similar): init project, add/remove dependencies, install from registry, publish to registry
  - Implement package registry system: package index, version resolution, dependency tree, conflict detection
  - Implement package distribution: create distributable packages, install from URL or registry
  - Write TDD tests FIRST: platform detection, path normalization, package resolution, version constraint solving
  - All tests must mock network — no actual registry access in tests
  - Run lint, test, line-count checks
  - Update PLAN.md
  - Commit

  **Must NOT do**:
  - Same guardrails as Task 1
  - Do not implement performance optimization (Task 40) or ecosystem tooling (Task 42)
  - Tests must not make network requests — mock registry responses

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: Package manager with dependency resolution is algorithmically complex (SAT solving)
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 12 (with Tasks 40, 42)
  - **Blocks**: None directly
  - **Blocked By**: Task 33 (Build System — package manager extends build system)

  **References**:

  **Pattern References**:
  - Build system from Task 33 — project configuration, dependency management
  - `HOT_RELOAD_ARCHITECTURE.md:14` — Versioned filenames for platform-specific dynamic libs

  **API/Type References**:
  - `PLAN.md:718-724` — Platform support plan items
  - `PLAN.md:726-734` — Ecosystem plan items (package manager, registry)
  - `language-spec/requirements/overview.md:10-15` — Dynamic library extensions per platform

  **External References**:
  - Cargo package manager design — reference for dependency resolution
  - npm/pip registry design — reference for package distribution

  **Acceptance Criteria**:
  - [ ] Windows paths handled correctly (backslash normalization)
  - [ ] macOS .dylib extension used correctly
  - [ ] Linux .so extension used correctly
  - [ ] Cross-compilation with target triple selection works
  - [ ] `opal pkg init` creates project skeleton
  - [ ] `opal pkg add <dep>` adds dependency to config
  - [ ] Package version constraint resolution works (semver)
  - [ ] Dependency tree built without conflicts
  - [ ] `cargo make test` passes, `cargo make lint` passes

  **QA Scenarios**:

  ```
  Scenario: Package dependency resolution
    Tool: Bash (cargo test)
    Steps:
      1. Run `cargo make test 2>&1 | tee temp.log`
      2. Verify test exists: two packages with compatible versions → resolved correctly
      3. Verify test exists: two packages with conflicting versions → clear error message
    Expected Result: Compatible versions resolve, conflicts report clearly
    Evidence: .sisyphus/evidence/task-41-dep-resolution.txt

  Scenario: Platform-specific dynamic library extension
    Tool: Bash (cargo test)
    Steps:
      1. Run `cargo make test 2>&1 | tee temp.log`
      2. Verify test exists: platform detection → correct extension (.so/.dylib/.dll)
    Expected Result: Correct extension for each platform
    Evidence: .sisyphus/evidence/task-41-platform-ext.txt
  ```

  **Commit**: YES
  - Message: `feat(platform): implement cross-platform support and package manager with dependency resolution`
  - Pre-commit: `cargo make lint-fix && cargo make test`

- [ ] 42. Ecosystem Tooling & End-to-End Integration (Phase 10)

  **What to do**:
  - Read all requirements files, PLAN.md, `REFACTORING_GUIDE.md`, `refactor-lint.chatmode.md`, `FIXES.txt`
  - Read ALL `language-spec/*.op` files — these are the ULTIMATE integration test
  - Implement compiler "Help" commands: `opal help <topic>` for command clarifications per `PLAN.md:733`
  - Implement IDE plugin packaging: VS Code extension with LSP client, syntax highlighting, snippets
  - End-to-end integration: compile and execute ALL `language-spec/*.op` example programs
  - Verify each .op file: parses → type checks → generates code → executes → produces expected output
  - Fix any remaining issues discovered during end-to-end testing
  - Create comprehensive integration test suite that runs all .op files as part of `cargo make test`
  - Run lint, test, line-count checks
  - Update PLAN.md — check off ALL remaining Phase 10 items and verify ENTIRE PLAN.md is checked off
  - Commit

  **Must NOT do**:
  - Same guardrails as Task 1
  - Do not implement performance optimization (Task 40) or platform support (Task 41)
  - Integration tests for .op files must not modify the .op files themselves

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: End-to-end integration testing across entire compiler pipeline requires deep system understanding
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 12 (with Tasks 40, 41)
  - **Blocks**: Final Verification Wave (F1-F4)
  - **Blocked By**: All prior waves (end-to-end needs everything)

  **References**:

  **Pattern References**:
  - `language-spec/*.op` — ALL 9 example programs (hello_world.op, fibonacci.op, type_showcase.op, error_handling.op, etc.)
  - `src/main.rs` — Compiler entry point and CLI interface

  **API/Type References**:
  - `PLAN.md:726-734` — Ecosystem plan items
  - `language-spec/requirements/` — Complete language specification

  **WHY Each Reference Matters**:
  - `.op` files are the ULTIMATE acceptance test — if these all compile and run, the language works
  - `main.rs` CLI is where `opal help` commands are added
  - Language specification is the ground truth for what each .op file should do

  **Acceptance Criteria**:
  - [ ] `opal help` provides useful command information
  - [ ] VS Code extension package created (package.json, syntax highlighting, LSP client)
  - [ ] ALL `language-spec/*.op` files parse successfully
  - [ ] ALL `language-spec/*.op` files type check successfully
  - [ ] ALL `language-spec/*.op` files compile to executable code
  - [ ] ALL `language-spec/*.op` files execute and produce expected output
  - [ ] Integration test suite added to `cargo make test`
  - [ ] PLAN.md is fully checked off
  - [ ] `cargo make test` passes, `cargo make lint` passes

  **QA Scenarios**:

  ```
  Scenario: hello_world.op end-to-end
    Tool: Bash (cargo test)
    Steps:
      1. Run `cargo make test 2>&1 | tee temp.log`
      2. Verify integration test exists: parse + typecheck + compile + execute hello_world.op
    Expected Result: hello_world.op produces expected output
    Evidence: .sisyphus/evidence/task-42-hello-world.txt

  Scenario: All .op files compile and execute
    Tool: Bash (cargo test)
    Steps:
      1. Run `cargo make test 2>&1 | tee temp.log`
      2. Verify integration tests exist for ALL 9 .op files in language-spec/
      3. All must parse → type check → compile → execute correctly
    Expected Result: 9/9 .op files pass end-to-end
    Failure Indicators: Any .op file fails at any stage
    Evidence: .sisyphus/evidence/task-42-all-op-files.txt

  Scenario: PLAN.md fully checked off
    Tool: Bash (grep)
    Steps:
      1. Run `grep -c '^\- \[ \]' PLAN.md 2>&1 | tee temp.log`
      2. Count should be 0 (no unchecked items)
    Expected Result: 0 unchecked items in PLAN.md
    Failure Indicators: Any remaining unchecked items
    Evidence: .sisyphus/evidence/task-42-plan-complete.txt
  ```

  **Commit**: YES
  - Message: `feat(ecosystem): implement help commands, IDE plugin, and end-to-end integration tests for all .op files`
  - Pre-commit: `cargo make lint-fix && cargo make test`

---

## Final Verification Wave (MANDATORY — after ALL implementation tasks)

> 4 review agents run in PARALLEL. ALL must APPROVE. Present consolidated results to user and get explicit "okay" before completing.
>
> **Do NOT auto-proceed after verification. Wait for user's explicit approval before marking work complete.**
> **Never mark F1-F4 as checked before getting user's okay.** Rejection or user feedback → fix → re-run → present again → wait for okay.

- [ ] F1. **Plan Compliance Audit** — `oracle`
  Read the plan end-to-end. For each "Must Have": verify implementation exists (read file, curl endpoint, run command). For each "Must NOT Have": search codebase for forbidden patterns — reject with file:line if found. Check evidence files exist in .sisyphus/evidence/. Compare deliverables against plan. Verify PLAN.md is fully checked off.
  Output: `Must Have [N/N] | Must NOT Have [N/N] | Tasks [N/N] | PLAN.md [COMPLETE/INCOMPLETE] | VERDICT: APPROVE/REJECT`

- [ ] F2. **Code Quality Review** — `unspecified-high`
  Run `cargo make lint 2>&1 | tee temp.log` + `cargo make test 2>&1 | tee temp.log`. Run `scripts/check-line-count.sh`. Review all changed files for: `as any`/`@ts-ignore` (wrong language but check for Rust equivalents like `unsafe`, unnecessary `allow` attributes — must use `expect` instead), empty catches, debug prints in prod (`println!` outside test modules), commented-out code, unused imports. Check AI slop: excessive comments, over-abstraction, generic names (data/result/item/temp). Verify `no_std` compliance in core modules. Verify all public AND private items have documentation.
  Output: `Build [PASS/FAIL] | Lint [PASS/FAIL] | Tests [N pass/N fail] | Line Count [PASS/FAIL] | Files [N clean/N issues] | VERDICT`

- [ ] F3. **Real Manual QA** — `unspecified-high`
  Start from clean state (`cargo clean && cargo make build-all`). Execute EVERY QA scenario from EVERY task — follow exact steps, capture evidence. Test cross-task integration (hot reload with code generation, LSP with type checker, formatter with parser). Test edge cases: empty source files, files with only comments, Unicode identifiers, deeply nested expressions. Run ALL `language-spec/*.op` files end-to-end. Save evidence to `.sisyphus/evidence/final-qa/`.
  Output: `Scenarios [N/N pass] | Integration [N/N] | Edge Cases [N tested] | .op files [N/N pass] | VERDICT`

- [ ] F4. **Scope Fidelity Check** — `deep`
  For each task: read "What to do", read actual diff (git log/diff). Verify 1:1 — everything in spec was built (no missing), nothing beyond spec was built (no creep). Check "Must NOT do" compliance. Detect cross-task contamination: Task N touching Task M's files. Flag unaccounted changes. Verify all commit messages follow conventional commit format. Verify no `--no-verify` was used. Verify no `allow` attributes (must be `expect`).
  Output: `Tasks [N/N compliant] | Contamination [CLEAN/N issues] | Unaccounted [CLEAN/N files] | Commits [N/N compliant] | VERDICT`

---

## Final Verification Wave

---

## Commit Strategy

Each task commits independently with atomic, passing commits:
- `cargo make lint-fix` before every commit
- `cargo make lint 2>&1 | tee temp.log` must show zero warnings
- `cargo make test 2>&1 | tee temp.log` must show all tests passing
- `scripts/check-line-count.sh` must pass
- `git commit -m "message"` (never --no-verify)
- After each task, update the relevant plan file in `plan/` AND check off items in `PLAN.md`

---

## Success Criteria

### Verification Commands
```bash
cargo make test 2>&1 | tee temp.log     # Expected: all tests pass
cargo make lint 2>&1 | tee temp.log     # Expected: zero warnings
cargo make build-all 2>&1 | tee temp.log # Expected: successful build
scripts/check-line-count.sh             # Expected: all files compliant
```

### Final Checklist
- [ ] All "Must Have" items present
- [ ] All "Must NOT Have" items absent
- [ ] All `language-spec/*.op` files parse, type-check, and execute correctly
- [ ] All tests pass (`cargo make test`)
- [ ] All lints pass (`cargo make lint`)
- [ ] Hot reload demo functional
- [ ] LSP functional with error reporting
- [ ] Formatter produces consistent output
- [ ] Build system creates native executables
- [ ] PLAN.md fully checked off
