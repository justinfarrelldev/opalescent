# Rust-Style Error Messages for the Opalescent Compiler

## TL;DR

> **Quick Summary**: Overhaul the Opalescent compiler's error output from barebones `eprintln!` messages into beautiful, Rust/cargo-style diagnostics using the already-present miette library — with source code context, colored annotations pointing at exact characters, actionable suggestions with example fixes, multi-error display, and a summary footer.
>
> **Deliverables**:
> - Fixed span conversion consistency across all error types
> - New `src/errors/renderer.rs` module using miette's `GraphicalReportHandler` + `NamedSource`
> - Refactored `compile_to_module` that collects ALL errors (not just the first)
> - Updated CLI (`app.rs`) using the new renderer everywhere
> - Enhanced `CodegenError` with optional span info
> - Context-aware suggestions for the ~12 most common error types
> - Updated tests for new rendering output
>
> **Estimated Effort**: Medium
> **Parallel Execution**: YES - 3 waves
> **Critical Path**: Task 1 → Task 3 → Task 5 → Task 7 → Task 8 → Task 9 → F1-F4

---

## Context

### Original Request
"Right now, the compiler errors and warnings are very barebones and do not give any real feedback when there is an issue. Same with the typechecker and just about everything else. Please add extensive, Rust-style error messages when there are issues. Be extremely helpful, and give example solutions to common problems similarly to how cargo does it. Point out exact lines and characters where there are issues, and what is happening with those parts, and make it look beautiful and modern on the CLI."

### Interview Summary
**Key Discussions**:
- miette 7.0 with `fancy` feature is already a dependency but never used for rendering
- Error enums already derive `miette::Diagnostic` with codes, help, labels, and spans
- The CLI uses bare `eprintln!("error: compilation failed: {error}")` everywhere
- Only the first error is shown; `lex_errors.errors.into_iter().next()` discards the rest
- `CompilationErrorReport` exists for multi-error collection but is only used by the LSP
- `errors/formatter.rs` produces plain text like `error[code]\n  phase: ...\n  x message\n  help: ...`
- `CodegenError` is just `{ message: String }` — no spans, no miette integration
- `errors/suggestions.rs` has Levenshtein distance but only covers 2 error types

**Research Findings**:
- **Span inconsistency bug**: `LexError::span_from_span` adds +1 to length (inclusive end), but `TypeError::span_from_span` and `Span::len()` do not (exclusive end). This will cause off-by-one underline errors.
- **Tab normalization**: `compile_to_module` normalizes tabs to 4 spaces before lexing. All offsets reference normalized source. Must use normalized source for `NamedSource`.
- **LSP pattern**: `src/lsp/diagnostics.rs:get_diagnostics` already shows the correct multi-error collection pattern using `CompilationErrorReport`.
- **Existing tests**: `src/errors/tests.rs` has 15 tests asserting exact plain-text output from the old formatter. These will need updating.

### Metis Review
**Identified Gaps** (addressed):
- Span conversion inconsistency must be fixed FIRST (Task 1)
- Must NOT add `#[source_code]` to error enums (breaks `no_std`, Clone, Eq derives) — attach at render time instead
- Must NOT change `#[diagnostic]`, `#[label]` attributes on error types — LSP depends on them
- `unknown_span()` sentinel (offset 0, len 0) needs graceful handling
- Multi-span errors (`TypeMismatch` has `found_span` + `expected_span`) need both rendered
- Suggestions should be limited to ~10-12 most common variants, not all 55
- CodegenError span threading should be limited to function-level (not full expression-level)

---

## Work Objectives

### Core Objective
Transform Opalescent's error output into beautiful, Rust/cargo-style diagnostics that show source context, point at exact locations, and provide actionable suggestions — using the miette library that's already a project dependency.

### Concrete Deliverables
- `src/errors/renderer.rs` — New miette-based rendering module
- `src/error.rs` — Fixed `LexError::span_from_span` (span consistency)
- `src/compiler.rs` — Refactored to collect all errors using `CompilationErrorReport`
- `src/app.rs` — All CLI commands using new renderer
- `src/codegen/expressions.rs` — `CodegenError` with optional span
- `src/errors/formatter.rs` — Updated to use miette rendering (or deprecated)
- `src/errors/suggestions.rs` — Enhanced with suggestions for ~12 error types
- `src/errors/tests.rs` — Updated test assertions
- `test-projects/error-display/` — Test project with deliberate errors for QA verification

### Definition of Done
- [ ] `cargo build` compiles without errors or new warnings
- [ ] `cargo test` passes (all existing + new tests)
- [ ] `cargo run -- check test-projects/error-display/src/main.op 2>&1` shows: source lines, underline annotations, error codes, help text, line numbers, filename, and error count summary
- [ ] Multiple errors displayed (not just the first)
- [ ] Colored output visible in terminal (red for errors, cyan for help)

### Must Have
- Source code context with line numbers in error output
- Underline annotations pointing at exact byte ranges
- Error codes (e.g., `opalescent::parser::unexpected_token`)
- Help text with actionable suggestions
- Multiple error display (not just first error)
- Summary footer (e.g., "error: aborting due to 3 previous errors")
- Correct span highlighting (no off-by-one from the span inconsistency)

