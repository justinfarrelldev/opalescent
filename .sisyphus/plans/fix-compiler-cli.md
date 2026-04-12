# Fix Compiler CLI: Use CLI Arguments Instead of Hardcoded Files

## TL;DR

> **Quick Summary**: Replace the hardcoded `language-spec/hello_world.op` path in the compiler CLI with proper CLI argument handling, so `opal src/main.op` actually compiles the specified file. Add `--run` flag for optional binary execution.
> 
> **Deliverables**:
> - Rewritten `run()` function in `src/app.rs` that reads the user-specified `.op` file
> - Full compile pipeline wired up via existing `compile_program()`
> - `--run` flag to optionally execute compiled binaries
> - Updated help text reflecting new behavior
> - Warmup probe calls removed from CLI hot path
> 
> **Estimated Effort**: Quick
> **Parallel Execution**: NO — single file change
> **Critical Path**: Task 1 (single task)

---

## Context

### Original Request
The Opalescent compiler ignores the file argument passed on the command line and instead hardcodes `language-spec/hello_world.op`. Running `opal src/main.op` from `test-projects/hello-world/` prints:
```
Opalescent Parser Test
Failed to read hello_world.op: No such file or directory (os error 2)
```
The user wants the compiler to actually compile the file they specify, and wants all hardcoded file references removed from the functional code path.

### Interview Summary
**Key Discussions**:
- **Warmup probes**: User asked to evaluate `touch_doc_gen_api_for_lints()` and `touch_error_api_for_lints()` — these are lint workarounds exercising dead code paths at runtime. Verified they serve no user purpose and all APIs they exercise have other callers (in tests/extractors). Safe to remove from CLI path.
- **Auto-execute**: User wants compile-only as default, with `--run` flag to optionally execute the compiled binary.
- **Output directory**: User wants `./target/` (Rust convention, CWD-relative).

**Research Findings**:
- `compile_program()` in `src/compiler.rs` is fully built and tested — signature: `pub fn compile_program(source: &str, output_dir: &Path) -> Result<PathBuf, CompileError>`
- Integration tests (`tests/integration_e2e.rs`) demonstrate the correct pattern: `read_to_string → compile_program → Command::new(binary).output()`
- `compile_program()` hardcodes `runtime/opal_runtime.c` as a relative path — this is a pre-existing limitation, NOT in scope for this task
- Strict clippy lint environment — all items need doc comments, no `unwrap()`/`expect()`, must use `Result` types

### Metis Review
**Identified Gaps** (addressed):
- **Dead imports**: After removing the manual lex/parse pipeline, imports of `crate::ast`, `crate::lexer::Lexer`, `crate::parser::Parser` become dead. Plan explicitly includes import cleanup.
- **Exit codes**: Current `run()` returns `()` with no exit codes. Plan includes `std::process::exit()` for proper error signaling.
- **Flag position**: `--run` should be accepted in any position (`opal --run file.op` or `opal file.op --run`). Plan uses flag-filtering approach.
- **Help text update**: "Compile and run" in help should become "Compile" with separate `--run` documentation.
- **Runtime path landmine**: `compile_program()` uses relative `runtime/opal_runtime.c` — documented as known limitation, explicitly OUT of scope.

---

## Work Objectives

### Core Objective
Replace the stub `run()` function in `src/app.rs` with proper CLI dispatch that compiles the user-specified `.op` file using the existing `compile_program()` pipeline, placing artifacts in `./target/`.

### Concrete Deliverables
- Rewritten `src/app.rs:run()` function
- Updated `src/app.rs:print_help()` text
- Clean imports (dead ones removed, needed ones added)

### Definition of Done
- [ ] `opal src/main.op` compiles the specified file (no hardcoded paths)
- [ ] `opal src/main.op --run` compiles AND executes the binary
- [ ] `opal help` shows updated help text with `--run` flag
- [ ] `cargo build` succeeds with zero warnings
- [ ] All existing tests pass (`cargo test`)

