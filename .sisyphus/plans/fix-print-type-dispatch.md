# Fix print() Type-Based Dispatch Bug

## TL;DR

> **Quick Summary**: Fix `print()` to dispatch to the correct typed runtime function (`print_int32`, `print_float64`, etc.) based on the LLVM type of the lowered argument, instead of unconditionally calling `puts(i8*)` which causes UB/segfault for non-string arguments.
> 
> **Deliverables**:
> - Type-based dispatch logic in `codegen_call_expression` for `print` calls
> - Integration test for existing `should-print-final-result` test project
> - New `print-types` test project covering int, float, bool, and string direct printing
> 
> **Estimated Effort**: Short
> **Parallel Execution**: YES - 2 waves
> **Critical Path**: Task 1 → Task 2 → Task 3 → F1-F4

---

## Context

### Original Request
Fix a bug where calling `print()` with a non-string argument (e.g. `print(add(3, 4))` where `add` returns `int32`) produces no output / silently crashes. The function `print` is declared as generic `print<T>(value: T): unit` in the type checker, but codegen unconditionally maps it to C's `puts(i8*)`, causing undefined behavior when the argument is not a string pointer.

### Interview Summary
**Key Discussions**:
- Root cause confirmed: `declare_stdlib_function` maps `"print"` → `puts(i8*)` unconditionally (line ~70 of `functions_stdlib.rs`)
- Runtime already has all typed print functions (`print_int8/16/32/64`, `print_uint8/16/32/64`, `print_float32/64`, `print_string`)
- `lower_interpolation_argument` in `expressions_string.rs` is the proven pattern for LLVM type-based dispatch
- Signed/unsigned cannot be distinguished at LLVM level (both are `iN`) — defaulting to signed is consistent with string interpolation behavior

**Research Findings**:
- No existing special cases in `codegen_call_expression` for function names — this would be the first
- `test-projects/should-print-final-result/src/main.op` has `print(add(num1_int, num2_int))` but no integration test
- All passing integration tests use string interpolation or explicit `*_to_string()` — none test direct non-string printing
- `bool_to_string` in `runtime/opal_string.c` uses `strdup()` — **heap-allocates**, so `free()` is needed after `puts`

### Metis Review
**Identified Gaps** (addressed):
- Bool memory management: `bool_to_string` heap-allocates via `strdup()` → must `free()` after `puts`
- `resolve_print_to_puts` test should NOT be modified (resolution layer unchanged, dispatch is downstream)
- Return type difference (`puts` returns `i32`, `print_*` returns `void`) handled by existing `try_as_basic_value()` fallback
- No `NodeId→CoreType` metadata bridge exists between TypeChecker and Codegen — LLVM type inspection is the only option
- Callee must be detected via AST `Expr::Identifier`, not `FunctionValue.get_name()` (which returns `"puts"`)

---

## Work Objectives

### Core Objective
Make `print()` correctly output values of any primitive type (string, int, float, bool) by dispatching to the appropriate typed runtime function based on the LLVM type of the lowered argument.

### Concrete Deliverables
- Modified `src/codegen/functions_call.rs`: type-based dispatch in `codegen_call_expression`
- New test project `test-projects/print-types/` with `main.op` testing all primitive types
- New integration test in `tests/integration_e2e.rs` for `print-types` project
- New integration test in `tests/integration_e2e.rs` for existing `should-print-final-result` project

### Definition of Done
- [ ] `cargo test` passes (0 new failures)
- [ ] `cargo test --features integration` passes (0 new failures)
- [ ] `print(42)` outputs `42` followed by newline
- [ ] `print(3.14)` outputs a float representation followed by newline
- [ ] `print(true)` outputs `true` followed by newline
- [ ] `print('hello')` outputs `hello` followed by newline (regression check)

### Must Have
- Type-based dispatch for all primitive types (string, int8/16/32/64, float32/64, bool)
- `free()` call after `bool_to_string` + `puts` to prevent memory leak
- Lazy declaration of target `print_*` functions via `declare_stdlib_function`
- Regression test confirming `print('hello')` still works via `puts`

