# CLI Help Alignment — Full Surface Coverage

## TL;DR

> **Quick Summary**: Expand the `opal` CLI help output to show every command and option across the entire toolchain surface, add `--help` as a full alias for `help`, stub unimplemented commands with `error: '<cmd>' not yet implemented`, and update the README CLI Reference to match. All behind TDD with extensive tests.
> 
> **Deliverables**:
> - `src/app.rs` — expanded `print_help` (all commands + all topics), `--help` alias, subcommand dispatch stubs, testability refactoring, extensive `#[cfg(test)]` module
> - `README.md` — CLI Reference section updated to document every command and option
> 
> **Estimated Effort**: Medium
> **Parallel Execution**: YES — 3 waves
> **Critical Path**: Task 1 → Task 2 → Task 3 → Task 4 → Task 5 → F1–F4

---

## Context

### Original Request
Align the help command for the entire CLI surface. Every single option should be shown in the help output. Additionally, show every single option in the README so users can interface with the compiler. `opal help` and `opal --help` should both display help. Unimplemented commands should echo an error when invoked but still appear in help.

### Interview Summary
**Key Discussions**:
- Full CLI surface: compile, `--run`, help, `--help`, pkg, fmt, lsp, test, doc, bench
- Unimplemented commands (pkg, fmt, lsp, test, doc, bench) appear in help but print `error: '<cmd>' not yet implemented` to stderr and exit 1 when dispatched
- `--help` is a full pass-through alias: `opal --help pkg` = `opal help pkg`
- TDD with red-green-refactor, extensive tests — not barebones
- README is a superset of help (tables, examples), help is compact plain text — both list the same command surface

**Research Findings**:
- `src/app.rs` is 126 lines total — `print_help()` (private, prints to stdout directly) and `run_impl()` (dispatch logic)
- `print_help` only handles topics: top-level, `pkg`, `fmt` — missing `lsp`, `test`, `doc`, `bench`
- `run_impl` only dispatches: `help`, `<file.op>`, `--run` — no subcommand dispatch at all
- No `--help` handling exists. `--help` is currently silently filtered as a flag, then "no source file" error fires
- No tests exist for `app.rs` — zero `#[cfg(test)]` module
- All 6 subcommand modules exist with full implementations, just not wired to CLI dispatch
- `print_help` cannot be tested without refactoring — it prints directly via `println!`

### Metis Review
**Identified Gaps** (addressed):
- **Dispatch order**: Subcommand match must happen BEFORE file-path logic to avoid `fs::read_to_string("pkg")` — addressed in Task 3 implementation
- **`--help` position**: Must be checked before flag filtering, otherwise swallowed — addressed in Task 3
- **`--run` flag leaking**: `--run` anywhere matches; after adding subcommands, `opal pkg --run` would incorrectly set flag — addressed by moving `--run` processing after subcommand dispatch
- **Testability**: `print_help()` prints to stdout; refactored to return `String` for TDD — addressed in Task 1
- **Exit code/stderr for unimplemented**: Decided exit 1 + stderr, matching existing `eprintln!("error: ...")` pattern
- **No-args behavior**: Keep current (error + exit 1) — not requested to change

---

## Work Objectives

### Core Objective
Make every CLI command, flag, and help topic visible in `opal help` / `opal --help` output, stub unimplemented commands, and synchronize the README CLI Reference section.

### Concrete Deliverables
- `src/app.rs` with complete help text, `--help` alias, subcommand stubs, and extensive test suite
- `README.md` CLI Reference section listing all commands and options

### Definition of Done
- [ ] `cargo test` passes with 0 failures
- [ ] `opal help` and `opal --help` both show all 10 commands/flags
- [ ] `help lsp`, `help test`, `help doc`, `help bench` each produce non-error output
- [ ] Unimplemented commands print `error: '<cmd>' not yet implemented` to stderr and exit 1
- [ ] README CLI Reference lists all commands matching help output
- [ ] Existing behavior (`opal <file.op>`, `opal <file.op> --run`, `opal help pkg`, `opal help fmt`) unchanged

### Must Have
- All 10 commands/flags visible in top-level help: `<file.op>`, `--run`, `help`, `--help`, `pkg`, `fmt`, `lsp`, `test`, `doc`, `bench`
- Help topics for all subcommands: `pkg`, `fmt`, `lsp`, `test`, `doc`, `bench`
- `--help` as full pass-through alias for `help` (with topic support)
- Unimplemented command stubs printing error and exiting 1
- TDD with extensive tests
- README CLI Reference matches help surface

