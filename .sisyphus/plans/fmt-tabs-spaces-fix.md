# Fix `opal fmt` Tab/Spaces Handling + `--output` Flag

## TL;DR

> **Quick Summary**: Fix the `opal fmt` command to correctly handle tab↔space indentation conversion, add an `--output <file>` CLI flag for writing formatted output to a separate file, and verify everything with TDD golden-file tests.
> 
> **Deliverables**:
> - Working tab/space indentation conversion in the formatter
> - `--output <file>` CLI flag (mutually exclusive with `--check`)
> - Test project (`test-projects/fmt-test/`) with unformatted inputs and golden expected outputs
> - Comprehensive unit + integration tests covering the full tab/space matrix
> 
> **Estimated Effort**: Medium
> **Parallel Execution**: YES — 3 waves
> **Critical Path**: Task 1 (verify) → Task 4 (fix if needed) → Task 6 (golden files) → Task 7 (integration tests) → F1-F4

---

## Context

### Original Request
Fix the `opal fmt` command to have actual working tab/spaces handling. Set up a test project with an unformatted file that is formatted into a different file (via `--output`) so correctness can be checked repeatedly. Write tests.

### Interview Summary
**Key Discussions**:
- User wants "ultrawork" — intensive execution, minimal overhead
- Formatter may already handle tabs correctly via `Formatter::indent()` → `config.indent_unit()` → `"\t"` when `use_tabs=true`, but `rules::apply_all()` or `normalize_indentation()` may undo this
- `FormatCommand.in_place` is dead code — neither `execute()` nor `execute_with_config()` reads it
- Doc comments and `#` line comments are dropped by the formatter — test files must avoid doc comments

**Research Findings**:
- `normalize_indentation()` in `printer.rs` converts tabs→spaces pre-lex (correct for parsing, printer regenerates)
- `rules::apply_all()` takes no config — if it touches indentation, it cannot respect `use_tabs`
- `Formatter::indent()` correctly returns tabs when `use_tabs=true`
- Test projects show mixed styles: `hello-world` uses tabs, `fib-iterative` uses spaces

### Metis Review
**Identified Gaps** (addressed):
- Verification test MUST come first to determine if `rules::apply_all()` needs refactoring
- `--output` and `--check` must be mutually exclusive
- Test `.op` files must NOT contain doc comments (formatter drops them)
- `rules::apply_all()` signature must change to accept `&FormatterConfig` if indentation fix is needed

---

## Work Objectives

### Core Objective
Make `opal fmt` correctly convert between tabs and spaces based on `FormatterConfig`, add `--output` for redirected output, and prove it all works with repeatable tests.

### Concrete Deliverables
- `src/app.rs`: `--output <file>` flag support in `run_fmt_command()`
- `src/formatter/rules.rs`: Config-aware indentation handling (if needed — Task 1 determines this)
- `test-projects/fmt-test/`: Unformatted input files + golden expected output files
- `src/formatter/tests.rs`: Unit tests for tab/space conversion matrix
- Integration test: End-to-end `--output` flag verification

### Definition of Done
- [ ] `cargo test` passes — all existing + new unit tests green
- [ ] `cargo test --features integration` passes — all integration tests green
- [ ] `opal fmt --output out.op input.op` writes formatted output to `out.op`
- [ ] `opal fmt --output x --check y` produces an error (mutually exclusive)
- [ ] Formatting with `use_tabs=true` produces tab-indented output
- [ ] Formatting with `use_tabs=false` produces space-indented output
- [ ] Formatting mixed tab/space input normalizes to config setting

### Must Have
- Tab→space and space→tab conversion based on `FormatterConfig.use_tabs`
- `--output <file>` CLI flag
- `--output` + `--check` mutual exclusion with clear error message
- Golden-file test project for repeatable verification
- TDD: tests written before implementation (RED-GREEN-REFACTOR)

### Must NOT Have (Guardrails)
- Do NOT touch the lexer or parser
- Do NOT add auto-detection of indentation style
- Do NOT add doc comment preservation (separate feature)
- Do NOT modify whitespace inside string literals
- Do NOT add `HashMap` to core modules — use `BTreeMap` if needed
- Do NOT create files exceeding 1000 lines
- Test `.op` files must NOT contain doc comments (formatter drops them)

---

## Verification Strategy (MANDATORY)

> **ZERO HUMAN INTERVENTION** — ALL verification is agent-executed. No exceptions.

### Test Decision
- **Infrastructure exists**: YES (Rust `cargo test` + `cargo test --features integration`)
- **Automated tests**: TDD (RED-GREEN-REFACTOR)
- **Framework**: `cargo test` (Rust built-in) + integration feature flag
- **Each task follows**: RED (failing test) → GREEN (minimal impl) → REFACTOR

### QA Policy
Every task MUST include agent-executed QA scenarios.
Evidence saved to `.sisyphus/evidence/task-{N}-{scenario-slug}.{ext}`.

- **CLI**: Use Bash — run `opal fmt` commands, compare output files, assert exit codes
- **Unit tests**: Use Bash (`cargo test`) — run specific test functions, assert pass counts
- **File comparison**: Use Bash (`diff`) — compare formatted output against golden files

---

## Execution Strategy

### Parallel Execution Waves

```
Wave 1 (Start Immediately — verification + independent scaffolding):
├── Task 1: Verification test — does use_tabs=true already produce tabs? [deep]
├── Task 2: Add --output <file> CLI flag with --check mutual exclusion [unspecified-high]
└── Task 3: Create test-projects/fmt-test/ with unformatted .op input files [quick]

Wave 2 (After Wave 1 — fixes + test writing):
├── Task 4: [CONDITIONAL] Fix indentation handling if Task 1 reveals breakage [deep]
├── Task 5: Write comprehensive unit tests for tab/space matrix [unspecified-high]
└── Task 6: Create golden expected output files for test project [quick]

Wave 3 (After Wave 2 — integration + cleanup):
├── Task 7: Integration tests — --output flag + golden file comparison [unspecified-high]
└── Task 8: Clean up FormatCommand.in_place dead code [quick]

Wave FINAL (After ALL tasks — 4 parallel reviews, then user okay):
├── Task F1: Plan compliance audit (oracle)
├── Task F2: Code quality review (unspecified-high)
├── Task F3: Real manual QA (unspecified-high)
└── Task F4: Scope fidelity check (deep)
→ Present results → Get explicit user okay
```

### Dependency Matrix

| Task | Depends On | Blocks | Wave |
|------|-----------|--------|------|
| 1 | — | 4, 5, 6 | 1 |
| 2 | — | 7 | 1 |
| 3 | — | 6, 7 | 1 |
| 4 | 1 | 5, 6 | 2 |
| 5 | 1, 4 (if needed) | 7 | 2 |
| 6 | 3, 4 (if needed) | 7 | 2 |
| 7 | 2, 5, 6 | — | 3 |
| 8 | — | — | 3 |