### Must Have
- CLI argument used as the source file path (not hardcoded)
- Proper error messages printed to stderr for missing files and compilation failures
- Non-zero exit codes on failure
- `--run` flag support (any position)
- Updated help text

### Must NOT Have (Guardrails)
- Do NOT modify `src/compiler.rs` — pipeline is correct as-is
- Do NOT rename output binary from `program` — integration tests depend on this name
- Do NOT fix the `runtime/opal_runtime.c` relative path issue — pre-existing bug, separate task
- Do NOT add `clap` or any arg-parsing crate — CLI is intentionally hand-rolled
- Do NOT remove the `touch_doc_gen_api_for_lints` and `touch_error_api_for_lints` function DEFINITIONS from their modules — only remove the CALLS from `run()`
- Do NOT update README.md — separate documentation task
- Do NOT add excessive error-handling boilerplate or over-abstraction
- Do NOT add unit tests for `run()` — integration tests already cover the pipeline

---

## Verification Strategy (MANDATORY)

> **ZERO HUMAN INTERVENTION** - ALL verification is agent-executed. No exceptions.

### Test Decision
- **Infrastructure exists**: YES (cargo test, integration feature flag)
- **Automated tests**: None additional needed — existing integration tests cover `compile_program()` pipeline
- **Framework**: cargo test

### QA Policy
Every task MUST include agent-executed QA scenarios.
Evidence saved to `.sisyphus/evidence/task-{N}-{scenario-slug}.{ext}`.

- **CLI**: Use Bash — Run compiler commands, assert exit codes, check stdout/stderr output
- **Build**: Use Bash — Run `cargo build`, `cargo test`, `cargo clippy`

---

## Execution Strategy

### Parallel Execution Waves

```
Wave 1 (Single task — this is a simple focused change):
└── Task 1: Rewrite src/app.rs run() function [quick]

Wave FINAL (After Task 1):
├── Task F1: Plan compliance audit (oracle)
├── Task F2: Code quality review (unspecified-high)
├── Task F3: Real manual QA (unspecified-high)
└── Task F4: Scope fidelity check (deep)
-> Present results -> Get explicit user okay
```

### Dependency Matrix

| Task | Depends On | Blocks |
|------|-----------|--------|
| 1 | None | F1-F4 |
| F1-F4 | 1 | user okay |

### Agent Dispatch Summary

- **Wave 1**: **1 task** — T1 → `quick`
- **Wave FINAL**: **4 tasks** — F1 → `oracle`, F2 → `unspecified-high`, F3 → `unspecified-high`, F4 → `deep`

---

## TODOs