### Must NOT Have (Guardrails)
- Do NOT wire actual subcommand implementations — only stub with "unimplemented" error
- Do NOT import or call into `package_manager`, `formatter`, `lsp`, `testing`, `doc_gen`, or `benchmarks` modules
- Do NOT modify any source files other than `src/app.rs` and `README.md`
- Do NOT add `--version` flag (not requested)
- Do NOT change `opal` (no-args) behavior — keep current error + exit 1
- Do NOT change existing working behavior of `opal <file.op>`, `opal <file.op> --run`, `opal help`, `opal help pkg`, `opal help fmt`
- Do NOT introduce external CLI parsing crates (clap, etc.) — keep hand-rolled dispatch

---

## Verification Strategy

> **ZERO HUMAN INTERVENTION** — ALL verification is agent-executed. No exceptions.

### Test Decision
- **Infrastructure exists**: YES (Rust built-in `#[test]`, extensive test suite 855+ tests)
- **Automated tests**: TDD (red-green-refactor)
- **Framework**: Rust built-in `#[cfg(test)]` + `#[test]`
- **Each task follows**: RED (failing test) → GREEN (minimal impl) → REFACTOR

### QA Policy
Every task MUST include agent-executed QA scenarios.
Evidence saved to `.sisyphus/evidence/task-{N}-{scenario-slug}.{ext}`.

- **CLI**: Use Bash — Run `cargo test`, check output
- **Text verification**: Use Bash — Assert help strings contain expected content

---

## Execution Strategy

### Parallel Execution Waves

```
Wave 1 (Foundation — testability refactoring):
├── Task 1: Refactor app.rs for testability [quick]

Wave 2 (Core implementation — TDD cycle):
├── Task 2: TDD RED — write all failing tests [deep]
├── Task 3: TDD GREEN — implement help expansion + dispatch stubs [deep]
│   (Task 3 depends on Task 2)

Wave 3 (Documentation + polish):
├── Task 4: TDD REFACTOR — clean up implementation [quick]
├── Task 5: Update README CLI Reference [quick]
│   (Tasks 4 and 5 are parallel)

Wave FINAL (After ALL tasks — 4 parallel reviews, then user okay):
├── Task F1: Plan compliance audit (oracle)
├── Task F2: Code quality review (unspecified-high)
├── Task F3: Real manual QA (unspecified-high)
└── Task F4: Scope fidelity check (deep)
-> Present results -> Get explicit user okay

Critical Path: Task 1 → Task 2 → Task 3 → Task 4 → F1-F4 → user okay
Parallel Speedup: Tasks 4 & 5 run in parallel
Max Concurrent: 2 (Wave 3)
```

### Dependency Matrix

| Task | Depends On | Blocks | Wave |
|------|-----------|--------|------|
| 1    | —         | 2, 3   | 1    |
| 2    | 1         | 3      | 2    |
| 3    | 2         | 4, 5   | 2    |
| 4    | 3         | F1-F4  | 3    |
| 5    | 3         | F1-F4  | 3    |
| F1-F4| 4, 5      | —      | FINAL|

### Agent Dispatch Summary

- **Wave 1**: **1 task** — T1 → `quick`
- **Wave 2**: **2 tasks** (sequential) — T2 → `deep`, T3 → `deep`
- **Wave 3**: **2 tasks** (parallel) — T4 → `quick`, T5 → `quick`
- **FINAL**: **4 tasks** (parallel) — F1 → `oracle`, F2 → `unspecified-high`, F3 → `unspecified-high`, F4 → `deep`

---

## TODOs

