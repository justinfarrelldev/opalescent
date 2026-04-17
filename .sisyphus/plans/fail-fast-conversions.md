# Fail-Fast String-to-Numeric Conversions

## TL;DR

> **Quick Summary**: Implement explicit error handling for all string-to-numeric conversions. Parse failures become compile-time enforced errors requiring `guard`/`propagate`, eliminating silent zero-on-failure behavior.
> 
> **Deliverables**:
> - C runtime functions returning `{value, error_ptr}` structs for 10 parse functions
> - 11 infallible `*_to_string` conversion functions
> - Updated LLVM codegen for struct returns with conditional branching
> - Type system enforcement of `errors ParseError` at call sites
> - Updated tests reflecting new error model
> 
> **Estimated Effort**: Large
> **Parallel Execution**: YES - 4 waves
> **Critical Path**: Task 1 (syntax fix) -> Task 2 (C runtime) -> Task 5 (codegen) -> Task 7 (type system) -> Final Verification

---

## Context

### Original Request
Implement fail-fast string-to-numeric and numeric-to-string conversion functions for all Opalescent numeric types, and refactor existing `string_to_int*`/`string_to_uint*` to use the same fail-fast error model.

### Interview Summary
**Key Discussions**:
- Current C runtime silently returns `0` on parse failure - this must change
- Parse failures must be **explicit errors** handled via `guard ... into ... else` or `propagate`
- Bare calls without error handling become **compile-time errors**
- Use TDD approach

**Research Findings**:
- 8 existing `string_to_*` functions in `runtime/opal_runtime.c` (lines 119-181)
- Functions use `ptr_to_int_fn!` macro in `src/codegen/functions.rs` (lines 535-542)
- Builtins registered in `src/type_system/checker/size_specific_builtins.rs` without `errors` clause
- `language-spec/simple_quiz.op` shows the intended `guard string_to_int32(s) into n else e =>` pattern
- 8 specific tests identified that will break with the new error model

### Pre-Requisite Fix
The `error_handling_samples.op` spec file has invalid guard syntax that must be fixed first (brace syntax `else { }` is not valid; only arrow syntax `else e =>` is valid).

---

## Work Objectives

### Core Objective
Transform string-to-numeric conversions from silent-failure to explicit-error model, enforcing error handling at compile time.

### Concrete Deliverables
- `runtime/opal_runtime.c`: 10 `string_to_*` functions returning `{value, error_ptr}` structs
- `runtime/opal_runtime.c`: 11 `*_to_string` infallible functions
- `src/codegen/functions.rs`: Updated LLVM declarations for struct returns
- `src/codegen/statements.rs`: Updated `known_runtime_return_type` mapping
- `src/type_system/checker/size_specific_builtins.rs`: Builtins with `errors ParseError`
- Updated test files reflecting new behavior

### Definition of Done
- [ ] `cargo test` passes with all 958+ tests green
- [ ] Bare `let n = string_to_int32(s)` without error handling produces compile error
- [ ] `guard string_to_int32(s) into n else e => ...` pattern works correctly
- [ ] All 11 numeric types have both parse and stringify functions

### Must Have
- Struct return type `{value, error_ptr}` for all parse functions
- Specific error messages (e.g., "invalid digit 'x' in input", "overflow: value exceeds int32 range")
- Compile-time enforcement of error handling
- TDD approach

### Must NOT Have (Guardrails)
- Silent zero-on-failure behavior
- Runtime exceptions or panics
- Changes to unrelated builtins
- Breaking the existing `guard`/`propagate` semantics

---

## Verification Strategy

> **ZERO HUMAN INTERVENTION** - ALL verification is agent-executed. No exceptions.

### Test Decision
- **Infrastructure exists**: YES
- **Automated tests**: TDD
- **Framework**: Rust's built-in `#[test]` with `cargo test`
- **If TDD**: Each task follows RED (failing test) -> GREEN (minimal impl) -> REFACTOR

### QA Policy
Every task MUST include agent-executed QA scenarios.
Evidence saved to `.sisyphus/evidence/task-{N}-{scenario-slug}.{ext}`.

- **Compiler**: Use Bash (cargo test, cargo build) - Run commands, assert exit codes and output
- **Runtime**: Use Bash - Compile test programs, execute, validate output

---

## Execution Strategy

### Parallel Execution Waves