### Must NOT Have (Guardrails)
- Do NOT modify `resolve_callee_function` or `declare_stdlib_function`'s mapping of `"print"` → `puts`
- Do NOT build a TypeChecker→Codegen metadata bridge (NodeId→CoreType map)
- Do NOT add new C runtime functions (use existing `print_*` and `bool_to_string`)
- Do NOT handle arrays, structs, custom types, or multi-argument print
- Do NOT add unsigned-specific dispatch (`print_uint*`) — document as known limitation
- Do NOT switch string printing from `puts` to `print_string` — no user benefit
- Do NOT modify the `resolve_print_to_puts` unit test

---

## Verification Strategy (MANDATORY)

> **ZERO HUMAN INTERVENTION** - ALL verification is agent-executed. No exceptions.

### Test Decision
- **Infrastructure exists**: YES
- **Automated tests**: YES (Tests-after — unit + integration)
- **Framework**: `cargo test` (unit) + `cargo test --features integration` (e2e)

### QA Policy
Every task MUST include agent-executed QA scenarios.
Evidence saved to `.sisyphus/evidence/task-{N}-{scenario-slug}.{ext}`.

- **Compiler output**: Use Bash — compile test programs, run binaries, assert stdout
- **Unit tests**: Use Bash — `cargo test` specific test names

---

## Execution Strategy

### Parallel Execution Waves

```
Wave 1 (Start Immediately — implementation + test scaffolding):
├── Task 1: Implement type-based print dispatch in codegen [deep]
├── Task 2: Create print-types test project [quick]

Wave 2 (After Wave 1 — integration tests + verification):
├── Task 3: Add integration tests for print dispatch [unspecified-high]

Wave FINAL (After ALL tasks — 4 parallel reviews, then user okay):
├── Task F1: Plan compliance audit (oracle)
├── Task F2: Code quality review (unspecified-high)
├── Task F3: Real manual QA (unspecified-high)
└── Task F4: Scope fidelity check (deep)
-> Present results -> Get explicit user okay

Critical Path: Task 1 → Task 3 → F1-F4 → user okay
Parallel Speedup: Tasks 1+2 run in parallel
Max Concurrent: 2 (Wave 1)
```

### Dependency Matrix

| Task | Depends On | Blocks | Wave |
|------|-----------|--------|------|
| 1    | —         | 3      | 1    |
| 2    | —         | 3      | 1    |
| 3    | 1, 2      | F1-F4  | 2    |

### Agent Dispatch Summary

- **Wave 1**: **2** — T1 → `deep`, T2 → `quick`
- **Wave 2**: **1** — T3 → `unspecified-high`
- **FINAL**: **4** — F1 → `oracle`, F2 → `unspecified-high`, F3 → `unspecified-high`, F4 → `deep`

---

## TODOs