- [x] 1. Refactor `app.rs` for testability (TDD prerequisite)

  **What to do**:
  - Refactor `print_help(topic: Option<&str>)` into two functions:
    - `help_text(topic: Option<&str>) -> String` — builds and returns the full help string (replaces all `println!` with string formatting). This is the testable core.
    - `print_help(topic: Option<&str>)` — thin wrapper that calls `help_text()` and prints to stdout. Keeps the public API identical.
  - Refactor `run_impl()` into two functions:
    - `run_with_args(args: &[String]) -> Result<(), i32>` — accepts an explicit args slice. Contains all dispatch logic currently in `run_impl()`. This is the testable core.
    - `run_impl()` — thin wrapper that calls `run_with_args(&std::env::args().collect::<Vec<_>>())`. Keeps the existing call chain from `run()` unchanged.
  - Keep all functions private (`fn`, not `pub fn`) except `run()` which is already `pub`.
  - Verify `cargo test` still passes after refactoring (no behavior change).

  **Must NOT do**:
  - Do NOT change any help text content yet — only restructure for testability
  - Do NOT change any dispatch behavior — only split functions
  - Do NOT add any new commands or flags
  - Do NOT make `help_text` or `run_with_args` public — they only need to be visible within the `#[cfg(test)]` module inside `app.rs`

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Mechanical refactoring of a 126-line file, no new logic
  - **Skills**: []
  - **Skills Evaluated but Omitted**:
    - None needed — straightforward Rust refactoring

  **Parallelization**:
  - **Can Run In Parallel**: NO
  - **Parallel Group**: Wave 1 (solo)
  - **Blocks**: Tasks 2, 3
  - **Blocked By**: None (can start immediately)

  **References**:

  **Pattern References**:
  - `src/app.rs:15-56` — Current `print_help()` function with all `println!` calls to convert to string building
  - `src/app.rs:61-66` — Current `run()` → `run_impl()` delegation pattern to replicate for `run_impl()` → `run_with_args()`
  - `src/app.rs:69-126` — Current `run_impl()` dispatch logic to move into `run_with_args(args: &[String])`

  **WHY Each Reference Matters**:
  - `app.rs:15-56`: Every `println!` in `print_help` must become string formatting (`format!` / `write!` / `push_str`) in `help_text`. The match arms and their text content must be preserved exactly.
  - `app.rs:61-66`: The `run()` → `run_impl()` wrapper pattern is the exact pattern to follow for `run_impl()` → `run_with_args()`.
  - `app.rs:69-126`: The args collection at line 70 (`std::env::args().collect()`) moves to `run_impl()` wrapper; the rest moves to `run_with_args(args)`. The args indexing (`.get(1)`, `.iter().skip(1)`) must reference the passed-in slice instead of re-collecting from env.

  **Acceptance Criteria**:

  - [ ] `help_text(None)` returns the same text that `print_help(None)` currently prints
  - [ ] `help_text(Some("pkg"))` returns the same text that `print_help(Some("pkg"))` currently prints
  - [ ] `help_text(Some("fmt"))` returns the same text that `print_help(Some("fmt"))` currently prints
  - [ ] `run_with_args` exists and accepts `&[String]`
  - [ ] `cargo test` passes with 0 failures (no regressions)
  - [ ] `cargo clippy -- -D warnings` passes

  **QA Scenarios:**

  ```
  Scenario: Refactoring preserves all existing behavior
    Tool: Bash
    Preconditions: src/app.rs has been refactored
    Steps:
      1. Run `cargo test` and capture output
      2. Assert exit code is 0
      3. Assert output contains "0 failed"
    Expected Result: All existing tests pass, exit code 0
    Failure Indicators: Any test failure, non-zero exit code
    Evidence: .sisyphus/evidence/task-1-cargo-test.txt

  Scenario: Refactoring preserves clippy compliance
    Tool: Bash
    Preconditions: src/app.rs has been refactored
    Steps:
      1. Run `cargo clippy -- -D warnings` and capture output
      2. Assert exit code is 0
    Expected Result: Zero clippy warnings
    Failure Indicators: Any clippy warning or error
    Evidence: .sisyphus/evidence/task-1-clippy.txt
  ```

  **Commit**: YES
  - Message: `refactor(cli): extract help text builder and args-based dispatch for testability`
  - Files: `src/app.rs`
  - Pre-commit: `cargo test`