- [x] 1. Rewrite `src/app.rs` — Proper CLI dispatch with `compile_program()`

  **What to do**:
  1. **Update imports**: Remove dead imports (`crate::ast`, `crate::lexer::Lexer`, `crate::parser::Parser`). Add needed imports (`crate::compiler::compile_program`, `std::path::Path`, `std::process::Command`). Keep `std::fs` (still needed for `read_to_string`).
  2. **Update `print_help(None)` branch**: Change `"Compile and run an Opalescent source file"` to `"Compile an Opalescent source file"`. Add a line for `--run` flag: `"  --run         Execute the compiled binary after compilation"`. Update the example to show both modes.
  3. **Rewrite `run()` body** — replace everything after the `help` check with proper CLI dispatch:
     - Remove both warmup probe calls (`touch_doc_gen_api_for_lints()` and `touch_error_api_for_lints()`) and their warning prints
     - Remove the `println!("Opalescent Parser Test")` debug banner
     - Remove the entire `match fs::read_to_string("language-spec/hello_world.op")` block
     - Parse CLI args: collect all args, separate flags (`--run`) from positional args (the `.op` file path)
     - Accept `--run` in any position (before or after the file argument)
     - If no file argument provided: print error to stderr, show help suggestion, exit with code 1
     - Read the user-specified source file with `fs::read_to_string()`. On failure: print descriptive error to stderr, exit with code 1
     - Call `compile_program(&source, Path::new("target"))`. On failure: print the `CompileError` to stderr, exit with code 1
     - On success: print the path to the compiled binary to stdout
     - If `--run` flag present: execute the compiled binary via `Command::new(&binary_path).status()`, forward the binary's exit code as the process exit code
  4. **Follow code conventions**: Add `///` doc comments to any new helper functions. Use `eprintln!()` for errors (not `println!()`). Do not use `unwrap()`/`expect()` — use pattern matching or `if let`.

  **Must NOT do**:
  - Do NOT modify `src/compiler.rs`
  - Do NOT modify `src/main.rs` (keep `run()` returning `()`, use `std::process::exit()` internally)
  - Do NOT add `clap` or any external arg parsing crate
  - Do NOT rename output binary from `program`
  - Do NOT remove the probe function DEFINITIONS from `doc_gen.rs` or `errors.rs`
  - Do NOT add unit tests — integration tests cover this

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Single-file rewrite with clear instructions. ~80 lines of straightforward Rust. No architectural decisions.
  - **Skills**: []
    - No specialized skills needed — this is vanilla Rust CLI code
  - **Skills Evaluated but Omitted**:
    - `git-master`: Not needed — simple single-file change

  **Parallelization**:
  - **Can Run In Parallel**: NO — single task
  - **Parallel Group**: Wave 1 (solo)
  - **Blocks**: F1-F4
  - **Blocked By**: None

  **References** (CRITICAL):

  **Pattern References** (existing code to follow):
  - `tests/integration_e2e.rs:184-220` — The canonical read → compile → execute pattern to replicate in `run()`. Shows `fs::read_to_string()` → `compile_program()` → `Command::new()` flow.
  - `src/app.rs:16-55` — Existing `print_help()` function. Follow its formatting style when updating help text.
  - `src/app.rs:58-135` — Current `run()` function to be fully rewritten. Understand what exists before replacing.

  **API/Type References** (contracts to implement against):
  - `src/compiler.rs:184` — `compile_program(source: &str, output_dir: &Path) -> Result<PathBuf, CompileError>` — the function to call. Returns `Ok(binary_path)` where `binary_path` is `output_dir/program`.
  - `src/compiler.rs:29-52` — `CompileError` enum — implements `Display` and `Error`. Can be printed with `{error}` in format strings.

  **WHY Each Reference Matters**:
  - `integration_e2e.rs` shows the EXACT pattern to copy — don't reinvent the flow
  - `compiler.rs:184` tells you the exact function signature and what it returns
  - `compiler.rs:29-52` tells you the error type implements `Display`, so `eprintln!("Compilation failed: {error}")` works

  **Acceptance Criteria**:

  **QA Scenarios (MANDATORY):**

  ```
  Scenario: Compile a valid .op file (happy path — compile only)
    Tool: Bash
    Preconditions: Project is built (`cargo build`), `language-spec/hello_world.op` exists
    Steps:
      1. Run: `cargo build 2>&1` — assert exit code 0
      2. Run: `./target/debug/opalescent language-spec/hello_world.op` — assert exit code 0
      3. Assert: stdout contains path to compiled binary (e.g., "target/program")
      4. Assert: file `target/program` exists and is executable
    Expected Result: Exit code 0, binary produced at target/program
    Failure Indicators: Non-zero exit code, "hello_world.op" in error output, "Parser Test" in output
    Evidence: .sisyphus/evidence/task-1-compile-happy.txt

  Scenario: Compile and run with --run flag (flag after file)
    Tool: Bash
    Preconditions: Project is built, `language-spec/hello_world.op` exists
    Steps:
      1. Run: `./target/debug/opalescent language-spec/hello_world.op --run`
      2. Assert: stdout contains "Hello world" (from the compiled program's execution)
      3. Assert: exit code 0
    Expected Result: Binary is compiled AND executed, stdout shows program output
    Failure Indicators: No "Hello world" in output, non-zero exit code
    Evidence: .sisyphus/evidence/task-1-compile-and-run.txt

  Scenario: Compile and run with --run flag (flag before file)
    Tool: Bash
    Preconditions: Project is built, `language-spec/hello_world.op` exists
    Steps:
      1. Run: `./target/debug/opalescent --run language-spec/hello_world.op`
      2. Assert: stdout contains "Hello world"
      3. Assert: exit code 0
    Expected Result: Same behavior regardless of flag position
    Failure Indicators: Flag not recognized, error about "--run" being a file
    Evidence: .sisyphus/evidence/task-1-run-flag-before.txt

  Scenario: Missing file argument (error case)
    Tool: Bash
    Preconditions: Project is built
    Steps:
      1. Run: `./target/debug/opalescent 2>&1`
      2. Assert: stderr contains error message about missing file argument
      3. Assert: exit code is non-zero (1)
    Expected Result: Helpful error message on stderr, non-zero exit
    Failure Indicators: Exit code 0, no error message, crash/panic
    Evidence: .sisyphus/evidence/task-1-no-args-error.txt

  Scenario: Nonexistent file (error case)
    Tool: Bash
    Preconditions: Project is built, `nonexistent.op` does NOT exist
    Steps:
      1. Run: `./target/debug/opalescent nonexistent.op 2>&1`
      2. Assert: stderr contains error about file not found
      3. Assert: exit code is non-zero (1)
    Expected Result: Clear error message mentioning the file name, non-zero exit
    Failure Indicators: Exit code 0, panic, generic error without file name
    Evidence: .sisyphus/evidence/task-1-missing-file-error.txt

  Scenario: Help command still works (regression check)
    Tool: Bash
    Preconditions: Project is built
    Steps:
      1. Run: `./target/debug/opalescent help`
      2. Assert: stdout contains "Opalescent Compiler"
      3. Assert: stdout contains "--run"
      4. Assert: stdout contains "Compile"
      5. Assert: exit code 0
    Expected Result: Updated help text showing --run flag and "Compile" (not "Compile and run")
    Failure Indicators: Missing --run documentation, still says "Compile and run"
    Evidence: .sisyphus/evidence/task-1-help-regression.txt

  Scenario: Build and lint pass (code quality)
    Tool: Bash
    Preconditions: None
    Steps:
      1. Run: `cargo build 2>&1` — assert exit code 0, no warnings
      2. Run: `cargo test 2>&1` — assert exit code 0, all tests pass
      3. Run: `cargo clippy -- -D warnings 2>&1` — assert exit code 0
    Expected Result: Clean build, all tests pass, no clippy warnings
    Failure Indicators: Dead import warnings, missing doc comments, test failures
    Evidence: .sisyphus/evidence/task-1-build-lint.txt

  Scenario: No hardcoded hello_world.op in functional code (verification)
    Tool: Bash
    Preconditions: Task implementation is complete
    Steps:
      1. Run: `grep -n "hello_world" src/app.rs`
      2. Assert: The ONLY match (if any) is in the help text example (line ~51), NOT in any `fs::read_to_string` or functional path
      3. Assert: No match for `"language-spec/hello_world.op"` in `src/app.rs`
    Expected Result: Zero references to hardcoded hello_world.op in functional code paths
    Failure Indicators: Any grep match outside of help text/comments
    Evidence: .sisyphus/evidence/task-1-no-hardcoded-paths.txt
  ```

  **Evidence to Capture:**
  - [ ] task-1-compile-happy.txt — compile-only run output
  - [ ] task-1-compile-and-run.txt — --run flag after file
  - [ ] task-1-run-flag-before.txt — --run flag before file
  - [ ] task-1-no-args-error.txt — no args error output
  - [ ] task-1-missing-file-error.txt — nonexistent file error
  - [ ] task-1-help-regression.txt — help command output
  - [ ] task-1-build-lint.txt — cargo build/test/clippy output
  - [ ] task-1-no-hardcoded-paths.txt — grep verification

  **Commit**: YES
  - Message: `fix(cli): use CLI file argument instead of hardcoded hello_world.op path`
  - Files: `src/app.rs`
  - Pre-commit: `cargo build && cargo test`