- [x] 1. Implement type-based print dispatch in `codegen_call_expression`

  **What to do**:
  - In `codegen_call_expression` (`src/codegen/functions_call.rs`), after arguments are lowered (after the `lowered_args` loop, around line ~85) but BEFORE the `build_call` (around line ~123), add a special case:
    - Check if `callee` matches `Expr::Identifier { name, .. }` where `name == "print"`
    - If so, and there is exactly one lowered argument, inspect the LLVM type of `lowered_args[0]`:
      - **Pointer type** (`is_pointer_value()`): Use the already-resolved `function` (which is `puts`) — fall through to existing `build_call` path. No changes needed.
      - **Integer type** (`is_int_value()`): Check bit width via `into_int_value().get_type().get_bit_width()`:
        - `1` (bool): Call `declare_stdlib_function(codegen_context, "bool_to_string")` to get `bool_to_string`, emit `build_call` to convert to `i8*`, then call `puts` with the result, then call `free()` on the returned pointer (since `bool_to_string` heap-allocates via `strdup`). For `free()`, declare it via `module.get_function("free").or_else(...)` with signature `void(i8*)`.
        - `8`: Call `declare_stdlib_function(codegen_context, "print_int8")`, emit `build_call` with the argument
        - `16`: Call `declare_stdlib_function(codegen_context, "print_int16")`, emit `build_call`
        - `32`: Call `declare_stdlib_function(codegen_context, "print_int32")`, emit `build_call`
        - `64`: Call `declare_stdlib_function(codegen_context, "print_int64")`, emit `build_call`
      - **Float type** (`is_float_value()`): Check bit width via `into_float_value().get_type().get_bit_width()`:
        - `32` (f32): Call `declare_stdlib_function(codegen_context, "print_float32")`, emit `build_call`
        - `64` (f64): Call `declare_stdlib_function(codegen_context, "print_float64")`, emit `build_call`
      - **Other**: Return `CodegenError` with message "unsupported type for print"
    - For all dispatched paths, handle the void return by returning `codegen_context.context.struct_type(&[], false).const_zero().as_basic_value_enum()` (matching the existing `try_as_basic_value` fallback pattern)
    - Early return from the function after the dispatched call — do NOT fall through to the generic `build_call`
  - Add `"free"` to `STDLIB_NAMES` in `functions_stdlib.rs` and add a match arm declaring `free` as `void(i8*)` — OR declare `free` inline in the print dispatch code (prefer inline declaration to minimize changes to stdlib list)
  - Verify the existing `resolve_print_to_puts` test still passes without modification

  **Must NOT do**:
  - Do NOT modify `resolve_callee_function` or `declare_stdlib_function`'s `"print"` → `puts` mapping
  - Do NOT use `FunctionValue.get_name()` to detect `print` — use the AST callee expression
  - Do NOT handle arrays, structs, custom types, or multi-argument print
  - Do NOT add unsigned-specific dispatch — all integers dispatch to signed `print_int*` variants

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: Requires understanding complex LLVM IR generation patterns, inkwell API for type inspection, and careful integration with existing codegen flow. Must handle edge cases (bool free, void returns) correctly.
  - **Skills**: []
  - **Skills Evaluated but Omitted**:
    - None applicable — this is pure Rust/LLVM codegen work

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1 (with Task 2)
  - **Blocks**: Task 3
  - **Blocked By**: None (can start immediately)

  **References** (CRITICAL):

  **Pattern References** (existing code to follow):
  - `src/codegen/expressions_string.rs:190-259` (`lower_interpolation_argument`) — The proven pattern for LLVM type-based dispatch. Shows exact API calls: `is_pointer_value()`, `is_int_value()`, `into_int_value().get_type().get_bit_width()`, `is_float_value()`. Follow this pattern exactly for type detection.
  - `src/codegen/functions_call.rs:17-139` (`codegen_call_expression`) — The function to modify. Shows where arguments are lowered (~line 33-85), where lambda captures are handled (~86-117), and where `build_call` happens (~123-127). Insert dispatch between the lambda-capture block and `build_call`.
  - `src/codegen/functions_call.rs:123-137` — The existing `build_call` + `try_as_basic_value` return pattern. The dispatched paths must replicate this void-handling pattern.

  **API/Type References** (contracts to implement against):
  - `src/codegen/functions_stdlib.rs:9-180` (`declare_stdlib_function`) — Call this to lazily declare `print_int32`, `print_float64`, `bool_to_string`, etc. Returns `Option<FunctionValue>`. Use `.ok_or_else(|| CodegenError::new(...))` to handle None.
  - `src/codegen/functions_stdlib.rs:195-241` (`STDLIB_NAMES`) — Lists all valid stdlib names. `print_int32`, `print_float64`, `bool_to_string` etc. are already listed. `free` is NOT listed — declare inline.

  **Test References** (testing patterns to follow):
  - `src/codegen/functions_call.rs:682-728` (`resolve_print_to_puts` test) — Do NOT modify. This test verifies the resolution layer, not the dispatch layer. It should still pass.

  **External References** (libraries and frameworks):
  - inkwell `BasicValueEnum` API: `is_pointer_value()`, `is_int_value()`, `is_float_value()`, `into_int_value()`, `into_float_value()`, `into_pointer_value()`
  - inkwell `IntType::get_bit_width()`, `FloatType::get_bit_width()` — return `u32`

  **WHY Each Reference Matters**:
  - `lower_interpolation_argument`: This is the EXACT pattern to replicate. It proves the LLVM type inspection API works and is the established convention in this codebase.
  - `codegen_call_expression`: The function body itself — must understand the flow to know where to insert the branch
  - `declare_stdlib_function`: The ONLY way to obtain typed print function declarations lazily
  - `resolve_print_to_puts`: Must NOT break — validates the resolution layer is untouched

  **Acceptance Criteria**:

  **QA Scenarios (MANDATORY):**

  ```
  Scenario: print(int32) dispatches to print_int32
    Tool: Bash (cargo test)
    Preconditions: Code changes applied to src/codegen/functions_call.rs
    Steps:
      1. Run `cargo test resolve_print_to_puts` — existing test must still pass
      2. Run `cargo test` — all existing unit tests pass with 0 new failures
      3. Run `cargo build` — compiler builds without warnings/errors
    Expected Result: All commands exit 0, no test failures
    Failure Indicators: Any test failure, compilation error, or LLVM verifier error
    Evidence: .sisyphus/evidence/task-1-unit-tests.txt

  Scenario: Compiler builds successfully after changes
    Tool: Bash
    Preconditions: Code changes applied
    Steps:
      1. Run `cargo build 2>&1`
      2. Check exit code is 0
      3. Verify no warnings related to the changed file
    Expected Result: Clean build, exit code 0
    Failure Indicators: Compilation errors, LLVM linking errors
    Evidence: .sisyphus/evidence/task-1-build.txt
  ```

  **Commit**: YES
  - Message: `fix(codegen): dispatch print() to typed runtime functions based on argument LLVM type`
  - Files: `src/codegen/functions_call.rs`
  - Pre-commit: `cargo test`