- [x] 2. TDD RED — Write all failing tests for complete help surface

  **What to do**:
  - Add a `#[cfg(test)] mod tests { ... }` module at the bottom of `src/app.rs`
  - Write extensive tests that will all FAIL initially (red phase). Tests must call `help_text()` and `run_with_args()` directly.
  - **Help text completeness tests** (test `help_text` return value):
    - `top_level_help_contains_all_commands`: Assert `help_text(None)` contains each of: `"<file.op>"`, `"--run"`, `"help"`, `"--help"`, `"pkg"`, `"fmt"`, `"lsp"`, `"test"`, `"doc"`, `"bench"`
    - `top_level_help_contains_examples_section`: Assert contains `"Examples:"`
    - `help_pkg_shows_all_subcommands`: Assert `help_text(Some("pkg"))` contains each of: `"init"`, `"add"`, `"remove"`, `"install"`, `"publish"`
    - `help_fmt_shows_all_flags`: Assert `help_text(Some("fmt"))` contains each of: `"--check"`, `"--config"`
    - `help_lsp_shows_stdio_flag`: Assert `help_text(Some("lsp"))` contains `"--stdio"` and does NOT contain `"Unknown help topic"`
    - `help_test_shows_flags`: Assert `help_text(Some("test"))` contains `"--target"` and `"--filter"` and does NOT contain `"Unknown help topic"`
    - `help_doc_shows_format_flag`: Assert `help_text(Some("doc"))` contains `"--format"` and `"md"` and `"html"` and does NOT contain `"Unknown help topic"`
    - `help_bench_shows_usage`: Assert `help_text(Some("bench"))` is non-empty and does NOT contain `"Unknown help topic"`
    - `help_unknown_topic_contains_error`: Assert `help_text(Some("nonexistent"))` contains `"Unknown help topic"`
  - **`--help` alias tests** (test `run_with_args` dispatch):
    - `dash_dash_help_shows_top_level_help`: Call `run_with_args` with `["opal", "--help"]`, capture behavior — should produce help (return `Ok(())`)
    - `dash_dash_help_with_topic_shows_topic`: Call `run_with_args` with `["opal", "--help", "pkg"]`, should produce pkg help (return `Ok(())`)
  - **Unimplemented command tests** (test `run_with_args` dispatch):
    - `unimplemented_pkg_returns_error`: `run_with_args(["opal", "pkg"])` returns `Err(1)`
    - `unimplemented_fmt_returns_error`: `run_with_args(["opal", "fmt"])` returns `Err(1)`
    - `unimplemented_lsp_returns_error`: `run_with_args(["opal", "lsp"])` returns `Err(1)`
    - `unimplemented_test_returns_error`: `run_with_args(["opal", "test"])` returns `Err(1)`
    - `unimplemented_doc_returns_error`: `run_with_args(["opal", "doc"])` returns `Err(1)`
    - `unimplemented_bench_returns_error`: `run_with_args(["opal", "bench"])` returns `Err(1)`
  - **Existing behavior preservation tests**:
    - `help_command_returns_ok`: `run_with_args(["opal", "help"])` returns `Ok(())`
    - `help_with_topic_returns_ok`: `run_with_args(["opal", "help", "pkg"])` returns `Ok(())`
    - `no_args_returns_error`: `run_with_args(["opal"])` returns `Err(1)`
    - `missing_file_returns_error`: `run_with_args(["opal", "nonexistent_file.op"])` returns `Err(1)`
  - Run `cargo test` and capture evidence that tests fail (red phase proof).

  **Must NOT do**:
  - Do NOT implement any changes to `help_text` or `run_with_args` yet — only write tests
  - Do NOT modify any production code
  - Do NOT write tests that test stdout/stderr text directly (test return values and `help_text` strings)

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: Extensive test suite requiring careful assertion design and understanding of dispatch semantics
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: NO
  - **Parallel Group**: Wave 2 (sequential with Task 3)
  - **Blocks**: Task 3
  - **Blocked By**: Task 1

  **References**:

  **Pattern References**:
  - `src/compiler.rs:302-354` — Existing `#[cfg(test)] mod tests` pattern in the codebase showing test module structure, assertion style, and naming convention
  - `src/app.rs` (after Task 1) — `help_text()` and `run_with_args()` function signatures to test against

  **API/Type References**:
  - `help_text(topic: Option<&str>) -> String` — returns help text string (from Task 1 refactoring)
  - `run_with_args(args: &[String]) -> Result<(), i32>` — returns Ok(()) for success, Err(code) for failure (from Task 1 refactoring)

  **WHY Each Reference Matters**:
  - `compiler.rs:302-354`: Shows the project's test naming convention (`snake_case_descriptive_name`), assertion pattern (`assert!(result.is_ok(), "message")`), and module placement
  - `app.rs` post-Task-1: The exact function signatures to call in tests

  **Acceptance Criteria**:

  - [ ] `#[cfg(test)] mod tests` module exists in `src/app.rs`
  - [ ] At least 20 test functions written
  - [ ] `cargo test` runs — new tests FAIL (this is expected in red phase)
  - [ ] Tests cover: help completeness (9 tests), `--help` alias (2 tests), unimplemented stubs (6 tests), existing behavior (4 tests)
  - [ ] Evidence captured showing test failures (red phase proof)

  **QA Scenarios:**

  ```
  Scenario: Red phase — new tests fail as expected
    Tool: Bash
    Preconditions: Tests written but no implementation changes
    Steps:
      1. Run `cargo test --lib -- app::tests 2>&1` and capture full output
      2. Count the number of FAILED tests
      3. Assert at least 15 tests fail (the new ones expecting expanded help)
    Expected Result: New tests fail, proving they test unimplemented behavior
    Failure Indicators: All tests pass (means tests are not asserting new behavior)
    Evidence: .sisyphus/evidence/task-2-red-phase.txt

  Scenario: Existing tests still pass
    Tool: Bash
    Preconditions: Tests written, no production code changed
    Steps:
      1. Run `cargo test --lib -- --skip app::tests 2>&1` and capture output
      2. Assert exit code is 0 and no failures outside `app::tests`
    Expected Result: All pre-existing tests still pass
    Failure Indicators: Any test outside app::tests fails
    Evidence: .sisyphus/evidence/task-2-existing-tests.txt
  ```

  **Commit**: NO (groups with Task 3)