---

## Final Verification Wave (MANDATORY — after ALL implementation tasks)

> 4 review agents run in PARALLEL. ALL must APPROVE. Present consolidated results to user and get explicit "okay" before completing.

- [x] F1. **Plan Compliance Audit** — `oracle`
  Read the plan end-to-end. For each "Must Have": verify implementation exists (read file, run command). For each "Must NOT Have": search codebase for forbidden patterns — reject with file:line if found. Check evidence files exist in .sisyphus/evidence/. Compare deliverables against plan.
  Output: `Must Have [N/N] | Must NOT Have [N/N] | Tasks [N/N] | VERDICT: APPROVE/REJECT`

- [x] F2. **Code Quality Review** — `unspecified-high`
  Run `cargo build`, `cargo clippy -- -D warnings`, `cargo test`. Review `src/app.rs` for: `as any`/`unwrap`/`expect`, empty catches, `println!` for errors (should be `eprintln!`), commented-out code, unused imports. Check for AI slop: excessive comments, over-abstraction, generic names.
  Output: `Build [PASS/FAIL] | Lint [PASS/FAIL] | Tests [N pass/N fail] | Files [N clean/N issues] | VERDICT`

- [x] F3. **Real Manual QA** — `unspecified-high`
  Start from clean state. Execute EVERY QA scenario from Task 1 — follow exact steps, capture evidence. Test cross-scenario interactions. Save to `.sisyphus/evidence/final-qa/`.
  Output: `Scenarios [N/N pass] | Edge Cases [N tested] | VERDICT`