### Agent Dispatch Summary

- **Wave 1**: **3 tasks** — T1 → `deep`, T2 → `unspecified-high`, T3 → `quick`
- **Wave 2**: **3 tasks** — T4 → `deep`, T5 → `unspecified-high`, T6 → `quick`
- **Wave 3**: **2 tasks** — T7 → `unspecified-high`, T8 → `quick`
- **FINAL**: **4 tasks** — F1 → `oracle`, F2 → `unspecified-high`, F3 → `unspecified-high`, F4 → `deep`

---

## TODOs

- [x] 1. Verification Test: Does `use_tabs=true` Already Produce Tab Output?

  **What to do**:
  - Write a unit test in `src/formatter/tests.rs` that creates a `FormatterConfig::new(4, 100, true)` (use_tabs=true)
  - Format a simple space-indented snippet through `FormatCommand::new(...).execute_with_config(config)`
  - Assert that the output lines use `\t` for indentation, NOT spaces
  - Also test the reverse: `FormatterConfig::default()` (use_tabs=false) with tab-indented input → output should use spaces
  - Run the test — if it PASSES, Task 4 can be skipped. If it FAILS, Task 4 is required
  - Record the result as a comment in the test: `// VERIFIED: use_tabs=true produces tabs` or `// VERIFIED: use_tabs=true does NOT produce tabs — fix needed`
  - TDD: Write the test FIRST (RED), then if it passes → GREEN with no changes needed. If it fails → leave it failing, Task 4 will fix it.

  **Must NOT do**:
  - Do NOT modify any formatter source code in this task — only write tests
  - Do NOT use doc comments in the test snippets (formatter drops them)

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: Requires understanding the full formatter pipeline to write meaningful verification tests
  - **Skills**: `[]`
  - **Skills Evaluated but Omitted**:
    - `playwright`: No browser UI involved

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1 (with Tasks 2, 3)
  - **Blocks**: Tasks 4, 5, 6 (they need to know if tabs work or not)
  - **Blocked By**: None (can start immediately)

  **References**:

  **Pattern References** (existing code to follow):
  - `src/formatter/tests.rs` — All existing formatter tests; follow the same assertion style and test naming conventions
  - `src/formatter/config.rs:FormatterConfig::new(indent_size, max_line_width, use_tabs)` — Constructor signature for creating test configs

  **API/Type References** (contracts to test against):
  - `src/formatter/command.rs:FormatCommand::execute_with_config(&self, config: FormatterConfig) -> FormatterResult<String>` — The method under test
  - `src/formatter/config.rs:FormatterConfig::default()` — Returns `indent_size=4, max_line_width=100, use_tabs=false`

  **Pipeline References** (understand the full flow):
  - `src/formatter/printer.rs:normalize_indentation()` — Pre-lex step: converts all tabs→spaces so the lexer can parse. Printer then regenerates indentation from AST
  - `src/formatter/printer.rs:Formatter::indent(&self, depth: usize)` — Returns `config.indent_unit().repeat(depth)` — should return `"\t".repeat(depth)` when `use_tabs=true`
  - `src/formatter/rules.rs:apply_all(source: &str)` — Post-processing rules applied AFTER printer output. Currently takes NO config — this is the suspected breakage point

  **WHY Each Reference Matters**:
  - `tests.rs`: Follow existing naming/assertion patterns so the new test fits in naturally
  - `FormatCommand::execute_with_config`: This is the public API being tested — the end-to-end entry point
  - `normalize_indentation`: Understanding this explains why tab input gets converted to spaces internally (pre-lex), which is correct
  - `Formatter::indent`: This confirms the printer SHOULD produce tabs — so if the final output has spaces, the bug is downstream
  - `rules::apply_all`: This is the prime suspect — it may strip tabs added by the printer

  **Acceptance Criteria**:

  **TDD (tests enabled):**
  - [ ] Test function `test_use_tabs_produces_tab_indentation` exists in `src/formatter/tests.rs`
  - [ ] Test function `test_tab_input_converted_to_spaces_by_default` exists in `src/formatter/tests.rs`
  - [ ] `cargo test test_use_tabs_produces_tab_indentation` → either PASS (tabs work) or FAIL (tabs broken — Task 4 needed)
  - [ ] `cargo test test_tab_input_converted_to_spaces_by_default` → either PASS or FAIL
  - [ ] Test result is recorded as a comment in the test file

  **QA Scenarios (MANDATORY):**

  ```
  Scenario: Verification test compiles and runs
    Tool: Bash (cargo test)
    Preconditions: Clean working tree, no pending changes to formatter
    Steps:
      1. Run `cargo test test_use_tabs_produces_tab_indentation -- --nocapture 2>&1`
      2. Capture stdout+stderr
      3. Check exit code (0=pass, 101=fail)
      4. Run `cargo test test_tab_input_converted_to_spaces_by_default -- --nocapture 2>&1`
      5. Capture stdout+stderr
    Expected Result: Both tests compile without errors. Results (pass/fail) are captured.
    Failure Indicators: Compilation error, panic outside assert, test harness failure
    Evidence: .sisyphus/evidence/task-1-verification-test.txt

  Scenario: No formatter source code was modified
    Tool: Bash (git diff)
    Preconditions: Tests written
    Steps:
      1. Run `git diff --name-only src/formatter/` — should show ONLY `tests.rs`
      2. Run `git diff --name-only src/formatter/printer.rs src/formatter/rules.rs src/formatter/command.rs src/formatter/config.rs` — should be empty
    Expected Result: Only `src/formatter/tests.rs` is modified. No other formatter files changed.
    Failure Indicators: Any file other than tests.rs appears in the diff
    Evidence: .sisyphus/evidence/task-1-no-source-changes.txt
  ```

  **Commit**: YES
  - Message: `test(formatter): add verification test for use_tabs output`
  - Files: `src/formatter/tests.rs`
  - Pre-commit: `cargo test`