- [x] 3. TDD GREEN — Implement full help expansion, `--help` alias, and subcommand stubs

  **What to do**:
  - **Expand `help_text(None)` (top-level help)** to show ALL commands and flags:
    ```
    Opalescent Compiler

    Usage:  opal <command> [arguments]

    Commands:
      <file.op>    Compile an Opalescent source file
      --run        Execute the compiled binary after compilation
      help         Show this help message
      --help       Alias for help
      pkg          Package manager commands
      fmt          Format Opalescent source files
      lsp          Start the language server
      test         Run project tests
      doc          Generate documentation
      bench        Run benchmarks

    Examples:
      opal src/main.op
      opal src/main.op --run
      opal help pkg
      opal --help fmt
    ```
  - **Add new help topics** by adding match arms in `help_text`:
    - `Some("lsp")`:
      ```
      opal lsp [options]

      Start the Opalescent language server.
        --stdio    Communicate over stdin/stdout (required for editor integration)
      ```
    - `Some("test")`:
      ```
      opal test [options]

      Run tests in the current project.
        --target <triple>     Run tests for a specific build target
        --filter <pattern>    Only run tests whose names contain <pattern>
      ```
    - `Some("doc")`:
      ```
      opal doc [options]

      Generate documentation for the current project.
        --format <md|html>    Output format (default: md)
      ```
    - `Some("bench")`:
      ```
      opal bench

      Run benchmarks in the current project.
      ```
  - **Add `--help` alias** in `run_with_args`:
    - After the existing `help` check (currently `args.get(1).map(String::as_str) == Some("help")`), add a check for `Some("--help")` that dispatches identically
    - `opal --help` → `help_text(None)` (same as `opal help`)
    - `opal --help pkg` → `help_text(Some("pkg"))` (full topic pass-through from `args.get(2)`)
  - **Add subcommand dispatch stubs** in `run_with_args`:
    - Between the help check and the file-path logic, add a match on `args.get(1)` for known subcommands: `"pkg"`, `"fmt"`, `"lsp"`, `"test"`, `"doc"`, `"bench"`
    - Each prints `eprintln!("error: '{}' command is not yet implemented", cmd)` and returns `Err(1)`
    - This MUST be checked BEFORE the file-path/compile logic to avoid `fs::read_to_string("pkg")`
  - **Fix `--run` flag isolation**: Move `--run` flag processing AFTER subcommand dispatch to prevent `opal pkg --run` from incorrectly setting `run_flag`
  - Run `cargo test` and verify ALL tests pass (green phase).

  **Must NOT do**:
  - Do NOT import or call into any other module (`package_manager`, `formatter`, `lsp`, etc.)
  - Do NOT implement actual subcommand logic
  - Do NOT change the Unknown help topic behavior (`help nonexistent` should still error)
  - Do NOT change help text for existing `pkg` and `fmt` topics (preserve exact text)

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: Core implementation task requiring careful dispatch ordering, edge case handling, and making 20+ tests pass
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: NO
  - **Parallel Group**: Wave 2 (sequential after Task 2)
  - **Blocks**: Tasks 4, 5
  - **Blocked By**: Task 2

  **References**:

  **Pattern References**:
  - `src/app.rs:15-56` — Current `print_help` match arms structure — new topics must follow this exact indentation and formatting pattern
  - `src/app.rs:34-37` — Unknown topic error handling (`Some(unknown)` arm) — preserve this arm, only add new `Some("lsp")` etc. arms before it
  - `src/app.rs:69-126` — Current `run_impl` dispatch flow — the ordering is: check help → collect flags → collect positional args → compile. New ordering must be: check help → check `--help` → check subcommands → collect flags → positional args → compile
  - `src/app.rs:78` — Current `--run` flag collection via `.any(|a| a == "--run")` — must be moved after subcommand dispatch

  **API/Type References**:
  - `src/testing/runner.rs:144-153` — `TestCommand` struct showing the test CLI options: `target: Option<BuildTarget>` and `name_filter: Option<String>` → these map to `--target <triple>` and `--filter <pattern>` flags
  - `src/doc_gen/renderer.rs:9-16` — `RenderFormat` enum: `Markdown`, `Html` → these map to `--format <md|html>` flag
  - `src/lsp/transport.rs` — stdio transport functions → confirms `--stdio` is the communication mode
  - `src/formatter/command.rs:13-29` — `FormatCommand` struct with `source` and `in_place` fields → `--check` flag corresponds to `!in_place`
  - `src/formatter/config.rs:13-38` — `FormatterConfig` struct → `--config` flag loads this from a TOML file

  **WHY Each Reference Matters**:
  - `app.rs:15-56`: The formatting pattern (alignment, spacing, println structure) must be consistent with new help text
  - `app.rs:34-37`: The `Some(unknown)` catch-all arm must remain LAST in the match, so new arms go before it
  - `app.rs:69-126`: Understanding the dispatch flow is critical — subcommand checks must be inserted at the right point
  - `testing/runner.rs:144-153`: Source of truth for what CLI flags the test subcommand should document
  - `doc_gen/renderer.rs:9-16`: Source of truth for the `--format` flag options
  - `lsp/transport.rs`: Confirms `--stdio` is the intended flag for LSP mode

  **Acceptance Criteria**:

  - [ ] All tests from Task 2 now PASS (green phase)
  - [ ] `cargo test` exits with 0 failures
  - [ ] `cargo clippy -- -D warnings` passes
  - [ ] `help_text(None)` contains all 10 commands/flags
  - [ ] `help_text(Some("lsp"))` contains `--stdio`
  - [ ] `help_text(Some("test"))` contains `--target` and `--filter`
  - [ ] `help_text(Some("doc"))` contains `--format`, `md`, `html`
  - [ ] `help_text(Some("bench"))` is non-empty
  - [ ] `run_with_args(["opal", "--help"])` returns `Ok(())`
  - [ ] `run_with_args(["opal", "--help", "pkg"])` returns `Ok(())`
  - [ ] `run_with_args(["opal", "pkg"])` returns `Err(1)`
  - [ ] Existing `opal help pkg` and `opal help fmt` text is UNCHANGED

  **QA Scenarios:**

  ```
  Scenario: Green phase — all tests pass
    Tool: Bash
    Preconditions: Implementation complete
    Steps:
      1. Run `cargo test --lib -- app::tests 2>&1` and capture output
      2. Assert exit code is 0
      3. Assert output contains "0 failed" or no "FAILED" lines
      4. Count test results — assert at least 20 tests ran
    Expected Result: All app::tests pass
    Failure Indicators: Any test failure
    Evidence: .sisyphus/evidence/task-3-green-phase.txt

  Scenario: Full regression — no existing tests broken
    Tool: Bash
    Preconditions: Implementation complete
    Steps:
      1. Run `cargo test 2>&1` and capture full output
      2. Assert exit code is 0
      3. Assert no "FAILED" lines in output
    Expected Result: Complete test suite passes
    Failure Indicators: Any existing test fails
    Evidence: .sisyphus/evidence/task-3-full-regression.txt

  Scenario: Clippy compliance maintained
    Tool: Bash
    Preconditions: Implementation complete
    Steps:
      1. Run `cargo clippy -- -D warnings 2>&1` and capture output
      2. Assert exit code is 0
    Expected Result: Zero clippy warnings
    Failure Indicators: Any clippy warning or error
    Evidence: .sisyphus/evidence/task-3-clippy.txt
  ```

  **Commit**: YES
  - Message: `feat(cli): expand help to full CLI surface with --help alias and subcommand stubs`
  - Files: `src/app.rs`
  - Pre-commit: `cargo test`