- [x] F4. **Scope Fidelity Check** — `deep`
  For Task 1: read "What to do", read actual diff (`git diff`). Verify 1:1 — everything in spec was built (no missing), nothing beyond spec was built (no creep). Check "Must NOT do" compliance: `compiler.rs` unchanged, `main.rs` unchanged, no new dependencies in Cargo.toml. Flag unaccounted changes.
  Output: `Tasks [N/N compliant] | Unaccounted [CLEAN/N files] | VERDICT`

---

## Commit Strategy

| Task | Message | Files | Pre-commit |
|------|---------|-------|------------|
| 1 | `fix(cli): use CLI file argument instead of hardcoded hello_world.op path` | `src/app.rs` | `cargo build && cargo test` |

---

## Success Criteria

### Verification Commands
```bash
# Compile a file (should work with ANY .op file, not just hello_world)
./target/debug/opalescent language-spec/hello_world.op
# Expected: exit 0, binary at target/program

# Compile and run
./target/debug/opalescent language-spec/hello_world.op --run
# Expected: stdout contains "Hello world"

# From test-projects directory (the original failing case)
cd test-projects/hello-world && ../../target/debug/opalescent src/main.op
# Expected: exit 0 (may fail on runtime path — known pre-existing limitation)

# Error handling
./target/debug/opalescent nonexistent.op
# Expected: error on stderr, exit 1

# No args
./target/debug/opalescent
# Expected: error on stderr, exit 1

# Help
./target/debug/opalescent help
# Expected: shows --run flag, says "Compile" not "Compile and run"
```

### Final Checklist
- [ ] All "Must Have" present
- [ ] All "Must NOT Have" absent
- [ ] All tests pass (`cargo test`)
- [ ] No hardcoded file paths in functional code
- [ ] `--run` flag works in any position
- [ ] Proper exit codes (0 success, 1 failure)
- [ ] Errors go to stderr, not stdout