```
Wave 1 (Start Immediately - prerequisite fix + foundation):
├── Task 1: Fix guard syntax in error_handling_samples.op [quick]
├── Task 2: C runtime - Refactor string_to_* to return structs [deep]
└── Task 3: C runtime - Add *_to_string functions [unspecified-high]

Wave 2 (After Wave 1 - codegen + type system, PARALLEL):
├── Task 4: Verify ParseError type registration is sufficient [quick]
├── Task 5: Codegen - Update LLVM declarations for struct returns [deep]
├── Task 6: Codegen - Emit guard-branch logic for struct returns [deep]
└── Task 7: Type system - Register builtins with errors ParseError [unspecified-high]

Wave 3 (After Wave 2 - test updates):
├── Task 8: Update breaking codegen tests [unspecified-high]
├── Task 9: Update breaking type system tests [unspecified-high]
└── Task 10: Add new integration tests for fail-fast behavior [unspecified-high]

Wave FINAL (After ALL tasks - 4 parallel reviews, then user okay):
├── Task F1: Plan compliance audit (oracle)
├── Task F2: Code quality review (unspecified-high)
├── Task F3: Real manual QA (unspecified-high)
└── Task F4: Scope fidelity check (deep)
-> Present results -> Get explicit user okay

Critical Path: Task 1 -> Task 2 -> Task 5 -> Task 7 -> Task 8/9 -> F1-F4 -> user okay
Parallel Speedup: ~60% faster than sequential
Max Concurrent: 4 (Waves 2 & 3)
```

### Dependency Matrix

| Task | Depends On | Blocks | Wave |
|------|------------|--------|------|
| 1    | -          | 2-10   | 1    |
| 2    | 1          | 5, 6   | 1    |
| 3    | 1          | 10     | 1    |
| 4    | 1          | 7      | 2    |
| 5    | 2          | 6, 8   | 2    |
| 6    | 5          | 8, 9   | 2    |
| 7    | 4          | 9      | 2    |
| 8    | 5, 6       | F1-F4  | 3    |
| 9    | 6, 7       | F1-F4  | 3    |
| 10   | 2, 3       | F1-F4  | 3    |

### Agent Dispatch Summary

- **Wave 1**: 3 tasks - T1 `quick`, T2 `deep`, T3 `unspecified-high`
- **Wave 2**: 4 tasks - T4 `quick`, T5 `deep`, T6 `deep`, T7 `unspecified-high`
- **Wave 3**: 3 tasks - T8-T10 `unspecified-high`
- **FINAL**: 4 tasks - F1 `oracle`, F2-F3 `unspecified-high`, F4 `deep`

---

## TODOs

- [x] 1. Fix guard syntax in error_handling_samples.op

  **What to do**:
  - Edit `language-spec/error_handling_samples.op` to fix two invalid guard statements
  - **Line 29-32**: Convert from expression-form guard (braces) to statement-form guard (arrow + indented body).
    Current (INVALID — braces not valid per language design):
    ```opal
    guard parse_number_core(text) into parsed else {
        log_parse_failure(text)
        return 0
    }
    ```
    Replace with (statement-form — arrow syntax with error binding + indented body):
    ```opal
    guard parse_number_core(text) into parsed else _e =>
        log_parse_failure(text)
        return 0
    ```
    Note: The indented body after `=>` replaces the brace block. The closing `}` on line 32 must be removed.
  - **Line 46**: Add missing error binding `_e` before `=>`.
    Current: `guard parse_number(content) into value else =>`
    Replace with: `guard parse_number(content) into value else _e =>`
  - Run `cargo test` to verify the 4 previously-failing tests now pass

  **Must NOT do**:
  - Change any other files
  - Modify guard semantics

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Trivial 2-line syntax fix with clear instructions
  - **Skills**: `[]`
    - No special skills needed

  **Parallelization**:
  - **Can Run In Parallel**: NO
  - **Parallel Group**: Wave 1 (prerequisite)
  - **Blocks**: Tasks 2-10 (all subsequent work)
  - **Blocked By**: None (can start immediately)

  **References**:
  - `language-spec/error_handling_samples.op:29-32` - First guard with invalid brace syntax
  - `language-spec/error_handling_samples.op:46` - Second guard missing error binding
  - `language-spec/simple_quiz.op:43` - Example of correct `guard ... else e =>` syntax

  **Important context — Guard syntax ambiguity:**
  > The parser has TWO guard forms: `parse_guard_expression()` (in `src/parser/expressions.rs:208-300`) supports `else { block }`, while `parse_guard_statement()` (in `src/parser/statements.rs:671+`) requires `else <err> => <body>`. The user has explicitly stated that brace syntax for guard else is NOT valid Opalescent — only arrow syntax `else e =>` is valid. If `parse_guard_expression` allows braces, that is a parser bug to be addressed separately, NOT in this task. This task only fixes the spec file to use valid syntax.

  **Acceptance Criteria**:
  - [ ] `cargo test parser::tests::test_error_handling_sample_parses_successfully` passes
  - [ ] `cargo test parser::tests::test_error_handling_sample_contains_guard_and_propagate` passes
  - [ ] `cargo test type_system::test_integration_ecosystem::ecosystem_tests::test_error_handling_samples_spec_file_parses` passes
  - [ ] `cargo test type_system::tests::test_type_check_error_handling_sample_program` passes

  **QA Scenarios**:
  ```
  Scenario: Guard syntax fix allows parsing
    Tool: Bash
    Preconditions: File edited with correct syntax
    Steps:
      1. Run: cargo test parser::tests::test_error_handling_sample_parses_successfully
      2. Assert exit code 0
      3. Assert output contains "1 passed"
    Expected Result: Test passes
    Evidence: .sisyphus/evidence/task-1-parse-test.txt

  Scenario: All 4 related tests pass
    Tool: Bash
    Preconditions: Syntax fix applied
    Steps:
      1. Run: cargo test error_handling_sample 2>&1
      2. Assert output contains "4 passed" or "test result: ok"
    Expected Result: All 4 tests pass
    Evidence: .sisyphus/evidence/task-1-all-tests.txt
  ```

  **Commit**: YES
  - Message: `fix(spec): correct guard syntax in error_handling_samples.op`
  - Files: `language-spec/error_handling_samples.op`
  - Pre-commit: `cargo test`

