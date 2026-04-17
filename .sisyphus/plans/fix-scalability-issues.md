# Fix All SCALABILITY_ISSUES.md Problems

## TL;DR

> **Quick Summary**: Fix all 28 issues in SCALABILITY_ISSUES.md across the Opalescent compiler codegen (Rust/LLVM IR), C runtime, parser, type checker, lexer, and module structure. Uses TDD red-green-refactor with test projects for end-to-end verification.
> 
> **Deliverables**:
> - All 28 issues from SCALABILITY_ISSUES.md resolved
> - Rust unit tests for each fix (red→green→refactor)
> - Test projects in `test-projects/` verifying compiler-testable items (1a, 1d, 1e, 1g, etc.)
> - Upgraded LLVM 14→18 and inkwell 0.8→0.9
> - Split monolithic runtime.c into focused modules
> - Refactored module structure (remove #[path] attrs)
> 
> **Estimated Effort**: XL
> **Parallel Execution**: YES - 7 waves
> **Critical Path**: T1 (runtime trap fn) → T2-T7 (P0 correctness) → T8-T16 (P1 spec) → T17-T24 (P2 safety) → T25-T28 (P2 arch) → T29 (LLVM upgrade) → T30 (test projects) → F1-F4 (verification)

---

## Context

### Original Request
Fix every single issue in SCALABILITY_ISSUES.md (28 total) using TDD with red-green-refactor. Create test projects to verify compiler-testable fixes compile and type-check successfully. Use language-spec files as syntax source-of-truth. Follow priority order (P0→P1→P2). Use Momus for high-accuracy review.

### Interview Summary
**Key Discussions**:
- Overflow in Release mode: User confirmed ALWAYS TRAP — release mode traps on overflow by default, wrapping_*/saturating_* are explicit escape hatches
- Trap mechanism: Emit runtime function call (`opal_runtime_error("message")`) that prints descriptive message + exits
- LLVM upgrade target: inkwell 0.9.0 + LLVM 18 (llvm18-1 feature)
- TDD approach: Rust's `cargo test` + integration test projects in `test-projects/`
- Syntax: Follow language-spec (colon-block, no curly braces on guards unless spec says so)

**Research Findings**:
- Lambda bodies completely missing — `resolve_callee_function` creates LLVM function but only emits `emit_default_return`
- Captured var fallback silently pushes `const_zero()` — data corruption
- String interp uses fixed 256-byte malloc + sprintf without bounds checking
- `codegen_cast` uses `sitofp` unconditionally regardless of source signedness
- `codegen_assignment` doesn't check `is_mutable`; `VariableBinding` lacks that field
- Runtime has 0 free() calls across 12 allocation sites
- Array GEP may need `[0, index]` not just `[index]` for alloca'd arrays
- 123 `#[path]` attributes across 18 files
- `pure` and `untested` keywords completely absent from token.rs/lexer.rs
- Inkwell 0.8→0.9 requires opaque pointer migration across entire codegen

### Metis Review
**Identified Gaps** (addressed):
- Overflow Release behavior clarified: ALWAYS TRAP (user confirmed)
- "Trap" mechanism defined: runtime function call with message + exit
- LLVM upgrade target resolved: inkwell 0.9.0 + LLVM 18

---

## Work Objectives

### Core Objective
Fix all 28 scalability and correctness issues cataloged in SCALABILITY_ISSUES.md, verified via TDD and end-to-end test projects, to produce a correct, safe, and maintainable compiler.

### Concrete Deliverables
- Fixed codegen for overflow trapping (1a), float→int casts (1b), lambda bodies (1d), array bounds (1g), captured vars (1e), default return (1f), uint→float (1c)
- String interpolation dynamic buffer (3c)
- `pure` and `untested` keyword support (4a, 4b)
- Cast safety matching spec (4c)
- Immutability enforcement in codegen (4d)
- Memory management in runtime (3a) and codegen (3b)
- Portable format specifiers (2b), cross-platform linker (6d)
- Thread-safe runtime (2a, 2e), NULL-checked malloc (2c), quality RNG (2d)
- Deduplicated stdlib names (5a), helpers (5b, 5c)
- Standard module structure (6a), scoped node IDs (6b), refactored TypeChecker (6c)
- LLVM 18 upgrade (6e), split runtime.c (6f)
- Test projects verifying key fixes

### Definition of Done
- [ ] `cargo test` passes with 0 failures
- [ ] `cargo test --features integration` passes with 0 failures
- [ ] All 28 issues addressed with code changes
- [ ] Test projects for compiler-testable items (1a, 1d, 1e, 1g, 3c, 4d) compile and run correctly
- [ ] No new compiler warnings introduced (`cargo build 2>&1 | grep warning | wc -l` same or less)

### Must Have
- Every issue from SCALABILITY_ISSUES.md addressed
- TDD red-green-refactor workflow for each fix
- Test projects for project-testable items
- Language-spec syntax followed exactly
- Runtime trap function for all safety checks

### Must NOT Have (Guardrails)
- NO curly braces on guard/if/while statements (use colon-block syntax per spec)
- NO changes to language syntax unless SCALABILITY_ISSUES.md specifically calls for it
- NO new language features beyond what's needed to fix the 28 issues
- NO `as any`, `@ts-ignore`, `unsafe` blocks without explicit justification
- NO over-abstraction — keep changes minimal and focused
- NO removal of existing passing tests
- NO changes to the public API surface unless required by an issue

---

## Verification Strategy (MANDATORY)

> **ZERO HUMAN INTERVENTION** — ALL verification is agent-executed. No exceptions.

### Test Decision
- **Infrastructure exists**: YES — `cargo test` + `tests/integration_e2e.rs` (feature-gated)
- **Automated tests**: TDD (red-green-refactor)
- **Framework**: Rust's built-in test framework (`#[test]`, `#[cfg(test)]`)
- **TDD**: Each task follows RED (failing test) → GREEN (minimal impl) → REFACTOR

### QA Policy
Every task MUST include agent-executed QA scenarios.
Evidence saved to `.sisyphus/evidence/task-{N}-{scenario-slug}.{ext}`.

- **Compiler codegen fixes**: Use Bash — `cargo test` specific test, verify output
- **Runtime C fixes**: Use Bash — compile runtime, run test projects, verify output
- **Test projects**: Use Bash — `cargo run -- test-projects/{name}/src/main.op --run`, verify exit code + output
- **Module restructuring**: Use Bash — `cargo build`, verify no errors

---

## Execution Strategy

### Parallel Execution Waves

```
Wave 1 (Foundation — runtime trap function + dedup helpers):
├── Task 1: Add opal_runtime_error() trap function to runtime [quick]
├── Task 2: Extract shared codegen helpers (5b integer_literal_bits + 5c is_signed_core_type) [quick]
├── Task 3: Deduplicate stdlib name registry (5a) [quick]
└── Task 4: Fix format specifiers %ld/%lu → PRId64/PRIu64 (2b) [quick]

Wave 2 (P0 Correctness — all depend on T1 trap function):
├── Task 5: Fix integer overflow — always trap (1a) [deep]
├── Task 6: Fix float→int cast range guard (1b) [deep]
├── Task 7: Fix lambda body codegen (1d) [deep]
├── Task 8: Fix array bounds checking (1g) [deep]
├── Task 9: Fix string interpolation buffer overflow (3c) [unspecified-high]
├── Task 10: Fix missing captured vars error (1e) [unspecified-high]
└── Task 11: Fix emit_default_return trap (1f) [unspecified-high]

Wave 3 (P1 Correctness + Spec — depends on Wave 2 helpers):
├── Task 12: Fix unsigned int→float instruction (1c) [unspecified-high]
├── Task 13: Add pure keyword (4a) [unspecified-high]
├── Task 14: Add untested keyword (4b) [unspecified-high]
├── Task 15: Cast safety matching spec (4c) [deep]
└── Task 16: Immutability enforcement in codegen (4d) [unspecified-high]

Wave 4 (P1 Memory + Portability):
├── Task 17: Add free() calls in runtime (3a) [deep]
├── Task 18: Add free() calls in codegen (3b) [deep]
└── Task 19: Cross-platform linker support (6d) [unspecified-high]

Wave 5 (P2 Runtime Safety):
├── Task 20: Thread-safe invalid_digit_error buffer (2a) [quick]
├── Task 21: malloc NULL checks (2c) [quick]
├── Task 22: Dynamic take_input buffer (2e) [quick]
└── Task 23: Quality RNG replacement (2d) [unspecified-high]

Wave 6 (P2 Architecture — independent refactors):
├── Task 24: Convert #[path] attrs to standard modules (6a) [unspecified-high]
├── Task 25: Scoped NEXT_NODE_ID (6b) [quick]
├── Task 26: Refactor TypeChecker fields (6c) [unspecified-high]
├── Task 27: Split monolithic runtime.c (6f) [unspecified-high]
└── Task 28: Upgrade inkwell 0.9.0 + LLVM 18 (6e) [ultrabrain]

Wave 7 (Integration — test projects + final verification):
├── Task 29: Create test projects for compiler-testable fixes [unspecified-high]
└── Task 30: Full regression test suite run [quick]

Wave FINAL (After ALL tasks — 4 parallel reviews, then user okay):
├── Task F1: Plan compliance audit (oracle)
├── Task F2: Code quality review (unspecified-high)
├── Task F3: Real manual QA (unspecified-high)
└── Task F4: Scope fidelity check (deep)
-> Present results -> Get explicit user okay
```

### Critical Path
T1 → T5-T11 (P0) → T12-T16 (P1 spec) → T17-T18 (memory) → T29 (test projects) → F1-F4 → user okay

### Dependency Matrix

| Task | Depends On | Blocks |
|------|-----------|--------|
| T1 | — | T5, T6, T7, T8, T9, T10, T11, T15 |
| T2 | — | T5, T6, T12 |
| T3 | — | — |
| T4 | — | — |
| T5 | T1, T2 | T29 |
| T6 | T1 | T15, T29 |
| T7 | T1 | T29 |
| T8 | T1 | T29 |
| T9 | — | T29 |
| T10 | T1 | T29 |
| T11 | T1 | T29 |
| T12 | T2 | T29 |
| T13 | — | — |
| T14 | — | — |
| T15 | T1, T6 | T29 |
| T16 | — | T29 |
| T17 | — | T27 |
| T18 | — | — |
| T19 | — | — |
| T20 | — | T27 |
| T21 | — | T27 |
| T22 | — | T27 |
| T23 | — | T27 |
| T24 | — | — |
| T25 | — | — |
| T26 | — | — |
| T27 | T17, T20, T21, T22, T23 | — |
| T28 | ALL prior tasks | T29 |
| T29 | T5-T16, T28 | T30 |
| T30 | T29 | F1-F4 |

### Agent Dispatch Summary

- **Wave 1**: **4 tasks** — T1→`quick`, T2→`quick`, T3→`quick`, T4→`quick`
- **Wave 2**: **7 tasks** — T5→`deep`, T6→`deep`, T7→`deep`, T8→`deep`, T9→`unspecified-high`, T10→`unspecified-high`, T11→`unspecified-high`
- **Wave 3**: **5 tasks** — T12→`unspecified-high`, T13→`unspecified-high`, T14→`unspecified-high`, T15→`deep`, T16→`unspecified-high`
- **Wave 4**: **3 tasks** — T17→`deep`, T18→`deep`, T19→`unspecified-high`
- **Wave 5**: **4 tasks** — T20→`quick`, T21→`quick`, T22→`quick`, T23→`unspecified-high`
- **Wave 6**: **5 tasks** — T24→`unspecified-high`, T25→`quick`, T26→`unspecified-high`, T27→`unspecified-high`, T28→`ultrabrain`
- **Wave 7**: **2 tasks** — T29→`unspecified-high`, T30→`quick`
- **FINAL**: **4 tasks** — F1→`oracle`, F2→`unspecified-high`, F3→`unspecified-high`, F4→`deep`

---

## TODOs

> Implementation + Test = ONE Task. Never separate.
> EVERY task MUST have: Recommended Agent Profile + Parallelization info + QA Scenarios.

### Wave 1 — Foundation (start immediately)

- [x] 1. Add `opal_runtime_error()` trap function to runtime

  **What to do**:
  - RED: Write a Rust unit test that asserts `opal_runtime_error` is declared as a known stdlib function — e.g., test that `is_stdlib_name("opal_runtime_error")` returns `true`, and that `resolve_imported_runtime_name("opal_runtime_error")` returns the correct C symbol name. Also write a test that compiles a minimal program and verifies the LLVM module contains the `opal_runtime_error` declaration (check via `module.get_function("opal_runtime_error").is_some()`). These tests will fail RED since the function doesn't exist yet. **Do NOT attempt a runtime-triggering integration test** — nothing calls `opal_runtime_error` until Wave 2 tasks wire up callers.
  - GREEN: Add a new function `opal_runtime_error(const char* message)` to `runtime/opal_runtime.c` that:
    - Prints `"Runtime error: {message}\n"` to stderr
    - Calls `exit(1)`
  - Add the function signature declaration in `src/codegen/functions_stdlib.rs` `declare_stdlib_function()` so codegen can call it
  - Add name to `resolve_imported_runtime_name()` and `is_stdlib_name()` match blocks
  - REFACTOR: Ensure the function signature matches what codegen will emit (takes `i8*` pointer to string constant)

  **Must NOT do**:
  - Do NOT change any existing trap/error behavior yet — this task only ADDS the function
  - Do NOT modify any codegen callers — those are in Wave 2 tasks

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Small, focused addition — one C function + three Rust match arm additions
  - **Skills**: []
  - **Skills Evaluated but Omitted**:
    - `playwright`: No browser interaction needed

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1 (with Tasks 2, 3, 4)
  - **Blocks**: Tasks 5, 6, 7, 8, 9, 10, 11, 15
  - **Blocked By**: None

  **References**:

  **Pattern References**:
  - `runtime/opal_runtime.c:21-25` — `invalid_digit_error()` function shows existing error-reporting pattern (uses fprintf to stderr)
  - `runtime/opal_runtime.c:1-10` — includes and header section showing existing runtime conventions

  **API/Type References**:
  - `src/codegen/functions_stdlib.rs:19-176` — `declare_stdlib_function()`: add function declaration following existing pattern (see how `print_string`, `take_input` are declared)
  - `src/codegen/functions_stdlib.rs:178-230` — `resolve_imported_runtime_name()`: add name mapping
  - `src/codegen/functions.rs:357-391` — `is_stdlib_name()`: add to match block

  **External References**:
  - None needed — follows existing patterns exactly

  **WHY Each Reference Matters**:
  - `invalid_digit_error` shows the existing error output pattern (fprintf to stderr) — follow this for consistency
  - `declare_stdlib_function` shows how to declare a C function so LLVM codegen can call it — must add `opal_runtime_error` with correct signature (void return, i8* param)
  - `is_stdlib_name` and `resolve_imported_runtime_name` are the other two places that need updating (issue 5a's duplication — all three must stay in sync until 5a is fixed)

  **Acceptance Criteria**:

  - [ ] `opal_runtime_error` function exists in `runtime/opal_runtime.c`
  - [ ] Function declared in `declare_stdlib_function()` with correct LLVM type signature
  - [ ] Name added to `resolve_imported_runtime_name()` and `is_stdlib_name()`
  - [ ] `cargo build` succeeds with no new warnings

  **QA Scenarios (MANDATORY)**:

  ```
  Scenario: Runtime trap function compiles into runtime
    Tool: Bash
    Preconditions: Clean working tree
    Steps:
      1. Run `cargo build 2>&1`
      2. Check output contains no errors
      3. Run `grep -n "opal_runtime_error" runtime/opal_runtime.c` to verify function exists
      4. Run `grep -n "opal_runtime_error" src/codegen/functions_stdlib.rs` to verify declaration exists
      5. Run `grep -n "opal_runtime_error" src/codegen/functions.rs` to verify is_stdlib_name entry
    Expected Result: Build succeeds, function found in all 3 locations
    Failure Indicators: Build error, grep returns empty for any location
    Evidence: .sisyphus/evidence/task-1-runtime-trap-compiles.txt

  Scenario: Existing tests still pass after adding trap function
    Tool: Bash
    Preconditions: Task 1 changes applied
    Steps:
      1. Run `cargo test 2>&1`
      2. Check exit code is 0
      3. Count test results line
    Expected Result: All existing tests pass, 0 failures
    Failure Indicators: Any test failure, non-zero exit code
    Evidence: .sisyphus/evidence/task-1-existing-tests-pass.txt
  ```

  **Commit**: YES (group with Wave 1)
  - Message: `fix(runtime): add opal_runtime_error trap function`
  - Files: `runtime/opal_runtime.c`, `src/codegen/functions_stdlib.rs`, `src/codegen/functions.rs`
  - Pre-commit: `cargo build && cargo test`

---

- [x] 2. Extract shared codegen helpers — `integer_literal_bits` + `is_signed_core_type` (issues 5b, 5c)

  **What to do**:
  - RED: Write unit tests in a new test module within the shared helpers file that test:
    - `integer_literal_bits(42)` returns `Ok(42)` (non-negative identity)
    - `integer_literal_bits(-1)` returns the correct two's complement `u64` encoding (i.e. `Ok(u64::MAX)` or equivalent)
    - `integer_literal_bits(-128)` returns correct two's complement encoding
    - `integer_literal_bits(0)` returns `Ok(0)`
    - `is_signed_core_type(&CoreType::Int32)` returns `true`
    - `is_signed_core_type(&CoreType::UInt32)` returns `false`
    - `is_signed_core_type(&CoreType::Int8)` / `Int16` / `Int64` all return `true`
    - `is_signed_core_type(&CoreType::UInt8)` / `UInt16` / `UInt64` all return `false`
  - GREEN: Create a shared module (e.g., `src/codegen/helpers.rs` or add functions to `src/codegen/types.rs`) containing:
    - `pub fn integer_literal_bits(number: i64) -> Result<u64, CodegenError>` — extracted from `src/codegen/expressions.rs` line 748 / `src/codegen/adts.rs` line 368. This function encodes a signed integer as its two's complement bit pattern (non-negative → identity, negative → two's complement u64).
    - `pub const fn is_signed_core_type(core_type: &CoreType) -> bool` — extracted from `src/codegen/expressions.rs` line 741 / `src/codegen/expressions_numeric.rs` line 431. Returns `true` for signed integer types (Int8, Int16, Int32, Int64), `false` otherwise.
  - Update all call sites to use the shared functions (delete duplicates)
  - REFACTOR: Ensure no other duplicates exist via grep

  **Must NOT do**:
  - Do NOT change the logic of these functions — only move them
  - Do NOT rename them unless there's a naming conflict

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Mechanical extract-and-redirect refactor with no logic changes
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1 (with Tasks 1, 3, 4)
  - **Blocks**: Tasks 5, 6, 12
  - **Blocked By**: None

  **References**:

  **Pattern References**:
  - `src/codegen/expressions.rs:741-759` — Original `is_signed_core_type` (line 741, `const fn is_signed_core_type(core_type: &CoreType) -> bool`) and `integer_literal_bits` (line 748, `fn integer_literal_bits(number: i64) -> Result<u64, CodegenError>`) implementations
  - `src/codegen/adts.rs:368-378` — Duplicate `integer_literal_bits(number: i64) -> Result<u64, CodegenError>`
  - `src/codegen/expressions_numeric.rs:431` — Duplicate `const fn is_signed_core_type(core_type: &CoreType) -> bool`

  **API/Type References**:
  - `src/codegen/types.rs` — Existing shared types module, potential home for extracted functions
  - `src/codegen.rs` — Module declarations with `#[path]` attributes, need to add new module if creating separate file
  - `src/codegen/codegen_error.rs` (or wherever `CodegenError` is defined) — `integer_literal_bits` returns `Result<u64, CodegenError>`

  **WHY Each Reference Matters**:
  - The two source locations for each function show exact current implementations to extract
  - `types.rs` is the natural home since these are type-query functions
  - `codegen.rs` module declarations show how to wire up a new submodule if needed
  - `CodegenError` must be imported by the shared module since `integer_literal_bits` returns `Result<u64, CodegenError>`

  **Acceptance Criteria**:

  - [ ] `integer_literal_bits` exists in exactly ONE location (shared module) with signature `fn integer_literal_bits(number: i64) -> Result<u64, CodegenError>`
  - [ ] `is_signed_core_type` exists in exactly ONE location (shared module) with signature `const fn is_signed_core_type(core_type: &CoreType) -> bool`
  - [ ] All call sites updated to import from shared module
  - [ ] Unit tests verify two's complement encoding: `integer_literal_bits(-1) == Ok(u64::MAX)`, `integer_literal_bits(42) == Ok(42)`, `integer_literal_bits(0) == Ok(0)`
  - [ ] Unit tests verify signedness: `is_signed_core_type(Int32) == true`, `is_signed_core_type(UInt32) == false`
  - [ ] `cargo test` passes with 0 failures
  - [ ] `grep -rn "fn integer_literal_bits" src/codegen/` returns exactly 1 result
  - [ ] `grep -rn "fn is_signed_core_type" src/codegen/` returns exactly 1 result

  **QA Scenarios (MANDATORY)**:

  ```
  Scenario: No duplicate function definitions remain
    Tool: Bash
    Preconditions: Task 2 changes applied
    Steps:
      1. Run `grep -rn "fn integer_literal_bits" src/codegen/`
      2. Assert exactly 1 line returned, with signature containing `number: i64`
      3. Run `grep -rn "fn is_signed_core_type" src/codegen/`
      4. Assert exactly 1 line returned, with signature containing `core_type: &CoreType`
    Expected Result: Each function defined in exactly one place with correct signatures
    Failure Indicators: More than 1 match for either grep, or wrong signature
    Evidence: .sisyphus/evidence/task-2-no-duplicates.txt

  Scenario: Unit tests validate actual behavior
    Tool: Bash
    Preconditions: Task 2 changes applied, unit tests added
    Steps:
      1. Run `cargo test integer_literal_bits 2>&1` — assert tests pass
      2. Run `cargo test is_signed_core_type 2>&1` — assert tests pass
    Expected Result: All unit tests pass, confirming two's complement encoding and signedness logic
    Failure Indicators: Any test failure
    Evidence: .sisyphus/evidence/task-2-unit-tests.txt

  Scenario: Build and tests pass after dedup
    Tool: Bash
    Preconditions: Task 2 changes applied
    Steps:
      1. Run `cargo build 2>&1` — assert no errors
      2. Run `cargo test 2>&1` — assert all pass
    Expected Result: Zero build errors, zero test failures
    Failure Indicators: Any compilation error or test failure
    Evidence: .sisyphus/evidence/task-2-build-tests-pass.txt
  ```

  **Commit**: YES (group with Wave 1)
  - Message: `refactor(codegen): extract shared integer_literal_bits and is_signed_core_type helpers (5b, 5c)`
  - Files: `src/codegen/types.rs` (or new helpers.rs), `src/codegen/expressions.rs`, `src/codegen/adts.rs`, `src/codegen/expressions_numeric.rs`
  - Pre-commit: `cargo test`

---

- [x] 3. Deduplicate stdlib function name registry (issue 5a)

  **What to do**:
  - RED: Write a test that calls a helper function listing all stdlib names and verifies the count matches the expected number (45 runtime functions).
  - GREEN: Create a single authoritative list of stdlib function names (e.g., a `const STDLIB_NAMES: &[&str]` array or a lazy_static HashSet) in `src/codegen/functions_stdlib.rs`. Then:
    - Refactor `declare_stdlib_function()` to iterate over this list (or derive from it)
    - Refactor `resolve_imported_runtime_name()` to use this list
    - Refactor `is_stdlib_name()` in `src/codegen/functions.rs` to use this list
  - REFACTOR: Verify that adding a new runtime function now only requires ONE change (adding to the registry)

  **Must NOT do**:
  - Do NOT change the actual set of stdlib functions
  - Do NOT add or remove any runtime function names
  - Do NOT change the behavior of any codegen path

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Mechanical deduplication with no logic changes — extract names into one place
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1 (with Tasks 1, 2, 4)
  - **Blocks**: None (other tasks can work with old or new pattern)
  - **Blocked By**: None

  **References**:

  **Pattern References**:
  - `src/codegen/functions_stdlib.rs:19-176` — `declare_stdlib_function()` with all 45 function declarations
  - `src/codegen/functions_stdlib.rs:178-230` — `resolve_imported_runtime_name()` with name→C-name mapping
  - `src/codegen/functions.rs:357-391` — `is_stdlib_name()` match block with name list

  **WHY Each Reference Matters**:
  - These are the THREE places where stdlib names are duplicated — all must be unified into one registry
  - The declare function shows both the name AND the LLVM type signature — registry may need to include both
  - `is_stdlib_name` is a simple name check — can be derived from registry keys

  **Acceptance Criteria**:

  - [ ] Single authoritative list of stdlib names exists
  - [ ] `declare_stdlib_function`, `resolve_imported_runtime_name`, `is_stdlib_name` all derive from this list
  - [ ] `cargo test` passes
  - [ ] `cargo build` has no new warnings

  **QA Scenarios (MANDATORY)**:

  ```
  Scenario: Stdlib names exist in one authoritative location
    Tool: Bash
    Preconditions: Task 3 changes applied
    Steps:
      1. Run `cargo build 2>&1` — assert success
      2. Run `cargo test 2>&1` — assert all pass
      3. Verify the registry exists: `grep -n "STDLIB" src/codegen/functions_stdlib.rs`
    Expected Result: Build passes, tests pass, single registry found
    Failure Indicators: Build failure, test failure, no registry pattern found
    Evidence: .sisyphus/evidence/task-3-stdlib-dedup.txt
  ```

  **Commit**: YES (group with Wave 1)
  - Message: `refactor(codegen): unify stdlib name registry into single source of truth (5a)`
  - Files: `src/codegen/functions_stdlib.rs`, `src/codegen/functions.rs`
  - Pre-commit: `cargo test`

---

- [x] 4. Fix format specifiers `%ld/%lu` → `PRId64/PRIu64` (issue 2b)

  **What to do**:
  - RED: Write a test (or note for test project) verifying that int64/uint64 printing uses portable format specifiers.
  - GREEN: In `runtime/opal_runtime.c`:
    - Add `#include <inttypes.h>` at the top
    - Replace `%ld` with `"%" PRId64` in `print_int64` (line ~567) and `int64_to_string` (line ~567-571)
    - Replace `%lu` with `"%" PRIu64` in `print_uint64` (line ~595) and `uint64_to_string` (line ~594-599)
    - Also check `string_to_int64` (line ~70) and `string_to_uint64` (line ~86) for `%ld`/`%lu` in scanf patterns
    - Replace ALL occurrences — search entire file for `%ld` and `%lu`
  - REFACTOR: Verify no `%ld` or `%lu` remain in the file

  **Must NOT do**:
  - Do NOT change format specifiers for types smaller than 64-bit (int8/int16/int32 use %d/%u which is fine)
  - Do NOT change any function signatures

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Simple find-and-replace in one C file with no logic changes
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1 (with Tasks 1, 2, 3)
  - **Blocks**: None
  - **Blocked By**: None

  **References**:

  **Pattern References**:
  - `runtime/opal_runtime.c:567-571` — `int64_to_string` using `%ld` in sprintf
  - `runtime/opal_runtime.c:594-599` — `uint64_to_string` using `%lu` in sprintf
  - `runtime/opal_runtime.c:70` — `string_to_int64` using `%ld` in strtol/sscanf
  - `runtime/opal_runtime.c:86` — `string_to_uint64` using `%lu` in strtoul/sscanf

  **External References**:
  - C11 `<inttypes.h>` — PRId64 and PRIu64 macro documentation

  **WHY Each Reference Matters**:
  - These are the exact lines with non-portable format specifiers
  - `<inttypes.h>` provides the portable macros — standard since C99

  **Acceptance Criteria**:

  - [ ] `#include <inttypes.h>` present in runtime/opal_runtime.c
  - [ ] Zero occurrences of `%ld` or `%lu` in runtime/opal_runtime.c
  - [ ] `PRId64` and `PRIu64` used for all int64/uint64 formatting
  - [ ] `cargo build` succeeds (runtime compiles)

  **QA Scenarios (MANDATORY)**:

  ```
  Scenario: No non-portable format specifiers remain
    Tool: Bash
    Preconditions: Task 4 changes applied
    Steps:
      1. Run `grep -n '%ld\|%lu' runtime/opal_runtime.c`
      2. Assert zero matches
      3. Run `grep -c 'PRId64\|PRIu64' runtime/opal_runtime.c`
      4. Assert count > 0
      5. Run `grep -n 'inttypes.h' runtime/opal_runtime.c`
      6. Assert match found
    Expected Result: No %ld/%lu, PRId64/PRIu64 present, inttypes.h included
    Failure Indicators: Any %ld or %lu found, no PRId64/PRIu64 found
    Evidence: .sisyphus/evidence/task-4-portable-format.txt

  Scenario: Build still succeeds
    Tool: Bash
    Preconditions: Task 4 changes applied
    Steps:
      1. Run `cargo build 2>&1`
      2. Assert no errors
    Expected Result: Clean build
    Failure Indicators: Compilation error in runtime
    Evidence: .sisyphus/evidence/task-4-build-pass.txt
  ```

  **Commit**: YES (group with Wave 1)
  - Message: `fix(runtime): use portable PRId64/PRIu64 format specifiers (2b)`
  - Files: `runtime/opal_runtime.c`
  - Pre-commit: `cargo build`

---

### Wave 2 — P0 Correctness (depends on T1 trap function)

- [x] 5. Fix integer overflow — always trap in both debug and release (issue 1a)

  **What to do**:
  - RED: Write a Rust codegen unit test that generates code for `int32 max_value + 1` and verifies the codegen emits checked overflow intrinsics (not plain `build_int_add`). Also write an integration test that compiles an Opalescent program doing `2147483647 + 1` and asserts non-zero exit code (trap).
  - GREEN: In `src/codegen/expressions_numeric.rs` function `codegen_numeric_binop` (lines 53-82):
    - Remove the `if env.debug_mode` gate (line 58)
    - ALWAYS take the checked overflow intrinsic path (`codegen_checked_overflow_intrinsic`)
    - When overflow is detected, emit a call to `opal_runtime_error("integer overflow")` instead of/before the `unreachable` instruction
  - This applies to `+`, `-`, `*` operations on ALL integer types (int8-int64, uint8-uint64)
  - REFACTOR: Remove dead code from the old unchecked path if it becomes unreachable

  **Must NOT do**:
  - Do NOT change float arithmetic behavior
  - Do NOT implement wrapping_*/saturating_*/checked_* variants (those are future work beyond the 28 issues)
  - Do NOT change division/modulo behavior (those already trap on div-by-zero per spec)

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: Touches core arithmetic codegen, requires understanding LLVM overflow intrinsics, must handle all integer types correctly
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2 (with Tasks 6-11)
  - **Blocks**: Task 29 (test projects)
  - **Blocked By**: Task 1 (needs opal_runtime_error), Task 2 (uses shared helpers)

  **References**:

  **Pattern References**:
  - `src/codegen/expressions_numeric.rs:17-82` — `codegen_numeric_binop`: the debug_mode gate at line 58, unchecked path at lines 65-79, checked path call at line 60
  - `src/codegen/expressions_numeric.rs:84-154` — `codegen_checked_overflow_intrinsic`: the checked overflow implementation using LLVM `llvm.sadd.with.overflow.*` intrinsics

  **API/Type References**:
  - `src/codegen/expressions_numeric.rs:84` — `codegen_checked_overflow_intrinsic` signature and how it emits the overflow check + branch
  - Task 1's `opal_runtime_error` — the trap function to call when overflow detected

  **External References**:
  - LLVM `llvm.sadd.with.overflow` intrinsic family — returns `{result, overflow_flag}` struct

  **WHY Each Reference Matters**:
  - Lines 53-82 are the EXACT code to modify — the `if env.debug_mode` gate is what must be removed
  - Lines 84-154 show the checked path that should ALWAYS be taken — understand its structure before making the gate unconditional
  - The overflow intrinsic returns a struct with an overflow bit — the existing code already extracts this, we just need to ensure the trap path calls `opal_runtime_error`

  **Acceptance Criteria**:

  - [ ] `env.debug_mode` gate removed from `codegen_numeric_binop`
  - [ ] Checked overflow intrinsics used for ALL integer +/-/* in both debug and release
  - [ ] Overflow triggers call to `opal_runtime_error` with descriptive message
  - [ ] `cargo test` passes
  - [ ] Unit test verifies overflow codegen path

  **QA Scenarios (MANDATORY)**:

  ```
  Scenario: Overflow traps in codegen (unit test)
    Tool: Bash
    Preconditions: Tasks 1, 2, 5 changes applied
    Steps:
      1. Run `cargo test codegen 2>&1 | grep -E "test result|overflow"`
      2. Assert new overflow test passes
    Expected Result: New test passes, no existing tests broken
    Failure Indicators: Test failure mentioning overflow
    Evidence: .sisyphus/evidence/task-5-overflow-unit-test.txt

  Scenario: debug_mode gate is removed
    Tool: Bash
    Preconditions: Task 5 changes applied
    Steps:
      1. Run `grep -n "debug_mode" src/codegen/expressions_numeric.rs`
      2. Assert the debug_mode check is NOT used to gate overflow checking in codegen_numeric_binop
    Expected Result: No debug_mode gate controlling overflow intrinsic selection
    Failure Indicators: debug_mode still gates overflow path
    Evidence: .sisyphus/evidence/task-5-no-debug-gate.txt
  ```

  **Commit**: YES (group with Wave 2)
  - Message: `fix(codegen): always trap on integer overflow regardless of build mode (1a)`
  - Files: `src/codegen/expressions_numeric.rs`
  - Pre-commit: `cargo test`

---

- [x] 6. Fix float→int cast range guard — emit runtime trap on out-of-range (issue 1b)

  **What to do**:
  - RED: Write a codegen unit test that verifies float→int cast emits range comparison before fptosi/fptoui. Write integration test that compiles `let x: int32 = 1e20 as int32` and asserts runtime trap.
  - GREEN: In `src/codegen/expressions.rs` function `codegen_cast` (lines 416-431), BEFORE emitting fptosi/fptoui:
    - Compute the min/max representable values for the target integer type as float constants
    - Emit `fcmp` instructions: `value >= min_float` AND `value <= max_float`
    - Emit conditional branch: if out of range, call `opal_runtime_error("float-to-integer cast out of range")`
    - Only emit the fptosi/fptoui in the "in range" branch
  - Handle all combinations: float32→int8/16/32/64, float64→int8/16/32/64, float32→uint8/16/32/64, float64→uint8/16/32/64
  - Also handle NaN: `fcmp uno value, value` — NaN always traps
  - REFACTOR: Extract range constants into a helper function

  **Must NOT do**:
  - Do NOT implement checked_* or saturating_* cast variants
  - Do NOT change int→float cast direction (that's issue 1c)
  - Do NOT change widening integer casts

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: Requires precise LLVM IR emission with float comparisons, branch construction, and handling 16+ type combinations
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2 (with Tasks 5, 7-11)
  - **Blocks**: Task 15 (cast safety), Task 29 (test projects)
  - **Blocked By**: Task 1 (needs opal_runtime_error)

  **References**:

  **Pattern References**:
  - `src/codegen/expressions.rs:416-431` — Current `codegen_cast` float→int path: lines 419-431 emit fptosi/fptoui with NO range check
  - `src/codegen/expressions.rs:350-415` — Full `codegen_cast` function showing all cast type combinations and how branches are structured

  **API/Type References**:
  - `src/codegen/expressions.rs:741-759` — (or Task 2's shared location) `integer_literal_bits` — needed to compute min/max for each integer type
  - Inkwell `build_float_compare` — for emitting fcmp instructions
  - Inkwell `build_conditional_branch` — for branching on range check result

  **External References**:
  - LLVM `fptosi`/`fptoui` semantics — produces poison for out-of-range values

  **WHY Each Reference Matters**:
  - Lines 416-431 are the EXACT code to wrap with range checks — understand the current emission pattern
  - `integer_literal_bits` gives the bit width needed to compute min/max constants (e.g., int32: min=-2^31, max=2^31-1)
  - LLVM poison semantics explain why this is UB and must be fixed

  **Acceptance Criteria**:

  - [ ] Range comparison emitted before every fptosi/fptoui
  - [ ] NaN check emitted (fcmp uno)
  - [ ] Out-of-range or NaN triggers `opal_runtime_error` call
  - [ ] All float→int type combinations covered (2 float sizes × 8 int types = 16)
  - [ ] `cargo test` passes

  **QA Scenarios (MANDATORY)**:

  ```
  Scenario: Range check exists in codegen
    Tool: Bash
    Preconditions: Task 6 changes applied
    Steps:
      1. Run `cargo test 2>&1` — assert all pass
      2. Run `grep -n "opal_runtime_error\|range\|out.of.range\|float.*int.*cast" src/codegen/expressions.rs`
      3. Assert range-check related code found near the fptosi/fptoui calls
    Expected Result: Range check code present, tests pass
    Failure Indicators: No range check code, test failures
    Evidence: .sisyphus/evidence/task-6-range-check.txt

  Scenario: NaN handling present
    Tool: Bash
    Preconditions: Task 6 changes applied
    Steps:
      1. Run `grep -n "uno\|nan\|NaN\|is_nan\|float_compare" src/codegen/expressions.rs`
      2. Assert NaN check code found
    Expected Result: NaN check emitted before float→int cast
    Failure Indicators: No NaN handling code
    Evidence: .sisyphus/evidence/task-6-nan-check.txt
  ```

  **Commit**: YES (group with Wave 2)
  - Message: `fix(codegen): add range guard for float-to-int casts to prevent LLVM UB (1b)`
  - Files: `src/codegen/expressions.rs`
  - Pre-commit: `cargo test`

---

- [x] 7. Fix lambda body codegen — actually emit lambda bodies (issue 1d)

  **What to do**:
  - RED: Write a unit test creating a lambda `let add = f(a: int32, b: int32): int32 => return a + b` and calling it. Currently this returns 0 (default return). Test should assert the lambda returns the correct result.
  - GREEN: In `src/codegen/functions.rs` function `resolve_callee_function` (lines 434-472):
    - After creating the LLVM function for the lambda and its entry basic block (current code already does this)
    - INSTEAD of calling `emit_default_return`, recursively codegen the lambda's body statements
    - The lambda body AST should be available from the Expression::Lambda node — find where the body is stored and pass it to the statement codegen
    - Handle parameter bindings: map lambda parameters to LLVM function parameters (alloca + store pattern used for regular functions)
    - After body codegen, only emit default return if the body doesn't already terminate
  - REFACTOR: Ensure the lambda codegen follows the same pattern as regular function codegen in `codegen_function_definition`

  **Must NOT do**:
  - Do NOT change how lambdas are parsed or type-checked
  - Do NOT change the lambda calling convention
  - Do NOT implement closures/captures in this task (that's related to issue 1e)

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: Requires understanding the full function codegen pipeline and replicating it for lambdas — body codegen, parameter binding, return handling
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2 (with Tasks 5, 6, 8-11)
  - **Blocks**: Task 29 (test projects)
  - **Blocked By**: Task 1 (may need trap for error paths)

  **References**:

  **Pattern References**:
  - `src/codegen/functions.rs:434-472` — `resolve_callee_function` Lambda arm: creates LLVM function + entry block but ONLY calls `emit_default_return` (no body codegen)
  - `src/codegen/functions.rs:1-100` — `codegen_function_definition`: the CORRECT pattern for function body codegen — parameter alloca, body statements, return handling. Lambda should follow this pattern.

  **API/Type References**:
  - AST `Expression::Lambda` node — find the body field (likely `body: Vec<Statement>` or similar)
  - `src/codegen/statements.rs` — `codegen_statement` function that processes individual statements

  **WHY Each Reference Matters**:
  - Lines 434-472 are the EXACT location to fix — the Lambda match arm
  - `codegen_function_definition` shows the CORRECT pattern: alloca params → codegen body → handle return. Lambda needs the same flow.
  - The AST Lambda node tells us where the body is stored — the executor must find this in the AST types

  **Acceptance Criteria**:

  - [ ] Lambda bodies are code-generated (not just default return)
  - [ ] Lambda parameters correctly bound to LLVM function parameters
  - [ ] Default return only emitted when body doesn't terminate
  - [ ] `cargo test` passes
  - [ ] A lambda `f(x: int32): int32 => return x + 1` called with `5` returns `6`

  **QA Scenarios (MANDATORY)**:

  ```
  Scenario: Lambda body codegen exists
    Tool: Bash
    Preconditions: Task 7 changes applied
    Steps:
      1. Run `cargo test 2>&1` — assert all pass
      2. Run `grep -n "emit_default_return" src/codegen/functions.rs` and verify it's NOT the only thing in the Lambda arm
      3. Check that body codegen calls exist in the Lambda arm
    Expected Result: Lambda arm has body codegen, not just default return
    Failure Indicators: Lambda arm still only has emit_default_return
    Evidence: .sisyphus/evidence/task-7-lambda-body.txt

  Scenario: Existing tests still pass
    Tool: Bash
    Preconditions: Task 7 changes applied
    Steps:
      1. Run `cargo test 2>&1`
      2. Assert exit code 0, no failures
    Expected Result: All tests pass
    Failure Indicators: Any test failure
    Evidence: .sisyphus/evidence/task-7-tests-pass.txt
  ```

  **Commit**: YES (group with Wave 2)
  - Message: `fix(codegen): emit lambda bodies instead of default return zero (1d)`
  - Files: `src/codegen/functions.rs`
  - Pre-commit: `cargo test`

---

- [x] 8. Fix array bounds checking — emit runtime trap on out-of-bounds access (issue 1g)

  **What to do**:
  - RED: Write a test compiling an array access with an out-of-bounds index and asserting runtime trap.
  - GREEN: In `src/codegen/expressions.rs` function `codegen_array_access` (lines 493-525):
    - **Array length tracking (REQUIRED)**: Arrays currently have NO length metadata. You MUST add length tracking to enable bounds checks:
      1. Change array representation to a fat pointer `{len: i64, ptr: T*}` struct, OR add a `length` field to `VariableBinding` / the internal representation used during codegen — whichever is more consistent with the existing architecture.
      2. Update array allocation sites (in `expressions.rs`, `statements.rs`, or wherever arrays are created) to store the length alongside the data pointer.
      3. Update array access to extract the length before the bounds check.
      4. For function parameters receiving arrays: the length must be passed alongside the pointer. This may require changing the ABI for array-typed parameters (add a second parameter for length, or pass the fat pointer struct).
    - Before the GEP instruction, emit code to:
      1. Load the array length from the fat pointer / binding
      2. Emit `icmp uge index, length` (unsigned greater-or-equal comparison)
      3. Emit conditional branch: if out of bounds, call `opal_runtime_error("array index out of bounds")`
    - Also check for negative index if index type is signed: `icmp slt index, 0`
    - Fix potential GEP issue: verify whether `[0, index]` vs `[index]` is correct for the array type
  - REFACTOR: Extract bounds checking into a helper function for reuse

  **Must NOT do**:
  - Do NOT add array resizing or dynamic arrays
  - Do NOT implement growable/dynamic array types — keep arrays fixed-size
  - Do NOT change the user-facing syntax for arrays (the `.op` source syntax stays the same)

  **Scope clarification (from Momus review)**:
  - Changing array allocation/creation IS allowed and EXPECTED — you cannot bounds-check without knowing the length
  - Changing how arrays are passed to functions IS allowed — length must travel with the pointer
  - These are INTERNAL representation changes, not user-facing syntax changes

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: Requires understanding LLVM array types, GEP semantics, and how arrays are represented in the Opalescent runtime
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2 (with Tasks 5-7, 9-11)
  - **Blocks**: Task 29 (test projects)
  - **Blocked By**: Task 1 (needs opal_runtime_error)

  **References**:

  **Pattern References**:
  - `src/codegen/expressions.rs:493-525` — `codegen_array_access`: current GEP-based array access with NO bounds checking — this is where bounds checks must be inserted
  - `src/codegen/expressions.rs:416-431` — Float→int range check pattern (Task 6) — similar comparison+branch structure to follow
  - `src/codegen/statements.rs:200-225` — `codegen_assignment`: check here for array creation/allocation sites that need length tracking added
  - `src/codegen/expressions.rs` — Search for `alloca` and `array` to find all array allocation sites

  **API/Type References**:
  - `src/codegen/values.rs` or wherever `VariableBinding` is defined — may need a `length` field added for array bindings
  - How arrays are currently represented in LLVM IR — check if they're `[N x T]` (fixed-size LLVM arrays) or `T*` (pointers)
  - Inkwell `build_int_compare` — for emitting icmp
  - Inkwell `build_conditional_branch` — for the bounds check branch

  **WHY Each Reference Matters**:
  - Lines 493-525 are the EXACT location to add bounds checking — before the GEP
  - The range check pattern from Task 6 shows how to structure comparison + conditional branch + trap call
  - Array allocation sites MUST be found and modified to store length — this is a prerequisite for bounds checking
  - `VariableBinding` or equivalent is where length metadata should be stored during codegen

  **Acceptance Criteria**:

  - [ ] Bounds check emitted before every array access GEP
  - [ ] Out-of-bounds index triggers `opal_runtime_error` with descriptive message
  - [ ] Negative index check for signed index types
  - [ ] GEP uses correct indexing pattern for array type
  - [ ] `cargo test` passes

  **QA Scenarios (MANDATORY)**:

  ```
  Scenario: Bounds check code exists
    Tool: Bash
    Preconditions: Task 8 changes applied
    Steps:
      1. Run `cargo build 2>&1` — assert success
      2. Run `cargo test 2>&1` — assert all pass
      3. Run `grep -n "bounds\|out.of.bounds\|opal_runtime_error" src/codegen/expressions.rs`
      4. Assert bounds check code found near array access
    Expected Result: Bounds check present, build and tests pass
    Failure Indicators: No bounds check code, build/test failures
    Evidence: .sisyphus/evidence/task-8-bounds-check.txt
  ```

  **Commit**: YES (group with Wave 2)
  - Message: `fix(codegen): add array bounds checking with runtime trap (1g)`
  - Files: `src/codegen/expressions.rs`, `src/codegen/statements.rs` (if array allocation modified), `src/codegen/values.rs` (if VariableBinding modified), any other files touched for array length tracking
  - Pre-commit: `cargo test`

---

- [x] 9. Fix string interpolation buffer overflow — replace fixed 256-byte buffer (issue 3c)

  **What to do**:
  - RED: Write a test with a string interpolation that exceeds 256 bytes (e.g., interpolating a long string or many values). Currently this would overflow the buffer.
  - GREEN: In `src/codegen/expressions_string.rs` (around line 32):
    - Replace the fixed `malloc(256)` with a dynamic sizing approach:
      - Option A (simplest): Use `snprintf` with NULL first to compute required size, then malloc that size + 1, then `snprintf` again
      - Option B: Emit LLVM IR that computes string lengths at runtime, allocates exact size
    - Replace `sprintf` calls with `snprintf` for safety
    - Ensure the `ensure_sprintf` and `ensure_malloc` helper functions (lines 170-207) are used correctly
  - REFACTOR: Remove any remaining fixed-size buffer patterns

  **Must NOT do**:
  - Do NOT change string representation or encoding
  - Do NOT implement a full string builder — keep it simple
  - Do NOT add GC or arena allocation for strings (that's issues 3a/3b)

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
    - Reason: Requires understanding LLVM IR string operations and potentially complex buffer management
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2 (with Tasks 5-8, 10-11)
  - **Blocks**: Task 29 (test projects)
  - **Blocked By**: None (doesn't need trap function for this)

  **References**:

  **Pattern References**:
  - `src/codegen/expressions_string.rs:14-74` — String interpolation codegen — the 256-byte malloc is at line 32
  - `src/codegen/expressions_string.rs:170-187` — `ensure_sprintf`: helper declaring sprintf in LLVM
  - `src/codegen/expressions_string.rs:190-207` — `ensure_malloc`: helper declaring malloc in LLVM

  **External References**:
  - `snprintf` semantics: returns required size when called with size=0 and NULL buffer

  **WHY Each Reference Matters**:
  - Line 32 is the EXACT buffer allocation to fix
  - `ensure_sprintf`/`ensure_malloc` are the LLVM function declarations used — may need to add `ensure_snprintf` similarly
  - The snprintf trick (call with NULL to get size) is the simplest correct fix

  **Acceptance Criteria**:

  - [ ] No fixed 256-byte buffer in string interpolation
  - [ ] Dynamic allocation based on actual required size
  - [ ] `snprintf` used instead of `sprintf` (or equivalent safe approach)
  - [ ] `cargo test` passes
  - [ ] Long string interpolation (>256 bytes) works correctly

  **QA Scenarios (MANDATORY)**:

  ```
  Scenario: No fixed-size buffer allocation
    Tool: Bash
    Preconditions: Task 9 changes applied
    Steps:
      1. Run `grep -n "256\|fixed.*buf\|malloc(256)" src/codegen/expressions_string.rs`
      2. Assert no fixed 256-byte allocation found
      3. Run `cargo build 2>&1` — assert success
      4. Run `cargo test 2>&1` — assert all pass
    Expected Result: No 256-byte buffer, build and tests pass
    Failure Indicators: Fixed buffer found, build/test failure
    Evidence: .sisyphus/evidence/task-9-dynamic-buffer.txt
  ```

  **Commit**: YES (group with Wave 2)
  - Message: `fix(codegen): replace fixed 256-byte string interpolation buffer with dynamic allocation (3c)`
  - Files: `src/codegen/expressions_string.rs`
  - Pre-commit: `cargo test`

---

- [x] 10. Fix missing captured vars — emit error instead of silent zero (issue 1e)

  **What to do**:
  - RED: Write a test that triggers the captured-var-not-found path and verifies it produces a CodegenError (not silent zero).
  - GREEN: In `src/codegen/functions.rs` function `codegen_call_expression` (lines 191-200):
    - Replace the `const_zero()` fallback (line 199) with a `CodegenError` (or compilation error)
    - When a capture name is not found in `env.variables`, return `Err(CodegenError::MissingCapture { name: capture_name.clone() })` or similar
    - Add the `MissingCapture` variant to the CodegenError enum if it doesn't exist
  - REFACTOR: Ensure the error message includes the capture name for debuggability

  **Must NOT do**:
  - Do NOT implement actual closure capture mechanism — just fail loudly instead of silently
  - Do NOT change how captures are declared or type-checked

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
    - Reason: Needs to understand the capture lowering path and correctly propagate errors through the codegen pipeline
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2 (with Tasks 5-9, 11)
  - **Blocks**: Task 29 (test projects)
  - **Blocked By**: Task 1 (needs opal_runtime_error for runtime traps)

  **References**:

  **Pattern References**:
  - `src/codegen/functions.rs:180-202` — `codegen_call_expression` capture lowering: iterates captures, looks up in `env.variables`, pushes `const_zero()` at line 199 when not found
  - `src/codegen/functions.rs:191-199` — The specific fallback: `else { env.context.i64_type().const_zero() }`

  **API/Type References**:
  - CodegenError enum — find where it's defined to add new variant
  - `env.variables` — HashMap<String, VariableBinding> — the lookup target

  **WHY Each Reference Matters**:
  - Line 199 is the EXACT const_zero fallback to replace with an error
  - The CodegenError enum needs a new variant for this specific failure case
  - Understanding `env.variables` helps explain why the lookup might fail (e.g., capture not in scope)

  **Acceptance Criteria**:

  - [ ] `const_zero()` fallback removed from capture path
  - [ ] Missing capture produces a CodegenError with the capture variable name
  - [ ] `cargo test` passes
  - [ ] Error message is descriptive (includes capture name)

  **QA Scenarios (MANDATORY)**:

  ```
  Scenario: const_zero fallback removed
    Tool: Bash
    Preconditions: Task 10 changes applied
    Steps:
      1. Run `grep -n "const_zero" src/codegen/functions.rs`
      2. Verify the const_zero in the capture-lookup fallback (around line 199) is gone
      3. Run `grep -n "MissingCapture\|missing.*capture\|capture.*error" src/codegen/functions.rs`
      4. Assert error handling code present
      5. Run `cargo test 2>&1` — assert all pass
    Expected Result: No const_zero in capture path, error handling present, tests pass
    Failure Indicators: const_zero still in capture path, no error handling
    Evidence: .sisyphus/evidence/task-10-capture-error.txt
  ```

  **Commit**: YES (group with Wave 2)
  - Message: `fix(codegen): emit error for missing captured variables instead of silent zero (1e)`
  - Files: `src/codegen/functions.rs`, codegen error enum file
  - Pre-commit: `cargo test`

---

- [x] 11. Fix `emit_default_return` — trap instead of silent zero for non-void functions (issue 1f)

  **What to do**:
  - RED: Write a test for a non-void function that falls through without a return statement, and verify it produces a trap (not silent zero).
  - GREEN: In `src/codegen/functions.rs` function `emit_default_return` (lines 505-540):
    - For void functions: keep current behavior (return void)
    - For non-void functions: instead of emitting `const_zero()`, emit a call to `opal_runtime_error("function reached end without returning a value")`
    - The trap should occur at RUNTIME if the code path is actually reached (the function might still have valid return statements in other branches)
  - REFACTOR: Consider if the type checker should catch this at compile time (but that's beyond scope — just add the runtime safety net)

  **Must NOT do**:
  - Do NOT change void function return behavior
  - Do NOT add compile-time exhaustive return checking (that's a type checker enhancement, not in the 28 issues)

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
    - Reason: Straightforward change but needs careful handling of void vs non-void distinction
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2 (with Tasks 5-10)
  - **Blocks**: Task 29 (test projects)
  - **Blocked By**: Task 1 (needs opal_runtime_error)

  **References**:

  **Pattern References**:
  - `src/codegen/functions.rs:505-540` — `emit_default_return`: currently returns `const_zero()` for ALL non-void types (int, float, string, etc.)
  - `src/codegen/functions.rs:505` — Function signature showing parameters: takes return CoreType

  **API/Type References**:
  - Task 1's `opal_runtime_error` — the trap function to call
  - CoreType enum — to distinguish void from non-void return types

  **WHY Each Reference Matters**:
  - Lines 505-540 are the EXACT function to modify — the match on return type that produces const_zero for non-void
  - The void case should remain unchanged, only non-void gets the trap

  **Acceptance Criteria**:

  - [ ] Void functions still return void normally
  - [ ] Non-void functions emit `opal_runtime_error` call instead of const_zero
  - [ ] `cargo test` passes
  - [ ] Error message is descriptive

  **QA Scenarios (MANDATORY)**:

  ```
  Scenario: Non-void default return emits trap
    Tool: Bash
    Preconditions: Task 11 changes applied
    Steps:
      1. Run `grep -n "const_zero\|opal_runtime_error" src/codegen/functions.rs`
      2. Verify emit_default_return calls opal_runtime_error for non-void types
      3. Run `cargo test 2>&1` — assert all pass
    Expected Result: Non-void path uses trap, void path unchanged, tests pass
    Failure Indicators: const_zero still used for non-void, test failures
    Evidence: .sisyphus/evidence/task-11-default-return-trap.txt
  ```

  **Commit**: YES (group with Wave 2)
  - Message: `fix(codegen): trap on implicit return in non-void functions instead of silent zero (1f)`
  - Files: `src/codegen/functions.rs`
  - Pre-commit: `cargo test`

---

### Wave 3 — P1 Correctness + Spec

- [x] 12. Fix unsigned int→float — use `uitofp` instead of `sitofp` (issue 1c)

  **What to do**:
  - RED: Write a test casting `uint64 max_value` (18446744073709551615) to float64 and verifying the result is correct (not a negative number, which sitofp would produce for large unsigned values).
  - GREEN: In `src/codegen/expressions.rs` function `codegen_cast` (lines 385-391):
    - Currently uses `build_signed_int_to_float` (`sitofp`) unconditionally
    - Add a check using the shared `is_signed_core_type` helper (from Task 2):
      - If source type is signed → `build_signed_int_to_float` (sitofp)
      - If source type is unsigned → `build_unsigned_int_to_float` (uitofp)
  - REFACTOR: Verify all int→float combinations produce correct results

  **Must NOT do**:
  - Do NOT change float→int cast direction (that's issue 1b/Task 6)
  - Do NOT add range checking for int→float (int→float is always representable, though may lose precision)

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
    - Reason: Small change but requires understanding LLVM int→float instruction selection
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 3 (with Tasks 13-16)
  - **Blocks**: Task 29 (test projects)
  - **Blocked By**: Task 2 (needs shared `is_signed_core_type`)

  **References**:

  **Pattern References**:
  - `src/codegen/expressions.rs:385-391` — Current int→float cast: unconditionally uses `build_signed_int_to_float`
  - Task 2's shared `is_signed_core_type` function — use this to branch on signedness

  **WHY Each Reference Matters**:
  - Lines 385-391 are the EXACT 6 lines to modify — add signedness check and branch to correct instruction

  **Acceptance Criteria**:

  - [ ] Signed int→float uses `sitofp` (build_signed_int_to_float)
  - [ ] Unsigned int→float uses `uitofp` (build_unsigned_int_to_float)
  - [ ] `cargo test` passes
  - [ ] Large unsigned values (e.g., uint64 max) convert correctly to float

  **QA Scenarios (MANDATORY)**:

  ```
  Scenario: Signedness-aware int→float conversion
    Tool: Bash
    Preconditions: Task 12 changes applied
    Steps:
      1. Run `grep -n "unsigned_int_to_float\|uitofp\|is_signed" src/codegen/expressions.rs`
      2. Assert both signed and unsigned conversion paths exist
      3. Run `cargo test 2>&1` — assert all pass
    Expected Result: Both sitofp and uitofp paths present, tests pass
    Failure Indicators: Only sitofp found, test failures
    Evidence: .sisyphus/evidence/task-12-uint-to-float.txt
  ```

  **Commit**: YES (group with Wave 3)
  - Message: `fix(codegen): use uitofp for unsigned int-to-float casts (1c)`
  - Files: `src/codegen/expressions.rs`
  - Pre-commit: `cargo test`

---

- [x] 13. Add `pure` keyword — lexer, parser, type checker enforcement (issue 4a)

  **What to do**:
  - RED: Write tests that:
    - Verify `pure` is lexed as a keyword token (not an identifier)
    - Verify a function declared `pure` that calls `print()` produces a type error
    - Verify a `pure` function that only does math compiles successfully
  - GREEN:
    - Add `Pure` variant to `TokenType` enum in `src/token.rs`
    - Add `"pure"` to `RESERVED_KEYWORDS` in `src/lexer.rs`
    - Add parser support: parse `pure` before function declaration (e.g., `let pure add = f(...)`)
    - Add type checker enforcement: a `pure` function may NOT call impure functions (no I/O, no mutation of external state). At minimum, reject calls to known impure stdlib functions (print, take_input, random_*, etc.)
    - Add AST field: `is_pure: bool` on function declaration nodes
  - REFACTOR: Ensure pure/impure tracking integrates with existing function metadata

  **Must NOT do**:
  - Do NOT implement full purity analysis (transitive purity checking is complex)
  - Keep it simple: check direct calls to known-impure functions
  - Do NOT change existing function behavior — only add opt-in purity checking

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
    - Reason: Cross-cutting change touching lexer, parser, AST, and type checker — but each change is small
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 3 (with Tasks 12, 14-16)
  - **Blocks**: None
  - **Blocked By**: None

  **References**:

  **Pattern References**:
  - `src/token.rs` — TokenType enum: add `Pure` variant following existing keyword pattern (see `Entry`, `Mutable`, `Match`, etc.)
  - `src/lexer.rs` — `RESERVED_KEYWORDS`: add `"pure"` → `TokenType::Pure` following existing entries
  - `src/type_system/checker.rs` — TypeChecker: where function purity would be tracked
  - `language-spec/requirements/overview.md` — Spec reference for pure keyword semantics

  **WHY Each Reference Matters**:
  - `token.rs` and `lexer.rs` show the existing pattern for adding keywords — follow exactly
  - TypeChecker is where enforcement happens — need to find function call checking logic
  - The spec defines what `pure` means in this language

  **Acceptance Criteria**:

  - [ ] `Pure` token type exists in `src/token.rs`
  - [ ] `"pure"` is a reserved keyword in `src/lexer.rs`
  - [ ] Parser handles `pure` on function declarations
  - [ ] Type checker rejects `pure` functions calling impure functions
  - [ ] `cargo test` passes

  **QA Scenarios (MANDATORY)**:

  ```
  Scenario: pure keyword is lexed correctly
    Tool: Bash
    Preconditions: Task 13 changes applied
    Steps:
      1. Run `grep -n "Pure\|pure" src/token.rs` — assert Pure variant exists
      2. Run `grep -n '"pure"' src/lexer.rs` — assert keyword registered
      3. Run `cargo test 2>&1` — assert all pass
    Expected Result: Pure keyword added to lexer and token, tests pass
    Failure Indicators: Keyword not found, test failures
    Evidence: .sisyphus/evidence/task-13-pure-keyword.txt

  Scenario: Pure function calling impure function is rejected
    Tool: Bash
    Preconditions: Task 13 changes applied, including a Rust test named `pure_function_rejects_impure_call`
    Steps:
      1. Run `cargo test pure_function_rejects_impure_call 2>&1`
      2. Assert test passes (exit code 0)
      3. The test itself should compile this Opalescent snippet:
         ```
         let pure bad = f(): void =>
             print('hello')
             return void
         ```
         and assert the type checker produces an error containing "pure" and "impure" (or similar diagnostic)
      4. Run `cargo test pure_function_allows_math 2>&1` — a companion test compiling:
         ```
         let pure add = f(a: int32, b: int32): int32 =>
             return a + b
         ```
         and asserting it compiles WITHOUT errors
    Expected Result: `pure_function_rejects_impure_call` PASSES (type error produced); `pure_function_allows_math` PASSES (no error)
    Failure Indicators: Either test fails, or no diagnostic is produced for the impure call
    Evidence: .sisyphus/evidence/task-13-pure-enforcement.txt
  ```

  **Commit**: YES (group with Wave 3)
  - Message: `feat(lang): add pure keyword with basic impurity detection (4a)`
  - Files: `src/token.rs`, `src/lexer.rs`, parser file, `src/type_system/checker.rs`
  - Pre-commit: `cargo test`

---

- [x] 14. Add `untested` keyword — lexer, parser, type checker enforcement (issue 4b)

  **What to do**:
  - RED: Write tests that:
    - Verify `untested` is lexed as a keyword token
    - Verify a function NOT marked `untested` that lacks tests produces a warning/error
    - Verify `entry main` implicitly has `untested` (per spec)
  - GREEN:
    - Add `Untested` variant to `TokenType` enum in `src/token.rs`
    - Add `"untested"` to `RESERVED_KEYWORDS` in `src/lexer.rs`
    - Add parser support: parse `untested` before function declaration
    - Add AST field: `is_untested: bool` on function declaration nodes
    - Add type checker: `entry` functions implicitly have `untested`. Non-entry functions without tests MAY produce a warning (the spec is light on exact enforcement — implement as warning, not error)
  - REFACTOR: Ensure untested integrates with test runner metadata

  **Must NOT do**:
  - Do NOT implement test discovery/linking — just the keyword and basic annotation
  - Do NOT make missing tests a hard error (use warning)

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
    - Reason: Same cross-cutting pattern as Task 13 (pure keyword)
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 3 (with Tasks 12, 13, 15, 16)
  - **Blocks**: None
  - **Blocked By**: None

  **References**:

  **Pattern References**:
  - Same as Task 13 — follow identical pattern for keyword addition
  - `src/token.rs`, `src/lexer.rs` — same files, add `Untested` variant and `"untested"` keyword

  **WHY Each Reference Matters**:
  - Identical pattern to Task 13 — ensures consistency between keyword additions

  **Acceptance Criteria**:

  - [ ] `Untested` token type exists
  - [ ] `"untested"` is reserved keyword
  - [ ] Parser handles `untested` on function declarations
  - [ ] `entry` functions implicitly marked untested
  - [ ] `cargo test` passes

  **QA Scenarios (MANDATORY)**:

  ```
  Scenario: untested keyword is lexed and parsed
    Tool: Bash
    Preconditions: Task 14 changes applied
    Steps:
      1. Run `grep -n "Untested\|untested" src/token.rs` — assert variant exists
      2. Run `grep -n '"untested"' src/lexer.rs` — assert keyword registered
      3. Run `cargo test 2>&1` — assert all pass
    Expected Result: Untested keyword added, tests pass
    Failure Indicators: Keyword not found, test failures
    Evidence: .sisyphus/evidence/task-14-untested-keyword.txt
  ```

  **Commit**: YES (group with Wave 3)
  - Message: `feat(lang): add untested keyword with implicit entry annotation (4b)`
  - Files: `src/token.rs`, `src/lexer.rs`, parser file, type checker
  - Pre-commit: `cargo test`

---

- [ ] 15. Cast safety matching spec — compile-time constant detection + runtime traps (issue 4c)

  **What to do**:
  - RED: Write tests:
    - Constant cast `let x: int8 = 1000 as int8` should produce compile error (out of range for int8)
    - Runtime cast with variable should produce runtime trap if out of range
  - GREEN: In `src/codegen/expressions.rs` `codegen_cast` and/or the type checker:
    - For compile-time constant expressions: evaluate the cast at compile time. If out of range, emit a compile error.
    - For runtime values: Task 6 already adds float→int range guards. Extend to cover:
      - Narrowing integer casts (e.g., int64→int8): add range check before `build_int_truncate`
      - Signed↔unsigned reinterpretation: add appropriate checks
    - Use `opal_runtime_error("cast out of range: {source_type} to {target_type}")` for runtime traps
  - REFACTOR: Organize cast checking into a clear hierarchy

  **Must NOT do**:
  - Do NOT implement checked_*/saturating_* cast APIs (future work)
  - Do NOT change widening casts (always safe)

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: Complex matrix of source×target type combinations, needs both compile-time and runtime handling
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 3 (with Tasks 12-14, 16)
  - **Blocks**: Task 29 (test projects)
  - **Blocked By**: Task 1 (trap function), Task 6 (float→int guards to extend)

  **References**:

  **Pattern References**:
  - `src/codegen/expressions.rs:350-438` — Full `codegen_cast` function
  - Task 6's range check pattern — reuse for narrowing integer casts
  - `language-spec/requirements/overview.md:41-48` — Spec on cast safety

  **WHY Each Reference Matters**:
  - The full `codegen_cast` shows all cast paths — need to add checks to narrowing and sign-change paths
  - Task 6's pattern can be reused: compare value against target type's min/max, branch to trap if out of range

  **Acceptance Criteria**:

  - [ ] Compile-time constant out-of-range casts produce compile error
  - [ ] Runtime narrowing casts have range checks + trap
  - [ ] All existing safe casts (widening) unchanged
  - [ ] `cargo test` passes

  **QA Scenarios (MANDATORY)**:

  ```
  Scenario: Narrowing cast has runtime check
    Tool: Bash
    Preconditions: Task 15 changes applied
    Steps:
      1. Run `cargo test 2>&1` — assert all pass
      2. Run `grep -n "opal_runtime_error\|range\|narrow\|truncat" src/codegen/expressions.rs`
      3. Assert range check code present for narrowing casts
    Expected Result: Range checks for narrowing casts, tests pass
    Failure Indicators: No range checks, test failures
    Evidence: .sisyphus/evidence/task-15-cast-safety.txt
  ```

  **Commit**: YES (group with Wave 3)
  - Message: `fix(codegen): add compile-time and runtime cast safety checks per spec (4c)`
  - Files: `src/codegen/expressions.rs`, type checker files
  - Pre-commit: `cargo test`

---

- [x] 16. Immutability enforcement in codegen (issue 4d)

  **What to do**:
  - RED: Write a test that attempts to assign to an immutable variable in codegen and verifies it produces an error (currently it silently succeeds at codegen level even though type checker catches it).
  - GREEN:
    - Add `is_mutable: bool` field to `VariableBinding` struct in `src/codegen/expressions.rs` (lines 58-61)
    - Set `is_mutable` correctly when creating VariableBindings (from AST variable declarations)
    - In `src/codegen/statements.rs` `codegen_assignment` (lines 200-225): add check before `build_store` — if `!binding.is_mutable`, emit `CodegenError::ImmutableAssignment` or call `opal_runtime_error`
    - Note: The type checker at `src/type_system/checker/statements.rs:527-539` already checks this. The codegen check is defense-in-depth.
  - REFACTOR: Ensure all VariableBinding creation sites set is_mutable correctly

  **Must NOT do**:
  - Do NOT change type checker immutability rules
  - Do NOT add mutability to types that don't have it (parameters, etc.)

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
    - Reason: Touches struct definition + all creation sites + assignment codegen — moderate spread
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 3 (with Tasks 12-15)
  - **Blocks**: Task 29 (test projects)
  - **Blocked By**: None

  **References**:

  **Pattern References**:
  - `src/codegen/expressions.rs:58-61` — `VariableBinding { alloca, core_type }`: add `is_mutable` field here
  - `src/codegen/statements.rs:200-225` — `codegen_assignment`: add mutability check before `build_store`
  - `src/type_system/checker/statements.rs:527-539` — Existing type checker `ImmutableAssignment` check — this is the authority; codegen is defense-in-depth

  **WHY Each Reference Matters**:
  - `VariableBinding` struct is where to add the field
  - `codegen_assignment` is where to check it
  - The type checker check shows what the error should look like

  **Acceptance Criteria**:

  - [ ] `VariableBinding` has `is_mutable: bool` field
  - [ ] All VariableBinding creation sites set `is_mutable` correctly
  - [ ] `codegen_assignment` checks `is_mutable` before `build_store`
  - [ ] Immutable assignment produces error (defense-in-depth)
  - [ ] `cargo test` passes

  **QA Scenarios (MANDATORY)**:

  ```
  Scenario: VariableBinding has is_mutable field
    Tool: Bash
    Preconditions: Task 16 changes applied
    Steps:
      1. Run `grep -n "is_mutable" src/codegen/expressions.rs`
      2. Assert field exists in VariableBinding struct
      3. Run `grep -n "is_mutable" src/codegen/statements.rs`
      4. Assert check exists in codegen_assignment
      5. Run `cargo test 2>&1` — assert all pass
    Expected Result: Field and check present, tests pass
    Failure Indicators: Field missing, check missing, test failures
    Evidence: .sisyphus/evidence/task-16-immutability.txt
  ```

  **Commit**: YES (group with Wave 3)
  - Message: `fix(codegen): enforce immutability as defense-in-depth in codegen (4d)`
  - Files: `src/codegen/expressions.rs`, `src/codegen/statements.rs`
  - Pre-commit: `cargo test`

---

### Wave 4 — P1 Memory + Portability

- [ ] 17. Add `free()` calls in runtime — scope-based deallocation (issue 3a)

  **What to do**:
  - RED: Write a test (or runtime analysis) identifying allocation sites and verifying corresponding free() calls exist after the fix.
  - GREEN: In `runtime/opal_runtime.c`:
    - Audit all 12 allocation sites (10 malloc + 2 strdup) — cataloged in research
    - Design a simple ownership model: each `*_to_string()` function returns a newly allocated string that the CALLER must free
    - For functions that return string pointers to callers: document that the caller owns the memory
    - For internal helper functions: free intermediate strings before returning
    - Add `free()` calls for:
      - `invalid_digit_error` static buffer → use strdup pattern or thread_local (overlaps with 2a)
      - Each `*_to_string` return value: caller (codegen) must free after use
    - NOTE: The codegen side (issue 3b/Task 18) handles freeing strings returned by runtime to LLVM-generated code
  - REFACTOR: Add comments documenting ownership for each function

  **Must NOT do**:
  - Do NOT implement garbage collection
  - Do NOT add reference counting
  - Keep it simple: explicit free at scope exit

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: Requires careful ownership analysis across 45 runtime functions to avoid use-after-free or double-free
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 4 (with Tasks 18, 19)
  - **Blocks**: Task 27 (runtime split depends on clean runtime)
  - **Blocked By**: None

  **References**:

  **Pattern References**:
  - `runtime/opal_runtime.c` — All 12 allocation sites: look for `malloc`, `strdup`, `calloc`
  - `runtime/opal_runtime.c:566-571` — `int64_to_string`: allocates with malloc, returns to caller
  - `runtime/opal_runtime.c:594-599` — `uint64_to_string`: same pattern

  **WHY Each Reference Matters**:
  - Each allocation site needs a corresponding deallocation strategy
  - `*_to_string` functions show the primary ownership pattern: runtime allocates, codegen/caller frees

  **Acceptance Criteria**:

  - [ ] Every malloc/strdup site has documented ownership
  - [ ] Internal intermediate allocations have free() calls
  - [ ] Ownership comments added for caller-owned returns
  - [ ] `cargo build` succeeds
  - [ ] No double-free or use-after-free (verified by running test projects)

  **QA Scenarios (MANDATORY)**:

  ```
  Scenario: Free calls added for internal allocations
    Tool: Bash
    Preconditions: Task 17 changes applied
    Steps:
      1. Run `grep -c "free(" runtime/opal_runtime.c`
      2. Assert count > 0 (currently 0)
      3. Run `cargo build 2>&1` — assert success
    Expected Result: free() calls present, build succeeds
    Failure Indicators: Zero free calls, build failure
    Evidence: .sisyphus/evidence/task-17-runtime-free.txt

  Scenario: Test projects still run correctly
    Tool: Bash
    Preconditions: Task 17 changes applied
    Steps:
      1. Run `cargo test --features integration 2>&1` (if available)
      2. Assert all pass
    Expected Result: No behavioral regression from adding free()
    Failure Indicators: Use-after-free crash, wrong output
    Evidence: .sisyphus/evidence/task-17-no-regression.txt
  ```

  **Commit**: YES (group with Wave 4)
  - Message: `fix(runtime): add free() calls for runtime string allocations with ownership model (3a)`
  - Files: `runtime/opal_runtime.c`
  - Pre-commit: `cargo build`

---

- [ ] 18. Add `free()` calls in codegen — emit LLVM `free` after string temporaries (issue 3b)

  **What to do**:
  - RED: Write a test verifying that codegen emits `free()` calls for temporary strings returned by runtime functions.
  - GREEN: In `src/codegen/expressions_string.rs` and other codegen files:
    - After using a string returned by a runtime function (e.g., `int64_to_string`, string concatenation), emit a `free()` call to release the memory
    - Declare `free` as an LLVM function if not already done (similar to `ensure_malloc`)
    - Key locations:
      - String interpolation: free intermediate sprintf results after concatenation
      - String conversion: free `*_to_string` results after they're consumed
    - Be careful with strings that are still needed (e.g., stored in variables) — only free temporaries
  - REFACTOR: Create an `ensure_free` helper function similar to `ensure_malloc`/`ensure_sprintf`

  **Must NOT do**:
  - Do NOT free string constants (they're in static memory)
  - Do NOT free strings stored in variables (they're owned by the scope)
  - Only free truly temporary intermediate strings

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: Requires precise understanding of string lifetime in LLVM IR — freeing too early causes use-after-free, too late causes leaks
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 4 (with Tasks 17, 19)
  - **Blocks**: None
  - **Blocked By**: None

  **References**:

  **Pattern References**:
  - `src/codegen/expressions_string.rs:14-74` — String interpolation codegen — intermediate malloc results should be freed after use
  - `src/codegen/expressions_string.rs:190-207` — `ensure_malloc` helper — model `ensure_free` on this pattern

  **WHY Each Reference Matters**:
  - String interpolation is the primary source of temporary string allocations in codegen
  - `ensure_malloc` shows the pattern for declaring C functions in LLVM — follow for `ensure_free`

  **Acceptance Criteria**:

  - [ ] `ensure_free` helper function created
  - [ ] `free()` calls emitted for temporary string allocations
  - [ ] String constants NOT freed
  - [ ] `cargo test` passes
  - [ ] No use-after-free (test projects produce correct output)

  **QA Scenarios (MANDATORY)**:

  ```
  Scenario: Free calls emitted in codegen
    Tool: Bash
    Preconditions: Task 18 changes applied
    Steps:
      1. Run `grep -n "ensure_free\|free(" src/codegen/expressions_string.rs`
      2. Assert free-related code present
      3. Run `cargo test 2>&1` — assert all pass
    Expected Result: Free calls in codegen, tests pass
    Failure Indicators: No free calls, test failures
    Evidence: .sisyphus/evidence/task-18-codegen-free.txt
  ```

  **Commit**: YES (group with Wave 4)
  - Message: `fix(codegen): emit free() for temporary string allocations (3b)`
  - Files: `src/codegen/expressions_string.rs`
  - Pre-commit: `cargo test`

---

- [ ] 19. Cross-platform linker support (issue 6d)

  **What to do**:
  - RED: Write a test for the linker command construction that verifies correct flags for different platforms.
  - GREEN: In `src/compiler.rs` `link_object_file` (lines 263-281):
    - Currently hardcodes `cc` as linker and `-no-pie` flag (Linux-only)
    - Add platform detection:
      - Linux: `cc` with `-no-pie` (current behavior)
      - macOS: `cc` without `-no-pie` (macOS doesn't need it and some versions don't support it)
      - Windows/MSVC: `link.exe` with appropriate flags (or `lld-link`)
      - Windows/MinGW: `gcc` with MinGW-specific flags
    - Use `std::env::consts::OS` and `std::env::consts::ARCH` for detection
    - Support cross-compilation via target triple from `opal.toml`
  - REFACTOR: Extract linker command construction into a separate function for testability

  **Must NOT do**:
  - Do NOT implement full cross-compilation support (just correct native linking per platform)
  - Do NOT add new dependencies for platform detection (std is enough)

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
    - Reason: Platform-specific logic with multiple branches but each is straightforward
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 4 (with Tasks 17, 18)
  - **Blocks**: None
  - **Blocked By**: None

  **References**:

  **Pattern References**:
  - `src/compiler.rs:263-281` — `link_object_file`: current Linux-only linker invocation

  **WHY Each Reference Matters**:
  - Lines 263-281 are the EXACT function to extend with platform-specific linker commands

  **Acceptance Criteria**:

  - [ ] Linker command varies by platform (Linux/macOS/Windows)
  - [ ] Current Linux behavior preserved
  - [ ] macOS omits `-no-pie`
  - [ ] Windows uses appropriate linker
  - [ ] `cargo test` passes
  - [ ] Linker logic extracted into testable function

  **QA Scenarios (MANDATORY)**:

  ```
  Scenario: Platform-specific linker code exists
    Tool: Bash
    Preconditions: Task 19 changes applied
    Steps:
      1. Run `grep -n "OS\|macos\|windows\|linux\|platform\|consts::OS" src/compiler.rs`
      2. Assert platform detection code present
      3. Run `cargo test 2>&1` — assert all pass
    Expected Result: Platform branches exist, tests pass
    Failure Indicators: No platform detection, test failures
    Evidence: .sisyphus/evidence/task-19-cross-platform.txt
  ```

  **Commit**: YES (group with Wave 4)
  - Message: `fix(compiler): add cross-platform linker support for macOS and Windows (6d)`
  - Files: `src/compiler.rs`
  - Pre-commit: `cargo test`

---

### Wave 5 — P2 Runtime Safety

- [ ] 20. Thread-safe `invalid_digit_error` buffer (issue 2a)

  **What to do**:
  - RED: Verify the current static buffer is shared and unsafe in multithreaded contexts.
  - GREEN: In `runtime/opal_runtime.c` (lines 21-25):
    - Replace the static `char error_message[256]` with one of:
      - Option A (preferred): `_Thread_local static char error_message[256]` — each thread gets its own copy
      - Option B: Use `strdup` to return heap-allocated error messages (caller frees)
    - If using `_Thread_local`, verify C11 support via `__STDC_VERSION__` or use `__thread` (GCC/Clang extension) with fallback
  - REFACTOR: Add comment explaining thread safety choice

  **Must NOT do**:
  - Do NOT add mutex locking (over-engineering for this case)
  - Do NOT change the function signature if using _Thread_local approach

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Single line change (add _Thread_local) or simple refactor to strdup
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 5 (with Tasks 21, 22, 23)
  - **Blocks**: Task 27 (runtime split)
  - **Blocked By**: None

  **References**:

  **Pattern References**:
  - `runtime/opal_runtime.c:21-25` — `invalid_digit_error` with static buffer

  **Acceptance Criteria**:

  - [ ] Static buffer is thread-safe (via _Thread_local, __thread, or strdup)
  - [ ] `cargo build` succeeds
  - [ ] No behavioral change for single-threaded use

  **QA Scenarios (MANDATORY)**:

  ```
  Scenario: Thread-safe buffer
    Tool: Bash
    Preconditions: Task 20 changes applied
    Steps:
      1. Run `grep -n "_Thread_local\|__thread\|strdup" runtime/opal_runtime.c`
      2. Assert thread safety mechanism present in invalid_digit_error area
      3. Run `cargo build 2>&1` — assert success
    Expected Result: Thread safety applied, build succeeds
    Failure Indicators: No thread safety, build failure
    Evidence: .sisyphus/evidence/task-20-thread-safe.txt
  ```

  **Commit**: YES (group with Wave 5)
  - Message: `fix(runtime): make invalid_digit_error buffer thread-safe (2a)`
  - Files: `runtime/opal_runtime.c`
  - Pre-commit: `cargo build`

---

- [ ] 21. Add malloc NULL checks (issue 2c)

  **What to do**:
  - RED: Identify all malloc/calloc/strdup call sites and verify none check for NULL.
  - GREEN: In `runtime/opal_runtime.c`:
    - After every `malloc()`, `calloc()`, and `strdup()` call, add NULL check:
      ```c
      char* buf = malloc(size);
      if (!buf) {
          fprintf(stderr, "Runtime error: out of memory\n");
          exit(1);
      }
      ```
    - Apply to ALL 12 allocation sites identified in research
  - REFACTOR: Consider extracting a `safe_malloc(size_t size)` helper that wraps malloc + NULL check

  **Must NOT do**:
  - Do NOT change allocation sizes or strategies
  - Do NOT add custom allocators

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Mechanical addition of NULL checks after each allocation — simple pattern
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 5 (with Tasks 20, 22, 23)
  - **Blocks**: Task 27 (runtime split)
  - **Blocked By**: None

  **References**:

  **Pattern References**:
  - `runtime/opal_runtime.c` — All 12 sites: search for `malloc(`, `calloc(`, `strdup(`

  **Acceptance Criteria**:

  - [ ] Every malloc/calloc/strdup has NULL check
  - [ ] NULL triggers error message + exit(1)
  - [ ] `cargo build` succeeds

  **QA Scenarios (MANDATORY)**:

  ```
  Scenario: All allocations have NULL checks
    Tool: Bash
    Preconditions: Task 21 changes applied
    Steps:
      1. Count malloc/calloc/strdup calls: `grep -c "malloc\|calloc\|strdup" runtime/opal_runtime.c`
      2. Count NULL checks near allocations: `grep -c "if.*!.*buf\|if.*==.*NULL\|if.*!.*result\|if.*!.*str" runtime/opal_runtime.c`
      3. Assert NULL check count >= allocation count
      4. Run `cargo build 2>&1` — assert success
    Expected Result: Every allocation checked, build succeeds
    Failure Indicators: Unchecked allocations, build failure
    Evidence: .sisyphus/evidence/task-21-null-checks.txt
  ```

  **Commit**: YES (group with Wave 5)
  - Message: `fix(runtime): add NULL checks after all malloc/calloc/strdup calls (2c)`
  - Files: `runtime/opal_runtime.c`
  - Pre-commit: `cargo build`

---

- [ ] 22. Dynamic `take_input` buffer (issue 2e)

  **What to do**:
  - RED: Verify current static buffer in take_input (lines 41-51) is fixed-size and truncates long input.
  - GREEN: In `runtime/opal_runtime.c` (lines 41-51):
    - Replace static `char buffer[1024]` (or whatever size) with dynamic allocation:
      - Option A: Use POSIX `getline()` which auto-allocates and resizes
      - Option B: Manual growth: start with 256 bytes, realloc to 2× when needed
    - Handle EOF and error conditions
    - Free the buffer before returning (return a strdup'd copy, or document that caller must free)
  - REFACTOR: Ensure consistent with Task 17's ownership model

  **Must NOT do**:
  - Do NOT change the function signature (returns char*)
  - Do NOT add Windows-specific input handling (that's separate scope)

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Replace static buffer with getline — well-understood pattern
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 5 (with Tasks 20, 21, 23)
  - **Blocks**: Task 27 (runtime split)
  - **Blocked By**: None

  **References**:

  **Pattern References**:
  - `runtime/opal_runtime.c:41-51` — `take_input` with static buffer

  **Acceptance Criteria**:

  - [ ] No static/fixed-size buffer in take_input
  - [ ] Dynamic allocation handles arbitrary-length input
  - [ ] EOF and error handled
  - [ ] `cargo build` succeeds

  **QA Scenarios (MANDATORY)**:

  ```
  Scenario: Dynamic input buffer
    Tool: Bash
    Preconditions: Task 22 changes applied
    Steps:
      1. Run `grep -n "getline\|realloc\|dynamic" runtime/opal_runtime.c`
      2. Assert dynamic allocation pattern in take_input
      3. Run `cargo build 2>&1` — assert success
    Expected Result: Dynamic buffer used, build succeeds
    Failure Indicators: Static buffer still present, build failure
    Evidence: .sisyphus/evidence/task-22-dynamic-input.txt
  ```

  **Commit**: YES (group with Wave 5)
  - Message: `fix(runtime): replace static take_input buffer with dynamic allocation (2e)`
  - Files: `runtime/opal_runtime.c`
  - Pre-commit: `cargo build`

---

- [ ] 23. Quality RNG replacement (issue 2d)

  **What to do**:
  - RED: Write a test showing the current `rand()` / `srand(time(NULL))` has poor quality (e.g., known bias patterns, predictable seed).
  - GREEN: In `runtime/opal_runtime.c`:
    - Replace `srand(time(NULL))` seeding with a better source:
      - Linux: read from `/dev/urandom` for seed
      - Fallback: mix `time()`, `getpid()`, `clock()` for entropy
    - Replace `rand()` with xorshift128+ or similar embedded PRNG:
      - State stored in static (or _Thread_local) variables
      - Implement: `uint64_t xorshift128plus(uint64_t state[2])`
      - Provides full 64-bit output range with good statistical properties
    - Update all `random_*` functions (8 total: random_int8 through random_uint64) to use new PRNG
  - REFACTOR: Extract PRNG into clear initialization + generation functions

  **Must NOT do**:
  - Do NOT add external library dependencies (embed the PRNG)
  - Do NOT use cryptographic RNG (overkill for general-purpose random)

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
    - Reason: Needs careful PRNG implementation and seeding — algorithmic correctness matters
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 5 (with Tasks 20, 21, 22)
  - **Blocks**: Task 27 (runtime split)
  - **Blocked By**: None

  **References**:

  **Pattern References**:
  - `runtime/opal_runtime.c:97-103` — `seed_rand_once`: current srand(time(NULL))
  - `runtime/opal_runtime.c:105-151` — `random_int8` through `random_uint64`: all 8 random functions

  **External References**:
  - xorshift128+ algorithm: well-known, fast, good statistical properties, public domain

  **Acceptance Criteria**:

  - [ ] `srand`/`rand` no longer used
  - [ ] xorshift128+ (or equivalent quality PRNG) implemented
  - [ ] Better seed source (urandom or mixed entropy)
  - [ ] All 8 random_* functions use new PRNG
  - [ ] `cargo build` succeeds

  **QA Scenarios (MANDATORY)**:

  ```
  Scenario: Old RNG removed, new PRNG present
    Tool: Bash
    Preconditions: Task 23 changes applied
    Steps:
      1. Run `grep -c "srand\|rand()" runtime/opal_runtime.c`
      2. Assert count is 0 (old RNG fully removed)
      3. Run `grep -n "xorshift\|prng\|urandom" runtime/opal_runtime.c`
      4. Assert new PRNG implementation present
      5. Run `cargo build 2>&1` — assert success
    Expected Result: Old RNG gone, new PRNG present, build succeeds
    Failure Indicators: srand/rand still present, no new PRNG
    Evidence: .sisyphus/evidence/task-23-quality-rng.txt
  ```

  **Commit**: YES (group with Wave 5)
  - Message: `fix(runtime): replace rand()/srand() with xorshift128+ PRNG (2d)`
  - Files: `runtime/opal_runtime.c`
  - Pre-commit: `cargo build`

---

### Wave 6 — P2 Architecture (independent refactors)

- [ ] 24. Convert `#[path]` attributes to standard module structure (issue 6a)

  **What to do**:
  - RED: Verify current state: 123 `#[path]` attributes across 18 files. After conversion, `grep -rn '#\[path' src/` should return 0.
  - GREEN: For each of the 18 files with `#[path]` attributes:
    - Create the standard Rust directory structure (e.g., `src/codegen/` directory with `mod.rs`)
    - Move or rename files to match the expected module path
    - Remove all `#[path = "..."]` attributes
    - Replace with standard `mod name;` declarations
    - Files to convert (18 total): `src/lib.rs` (12), `src/hot_reload.rs` (11), `src/lsp.rs` (10), `src/stdlib.rs` (8), `src/runtime.rs` (8), `src/formatter.rs` (7), `src/testing.rs` (7), `src/package_manager.rs` (7), `src/build_system.rs` (6), `src/benchmarks.rs` (6), `src/doc_gen.rs` (5), `src/errors.rs` (4), `src/codegen.rs` (15), plus others
  - REFACTOR: Verify all `use` paths still resolve correctly

  **Must NOT do**:
  - Do NOT change any module's public API
  - Do NOT rename any types or functions
  - Do NOT split or merge modules (just fix the path declarations)

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
    - Reason: High volume of file moves (100+ path attrs) but each is mechanical. Needs careful execution.
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 6 (with Tasks 25, 26, 27, 28)
  - **Blocks**: None
  - **Blocked By**: None

  **References**:

  **Pattern References**:
  - `src/codegen.rs` — 15 `#[path]` attributes — largest single file to convert
  - `src/lib.rs` — 12 `#[path]` attributes
  - All 18 files listed in research findings

  **WHY Each Reference Matters**:
  - Each file needs its #[path] attributes removed and replaced with standard mod structure

  **Acceptance Criteria**:

  - [ ] Zero `#[path]` attributes remain in `src/` (grep returns 0 matches)
  - [ ] All modules use standard `mod name;` declarations
  - [ ] `cargo build` succeeds
  - [ ] `cargo test` passes
  - [ ] No public API changes

  **QA Scenarios (MANDATORY)**:

  ```
  Scenario: All #[path] attributes removed
    Tool: Bash
    Preconditions: Task 24 changes applied
    Steps:
      1. Run `grep -rn '#\[path' src/ | wc -l`
      2. Assert count is 0
      3. Run `cargo build 2>&1` — assert success
      4. Run `cargo test 2>&1` — assert all pass
    Expected Result: Zero #[path] attrs, build and tests pass
    Failure Indicators: Any #[path] remaining, build/test failure
    Evidence: .sisyphus/evidence/task-24-module-structure.txt
  ```

  **Commit**: YES (group with Wave 6)
  - Message: `refactor: convert all #[path] attributes to standard Rust module structure (6a)`
  - Files: All 18 module files + moved source files
  - Pre-commit: `cargo build && cargo test`

---

- [ ] 25. Scoped `NEXT_NODE_ID` — make per-parser or add reset (issue 6b)

  **What to do**:
  - RED: Write a test that creates two Parser instances and verifies their node IDs are independent (currently they share global state).
  - GREEN: In `src/parser.rs` (lines 47-54):
    - Move `NEXT_NODE_ID` from a `static AtomicU64` to a field on the `Parser` struct
    - Change `next_node_id()` from a free function to a method on `Parser`
    - OR: Add a `reset_node_ids()` function called at the start of each parse session
    - Update all call sites of `next_node_id()` to use the parser method
  - REFACTOR: Verify no other global state in parser

  **Must NOT do**:
  - Do NOT change the NodeId type or structure
  - Do NOT change how AST nodes use their IDs

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Move one static to a struct field + update call sites — straightforward refactor
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 6 (with Tasks 24, 26, 27, 28)
  - **Blocks**: None
  - **Blocked By**: None

  **References**:

  **Pattern References**:
  - `src/parser.rs:47-54` — `NEXT_NODE_ID` static and `next_node_id()` function
  - `src/parser.rs` — `Parser` struct definition with fields

  **Acceptance Criteria**:

  - [ ] `NEXT_NODE_ID` is no longer a module-level static
  - [ ] Node ID generation is scoped to parser instance
  - [ ] `cargo test` passes

  **QA Scenarios (MANDATORY)**:

  ```
  Scenario: No global NEXT_NODE_ID
    Tool: Bash
    Preconditions: Task 25 changes applied
    Steps:
      1. Run `grep -n "static.*NEXT_NODE_ID\|AtomicU64" src/parser.rs`
      2. Assert no global static remains for node IDs
      3. Run `cargo test 2>&1` — assert all pass
    Expected Result: No global node ID state, tests pass
    Failure Indicators: Global static still present, test failures
    Evidence: .sisyphus/evidence/task-25-scoped-node-id.txt
  ```

  **Commit**: YES (group with Wave 6)
  - Message: `refactor(parser): make NEXT_NODE_ID per-parser instead of global static (6b)`
  - Files: `src/parser.rs`
  - Pre-commit: `cargo test`

---

- [ ] 26. Refactor TypeChecker — extract ad-hoc stacks into context struct (issue 6c)

  **What to do**:
  - RED: Write a test that verifies TypeChecker still works correctly after refactoring (existing tests should suffice).
  - GREEN: In `src/type_system/checker.rs`:
    - Extract fields 5-9 and 15 (the ad-hoc stack fields) into a new `TypeCheckContext` struct:
      - `current_function_return_type`
      - `current_function_name`
      - `in_loop`
      - `current_match_type`
      - `current_adt_variant_types`
      - `generic_type_map`
    - Replace direct field access with `self.context.field_name`
    - Keep remaining fields (symbol tables, error lists, etc.) on TypeChecker
  - REFACTOR: Verify the separation makes sense and doesn't create awkward cross-references

  **Must NOT do**:
  - Do NOT change type checking logic
  - Do NOT rename types or change public API
  - Do NOT merge or split type checking phases

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
    - Reason: Touches many call sites across the type checker — needs careful find-and-replace
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 6 (with Tasks 24, 25, 27, 28)
  - **Blocks**: None
  - **Blocked By**: None

  **References**:

  **Pattern References**:
  - `src/type_system/checker.rs:63-101` — TypeChecker struct with 20 fields
  - Fields 5-9: `current_function_return_type`, `current_function_name`, `in_loop`, `current_match_type`, `current_adt_variant_types`
  - Field 15: `generic_type_map`

  **WHY Each Reference Matters**:
  - The 20 fields are what needs restructuring — the ad-hoc stacks are the extraction targets

  **Acceptance Criteria**:

  - [ ] `TypeCheckContext` struct created with 6 extracted fields
  - [ ] TypeChecker uses `self.context.field` for extracted fields
  - [ ] All existing type checker tests pass
  - [ ] `cargo test` passes

  **QA Scenarios (MANDATORY)**:

  ```
  Scenario: TypeCheckContext struct exists
    Tool: Bash
    Preconditions: Task 26 changes applied
    Steps:
      1. Run `grep -n "TypeCheckContext\|struct.*Context" src/type_system/checker.rs`
      2. Assert new struct found
      3. Run `cargo test 2>&1` — assert all pass
    Expected Result: Context struct exists, tests pass
    Failure Indicators: No context struct, test failures
    Evidence: .sisyphus/evidence/task-26-typechecker-refactor.txt
  ```

  **Commit**: YES (group with Wave 6)
  - Message: `refactor(typechecker): extract ad-hoc stacks into TypeCheckContext struct (6c)`
  - Files: `src/type_system/checker.rs`, type checker submodules
  - Pre-commit: `cargo test`

---

- [ ] 27. Split monolithic `runtime.c` into focused modules (issue 6f)

  **What to do**:
  - RED: Verify current file is 617 lines with 45 functions all in one file.
  - GREEN: Split `runtime/opal_runtime.c` into focused files:
    - `runtime/opal_io.c` — `print_string`, `take_input` (2 functions)
    - `runtime/opal_print.c` — `print_int8` through `print_float64` (10 functions)
    - `runtime/opal_parse.c` — `string_to_int8` through `string_to_float64` (10 functions)
    - `runtime/opal_string.c` — `int8_to_string` through `bool_to_string` (11 functions)
    - `runtime/opal_rng.c` — `seed_rand_once`, `random_int8` through `random_uint64` (9 functions)
    - `runtime/opal_error.c` — `invalid_digit_error`, `opal_runtime_error` (2 functions)
    - `runtime/opal_runtime.h` — shared header with all function declarations
    - Keep `runtime/opal_runtime.c` as a thin file that `#include`s all others (or update the build to compile all separately)
  - Update `src/compiler.rs` to compile the split files (or use the include approach)
  - REFACTOR: Ensure the build still produces a single linkable runtime

  **Must NOT do**:
  - Do NOT change any function signatures or behavior
  - Do NOT split mid-function
  - Do NOT change the linking model (still compile to one object)

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
    - Reason: File splitting + build system update — moderate complexity
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 6 (with Tasks 24, 25, 26, 28)
  - **Blocks**: None
  - **Blocked By**: Tasks 17, 20, 21, 22, 23 (runtime changes must be done first)

  **References**:

  **Pattern References**:
  - `runtime/opal_runtime.c` — All 617 lines, 45 functions to split
  - `src/compiler.rs` — Build/link configuration that may need updating

  **Acceptance Criteria**:

  - [ ] `runtime/opal_runtime.c` split into 6+ focused files
  - [ ] Shared header `runtime/opal_runtime.h` created
  - [ ] All 45 functions accounted for in split files
  - [ ] `cargo build` succeeds
  - [ ] `cargo test` passes
  - [ ] Test projects still compile and run

  **QA Scenarios (MANDATORY)**:

  ```
  Scenario: Runtime split correctly
    Tool: Bash
    Preconditions: Task 27 changes applied
    Steps:
      1. Run `ls runtime/opal_*.c | wc -l`
      2. Assert count >= 6 (split into multiple files)
      3. Run `ls runtime/opal_runtime.h`
      4. Assert header exists
      5. Run `cargo build 2>&1` — assert success
      6. Run `cargo test 2>&1` — assert all pass
    Expected Result: Multiple C files, header exists, build and tests pass
    Failure Indicators: Still one file, no header, build/test failure
    Evidence: .sisyphus/evidence/task-27-runtime-split.txt
  ```

  **Commit**: YES (group with Wave 6)
  - Message: `refactor(runtime): split monolithic opal_runtime.c into focused modules (6f)`
  - Files: `runtime/opal_*.c`, `runtime/opal_runtime.h`, `src/compiler.rs`
  - Pre-commit: `cargo build && cargo test`

---

- [ ] 28. Upgrade inkwell 0.9.0 + LLVM 18 (issue 6e)

  **What to do**:
  - RED: Build currently works with inkwell 0.8.0 + LLVM 14. After upgrade, build must still work.
  - GREEN:
    - Update `Cargo.toml`:
      - `inkwell = { version = "0.9.0", features = ["llvm18-1"] }` (remove llvm14-0 features)
      - Update `thiserror` from 1.0 to 2.0 (required by inkwell 0.9.0)
    - Migrate ALL codegen to opaque pointers:
      - Replace all `some_type.ptr_type(AddressSpace::default())` → `context.ptr_type(AddressSpace::default())`
      - Replace all `build_load(ptr, name)` → `build_load(pointee_type, ptr, name)`
      - Replace all `build_gep(ptr, indices, name)` → `build_gep(element_type, ptr, indices, name)`
      - Replace all `build_call(fn_value, args, name)` → verify signature (may need CallSiteValue changes)
      - Remove all `get_element_type()` calls on pointer types
    - Migrate PassManager:
      - Replace `PassManager` usage with `run_passes` + `PassBuilderOptions`
    - Update environment variable: `LLVM_SYS_140_PREFIX` → `LLVM_SYS_181_PREFIX`
    - Update README.md installation instructions
  - REFACTOR: Verify all codegen tests pass with new LLVM

  **Must NOT do**:
  - Do NOT change codegen logic/behavior — only adapt to new API
  - Do NOT downgrade optimization levels
  - Do NOT remove any existing functionality

  **Recommended Agent Profile**:
  - **Category**: `ultrabrain`
    - Reason: Massive API migration across entire codegen (~49 call sites across 16 files), requires deep understanding of LLVM opaque pointer model
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES (with other Wave 6 tasks)
  - **Parallel Group**: Wave 6 (with Tasks 24-27)
  - **Blocks**: Task 29 (test projects depend on working build)
  - **Blocked By**: ALL prior tasks (should be last major change to avoid conflicts)

  **References**:

  **Pattern References**:
  - `Cargo.toml:10` — Current inkwell dependency
  - `src/codegen/` — All 16 codegen files with LLVM API calls
  - `src/codegen/optimization.rs` — PassManager usage to migrate

  **External References**:
  - inkwell 0.9.0 changelog — API breaking changes
  - LLVM opaque pointer migration guide — conceptual background

  **WHY Each Reference Matters**:
  - Cargo.toml is the dependency to change
  - All codegen files need opaque pointer migration
  - PassManager migration is a separate concern within the same upgrade

  **Acceptance Criteria**:

  - [ ] `Cargo.toml` uses `inkwell = "0.9.0"` with `llvm18-1` feature
  - [ ] All `ptr_type()` calls use `context.ptr_type()` (opaque pointers)
  - [ ] All `build_load`/`build_gep` calls include explicit pointee type
  - [ ] PassManager migrated to `run_passes`
  - [ ] `cargo build` succeeds
  - [ ] `cargo test` passes
  - [ ] All test projects compile and run

  **QA Scenarios (MANDATORY)**:

  ```
  Scenario: LLVM 18 build succeeds
    Tool: Bash
    Preconditions: Task 28 changes applied, LLVM 18 installed
    Steps:
      1. Run `grep "inkwell" Cargo.toml` — assert version 0.9.0
      2. Run `grep "llvm18" Cargo.toml` — assert llvm18-1 feature
      3. Run `cargo build 2>&1` — assert success
      4. Run `cargo test 2>&1` — assert all pass
    Expected Result: Build with LLVM 18 succeeds, all tests pass
    Failure Indicators: Build failure, test failure
    Evidence: .sisyphus/evidence/task-28-llvm18-upgrade.txt

  Scenario: No old LLVM 14 references remain
    Tool: Bash
    Preconditions: Task 28 changes applied
    Steps:
      1. Run `grep -rn "llvm14\|LLVM_SYS_140" . --include="*.rs" --include="*.toml" --include="*.md"`
      2. Assert zero matches (all references updated)
    Expected Result: No LLVM 14 references
    Failure Indicators: Old LLVM 14 references found
    Evidence: .sisyphus/evidence/task-28-no-llvm14.txt
  ```

  **Commit**: YES (group with Wave 6)
  - Message: `refactor(codegen): upgrade inkwell 0.8→0.9 and LLVM 14→18 with opaque pointer migration (6e)`
  - Files: `Cargo.toml`, all `src/codegen/*.rs` files, `README.md`
  - Pre-commit: `cargo build && cargo test`

---

### Wave 7 — Integration (test projects + regression)

- [ ] 29. Create test projects for compiler-testable fixes

  **What to do**:
  - Create new test projects in `test-projects/` that exercise the key fixes:
    - `test-projects/overflow-trap/` — Tests issue 1a: does `2147483647 + 1` trigger a runtime trap (non-zero exit)?
    - `test-projects/lambda-basic/` — Tests issue 1d: defines and calls a lambda, verifies correct return value
    - `test-projects/array-bounds/` — Tests issue 1g: accesses array out of bounds, verifies runtime trap
    - `test-projects/immutability/` — Tests issue 4d: attempts to assign to immutable var, verifies compile-time or runtime error
    - `test-projects/string-interp-long/` — Tests issue 3c: interpolates a string longer than 256 chars, verifies no crash
    - `test-projects/cast-safety/` — Tests issues 1b, 1c, 4c: various cast operations with range checks
  - Each test project follows existing structure: `opal.toml` + `src/main.op`
  - Use language-spec syntax exactly (colon-block, no curly braces on guards)
  - Add integration tests in `tests/integration_e2e.rs` for each new test project
  - REFACTOR: Ensure test projects also serve as documentation/examples

  **Must NOT do**:
  - Do NOT use curly braces on guard/if/while statements (follow language-spec)
  - Do NOT create overly complex test programs — keep each focused on one issue

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
    - Reason: Multiple test projects + integration test wiring — moderate volume
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: NO — depends on all prior fixes being complete
  - **Parallel Group**: Sequential (after Wave 6)
  - **Blocks**: Task 30
  - **Blocked By**: Tasks 5-16, 28 (all codegen fixes + LLVM upgrade)

  **References**:

  **Pattern References**:
  - `test-projects/hello-world/` — Structure to follow: opal.toml + src/main.op
  - `test-projects/fib-recursive/src/main.op` — Example of Opalescent code style
  - `tests/integration_e2e.rs` — Where to add new integration tests
  - `language-spec/error_handling_samples.op` — Syntax reference for guard/error handling

  **WHY Each Reference Matters**:
  - Existing test projects show the exact structure and conventions to follow
  - `integration_e2e.rs` shows how to wire test projects into the Rust test harness
  - Language spec samples show correct syntax to use

  **Acceptance Criteria**:

  - [ ] 6 new test projects created in `test-projects/`
  - [ ] Each test project has `opal.toml` + `src/main.op`
  - [ ] Integration tests added for each test project
  - [ ] `cargo test --features integration` passes with new tests
  - [ ] All test projects use language-spec syntax (no curly braces on guards)

  **QA Scenarios (MANDATORY)**:

  ```
  Scenario: Test projects exist and compile
    Tool: Bash
    Preconditions: All prior tasks complete
    Steps:
      1. Run `ls test-projects/overflow-trap/src/main.op test-projects/lambda-basic/src/main.op test-projects/array-bounds/src/main.op`
      2. Assert all files exist
      3. Run `cargo test --features integration 2>&1`
      4. Assert all integration tests pass
    Expected Result: All test projects exist and pass
    Failure Indicators: Missing files, test failures
    Evidence: .sisyphus/evidence/task-29-test-projects.txt

  Scenario: No curly braces on guard/if/while in test projects
    Tool: Bash
    Preconditions: Task 29 complete
    Steps:
      1. Run `grep -rn '{' test-projects/*/src/main.op`
      2. Verify any `{` is for string interpolation only, not for control flow blocks
    Expected Result: No curly-brace block syntax in test projects
    Failure Indicators: Curly braces used for blocks
    Evidence: .sisyphus/evidence/task-29-no-braces.txt
  ```

  **Commit**: YES
  - Message: `test: add test projects for overflow, lambda, array bounds, immutability, string interp, cast safety`
  - Files: `test-projects/*/`, `tests/integration_e2e.rs`
  - Pre-commit: `cargo test --features integration`

---

- [ ] 30. Full regression test suite run

  **What to do**:
  - Run the complete test suite to verify no regressions:
    - `cargo test` — all unit tests
    - `cargo test --features integration` — all integration tests
    - `cargo clippy` — no new warnings
    - `cargo build --release` — release build works
  - Fix any failures discovered
  - Document test results

  **Must NOT do**:
  - Do NOT skip any failing tests — fix them

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Run commands, check output, fix any issues
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: NO — final verification
  - **Parallel Group**: Sequential (after Task 29)
  - **Blocks**: F1-F4 (final verification wave)
  - **Blocked By**: Task 29

  **References**:

  - All prior tasks — this verifies their combined effect

  **Acceptance Criteria**:

  - [ ] `cargo test` — 0 failures
  - [ ] `cargo test --features integration` — 0 failures
  - [ ] `cargo clippy` — no new warnings
  - [ ] `cargo build --release` — success

  **QA Scenarios (MANDATORY)**:

  ```
  Scenario: Full test suite passes
    Tool: Bash
    Preconditions: All 29 tasks complete
    Steps:
      1. Run `cargo test 2>&1` — capture output
      2. Run `cargo test --features integration 2>&1` — capture output
      3. Run `cargo clippy 2>&1` — capture output
      4. Run `cargo build --release 2>&1` — capture output
      5. Assert all commands exit with code 0
    Expected Result: All pass, no regressions
    Failure Indicators: Any command fails
    Evidence: .sisyphus/evidence/task-30-full-regression.txt
  ```

  **Commit**: YES (if any fixes needed)
  - Message: `fix: resolve regression test failures`
  - Pre-commit: `cargo test`

---

## Final Verification Wave (MANDATORY — after ALL implementation tasks)

> 4 review agents run in PARALLEL. ALL must APPROVE. Present consolidated results to user and get explicit "okay" before completing.

- [ ] F1. **Plan Compliance Audit** — `oracle`
  Read the plan end-to-end. For each "Must Have": verify implementation exists (read file, run command). For each "Must NOT Have": search codebase for forbidden patterns — reject with file:line if found. Check evidence files exist in .sisyphus/evidence/. Compare deliverables against plan.
  Output: `Must Have [N/N] | Must NOT Have [N/N] | Tasks [N/N] | VERDICT: APPROVE/REJECT`

- [ ] F2. **Code Quality Review** — `unspecified-high`
  Run `cargo build 2>&1`, `cargo test`, `cargo clippy`. Review all changed files for: `unsafe` without justification, `unwrap()` in non-test code, commented-out code, unused imports. Check AI slop: excessive comments, over-abstraction, generic names.
  Output: `Build [PASS/FAIL] | Tests [N pass/N fail] | Clippy [N warnings] | Files [N clean/N issues] | VERDICT`

- [ ] F3. **Real Manual QA** — `unspecified-high`
  Start from clean state. Execute EVERY QA scenario from EVERY task — follow exact steps, capture evidence. Test cross-task integration. Test edge cases. Save to `.sisyphus/evidence/final-qa/`.
  Output: `Scenarios [N/N pass] | Integration [N/N] | Edge Cases [N tested] | VERDICT`

- [ ] F4. **Scope Fidelity Check** — `deep`
  For each task: read "What to do", read actual diff (git log/diff). Verify 1:1. Check "Must NOT do" compliance. Detect cross-task contamination. Flag unaccounted changes.
  Output: `Tasks [N/N compliant] | Contamination [CLEAN/N issues] | Unaccounted [CLEAN/N files] | VERDICT`

---

## Commit Strategy

Each wave gets its own commit(s):
- Wave 1: `fix(codegen): extract shared helpers and add runtime trap function`
- Wave 2: `fix(codegen): resolve all P0 correctness issues (1a,1b,1d,1e,1f,1g,3c)`
- Wave 3: `fix(codegen): resolve P1 spec conformance (1c,4a,4b,4c,4d)`
- Wave 4: `fix(memory): add free() calls in runtime and codegen (3a,3b) + cross-platform linker (6d)`
- Wave 5: `fix(runtime): thread safety, NULL checks, dynamic buffers, quality RNG (2a,2c,2d,2e)`
- Wave 6: `refactor: module structure, scoped IDs, TypeChecker, split runtime, LLVM 18 upgrade (5a-c,6a-f)`
- Wave 7: `test: add test projects for compiler-testable fixes`

---

## Success Criteria

### Verification Commands
```bash
cargo build 2>&1                           # Expected: no errors
cargo test                                  # Expected: all tests pass
cargo test --features integration           # Expected: all integration tests pass
cargo clippy 2>&1                          # Expected: no new warnings
```

### Final Checklist
- [ ] All 28 issues from SCALABILITY_ISSUES.md addressed
- [ ] TDD workflow followed (failing test → passing test → refactor)
- [ ] Test projects created for compiler-testable items
- [ ] Language-spec syntax followed (no curly braces on guards)
- [ ] Runtime trap function used for all safety checks
- [ ] No regressions in existing tests
- [ ] LLVM 18 upgrade complete with opaque pointer migration