### Must NOT Have (Guardrails)
- Do NOT add `#[source_code]` fields to `LexError`, `ParseError`, or `TypeError` enums — attach source at render time only
- Do NOT modify `#[diagnostic]`, `#[label]`, or `#[help]` attributes on existing error enum variants — LSP depends on the `Diagnostic` trait
- Do NOT change LSP diagnostic behavior in `src/lsp/diagnostics.rs` — only update for type/signature compatibility if `CompilerError` variants change (e.g., `Codegen(String)` → `Codegen(CodegenError)`)
- Do NOT add suggestions to all 55 error variants — limit to ~12 most common
- Do NOT thread spans through all codegen expression functions — limit to function-level where `Decl` already carries `span`
- Do NOT add new error detection logic or new error types (beyond what's needed for suggestions)
- Do NOT add excessive comments, JSDoc-style blocks, or over-abstract the rendering code

---

## Verification Strategy

> **ZERO HUMAN INTERVENTION** - ALL verification is agent-executed. No exceptions.

### Test Decision
- **Infrastructure exists**: YES (cargo test, existing `src/errors/tests.rs` with 15 tests, `src/compiler.rs` tests)
- **Automated tests**: Tests-after (update existing tests + add new rendering tests)
- **Framework**: cargo test (Rust built-in)

### QA Policy
Every task MUST include agent-executed QA scenarios.
Evidence saved to `.sisyphus/evidence/task-{N}-{scenario-slug}.{ext}`.

- **Compiler output**: Use Bash (`cargo run -- check <file> 2>&1`) — compile known-bad `.op` files, capture stderr, assert content
- **Unit tests**: Use Bash (`cargo test`) — run test suite, assert pass counts
- **Build verification**: Use Bash (`cargo build 2>&1`) — verify clean build

---

## Execution Strategy

### Parallel Execution Waves

```
Wave 1 (Foundation — independent fixes, can all start immediately):
├── Task 1: Fix span conversion inconsistency [quick]
├── Task 2: Create error display test project [quick]
├── Task 3: Create miette-based renderer module [deep]
├── Task 4: Enhance CodegenError with optional span [quick]

Wave 2 (Core wiring — depends on Wave 1):
├── Task 5: Refactor compile_to_module for multi-error collection (depends: 1, 3) [deep]
├── Task 6: Enhance suggestions for common error types (depends: 1) [unspecified-high]

Wave 3 (Integration — depends on Wave 2):
├── Task 7: Update CLI (app.rs) to use new renderer (depends: 3, 5) [unspecified-high]
├── Task 8: Update existing tests for new rendering (depends: 3, 5, 6, 7) [unspecified-high]
├── Task 9: End-to-end integration verification (depends: 2, 7, 8) [deep]

Wave FINAL (After ALL tasks — 4 parallel reviews, then user okay):
├── Task F1: Plan compliance audit (oracle)
├── Task F2: Code quality review (unspecified-high)
├── Task F3: Real manual QA (unspecified-high)
└── Task F4: Scope fidelity check (deep)
-> Present results -> Get explicit user okay
```

### Dependency Matrix

| Task | Depends On | Blocks | Wave |
|------|-----------|--------|------|
| 1 | — | 3, 5, 6 | 1 |
| 2 | — | 9 | 1 |
| 3 | 1 | 5, 7, 8 | 1 |
| 4 | — | 5 | 1 |
| 5 | 1, 3, 4 | 7, 8 | 2 |
| 6 | 1 | 8 | 2 |
| 7 | 3, 5 | 8, 9 | 3 |
| 8 | 3, 5, 6, 7 | 9 | 3 |
| 9 | 2, 7, 8 | F1-F4 | 3 |

> Critical Path: Task 1 → Task 3 → Task 5 → Task 7 → Task 8 → Task 9 → F1-F4 → user okay
> Parallel Speedup: ~50% faster than sequential
> Max Concurrent: 4 (Wave 1)

### Agent Dispatch Summary

- **Wave 1**: **4 tasks** — T1 → `quick`, T2 → `quick`, T3 → `deep`, T4 → `quick`
- **Wave 2**: **2 tasks** — T5 → `deep`, T6 → `unspecified-high`
- **Wave 3**: **3 tasks** — T7 → `unspecified-high`, T8 → `unspecified-high`, T9 → `deep`
- **FINAL**: **4 tasks** — F1 → `oracle`, F2 → `unspecified-high`, F3 → `unspecified-high`, F4 → `deep`

---

## TODOs

- [x] 1. Fix span conversion inconsistency

  **What to do**:
  - In `src/error.rs`, fix `LexError::span_from_span()` (lines 157-166) to use **exclusive end** (matching `Span::len()` and `TypeError::span_from_span()`):
    ```rust
    pub fn span_from_span(span: Span) -> SourceSpan {
        let start = span.start.offset;
        let len = span.end.offset.saturating_sub(span.start.offset);
        SourceSpan::new(start.into(), len)
    }
    ```
  - The current code adds `.saturating_add(1)` to the length, making lexer/parser spans 1 byte longer than typechecker spans for the same source range. Remove the `+1`.
  - Verify `Span::single(pos)` still works correctly (it creates `Span::new(pos, pos)` → length 0, which is correct for a zero-width cursor position).
  - Run `cargo test` to catch any tests that relied on the old +1 behavior and fix them.

  **Must NOT do**:
  - Do NOT change `LexError::span_from_position()` — it takes an explicit `len` parameter and is correct.
  - Do NOT change `TypeError::span_from_span()` — it's already correct.
  - Do NOT modify any `#[label]` or `#[diagnostic]` attributes on error variants.

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Single file, surgical 3-line fix with straightforward test verification.
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1 (with Tasks 2, 3, 4)
  - **Blocks**: Tasks 3, 5, 6
  - **Blocked By**: None (can start immediately)

  **References**:

  **Pattern References**:
  - `src/error.rs:157-166` — Current `LexError::span_from_span()` with the +1 bug to fix
  - `src/type_system/errors.rs:801-805` — `TypeError::span_from_span()` — the CORRECT pattern to match
  - `src/token.rs:63-69` — `Span::len()` — canonical length calculation (exclusive end, no +1)

  **Test References**:
  - `src/errors/tests.rs:228-248` — `test_type_error_symbol_not_found_supports_suggestion_field` — uses `Span::single()` with `span_from_span`; verify this still works
  - `src/compiler.rs:370-457` — Compiler module tests — must still pass

  **Acceptance Criteria**:

  - [ ] `LexError::span_from_span()` no longer adds +1 to span length
  - [ ] `cargo test` passes with no failures

  **QA Scenarios (MANDATORY)**:

  ```
  Scenario: Span conversion produces correct length
    Tool: Bash (cargo test)
    Preconditions: Code change applied to src/error.rs
    Steps:
      1. Run `cargo test -- test_type_error_symbol_not_found_supports_suggestion_field 2>&1`
      2. Run `cargo test -- test_lex_errors_can_be_promoted_to_report 2>&1`
      3. Run `cargo test 2>&1` (full suite)
    Expected Result: All tests pass; 0 failures
    Failure Indicators: Any test failure mentioning span or source_span
    Evidence: .sisyphus/evidence/task-1-span-fix-tests.txt

  Scenario: Span length matches TypeError convention
    Tool: Bash (grep + manual verification)
    Preconditions: Code change applied
    Steps:
      1. Run `grep -n 'saturating_add(1)' src/error.rs` — should return NO matches
      2. Run `grep -n 'saturating_sub' src/error.rs` — should show the subtraction without +1
    Expected Result: No `saturating_add(1)` in `span_from_span`; only `saturating_sub` remains
    Failure Indicators: `saturating_add(1)` still present in `span_from_span`
    Evidence: .sisyphus/evidence/task-1-span-grep.txt
  ```

  **Commit**: YES
  - Message: `fix(errors): unify span conversion to use exclusive end offset`
  - Files: `src/error.rs`
  - Pre-commit: `cargo test`

---

- [x] 2. Create error display test project

  **What to do**:
  - Create `test-projects/error-display/` with the standard Opalescent test project structure:
    - `opal.toml` with `name = "error-display"` and `version = "0.1.0"`
    - `src/main.op` containing deliberate errors across multiple compiler phases:
      - A lexer error (e.g., `@` unexpected character)
      - A parser error (e.g., missing closing paren or unexpected token)
      - A type error (e.g., `let x: int32 = 'hello'` type mismatch)
      - A typo that should trigger "did you mean" (e.g., `pritn('hello')` instead of `print`)
    - `.gitignore` with `target/`
  - This test project is used by QA scenarios in later tasks and by the final verification wave.
  - Create additional single-error test files for focused testing:
    - `test-projects/error-display/src/lex_error.op` — only a lex error
    - `test-projects/error-display/src/type_error.op` — only a type error
    - `test-projects/error-display/src/parse_error.op` — only a parse error

  **Must NOT do**:
  - Do NOT create any Rust code — this is Opalescent `.op` source only
  - Do NOT try to make the files compilable — they must contain errors intentionally

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: File creation only, no Rust code, straightforward content.
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1 (with Tasks 1, 3, 4)
  - **Blocks**: Task 9
  - **Blocked By**: None (can start immediately)

  **References**:

  **Pattern References**:
  - `test-projects/hello-world/` — Standard test project structure to follow
  - `test-projects/hello-world/opal.toml` — Project config format
  - `test-projects/hello-world/src/main.op` — Entry function pattern

  **External References**:
  - README.md "Language Basics" section — Opalescent syntax reference for creating valid-looking code with intentional errors

  **Acceptance Criteria**:

  - [ ] `test-projects/error-display/opal.toml` exists with correct format
  - [ ] `test-projects/error-display/src/main.op` exists with errors from multiple phases
  - [ ] Each focused error file exists and contains exactly one type of error
  - [ ] File content uses valid Opalescent syntax structure (aside from the intentional errors)

  **QA Scenarios (MANDATORY)**:

  ```
  Scenario: Test project structure is valid
    Tool: Bash (ls, cat)
    Preconditions: Files created
    Steps:
      1. Run `ls test-projects/error-display/src/` — should list main.op and focused error files
      2. Run `cat test-projects/error-display/opal.toml` — should contain name and version fields
      3. Run `cat test-projects/error-display/.gitignore` — should contain target/
    Expected Result: All files present with expected content
    Failure Indicators: Missing files or empty content
    Evidence: .sisyphus/evidence/task-2-test-project-structure.txt

  Scenario: Error files actually trigger errors
    Tool: Bash (cargo run)
    Preconditions: Compiler builds successfully
    Steps:
      1. Run `cargo run -- check test-projects/error-display/src/main.op 2>&1` — should exit with non-zero
      2. Run `echo $?` — should be non-zero
    Expected Result: Exit code 1 (compilation errors detected)
    Failure Indicators: Exit code 0 (no errors found — the test files aren't triggering errors)
    Evidence: .sisyphus/evidence/task-2-error-files-trigger.txt
  ```

  **Commit**: YES
  - Message: `test(errors): add error-display test project with deliberate errors`
  - Files: `test-projects/error-display/**`
  - Pre-commit: —

---

- [x] 3. Create miette-based diagnostic renderer module

  **What to do**:
  - Create `src/errors/renderer.rs` — the core rendering module that replaces the plain-text formatter with miette's `GraphicalReportHandler`:
    - Create a `DiagnosticWithSource<E>` wrapper struct that wraps any `miette::Diagnostic` with a `NamedSource<String>`:
      ```rust
      struct DiagnosticWithSource {
          #[source_code]
          source_code: NamedSource<String>,
          // Delegate Diagnostic trait to inner error
      }
      ```
    - Implement `miette::Diagnostic` for the wrapper by delegating `code()`, `help()`, `labels()`, `severity()`, `url()` to the inner error's trait methods.
    - Create a public `render_diagnostic(filename: &str, source: &str, error: &dyn miette::Diagnostic) -> String` function that:
      1. Creates `NamedSource::new(filename, source.to_owned())`
      2. Wraps the error with the source
      3. Uses `GraphicalReportHandler::new_themed(GraphicalTheme::unicode())` to render to a String
    - Create a public `render_report(filename: &str, source: &str, report: &CompilationErrorReport) -> String` function that:
      1. Renders each error in the report using `render_diagnostic`
      2. Appends a summary footer: `"error: aborting due to {N} previous error(s)"`
      3. Handles warnings separately with appropriate coloring
    - Handle the `unknown_span()` sentinel (offset 0, len 0) gracefully — either render without source context or render with a note that location is unknown
    - Handle `CompilerError::Codegen(String)` variant by rendering as a plain message without source context (since codegen errors have no spans yet — Task 4 adds optional spans)
  - Register the module in `src/errors.rs` with `pub mod renderer;`

  **Must NOT do**:
  - Do NOT add `#[source_code]` fields to `LexError`, `ParseError`, or `TypeError` — source is attached at render time only
  - Do NOT modify existing `#[diagnostic]`, `#[label]`, or `#[help]` attributes on error variants
  - Do NOT delete `formatter.rs` yet — keep it functional during transition
  - Do NOT change LSP diagnostic behavior in `src/lsp/diagnostics.rs` — only update for type/signature compatibility (e.g., `Codegen(String)` → `Codegen(CodegenError)`)

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: Core architectural component. Requires understanding miette's Diagnostic trait delegation, wrapper pattern, and rendering API. Must handle edge cases (unknown spans, codegen errors without source, multi-span errors).
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES (with Tasks 2, 4; depends on Task 1 completing first)
  - **Parallel Group**: Wave 1 (but wait for Task 1 to finish before starting)
  - **Blocks**: Tasks 5, 7, 8
  - **Blocked By**: Task 1

  **References**:

  **Pattern References**:
  - `src/errors/formatter.rs:37-117` — Current `format_diagnostic()` function to understand what information is extracted from each `CompilerError` variant
  - `src/errors/reporter.rs:1-108` — `CompilationErrorReport` struct and `CompilerError` enum — the input this renderer will consume
  - `src/lsp/diagnostics.rs:14-37` — `get_diagnostics()` — shows the correct pattern for iterating over `report.entries()` and processing each `CompilerError` variant

  **API/Type References**:
  - `src/errors/reporter.rs:13-22` — `CompilerError` enum (Lexer, Parser, TypeChecker, Codegen variants)
  - `src/errors/reporter.rs:26-29` — `CompilationErrorReport` struct with `entries()` returning `&[(CompilerPhase, CompilerError)]`
  - `src/errors/formatter.rs:10-20` — `CompilerPhase` enum (Lexer, Parser, TypeChecker, Codegen)

  **External References**:
  - miette docs: `GraphicalReportHandler`, `NamedSource`, `Diagnostic` trait — Use Context7 or `https://docs.rs/miette/7.0.0/miette/`
  - miette `GraphicalTheme::unicode()` — for pretty Unicode box-drawing characters in output

  **WHY Each Reference Matters**:
  - `formatter.rs` shows the current rendering logic that needs to be replaced — understand what fields are extracted per variant
  - `reporter.rs` defines the exact input type (`CompilationErrorReport`) the renderer consumes
  - `lsp/diagnostics.rs` shows how to iterate the report and access trait methods — proven working pattern
  - miette docs explain `GraphicalReportHandler` API for rendering to String and `NamedSource` for attaching source text

  **Acceptance Criteria**:

  - [ ] `src/errors/renderer.rs` exists and compiles
  - [ ] `render_diagnostic()` produces output containing source code lines and underline annotations
  - [ ] `render_report()` produces output with all errors and summary footer
  - [ ] `src/errors.rs` declares `pub mod renderer;`
  - [ ] `cargo build` succeeds with no new warnings

  **QA Scenarios (MANDATORY)**:

  ```
  Scenario: Renderer produces miette-style output for a lex error
    Tool: Bash (cargo test)
    Preconditions: renderer.rs created and registered
    Steps:
      1. Write a unit test in renderer.rs that creates a LexError::UnexpectedCharacter with a known span
      2. Call render_diagnostic("test.op", "let x = @;", &error)
      3. Assert the output contains:
         - The source line "let x = @;"
         - An underline character (─ or ^) near column 9
         - The error code "opalescent::lexer::unexpected_character"
         - Help text
      4. Run `cargo test -- renderer 2>&1`
    Expected Result: Test passes, output contains source context and annotations
    Failure Indicators: Output is plain text without source lines, or test panics
    Evidence: .sisyphus/evidence/task-3-renderer-lex-error.txt

  Scenario: Renderer handles unknown_span gracefully
    Tool: Bash (cargo test)
    Preconditions: renderer.rs created
    Steps:
      1. Write a unit test that creates a TypeError with `unknown_span()` (offset 0, len 0)
      2. Call render_diagnostic("test.op", "let x = 1", &error)
      3. Assert the output does NOT panic and contains the error message
    Expected Result: Renders without panic, shows error message even without source highlight
    Failure Indicators: Panic, crash, or empty output
    Evidence: .sisyphus/evidence/task-3-renderer-unknown-span.txt

  Scenario: Report renderer shows multiple errors with summary
    Tool: Bash (cargo test)
    Preconditions: renderer.rs created
    Steps:
      1. Write a unit test that creates a CompilationErrorReport with 3 errors (lex, parse, type)
      2. Call render_report("test.op", "source text", &report)
      3. Assert output contains 3 error blocks and ends with "error: aborting due to 3 previous error(s)"
    Expected Result: All 3 errors rendered, summary footer present
    Failure Indicators: Only 1 error shown, or missing summary
    Evidence: .sisyphus/evidence/task-3-renderer-multi-error.txt
  ```

  **Commit**: YES
  - Message: `feat(errors): add miette-based diagnostic renderer with source context`
  - Files: `src/errors/renderer.rs`, `src/errors.rs`
  - Pre-commit: `cargo test`

---

- [x] 4. Enhance CodegenError with optional span

  **What to do**:
  - In `src/codegen/expressions.rs`, add an optional `span` field to `CodegenError`:
    ```rust
    #[derive(Debug, Clone)]
    pub struct CodegenError {
        pub message: String,
        pub span: Option<miette::SourceSpan>,
    }
    ```
  - Update the `CodegenError::new()` constructor to initialize `span: None` (backward-compatible).
  - Add a `CodegenError::with_span(message: String, span: miette::SourceSpan) -> Self` constructor for codegen sites that have access to a span.
  - Update `impl From<BuilderError> for CodegenError` to set `span: None`.
  - In `src/compiler.rs`, update codegen call sites within `compile_to_module()` that have access to a `Decl`'s `span` field to use `CodegenError::with_span()`:
    - The `Decl::Function { span, .. }` and `Decl::Let { span, .. }` match arms (lines 152-197) — when `codegen_function_declaration` returns an error, wrap it with the declaration's span if the error doesn't already have one.
  - In `src/errors/reporter.rs`, update `CompilerError::Codegen` to store `CodegenError` instead of `String`:
    ```rust
    Codegen(CodegenError),
    ```
  - Update `push_codegen_error` to accept `CodegenError` instead of `String`.
  - Update all call sites that create `CompilerError::Codegen(String)` — e.g., `src/lsp/diagnostics.rs` and `src/errors/tests.rs`.

  **Must NOT do**:
  - Do NOT try to thread spans through expression-level codegen functions — only function/declaration level
  - Do NOT add miette `#[derive(Diagnostic)]` to `CodegenError` yet — the renderer will handle it via a wrapper

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Struct modification + updating call sites. Straightforward refactor with find-and-replace pattern.
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1 (with Tasks 1, 2, 3)
  - **Blocks**: Task 5
  - **Blocked By**: None (can start immediately)

  **References**:

  **Pattern References**:
  - `src/codegen/expressions.rs:31-55` — Current `CodegenError` struct and impls to modify
  - `src/compiler.rs:150-200` — Codegen call sites in `compile_to_module` where `Decl` span is available

  **API/Type References**:
  - `src/token.rs:41-75` — `Span` struct with start/end `Position`
  - `src/error.rs:146-166` — `span_from_position` and `span_from_span` — pattern for converting `Span` to `SourceSpan`
  - `src/errors/reporter.rs:13-22` — `CompilerError::Codegen(String)` variant that needs updating

  **Test References**:
  - `src/errors/tests.rs:173-182` — `test_format_diagnostic_uses_codegen_variant_with_codegen_error_message` — creates `CodegenError::new(String)` and accesses `.message` — must update
  - `src/errors/tests.rs:96-106` — `test_error_bundle_joins_multiple_entries` — creates `CompilerError::Codegen(String::from("broken ir"))` — must update

  **WHY Each Reference Matters**:
  - `expressions.rs:31-55` — the exact struct being modified
  - `compiler.rs:150-200` — shows where `span` from `Decl` can be captured and attached to codegen errors
  - `reporter.rs` — `Codegen(String)` variant needs to become `Codegen(CodegenError)` so span info flows through
  - `tests.rs` — two tests construct `CompilerError::Codegen(String)` directly — must update to new type

  **Acceptance Criteria**:

  - [ ] `CodegenError` has `pub span: Option<miette::SourceSpan>` field
  - [ ] `CodegenError::new()` and `CodegenError::with_span()` constructors exist
  - [ ] `CompilerError::Codegen` stores `CodegenError` not `String`
  - [ ] `cargo build` compiles cleanly
  - [ ] `cargo test` passes (all existing tests updated)

  **QA Scenarios (MANDATORY)**:

  ```
  Scenario: CodegenError retains backward compatibility
    Tool: Bash (cargo test)
    Preconditions: CodegenError modified, tests updated
    Steps:
      1. Run `cargo test 2>&1`
      2. Verify no compilation errors related to CodegenError
    Expected Result: All tests pass, zero compilation errors
    Failure Indicators: "expected String, found CodegenError" or similar type errors
    Evidence: .sisyphus/evidence/task-4-codegen-error-tests.txt

  Scenario: CodegenError with span preserves span info
    Tool: Bash (cargo test)
    Preconditions: with_span constructor exists
    Steps:
      1. Add a unit test that creates `CodegenError::with_span("msg", SourceSpan::new(10.into(), 5))`
      2. Assert `error.span` is `Some(SourceSpan)` with offset 10 and length 5
      3. Run `cargo test -- codegen_error 2>&1`
    Expected Result: Test passes, span info preserved
    Failure Indicators: span is None or wrong values
    Evidence: .sisyphus/evidence/task-4-codegen-span-preserved.txt
  ```

  **Commit**: YES
  - Message: `feat(codegen): add optional span to CodegenError for source context`
  - Files: `src/codegen/expressions.rs`, `src/compiler.rs`, `src/errors/reporter.rs`, `src/errors/tests.rs`, `src/lsp/diagnostics.rs`
  - Pre-commit: `cargo test`

---

- [x] 5. Refactor compile_to_module for multi-error collection

  **What to do**:
  - In `src/compiler.rs`, change `compile_to_module()` to collect ALL errors instead of returning on the first one:
    - Create a `CompilationErrorReport` at the start of the function
    - Collect ALL lex errors via `report.extend_lex_errors(lex_errors.errors)` instead of `lex_errors.errors.into_iter().next()`
    - If there are lex errors, skip parsing (can't parse with invalid tokens), but still return all collected errors
    - Collect ALL parse errors via `report.extend_parse_errors(parse_errors.errors)`
    - If there are parse errors, skip type checking, but return all collected errors
    - Collect ALL type errors via `report.extend_type_errors(type_errors)`
    - If the report has any errors, return `Err(report)` instead of `Err(CompileError::Lex/Parse/Type(single_error))`
    - If no errors, proceed with codegen as before
    - Codegen errors can still be returned individually (they're fatal)
  - Change the return type: `compile_to_module` should return `Result<Module, CompilationResult>` where `CompilationResult` is either a new struct or reuse of `CompilationErrorReport`:
    - Option A: Return `Result<Module, CompilationErrorReport>` — cleaner, but requires the caller to handle the report instead of a single `CompileError`
    - Option B: Keep `CompileError` but add a `CompileError::Multiple(CompilationErrorReport)` variant
    - **Choose Option A** — the caller (`compile_program` and CLI) needs the full report for rendering
  - Update `compile_program()` (line 357-368) to handle the new return type — convert the report into an error message or pass it through
  - Follow the proven pattern from `src/lsp/diagnostics.rs:14-37` for error collection
  - **Important**: The source text (normalized) must be available to the caller for rendering. Either:
    - Return the normalized source alongside the report in the error case: `Err((CompilationErrorReport, String))` where String is the normalized source
    - Or store the normalized source in the report
    - **Choose**: Return `Err((CompilationErrorReport, String))` as a tuple — keeps the report clean

  **Must NOT do**:
  - Do NOT change the `CompileError` enum for IO, Linker, or Codegen variants — those are still single errors
  - Do NOT modify the LSP diagnostics path
  - Do NOT change the behavior when there are zero errors (success path unchanged)

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: Core pipeline refactor. Changes function signatures, return types, and error flow. Must preserve correctness for codegen/link/IO paths while changing lex/parse/type error collection. Requires careful consideration of what to skip when earlier phases fail.
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: NO
  - **Parallel Group**: Wave 2
  - **Blocks**: Tasks 7, 8
  - **Blocked By**: Tasks 1, 3, 4

  **References**:

  **Pattern References**:
  - `src/lsp/diagnostics.rs:14-37` — `get_diagnostics()` — THE pattern to follow for multi-error collection. Shows: create report → extend with lex errors → parse → type check → iterate report entries
  - `src/compiler.rs:112-203` — Current `compile_to_module()` — the function being refactored

  **API/Type References**:
  - `src/errors/reporter.rs:31-108` — `CompilationErrorReport` API: `new()`, `extend_lex_errors()`, `extend_parse_errors()`, `extend_type_errors()`, `push_codegen_error()`, `is_empty()`, `len()`, `entries()`
  - `src/compiler.rs:83-106` — `CompileError` enum — will need modification or the return type changes
  - `src/compiler.rs:357-368` — `compile_program()` — caller of `compile_to_module`, needs updating

  **Test References**:
  - `src/compiler.rs:370-457` — Module tests (`compile_to_module_valid_void_program`, `compile_to_module_lex_error`, `compile_to_module_type_error`) — must be updated for new return type

  **WHY Each Reference Matters**:
  - `lsp/diagnostics.rs` is the PROVEN working pattern — copy its collection strategy
  - `compiler.rs:112-203` is the target function — understand what currently happens at each phase
  - `reporter.rs` is the API to use — know the exact method signatures
  - `compiler.rs` tests will break and need updating

  **Acceptance Criteria**:

  - [ ] `compile_to_module()` collects ALL lex, parse, and type errors (not just the first)
  - [ ] Returns `CompilationErrorReport` with normalized source text in error case
  - [ ] `compile_program()` handles the new return type
  - [ ] Codegen/IO/Linker errors still work as single errors
  - [ ] `cargo test` passes (with updated tests)
  - [ ] When a file has 3 errors, all 3 are collected (not just error #1)

  **QA Scenarios (MANDATORY)**:

  ```
  Scenario: Multiple errors are collected
    Tool: Bash (cargo test)
    Preconditions: compile_to_module refactored
    Steps:
      1. Write a test with source containing multiple errors (e.g., type mismatch AND symbol not found)
      2. Call compile_to_module with this source
      3. Assert the returned error report contains >= 2 errors
      4. Run `cargo test -- compile_to_module 2>&1`
    Expected Result: Report contains all errors, not just the first
    Failure Indicators: Report contains only 1 error, or test panics
    Evidence: .sisyphus/evidence/task-5-multi-error-collection.txt

  Scenario: Single-error files still work
    Tool: Bash (cargo test)
    Preconditions: compile_to_module refactored
    Steps:
      1. Run existing `compile_to_module_lex_error` test (updated for new return type)
      2. Run `compile_to_module_type_error` test
    Expected Result: Both tests pass with updated assertions
    Failure Indicators: Type errors or assertion failures
    Evidence: .sisyphus/evidence/task-5-single-error-still-works.txt

  Scenario: Success path unchanged
    Tool: Bash (cargo test)
    Preconditions: compile_to_module refactored
    Steps:
      1. Run `cargo test -- compile_to_module_valid_void_program 2>&1`
    Expected Result: Valid source still compiles to a module successfully
    Failure Indicators: Test failure on valid source
    Evidence: .sisyphus/evidence/task-5-success-path.txt
  ```

  **Commit**: YES
  - Message: `feat(compiler): collect all errors instead of stopping at first`
  - Files: `src/compiler.rs`
  - Pre-commit: `cargo test`

---

- [x] 6. Enhance suggestions for common error types

  **What to do**:
  - In `src/errors/suggestions.rs`, add new suggestion functions for the ~12 most common error types. Each function takes a `&TypeError` (or `&ParseError`/`&LexError`) and returns `Option<String>` with a context-aware suggestion including example code.
  - **Priority error types to add suggestions for** (in addition to existing `SymbolNotFound` and `CannotInferGenericType`):
    1. `TypeError::TypeMismatch { expected, found, .. }` → "expected `{expected}` but found `{found}`. Try: `let x: {expected} = ...` or cast with `as {expected}`"
    2. `TypeError::ArityMismatch { expected, found, .. }` → "function expects {expected} argument(s) but {found} were provided"
    3. `TypeError::ImmutableAssignment { name, .. }` → "cannot assign to immutable variable `{name}`. Try: `let mutable {name} = ...`"
    4. `TypeError::TypeNotFound { name, .. }` → "type `{name}` not found in scope" + typo suggestion if applicable
    5. `TypeError::NotCallable { .. }` → "expression is not callable. Only functions can be called with `()`"
    6. `TypeError::InvalidCast { from_type, to_type, .. }` → "cannot cast `{from_type}` to `{to_type}`" + suggest valid cast targets
    7. `TypeError::MissingReturnValue { .. }` → "function body must return a value. Try adding `return <value>` at the end"
    8. `TypeError::MissingEntryPoint { .. }` → "no `entry main` function found. Add:\n```\nentry main = f(args: string[]): void =>\n    return void\n```"
    9. `ParseError::UnexpectedToken { expected, found, .. }` → "expected {expected} but found `{found}`"
    10. `ParseError::MissingToken { expected, .. }` → "expected `{expected}` — did you forget to add it?"
    11. `LexError::UnterminatedString { .. }` → "string literal is not closed. Add a closing `'`"
    12. `LexError::InvalidEscapeSequence { .. }` → "invalid escape sequence. Valid escapes: `\\n`, `\\t`, `\\\\`, `\\'`"
  - Create a unified `get_suggestion(error: &CompilerError) -> Option<String>` function that dispatches to the appropriate suggestion function based on the error variant.
  - **Important**: Suggestions should use Opalescent syntax (single quotes for strings, `mutable` keyword not `mut`, `f()` for functions, etc.)
  - Update `format_diagnostic` in `formatter.rs` to use the new unified suggestion function (so the old formatter also benefits while it's still in use).

  **Must NOT do**:
  - Do NOT add suggestions for all 55 error variants — limit to these ~12
  - Do NOT change error enum definitions or their `#[help]` attributes
  - Do NOT create suggestions that reference Rust syntax — use Opalescent syntax only

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
    - Reason: Requires understanding Opalescent language syntax to write correct example code in suggestions. Multiple error types to handle. Not architecturally complex but needs domain knowledge.
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2 (with Task 5)
  - **Blocks**: Task 8
  - **Blocked By**: Task 1

  **References**:

  **Pattern References**:
  - `src/errors/suggestions.rs:24-60` — `closest_identifier_suggestion()` — existing suggestion pattern to follow
  - `src/errors/suggestions.rs:63-71` — `did_you_mean_type_annotation()` — existing pattern for type-specific suggestions
  - `src/errors/formatter.rs:75-108` — Current suggestion rendering in `format_diagnostic` for TypeChecker — shows how suggestions are appended

  **API/Type References**:
  - `src/type_system/errors.rs:1-814` — All `TypeError` variants with their fields — need field names for generating suggestions
  - `src/parser/errors.rs` — All `ParseError` variants
  - `src/error.rs` — All `LexError` variants

  **External References**:
  - README.md "Language Basics" section — Opalescent syntax reference. Suggestions must use correct Opalescent syntax:
    - `let mutable x = ...` (not `let mut`)
    - `f(params): return_type =>` (not `fn`)
    - Single-quoted strings `'hello'` (not `"hello"`)
    - `entry main = f(args: string[]): void =>` (entry point format)

  **WHY Each Reference Matters**:
  - Existing suggestions show the function signature pattern to follow
  - `formatter.rs` shows where suggestions are currently injected — update point
  - TypeError/ParseError/LexError definitions provide the field names needed for generating contextual messages
  - README syntax reference is CRITICAL — suggestions must use correct Opalescent syntax

  **Acceptance Criteria**:

  - [ ] `get_suggestion()` function exists and dispatches to per-variant suggestion functions
  - [ ] At least 12 error variants have context-aware suggestions
  - [ ] Suggestions use correct Opalescent syntax (not Rust)
  - [ ] `cargo test` passes
  - [ ] ImmutableAssignment suggestion mentions `let mutable`

  **QA Scenarios (MANDATORY)**:

  ```
  Scenario: TypeMismatch produces helpful suggestion
    Tool: Bash (cargo test)
    Preconditions: Suggestion functions implemented
    Steps:
      1. Write a test that creates TypeError::TypeMismatch { expected: "int32", found: "string", ... }
      2. Call get_suggestion with this error
      3. Assert result contains "int32" and "string" and actionable text
    Expected Result: Suggestion mentions both types and suggests a fix
    Failure Indicators: Returns None or generic message without type names
    Evidence: .sisyphus/evidence/task-6-type-mismatch-suggestion.txt

  Scenario: ImmutableAssignment suggests mutable keyword
    Tool: Bash (cargo test)
    Preconditions: Suggestion functions implemented
    Steps:
      1. Write a test that creates TypeError::ImmutableAssignment { name: "x", ... }
      2. Call get_suggestion with this error
      3. Assert result contains "let mutable" (Opalescent syntax, not "let mut")
    Expected Result: Suggestion contains "let mutable x"
    Failure Indicators: Contains "let mut" (Rust syntax) or returns None
    Evidence: .sisyphus/evidence/task-6-immutable-suggestion.txt

  Scenario: MissingEntryPoint shows full example
    Tool: Bash (cargo test)
    Preconditions: Suggestion functions implemented
    Steps:
      1. Write a test that creates TypeError::MissingEntryPoint
      2. Call get_suggestion
      3. Assert result contains "entry main" and "f(args: string[]): void"
    Expected Result: Full entry point example in suggestion
    Failure Indicators: Missing or incorrect example syntax
    Evidence: .sisyphus/evidence/task-6-entry-point-suggestion.txt
  ```

  **Commit**: YES
  - Message: `feat(errors): add context-aware suggestions for common error types`
  - Files: `src/errors/suggestions.rs`, `src/errors/formatter.rs`
  - Pre-commit: `cargo test`

---

- [x] 7. Update CLI (app.rs) to use new renderer for all error output

  **What to do**:
  - In `src/app.rs`, replace ALL `eprintln!("error: compilation failed: {error}")` patterns with calls to the new `render_report()` from `src/errors/renderer.rs`.
  - The source text and filename are needed for rendering. These are already available in each CLI function that reads source files. Thread them to the renderer:
    - `compile_and_run()` (line 198-233): Has `source` and `source_path`. On compilation failure, render the error report.
    - `run_with_args()` default path (line 176-194): Has `source` and `source_path`. Same treatment.
    - `run_check_command()` (line 454-488): Has `source` and `source_path`. Currently does its own lex/parse/type pipeline. Refactor to use `compile_to_module` (or duplicate the multi-error collection pattern). Render errors with `render_report()`.
    - `run_build_command()` (line 491-526): Has `source` and hardcoded `"src/main.op"` filename. Render on failure.
    - `run_doc_command()` (line 392-436): Has `source` and `source_path`. Currently prints bare "error: lex errors in source". Render with `render_report()`.
    - `run_watch_command()` (line 529-557): Calls `compile_and_run()` — inherits its error rendering.
  - **Key change**: `compile_program()` currently returns `Result<PathBuf, CompileError>`. After Task 5, the error path includes `CompilationErrorReport` + normalized source. The CLI functions need to:
    1. Read the file
    2. Call the compiler (which now returns multi-error reports)
    3. On error, call `render_report(filename, &normalized_source, &report)`
    4. Print the rendered output to stderr via `eprintln!("{rendered}")`
  - **For `run_check_command`**: This function currently does its own lex→parse→typecheck pipeline manually. It should be updated to:
    - Either reuse `compile_to_module` (without the codegen phase — which means extracting the front-end pipeline into a shared function)
    - Or replicate the `CompilationErrorReport` collection pattern from Task 5 locally
    - **Choose**: Extract a `check_source(source: &str) -> Result<Program, (CompilationErrorReport, String)>` function that both `compile_to_module` and `run_check_command` can use
  - Print rendered output to **stderr** (not stdout) — stdout is for program output.
  - Respect `NO_COLOR` environment variable: if set, disable miette's fancy rendering. miette handles this automatically via `set_hook` or by checking the env var.

  **Must NOT do**:
  - Do NOT change the exit codes (still return Err(1) on compilation failure)
  - Do NOT change the success output (e.g., `println!("{}", binary_path.display())` stays)
  - Do NOT break any existing CLI tests in `app.rs` — update assertions if needed
  - Do NOT add a `--color` flag (out of scope for this plan)

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
    - Reason: Multiple functions to update across a large file. Requires understanding the data flow from file reading through compilation to error rendering. Must handle the check command specially (no codegen). Integration work.
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: NO
  - **Parallel Group**: Wave 3
  - **Blocks**: Tasks 8, 9
  - **Blocked By**: Tasks 3, 5

  **References**:

  **Pattern References**:
  - `src/app.rs:198-233` — `compile_and_run()` — primary compilation + run function. Shows current error handling pattern (`eprintln!("error: compilation failed: {error}")`) to replace
  - `src/app.rs:454-488` — `run_check_command()` — does manual lex→parse→typecheck. Needs most work to collect all errors
  - `src/app.rs:491-526` — `run_build_command()` — reads source, calls `compile_program()`, prints error
  - `src/app.rs:392-436` — `run_doc_command()` — does lex→parse manually, prints bare "error: lex errors in source"

  **API/Type References**:
  - `src/errors/renderer.rs` (from Task 3) — `render_report(filename, source, &report)` — the function to call
  - `src/compiler.rs` (after Task 5) — New `compile_to_module` return type with `CompilationErrorReport`

  **Test References**:
  - `src/app.rs:565-1098` — Extensive CLI tests. Many test error paths by asserting `Err(1)`. These should still pass but some may need adjustment if they capture stderr.

  **WHY Each Reference Matters**:
  - Each `app.rs` function is a separate CLI command that needs updating — must visit ALL of them
  - `renderer.rs` API determines how to call the rendering
  - CLI tests ensure no regression in exit codes or basic behavior

  **Acceptance Criteria**:

  - [ ] All `eprintln!("error: compilation failed: ...")` replaced with `render_report()` calls
  - [ ] `run_check_command` collects and renders all errors (not just "error: lex errors in source")
  - [ ] `run_doc_command` renders errors properly (not just "error: lex errors in source")
  - [ ] `compile_and_run` renders errors with source context
  - [ ] `run_build_command` renders errors with source context
  - [ ] All CLI tests still pass
  - [ ] `cargo test` passes

  **QA Scenarios (MANDATORY)**:

  ```
  Scenario: Check command shows source context for type errors
    Tool: Bash (cargo run)
    Preconditions: All previous tasks complete, test project exists
    Steps:
      1. Run `cargo run -- check test-projects/error-display/src/type_error.op 2>&1`
      2. Assert stderr contains source code lines with line numbers
      3. Assert stderr contains underline annotations (─, ^, or similar Unicode markers)
      4. Assert stderr contains an error code matching "opalescent::"
      5. Assert exit code is 1
    Expected Result: Beautiful miette-style error output with source context
    Failure Indicators: Bare "error: type check failed" without source lines
    Evidence: .sisyphus/evidence/task-7-check-source-context.png or .txt

  Scenario: Multiple errors shown in check command
    Tool: Bash (cargo run)
    Preconditions: test-projects/error-display/src/main.op has multiple errors
    Steps:
      1. Run `cargo run -- check test-projects/error-display/src/main.op 2>&1`
      2. Count occurrences of "error" or "×" in output
      3. Assert at least 2 distinct error blocks are shown
      4. Assert output ends with error count summary (e.g., "error: aborting due to N previous error(s)")
    Expected Result: Multiple errors displayed, summary footer present
    Failure Indicators: Only 1 error shown, no summary footer
    Evidence: .sisyphus/evidence/task-7-multi-error-output.txt

  Scenario: CLI tests still pass
    Tool: Bash (cargo test)
    Preconditions: app.rs updated
    Steps:
      1. Run `cargo test -- app::tests 2>&1`
      2. Assert all tests pass
    Expected Result: 0 failures in app::tests module
    Failure Indicators: Any test failure in app::tests
    Evidence: .sisyphus/evidence/task-7-cli-tests.txt
  ```

  **Commit**: YES
  - Message: `feat(cli): use miette renderer for all error output`
  - Files: `src/app.rs`
  - Pre-commit: `cargo test`

---

- [x] 8. Update existing tests for new rendering output

  **What to do**:
  - In `src/errors/tests.rs`, update all test assertions that check plain-text formatter output to work with the new system:
    - Tests asserting `format_diagnostic()` output (e.g., `test_format_diagnostic_includes_phase_code_help_and_docs`) — either update to check the new renderer output, or keep testing the old formatter (if it's still used) and add parallel tests for the new renderer
    - Tests asserting `CompilationErrorReport::render()` output — update `render()` to use the new renderer, or add new tests
    - Tests for `format_codegen_error()` and `format_error_bundle()` — similar treatment
  - **Strategy**: Keep the old `formatter.rs` tests as-is (they test a module that still exists), and ADD new tests for `renderer.rs`:
    - `test_renderer_produces_source_context_for_lex_error`
    - `test_renderer_produces_source_context_for_parse_error`
    - `test_renderer_produces_source_context_for_type_error`
    - `test_renderer_handles_codegen_error_without_span`
    - `test_renderer_handles_codegen_error_with_span`
    - `test_renderer_handles_unknown_span`
    - `test_renderer_shows_multiple_errors_with_summary`
    - `test_renderer_includes_suggestions_when_available`
  - Update `CompilationErrorReport::render()` in `reporter.rs` to use the new renderer instead of `format_error_bundle`. This requires passing source text and filename — either change the `render()` method signature or add a `render_with_source()` method.
  - Add tests for the updated `compile_to_module` that verify multi-error collection.

  **Must NOT do**:
  - Do NOT delete old formatter tests — they test `formatter.rs` which still exists
  - Do NOT add tests for all 55 error variants — focus on representative samples (1 per phase + edge cases)

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
    - Reason: Multiple test files to update/create. Requires understanding both old and new rendering output. Must create meaningful assertions for miette's graphical output.
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: NO
  - **Parallel Group**: Wave 3
  - **Blocks**: Task 9
  - **Blocked By**: Tasks 3, 5, 6, 7

  **References**:

  **Pattern References**:
  - `src/errors/tests.rs:1-259` — All existing tests to review and update
  - `src/errors/renderer.rs` (from Task 3) — New renderer API to test

  **API/Type References**:
  - `src/errors/reporter.rs:103-107` — `CompilationErrorReport::render()` — may need updating to use new renderer

  **Test References**:
  - `src/errors/tests.rs:66-77` — `test_format_diagnostic_includes_phase_code_help_and_docs` — example of existing assertion pattern
  - `src/errors/tests.rs:109-135` — `test_compilation_error_report_collects_and_renders_multi_phase_errors` — multi-error test pattern

  **Acceptance Criteria**:

  - [ ] All existing tests pass or are updated to pass
  - [ ] At least 8 new tests for the renderer module
  - [ ] `CompilationErrorReport::render()` optionally supports source-context rendering
  - [ ] `cargo test` passes with 0 failures

  **QA Scenarios (MANDATORY)**:

  ```
  Scenario: All tests pass
    Tool: Bash (cargo test)
    Preconditions: Tests updated
    Steps:
      1. Run `cargo test 2>&1`
      2. Assert 0 failures
      3. Count total test count — should be higher than before (new tests added)
    Expected Result: All tests pass, test count increased
    Failure Indicators: Any test failure, or test count same as before (no new tests)
    Evidence: .sisyphus/evidence/task-8-all-tests-pass.txt

  Scenario: Renderer tests cover all phases
    Tool: Bash (cargo test)
    Preconditions: New tests written
    Steps:
      1. Run `cargo test -- renderer 2>&1`
      2. Assert tests exist for lex, parse, type, and codegen errors
      3. Assert all pass
    Expected Result: At least 4 phase-specific tests pass
    Failure Indicators: Missing tests for any phase
    Evidence: .sisyphus/evidence/task-8-renderer-tests.txt
  ```

  **Commit**: YES
  - Message: `test(errors): update tests for miette-rendered output`
  - Files: `src/errors/tests.rs`, `src/errors/reporter.rs`
  - Pre-commit: `cargo test`

---

- [x] 9. End-to-end integration verification

  **What to do**:
  - Write integration tests (or add to existing test infrastructure) that verify the full pipeline from source file to rendered error output:
    - Test 1: Compile `test-projects/error-display/src/main.op` with multiple errors → verify stderr contains source lines, annotations, error codes, and summary footer
    - Test 2: Compile `test-projects/error-display/src/lex_error.op` → verify lex error renders with source context
    - Test 3: Compile `test-projects/error-display/src/type_error.op` → verify type error renders with suggestion
    - Test 4: Compile `test-projects/error-display/src/parse_error.op` → verify parse error renders with expected/found info
    - Test 5: Compile a valid source file → verify no error output (success path regression)
    - Test 6: Compile an empty file → verify graceful error handling (no panic)
  - These tests should be Rust integration tests (in `tests/` directory or `src/errors/tests.rs`) that:
    1. Read the test `.op` file
    2. Run the compilation pipeline
    3. Render the error report
    4. Assert specific content in the rendered output
  - **Alternatively**: Use `cargo run -- check <file> 2>&1` in a bash-based test and assert on captured stderr. This tests the full CLI integration.
  - Verify edge cases:
    - Error at the very end of a file (EOF)
    - Error on the first line/column
    - Very long line (>200 chars) — miette should handle wrapping
    - File with only whitespace

  **Must NOT do**:
  - Do NOT gate these behind the `integration` feature flag unless they spawn external processes
  - Do NOT create excessive numbers of test files — focus on representative cases

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: Integration testing across the full pipeline. Requires creating test fixtures, running the compiler, and asserting on complex string output. Edge case verification needs careful setup.
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: NO
  - **Parallel Group**: Wave 3 (last task before FINAL)
  - **Blocks**: F1-F4
  - **Blocked By**: Tasks 2, 7, 8

  **References**:

  **Pattern References**:
  - `src/compiler.rs:370-457` — Existing compiler tests that create source and call `compile_to_module()` — follow this pattern
  - `tests/` directory (if exists) — existing integration test patterns
  - `src/errors/tests.rs:109-135` — Multi-error report test — similar assertion pattern

  **Test References**:
  - `test-projects/error-display/` (from Task 2) — Test fixture files

  **Acceptance Criteria**:

  - [ ] At least 6 integration tests covering all error phases + success + edge cases
  - [ ] All integration tests pass
  - [ ] `cargo test` passes
  - [ ] Edge cases (EOF error, empty file) don't panic

  **QA Scenarios (MANDATORY)**:

  ```
  Scenario: Full pipeline renders lex errors with source context
    Tool: Bash (cargo run + assertions)
    Preconditions: All previous tasks complete
    Steps:
      1. Run `cargo run -- check test-projects/error-display/src/lex_error.op 2>&1`
      2. Capture stderr output
      3. Assert output contains: line number, source code line, underline annotation, error code
      4. Assert exit code is 1
    Expected Result: Lex error with full miette-style rendering
    Failure Indicators: Bare text error or panic
    Evidence: .sisyphus/evidence/task-9-e2e-lex-error.txt

  Scenario: Full pipeline renders multiple errors with summary
    Tool: Bash (cargo run + assertions)
    Preconditions: main.op has multiple deliberate errors
    Steps:
      1. Run `cargo run -- check test-projects/error-display/src/main.op 2>&1`
      2. Count distinct error blocks in output
      3. Assert >= 2 error blocks
      4. Assert output contains "aborting due to" or similar summary footer
    Expected Result: Multiple errors shown, summary present
    Failure Indicators: Single error only, no summary
    Evidence: .sisyphus/evidence/task-9-e2e-multi-error.txt

  Scenario: Valid source produces no error output
    Tool: Bash (cargo run)
    Preconditions: Valid test project exists
    Steps:
      1. Run `cargo run -- check test-projects/hello-world/src/main.op 2>&1`
      2. Assert stderr does not contain "error"
      3. Assert stdout contains "check passed"
      4. Assert exit code is 0
    Expected Result: Clean success, no error rendering
    Failure Indicators: Error output on valid source, or exit code 1
    Evidence: .sisyphus/evidence/task-9-e2e-success-path.txt

  Scenario: Empty file handled gracefully
    Tool: Bash (cargo run)
    Preconditions: Create a temporary empty .op file
    Steps:
      1. Create `/tmp/empty.op` with empty content
      2. Run `cargo run -- check /tmp/empty.op 2>&1`
      3. Assert no panic (exit code is 0 or 1, not 101/signal)
    Expected Result: Graceful error or success, no panic
    Failure Indicators: Process killed by signal (panic/segfault)
    Evidence: .sisyphus/evidence/task-9-e2e-empty-file.txt
  ```

  **Commit**: YES
  - Message: `test(errors): add end-to-end error rendering integration tests`
  - Files: `src/errors/tests.rs` or `tests/error_rendering.rs`
  - Pre-commit: `cargo test`

---

## Final Verification Wave (MANDATORY — after ALL implementation tasks)

> 4 review agents run in PARALLEL. ALL must APPROVE. Present consolidated results to user and get explicit "okay" before completing.
>
> **Do NOT auto-proceed after verification. Wait for user's explicit approval before marking work complete.**

- [x] F1. **Plan Compliance Audit** — `oracle`
  Read the plan end-to-end. For each "Must Have": verify implementation exists (read file, run `cargo run -- check test-projects/error-display/src/main.op 2>&1`). For each "Must NOT Have": search codebase for forbidden patterns (e.g., `#[source_code]` on `LexError`/`ParseError`/`TypeError`, behavioral changes to LSP diagnostics beyond type compatibility) — reject with file:line if found. Check evidence files exist in `.sisyphus/evidence/`. Compare deliverables against plan.
  Output: `Must Have [N/N] | Must NOT Have [N/N] | Tasks [N/N] | VERDICT: APPROVE/REJECT`

- [x] F2. **Code Quality Review** — `unspecified-high`
  Run `cargo build 2>&1`, `cargo clippy 2>&1`, `cargo test 2>&1`. Review all changed files for: `as any` equivalents (`unsafe` blocks, `unwrap()` in non-test code), dead code, commented-out code, unused imports. Check for AI slop: excessive comments, over-abstraction, generic variable names (data/result/item/temp). Verify no `#[allow(unused)]` was added to suppress legitimate warnings.
  Output: `Build [PASS/FAIL] | Clippy [PASS/FAIL] | Tests [N pass/N fail] | Files [N clean/N issues] | VERDICT`

- [x] F3. **Real Manual QA** — `unspecified-high`
  Start from clean state. Create multiple `.op` test files with known errors across all phases (lex, parse, type, codegen). Run `cargo run -- check <file> 2>&1` for each. Verify: source code lines visible, underline annotations correct, error codes present, help text actionable, multiple errors shown, summary footer present, colored output works. Test edge cases: empty file, error at EOF, very long lines, multiple errors same line. Save all output to `.sisyphus/evidence/final-qa/`.
  Output: `Scenarios [N/N pass] | Integration [N/N] | Edge Cases [N tested] | VERDICT`

- [x] F4. **Scope Fidelity Check** — `deep`
  For each task: read "What to do", read actual diff (`git diff`). Verify 1:1 — everything in spec was built (no missing), nothing beyond spec was built (no creep). Check "Must NOT do" compliance: no `#[source_code]` on error enums, no behavioral changes to LSP diagnostics (type-compatibility updates OK), no changes to `#[diagnostic]`/`#[label]` attributes, suggestions limited to ~12 types. Detect cross-task contamination. Flag unaccounted changes.
  Output: `Tasks [N/N compliant] | Contamination [CLEAN/N issues] | Unaccounted [CLEAN/N files] | VERDICT`

---

## Commit Strategy

| After Task(s) | Commit Message | Files | Pre-commit |
|---|---|---|---|
| 1 | `fix(errors): unify span conversion to use exclusive end offset` | `src/error.rs` | `cargo test` |
| 2 | `test(errors): add error-display test project with deliberate errors` | `test-projects/error-display/**` | — |
| 3 | `feat(errors): add miette-based diagnostic renderer with source context` | `src/errors/renderer.rs`, `src/errors.rs` | `cargo test` |
| 4 | `feat(codegen): add optional span to CodegenError for source context` | `src/codegen/expressions.rs`, `src/compiler.rs`, `src/errors/reporter.rs` | `cargo test` |
| 5 | `feat(compiler): collect all errors instead of stopping at first` | `src/compiler.rs` | `cargo test` |
| 6 | `feat(errors): add context-aware suggestions for common error types` | `src/errors/suggestions.rs` | `cargo test` |
| 7 | `feat(cli): use miette renderer for all error output` | `src/app.rs` | `cargo test` |
| 8 | `test(errors): update tests for miette-rendered output` | `src/errors/tests.rs` | `cargo test` |
| 9 | `test(errors): add end-to-end error rendering integration tests` | `src/errors/tests.rs` or integration test | `cargo test` |

---

## Success Criteria

### Verification Commands
```bash
cargo build 2>&1                                    # Expected: clean build, no errors
cargo test 2>&1                                      # Expected: all tests pass
cargo clippy 2>&1                                    # Expected: no new warnings
cargo run -- check test-projects/error-display/src/main.op 2>&1  # Expected: colored error output with source context
```

### Final Checklist
- [ ] All "Must Have" features present and working
- [ ] All "Must NOT Have" guardrails respected
- [ ] All tests pass (`cargo test`)
- [ ] Clean build (`cargo build`)
- [ ] Error output shows source lines with line numbers
- [ ] Error output shows underline annotations at correct positions
- [ ] Multiple errors displayed (not just the first)
- [ ] Summary footer shows error count
- [ ] Suggestions displayed for common error types