- [x] 4. TDD REFACTOR — Clean up implementation

  **What to do**:
  - Review `src/app.rs` for code quality after Tasks 1-3:
    - Remove any dead code or unused variables
    - Ensure consistent formatting in help text (alignment of command descriptions)
    - Verify all match arms have consistent structure
    - Check for any duplication that can be reduced
    - Ensure all doc comments (`///`) are present on functions
    - Verify `#![allow(...)]` and `#[expect(...)]` attributes are still appropriate
  - Run `cargo clippy -- -D warnings` to verify compliance
  - Run `cargo test` to verify no regressions from cleanup

  **Must NOT do**:
  - Do NOT change any test assertions or test behavior
  - Do NOT add new functionality
  - Do NOT change the help text content
  - Do NOT change any dispatch behavior

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Light cleanup pass over already-working code
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 3 (with Task 5)
  - **Blocks**: F1-F4
  - **Blocked By**: Task 3

  **References**:

  **Pattern References**:
  - `src/app.rs` (after Task 3) — the complete file to review for cleanup
  - `src/compiler.rs:1` — Example of module-level doc comment style (`//! ...`)
  - `src/main.rs:1` — Example of `#![allow(...)]` attribute with `reason` parameter

  **WHY Each Reference Matters**:
  - `app.rs`: The file being cleaned up
  - `compiler.rs:1`: Module doc comment convention to follow
  - `main.rs:1`: Attribute style with `reason` parameter (project convention)

  **Acceptance Criteria**:

  - [ ] `cargo test` passes with 0 failures
  - [ ] `cargo clippy -- -D warnings` passes
  - [ ] No dead code warnings
  - [ ] All functions have `///` doc comments
  - [ ] Help text alignment is visually consistent

  **QA Scenarios:**

  ```
  Scenario: Refactoring doesn't break anything
    Tool: Bash
    Preconditions: Cleanup applied to src/app.rs
    Steps:
      1. Run `cargo test 2>&1` and capture output
      2. Assert exit code is 0
      3. Run `cargo clippy -- -D warnings 2>&1` and capture output
      4. Assert exit code is 0
    Expected Result: All tests pass, zero clippy warnings
    Failure Indicators: Any test failure or clippy warning
    Evidence: .sisyphus/evidence/task-4-refactor.txt
  ```

  **Commit**: NO (amends previous commit if needed, otherwise no separate commit for cleanup)