- [x] 3. Create `test-projects/fmt-test/` with Unformatted Input Files

  **What to do**:
  - Create directory structure: `test-projects/fmt-test/src/`, `test-projects/fmt-test/expected/`
  - Create `test-projects/fmt-test/opal.toml` with `name = "fmt-test"` and `version = "0.1.0"`
  - Create `test-projects/fmt-test/opal-fmt-tabs.toml` with `use_tabs = true`, `indent_size = 4`, `max_line_width = 100`
  - Create `test-projects/fmt-test/opal-fmt-2spaces.toml` with `use_tabs = false`, `indent_size = 2`, `max_line_width = 100`
  - Create `test-projects/fmt-test/.gitignore` with `target/`
  - Create input files (deliberately messy formatting, NO doc comments):
    1. `src/input-spaces.op` — Valid Opalescent code indented with 4 spaces, but with inconsistent trailing whitespace, extra blank lines, mixed CRLF/LF — tests that default config cleans it up
    2. `src/input-tabs.op` — Same logic but indented with tabs, some trailing spaces — tests tab→space conversion with default config
    3. `src/input-mixed.op` — Mix of tabs and spaces for indentation — tests normalization
    4. `src/input-clean-spaces.op` — Already perfectly formatted with 4-space indent — tests idempotency
    5. `src/input-clean-tabs.op` — Already perfectly formatted with tab indent — tests idempotency with `use_tabs=true`
  - All input files must use ONLY `entry main`, simple `let` bindings, `if`/`return`, `print()` — no doc comments, no complex features
  - Files must be valid Opalescent that the formatter can parse (no syntax errors)

  **Must NOT do**:
  - Do NOT include doc comments (`## ... ##`) in any test `.op` file
  - Do NOT create expected output files yet (Task 6 does that after formatting is verified/fixed)
  - Do NOT use complex language features that might trigger other formatter bugs

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: File creation only, no logic needed, just writing `.op` and `.toml` files
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1 (with Tasks 1, 2)
  - **Blocks**: Tasks 6, 7 (need input files to exist)
  - **Blocked By**: None (can start immediately)

  **References**:

  **Pattern References** (existing test projects to follow):
  - `test-projects/hello-world/` — Directory structure pattern: `opal.toml`, `.gitignore`, `src/main.op`
  - `test-projects/hello-world/opal.toml` — TOML format for project config
  - `test-projects/hello-world/src/main.op` — Example of tab-indented `.op` code WITHOUT doc comments (uses simple `entry main`)
  - `test-projects/fib-iterative/src/main.op` — Example of space-indented `.op` code with `let`, `if`, `while`, `return`

  **Config References**:
  - `src/formatter/config.rs:FormatterConfig::from_toml_str()` — Parses `opal-fmt.toml` files; the TOML keys are `indent_size`, `max_line_width`, `use_tabs`

  **WHY Each Reference Matters**:
  - `hello-world/`: Copy this exact project structure (opal.toml, .gitignore, src/) for the new test project
  - `hello-world/src/main.op`: This file uses tabs and NO doc comments — safe pattern to copy for tab-indented input
  - `fib-iterative/src/main.op`: Uses spaces and shows `if`/`while`/`let mutable` — good complexity level for test inputs
  - `FormatterConfig::from_toml_str()`: Confirms the exact TOML key names to use in config files

  **Acceptance Criteria**:

  **QA Scenarios (MANDATORY):**

  ```
  Scenario: All input files exist and are valid
    Tool: Bash
    Preconditions: Task completed
    Steps:
      1. Run `ls -la test-projects/fmt-test/src/` — verify all 5 input files exist
      2. Run `ls -la test-projects/fmt-test/opal.toml test-projects/fmt-test/opal-fmt-tabs.toml test-projects/fmt-test/opal-fmt-2spaces.toml` — verify configs exist
      3. Run `test -d test-projects/fmt-test/expected && echo "EXISTS" || echo "MISSING"` — verify expected/ directory exists (empty is OK)
      4. For each input file, run `cargo run -- check test-projects/fmt-test/src/<file>` or verify the formatter can at least lex/parse it without crashing
    Expected Result: All files exist. All input files parseable by the formatter.
    Failure Indicators: Missing files, parse errors, directory structure wrong
    Evidence: .sisyphus/evidence/task-3-files-exist.txt

  Scenario: Input files have expected formatting defects
    Tool: Bash (grep)
    Preconditions: Files created
    Steps:
      1. Run `grep -P '\t' test-projects/fmt-test/src/input-tabs.op | head -3` — verify tabs present
      2. Run `grep -P '^\s{4}' test-projects/fmt-test/src/input-spaces.op | head -3` — verify spaces present
      3. Run `grep -P '[\t ]+$' test-projects/fmt-test/src/input-spaces.op` — verify trailing whitespace exists
      4. Run `grep -cP '\t' test-projects/fmt-test/src/input-mixed.op` — verify tabs in mixed file
      5. Run `grep -cP '^    ' test-projects/fmt-test/src/input-mixed.op` — verify spaces in mixed file
    Expected Result: Tab files have tabs, space files have spaces, mixed has both, trailing whitespace exists where expected.
    Failure Indicators: Input files are already clean (defeats the purpose of testing)
    Evidence: .sisyphus/evidence/task-3-formatting-defects.txt

  Scenario: No doc comments in any input file
    Tool: Bash (grep)
    Preconditions: Files created
    Steps:
      1. Run `grep -r '##' test-projects/fmt-test/src/*.op` — should return NO matches
    Expected Result: Zero matches. No doc comments in any input file.
    Failure Indicators: Any `##` found in input files
    Evidence: .sisyphus/evidence/task-3-no-doc-comments.txt
  ```

  **Commit**: YES
  - Message: `test(fmt-test): add unformatted input files for formatter testing`
  - Files: `test-projects/fmt-test/**`
  - Pre-commit: —

- [x] 2. Add `--output <file>` CLI Flag with `--check` Mutual Exclusion

  **What to do**:
  - TDD RED: Write a test (in `src/formatter/tests.rs` or an appropriate test location) that verifies:
    - `opal fmt --output out.op input.op` writes formatted output to `out.op` instead of overwriting `input.op`
    - `opal fmt --check --output out.op input.op` prints an error message and returns exit code 1
  - TDD GREEN: Modify `run_fmt_command()` in `src/app.rs` to:
    1. Parse `--output` flag and extract the output path (same pattern as `--config` parsing)
    2. Check mutual exclusion: if both `--check` and `--output` are present, print error and return `Err(1)`
    3. When `--output` is set (and not `--check`): write formatted output to the output path instead of the source path
    4. When `--output` is NOT set: existing behavior unchanged (write in-place)
  - TDD REFACTOR: Clean up any duplication introduced

  **Must NOT do**:
  - Do NOT change `FormatCommand` struct or its methods — the `--output` flag is purely CLI-level I/O routing
  - Do NOT modify the formatter core logic
  - Do NOT add `--output` to `FormatCommand.in_place` — that field is dead code (Task 8 removes it)

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
    - Reason: Moderate complexity CLI modification touching `app.rs` with clear patterns to follow
  - **Skills**: `[]`
  - **Skills Evaluated but Omitted**:
    - `playwright`: No browser UI

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1 (with Tasks 1, 3)
  - **Blocks**: Task 7 (integration tests use `--output`)
  - **Blocked By**: None (can start immediately)

  **References**:

  **Pattern References** (existing code to follow):
  - `src/app.rs:254-320` — `run_fmt_command()`: The function to modify. Follow the exact pattern used for `--config` flag parsing (lines 257-260) to add `--output` parsing
  - `src/app.rs:256` — `--check` flag parsing: `let check_mode = fmt_args.contains(&"--check");` — simple boolean flag pattern
  - `src/app.rs:257-260` — `--config` flag parsing with value extraction: positional lookup + `get(i+1)` pattern — use identical approach for `--output`
  - `src/app.rs:261-264` — Source path extraction: Filters out flag arguments. Must also filter out `--output` and its value argument

  **API/Type References**:
  - `std::fs::write(path, contents)` — Used at line 314 to write output; same function for `--output` path
  - `src/app.rs:307-313` — Check mode logic: If `--check` and `--output` are both set, error before reaching this block

  **Test References**:
  - No existing CLI integration tests for `run_fmt_command` — this task creates the first ones

  **WHY Each Reference Matters**:
  - `run_fmt_command()` lines 254-320: This is the ONLY function to modify. Every pattern (flag parsing, error handling, file I/O) is already demonstrated here
  - `--config` parsing pattern: Copy this exact pattern for `--output` — same positional argument with value extraction
  - Source path extraction: Must be updated to also exclude `--output` and its value from the source path search
  - `fs::write`: Same function used for output, just with a different path argument

  **Acceptance Criteria**:

  **TDD (tests enabled):**
  - [ ] Test for `--output` flag basic operation exists and passes
  - [ ] Test for `--check` + `--output` mutual exclusion exists and passes
  - [ ] `cargo test` → all existing tests still pass (no regressions)

  **QA Scenarios (MANDATORY):**

  ```
  Scenario: --output flag writes to specified file
    Tool: Bash
    Preconditions: A valid .op file exists at a known path (e.g., test-projects/hello-world/src/main.op)
    Steps:
      1. Run `cargo build 2>&1` — verify build succeeds
      2. Run `./target/debug/opalescent fmt test-projects/hello-world/src/main.op --output /tmp/fmt-output-test.op 2>&1`
      3. Check exit code is 0
      4. Run `test -f /tmp/fmt-output-test.op && echo "EXISTS" || echo "MISSING"`
      5. Run `cat /tmp/fmt-output-test.op` — verify it contains formatted Opalescent code
      6. Verify source file is unchanged: `git diff test-projects/hello-world/src/main.op` should be empty
    Expected Result: Exit code 0. `/tmp/fmt-output-test.op` exists and contains formatted code. Source file unmodified.
    Failure Indicators: Non-zero exit code, output file missing, source file modified
    Evidence: .sisyphus/evidence/task-2-output-flag.txt

  Scenario: --check and --output are mutually exclusive
    Tool: Bash
    Preconditions: Build succeeded
    Steps:
      1. Run `./target/debug/opalescent fmt --check --output /tmp/x.op test-projects/hello-world/src/main.op 2>&1`
      2. Capture stderr
      3. Check exit code is 1 (not 0)
      4. Verify stderr contains an error message about mutual exclusion (e.g., "cannot use --check and --output together")
      5. Verify `/tmp/x.op` does NOT exist
    Expected Result: Exit code 1. Stderr contains mutual exclusion error. No output file created.
    Failure Indicators: Exit code 0, no error message, output file created
    Evidence: .sisyphus/evidence/task-2-mutual-exclusion.txt

  Scenario: Existing behavior preserved (no --output flag)
    Tool: Bash
    Preconditions: Copy a test file to /tmp first to avoid modifying repo
    Steps:
      1. Run `cp test-projects/hello-world/src/main.op /tmp/fmt-inplace-test.op`
      2. Run `./target/debug/opalescent fmt /tmp/fmt-inplace-test.op 2>&1`
      3. Check exit code is 0
      4. Verify `/tmp/fmt-inplace-test.op` was modified in-place (file exists, content is formatted)
    Expected Result: Exit code 0. File formatted in-place. No separate output file.
    Failure Indicators: Non-zero exit code, file unchanged, crash
    Evidence: .sisyphus/evidence/task-2-inplace-preserved.txt
  ```

  **Commit**: YES
  - Message: `feat(cli): add --output flag to opal fmt command`
  - Files: `src/app.rs`
  - Pre-commit: `cargo test`

- [x] 4. [CONDITIONAL] Fix Indentation Handling to Respect `FormatterConfig` — **SKIPPED**: Task 1 verified use_tabs=true already produces tab output correctly

  > **CONDITIONAL**: Only execute this task if Task 1's verification test FAILED (i.e., `use_tabs=true` does NOT produce tab-indented output). If Task 1's test passed, mark this task as SKIPPED and proceed to Task 5.

  **What to do**:
  - Diagnose exactly WHERE tab indentation is lost by examining Task 1's test failure output
  - The likely fix: modify `rules::apply_all()` in `src/formatter/rules.rs` to accept `&FormatterConfig` and pass it through to any rule that touches indentation
  - Specifically: `remove_trailing_whitespace()` calls `line.trim_end()` which strips trailing tabs — this is correct (trailing whitespace removal). BUT if any rule normalizes LEADING whitespace, it needs to know `use_tabs`
  - Update `format_source()` in `src/formatter/printer.rs` to pass config to `rules::apply_all()` (it already has `&self.config`)
  - Update all callers of `apply_all()` to pass config — search with `lsp_find_references`
  - TDD: Task 1's failing test becomes the RED test. Make it GREEN by implementing the fix. Then REFACTOR.
  - Run the full test suite to ensure no regressions

  **Must NOT do**:
  - Do NOT modify the lexer or parser
  - Do NOT change `normalize_indentation()` behavior — it correctly converts tabs→spaces pre-lex; the printer regenerates indentation afterward
  - Do NOT touch whitespace inside string literals
  - Do NOT add auto-detection of indentation style
  - Do NOT change `FormatterConfig` struct — only pass it to functions that need it

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: Requires understanding the full formatter pipeline and making a targeted fix without breaking the indentation flow
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES (with Tasks 5, 6 — but 5 and 6 depend on this task's outcome)
  - **Parallel Group**: Wave 2 (with Tasks 5, 6 — but 5 and 6 start after this if it was needed)
  - **Blocks**: Tasks 5, 6 (they need correct formatting to generate golden files and write passing tests)
  - **Blocked By**: Task 1 (must know whether fix is needed)

  **References**:

  **Pattern References**:
  - `src/formatter/rules.rs:26-32` — `apply_all()`: Current signature `pub fn apply_all(source: &str) -> String`. Must change to `pub fn apply_all(source: &str, config: &FormatterConfig) -> String`
  - `src/formatter/rules.rs:43-49` — `remove_trailing_whitespace()`: Uses `trim_end()` which strips trailing tabs/spaces. This is correct behavior — trailing whitespace should always be removed regardless of config

  **API/Type References**:
  - `src/formatter/config.rs:FormatterConfig` — The config struct with `use_tabs` field; must be passed to `apply_all()`
  - `src/formatter/config.rs:FormatterConfig::indent_unit(&self) -> String` — Returns `"\t"` or `" ".repeat(indent_size)`. Use this in any indentation normalization rule
  - `src/formatter/printer.rs:Formatter::format_source()` — Calls `rules::apply_all()`. Has `&self.config` available to pass

  **Diagnostic References**:
  - Task 1's test output — Shows exact expected vs actual output, revealing where tabs are lost

  **WHY Each Reference Matters**:
  - `apply_all()` signature: This is the most likely change needed — adding `config: &FormatterConfig` parameter
  - `remove_trailing_whitespace()`: Confirms this rule is NOT the problem (trim_end is correct)
  - `format_source()`: This is where `apply_all()` is called — must be updated to pass config
  - `indent_unit()`: If a new indentation normalization rule is needed, use this method to get the correct indent string

  **Acceptance Criteria**:

  **TDD (tests enabled):**
  - [ ] Task 1's `test_use_tabs_produces_tab_indentation` now PASSES (was RED, now GREEN)
  - [ ] Task 1's `test_tab_input_converted_to_spaces_by_default` still PASSES
  - [ ] `cargo test` → all existing tests pass (zero regressions)

  **QA Scenarios (MANDATORY):**

  ```
  Scenario: use_tabs=true produces tab-indented output
    Tool: Bash (cargo test)
    Preconditions: Fix applied to rules/printer
    Steps:
      1. Run `cargo test test_use_tabs_produces_tab_indentation -- --nocapture 2>&1`
      2. Verify exit code is 0 (test passes)
      3. Run `cargo test test_tab_input_converted_to_spaces_by_default -- --nocapture 2>&1`
      4. Verify exit code is 0 (test passes)
    Expected Result: Both tests pass. Tab/space conversion works correctly in both directions.
    Failure Indicators: Any test failure, compilation error
    Evidence: .sisyphus/evidence/task-4-tabs-work.txt

  Scenario: No regressions in existing formatter tests
    Tool: Bash (cargo test)
    Preconditions: Fix applied
    Steps:
      1. Run `cargo test formatter 2>&1` — run all formatter-related tests
      2. Count pass/fail
      3. Verify zero failures
    Expected Result: All existing formatter tests pass. Zero regressions.
    Failure Indicators: Any pre-existing test now fails
    Evidence: .sisyphus/evidence/task-4-no-regressions.txt

  Scenario: apply_all signature updated correctly
    Tool: Bash (grep)
    Preconditions: Fix applied
    Steps:
      1. Run `grep 'pub fn apply_all' src/formatter/rules.rs` — verify config parameter added
      2. Run `cargo clippy -- -D warnings 2>&1` — verify no clippy warnings from the change
    Expected Result: `apply_all` signature includes config parameter. Clippy clean.
    Failure Indicators: Old signature still present, clippy warnings
    Evidence: .sisyphus/evidence/task-4-signature-updated.txt
  ```

  **Commit**: YES
  - Message: `fix(formatter): make rules respect FormatterConfig for indentation`
  - Files: `src/formatter/rules.rs`, `src/formatter/printer.rs`
  - Pre-commit: `cargo test`

- [x] 5. Write Comprehensive Unit Tests for Tab/Space Conversion Matrix

  **What to do**:
  - In `src/formatter/tests.rs`, add a test module or section for indentation conversion tests
  - Write tests covering the FULL matrix (all must pass):
    1. `test_tabs_to_spaces_default_config` — Tab-indented input + default config → 4-space output
    2. `test_spaces_to_tabs` — Space-indented input + `use_tabs=true` config → tab output
    3. `test_mixed_to_spaces` — Mixed tab+space input + default config → 4-space output
    4. `test_mixed_to_tabs` — Mixed tab+space input + `use_tabs=true` → tab output
    5. `test_idempotent_spaces` — Already 4-space input + default config → identical output (idempotent)
    6. `test_idempotent_tabs` — Already tab-indented + `use_tabs=true` → identical output (idempotent)
    7. `test_custom_indent_size_2` — Any input + `indent_size=2, use_tabs=false` → 2-space output
    8. `test_custom_indent_size_8` — Any input + `indent_size=8, use_tabs=false` → 8-space output
    9. `test_nested_indentation_tabs` — Multi-level nesting (if inside if) + `use_tabs=true` → correct tab depth
    10. `test_nested_indentation_spaces` — Multi-level nesting + default config → correct space depth
  - TDD: Write tests FIRST (RED if Task 4 fix was needed and not yet applied, GREEN if formatter already works)
  - All test snippets must be valid Opalescent WITHOUT doc comments
  - Use multi-level nesting to verify depth handling (at least 2 levels of indentation)

  **Must NOT do**:
  - Do NOT modify formatter source code — tests only
  - Do NOT use doc comments in test snippets
  - Do NOT test comment preservation (known broken, out of scope)

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
    - Reason: Test writing that requires understanding the formatter API but no deep architectural analysis
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2 (with Tasks 4, 6)
  - **Blocks**: Task 7 (integration tests build on unit test confidence)
  - **Blocked By**: Task 1 (need to know what works), Task 4 (if fix was needed)

  **References**:

  **Pattern References**:
  - `src/formatter/tests.rs` — Existing test patterns; follow naming convention `test_<what_is_tested>`
  - `src/formatter/tests.rs` — Look at how existing tests create input strings and assert output; use the same `FormatCommand::new(...).execute_with_config(config)` pattern

  **API/Type References**:
  - `src/formatter/command.rs:FormatCommand::execute_with_config(&self, config: FormatterConfig)` — Entry point for all tests
  - `src/formatter/config.rs:FormatterConfig::new(indent_size: usize, max_line_width: usize, use_tabs: bool)` — Constructor for custom configs
  - `src/formatter/config.rs:FormatterConfig::default()` — `indent_size=4, max_line_width=100, use_tabs=false`

  **Test Input References** (valid Opalescent code to use in tests):
  - `test-projects/hello-world/src/main.op` — Simple valid code with entry main, tabs
  - `test-projects/fib-iterative/src/main.op` — Valid code with nested control flow (`if`, `while`), spaces

  **WHY Each Reference Matters**:
  - Existing tests: Must match naming convention and assertion patterns for consistency
  - `execute_with_config`: Every test calls this — it's the API under test
  - `FormatterConfig::new()`: Needed to create custom configs for each matrix cell
  - Test project files: Source of valid Opalescent snippets safe to use in tests (no doc comments)

  **Acceptance Criteria**:

  **TDD (tests enabled):**
  - [ ] All 10 test functions exist in `src/formatter/tests.rs`
  - [ ] `cargo test` → all 10 tests pass
  - [ ] Tests cover: tabs→spaces, spaces→tabs, mixed→both, idempotent×2, custom sizes×2, nested×2

  **QA Scenarios (MANDATORY):**

  ```
  Scenario: All matrix tests pass
    Tool: Bash (cargo test)
    Preconditions: Tasks 1 and 4 (if needed) completed
    Steps:
      1. Run `cargo test test_tabs_to_spaces_default_config test_spaces_to_tabs test_mixed_to_spaces test_mixed_to_tabs test_idempotent_spaces test_idempotent_tabs test_custom_indent_size_2 test_custom_indent_size_8 test_nested_indentation_tabs test_nested_indentation_spaces -- --nocapture 2>&1`
      2. Count "test result: ok" lines
      3. Verify 10 tests passed, 0 failed
    Expected Result: `test result: ok. 10 passed; 0 failed`
    Failure Indicators: Any test failure, compilation error, fewer than 10 tests found
    Evidence: .sisyphus/evidence/task-5-matrix-tests.txt

  Scenario: No regressions in existing tests
    Tool: Bash (cargo test)
    Steps:
      1. Run `cargo test 2>&1 | tail -5`
      2. Verify total test count matches or exceeds previous count
      3. Verify 0 failures
    Expected Result: All tests pass. No regressions.
    Failure Indicators: Any failure in pre-existing tests
    Evidence: .sisyphus/evidence/task-5-no-regressions.txt
  ```

  **Commit**: YES
  - Message: `test(formatter): add comprehensive tab/space conversion test matrix`
  - Files: `src/formatter/tests.rs`
  - Pre-commit: `cargo test`

- [x] 6. Create Golden Expected Output Files for Test Project

  **What to do**:
  - For each input file in `test-projects/fmt-test/src/`, generate the correct expected output and save to `test-projects/fmt-test/expected/`:
    1. `expected/input-spaces.expected.op` — Result of formatting `input-spaces.op` with default config (4 spaces)
    2. `expected/input-tabs.expected.op` — Result of formatting `input-tabs.op` with default config (4 spaces) — tabs converted to spaces
    3. `expected/input-tabs-to-tabs.expected.op` — Result of formatting `input-tabs.op` with `opal-fmt-tabs.toml` (use_tabs=true) — tabs preserved
    4. `expected/input-mixed.expected.op` — Result of formatting `input-mixed.op` with default config (4 spaces)
    5. `expected/input-mixed-to-tabs.expected.op` — Result of formatting `input-mixed.op` with `opal-fmt-tabs.toml` (use_tabs=true)
    6. `expected/input-clean-spaces.expected.op` — Should be identical to input (idempotent)
    7. `expected/input-clean-tabs.expected.op` — Result of formatting `input-clean-tabs.op` with `opal-fmt-tabs.toml` (should be identical)
    8. `expected/input-spaces-2indent.expected.op` — Result of formatting `input-spaces.op` with `opal-fmt-2spaces.toml` (2-space indent)
  - Generate these by RUNNING the formatter: `cargo run -- fmt --output <expected-path> [--config <config>] <input-path>` for each combination
  - Verify each golden file by visual inspection (read the file, check indentation is correct)
  - These golden files are the "expected" half of golden-file testing — Task 7's integration tests will compare formatter output against these

  **Must NOT do**:
  - Do NOT hand-write the expected files — generate them with the (now-working) formatter
  - Do NOT include files that would expose comment-dropping bugs (no doc comments in inputs)

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Mechanical file generation — run formatter, save output, verify
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2 (with Tasks 4, 5)
  - **Blocks**: Task 7 (golden files needed for integration tests)
  - **Blocked By**: Task 3 (input files must exist), Task 4 (if fix was needed, formatter must be correct first)

  **References**:

  **Input References**:
  - `test-projects/fmt-test/src/input-spaces.op` — Created in Task 3
  - `test-projects/fmt-test/src/input-tabs.op` — Created in Task 3
  - `test-projects/fmt-test/src/input-mixed.op` — Created in Task 3
  - `test-projects/fmt-test/src/input-clean-spaces.op` — Created in Task 3
  - `test-projects/fmt-test/src/input-clean-tabs.op` — Created in Task 3

  **Config References**:
  - `test-projects/fmt-test/opal-fmt-tabs.toml` — Created in Task 3, `use_tabs=true`
  - `test-projects/fmt-test/opal-fmt-2spaces.toml` — Created in Task 3, `indent_size=2`

  **CLI References**:
  - `src/app.rs:254-320` — `run_fmt_command()` with `--output` flag (added in Task 2)

  **WHY Each Reference Matters**:
  - Input files: These are the source inputs that the formatter will process to generate golden files
  - Config files: Different configs produce different expected outputs (tabs vs spaces vs 2-space)
  - `--output` flag: Used to generate golden files without overwriting inputs

  **Acceptance Criteria**:

  **QA Scenarios (MANDATORY):**

  ```
  Scenario: All golden files exist and are non-empty
    Tool: Bash
    Preconditions: Tasks 2, 3, 4 completed
    Steps:
      1. Run `ls -la test-projects/fmt-test/expected/` — verify all 8 expected files exist
      2. For each file, run `wc -l test-projects/fmt-test/expected/<file>` — verify non-empty (>0 lines)
    Expected Result: 8 golden files exist, all non-empty.
    Failure Indicators: Missing files, empty files
    Evidence: .sisyphus/evidence/task-6-golden-files-exist.txt

  Scenario: Golden files have correct indentation style
    Tool: Bash (grep)
    Steps:
      1. `grep -P '^\t' test-projects/fmt-test/expected/input-tabs-to-tabs.expected.op | head -3` — tabs present in tab-config output
      2. `grep -P '^\t' test-projects/fmt-test/expected/input-spaces.expected.op | wc -l` — should be 0 (no tabs in space output)
      3. `grep -P '^  [^ ]' test-projects/fmt-test/expected/input-spaces-2indent.expected.op | head -3` — 2-space indent present
    Expected Result: Tab files use tabs, space files use spaces, 2-space files use 2 spaces.
    Failure Indicators: Wrong indentation style in any golden file
    Evidence: .sisyphus/evidence/task-6-golden-indentation.txt

  Scenario: Idempotent files match input
    Tool: Bash (diff)
    Steps:
      1. Run `diff test-projects/fmt-test/src/input-clean-spaces.op test-projects/fmt-test/expected/input-clean-spaces.expected.op`
      2. Run `diff test-projects/fmt-test/src/input-clean-tabs.op test-projects/fmt-test/expected/input-clean-tabs.expected.op`
    Expected Result: Both diffs are empty — idempotent files are unchanged by formatting.
    Failure Indicators: Diff output shows changes (formatting is not idempotent)
    Evidence: .sisyphus/evidence/task-6-idempotent.txt
  ```

  **Commit**: YES
  - Message: `test(fmt-test): add golden expected output files`
  - Files: `test-projects/fmt-test/expected/**`
  - Pre-commit: —

- [x] 7. Integration Tests: `--output` Flag + Golden File Comparison

  **What to do**:
  - Write integration tests (gated behind `#[cfg(feature = "integration")]`) that:
    1. Build the binary with `cargo build`
    2. Run `opalescent fmt --output <tmpdir>/out.op test-projects/fmt-test/src/input-spaces.op`
    3. Compare `<tmpdir>/out.op` with `test-projects/fmt-test/expected/input-spaces.expected.op` using string comparison
    4. Repeat for each input/config/expected combination:
       - Default config: `input-spaces.op` → `input-spaces.expected.op`
       - Default config: `input-tabs.op` → `input-tabs.expected.op` (tabs→spaces)
       - Tabs config: `input-tabs.op` + `--config opal-fmt-tabs.toml` → `input-tabs-to-tabs.expected.op`
       - Default config: `input-mixed.op` → `input-mixed.expected.op`
       - Tabs config: `input-mixed.op` + `--config opal-fmt-tabs.toml` → `input-mixed-to-tabs.expected.op`
       - Default config: `input-clean-spaces.op` → `input-clean-spaces.expected.op` (idempotent)
       - Tabs config: `input-clean-tabs.op` + `--config opal-fmt-tabs.toml` → `input-clean-tabs.expected.op` (idempotent)
       - 2-space config: `input-spaces.op` + `--config opal-fmt-2spaces.toml` → `input-spaces-2indent.expected.op`
    5. Test `--check` + `--output` mutual exclusion: run with both flags, assert exit code 1 and stderr contains error
    6. Test `--output` preserves source file: verify source file hash before and after is identical
  - Place tests in an appropriate integration test file (e.g., `tests/fmt_integration.rs` or add to existing integration test structure)
  - Use `std::process::Command` to invoke the binary, `tempfile` or manual tmpdir for outputs
  - TDD: Write tests FIRST (should pass immediately since all prior tasks are done)

  **Must NOT do**:
  - Do NOT modify formatter source code
  - Do NOT modify the golden files
  - Do NOT hardcode absolute paths — use `env!("CARGO_MANIFEST_DIR")` for project root

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
    - Reason: Integration test writing with process spawning and file comparison
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES (with Task 8)
  - **Parallel Group**: Wave 3 (with Task 8)
  - **Blocks**: None
  - **Blocked By**: Tasks 2 (--output flag), 5 (unit tests confirm correctness), 6 (golden files exist)

  **References**:

  **Pattern References**:
  - `tests/` directory — Check for existing integration test files and follow their patterns
  - `src/app.rs:254-320` — `run_fmt_command()`: Understanding what the CLI does helps write correct integration tests

  **Test Project References**:
  - `test-projects/fmt-test/src/*.op` — Input files (created Task 3)
  - `test-projects/fmt-test/expected/*.op` — Golden expected files (created Task 6)
  - `test-projects/fmt-test/opal-fmt-tabs.toml` — Tabs config (created Task 3)
  - `test-projects/fmt-test/opal-fmt-2spaces.toml` — 2-space config (created Task 3)

  **Integration Test Pattern References**:
  - Search for existing `#[cfg(feature = "integration")]` tests in the codebase — follow their structure for process invocation, tmpdir handling, and assertion patterns

  **WHY Each Reference Matters**:
  - Existing integration tests: Must match the project's integration test conventions (feature flag, file organization, assertion style)
  - `run_fmt_command()`: Understanding the CLI's behavior ensures tests invoke it correctly
  - Golden files: These are the "expected" side of every test assertion

  **Acceptance Criteria**:

  **TDD (tests enabled):**
  - [ ] Integration test file exists with all 8 golden-file comparisons + mutual exclusion test + source preservation test
  - [ ] `cargo test --features integration` → all new integration tests pass
  - [ ] No regressions in existing integration tests

  **QA Scenarios (MANDATORY):**

  ```
  Scenario: All integration tests pass
    Tool: Bash (cargo test)
    Preconditions: All prior tasks completed, binary builds
    Steps:
      1. Run `cargo test --features integration fmt 2>&1` (filter to fmt-related integration tests)
      2. Count pass/fail
      3. Verify zero failures
    Expected Result: All fmt integration tests pass.
    Failure Indicators: Any test failure, compilation error, golden file mismatch
    Evidence: .sisyphus/evidence/task-7-integration-tests.txt

  Scenario: Golden file comparison catches intentional breakage
    Tool: Bash
    Preconditions: Tests pass normally
    Steps:
      1. Temporarily corrupt a golden file: `echo "BROKEN" >> test-projects/fmt-test/expected/input-spaces.expected.op`
      2. Run `cargo test --features integration fmt 2>&1`
      3. Verify at least one test FAILS (proving golden comparison works)
      4. Restore the golden file: `git checkout test-projects/fmt-test/expected/input-spaces.expected.op`
      5. Run tests again — verify they pass
    Expected Result: Tests correctly detect golden file mismatch and fail. After restore, tests pass again.
    Failure Indicators: Tests pass despite corrupted golden file (comparison is broken)
    Evidence: .sisyphus/evidence/task-7-golden-validation.txt
  ```

  **Commit**: YES
  - Message: `test(formatter): add integration tests for --output flag and golden files`
  - Files: `tests/fmt_integration.rs` (or equivalent)
  - Pre-commit: `cargo test --features integration`

- [x] 8. Clean Up `FormatCommand.in_place` Dead Code

  **What to do**:
  - Remove the `in_place` field from `FormatCommand` struct in `src/formatter/command.rs`
  - Remove `in_place` parameter from `FormatCommand::new()` constructor
  - Update all callers of `FormatCommand::new()` to remove the `in_place` argument:
    - `src/app.rs` — `run_fmt_command()` calls `FormatCommand::new(source.clone(), false)` at lines 291 and 299
    - Any test files that construct `FormatCommand`
  - Use `lsp_find_references` on `FormatCommand::new` to find ALL callers
  - Run `cargo test` to verify no regressions

  **Must NOT do**:
  - Do NOT change any formatter behavior — this is purely dead code removal
  - Do NOT add new functionality in this task

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Simple dead code removal with mechanical changes across a few files
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES (with Task 7)
  - **Parallel Group**: Wave 3 (with Task 7)
  - **Blocks**: None
  - **Blocked By**: None (independent of other tasks, but placed in Wave 3 to avoid conflicts with Task 2's changes to `app.rs`)

  **References**:

  **Code References**:
  - `src/formatter/command.rs:22-30` — `FormatCommand` struct with `in_place: bool` field to remove
  - `src/formatter/command.rs:34-37` — `FormatCommand::new(source, in_place)` constructor to simplify
  - `src/app.rs:291` — `FormatCommand::new(source.clone(), false)` — remove `false` arg
  - `src/app.rs:299` — `FormatCommand::new(source.clone(), false)` — remove `false` arg

  **WHY Each Reference Matters**:
  - `command.rs` struct: The dead field to remove
  - `command.rs` constructor: Must update signature
  - `app.rs` callers: Must update call sites. Use `lsp_find_references` to find any others

  **Acceptance Criteria**:

  **QA Scenarios (MANDATORY):**

  ```
  Scenario: in_place field fully removed
    Tool: Bash (grep)
    Steps:
      1. Run `grep -r 'in_place' src/formatter/command.rs` — should return NO matches
      2. Run `grep -r 'in_place' src/` — should return NO matches (unless in unrelated code)
    Expected Result: Zero references to `in_place` in formatter code.
    Failure Indicators: Any remaining reference to `in_place`
    Evidence: .sisyphus/evidence/task-8-dead-code-removed.txt

  Scenario: All tests still pass after removal
    Tool: Bash (cargo test)
    Steps:
      1. Run `cargo test 2>&1 | tail -5`
      2. Verify 0 failures
      3. Run `cargo clippy -- -D warnings 2>&1 | tail -10`
      4. Verify 0 warnings
    Expected Result: All tests pass, clippy clean.
    Failure Indicators: Compilation error, test failure
    Evidence: .sisyphus/evidence/task-8-no-regressions.txt
  ```

  **Commit**: YES
  - Message: `refactor(formatter): remove dead in_place field from FormatCommand`
  - Files: `src/formatter/command.rs`, `src/app.rs`, any test files
  - Pre-commit: `cargo test`

---

## Final Verification Wave (MANDATORY — after ALL implementation tasks)

> 4 review agents run in PARALLEL. ALL must APPROVE. Present consolidated results to user and get explicit "okay" before completing.
>
> **Do NOT auto-proceed after verification. Wait for user's explicit approval before marking work complete.**

- [x] F1. **Plan Compliance Audit** — `oracle`
  Read the plan end-to-end. For each "Must Have": verify implementation exists (read file, run command). For each "Must NOT Have": search codebase for forbidden patterns — reject with file:line if found. Check evidence files exist in `.sisyphus/evidence/`. Compare deliverables against plan.
  Output: `Must Have [N/N] | Must NOT Have [N/N] | Tasks [N/N] | VERDICT: APPROVE/REJECT`

- [x] F2. **Code Quality Review** — `unspecified-high`
  Run `cargo clippy -- -D warnings` + `cargo test` + `cargo test --features integration`. Review all changed files for: `as any` equivalents, empty error handling, `println!` in prod code paths, commented-out code, unused imports. Check AI slop: excessive comments, over-abstraction, generic names. Verify files stay under 1000 lines (`scripts/check-line-count.sh`).
  Output: `Build [PASS/FAIL] | Clippy [PASS/FAIL] | Tests [N pass/N fail] | Files [N clean/N issues] | VERDICT`

- [x] F3. **Real Manual QA** — `unspecified-high`
  Start from clean state. Build with `cargo build`. Run these commands:
  1. `opal fmt test-projects/fmt-test/src/input-spaces.op --output /tmp/out.op` — verify output matches golden file
  2. `opal fmt --config test-projects/fmt-test/opal-fmt-tabs.toml test-projects/fmt-test/src/input-tabs.op --output /tmp/out-tabs.op` — verify tab output
  3. `opal fmt --check --output /tmp/x test-projects/fmt-test/src/input-spaces.op` — verify mutual exclusion error
  4. Run `diff` between each output and corresponding golden expected file
  Save evidence to `.sisyphus/evidence/final-qa/`.
  Output: `Scenarios [N/N pass] | Integration [N/N] | Edge Cases [N tested] | VERDICT`

- [x] F4. **Scope Fidelity Check** — `deep`
  For each task: read "What to do", read actual diff (`git diff`). Verify 1:1 — everything in spec was built, nothing beyond spec was built. Check "Must NOT do" compliance (no lexer/parser changes, no auto-detection, no doc comment preservation). Detect cross-task contamination. Flag unaccounted changes.
  Output: `Tasks [N/N compliant] | Contamination [CLEAN/N issues] | Unaccounted [CLEAN/N files] | VERDICT`

---

## Commit Strategy

| After Task | Commit Message | Files | Pre-commit Check |
|-----------|---------------|-------|-----------------|
| 1 | `test(formatter): add verification test for use_tabs output` | `src/formatter/tests.rs` | `cargo test` |
| 2 | `feat(cli): add --output flag to opal fmt command` | `src/app.rs` | `cargo test` |
| 3 | `test(fmt-test): add unformatted input files for formatter testing` | `test-projects/fmt-test/**` | — |
| 4 | `fix(formatter): make rules respect FormatterConfig for indentation` | `src/formatter/rules.rs`, `src/formatter/printer.rs` | `cargo test` |
| 5 | `test(formatter): add comprehensive tab/space conversion test matrix` | `src/formatter/tests.rs` | `cargo test` |
| 6 | `test(fmt-test): add golden expected output files` | `test-projects/fmt-test/**` | — |
| 7 | `test(formatter): add integration tests for --output flag and golden files` | `tests/` or `src/formatter/tests.rs` | `cargo test --features integration` |
| 8 | `refactor(formatter): remove dead in_place field from FormatCommand` | `src/formatter/command.rs` | `cargo test` |

---

## Success Criteria

### Verification Commands
```bash
cargo test                           # Expected: all tests pass
cargo test --features integration    # Expected: all integration tests pass
cargo clippy -- -D warnings          # Expected: no warnings
scripts/check-line-count.sh          # Expected: all files under 1000 lines
```

### Final Checklist
- [ ] All "Must Have" present (tab/space conversion, --output flag, mutual exclusion, golden files, TDD)
- [ ] All "Must NOT Have" absent (no lexer/parser changes, no auto-detection, no doc comment work)
- [ ] All unit tests pass
- [ ] All integration tests pass
- [ ] Clippy clean
- [ ] No files over 1000 lines
