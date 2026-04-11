# End-to-End Test Projects — Compile, Link, Run, Verify

## TL;DR

> **Quick Summary**: Create realistic Opalescent test projects (hello-world, fib-recursive, fib-iterative, simple-quiz) under `test-projects/`, then build the missing compiler pipeline (orchestration → object emission → linker invocation) and fix codegen gaps (stdlib bindings, string interpolation, imports) so each project compiles to a native binary and produces correct output. A `src/lib.rs` library target exposes `compile_to_module` (pipeline without emission) and `compile_program` (full end-to-end with binary output) for integration tests in the main Rust project that verify everything end-to-end. Features are added TDD-style as each test project demands them.
> 
> **Deliverables**:
> - `test-projects/hello-world/` — realistic project structure, compiles and prints "Hello world"
> - `test-projects/fib-recursive/` — compiles and prints "55" (fib(10))
> - `test-projects/fib-iterative/` — compiles and prints "55" (fib(10))
> - `test-projects/simple-quiz/` — compiles and runs interactive quiz
> - `compile_to_module` orchestration function (parse → typecheck → codegen → LLVM module)
> - `compile_program` end-to-end function (compile_to_module → emit .o → link → binary path)
> - Object file emission via `TargetMachine::write_to_file`
> - Linker invocation via `cc`
> - Fixed stdlib/runtime bindings (print, take_input, random_int32, string_to_int32) — all using int64 ABI
> - String interpolation codegen
> - Import system codegen (for simple-quiz)
> - `src/lib.rs` library crate exposing `compile_to_module` and `compile_program` for integration tests
> - Integration test suite under `tests/` in main project
> - PLAN.md kept in sync throughout
> 
> **Estimated Effort**: Large
> **Parallel Execution**: YES — 5 waves
> **Critical Path**: Task 1 → Task 2 → Task 4 → Task 7 → Task 9 → Task 12 → Task 14 → Task 15 → F1-F4

---

## Context

### Original Request
"Please add a new folder called 'test-projects' at the top level. Within this folder, create multiple Opalescent test projects (one for each .op file in language-spec that has 'entry' in it). Each of these projects should have an automated test suite created in the main Opalescent project (NOT in the test project directory) that verifies that the project compiles and runs correctly, enforcing correct behavior. Create the simplest projects first (starting with the hello world) and increase in complexity. The goal is to end-to-end test the compiler in general, so make these extensively check values."

Also: "The test projects should be very realistic — IE, add .gitignore files, readme files, etc."

### Interview Summary
**Key Discussions**:
- **Testing approach**: Object file compilation → link with `cc` → run binary → capture stdout/exit code → assert → cleanup. Files stay within test-project dir.
- **String interpolation**: Implement in codegen first (user prefers real implementation over simplified test files)
- **Type checker**: Include in E2E pipeline (parse → typecheck → codegen)
- **simple_quiz.op**: Include it. Missing features implemented TDD-style as the test requires them.
- **PLAN.md sync**: Keep in sync with this plan as development progresses
- **Completion plan fixup**: Uncheck falsely-completed items in `opalescent-completion.md`

**Research Findings**:
- Codegen is scaffolding — `main.rs` only does lex+parse, no `compile_program` orchestration exists
- `resolve_callee_function` uses fallback `i64 fn()` stub for ALL unknown functions (print, take_input, etc.)
- `Expr::StringInterpolation` has no codegen match arm despite runtime `format_interpolated_string` existing
- Import system has no codegen handler
- Labeled break payloads are silently dropped — but simple-quiz has been REWRITTEN to avoid needing them (uses `while` + `let mutable` instead)
- Parser does NOT support colon-block syntax — test files MUST use brace syntax `{ }`
- Integer literals inferred as `int64` — test files must use `int64`, not `int32`
- `emit_c_main_wrapper` passes zero arguments — entry functions must be `f(): void`
- **Crate is binary-only** (`src/main.rs`, no `src/lib.rs`) — must create `src/lib.rs` to expose modules for integration tests
- Parser does NOT support loop-as-expression or multi-binding let — simple-quiz MUST NOT use `let x, y = loop => { break x: a, y: b }`; use `let mutable` + `while` instead