---

- [x] 2. C Runtime - Refactor string_to_* to return structs

  **What to do**:
  - Define result struct types for each numeric size in `runtime/opal_runtime.c`:
    ```c
    typedef struct { int8_t value; const char* error; } ParseResultI8;
    typedef struct { int16_t value; const char* error; } ParseResultI16;
    typedef struct { int32_t value; const char* error; } ParseResultI32;
    typedef struct { int64_t value; const char* error; } ParseResultI64;
    typedef struct { uint8_t value; const char* error; } ParseResultU8;
    typedef struct { uint16_t value; const char* error; } ParseResultU16;
    typedef struct { uint32_t value; const char* error; } ParseResultU32;
    typedef struct { uint64_t value; const char* error; } ParseResultU64;
    typedef struct { float value; const char* error; } ParseResultF32;
    typedef struct { double value; const char* error; } ParseResultF64;
    ```
  - Refactor 8 existing `string_to_*` functions to return these structs
  - Add 2 new functions: `string_to_float32`, `string_to_float64`
  - On success: `{ parsed_value, NULL }`
  - On failure: `{ 0, "specific error message" }`
  - Error messages must be specific:
    - `"null input"` for NULL string
    - `"empty input"` for empty string
    - `"invalid digit 'X' in input"` for bad characters
    - `"overflow: value exceeds intN range"` for out-of-range

  **Must NOT do**:
  - Remove the old function signatures (they'll be updated, not removed)
  - Change print functions or other unrelated runtime code

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: Core infrastructure change requiring careful handling of edge cases and error messages
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES (with Task 3)
  - **Parallel Group**: Wave 1
  - **Blocks**: Tasks 5, 6 (codegen depends on runtime)
  - **Blocked By**: Task 1 (syntax fix must pass tests first)

  **References**:
  - `runtime/opal_runtime.c:119-181` - Current `string_to_*` implementations
  - `runtime/opal_runtime.c:1-50` - Header area for struct definitions
  - Standard C: `strtoll`, `strtoull`, `strtof`, `strtod` for parsing

  **Acceptance Criteria**:
  - [ ] 10 struct types defined (8 int + 2 float)
  - [ ] 10 functions return structs instead of raw values
  - [ ] NULL input returns `{ 0, "null input" }`
  - [ ] Empty string returns `{ 0, "empty input" }`
  - [ ] Invalid character returns specific error with the bad char
  - [ ] Overflow returns specific error mentioning the type
  - [ ] Runtime compiles without warnings: `gcc -Wall -c runtime/opal_runtime.c`

  **QA Scenarios**:
  ```
  Scenario: Runtime compiles cleanly
    Tool: Bash
    Preconditions: Struct types and functions defined
    Steps:
      1. Run: gcc -Wall -Wextra -c runtime/opal_runtime.c -o /tmp/runtime_test.o
      2. Assert exit code 0
      3. Assert no warning output
    Expected Result: Clean compilation
    Evidence: .sisyphus/evidence/task-2-compile.txt

  Scenario: Struct types are correctly defined
    Tool: Bash
    Preconditions: Runtime modified
    Steps:
      1. Run: grep -c "typedef struct.*ParseResult" runtime/opal_runtime.c
      2. Assert output is "10"
    Expected Result: All 10 struct types defined
    Evidence: .sisyphus/evidence/task-2-struct-count.txt
  ```

  **Commit**: YES
  - Message: `feat(runtime): add fail-fast parse functions with struct returns`
  - Files: `runtime/opal_runtime.c`
  - Pre-commit: `gcc -Wall -c runtime/opal_runtime.c`

---

- [x] 3. C Runtime - Add *_to_string functions

  **What to do**:
  - Add 11 infallible stringify functions to `runtime/opal_runtime.c`:
    ```c
    char* int8_to_string(int8_t value);
    char* int16_to_string(int16_t value);
    char* int32_to_string(int32_t value);
    char* int64_to_string(int64_t value);
    char* uint8_to_string(uint8_t value);
    char* uint16_to_string(uint16_t value);
    char* uint32_to_string(uint32_t value);
    char* uint64_to_string(uint64_t value);
    char* float32_to_string(float value);
    char* float64_to_string(double value);
    char* bool_to_string(int8_t value);  // "true" or "false"
    ```
  - Use `snprintf` to format values into heap-allocated strings
  - Return heap-allocated strings (caller responsible for freeing)
  - Float formatting: use `%g` format for clean output

  **Must NOT do**:
  - Add error handling (these are infallible)
  - Change existing parse functions (that's Task 2)

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
    - Reason: Straightforward but needs 11 functions with consistent patterns
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES (with Task 2)
  - **Parallel Group**: Wave 1
  - **Blocks**: Task 10 (integration tests)
  - **Blocked By**: Task 1

  **References**:
  - `runtime/opal_runtime.c` - Add after parse functions section
  - Standard C: `snprintf`, `malloc` for string allocation

  **Acceptance Criteria**:
  - [ ] 11 `*_to_string` functions defined
  - [ ] Each returns heap-allocated string
  - [ ] `bool_to_string(1)` returns `"true"`, `bool_to_string(0)` returns `"false"`
  - [ ] Runtime compiles without warnings

  **QA Scenarios**:
  ```
  Scenario: All stringify functions defined
    Tool: Bash
    Preconditions: Functions added
    Steps:
      1. Run: grep -c "_to_string" runtime/opal_runtime.c
      2. Assert output >= 11
    Expected Result: At least 11 stringify functions
    Evidence: .sisyphus/evidence/task-3-stringify-count.txt

  Scenario: Runtime still compiles
    Tool: Bash
    Steps:
      1. Run: gcc -Wall -c runtime/opal_runtime.c -o /tmp/runtime_test.o
      2. Assert exit code 0
    Expected Result: Clean compilation
    Evidence: .sisyphus/evidence/task-3-compile.txt
  ```

  **Commit**: YES
  - Message: `feat(runtime): add numeric-to-string conversion functions`
  - Files: `runtime/opal_runtime.c`
  - Pre-commit: `gcc -Wall -c runtime/opal_runtime.c`

---

- [x] 4. Verify ParseError type registration is sufficient

  **What to do**:
  - `ParseError` is ALREADY registered as a built-in type in `src/type_system/environment.rs:53-59`:
    ```rust
    self.types.insert(
        "ParseError".to_owned(),
        CoreType::Generic {
            name: "ParseError".to_owned(),
            type_args: Vec::new(),
        },
    );
    ```
  - Verify this existing registration is sufficient for the error model (it should be — the type system already uses `ParseError` in 50+ test fixtures for guard/propagate tests)
  - If the `ParseError` type needs ADT variants (e.g., `InvalidInput`, `Overflow`), check whether the type system requires them or if the current generic placeholder is enough for compile-time enforcement
  - **No new files to create** — the stdlib is Rust-only (`src/stdlib/*.rs`), there are NO `.op` stdlib files

  **Must NOT do**:
  - Create new `.op` files (there are no `.op` stdlib files — stdlib is Rust-only)
  - Re-register ParseError if already present
  - Change existing test fixtures that reference ParseError

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Verification task, likely no changes needed
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES (with Tasks 5, 6)
  - **Parallel Group**: Wave 2
  - **Blocks**: Task 7 (confirmation that ParseError is usable)
  - **Blocked By**: Task 1

  **References**:
  - `src/type_system/environment.rs:53-59` - **Existing** ParseError registration as built-in type
  - `src/type_system/tests.rs:359` - Test fixtures using `make_unit_type_decl("ParseError", ...)` confirming how ParseError is used in tests (50+ references)

  **Acceptance Criteria**:
  - [ ] `ParseError` type confirmed registered in type environment at `environment.rs:53-59`
  - [ ] Confirm existing registration works with `error_types: vec![CoreType::Generic { name: "ParseError", .. }]` pattern used in builtins
  - [ ] No new files needed — document finding

  **QA Scenarios**:
  ```
  Scenario: ParseError is already registered
    Tool: Bash
    Steps:
      1. Run: grep -n "ParseError" src/type_system/environment.rs
      2. Assert output shows registration at line ~54
    Expected Result: ParseError already registered as built-in type
    Evidence: .sisyphus/evidence/task-4-parse-error.txt
  ```

  **Commit**: NO (groups with Task 7)

---

- [x] 5. Codegen - Update LLVM declarations for struct returns

  **What to do**:
  - Update `src/codegen/functions.rs` to declare struct return types for `string_to_*` functions
  - Replace `ptr_to_int_fn!` macro usage with struct-returning declarations
  - Define LLVM struct types matching C structs:
    ```
    { i8, ptr }   for ParseResultI8
    { i16, ptr }  for ParseResultI16
    { i32, ptr }  for ParseResultI32
    { i64, ptr }  for ParseResultI64
    { i8, ptr }   for ParseResultU8 (unsigned same size)
    { i16, ptr }  for ParseResultU16
    { i32, ptr }  for ParseResultU32
    { i64, ptr }  for ParseResultU64
    { float, ptr } for ParseResultF32
    { double, ptr } for ParseResultF64
    ```
  - Update `resolve_callee_function` to handle the new struct return types

  **Must NOT do**:
  - Change `print_*` or other unrelated function declarations
  - Break existing non-parse runtime function calls

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: Complex LLVM IR manipulation requiring understanding of type system
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES (with Tasks 4, 7)
  - **Parallel Group**: Wave 2
  - **Blocks**: Task 6 (guard-branch logic needs declarations), Task 8 (test updates)
  - **Blocked By**: Task 2 (C runtime must define structs first)

  **References**:
  - `src/codegen/functions.rs:535-542` - Current `ptr_to_int_fn!` macro usage
  - `src/codegen/functions.rs:1-100` - LLVM type creation patterns
  - `src/codegen/statements.rs:330-342` - `known_runtime_return_type` mapping
  - Inkwell docs: struct type creation, function signatures

  **Acceptance Criteria**:
  - [ ] 10 struct types defined in LLVM IR
  - [ ] Function declarations use struct return types
  - [ ] `known_runtime_return_type` updated for new return types
  - [ ] `cargo build` succeeds

  **QA Scenarios**:
  ```
  Scenario: Codegen compiles
    Tool: Bash
    Steps:
      1. Run: cargo build 2>&1
      2. Assert exit code 0
    Expected Result: Build succeeds
    Evidence: .sisyphus/evidence/task-5-build.txt

  Scenario: Struct return types declared
    Tool: Bash
    Steps:
      1. Run: grep -c "struct_type" src/codegen/functions.rs
      2. Assert output > 0
    Expected Result: Struct types found in codegen
    Evidence: .sisyphus/evidence/task-5-struct-types.txt
  ```

  **Commit**: NO (groups with Task 6)

---

- [x] 6. Codegen - Emit guard-branch logic for struct returns

  **What to do**:
  - Update `src/codegen/statements.rs` to handle guard statements with struct-returning calls
  - After calling a struct-returning function:
    1. Extract the error pointer: `extractvalue %result, 1`
    2. Compare to null: `icmp eq ptr %error, null`
    3. Branch: success path extracts value, else path uses error
  - Generate LLVM IR pattern:
    ```llvm
    %result = call { i32, ptr } @string_to_int32(ptr %input)
    %error = extractvalue { i32, ptr } %result, 1
    %is_success = icmp eq ptr %error, null
    br i1 %is_success, label %guard.success, label %guard.else
    
    guard.success:
      %value = extractvalue { i32, ptr } %result, 0
      ; bind to success variable
      br label %guard.merge
    
    guard.else:
      ; bind error to else variable
      ; execute else body
      br label %guard.merge
    ```

  **Must NOT do**:
  - Change guard handling for non-struct-returning functions
  - Break existing guard/propagate codegen

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: Complex control flow generation requiring LLVM IR expertise
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: NO (depends on Task 5)
  - **Parallel Group**: Wave 2 (after Task 5)
  - **Blocks**: Tasks 8, 9 (test updates)
  - **Blocked By**: Task 5 (declarations must exist first)

  **References**:
  - `src/codegen/statements.rs` - Guard statement codegen
  - `src/codegen/tests.rs:1025` - Existing guard codegen test pattern
  - LLVM IR: `extractvalue`, `icmp`, `br` instructions

  **Acceptance Criteria**:
  - [ ] Guard statements with struct-returning calls emit correct branching
  - [ ] Success path extracts value at index 0
  - [ ] Else path receives error string from index 1
  - [ ] `cargo build` succeeds
  - [ ] Basic guard codegen test compiles valid IR

  **QA Scenarios**:
  ```
  Scenario: Guard codegen produces valid IR
    Tool: Bash
    Steps:
      1. Run: cargo test test_guard_statement_compiles_to_valid_llvm_ir
      2. Assert exit code 0
    Expected Result: Guard IR test passes
    Evidence: .sisyphus/evidence/task-6-guard-ir.txt
  ```

  **Commit**: YES
  - Message: `feat(codegen): handle struct returns from parse functions`
  - Files: `src/codegen/functions.rs`, `src/codegen/statements.rs`
  - Pre-commit: `cargo test --lib`

---

- [x] 7. Type system - Register builtins with errors ParseError

  **What to do**:
  - Update `src/type_system/checker/size_specific_builtins.rs` to register parse functions with `errors ParseError`
  - Current pattern (without errors):
    ```rust
    register_builtin!("string_to_int32", [String] -> Int32);
    ```
  - New pattern (with errors):
    ```rust
    register_builtin!("string_to_int32", [String] -> Int32 errors ParseError);
    ```
  - Add registration for new `*_to_string` functions (infallible, no errors clause)
  - Ensure `ParseError` is importable/available in the type system

  **Must NOT do**:
  - Add errors to infallible functions (`*_to_string`, `print_*`)
  - Change unrelated builtin registrations

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
    - Reason: Type system modification with multiple registrations
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES (with Tasks 5, 6)
  - **Parallel Group**: Wave 2
  - **Blocks**: Task 9 (type system test updates)
  - **Blocked By**: Task 4 (ParseError must be defined)

  **References**:

  **CRITICAL — `string_to_int32` is registered in THREE places (all must be updated):**
  - `src/type_system/checker.rs:218-237` - **Direct registration** of `string_to_int32` with `error_types: Vec::new()` — MUST add `ParseError` here
  - `src/type_system/checker/size_specific_builtins.rs:15-20` - Registers int8/16/uint8/16/32/64 parse builtins (but NOT int32 or int64) — add `errors ParseError` to all
  - `src/type_system/module_resolver.rs:315-334` - `standard` module exports `string_to_int32` (line 316, note: incorrectly returns Int64) and `string_to_int64` (line 326), both with `error_types: Vec::new()` — MUST add `ParseError` here too

  **Context references:**
  - `src/type_system/checker.rs:67-105` - How `guard_else_depth` tracks error context
  - `src/type_system/tests.rs:1205` - How guard with errors is tested
  - `src/type_system/environment.rs:53-59` - ParseError type already registered here (confirmed in Task 4)

  **Acceptance Criteria**:
  - [ ] All 10 `string_to_*` functions registered with `errors ParseError`
  - [ ] All 11 `*_to_string` functions registered (no errors)
  - [ ] Bare call `let n = string_to_int32(s)` produces type error
  - [ ] `guard string_to_int32(s) into n else e => ...` type-checks successfully

  **QA Scenarios**:
  ```
  Scenario: Bare call produces type error
    Tool: Bash
    Steps:
      1. Run: cargo test type_system::tests::test_builtin_string_to_int32_signature_type_checks 2>&1
      2. Assert exit code 0 (the test itself verifies bare calls produce type errors)
      3. Additionally, write a Rust integration test that creates a program with
         `let n = string_to_int32("5")` (no guard/propagate), runs type_check_program,
         and asserts the returned errors contain a reference to ParseError
    Expected Result: Type system rejects bare calls to error-producing functions
    Failure Indicators: Test fails or no error returned for bare call
    Evidence: .sisyphus/evidence/task-7-bare-call-error.txt

  Scenario: Guard pattern type-checks
    Tool: Bash
    Steps:
      1. Run: cargo test type_system::tests::test_guard_statement_binds_success_and_error_types 2>&1
      2. Assert exit code 0
      3. Additionally, write a Rust integration test with a complete program containing
         `entry main = f(args: string[]): void =>` that uses
         `guard string_to_int32(s) into n else e =>` and verify type_check_program succeeds
    Expected Result: Guard pattern with error binding is accepted by type checker
    Evidence: .sisyphus/evidence/task-7-guard-typechecks.txt
  ```

  **Commit**: YES
  - Message: `feat(types): register parse builtins with errors ParseError`
  - Files: `src/type_system/checker/size_specific_builtins.rs`, related type files
  - Pre-commit: `cargo test --lib`

---

- [x] 8. Update breaking codegen tests

  **What to do**:
  - Update tests in `src/codegen/tests.rs` that expect old function signatures
  - Tests to update (identified from original spec):
    - `test_import_string_to_int64_emits_correct_declaration` - Update expected LLVM declaration
    - `test_import_standard_multiple_symbols_emit_all_runtime_declarations` - Update expected declarations
    - `test_guard_statement_compiles_to_valid_llvm_ir` - May need update for struct return handling
    - `test_builtin_calls_emit_runtime_declarations_without_imports` - Update expected signatures
  - Update expected LLVM IR patterns to match struct return types
  - Ensure tests verify the new `extractvalue` pattern for guards

  **Must NOT do**:
  - Delete tests (update them instead)
  - Skip verification of struct return handling

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
    - Reason: Multiple test updates requiring understanding of expected IR patterns
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES (with Tasks 9, 10)
  - **Parallel Group**: Wave 3
  - **Blocks**: Final verification
  - **Blocked By**: Tasks 5, 6 (codegen must be done)

  **References**:
  - `src/codegen/tests.rs` - All codegen tests
  - `src/codegen/tests.rs:1025` - Guard test with `string_to_int32`
  - Search for `string_to_int` in test files to find all affected tests

  **Acceptance Criteria**:
  - [ ] All identified tests updated to expect struct returns
  - [ ] Tests verify `extractvalue` pattern where applicable
  - [ ] `cargo test codegen::tests` passes

  **QA Scenarios**:
  ```
  Scenario: Codegen tests pass
    Tool: Bash
    Steps:
      1. Run: cargo test codegen::tests 2>&1
      2. Assert exit code 0
      3. Assert output shows all tests passing
    Expected Result: All codegen tests pass
    Evidence: .sisyphus/evidence/task-8-codegen-tests.txt
  ```

  **Commit**: NO (groups with Task 10)

---

- [x] 9. Update breaking type system tests

  **What to do**:
  - Update tests in `src/type_system/tests.rs` and `src/type_system/test_integration.rs`
  - Tests to update (identified from original spec):
    - `test_builtin_string_to_int32_signature_type_checks` - Update expected signature
    - `test_builtin_string_to_int64_is_not_registered` - May need update if we register it
    - `test_guard_propagate_and_multiple_returns_integrate` - Update for error handling
    - `test_guard_statement_binds_success_and_error_types` - Verify error type binding
  - Ensure tests verify that bare calls produce type errors
  - Ensure tests verify that guard pattern accepts ParseError

  **Must NOT do**:
  - Delete tests
  - Weaken type checking assertions

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
    - Reason: Multiple test updates requiring type system knowledge
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES (with Tasks 8, 10)
  - **Parallel Group**: Wave 3
  - **Blocks**: Final verification
  - **Blocked By**: Tasks 6, 7 (codegen and type system must be done)

  **References**:
  - `src/type_system/tests.rs` - Type system unit tests
  - `src/type_system/test_integration.rs:506-522` - Integration test with `string_to_int32`
  - Search for `string_to_int` in type system tests

  **Acceptance Criteria**:
  - [ ] All identified tests updated for new error model
  - [ ] Tests verify compile error on bare `string_to_*` calls
  - [ ] Tests verify guard pattern works with ParseError
  - [ ] `cargo test type_system::tests` passes

  **QA Scenarios**:
  ```
  Scenario: Type system tests pass
    Tool: Bash
    Steps:
      1. Run: cargo test type_system::tests 2>&1
      2. Assert exit code 0
    Expected Result: All type system tests pass
    Evidence: .sisyphus/evidence/task-9-type-tests.txt
  ```

  **Commit**: NO (groups with Task 10)

---

- [x] 10. Add new integration tests for fail-fast behavior

  **What to do**:
  - Add new tests that specifically verify the fail-fast error model:
    - Test: bare call produces compile error
    - Test: guard with ParseError type-checks
    - Test: propagate with ParseError works in error-declaring function
    - Test: error message specificity (null, empty, invalid char, overflow)
  - Add roundtrip tests for `*_to_string` functions
  - Update or verify `simple_quiz` test project still works

  **Must NOT do**:
  - Duplicate existing tests
  - Skip edge case testing

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
    - Reason: Comprehensive test coverage requiring multiple test patterns
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES (with Tasks 8, 9)
  - **Parallel Group**: Wave 3
  - **Blocks**: Final verification
  - **Blocked By**: Tasks 2, 3 (runtime functions must exist)

  **References**:
  - `tests/integration_e2e.rs` - E2E test patterns
  - `test-projects/simple-quiz/src/main.op` - Uses `guard string_to_int32`
  - `src/type_system/tests.rs` - Test patterns for type errors

  **Acceptance Criteria**:
  - [ ] New test: bare call compile error
  - [ ] New test: guard with parse function works
  - [ ] New test: propagate with parse function works
  - [ ] New test: error messages are specific
  - [ ] `simple_quiz` test project compiles and runs
  - [ ] `cargo test` all 958+ tests pass

  **QA Scenarios**:
  ```
  Scenario: All tests pass
    Tool: Bash
    Steps:
      1. Run: cargo test 2>&1
      2. Assert exit code 0
      3. Assert output shows 958+ tests passing
    Expected Result: Full test suite passes
    Evidence: .sisyphus/evidence/task-10-all-tests.txt

  Scenario: Simple quiz compiles without errors
    Tool: Bash
    Steps:
      1. Run: cargo run -- check test-projects/simple-quiz/src/main.op 2>&1
      2. Assert exit code 0
      3. Assert output contains "check passed"
    Expected Result: Quiz program type-checks successfully with new error model
    Evidence: .sisyphus/evidence/task-10-simple-quiz.txt
  ```

  **Commit**: YES
  - Message: `test: update tests for fail-fast conversion model`
  - Files: `src/codegen/tests.rs`, `src/type_system/tests.rs`, `tests/integration_e2e.rs`
  - Pre-commit: `cargo test`

---

## Final Verification Wave

- [x] F1. **Plan Compliance Audit** - `oracle`
  Read the plan end-to-end. For each "Must Have": verify implementation exists (read file, run command). For each "Must NOT Have": search codebase for forbidden patterns - reject with file:line if found. Check evidence files exist in .sisyphus/evidence/. Compare deliverables against plan.
  Output: `Must Have [N/N] | Must NOT Have [N/N] | Tasks [N/N] | VERDICT: APPROVE/REJECT`

- [x] F2. **Code Quality Review** - `unspecified-high`
  Run `cargo test`, `cargo clippy`. Review all changed files for: `as any`/unsafe code without justification, empty catches, debug prints in prod. Check AI slop: excessive comments, over-abstraction, generic names.
  Output: `Build [PASS/FAIL] | Tests [N pass/N fail] | Clippy [N warnings] | VERDICT`

- [x] F3. **Real Manual QA** - `unspecified-high`
  Start from clean state. Execute EVERY QA scenario from EVERY task - follow exact steps, capture evidence. Test cross-task integration (parse + stringify roundtrip). Test edge cases: empty string, overflow, invalid chars. Save to `.sisyphus/evidence/final-qa/`.
  Output: `Scenarios [N/N pass] | Integration [N/N] | Edge Cases [N tested] | VERDICT`

- [x] F4. **Scope Fidelity Check** - `deep`
  For each task: read "What to do", read actual diff (git diff). Verify 1:1 - everything in spec was built (no missing), nothing beyond spec was built (no creep). Check "Must NOT do" compliance. Flag unaccounted changes.
  Output: `Tasks [N/N compliant] | Contamination [CLEAN/N issues] | Unaccounted [CLEAN/N files] | VERDICT`

---

## Commit Strategy

| Commit | Message | Files | Pre-commit |
|--------|---------|-------|------------|
| 1 | `fix(spec): correct guard syntax in error_handling_samples.op` | `language-spec/error_handling_samples.op` | `cargo test` |
| 2 | `feat(runtime): add fail-fast parse functions with struct returns` | `runtime/opal_runtime.c` | N/A |
| 3 | `feat(runtime): add numeric-to-string conversion functions` | `runtime/opal_runtime.c` | N/A |
| 4 | `feat(codegen): handle struct returns from parse functions` | `src/codegen/*.rs` | `cargo test --lib` |
| 5 | `feat(types): register parse builtins with errors ParseError` | `src/type_system/**/*.rs` | `cargo test --lib` |
| 6 | `test: update tests for fail-fast conversion model` | `src/**/*tests*.rs` | `cargo test` |

---

## Success Criteria

### Verification Commands
```bash
# All tests pass
cargo test  # Expected: 958+ tests pass, 0 failures

# Type error on bare call (compile this test program)
echo 'let n = string_to_int32("5")' | cargo run -- check -  
# Expected: error about unhandled ParseError

# Guard pattern works
cargo run -- run test-projects/simple-quiz/src/main.op
# Expected: Program runs, handles parse errors gracefully
```

### Final Checklist
- [ ] All "Must Have" present (struct returns, error messages, compile-time enforcement, TDD)
- [ ] All "Must NOT Have" absent (no silent zeros, no panics)
- [ ] All 958+ tests pass
- [ ] `simple_quiz` test project compiles and runs