---

- [x] 2. Create `print-types` test project

  **What to do**:
  - Create `test-projects/print-types/` directory with standard test project structure:
    - `opal.toml` with `name = "print-types"` and `version = "0.1.0"`
    - `.gitignore` with `target/`
    - `src/main.op` with entry function that tests all print dispatch paths
  - `src/main.op` content should be:
    ```opal
    ##
      Description: Tests print() with all primitive types
    ##
    entry main = f(args: string[]): void =>
        print(42)
        print(3)
        print(true)
        print(false)
        print('hello')
        return void
    ```
  - Note: `42` and `3` will default to `int64` (the compiler's default integer literal type). `3.14` would need explicit float annotation if supported, or we test with what the compiler accepts. If float literals compile as `float64`, include `print(3.14)`.
  - Also check `test-projects/should-print-final-result/src/main.op` — verify it compiles and runs correctly with the fix

  **Must NOT do**:
  - Do NOT use complex types (arrays, structs, custom ADTs)
  - Do NOT add more than one test project
  - Do NOT modify existing test projects

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Simple file creation following an established test project convention
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1 (with Task 1)
  - **Blocks**: Task 3
  - **Blocked By**: None (can start immediately)

  **References** (CRITICAL):

  **Pattern References** (existing code to follow):
  - `test-projects/hello-world/` — Canonical test project structure: `opal.toml`, `.gitignore`, `src/main.op`. Follow this layout exactly.
  - `test-projects/hello-world/src/main.op` — Example of entry function with print call and doc comment format
  - `test-projects/hello-world/opal.toml` — Example `opal.toml` format
  - `test-projects/should-print-final-result/src/main.op` — Existing test project that calls `print(add(num1_int, num2_int))` with direct int argument

  **WHY Each Reference Matters**:
  - `hello-world/`: This is the simplest test project template to copy — guaranteed to follow conventions
  - `should-print-final-result/src/main.op`: Shows the exact bug scenario that must work after the fix

  **Acceptance Criteria**:

  **QA Scenarios (MANDATORY):**

  ```
  Scenario: print-types project has correct structure
    Tool: Bash
    Preconditions: Files created
    Steps:
      1. Verify `test-projects/print-types/opal.toml` exists and contains `name = "print-types"`
      2. Verify `test-projects/print-types/src/main.op` exists and contains `entry main`
      3. Verify `test-projects/print-types/src/main.op` contains `print(42)`, `print(true)`, `print('hello')`
      4. Verify `test-projects/print-types/.gitignore` exists
    Expected Result: All files exist with correct content
    Failure Indicators: Missing files, wrong content
    Evidence: .sisyphus/evidence/task-2-project-structure.txt
  ```

  **Commit**: YES (groups with Task 1)
  - Message: `test: add print-types test project for direct non-string printing`
  - Files: `test-projects/print-types/opal.toml`, `test-projects/print-types/.gitignore`, `test-projects/print-types/src/main.op`
  - Pre-commit: N/A (no Rust code to test)

---

- [x] 3. Add integration tests for print type dispatch

  **What to do**:
  - Add integration test `print_types_compiles_and_outputs_correctly` in `tests/integration_e2e.rs`:
    - Read source from `test-projects/print-types/src/main.op`
    - Compile via `compile_program(&source, &output_dir)`
    - Run the binary and capture stdout
    - Assert stdout contains `42` (integer print)
    - Assert stdout contains `true` (bool print)
    - Assert stdout contains `false` (bool print)
    - Assert stdout contains `hello` (string print, regression)
    - Assert binary exits with code 0
  - Add integration test `should_print_final_result_outputs_sum` in `tests/integration_e2e.rs`:
    - Read source from `test-projects/should-print-final-result/src/main.op`
    - Compile and run with stdin providing two numbers (e.g., `"3\n4\n"`)
    - Assert stdout contains the sum (e.g., `"7"`)
    - Assert binary exits with code 0
  - Follow the exact test pattern used by `hello_world_compiles_links_and_runs` and `simple_quiz_compiles_links_and_runs`
  - Run `cargo test --features integration` to verify all tests pass

  **Must NOT do**:
  - Do NOT modify existing integration tests
  - Do NOT create tests requiring manual intervention

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
    - Reason: Requires understanding the integration test harness patterns, stdin piping, and stdout capture. Not trivially simple but not architecturally complex.
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: NO
  - **Parallel Group**: Wave 2 (sequential after Wave 1)
  - **Blocks**: F1-F4
  - **Blocked By**: Task 1, Task 2

  **References** (CRITICAL):

  **Pattern References** (existing code to follow):
  - `tests/integration_e2e.rs` (lines containing `hello_world_compiles_links_and_runs`) — Exact integration test pattern: read source, compile, run binary, assert stdout, assert exit code. Copy this pattern.
  - `tests/integration_e2e.rs` (lines containing `simple_quiz_compiles_links_and_runs`) — Pattern for tests that pipe stdin to compiled binary. Shows how to use `.stdin(Stdio::piped())` and write input.
  - `tests/integration_e2e.rs` (lines containing `fib_iterative_compiles_links_and_runs`) — Pattern for asserting specific numeric values in stdout output.

  **Test References**:
  - `tests/integration_e2e.rs` — All existing integration tests follow the same pattern. The new tests must be gated behind `#[cfg(feature = "integration")]`.

  **WHY Each Reference Matters**:
  - `hello_world` test: Simplest e2e pattern — read/compile/run/assert
  - `simple_quiz` test: Shows stdin piping pattern needed for `should-print-final-result` (which reads two numbers)
  - `fib_iterative` test: Shows numeric stdout assertion pattern

  **Acceptance Criteria**:

  **QA Scenarios (MANDATORY):**

  ```
  Scenario: print-types integration test passes
    Tool: Bash
    Preconditions: Task 1 (dispatch fix) and Task 2 (test project) complete
    Steps:
      1. Run `cargo test --features integration print_types` 
      2. Check exit code is 0
      3. Verify test output shows PASS
    Expected Result: Test passes, stdout contains expected values for all types
    Failure Indicators: Test failure, compilation error, missing output
    Evidence: .sisyphus/evidence/task-3-integration-print-types.txt

  Scenario: should-print-final-result integration test passes
    Tool: Bash
    Preconditions: Task 1 (dispatch fix) complete
    Steps:
      1. Run `cargo test --features integration should_print_final_result`
      2. Check exit code is 0
      3. Verify test output shows PASS
    Expected Result: Test passes, stdout contains sum of piped input numbers
    Failure Indicators: Test failure, wrong output, crash
    Evidence: .sisyphus/evidence/task-3-integration-should-print.txt

  Scenario: All existing integration tests still pass (regression)
    Tool: Bash
    Preconditions: All code changes applied
    Steps:
      1. Run `cargo test --features integration 2>&1`
      2. Check exit code is 0
      3. Verify no test regressions
    Expected Result: All tests pass, 0 failures
    Failure Indicators: Any existing test failure
    Evidence: .sisyphus/evidence/task-3-regression.txt
  ```

  **Commit**: YES
  - Message: `test: add integration tests for typed print dispatch`
  - Files: `tests/integration_e2e.rs`
  - Pre-commit: `cargo test --features integration`

---

## Final Verification Wave (MANDATORY — after ALL implementation tasks)

> 4 review agents run in PARALLEL. ALL must APPROVE. Present consolidated results to user and get explicit "okay" before completing.
>
> **Do NOT auto-proceed after verification. Wait for user's explicit approval before marking work complete.**

- [x] F1. **Plan Compliance Audit** — `oracle`
  Read the plan end-to-end. For each "Must Have": verify implementation exists (read file, run command). For each "Must NOT Have": search codebase for forbidden patterns — reject with file:line if found. Check evidence files exist in .sisyphus/evidence/. Compare deliverables against plan.
  Output: `Must Have [N/N] | Must NOT Have [N/N] | Tasks [N/N] | VERDICT: APPROVE/REJECT`

- [x] F2. **Code Quality Review** — `unspecified-high`
  Run `cargo test` + `cargo clippy`. Review all changed files for: `as any`/`@ts-ignore` (N/A for Rust), empty catches, `println!` in prod code, commented-out code, unused imports. Check for proper error handling in the dispatch code. Verify `free()` is called after `bool_to_string` → `puts`.
  Output: `Build [PASS/FAIL] | Clippy [PASS/FAIL] | Tests [N pass/N fail] | Files [N clean/N issues] | VERDICT`

- [x] F3. **Real Manual QA** — `unspecified-high`
  Start from clean state. Compile and run the `print-types` test project manually. Compile and run `should-print-final-result` manually with stdin input. Verify actual terminal output matches expected values. Test edge cases: `print(0)`, `print(-1)` if possible. Save to `.sisyphus/evidence/final-qa/`.
  Output: `Scenarios [N/N pass] | Integration [N/N] | Edge Cases [N tested] | VERDICT`

- [x] F4. **Scope Fidelity Check** — `deep`
  For each task: read "What to do", read actual diff (git log/diff). Verify 1:1 — everything in spec was built (no missing), nothing beyond spec was built (no creep). Check "Must NOT do" compliance. Detect cross-task contamination: Task N touching Task M's files. Flag unaccounted changes.
  Output: `Tasks [N/N compliant] | Contamination [CLEAN/N issues] | Unaccounted [CLEAN/N files] | VERDICT`

---

## Commit Strategy

| Task | Commit Message | Files | Pre-commit |
|------|---------------|-------|------------|
| 1 | `fix(codegen): dispatch print() to typed runtime functions based on argument LLVM type` | `src/codegen/functions_call.rs` | `cargo test` |
| 2 | `test: add print-types test project for direct non-string printing` | `test-projects/print-types/*` | N/A |
| 3 | `test: add integration tests for typed print dispatch` | `tests/integration_e2e.rs` | `cargo test --features integration` |

---

## Success Criteria

### Verification Commands
```bash
# Unit tests (all existing + no regressions)
cargo test                          # Expected: all pass, 0 failures

# Integration tests (including new print dispatch tests)
cargo test --features integration   # Expected: all pass, 0 failures

# Specific new test targeting
cargo test --features integration print_types
cargo test --features integration should_print_final_result

# Build check
cargo build                         # Expected: clean build, exit 0
cargo clippy                        # Expected: no new warnings
```

### Final Checklist
- [ ] All "Must Have" present (type dispatch for int/float/bool/string, free after bool_to_string, lazy declaration)
- [ ] All "Must NOT Have" absent (no resolve_callee_function changes, no TypeChecker bridge, no new runtime functions)
- [ ] All tests pass (`cargo test` and `cargo test --features integration`)
- [ ] `print(42)` outputs `42\n`, `print(true)` outputs `true\n`, `print('hello')` outputs `hello\n`

### Known Limitations (Documented)
- **Signed/unsigned ambiguity**: `print(my_uint32_var)` will print as signed because LLVM `i32` cannot distinguish signed from unsigned. Values above `2^31` will appear negative. This matches existing string interpolation behavior. Fix requires a TypeChecker→Codegen metadata bridge (out of scope).
- **No `print_bool` runtime function**: Booleans use two-step `bool_to_string` → `puts` → `free`. A dedicated `print_bool` runtime function would be more efficient but is out of scope.