### Metis Review
**Identified Gaps** (addressed):
- **Brace syntax requirement**: Test .op files written in brace syntax (parser doesn't support colon-block)
- **int64 default**: All numeric types in test files use `int64`
- **Zero-arg entry**: Entry functions use `f(): void` (C wrapper passes no args)
- **String as i8 pointer**: `CoreType::String` → `i8*`, `string[]` would produce broken IR — avoided via zero-arg entry
- **`is` operator**: Needs verification in codegen — covered in Wave 3 validation
- **Bug fix budget**: Max 2 hours per bug; if longer, document and skip test
- **Integration test gating**: Use feature flag or separate test binary so `cargo test` works without LLVM
- **Crate structure**: Must create `src/lib.rs` to expose compile_to_module and compile_program for integration tests (`tests/` can't access binary crate internals)
- **Parser limitations**: Loop-as-expression and multi-binding `let` are NOT supported — simple-quiz rewritten to use `while` + `let mutable`
- **Labeled break payloads**: Removed from plan scope — simple-quiz no longer needs them
- **Test I/O override**: Integration tests (feature-gated) ARE allowed to write to `test-projects/<name>/target/` despite chatmode no-file-write rule — user explicitly confirmed during interview. Cleanup required.
- **compile_program signature split**: Task 1 creates `compile_to_module` (returns module, no emission). Task 2 creates `compile_program` on top (emit + link → returns PathBuf). Eliminates signature contradiction.
- **compile_program path contract**: `compile_program(source, output_dir)` accepts an **output directory**, constructs `program.o` and `program` filenames internally. All callers pass directories (e.g., `test-projects/<name>/target/`).
- **let mutable syntax**: Fib-iterative sample uses `let mutable` (NOT `let mut`) to match parser expectations.
- **string_to_int32 is a plain C call**: Returns `int64` (0 on parse error). Simple-quiz does NOT use `guard`/`propagate` with it — uses `if n is 0` check instead.
- **Type checker builtin signatures updated to int64**: `string_to_int32` signature changed from `f(string): int32 errors ParseError` to `f(string): int64` (no errors). `random_int32` changed from `f(int32, int32): int32` to `f(int64, int64): int64`. This matches the C ABI (int64_t) and the language's default integer literal type (int64). Existing type checker tests updated accordingly (Task 13).
- **inkwell lifetime handling**: `compile_to_module` accepts `&'ctx Context` from caller (standard inkwell pattern) so the returned `Module<'ctx>` has valid lifetime. `compile_program` creates its own `Context` internally — no lifetime leaks.
- **Runtime artifact location**: `runtime/opal_runtime.c` is linked directly as a source file via `cc program.o runtime/opal_runtime.c -o program` — no intermediate `.o` file is created outside `test-projects/<name>/target/`.

---

## Work Objectives

### Core Objective
Build a complete end-to-end compiler pipeline and verify it with realistic test projects that compile to native binaries and produce correct output.

### Concrete Deliverables
- 4 realistic test project directories under `test-projects/`
- `compile_to_module` function wiring parse → typecheck → codegen (returns LLVM module)
- `compile_program` function wiring compile_to_module → object emission → linking (returns binary path)
- `src/lib.rs` library crate target exposing compiler modules for integration tests
- Integration test suite that compiles each project, runs the binary, and asserts on stdout/exit code
- All missing codegen features implemented TDD-style as each test project requires them

### Definition of Done
- [ ] `cargo test` (without integration feature) — all existing tests pass (no regressions)
- [ ] `cargo test --features integration` — all 4 test projects compile, link, run, produce correct output
- [ ] `cargo make lint` — zero warnings
- [ ] Each test project has: `opal.toml`, `.gitignore`, `README.md`, `src/main.op`
- [ ] No test artifacts remain after test completion (cleaned up)
- [ ] PLAN.md reflects current progress

### Must Have
- TDD for ALL new features (red-green-refactor, all 3 steps)
- Atomic commits (each commit = unit of work, tests + lint pass)
- Brace syntax `{ }` in all test .op files (parser doesn't support colon-block)
- `int64` for all numeric types in test files (type checker default)
- `f(): void` entry functions (C main wrapper passes zero args)
- Integration tests gated behind feature flag
- Integration tests MAY write to `test-projects/<name>/target/` (explicit override of chatmode no-file-write rule — see Verification Strategy)
- Test artifacts cleaned up after each test (Drop guard or explicit cleanup)
- All assertions with descriptive messages
- Doc comments on ALL items (public and private)

### Must NOT Have (Guardrails)
- No colon-block syntax in test .op files (parser doesn't support it)
- No `int32` numeric types in test .op files (use `int64`)
- No `args: string[]` parameter on entry functions (C wrapper passes zero args)
- No `unwrap()`, `expect()`, `panic!()`, `todo!()`, `unimplemented!()`
- No `as` conversions (use TryFrom/TryInto)
- No `str.to_string()` (use `to_owned()` or `String::from()`)
- No `HashMap` in core modules (use `BTreeMap`)
- No `#[allow(...)]` without reason (use `#[expect(..., reason = "...")]`)
- No `--no-verify` on git commands
- No newly created files exceeding 500 lines (1000 for test files). Existing files already exceeding this limit (e.g., `src/type_system/tests.rs`) are exempt — do not refactor them as part of this plan. Enforcement is via `scripts/check-line-count.sh` (uses 1000-line limit and excludes certain test filenames).
- No test artifacts written outside `test-projects/<name>/target/`
- No more than 2 hours per compiler bug — document and skip if longer
- No modifying .git, AGENTS.md, target, scripts, Makefile.toml, or lint rules

---

## Verification Strategy (MANDATORY)

> **ZERO HUMAN INTERVENTION** — ALL verification is agent-executed. No exceptions.

### Test Decision
- **Infrastructure exists**: YES (cargo test with cargo-make)
- **Automated tests**: YES (TDD — mandated by chatmode)
- **Framework**: cargo test (Rust built-in) via `cargo make test`
- **TDD**: Each task follows RED (failing test) → GREEN (minimal impl) → REFACTOR
- **Integration gating**: Feature flag `integration` — `cargo test --features integration`

### Integration Test I/O Exception (EXPLICIT OVERRIDE — CODIFIED IN CHATMODE)
> The repo chatmode (`.github/chatmodes/principal-engineer.chatmode.md`) states tests must "NEVER alter any files."
> **This plan explicitly overrides that rule for `integration` feature-gated tests ONLY**, per user confirmation during interview.
>
> **Codification step (Task 1 MUST perform this)**: Update `.github/chatmodes/principal-engineer.chatmode.md` line 43 to add the exception. Change:
> ```
> All new code should be well-tested. All tests should NEVER, UNDER ANY CIRCUMSTANCES, actually alter any files on the machine. They must be mocked or stubbed out in their entirety.
> ```
> to:
> ```
> All new code should be well-tested. All tests should NEVER, UNDER ANY CIRCUMSTANCES, actually alter any files on the machine. They must be mocked or stubbed out in their entirety. **Exception**: Integration tests gated by `#[cfg(feature = "integration")]` MAY write temporary compilation artifacts (.o files, binaries) to `test-projects/<name>/target/`, provided ALL artifacts are cleaned up after each test.
> ```
> This ensures the authoritative ruleset matches the plan — no downstream conflict.
>
> **Allowed**: Integration tests (under `tests/`, gated by `#[cfg(feature = "integration")]`) MAY:
> - Write temporary `.o` files and binaries to `test-projects/<name>/target/` or OS temp dirs (for isolated emit/link tests)
> - These are compilation artifacts required to verify the end-to-end pipeline
> - This includes Task 2's object emission and linker tests, as well as Tasks 7/10/14 E2E tests
>
> **Required**: ALL such artifacts MUST be cleaned up after each test (use `Drop` guard or explicit cleanup in test teardown).
>
> **Forbidden**: Integration tests MUST NOT write files outside `test-projects/<name>/target/`. No artifacts may persist after test completion.

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
- `.github/chatmodes/refactor-lint.chatmode.md` (full file — detailed lint rule explanations)
- `language-spec/requirements/overview.md` (language design specification)
- `PLAN.md` (full file — master project plan)
- `ERROR_HANDLING_STANDARDS.md` (error handling patterns)
- `Makefile.toml` (to understand lint rules — READ-ONLY, never modify)
- `FIXES.txt` (known bug fixes — read to avoid reintroducing)

---

## Execution Strategy

### Parallel Execution Waves

> Tasks within each wave can run in parallel. Each wave completes before the next begins.
> Complexity increases with each wave: void program → hello world → fibonacci → simple quiz.

```
Wave 1 (Pipeline Foundation — everything depends on this):
├── Task 1: compile_to_module orchestration function + lib.rs [deep]
├── Task 2: Object file emission + linker invocation [deep]
├── Task 3: Fix resolve_callee_function stdlib prototypes (print → puts) [deep]
├── Task 4: Smoke test — void program compiles and runs [deep]

Wave 2 (Hello World — first real program):
├── Task 5: Create test-projects/hello-world/ project structure [quick]
├── Task 6: String interpolation codegen [deep]
├── Task 7: Integration test for hello-world [deep]

Wave 3 (Fibonacci — arithmetic + control flow):
├── Task 8: Create test-projects/fib-recursive/ and test-projects/fib-iterative/ [quick]
├── Task 9: Verify/fix `is` operator codegen for equality [deep]
├── Task 10: Integration tests for both fib projects [deep]

Wave 4 (Simple Quiz — advanced features):
├── Task 11: Create test-projects/simple-quiz/ project structure [quick]
├── Task 12: Import system codegen [deep]
├── Task 13: Fix take_input/random_int32/string_to_int32 bindings [deep]
├── Task 14: Integration test for simple-quiz [deep]

Wave 5 (Polish + Sync):
├── Task 15: PLAN.md sync + completion plan fixup [quick]
├── Task 16: README.md escape hatch documentation [quick]

Wave FINAL (After ALL tasks — 4 parallel reviews, then user okay):
├── Task F1: Plan compliance audit (oracle)
├── Task F2: Code quality review (unspecified-high)
├── Task F3: Real manual QA — compile and run all test projects (unspecified-high)
├── Task F4: Scope fidelity check (deep)
-> Present results -> Get explicit user okay
```

### Dependency Matrix

| Task | Depends On | Blocks |
|------|-----------|--------|
| 1 | None | 2, 4 |
| 2 | 1 | 4 |
| 3 | None | 4, 7 |
| 4 | 1, 2, 3 | 7, 10, 14 |
| 5 | None | 7 |
| 6 | 3 | 7 |
| 7 | 4, 5, 6 | 10 |
| 8 | None | 10 |
| 9 | 4 | 10 |
| 10 | 7, 8, 9 | 14 |
| 11 | None | 14 |
| 12 | 4 | 14 |
| 13 | 3 | 14 |
| 14 | 10, 11, 12, 13 | 15 |
| 15 | 14 | F1-F4 |
| 16 | 14 | F1-F4 |

### Agent Dispatch Summary

- **Wave 1**: 4 tasks — T1-T4 → `deep`
- **Wave 2**: 3 tasks — T5 → `quick`, T6 → `deep`, T7 → `deep`
- **Wave 3**: 3 tasks — T8 → `quick`, T9 → `deep`, T10 → `deep`
- **Wave 4**: 4 tasks — T11 → `quick`, T12-T13 → `deep`, T14 → `deep`
- **Wave 5**: 2 tasks — T15-T16 → `quick`
- **FINAL**: 4 tasks — F1 → `oracle`, F2 → `unspecified-high`, F3 → `unspecified-high`, F4 → `deep`

---

## TODOs

> Implementation + Test = ONE Task. Never separate.
> EVERY task follows TDD: write failing test FIRST → minimal implementation → refactor.
> **A task WITHOUT QA Scenarios is INCOMPLETE. No exceptions.**

- [x] 1. Build `compile_to_module` Orchestration Function + Create Library Crate

  **What to do**:
  - Read all pre-task mandatory files
  - **CODIFY INTEGRATION TEST I/O EXCEPTION**: Update `.github/chatmodes/principal-engineer.chatmode.md` line 43 to append the integration test exception. The current text:
    ```
    All new code should be well-tested. All tests should NEVER, UNDER ANY CIRCUMSTANCES, actually alter any files on the machine. They must be mocked or stubbed out in their entirety.
    ```
    must become:
    ```
    All new code should be well-tested. All tests should NEVER, UNDER ANY CIRCUMSTANCES, actually alter any files on the machine. They must be mocked or stubbed out in their entirety. **Exception**: Integration tests gated by `#[cfg(feature = "integration")]` MAY write temporary compilation artifacts (.o files, binaries) to `test-projects/<name>/target/`, provided ALL artifacts are cleaned up after each test.
    ```
    This ensures the authoritative project ruleset matches the agreed plan. Without this change, downstream agents reading the chatmode will see a conflict.
  - **CRITICAL — Create `src/lib.rs`**: The project is currently binary-only (`src/main.rs` declares all modules). Integration tests under `tests/` can only access a library crate's public API. You MUST:
    1. Create `src/lib.rs` that declares all existing modules currently in `main.rs` (e.g., `pub mod codegen; pub mod parser; pub mod type_system;` etc.)
    2. Move module declarations from `src/main.rs` to `src/lib.rs` — main.rs should `use opalescent::*` or specific imports instead of declaring `mod` itself
    3. Keep `main.rs` as a thin binary entry point that imports from the library crate
    4. Verify `Cargo.toml` has both `[[bin]]` and `[lib]` targets (Cargo auto-detects when both `src/main.rs` and `src/lib.rs` exist, but verify)
  - Create a new module (e.g., `src/compiler.rs` or `src/codegen/driver.rs`) with a `compile_to_module` function, exposed via `src/lib.rs`
  - Signature: `fn compile_to_module<'ctx>(context: &'ctx Context, source: &str) -> Result<Module<'ctx>, CompileError>` — takes a caller-provided inkwell `Context` to avoid lifetime issues (the returned `Module` borrows the `Context`, so the caller must keep it alive)
  - **CRITICAL inkwell lifetime note**: `CodegenContext<'context>` borrows `&'context Context` and holds `Module<'context>`. If `compile_to_module` created its own `Context` internally, the returned `Module` would reference a dropped value. By accepting `&'ctx Context` from the caller, the `Module`'s lifetime is correctly tied to the caller's `Context`. This is the standard inkwell pattern.
  - Pipeline: tokenize (Lexer) → parse (Parser) → type-check (TypeChecker) → codegen (CodegenContext + codegen_function_declaration for all Decl::Function) → return `Module<'ctx>`
  - Create a `CompileError` enum that wraps LexError, ParseError, TypeError, CodegenError
  - Use miette for error reporting throughout
  - Wire the full pipeline but do NOT emit object files yet (that's Task 2) — this function only produces a verified LLVM module
  - Write TDD tests FIRST: test that valid source produces Ok(Module), test that invalid source produces appropriate errors at each stage (lex error, parse error, type error). Tests create their own `Context::create()` and pass a reference.
  - Run lint, test, line-count checks
  - Commit

  **Must NOT do**:
  - Do not implement object file emission (Task 2)
  - Do not modify Makefile.toml or scripts
  - Do not break existing tests — `cargo make test` must still pass after restructuring
  - All guardrails from Must NOT Have section apply

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: Multi-module integration spanning lexer, parser, type system, and codegen with error type design
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES (with Task 3)
  - **Parallel Group**: Wave 1
  - **Blocks**: Tasks 2, 4
  - **Blocked By**: None

  **References**:

  **Pattern References**:
  - `src/main.rs:11-41` — Current module declarations (`mod ast; #[path = "..."] pub mod benchmarks;` etc.) — these MUST be moved to `src/lib.rs`
  - `src/type_system/test_integration.rs:41-58` — Parse pipeline pattern showing lex → parse → typecheck flow
  - `src/main.rs:108-170` — Current lex + parse flow (extend, don't replace)
  - `src/codegen/context.rs` — CodegenContext::new() for creating codegen context
  - `src/codegen/functions.rs:codegen_function_declaration` — How to lower each function declaration

  **API/Type References**:
  - `src/lexer.rs` — Lexer struct and tokenize API
  - `src/parser.rs` — Parser struct and parse API
  - `src/type_system/checker.rs` — TypeChecker struct and check API
  - `src/codegen/context.rs` — CodegenContext struct
  - `src/error.rs` — Existing LexError patterns with miette
  - `Cargo.toml` — Verify dual target setup (`[[bin]]` + `[lib]`)

  **Test References**:
  - `src/type_system/test_integration.rs` — Integration test patterns to follow
  - `src/codegen/tests.rs` — Codegen test patterns

  **Acceptance Criteria**:
  - [ ] `.github/chatmodes/principal-engineer.chatmode.md` line 43 updated to include integration test I/O exception
  - [ ] `src/lib.rs` exists, declares all modules previously in `main.rs`
  - [ ] `src/main.rs` imports from the library crate (`use opalescent::...`) instead of declaring `mod` itself
  - [ ] `cargo make test` still passes after restructuring (no regressions)
  - [ ] `compile_to_module(&context, "entry main = f(): void => { return void }")` returns `Ok(Module)` with valid LLVM module
  - [ ] `compile_to_module(&context, "invalid syntax @#$")` returns `Err(CompileError::Lex(...))`
  - [ ] `compile_to_module(&context, "let x: int64 = true")` returns `Err(CompileError::Type(...))`
  - [ ] Pipeline runs: lex → parse → typecheck → codegen in sequence
  - [ ] `cargo make test` passes, `cargo make lint` passes

  **QA Scenarios**:

  ```
  Scenario: Library crate created and all existing tests pass
    Tool: Bash (cargo test)
    Preconditions: src/lib.rs created, module declarations moved from main.rs
    Steps:
      1. Run `ls src/lib.rs` — verify file exists
      2. Run `cargo make test 2>&1 | tee temp.log`
      3. Read temp.log, verify ALL existing tests pass (zero regressions)
      4. Run `grep "mod " src/main.rs` — verify main.rs no longer declares modules (uses imports instead)
    Expected Result: lib.rs exists, main.rs is thin, all tests pass
    Failure Indicators: Missing lib.rs, main.rs still declares modules, test regressions
    Evidence: .sisyphus/evidence/task-1-lib-crate.txt

  Scenario: Valid void program compiles through pipeline
    Tool: Bash (cargo test)
    Preconditions: compile_to_module function implemented
    Steps:
      1. Run `cargo make test 2>&1 | tee temp.log`
      2. Read temp.log, verify test "compile_to_module_valid_void_program" passes
    Expected Result: Test passes, compile_to_module returns Ok(Module) with valid LLVM module
    Failure Indicators: Test failure, panic, or compile error
    Evidence: .sisyphus/evidence/task-1-compile-to-module.txt

  Scenario: Invalid source produces appropriate error stage
    Tool: Bash (cargo test)
    Preconditions: Error variant tests implemented
    Steps:
      1. Run `cargo make test 2>&1 | tee temp.log`
      2. Read temp.log, verify error variant tests pass
    Expected Result: Each error type (lex, parse, type) is correctly reported
    Evidence: .sisyphus/evidence/task-1-error-variants.txt
  ```

  **Commit**: YES
  - Message: `feat(compiler): create lib.rs crate structure and add compile_to_module orchestration`
  - Pre-commit: `cargo make lint-fix && cargo make test`

- [x] 2. Object File Emission + Linker Invocation

  **What to do**:
  - Read all pre-task mandatory files
  - Add `emit_object_file(module: &Module, path: &Path) -> Result<(), CodegenError>` function (or method on a helper) that uses the module's target machine to write the object file
  - Use `target_machine.write_to_file(module, FileType::Object, path)`
  - Import `inkwell::targets::FileType`
  - Add `link_object_file(object_path: &Path, output_path: &Path, extra_sources: &[&Path]) -> Result<PathBuf, CompileError>` function
  - Use `std::process::Command::new("cc")` with args `[object_path, ...extra_sources, "-o", output_path]`
  - `extra_sources` allows linking additional C source files alongside the .o (used by Task 13 for `runtime/opal_runtime.c`)
  - Handle linker errors (non-zero exit, stderr capture)
  - Create `compile_program(source: &str, output_dir: &Path) -> Result<PathBuf, CompileError>` that chains: creates `Context::create()` internally → `compile_to_module(&context, source)` → `emit_object_file(&module, output_dir/program.o)` → `link_object_file(output_dir/program.o, output_dir/program, &[])` → return binary path (`output_dir/program`)
  - `compile_program` accepts an **output directory** (not a file path) — it constructs `program.o` and `program` filenames within that directory internally
  - `compile_program` owns the inkwell `Context` internally — the `Module`'s lifetime is bounded by the function scope, no lifetime leaks to the caller
  - `compile_program` is the full end-to-end function that integration tests will use
  - Note: Task 13 will later extend `compile_program` to pass `runtime/opal_runtime.c` in `extra_sources` when stdlib functions are used
  - Write TDD tests FIRST: test that emit_object_file creates a .o file, test that link_object_file produces an executable. **IMPORTANT**: These tests write real files to disk, so they MUST be gated behind `#[cfg(feature = "integration")]` (same as all other file-writing tests). Place them in `tests/integration_e2e.rs` alongside the other integration tests, not in unit test modules.
  - Run lint, test, line-count checks (use `cargo test --features integration` to run the new tests)
  - Commit

  **Must NOT do**:
  - Do not hardcode output paths — caller specifies directory
  - Do not leave test artifacts behind (clean up in tests)
  - All guardrails apply

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: LLVM target machine integration with file system and external process orchestration
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: NO (depends on Task 1)
  - **Parallel Group**: Wave 1 (sequential after Task 1)
  - **Blocks**: Task 4
  - **Blocked By**: Task 1

  **References**:

  **Pattern References**:
  - `src/codegen/context.rs` — CodegenContext with `target_machine: Option<TargetMachine>` field
  - `src/codegen/functions.rs:421-443` — `emit_c_main_wrapper` creates C-ABI `main()` (needed for linking)

  **API/Type References**:
  - `inkwell::targets::FileType` — `FileType::Object` for object file emission
  - `inkwell::targets::TargetMachine::write_to_file` — Method to emit object code
  - `std::process::Command` — For invoking `cc` linker

  **External References**:
  - inkwell docs: TargetMachine::write_to_file API

  **Acceptance Criteria**:
  - [ ] `emit_object_file(path)` creates a valid .o file at specified path
  - [ ] `link_object_file(obj, out)` produces an executable binary
  - [ ] `compile_program(source, output_dir)` accepts a directory, creates `program.o` + `program` inside it, returns `PathBuf` to the binary
  - [ ] Produced binary is runnable (`Command::new(binary).output()` succeeds)
  - [ ] Linker errors are captured and reported as CompileError
  - [ ] Test artifacts cleaned up after test
  - [ ] `cargo make test` passes, `cargo make lint` passes

  **QA Scenarios**:

  ```
  Scenario: Object file emission produces valid .o
    Tool: Bash (cargo test)
    Preconditions: emit_object_file implemented, compile_program wired
    Steps:
      1. Run `cargo make test 2>&1 | tee temp.log`
      2. Read temp.log, verify "emit_object_file_creates_valid_object" test passes
    Expected Result: .o file exists at specified path after emission
    Failure Indicators: File not created, LLVM write error, wrong file type
    Evidence: .sisyphus/evidence/task-2-object-emission.txt

  Scenario: Linker produces executable binary
    Tool: Bash (cargo test)
    Preconditions: link_object_file implemented
    Steps:
      1. Run `cargo make test 2>&1 | tee temp.log`
      2. Read temp.log, verify "link_produces_executable" test passes
    Expected Result: Binary file exists and is executable
    Evidence: .sisyphus/evidence/task-2-linker.txt

  Scenario: Linker failure produces clear error
    Tool: Bash (cargo test)
    Preconditions: Error handling implemented
    Steps:
      1. Run `cargo make test 2>&1 | tee temp.log`
      2. Verify test for invalid .o file → linker error → CompileError
    Expected Result: CompileError with stderr from linker
    Evidence: .sisyphus/evidence/task-2-linker-error.txt
  ```

  **Commit**: YES
  - Message: `feat(codegen): add object file emission and linker invocation`
  - Pre-commit: `cargo make lint-fix && cargo make test`

- [x] 3. Fix `resolve_callee_function` — Correct Stdlib Prototypes

  **What to do**:
  - Read all pre-task mandatory files
  - Read `src/codegen/functions.rs` — find `resolve_callee_function` (currently creates fallback `i64 fn()` for unknown names)
  - Replace fallback with a stdlib prototype registry: when function name matches a known stdlib name, emit the correct LLVM function declaration instead of the generic fallback
  - At minimum, wire `print` → `declare i32 @puts(i8*)` (C `puts` for string printing)
  - Also add: `declare i32 @printf(i8*, ...)` (for formatted output like int printing)
  - Consider a helper function `declare_stdlib_function(module, name)` that maps known names to correct prototypes
  - Use `lsp_find_references` on `resolve_callee_function` before modifying to understand all callers
  - Write TDD tests FIRST: test that `resolve_callee_function("print", ...)` returns a function with `puts` signature, test that unknown functions still get a reasonable fallback or error
  - Run lint, test, line-count checks
  - Commit

  **Must NOT do**:
  - Do not break existing codegen tests that rely on current resolve_callee_function behavior
  - Do not implement all stdlib functions yet (just print for now; take_input/random/etc. in Task 14)
  - All guardrails apply

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: Critical codegen function modification requiring understanding of all callers and LLVM function type construction
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES (with Task 1)
  - **Parallel Group**: Wave 1
  - **Blocks**: Tasks 4, 7, 14
  - **Blocked By**: None

  **References**:

  **Pattern References**:
  - `src/codegen/functions.rs` — `resolve_callee_function` (the function to modify)
  - `src/codegen/functions.rs` — `build_function_type` (how LLVM function types are constructed)
  - `src/codegen/context.rs` — `CodegenContext` module for adding function declarations

  **API/Type References**:
  - `inkwell::module::Module::add_function` — Adding external function declarations
  - `inkwell::types::FunctionType` — Building correct function signatures
  - C stdlib: `int puts(const char*)`, `int printf(const char*, ...)`

  **Test References**:
  - `src/codegen/tests.rs` — Existing codegen tests that call resolve_callee_function

  **Acceptance Criteria**:
  - [ ] `resolve_callee_function("print", ...)` returns function with `puts`-compatible signature
  - [ ] Existing codegen tests still pass (no regressions)
  - [ ] Unknown functions produce a meaningful error (not a silent i64 fallback)
  - [ ] `cargo make test` passes, `cargo make lint` passes

  **QA Scenarios**:

  ```
  Scenario: print resolves to puts prototype
    Tool: Bash (cargo test)
    Preconditions: Stdlib prototype registry implemented
    Steps:
      1. Run `cargo make test 2>&1 | tee temp.log`
      2. Read temp.log, verify "resolve_print_to_puts" test passes
      3. Verify LLVM IR contains `declare i32 @puts(i8*)`
    Expected Result: print() calls lower to puts() calls in LLVM IR
    Failure Indicators: Fallback i64 fn() still emitted, or existing tests break
    Evidence: .sisyphus/evidence/task-3-stdlib-prototypes.txt

  Scenario: Existing codegen tests still pass
    Tool: Bash (cargo test)
    Steps:
      1. Run `cargo make test 2>&1 | tee temp.log`
      2. Verify zero test regressions
    Expected Result: All pre-existing tests pass
    Evidence: .sisyphus/evidence/task-3-no-regressions.txt
  ```

  **Commit**: YES
  - Message: `fix(codegen): emit correct stdlib prototypes instead of i64 fallback`
  - Pre-commit: `cargo make lint-fix && cargo make test`

- [x] 4. Smoke Test — Void Program Compiles, Links, and Runs

  **What to do**:
  - Read all pre-task mandatory files
   - Create integration test file: `tests/integration_e2e.rs` (or similar)
   - Gate with `#[cfg(feature = "integration")]`
   - Add `integration` feature to `Cargo.toml` under `[features]`
   - Import `compile_program` from the library crate: `use opalescent::compiler::compile_program;` (or wherever Task 1 placed it)
   - Write a smoke test that:
    1. Defines source: `entry main = f(): void => { return void }`
    2. Creates a temp directory inside the project (e.g., `test-projects/_smoke/target/`)
    3. Calls `compile_program(source, temp_dir)` to produce a binary
    4. Runs the binary with `Command::new(binary).output()`
    5. Asserts exit code is 0
    6. Asserts stdout is empty
    7. Cleans up temp directory
  - This is RED step of TDD — write the test FIRST, watch it fail (compile_program not fully wired), then fix
  - Ensure temp directory is always cleaned up (use Drop guard or `finally` pattern)
  - Run lint, test (with `--features integration`), line-count checks
  - Commit

  **Must NOT do**:
  - Do not create test artifacts outside project directory
  - Do not skip cleanup on test failure
  - All guardrails apply

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: First E2E integration test requiring coordination of compile + link + run + cleanup
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: NO (depends on Tasks 1, 2, 3)
  - **Parallel Group**: Wave 1 (sequential, last in wave)
  - **Blocks**: Tasks 7, 10, 15
  - **Blocked By**: Tasks 1, 2, 3

  **References**:

  **Pattern References**:
  - `src/type_system/test_integration.rs` — Integration test patterns
  - `src/codegen/tests.rs` — Test setup patterns with inkwell contexts
  - `Cargo.toml` — Where to add `[features]` section

  **API/Type References**:
  - `std::process::Command` — Running compiled binary
  - `std::fs` — Temp directory creation and cleanup
  - `tempfile` crate — Consider for reliable temp dir management (check if already a dependency)

  **Acceptance Criteria**:
  - [ ] Integration test file exists at `tests/integration_e2e.rs`
  - [ ] `integration` feature exists in Cargo.toml
  - [ ] `cargo test --features integration test_smoke_void_program` passes
  - [ ] Binary exits with code 0, empty stdout
  - [ ] No artifacts remain after test
  - [ ] `cargo make test` (without integration) still passes all existing tests
  - [ ] `cargo make lint` passes

  **QA Scenarios**:

  ```
  Scenario: Void program compiles and runs with exit 0
    Tool: Bash (cargo test --features integration)
    Preconditions: Tasks 1, 2, 3 complete
    Steps:
      1. Run `cargo test --features integration test_smoke 2>&1 | tee temp.log`
      2. Read temp.log, verify "test_smoke_void_program" passes
      3. Verify no .o or binary files remain in test-projects/_smoke/
    Expected Result: Test passes, exit code 0, empty stdout, clean cleanup
    Failure Indicators: Compilation failure, link failure, non-zero exit, leftover files
    Evidence: .sisyphus/evidence/task-4-smoke-test.txt

  Scenario: Existing tests unaffected by integration feature
    Tool: Bash (cargo test)
    Preconditions: Integration feature added to Cargo.toml
    Steps:
      1. Run `cargo make test 2>&1 | tee temp.log`
      2. Verify all existing tests still pass
    Expected Result: Zero regressions, integration tests skipped without feature flag
    Evidence: .sisyphus/evidence/task-4-no-regressions.txt
  ```

  **Commit**: YES
  - Message: `test(integration): add smoke test for void program compilation and execution`
  - Pre-commit: `cargo make lint-fix && cargo make test`

- [x] 5. Create `test-projects/hello-world/` Project Structure

  **What to do**:
  - Create directory: `test-projects/hello-world/`
  - Create `test-projects/hello-world/opal.toml`:
    ```toml
    name = "hello-world"
    version = "1.0.0"
    ```
  - Create `test-projects/hello-world/.gitignore`:
    ```
    /target/
    *.o
    ```
  - Create `test-projects/hello-world/README.md` with brief project description
  - Create `test-projects/hello-world/src/main.op` with brace-syntax hello world:
    ```
    ## Description: Entry point for hello world test project ##
    entry main = f(): void => {
        let world = 'world'
        print('Hello {world}')
        return void
    }
    ```
    NOTE: This uses string interpolation which will be implemented in Task 6. If Task 6 is not yet complete, use `print('Hello world')` as a temporary fallback and update after Task 6.
  - Create `test-projects/hello-world/target/` directory with `.gitkeep` or add to `.gitignore`
  - Verify project structure matches the Opalescent project conventions from README.md
  - Commit

  **Must NOT do**:
  - Do not use colon-block syntax in .op files
  - Do not use `int32` (use `int64` if numeric types needed)
  - Do not put `args: string[]` on entry function
  - Do not create test files (tests go in main project in Task 7)

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: File creation only, no code logic
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES (with Tasks 6)
  - **Parallel Group**: Wave 2
  - **Blocks**: Task 7
  - **Blocked By**: None (can start immediately, but Wave 1 must complete before Task 7)

  **References**:

  **Pattern References**:
  - `README.md` — Project Architecture section showing directory layout, opal.toml structure
  - `language-spec/hello_world.op` — Original hello world (reference for intent, NOT for syntax — must convert to brace syntax)

  **Acceptance Criteria**:
  - [ ] `test-projects/hello-world/opal.toml` exists with name and version
  - [ ] `test-projects/hello-world/.gitignore` exists, ignores target/ and *.o
  - [ ] `test-projects/hello-world/README.md` exists with description
  - [ ] `test-projects/hello-world/src/main.op` exists with valid brace-syntax hello world
  - [ ] Entry function uses `f(): void` (no args parameter)

  **QA Scenarios**:

  ```
  Scenario: Project structure is complete and realistic
    Tool: Read tool + Glob tool
    Steps:
      1. Use Glob to find all files under `test-projects/hello-world/`
      2. Verify opal.toml, .gitignore, README.md, src/main.op are present
      3. Use Read to read `test-projects/hello-world/src/main.op`
      4. Verify it uses brace syntax, f(): void, no int32
    Expected Result: All files present with correct content
    Evidence: .sisyphus/evidence/task-5-hello-world-structure.txt
  ```

  **Commit**: YES
  - Message: `chore: create test-projects/hello-world project structure`
  - Pre-commit: `cargo make lint-fix && cargo make test`

- [x] 6. String Interpolation Codegen

  **What to do**:
  - Read all pre-task mandatory files
  - Read `src/codegen/expressions.rs` — find the catch-all `_ => Err(...)` that drops `Expr::StringInterpolation`
  - Read `src/runtime/stdlib.rs` — find `format_interpolated_string` runtime implementation for reference
  - Read `src/ast.rs` — understand `Expr::StringInterpolation` structure (parts: Vec of string literals and expressions)
  - Add a match arm for `Expr::StringInterpolation` in `codegen_expression`
  - Implementation strategy: for simple cases (single `{expr}` in string), lower to:
    1. Codegen each literal part as a global string constant
    2. Codegen each expression part and convert to string representation
    3. Concatenate all parts into a single string (may need runtime helper or inline sprintf)
  - For initial implementation, consider simplifying: if interpolation has only literal parts + simple variable references, emit as `printf` format string with arguments
  - Alternative: emit call to a runtime `format_interpolated_string` function that takes a format pattern and value array
  - Declare necessary external functions (printf, sprintf, or custom runtime helper)
  - Write TDD tests FIRST (in codegen/tests.rs): test that `Expr::StringInterpolation` with one variable produces valid IR containing format/concat logic
  - Run lint, test, line-count checks
  - Commit

  **Must NOT do**:
  - Do not implement complex memory management / GC for strings
  - Do not handle nested interpolation (e.g., `'{'{x}'}'`)
  - All guardrails apply

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: New codegen path requiring string handling, runtime integration, and LLVM IR construction for string operations
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES (with Task 5)
  - **Parallel Group**: Wave 2
  - **Blocks**: Task 7
  - **Blocked By**: Task 3 (stdlib prototypes needed for printf/puts integration)

  **References**:

  **Pattern References**:
  - `src/codegen/expressions.rs:159-161` — Catch-all that drops StringInterpolation (the code to extend)
  - `src/codegen/expressions.rs` — Other Expr match arms showing codegen_expression patterns
  - `src/codegen/functions.rs:resolve_callee_function` — How to declare external functions

  **API/Type References**:
  - `src/ast.rs` — `Expr::StringInterpolation` AST structure
  - `src/runtime/stdlib.rs:format_interpolated_string` — Runtime reference implementation
  - C stdlib: `int sprintf(char*, const char*, ...)` for formatting

  **Test References**:
  - `src/codegen/tests.rs` — Existing codegen tests showing IR assertion patterns

  **Acceptance Criteria**:
  - [ ] `Expr::StringInterpolation` with `'Hello {world}'` produces valid LLVM IR
  - [ ] Generated IR contains string constant for literal parts
  - [ ] Generated IR contains call to format/concat for interpolated parts
  - [ ] No existing codegen tests break
  - [ ] `cargo make test` passes, `cargo make lint` passes

  **QA Scenarios**:

  ```
  Scenario: String interpolation produces valid IR
    Tool: Bash (cargo test)
    Preconditions: StringInterpolation codegen arm implemented
    Steps:
      1. Run `cargo make test 2>&1 | tee temp.log`
      2. Read temp.log, verify "codegen_string_interpolation" test passes
      3. Verify IR contains string constants and format/concat calls
    Expected Result: Valid IR generated for interpolation expressions
    Failure Indicators: Unsupported expression error, invalid IR, crashes
    Evidence: .sisyphus/evidence/task-6-string-interpolation.txt

  Scenario: Simple interpolation with one variable works
    Tool: Bash (cargo test)
    Steps:
      1. Run `cargo make test 2>&1 | tee temp.log`
      2. Verify test for `'Hello {name}'` interpolation passes
    Expected Result: IR has string constant "Hello " and variable reference
    Evidence: .sisyphus/evidence/task-6-simple-interpolation.txt
  ```

  **Commit**: YES
  - Message: `feat(codegen): implement string interpolation expression lowering`
  - Pre-commit: `cargo make lint-fix && cargo make test`

- [x] 7. Integration Test for Hello World

  **What to do**:
  - Read all pre-task mandatory files
  - Add test to `tests/integration_e2e.rs` (created in Task 4):
    ```rust
    #[test]
    #[cfg(feature = "integration")]
    fn test_hello_world_e2e() {
        // 1. Read test-projects/hello-world/src/main.op
        // 2. Call compile_program(source, test-projects/hello-world/target/)
        // 3. Run produced binary
        // 4. Assert stdout contains "Hello world"
        // 5. Assert exit code is 0
        // 6. Clean up target/ directory
    }
    ```
  - Test should read the .op source file from disk (this is an integration test, file I/O is expected)
  - Use absolute or project-relative path resolution for test-projects/
  - Verify cleanup: assert target/ dir is empty after test
  - If string interpolation doesn't produce expected output, adjust hello-world's main.op to use literal `print('Hello world')` and add a TODO for updating after interpolation is fully working
  - Write the test FIRST (RED), run it, watch it fail, then ensure all dependencies (Tasks 1-6) are complete
  - Run lint, test (with integration), line-count checks
  - Commit

  **Must NOT do**:
  - Do not put test files in test-projects/ (tests live in main project)
  - Do not leave artifacts after test
  - All guardrails apply

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: First full E2E test reading real .op file, compiling, running, and asserting behavior
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: NO (depends on Tasks 4, 5, 6)
  - **Parallel Group**: Wave 2 (last in wave)
  - **Blocks**: Task 10
  - **Blocked By**: Tasks 4, 5, 6

  **References**:

  **Pattern References**:
  - `tests/integration_e2e.rs` — Smoke test from Task 4 (extend this file)
  - `test-projects/hello-world/src/main.op` — Source file to compile

  **Acceptance Criteria**:
  - [ ] `cargo test --features integration test_hello_world` passes
  - [ ] stdout output is `"Hello world\n"` (or similar expected output)
  - [ ] Exit code is 0
  - [ ] No artifacts remain in `test-projects/hello-world/target/` after test
  - [ ] `cargo make test` (without integration) still passes

  **QA Scenarios**:

  ```
  Scenario: Hello world compiles and prints correct output
    Tool: Bash (cargo test --features integration)
    Preconditions: Tasks 1-6 complete, hello-world project exists
    Steps:
      1. Run `cargo test --features integration test_hello_world 2>&1 | tee temp.log`
      2. Read temp.log, verify test passes
      3. Run `ls test-projects/hello-world/target/` — should be empty or nonexistent
    Expected Result: Test passes, stdout == "Hello world\n", exit 0, clean target/
    Failure Indicators: Compilation failure, wrong output, non-zero exit, leftover files
    Evidence: .sisyphus/evidence/task-7-hello-world-e2e.txt

  Scenario: Test reads source from actual project directory
    Tool: Bash (cargo test --features integration)
    Steps:
      1. Verify test reads from test-projects/hello-world/src/main.op (not inline string)
      2. Run test and verify it compiles the actual project file
    Expected Result: Test uses real file from test-projects/ directory
    Evidence: .sisyphus/evidence/task-7-reads-project-file.txt
  ```

  **Commit**: YES
  - Message: `test(integration): add hello-world E2E compilation and execution test`
  - Pre-commit: `cargo make lint-fix && cargo make test`

- [x] 8. Create `test-projects/fib-recursive/` and `test-projects/fib-iterative/` Project Structures

  **What to do**:
  - Create `test-projects/fib-recursive/` with: `opal.toml`, `.gitignore`, `README.md`, `src/main.op`
  - Create `test-projects/fib-iterative/` with: `opal.toml`, `.gitignore`, `README.md`, `src/main.op`
  - `fib-recursive/src/main.op` (brace syntax, int64, no args):
    ```
    ## Description: Recursive Fibonacci computation ##
    let fib = f(n: int64): int64 => {
        if n is 0 { return 0 }
        if n is 1 { return 1 }
        return fib(n - 1) + fib(n - 2)
    }

    ## Description: Entry point — prints fib(10) ##
    entry main = f(): void => {
        let result = fib(10)
        print(result)
        return void
    }
    ```
  - `fib-iterative/src/main.op` (brace syntax, int64, no args):
    ```
    ## Description: Entry point — iterative fib(10) ##
    entry main = f(): void => {
        let mutable a: int64 = 0
        let mutable b: int64 = 1
        let mutable i: int64 = 0
        while i < 10 {
            let temp = b
            b = a + b
            a = temp
            i = i + 1
        }
        print(a)
        return void
    }
    ```
  - Both projects should produce `55` as output (fib(10) = 55)
  - Note: `print(result)` prints an int64 directly — need to verify print handles non-string types, or convert to string first. If print only accepts strings, may need `print('{result}')` (interpolation from Task 6)
  - Commit

  **Must NOT do**:
  - Do not use colon-block syntax
  - Do not use int32
  - Do not put args on entry function
  - Do not create tests (Task 10)

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: File creation only
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES (with Task 9)
  - **Parallel Group**: Wave 3
  - **Blocks**: Task 10
  - **Blocked By**: None (can start immediately, but Task 10 needs Wave 1+2 complete)

  **References**:

  **Pattern References**:
  - `test-projects/hello-world/` — Project structure to replicate (from Task 5)
  - `language-spec/fib_recursive.op` — Original recursive fib (reference for logic, NOT syntax)
  - `language-spec/fib_iterative.op` — Original iterative fib (reference for logic, NOT syntax)
  - `src/type_system/test_integration.rs:91-148` — Brace-syntax fib equivalents already tested in type checker

  **Acceptance Criteria**:
  - [ ] Both project directories exist with full structure (opal.toml, .gitignore, README.md, src/main.op)
  - [ ] Both use brace syntax, int64 types, f(): void entry
  - [ ] Both should produce "55" as output when compiled and run
  - [ ] `fib-recursive` uses recursive function calls
  - [ ] `fib-iterative` uses while loop with mutable variables

  **QA Scenarios**:

  ```
  Scenario: Project structures are complete
    Tool: Read tool + Glob tool
    Steps:
      1. Use Glob to find all files under `test-projects/fib-recursive/` and `test-projects/fib-iterative/`
      2. Verify each has opal.toml, .gitignore, README.md, src/main.op
      3. Use Read to read both main.op files and verify brace syntax, int64, f(): void
    Expected Result: Both projects have correct structure and syntax
    Evidence: .sisyphus/evidence/task-8-fib-structure.txt
  ```

  **Commit**: YES
  - Message: `chore: create test-projects/fib-recursive and fib-iterative project structures`
  - Pre-commit: `cargo make lint-fix && cargo make test`

- [x] 9. Verify/Fix `is` Operator Codegen for Equality

  **What to do**:
  - Read all pre-task mandatory files
  - Read `src/ast.rs` — find how `is` operator is represented (likely `BinaryOp::Is` or `BinaryOp::Equal`)
  - Read `src/codegen/expressions.rs` — find `codegen_binary` or `codegen_cmp` and check if `is`/equality is handled
  - Use `ast_grep_search` to find all `BinaryOp::Is` or similar patterns
  - If `is` maps to equality comparison and codegen already handles it → write a verification test and move on
  - If `is` is NOT handled in codegen → implement it as LLVM `icmp eq` (integer compare equal)
  - Write TDD tests FIRST: test that `n is 0` produces `icmp eq` in LLVM IR, test that `n is 1` works
  - Run lint, test, line-count checks
  - Commit

  **Must NOT do**:
  - Do not implement pattern-matching `is` (type narrowing) — just equality comparison
  - Do not break existing codegen tests
  - All guardrails apply

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: Requires tracing AST representation through parser into codegen to verify/fix operator handling
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES (with Task 8)
  - **Parallel Group**: Wave 3
  - **Blocks**: Task 10
  - **Blocked By**: Task 4 (needs working compile pipeline to verify)

  **References**:

  **Pattern References**:
  - `src/ast.rs` — BinaryOp enum (find `Is` or `Equal` variant)
  - `src/codegen/expressions.rs` — `codegen_binary`, `codegen_cmp` functions
  - `src/parser/expressions.rs` — How `is` keyword is parsed into AST

  **API/Type References**:
  - `inkwell::IntPredicate::EQ` — LLVM integer equality comparison
  - `src/type_system/test_integration.rs:93` — `if n is 0 { return 0 }` brace-syntax that type-checks

  **Acceptance Criteria**:
  - [ ] `n is 0` produces `icmp eq` in LLVM IR
  - [ ] `n is 1` produces `icmp eq` in LLVM IR
  - [ ] Existing codegen tests still pass
  - [ ] `cargo make test` passes, `cargo make lint` passes

  **QA Scenarios**:

  ```
  Scenario: is operator produces equality comparison in IR
    Tool: Bash (cargo test)
    Preconditions: is operator verified/fixed in codegen
    Steps:
      1. Run `cargo make test 2>&1 | tee temp.log`
      2. Read temp.log, verify "is_operator_equality" test passes
      3. Verify IR contains "icmp eq" for `n is 0`
    Expected Result: is operator correctly lowers to LLVM icmp eq
    Failure Indicators: Missing match arm, wrong comparison type, panic
    Evidence: .sisyphus/evidence/task-9-is-operator.txt
  ```

  **Commit**: YES
  - Message: `fix(codegen): handle is operator as equality comparison in expression lowering`
  - Pre-commit: `cargo make lint-fix && cargo make test`

- [x] 10. Integration Tests for Both Fibonacci Projects

  **What to do**:
  - Read all pre-task mandatory files
  - Add two tests to `tests/integration_e2e.rs`:
    1. `test_fib_recursive_e2e` — reads `test-projects/fib-recursive/src/main.op`, compiles, runs, asserts stdout == `"55\n"` and exit 0
    2. `test_fib_iterative_e2e` — reads `test-projects/fib-iterative/src/main.op`, compiles, runs, asserts stdout == `"55\n"` and exit 0
  - Both tests follow the same pattern as Task 7 (hello world test)
  - Both tests clean up `target/` directory after completion
  - Consider extracting a shared test helper: `compile_and_run_test_project(project_name: &str, expected_stdout: &str, expected_exit: i32)`
  - If `print(result)` doesn't work for int64 (print may only accept strings), either:
    - Fix print to handle integers (add printf %d prototype)
    - Or change test projects to use string interpolation: `print('{result}')`
  - Run lint, test (with integration), line-count checks
  - Commit

  **Must NOT do**:
  - Do not put test code in test-projects/
  - Do not leave artifacts
  - All guardrails apply

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: E2E tests requiring coordination of compile pipeline + fib computation verification
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: NO (depends on Tasks 7, 8, 9)
  - **Parallel Group**: Wave 3 (last in wave)
  - **Blocks**: Task 15
  - **Blocked By**: Tasks 7, 8, 9

  **References**:

  **Pattern References**:
  - `tests/integration_e2e.rs` — Smoke test (Task 4) and hello-world test (Task 7) patterns
  - `test-projects/fib-recursive/src/main.op` — Source to compile
  - `test-projects/fib-iterative/src/main.op` — Source to compile

  **Acceptance Criteria**:
  - [ ] `cargo test --features integration test_fib_recursive` passes with stdout "55\n"
  - [ ] `cargo test --features integration test_fib_iterative` passes with stdout "55\n"
  - [ ] Both tests clean up target/ after completion
  - [ ] Shared test helper extracted for reusable compile-and-run pattern
  - [ ] `cargo make test` (without integration) still passes

  **QA Scenarios**:

  ```
  Scenario: Recursive fib(10) produces 55
    Tool: Bash (cargo test --features integration)
    Preconditions: Tasks 7-9 complete, fib-recursive project exists
    Steps:
      1. Run `cargo test --features integration test_fib_recursive 2>&1 | tee temp.log`
      2. Read temp.log, verify test passes
      3. Verify stdout assertion is exactly "55\n"
    Expected Result: Recursive fib compiles, runs, prints "55"
    Failure Indicators: Wrong output, compilation failure, stack overflow
    Evidence: .sisyphus/evidence/task-10-fib-recursive-e2e.txt

  Scenario: Iterative fib(10) produces 55
    Tool: Bash (cargo test --features integration)
    Steps:
      1. Run `cargo test --features integration test_fib_iterative 2>&1 | tee temp.log`
      2. Read temp.log, verify test passes
      3. Verify stdout assertion is exactly "55\n"
    Expected Result: Iterative fib compiles, runs, prints "55"
    Failure Indicators: Wrong output, compilation failure, infinite loop
    Evidence: .sisyphus/evidence/task-10-fib-iterative-e2e.txt

  Scenario: Shared test helper works for both projects
    Tool: Bash (cargo test --features integration)
    Steps:
      1. Run both fib tests via `cargo test --features integration fib 2>&1 | tee temp.log`
      2. Verify both pass using shared helper
    Expected Result: Both tests pass, helper is reusable
    Evidence: .sisyphus/evidence/task-10-shared-helper.txt
  ```

  **Commit**: YES
  - Message: `test(integration): add fibonacci recursive and iterative E2E tests`
  - Pre-commit: `cargo make lint-fix && cargo make test`

- [x] 11. Create `test-projects/simple-quiz/` Project Structure

  **What to do**:
  - Create directory: `test-projects/simple-quiz/`
  - Create `test-projects/simple-quiz/opal.toml`:
    ```toml
    name = "simple-quiz"
    version = "1.0.0"
    ```
  - Create `test-projects/simple-quiz/.gitignore`:
    ```
    /target/
    *.o
    ```
  - Create `test-projects/simple-quiz/README.md` with description of the quiz program
  - Create `test-projects/simple-quiz/src/main.op` — a REWRITTEN brace-syntax version that avoids unsupported parser features:
    - **CRITICAL**: The original `simple_quiz.op` uses `let user_input, user_number = loop => { ... break user_input: s, user_number: n }` — the parser does NOT support loop-as-expression or multi-binding `let`. MUST rewrite using `let mutable` + `while` loop instead.
    - MUST use brace syntax `{ }` (no colon-block)
    - MUST use `f(): void` entry function (no args parameter)
    - MUST use `int64` (not `int32`)
    - MUST use `let mutable` (NOT `let mut` or `var` — parser only supports `let mutable`)
    - Uses `import take_input, string_to_int32 from standard` and `import random_int32 from math`
    - Uses `is` operator for equality comparison
    - Uses string interpolation for output
    - **Rewritten control flow**: Instead of loop-as-expression returning values, use:
      ```
      let mutable user_input: string = ''
      let mutable user_number: int64 = 0
      let mutable valid: boolean = false
      while valid is false {
          let s = take_input()
          let n = string_to_int32(s)
          if n is 0 {
              print('Please enter a valid number')
          } else {
              user_input = s
              user_number = n
              valid = true
          }
      }
      ```
    - **NOTE**: `string_to_int32` is a plain C function returning `int64` (0 on parse error). It does NOT return an error type — do NOT use `guard ... into ... else` or `propagate` with it. Use a simple call and check the return value.
    - **CRITICAL**: Since the integration test will mock stdin, design the quiz to be deterministic when given specific inputs. The test will pipe in stdin and assert on stdout patterns.
  - Commit

  **Must NOT do**:
  - Do NOT use loop-as-expression (`let x = loop => { ... }`) — parser doesn't support it
  - Do NOT use multi-binding let (`let a, b = ...`) — parser doesn't support it
  - Do NOT use labeled break payloads (`break name: value`) — codegen doesn't support it
  - Do not use colon-block syntax
  - Do not use `args: string[]` on entry function
  - Do not use `int32`
  - Do not use `let mut` (use `let mutable`)
  - Do not create tests (Task 14)

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: File creation with careful syntax conversion, no Rust code changes
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES (with Tasks 12, 13)
  - **Parallel Group**: Wave 4
  - **Blocks**: Task 14
  - **Blocked By**: None (can start immediately, but Task 14 needs Tasks 10-13 complete)

  **References**:

  **Pattern References**:
  - `language-spec/simple_quiz.op` — Original quiz program (reference for LOGIC ONLY — must completely rewrite control flow to avoid loop-as-expression and multi-binding let)
  - `test-projects/hello-world/` — Project structure to replicate
  - `src/type_system/test_integration.rs:91-148` — Brace-syntax patterns showing `{ }` blocks with `if n is 0 { ... }`
  - `src/parser/statements.rs:57-109` — `parse_let_statement` showing `let mutable` syntax support

  **API/Type References**:
  - `src/ast.rs` — `Decl::Import` structure for import syntax
  - `src/parser/declarations.rs:568-622` — Import parsing to verify supported import syntax in brace mode

  **Acceptance Criteria**:
  - [ ] `test-projects/simple-quiz/` exists with opal.toml, .gitignore, README.md, src/main.op
  - [ ] `src/main.op` uses brace syntax, `f(): void`, `int64`, `let mutable`, imports
  - [ ] `src/main.op` does NOT use loop-as-expression, multi-binding let, labeled break payloads, or `guard`/`propagate` with stdlib calls
  - [ ] Content mirrors `simple_quiz.op` logic: name prompt → random number → guess loop → result
  - [ ] Uses `while` + `let mutable` for the input validation loop (plain call to `string_to_int32`, check return value with `if`)

  **QA Scenarios**:

  ```
  Scenario: Project structure is complete and realistic
    Tool: Read tool + Glob tool
    Steps:
      1. Use Glob to find all files under `test-projects/simple-quiz/`
      2. Verify opal.toml, .gitignore, README.md, src/main.op are present
      3. Use Read to read `test-projects/simple-quiz/src/main.op`
      4. Verify brace syntax throughout (no colon-indentation blocks)
      5. Verify `f(): void` entry function (no args parameter)
      6. Verify `int64` types (no int32)
      7. Verify import statements present
      8. Verify NO loop-as-expression (no `let x = loop =>`)
      9. Verify NO multi-binding let (no `let a, b = ...`)
      10. Verify uses `let mutable` (not `let mut` or `var`)
    Expected Result: All files present, correct syntax throughout, no unsupported features
    Evidence: .sisyphus/evidence/task-11-simple-quiz-structure.txt
  ```

  **Commit**: YES
  - Message: `chore: create test-projects/simple-quiz project structure`
  - Pre-commit: `cargo make lint-fix && cargo make test`

- [x] 12. Import System Codegen

  **What to do**:
  - Read all pre-task mandatory files
  - Read `src/ast.rs` — find `Decl::Import` structure (what fields: module name, imported symbols, aliases)
  - Read `src/codegen/functions.rs` or whichever file handles `Decl` variants — find where imports would be processed
  - Read `src/parser/` — find how `import take_input, string_to_int32 from standard` is parsed
  - The import system for the E2E test projects needs to resolve `standard.take_input`, `standard.string_to_int32`, and `math.random_int32` to their runtime function declarations
  - Implementation strategy: when encountering `Decl::Import`, look up the imported symbol names in a registry of known stdlib/math functions and emit the corresponding LLVM external function declarations
  - This is similar to Task 3 (stdlib prototype registry) but triggered by import declarations rather than call-site resolution
  - The imported names should be registered in the `CodegenEnv` so that subsequent code using `take_input()` resolves to the declared external function
  - Write TDD tests FIRST: test that `import take_input from standard` followed by `take_input()` produces valid IR with correct function declaration and call
  - Run lint, test, line-count checks
  - Commit

  **Must NOT do**:
  - Do not implement a real module/file resolution system (just map known names to runtime functions)
  - Do not implement cross-file compilation (out of scope)
  - Do not break existing codegen tests
  - All guardrails apply

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: Requires understanding AST import structure, codegen dispatch, and stdlib function prototype registry
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES (with Tasks 11, 13, 14)
  - **Parallel Group**: Wave 4
  - **Blocks**: Task 15
  - **Blocked By**: Task 4 (needs working compile pipeline)

  **References**:

  **Pattern References**:
  - `src/ast.rs` — `Decl::Import` AST node structure
  - `src/codegen/functions.rs` — `resolve_callee_function` and stdlib prototype registry (from Task 3)
  - `src/codegen/functions.rs` — `codegen_function_declaration` — how other Decl variants are handled

  **API/Type References**:
  - `src/parser/declarations.rs:568-622` — Import parsing rules (how `Decl::Import` is constructed from source)
  - `src/codegen/context.rs` — `CodegenContext` and any environment/scope tracking for registering imported function bindings
  - `src/codegen/functions.rs:codegen_function_declaration` — Main dispatch point where `Decl` variants are lowered to LLVM IR (add `Decl::Import` handling here or nearby)
  - `src/ast.rs:Decl::Import` — Import node fields: module name, imported symbol list, aliases
  - `src/runtime/stdlib.rs` — Runtime function signatures (reference only — for understanding what prototypes to emit for `string_to_int32`, `random_int32`)
  - `src/runtime/io.rs` — `take_input` runtime signature (reference only — for understanding what prototype to emit)
  - `src/stdlib/io.rs` — stdlib I/O wrappers (reference only — NOT codegen code, just for understanding function contracts)

  **Acceptance Criteria**:
  - [ ] `import take_input from standard` produces `declare` for `take_input` in LLVM IR
  - [ ] `import random_int32 from math` produces `declare` for `random_int32` in LLVM IR
  - [ ] Imported function names are callable in subsequent code
  - [ ] Multiple imports from same module work (`import a, b from mod`)
  - [ ] No existing codegen tests break
  - [ ] `cargo make test` passes, `cargo make lint` passes

  **QA Scenarios**:

  ```
  Scenario: Import declaration emits correct function prototype
    Tool: Bash (cargo test)
    Preconditions: Import codegen handler implemented
    Steps:
      1. Run `cargo make test 2>&1 | tee temp.log`
      2. Read temp.log, verify "import_declaration_emits_prototype" test passes
      3. Verify IR contains `declare` for imported function with correct signature
    Expected Result: Import produces external function declaration in LLVM module
    Failure Indicators: Missing Decl::Import match arm, wrong prototype, name clash
    Evidence: .sisyphus/evidence/task-12-import-codegen.txt

  Scenario: Imported function is callable
    Tool: Bash (cargo test)
    Steps:
      1. Run `cargo make test 2>&1 | tee temp.log`
      2. Verify test for "import X from Y; call X()" produces valid IR with call instruction
    Expected Result: Call to imported function resolves correctly
    Evidence: .sisyphus/evidence/task-12-import-callable.txt
  ```

  **Commit**: YES
  - Message: `feat(codegen): implement import declaration lowering for stdlib functions`
  - Pre-commit: `cargo make lint-fix && cargo make test`

- [x] 13. Wire `take_input`/`random_int32`/`string_to_int32` as libc-backed Builtins + Update Type Checker Signatures

  **What to do**:
  - Read all pre-task mandatory files
  - **CRITICAL — UPDATE TYPE CHECKER BUILTIN SIGNATURES**: The type checker (`src/type_system/checker.rs` lines 207-256) currently registers:
    - `string_to_int32` as `f(string): int32 errors ParseError`
    - `random_int32` as `f(int32, int32): int32`

    These MUST be updated to match the int64 C ABI used by the runtime:
    - `string_to_int32` → `f(string): int64` (NO error types — the C wrapper returns 0 on error, not an error variant)
    - `random_int32` → `f(int64, int64): int64`

    **ALSO update `src/type_system/module_resolver.rs`**: The module resolver independently registers these same builtins for the `standard` and `math` modules (lines ~316-356). These MUST also be updated:
    - `module_resolver.rs:316-327` — `string_to_int32` registered as `f(string): int32 errors ParseError` → change to `f(string): int64` (no error_types)
    - `module_resolver.rs:349-355` — `random_int32` registered as `f(int32, int32): int32` → change to `f(int64, int64): int64`

    Without updating the module resolver, `import string_to_int32 from standard` and `import random_int32 from math` will type-check against the old `int32` signatures, causing type errors when called with `int64` literals.

    **Why**: Integer literals in Opalescent default to `int64`. Calling `random_int32(1, 5)` where `1` and `5` are inferred as `int64` would fail type checking if the signature expects `int32`. The C runtime functions all operate on `int64_t`.

    **After updating the signatures, you MUST fix the existing type checker tests** that assert the old signatures:
    - `src/type_system/tests.rs` — test `test_builtin_string_to_int32_propagate_type_checks` (line ~4612): This test asserts `string_to_int32` works with `propagate` and `ParseError`. Since the new signature has no error types, this test must be rewritten to verify `string_to_int32` type-checks as a plain call returning `int64` (without propagate/guard).
    - `src/type_system/tests.rs` — test `test_builtin_random_int32_signature_type_checks` (line ~4629): This test asserts `random_int32(1, 5)` returns `int32`. Update to assert it returns `int64` with `int64` parameters.
    - `src/type_system/test_integration.rs:460` — Uses `guard string_to_int32('7') into parsed else { ... }`. Since `string_to_int32` no longer has error types, `guard`/`propagate` are invalid with it. Update this test to use a plain call instead.
    - `src/type_system/module_resolver.rs:316, 349` — Module resolver independently registers `string_to_int32` and `random_int32` with old int32 signatures — MUST UPDATE alongside checker.rs (see "CRITICAL — UPDATE TYPE CHECKER BUILTIN SIGNATURES" above)
    - Search for any other tests referencing the old int32 signatures and update them.

    **TDD approach**: Write a failing test first that asserts the NEW signatures (`string_to_int32(s)` returns `int64`, `random_int32(1,5)` accepts `int64` args), then update the signatures to make it pass, then refactor.
  - **CRITICAL CONTEXT**: The existing Rust runtime functions in `src/runtime/io.rs` and `src/runtime/stdlib.rs` use Rust-specific types (`&OpalString`, `&mut impl IoHandler`, `RuntimeAllocator`, `RuntimeResult<>`) and are NOT `extern "C"` — they cannot be linked by `cc` against the LLVM-emitted object file. **Do NOT attempt to link against these Rust functions.**
  - **Strategy: Lower builtins to libc function calls + a small C runtime file.** Create `runtime/opal_runtime.c` containing thin C wrapper functions with `extern` linkage that the LLVM-emitted code calls. These wrappers use libc directly:
    1. `char* opal_take_input()` — calls `fgets(buf, sizeof(buf), stdin)`, strips trailing newline, returns `strdup(buf)` (heap-allocated C string)
    2. `int64_t opal_random_int32(int64_t min, int64_t max)` — calls `rand()` with range `min + (rand() % (max - min + 1))`, seeded via `srand(time(NULL))` on first call (using a static flag)
    3. `int64_t opal_string_to_int32(const char* s)` — calls `strtol(s, &endptr, 10)`, returns parsed value (for simplicity, returns 0 on error; error path can print to stderr)
    4. `void opal_print_string(const char* s)` — calls `puts(s)` (already handled via Task 3, but include here for completeness)
    5. `void opal_print_int(int64_t n)` — calls `printf("%lld\n", n)` for integer printing
  - **Extend Task 2's linker invocation**: Update `compile_program` to pass `runtime/opal_runtime.c` as an extra source to `link_object_file`. The linker command becomes:
    ```
    cc program.o runtime/opal_runtime.c -o program
    ```
    The C compiler compiles the runtime source and links it in one step — **no separate `.o` file is created for the runtime**, avoiding artifacts outside `test-projects/<name>/target/`. The `extra_sources` parameter added to `link_object_file` in Task 2 enables this.
  - **Update stdlib prototype registry** (from Task 3) to emit LLVM `declare` statements matching these C function signatures:
    - `declare i8* @opal_take_input()`
    - `declare i64 @opal_random_int32(i64, i64)`
    - `declare i64 @opal_string_to_int32(i8*)`
    - `declare void @opal_print_string(i8*)`
    - `declare void @opal_print_int(i64)`
  - **Name mapping**: When codegen encounters a call to `take_input()`, emit a call to `@opal_take_input`. When it encounters `print(x)` where `x` is a string, emit `@opal_print_string`; where `x` is an int64, emit `@opal_print_int`.
  - Write TDD tests FIRST:
    - Unit test: `take_input()` call produces IR containing `declare i8* @opal_take_input()` and `call i8* @opal_take_input()`
    - Unit test: `random_int32(1, 5)` produces IR containing `declare i64 @opal_random_int32(i64, i64)` and correct call
    - Unit test: `string_to_int32(s)` produces IR containing `declare i64 @opal_string_to_int32(i8*)` and correct call
    - Integration test: compile a program calling `opal_take_input`, link with runtime.o, verify binary runs
  - Run lint, test, line-count checks
  - Commit

  **Must NOT do**:
  - Do NOT link against the Rust runtime crate (it uses Rust-specific types, not C ABI)
  - Do NOT modify `src/runtime/io.rs` or `src/runtime/stdlib.rs` (they serve the interpreter/future runtime, not the compiler backend)
  - Do not break the print binding from Task 3
  - All guardrails apply

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: Requires creating C runtime file, updating linker invocation, and extending codegen prototype registry with correct C ABI signatures
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES (with Tasks 11, 12)
  - **Parallel Group**: Wave 4
  - **Blocks**: Task 14
  - **Blocked By**: Task 3 (extends stdlib prototype registry)

  **References**:

  **Pattern References**:
  - `src/codegen/functions.rs` — Stdlib prototype registry from Task 3 (extend it with opal_* names)
  - `src/codegen/context.rs` — How function declarations are added to the LLVM module

  **API/Type References** (reference for intended behavior, NOT for linking):
  - `src/type_system/checker.rs:207-256` — Current builtin registrations for `string_to_int32` and `random_int32` — MUST UPDATE signatures from int32→int64 and remove ParseError error type from string_to_int32
  - `src/type_system/tests.rs:4612-4641` — Tests asserting old int32 signatures — MUST UPDATE to match new int64 signatures
  - `src/type_system/test_integration.rs:460` — Uses `guard string_to_int32(...)` — MUST UPDATE to plain call (no guard, since string_to_int32 no longer has error types)
  - `src/type_system/module_resolver.rs:316-327` — Module resolver registers `string_to_int32` for `standard` module with old `int32 errors ParseError` signature — MUST UPDATE to `int64` with no errors
  - `src/type_system/module_resolver.rs:349-355` — Module resolver registers `random_int32` for `math` module with old `(int32, int32): int32` signature — MUST UPDATE to `(int64, int64): int64`
  - `src/runtime/io.rs:59-73` — `take_input` behavior (read line from stdin, return string) — replicate in C
  - `src/runtime/stdlib.rs:38-49` — `string_to_int32` behavior (parse string to int) — replicate in C
  - `src/runtime/stdlib.rs:51-108` — `random_int32` behavior (random int in range) — replicate in C
  - `src/stdlib/io.rs:91-114` — print/println wrappers (reference for expected print behavior)

  **External References**:
  - C stdlib: `fgets`, `strdup`, `strtol`, `rand`, `srand`, `printf`, `puts`
  - LLVM calling conventions: how to declare and call C functions from LLVM IR

  **Acceptance Criteria**:
  - [ ] Type checker: `string_to_int32` registered as `f(string): int64` (no error types)
  - [ ] Type checker: `random_int32` registered as `f(int64, int64): int64`
  - [ ] Module resolver: `string_to_int32` in `standard` module updated to `f(string): int64` (no error types) in `module_resolver.rs`
  - [ ] Module resolver: `random_int32` in `math` module updated to `f(int64, int64): int64` in `module_resolver.rs`
  - [ ] Existing type checker tests updated to match new signatures (no test failures)
  - [ ] `runtime/opal_runtime.c` exists with all 5 wrapper functions (`opal_take_input`, `opal_random_int32`, `opal_string_to_int32`, `opal_print_string`, `opal_print_int`)
  - [ ] `runtime/opal_runtime.c` compiles with `cc -c` without errors
  - [ ] `take_input()` call in Opalescent source produces IR with `declare i8* @opal_take_input()` and `call`
  - [ ] `random_int32(1, 5)` produces IR with `declare i64 @opal_random_int32(i64, i64)` and call with two i64 args
  - [ ] `string_to_int32(s)` produces IR with `declare i64 @opal_string_to_int32(i8*)` and call
  - [ ] `compile_program` updated to pass `runtime/opal_runtime.c` as extra source to `link_object_file` — no `.o` artifacts left outside `test-projects/<name>/target/`
  - [ ] Print binding from Task 3 still works
  - [ ] `cargo make test` passes, `cargo make lint` passes

  **QA Scenarios**:

  ```
  Scenario: C runtime file compiles
    Tool: Bash (cc)
    Preconditions: runtime/opal_runtime.c created
    Steps:
      1. Run `cc -c runtime/opal_runtime.c -o /tmp/opal_runtime_test.o 2>&1 | tee temp.log`
      2. Read temp.log, verify no errors
      3. Run `rm /tmp/opal_runtime_test.o`
    Expected Result: C file compiles cleanly with no warnings
    Failure Indicators: Missing includes, type errors, undefined references
    Evidence: .sisyphus/evidence/task-13-c-runtime-compiles.txt

  Scenario: take_input produces correct IR with C ABI
    Tool: Bash (cargo test)
    Preconditions: opal_take_input binding added to registry
    Steps:
      1. Run `cargo make test 2>&1 | tee temp.log`
      2. Read temp.log, verify "take_input_binding" test passes
      3. Verify IR contains `declare i8* @opal_take_input()`
    Expected Result: take_input() lowers to @opal_take_input() call
    Failure Indicators: Wrong prototype, Rust function referenced instead of C wrapper
    Evidence: .sisyphus/evidence/task-13-take-input.txt

  Scenario: random_int32 accepts two integer arguments
    Tool: Bash (cargo test)
    Steps:
      1. Run `cargo make test 2>&1 | tee temp.log`
      2. Verify "random_int32_binding" test passes
      3. Verify IR contains `declare i64 @opal_random_int32(i64, i64)`
    Expected Result: random_int32(min, max) lowers to @opal_random_int32(i64, i64)
    Evidence: .sisyphus/evidence/task-13-random-int32.txt

  Scenario: string_to_int32 handles string input via C ABI
    Tool: Bash (cargo test)
    Steps:
      1. Run `cargo make test 2>&1 | tee temp.log`
      2. Verify "string_to_int32_binding" test passes
      3. Verify IR contains `declare i64 @opal_string_to_int32(i8*)`
    Expected Result: string_to_int32(s) lowers to @opal_string_to_int32(i8*)
    Evidence: .sisyphus/evidence/task-13-string-to-int32.txt

  Scenario: Linker includes C runtime source
    Tool: Bash (cargo test --features integration)
    Preconditions: Linker updated to compile and link runtime/opal_runtime.c as source
    Steps:
      1. Run a simple integration test that calls take_input or print
      2. Verify binary links successfully (no undefined symbol errors)
    Expected Result: Binary links with opal_runtime.c compiled inline and runs
    Evidence: .sisyphus/evidence/task-13-linker-integration.txt

  Scenario: Type checker signatures updated to int64
    Tool: Bash (cargo test)
    Preconditions: string_to_int32 and random_int32 signatures updated in checker.rs
    Steps:
      1. Run `cargo make test 2>&1 | tee temp.log`
      2. Read temp.log, verify ALL existing type checker tests pass (including updated ones)
      3. Verify test asserting `random_int32(1, 5)` type-checks with int64 literals passes
      4. Verify test asserting `string_to_int32(s)` returns int64 without error types passes
    Expected Result: All type checker tests pass with new int64 signatures
    Failure Indicators: Type mismatch errors for int32 vs int64, guard/propagate errors for string_to_int32
    Evidence: .sisyphus/evidence/task-13-typechecker-signatures.txt
  ```

  **Commit**: YES
  - Message: `feat(codegen): add C runtime wrappers, update builtin signatures to int64, and wire stdlib builtins to libc`
  - Pre-commit: `cargo make lint-fix && cargo make test`

- [x] 14. Integration Test for Simple Quiz

  **What to do**:
  - Read all pre-task mandatory files
  - Add test to `tests/integration_e2e.rs`:
    ```rust
    #[test]
    #[cfg(feature = "integration")]
    fn test_simple_quiz_e2e() {
        // 1. Read test-projects/simple-quiz/src/main.op
        // 2. Call compile_program(source, test-projects/simple-quiz/target/)
        // 3. Run produced binary with stdin piped in
        // 4. Assert on stdout patterns
        // 5. Assert exit code is 0
        // 6. Clean up target/ directory
    }
    ```
  - **Stdin mocking**: Use `Command::new(binary).stdin(Stdio::piped())` and write test input via `child.stdin.write_all(input.as_bytes())`
  - **Determinism challenge**: `random_int32` produces random numbers, so we can't assert exact output. Options:
    1. Mock random at the runtime level (e.g., set a seed via env var)
    2. Assert only on the deterministic parts of output (name prompt, greeting, "Let's see how close you are")
    3. Test multiple runs and verify output matches one of the expected patterns
  - **Recommended approach**: Assert on structural patterns — the program should:
    - Print "What is your name?" first
    - After receiving name input, print a greeting containing the name
    - After receiving a number, print a result message
    - Exit cleanly with code 0
  - Provide stdin like: `"TestUser\n3\n"` (name + guess)
  - Assert stdout contains: "What is your name?", "Hello, TestUser!", and one of ["guessed correctly", "too low", "too high"]
  - The error handling path can be tested with a second test case providing non-numeric input: `"TestUser\nabc\n3\n"` — `string_to_int32("abc")` returns 0, which triggers the `if n is 0` branch to re-prompt
  - Use the shared test helper from Task 10, extended to support stdin piping
  - Run lint, test (with integration), line-count checks
  - Commit

  **Must NOT do**:
  - Do not put test code in test-projects/
  - Do not leave artifacts
  - Do not assert on random number values (non-deterministic)
  - All guardrails apply

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: Complex E2E test requiring stdin mocking, non-deterministic output handling, error path testing
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: NO (depends on Tasks 10, 11, 12, 13)
  - **Parallel Group**: Wave 4 (last in wave)
  - **Blocks**: Tasks 15, 16
  - **Blocked By**: Tasks 10, 11, 12, 13

  **References**:

  **Pattern References**:
  - `tests/integration_e2e.rs` — Existing test patterns from Tasks 4, 7, 10
  - `test-projects/simple-quiz/src/main.op` — Source to compile
  - `language-spec/simple_quiz.op` — Original program logic for expected behavior

  **API/Type References**:
  - `std::process::Command` — stdin piping via `Stdio::piped()`
  - `std::io::Write` — Writing to child stdin

  **Acceptance Criteria**:
  - [ ] `cargo test --features integration test_simple_quiz` passes
  - [ ] Test provides stdin input and captures stdout
  - [ ] stdout contains "What is your name?"
  - [ ] stdout contains greeting with the provided name
  - [ ] stdout contains a result message (correct/too low/too high)
  - [ ] Exit code is 0
  - [ ] Error path test: non-numeric input (`string_to_int32` returns 0) produces error message and re-prompts
  - [ ] No artifacts remain in `test-projects/simple-quiz/target/` after test
  - [ ] `cargo make test` (without integration) still passes

  **QA Scenarios**:

  ```
  Scenario: Quiz runs with valid input
    Tool: Bash (cargo test --features integration)
    Preconditions: Tasks 10-13 complete, simple-quiz project exists
    Steps:
      1. Run `cargo test --features integration test_simple_quiz 2>&1 | tee temp.log`
      2. Read temp.log, verify test passes
      3. Verify stdout assertions match expected patterns
    Expected Result: Quiz compiles, accepts input, prints greeting and result, exits 0
    Failure Indicators: Compilation failure, stdin not piped, missing output patterns, non-zero exit
    Evidence: .sisyphus/evidence/task-14-simple-quiz-e2e.txt

  Scenario: Quiz handles invalid input gracefully
    Tool: Bash (cargo test --features integration)
    Steps:
      1. Run `cargo test --features integration test_simple_quiz_invalid 2>&1 | tee temp.log`
      2. Verify test passes — non-numeric input triggers `if n is 0` error branch
      3. Verify stdout contains error message about parse failure
    Expected Result: Error printed, loop continues, valid input accepted on retry
    Evidence: .sisyphus/evidence/task-14-simple-quiz-error.txt

  Scenario: Cleanup after quiz test
    Tool: Glob tool
    Steps:
      1. Run quiz tests via `cargo test --features integration test_simple_quiz 2>&1 | tee temp.log`
      2. Use Glob to search `test-projects/simple-quiz/target/**/*` — should return no files
    Expected Result: No artifacts remain
    Evidence: .sisyphus/evidence/task-14-cleanup.txt
  ```

  **Commit**: YES
  - Message: `test(integration): add simple-quiz E2E test with stdin mocking`
  - Pre-commit: `cargo make lint-fix && cargo make test`

- [ ] 15. Sync PLAN.md + Fix Completion Plan False Completions

  **What to do**:
  - Read all pre-task mandatory files
  - Read `PLAN.md` — understand its current structure and update it to reflect the work done in this plan:
    - Add section documenting the E2E test projects
    - Add section documenting the compiler pipeline (compile_program, object emission, linker)
    - Add section documenting codegen features added (string interpolation, import system, labeled break payloads, stdlib bindings)
    - Mark relevant milestones as complete
  - Read `.sisyphus/plans/opalescent-completion.md` — find tasks marked as complete that are actually incomplete or were scaffolding
  - **Specifically uncheck these categories** (verified during research as scaffolding, not complete):
    - Any codegen tasks marked complete that relied on the `i64 fn()` fallback (these were never truly working)
    - Any integration test tasks marked complete before this plan existed
    - Any tasks referencing end-to-end compilation that didn't exist before this plan
  - Be conservative: only uncheck items you can verify are NOT actually complete. Items that ARE genuinely complete should remain checked.
  - Use `lsp_find_references` or `grep` to verify status of each questionable item before unchecking
  - Commit

  **Must NOT do**:
  - Do not uncheck items that are genuinely complete
  - Do not modify PLAN.md structure in ways that conflict with existing content
  - Do not modify Makefile.toml, scripts, or lint rules
  - All guardrails apply

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Documentation updates only, no code changes
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES (with Task 16)
  - **Parallel Group**: Wave 5
  - **Blocks**: F1-F4
  - **Blocked By**: Task 14

  **References**:

  **Pattern References**:
  - `PLAN.md` — Current project plan (THE FILE TO UPDATE)
  - `.sisyphus/plans/opalescent-completion.md` — Completion plan with false completions
  - `.sisyphus/plans/end-to-end-test-projects.md` — This plan (source of truth for what was implemented)

  **Acceptance Criteria**:
  - [ ] PLAN.md reflects all work done (E2E test projects, compiler pipeline, codegen features)
  - [ ] `.sisyphus/plans/opalescent-completion.md` has false completions unchecked
  - [ ] Only genuinely incomplete items were unchecked (conservative approach)
  - [ ] `cargo make lint` passes (no content changes to Rust files)

  **QA Scenarios**:

  ```
  Scenario: PLAN.md accurately reflects implementation
    Tool: Read tool
    Steps:
      1. Use Read to read PLAN.md
      2. Verify it mentions: compile_to_module, compile_program, object file emission, linker, E2E test projects
      3. Verify it mentions: string interpolation, import system, stdlib bindings
    Expected Result: PLAN.md is up-to-date with all implemented features
    Evidence: .sisyphus/evidence/task-15-plan-sync.txt

  Scenario: Completion plan has false completions unchecked
    Tool: Read tool + Grep tool
    Steps:
      1. Use Read to read `.sisyphus/plans/opalescent-completion.md`
      2. Use Grep to count `- [x]` and `- [ ]` patterns
      3. Verify previously-false completions are now unchecked
    Expected Result: Only genuinely complete items remain checked
    Evidence: .sisyphus/evidence/task-15-completion-fixup.txt
  ```

  **Commit**: YES
  - Message: `docs: sync PLAN.md and fix completion plan false completions`
  - Pre-commit: `cargo make lint-fix && cargo make test`

- [ ] 16. README.md — Document Compiler Escape Hatches and Test Project Conventions

  **What to do**:
  - Read all pre-task mandatory files
  - Read `README.md` — understand current structure
  - Add a new section documenting:
    1. **Test Projects**: What they are, where they live, how to run them
    2. **Compiler Escape Hatches**: Any escape hatches added during development (e.g., if file output was used for testing, if any compile flags were added, if any runtime stubs were created)
    3. **Integration Test Feature Flag**: How to run integration tests (`cargo test --features integration`)
    4. **Test Project Conventions**: brace syntax requirement, int64 types, f(): void entry, project structure (opal.toml, .gitignore, README.md, src/main.op)
    5. **Compiler Pipeline**: Brief overview of compile_program → object emission → linking
  - If no escape hatches were needed, document that explicitly: "No escape hatches were required"
  - Keep documentation concise — match the existing README style
  - Commit

  **Must NOT do**:
  - Do not rewrite existing README sections
  - Do not add excessive documentation (match existing style)
  - All guardrails apply

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Documentation only
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES (with Task 15)
  - **Parallel Group**: Wave 5
  - **Blocks**: F1-F4
  - **Blocked By**: Task 14

  **References**:

  **Pattern References**:
  - `README.md` — Existing documentation style to match
  - `test-projects/` — The test projects to document
  - `tests/integration_e2e.rs` — Integration test file to reference

  **Acceptance Criteria**:
  - [ ] README.md has new section for test projects and compiler pipeline
  - [ ] Integration test instructions included (`cargo test --features integration`)
  - [ ] Escape hatch documentation present (even if "none needed")
  - [ ] Documentation style matches existing README
  - [ ] `cargo make lint` passes

  **QA Scenarios**:

  ```
  Scenario: README has test project documentation
    Tool: Read tool
    Steps:
      1. Use Read to read README.md
      2. Verify section about test projects exists
      3. Verify section about integration tests exists
      4. Verify escape hatch documentation exists
    Expected Result: All required documentation sections present and well-formatted
    Evidence: .sisyphus/evidence/task-16-readme.txt
  ```

  **Commit**: YES
  - Message: `docs(readme): document compiler escape hatches and test project conventions`
  - Pre-commit: `cargo make lint-fix && cargo make test`

---

## Final Verification Wave (MANDATORY — after ALL implementation tasks)

> 4 review agents run in PARALLEL. ALL must APPROVE. Present consolidated results to user and get explicit "okay" before completing.
>
> **Do NOT auto-proceed after verification. Wait for user's explicit approval before marking work complete.**

- [ ] F1. **Plan Compliance Audit** — `oracle`
  Read this plan end-to-end. For each "Must Have": verify implementation exists (read file, run command). For each "Must NOT Have": search codebase for forbidden patterns — reject with file:line if found. Check evidence files exist in `.sisyphus/evidence/`. Compare deliverables against plan.
  Output: `Must Have [N/N] | Must NOT Have [N/N] | Tasks [N/N] | VERDICT: APPROVE/REJECT`

- [ ] F2. **Code Quality Review** — `unspecified-high`
  Run `cargo make lint 2>&1 | tee temp.log` + `cargo make test 2>&1 | tee temp.log` + `cargo test --features integration 2>&1 | tee temp.log`. Review all changed files for: `as any`/`@ts-ignore` (N/A for Rust), `unwrap()`, `expect()`, `panic!()`, empty catches, `todo!()`, unused imports, files >500 lines. Check AI slop: excessive comments, over-abstraction, generic names.
  Output: `Build [PASS/FAIL] | Lint [PASS/FAIL] | Tests [N pass/N fail] | Integration [N pass/N fail] | VERDICT`

- [ ] F3. **Real Manual QA** — `unspecified-high`
  Start from clean state. For each test project: compile it using the integration test pathway, run the binary, verify stdout matches expected output exactly. Test error cases: missing file, syntax error in .op file, compile failure produces useful error. Verify cleanup: no `.o` files or binaries remain after tests.
  Output: `Projects [N/N pass] | Error cases [N/N] | Cleanup [CLEAN/DIRTY] | VERDICT`

- [ ] F4. **Scope Fidelity Check** — `deep`
  For each task: read "What to do", read actual diff. Verify 1:1 — everything in spec was built, nothing beyond spec was built. Check "Must NOT do" compliance. Detect cross-task contamination. Flag unaccounted changes.
  Output: `Tasks [N/N compliant] | Contamination [CLEAN/N issues] | Unaccounted [CLEAN/N files] | VERDICT`

---

## Commit Strategy

| Task | Commit Message | Key Files |
|------|---------------|-----------|
| 1 | `feat(compiler): create lib.rs crate structure and add compile_to_module orchestration` | `src/lib.rs`, `src/main.rs`, `src/compiler.rs` or `src/codegen/driver.rs` |
| 2 | `feat(codegen): add object file emission, linker invocation, and compile_program end-to-end` | `src/codegen/context.rs`, `src/compiler/linker.rs` |
| 3 | `fix(codegen): emit correct stdlib prototypes instead of i64 fallback` | `src/codegen/functions.rs` |
| 4 | `test(integration): add smoke test for void program compilation` | `tests/integration_*.rs` |
| 5 | `chore: create test-projects/hello-world project structure` | `test-projects/hello-world/*` |
| 6 | `feat(codegen): implement string interpolation expression lowering` | `src/codegen/expressions.rs` |
| 7 | `test(integration): add hello-world E2E test` | `tests/integration_*.rs` |
| 8 | `chore: create test-projects/fib-recursive and fib-iterative project structures` | `test-projects/fib-*/*` |
| 9 | `fix(codegen): handle is operator as equality comparison` | `src/codegen/expressions.rs` |
| 10 | `test(integration): add fibonacci E2E tests` | `tests/integration_*.rs` |
| 11 | `chore: create test-projects/simple-quiz project structure` | `test-projects/simple-quiz/*` |
| 12 | `feat(codegen): implement import system lowering` | `src/codegen/functions.rs`, new module |
| 13 | `feat(codegen): add C runtime wrappers and wire stdlib builtins to libc` | `runtime/opal_runtime.c`, `src/codegen/functions.rs` |
| 14 | `test(integration): add simple-quiz E2E test with stdin mock` | `tests/integration_*.rs` |
| 15 | `docs: sync PLAN.md and fix completion plan false completions` | `PLAN.md`, `.sisyphus/plans/opalescent-completion.md` |
| 16 | `docs(readme): document compiler escape hatches and test project conventions` | `README.md` |

---

## Success Criteria

### Verification Commands
```bash
cargo make test 2>&1 | tee temp.log          # Expected: all existing tests pass
cargo make lint 2>&1 | tee temp.log           # Expected: zero warnings
cargo test --features integration 2>&1 | tee temp.log  # Expected: 4+ integration tests pass
scripts/check-line-count.sh                    # Expected: all files compliant
ls test-projects/*/target/                     # Expected: empty or nonexistent after tests
```

### Final Checklist
- [ ] All "Must Have" present
- [ ] All "Must NOT Have" absent
- [ ] All existing tests pass (no regressions)
- [ ] All 4 test projects compile, link, run, produce correct output
- [ ] PLAN.md reflects completed work
- [ ] No test artifacts remain after cleanup