- [x] 5. Update README CLI Reference section

  **What to do**:
  - Update the CLI Reference section of `README.md` (currently lines 88–174) to document the full CLI surface, matching the expanded help output.
  - **Update the top-level commands table** to include ALL commands:
    ```markdown
    | Command | Description |
    |---------|-------------|
    | `opal <file.op>` | Compile an Opalescent source file |
    | `opal <file.op> --run` | Compile and execute an Opalescent source file |
    | `opal help` | Show the top-level help message |
    | `opal --help` | Alias for `opal help` |
    | `opal help <topic>` | Show help for a specific command |
    | `opal pkg <command>` | Package manager commands |
    | `opal fmt [options] <file>` | Format an Opalescent source file |
    | `opal lsp [options]` | Start the language server |
    | `opal test [options]` | Run project tests |
    | `opal doc [options]` | Generate documentation |
    | `opal bench` | Run benchmarks |
    ```
  - **Add new subsections** for `lsp`, `test`, `doc`, `bench` (matching help topic content):
    - `### opal lsp — Language Server` with `--stdio` flag table
    - `### opal test — Test Runner` with `--target` and `--filter` flag tables
    - `### opal doc — Documentation Generator` with `--format` flag table
    - `### opal bench — Benchmarks` with usage description
  - **Update existing subsections** (`opal <file.op>`, `opal fmt`, `opal pkg`, `opal help`) to ensure accuracy:
    - `opal <file.op>` section: clarify that without `--run`, it compiles only (does NOT run). Add `--run` flag documentation.
    - `opal help` section: add `--help` as alias, list ALL available topics (pkg, fmt, lsp, test, doc, bench)
  - **Remove the disclaimer** on line 105: `"opal pkg, opal fmt, and opal lsp --stdio are currently documented interfaces, but they are not dispatched as executable subcommands in src/app.rs yet."` — Replace with a note that these commands will print an "unimplemented" error until fully wired.
  - **Update Quick Start section** (lines 42-55) to include `--help` example:
    ```bash
    # Get help
    ./target/release/opalescent help
    ./target/release/opalescent --help
    ```

  **Must NOT do**:
  - Do NOT modify any section of README.md outside of CLI Reference (lines 88–174), Quick Start (lines 42-55), and the disclaimer on line 105
  - Do NOT change Language Basics, Project Architecture, Compiler, Testing, or any other README section
  - Do NOT add implementation status notes to each command (keep it clean — users don't need to know internal wiring status)

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Documentation update with clear source content (help text) to translate into markdown tables
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 3 (with Task 4)
  - **Blocks**: F1-F4
  - **Blocked By**: Task 3

  **References**:

  **Pattern References**:
  - `README.md:88-174` — Current CLI Reference section structure (tables, subsections, code blocks) — follow this exact formatting pattern for new subsections
  - `README.md:96-104` — Current top-level commands table — extend with new rows
  - `README.md:119-140` — Current `opal fmt` subsection — use as template for new subsections (usage line, flag table, examples)
  - `README.md:142-165` — Current `opal pkg` subsection — shows command table pattern
  - `README.md:42-55` — Quick Start section — add `--help` example

  **API/Type References**:
  - `src/app.rs` (after Task 3) — `help_text()` function output — the source of truth for what commands/flags to document

  **WHY Each Reference Matters**:
  - `README.md:88-174`: The formatting, table style, and subsection structure must be consistent with existing content
  - `README.md:119-140`: The exact pattern (usage synopsis → flag table → examples) to replicate for `lsp`, `test`, `doc`, `bench`
  - `app.rs` help text: The README must document the same flags and descriptions as the help output — consistency is the goal

  **Acceptance Criteria**:

  - [ ] README CLI Reference top-level table lists ALL 11 rows (see table above)
  - [ ] New subsections exist: `opal lsp`, `opal test`, `opal doc`, `opal bench`
  - [ ] Each new subsection has: usage line, flag/option table (where applicable), examples
  - [ ] `--help` documented as alias
  - [ ] `--run` properly documented in `opal <file.op>` section
  - [ ] Old disclaimer (line 105) replaced or removed
  - [ ] Quick Start includes `--help` example
  - [ ] `cargo test` still passes (no changes to Rust code)

  **QA Scenarios:**

  ```
  Scenario: README contains all CLI commands
    Tool: Bash (grep)
    Preconditions: README.md updated
    Steps:
      1. Search README.md for "opal lsp" — assert found
      2. Search README.md for "opal test" — assert found
      3. Search README.md for "opal doc" — assert found
      4. Search README.md for "opal bench" — assert found
      5. Search README.md for "--help" — assert found
      6. Search README.md for "--run" — assert found
      7. Search README.md for "--stdio" — assert found
      8. Search README.md for "--target" — assert found
      9. Search README.md for "--filter" — assert found
      10. Search README.md for "--format" — assert found
    Expected Result: All commands and flags present in README
    Failure Indicators: Any search returns no results
    Evidence: .sisyphus/evidence/task-5-readme-coverage.txt

  Scenario: README doesn't break existing content
    Tool: Bash (grep)
    Preconditions: README.md updated
    Steps:
      1. Search README.md for "## Language Basics" — assert found (section boundary intact)
      2. Search README.md for "## Quick Start" — assert found
      3. Search README.md for "## Installation" — assert found
    Expected Result: All major sections still present
    Failure Indicators: Any section header missing
    Evidence: .sisyphus/evidence/task-5-readme-integrity.txt
  ```

  **Commit**: YES
  - Message: `docs(readme): update CLI Reference to match expanded help output`
  - Files: `README.md`
  - Pre-commit: `cargo test`

---

## Final Verification Wave

> 4 review agents run in PARALLEL. ALL must APPROVE. Present consolidated results to user and get explicit "okay" before completing.
>
> **Do NOT auto-proceed after verification. Wait for user's explicit approval before marking work complete.**

- [x] F1. **Plan Compliance Audit** — `oracle`
  Read the plan end-to-end. For each "Must Have": verify implementation exists (read file, run command). For each "Must NOT Have": search codebase for forbidden patterns — reject with file:line if found. Check evidence files exist in `.sisyphus/evidence/`. Compare deliverables against plan.
  Output: `Must Have [N/N] | Must NOT Have [N/N] | Tasks [N/N] | VERDICT: APPROVE/REJECT`

- [x] F2. **Code Quality Review** — `unspecified-high`
  Run `cargo test` + `cargo clippy`. Review `src/app.rs` changes for: `as any`/`@ts-ignore` equivalents, empty catches, `println!` in test code (should use returned strings), commented-out code, unused imports. Check AI slop: excessive comments, over-abstraction, generic names. Verify strict clippy passes (project uses extremely strict clippy config).
  Output: `Build [PASS/FAIL] | Clippy [PASS/FAIL] | Tests [N pass/N fail] | Files [N clean/N issues] | VERDICT`

- [x] F3. **Real Manual QA** — `unspecified-high`
  Build the binary with `cargo build`. Run each help command manually and verify output:
  - `./target/debug/opalescent help` — shows all commands
  - `./target/debug/opalescent --help` — identical to `help`
  - `./target/debug/opalescent --help pkg` — shows pkg help
  - `./target/debug/opalescent help lsp` — shows lsp help
  - `./target/debug/opalescent help test` — shows test help
  - `./target/debug/opalescent help doc` — shows doc help
  - `./target/debug/opalescent help bench` — shows bench help
  - `./target/debug/opalescent pkg` — prints error unimplemented
  - `./target/debug/opalescent fmt` — prints error unimplemented
  - `./target/debug/opalescent lsp` — prints error unimplemented
  - `./target/debug/opalescent test` — prints error unimplemented
  - `./target/debug/opalescent doc` — prints error unimplemented
  - `./target/debug/opalescent bench` — prints error unimplemented
  - `./target/debug/opalescent help nonexistent` — prints "Unknown help topic"
  Save evidence to `.sisyphus/evidence/final-qa/`.
  Output: `Scenarios [N/N pass] | Integration [N/N] | Edge Cases [N tested] | VERDICT`

- [x] F4. **Scope Fidelity Check** — `deep`
  For each task: read "What to do", read actual diff (`git diff`). Verify 1:1 — everything in spec was built (no missing), nothing beyond spec was built (no creep). Check "Must NOT do" compliance: no imports from `package_manager`, `formatter`, `lsp`, `testing`, `doc_gen`, `benchmarks`. Only `src/app.rs` and `README.md` modified. Flag unaccounted changes.
  Output: `Tasks [N/N compliant] | Contamination [CLEAN/N issues] | Unaccounted [CLEAN/N files] | VERDICT`

---

## Commit Strategy

- **After Task 1**: `refactor(cli): extract help text builder and args-based dispatch for testability`
  - Files: `src/app.rs`
  - Pre-commit: `cargo test`
- **After Task 3**: `feat(cli): expand help to full CLI surface with --help alias and subcommand stubs`
  - Files: `src/app.rs`
  - Pre-commit: `cargo test`
- **After Task 5**: `docs(readme): update CLI Reference to match expanded help output`
  - Files: `README.md`
  - Pre-commit: `cargo test`

---

## Success Criteria

### Verification Commands
```bash
cargo test                    # Expected: 0 failures, test count ≥ baseline + new tests
cargo clippy -- -D warnings   # Expected: 0 warnings (strict clippy config)
cargo build                   # Expected: successful build
```

### Final Checklist
- [ ] All "Must Have" present
- [ ] All "Must NOT Have" absent
- [ ] All tests pass
- [ ] README CLI Reference matches help output surface
- [ ] Existing CLI behavior unchanged
