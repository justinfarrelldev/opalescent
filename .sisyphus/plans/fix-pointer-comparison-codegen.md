# Fix Pointer Comparison Codegen (String/Function `is` Operator)

## TL;DR

> **Quick Summary**: Fix `codegen_cmp` to handle pointer types (strings, function pointers) instead of panicking with "expected IntValue variant".
> 
> **Deliverables**:
> - String `is`/`is not` comparisons work via `strcmp`
> - Function pointer comparisons work via direct pointer `icmp eq/ne`
> - `basic-calculator` test project compiles and runs
> 
> **Estimated Effort**: Medium
> **Parallel Execution**: NO - sequential (tests depend on implementation)
> **Critical Path**: Task 1 → Task 2 → Task 3 → Task 4

---

## Context

### Original Request
Fix the panic in `basic-calculator` test project:
```
thread 'main' panicked at src/codegen/expressions_numeric.rs:242:13:
Found PointerValue(...) but expected the IntValue variant
```

The code `if operation is '+'` compares strings, but `codegen_cmp` assumes all non-float values are integers.

### Research Findings

**Root Cause** (`src/codegen/expressions_numeric.rs:216-288`):
```rust
pub fn codegen_cmp<'context>(...) {
    if lhs.is_float_value() {
        // float comparison - OK
    }
    // MISSING: if lhs.is_pointer_value() { ... }
    
    lhs.into_int_value()  // ← PANICS on strings (PointerValue)
}
```

**Type Checker Allows** (verified in `src/type_system/checker/expressions.rs:547-567`):
- `BinaryOp::Equal | NotEqual | Is | IsNot` for ANY compatible types
- This includes strings, function pointers, and generic types

**LLVM Type Mapping** (`src/codegen/types.rs:22-28`):
- `CoreType::String` → `i8*` (pointer)
- `CoreType::Function` → `i8*` (pointer)
- `CoreType::Generic` → `i8*` BUT actual ADT values are structs, not pointers

**ADTs are NOT pointers** (`src/codegen/adts.rs:277-283`):
```rust
// Sum types are structs: { i64 tag, [64 x i8] payload }
let tagged_type = context.struct_type(&[
    context.i64_type().into(),
    context.i8_type().array_type(64).into(),
], false);
```

### Scope Clarification
- **Strings**: Need `strcmp` call - MUST FIX (immediate need for basic-calculator)
- **Function pointers**: Need direct pointer `icmp eq/ne` - MUST FIX (type checker allows it)
- **ADTs**: Are struct values, not pointers - OUT OF SCOPE for this fix

### Architectural Discovery
The `expected_type` parameter in `codegen_cmp` is the *expression's* expected type, NOT the operand type. It's often `None` (e.g., from control_flow.rs:20).

**However**, we CAN get operand type info via:
1. **VariableBinding.core_type** (`expressions.rs:60`) - Environment stores CoreType for each variable
2. **infer_core_type_from_expr** (`statements.rs:274-286`) - Can infer type from literals

**Solution**: Create a helper `infer_operand_type(expr, env)` that:
- For `Expr::Identifier` → look up `env.variables[name].core_type`
- For `Expr::Literal` → use literal type (String, Int64, etc.)
- For other expressions → return `None` (fall back to strcmp)

This allows proper discrimination between string and function pointer comparisons.

---

## Work Objectives

### Core Objective
Add pointer comparison support to `codegen_cmp` so string and function pointer `is`/`is not` comparisons work correctly.

### Concrete Deliverables
- Modified `src/codegen/expressions_numeric.rs` with pointer comparison branch
- Modified `src/codegen/expressions.rs` with operand type inference
- New tests for string AND function pointer comparison codegen
- `test-projects/basic-calculator` compiles and runs correctly

### Definition of Done
- [ ] `cargo test test_codegen_string_is_comparison` passes
- [ ] `cargo test test_codegen_string_is_not_comparison` passes
- [ ] `cargo test test_codegen_function_pointer_is_comparison` passes
- [ ] `cargo test test_codegen_function_pointer_is_not_comparison` passes
- [ ] `cargo build --release && ./target/release/opalescent test-projects/basic-calculator/src/main.op --run` succeeds
- [ ] All existing tests still pass: `cargo test --lib`

### Must Have
- String `is` comparison calls `strcmp` and checks result == 0
- String `is not` comparison calls `strcmp` and checks result != 0
- Function pointer `is` comparison uses direct `icmp eq` on pointers
- Function pointer `is not` comparison uses direct `icmp ne` on pointers
- Helper function to infer operand type from expression

### Must NOT Have (Guardrails)
- Do NOT modify type checker - it correctly allows these comparisons
- Do NOT attempt to handle ADT/struct comparisons in this fix
- Do NOT change the existing integer or float comparison logic
- Do NOT add relational operators (`<`, `>`) for strings (type checker blocks these)

---

## Verification Strategy

### Test Decision
- **Infrastructure exists**: YES (cargo test)
- **Automated tests**: YES (TDD - RED-GREEN-REFACTOR)
- **Framework**: Rust's built-in `#[test]`

### TDD Approach
1. **RED**: Add failing tests for string `is` and `is not` comparisons
2. **GREEN**: Implement `codegen_pointer_cmp` function
3. **REFACTOR**: Clean up, ensure all tests pass

### QA Policy
- Run `cargo test --lib` to verify no regressions
- Run basic-calculator test project end-to-end

---

## Execution Strategy

### Sequential Execution (TDD requires order)

```
Task 1: RED - Add failing tests for string comparison
    ↓
Task 2: GREEN - Implement pointer comparison in codegen_cmp
    ↓
Task 3: Verify basic-calculator test project works
    ↓
Task 4: Final verification - all tests pass
```

---

## TODOs

- [x] 1. RED: Add Failing Tests for String and Function Pointer Comparison

  **What to do**:
  
  **String comparison tests**:
  - Add test `test_codegen_string_is_comparison_emits_strcmp` to `src/codegen/tests.rs`
    - Create string variable binding: `let x = "hello"`
    - Compare: `x is "hello"`
    - Assert codegen succeeds (currently will panic)
    - Assert IR contains `strcmp` call
  - Add test `test_codegen_string_is_not_comparison_emits_strcmp`
    - Similar but with `IsNot` operator
    - Assert IR contains `strcmp` and `icmp ne`

  **Function pointer comparison tests**:
  - Add test `test_codegen_function_pointer_is_comparison_emits_icmp`
    - Create two function pointer variables
    - Compare: `f1 is f2`
    - Assert codegen succeeds (currently will panic)
    - Assert IR contains `icmp eq` (NOT strcmp)
  - Add test `test_codegen_function_pointer_is_not_comparison_emits_icmp`
    - Similar but with `IsNot` operator
    - Assert IR contains `icmp ne`

  - Run tests to confirm they fail (panic with PointerValue error)

  **Must NOT do**:
  - Do NOT implement the fix yet - just write failing tests
  - Do NOT modify expressions_numeric.rs or expressions.rs

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Adding tests is straightforward, patterns exist in the file
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: NO
  - **Parallel Group**: Sequential
  - **Blocks**: Task 2
  - **Blocked By**: None

  **References**:
  - `src/codegen/tests.rs:560-587` - Existing `test_codegen_is_operator_on_int64` pattern to follow
  - `src/codegen/tests.rs:68-74` - `string_lit` helper function
  - `src/codegen/tests.rs:92-100` - `binary` helper for creating BinaryOp expressions
  - `src/codegen/tests.rs:44-50` - `int_lit` pattern (adapt for strings)
  - `src/codegen/functions.rs:87-92` - Pattern for creating function pointer VariableBinding

  **Acceptance Criteria**:
  - [ ] Test file compiles: `cargo build --lib`
  - [ ] String tests fail with expected panic: `cargo test test_codegen_string_is_comparison 2>&1 | grep -q "PointerValue"`
  - [ ] Function pointer tests fail with expected panic: `cargo test test_codegen_function_pointer_is_comparison 2>&1 | grep -q "PointerValue"`
  - [ ] Test structure follows existing patterns in the file

  **QA Scenarios**:
  ```
  Scenario: New comparison tests compile successfully
    Tool: Bash
    Preconditions: None
    Steps:
      1. Run `cargo build --lib 2>&1`
      2. Verify exit code is 0
      3. Verify no "error[E" in output
    Expected Result: Build succeeds with exit code 0
    Failure Indicators: Non-zero exit code, "error[E" in output
    Evidence: .sisyphus/evidence/task-1-tests-compile.txt

  Scenario: String is comparison test fails with expected PointerValue panic
    Tool: Bash
    Preconditions: Tests compile
    Steps:
      1. Run `cargo test test_codegen_string_is_comparison 2>&1`
      2. Capture output to file
      3. Verify output contains "PointerValue" (the expected panic)
      4. Verify test result shows FAILED (not passed)
    Expected Result: Test fails with panic message containing "PointerValue"
    Failure Indicators: Test passes (would mean fix already exists), different panic message
    Evidence: .sisyphus/evidence/task-1-string-is-fails.txt

  Scenario: String is-not comparison test fails with expected PointerValue panic
    Tool: Bash
    Preconditions: Tests compile
    Steps:
      1. Run `cargo test test_codegen_string_is_not_comparison 2>&1`
      2. Capture output to file
      3. Verify output contains "PointerValue"
    Expected Result: Test fails with panic message containing "PointerValue"
    Failure Indicators: Test passes, different panic message
    Evidence: .sisyphus/evidence/task-1-string-is-not-fails.txt

  Scenario: Function pointer is comparison test fails with expected PointerValue panic
    Tool: Bash
    Preconditions: Tests compile
    Steps:
      1. Run `cargo test test_codegen_function_pointer_is_comparison 2>&1`
      2. Capture output to file
      3. Verify output contains "PointerValue" (the expected panic)
    Expected Result: Test fails with panic message containing "PointerValue"
    Failure Indicators: Test passes, different panic message
    Evidence: .sisyphus/evidence/task-1-fnptr-is-fails.txt

  Scenario: Function pointer is-not comparison test fails with expected PointerValue panic
    Tool: Bash
    Preconditions: Tests compile
    Steps:
      1. Run `cargo test test_codegen_function_pointer_is_not_comparison 2>&1`
      2. Capture output to file
      3. Verify output contains "PointerValue"
    Expected Result: Test fails with panic message containing "PointerValue"
    Failure Indicators: Test passes, different panic message
    Evidence: .sisyphus/evidence/task-1-fnptr-is-not-fails.txt
  ```

  **Commit**: YES
  - Message: `test(codegen): add failing tests for string and function pointer comparisons`
  - Files: `src/codegen/tests.rs`

---

- [x] 2. GREEN: Implement Pointer Comparison in codegen_cmp

  **What to do**:
  
  **Step 2a: Add operand type inference helper** in `src/codegen/expressions.rs`:
  ```rust
  /// Infer the CoreType of an expression for comparison purposes.
  /// Returns None if type cannot be determined (fallback to strcmp).
  fn infer_operand_type(expr: &Expr, env: &CodegenEnv) -> Option<CoreType> {
      match expr {
          Expr::Identifier { name, .. } => {
              env.variables.get(name).map(|binding| binding.core_type.clone())
          }
          Expr::Literal { value, .. } => Some(match value {
              LiteralValue::String(_) => CoreType::String,
              LiteralValue::Integer(_) => CoreType::Int64,
              LiteralValue::Float(_) => CoreType::Float64,
              LiteralValue::Boolean(_) => CoreType::Boolean,
              LiteralValue::Void => CoreType::Unit,
          }),
          // For complex expressions, return None (will use strcmp as fallback)
          _ => None,
      }
  }
  ```

  **Step 2b: Modify `codegen_binary`** to pass operand type to comparison:
  - For comparison operators, infer the left operand's type
  - Pass it to `codegen_cmp` via a new parameter
  ```rust
  BinaryOp::Equal | BinaryOp::NotEqual | ... | BinaryOp::Is | BinaryOp::IsNot => {
      let operand_type = infer_operand_type(left, env);
      codegen_cmp(codegen_context, lhs, rhs, operator, operand_type.as_ref())
  }
  ```

  **Step 2c: Modify `codegen_cmp` signature** in `src/codegen/expressions_numeric.rs`:
  - Change `expected_type: Option<&CoreType>` to `operand_type: Option<&CoreType>`
  - This parameter now represents the OPERAND type, not expression expected type
  - After the float check, add pointer check:
  ```rust
  if lhs.is_pointer_value() {
      return codegen_pointer_cmp(codegen_context, lhs, rhs, operator, operand_type);
  }
  ```

  **Step 2d: Add `codegen_pointer_cmp` function**:
  ```rust
  fn codegen_pointer_cmp<'context>(
      codegen_context: &CodegenContext<'context>,
      lhs: BasicValueEnum<'context>,
      rhs: BasicValueEnum<'context>,
      operator: &BinaryOp,
      operand_type: Option<&CoreType>,
  ) -> Result<BasicValueEnum<'context>, CodegenError> {
      let lhs_ptr = lhs.into_pointer_value();
      let rhs_ptr = rhs.into_pointer_value();
      
      // Determine comparison strategy based on operand type
      let is_string = matches!(operand_type, Some(CoreType::String) | None);
      // None defaults to strcmp (safe fallback for unknown pointer types)
      
      if is_string {
          // String comparison: use strcmp
          let strcmp_fn = ensure_strcmp_function(codegen_context);
          let strcmp_result = codegen_context.builder.build_call(
              strcmp_fn,
              &[lhs_ptr.into(), rhs_ptr.into()],
              "strcmp_result",
          )?.try_as_basic_value().left().unwrap().into_int_value();
          
          let zero = codegen_context.context.i32_type().const_int(0, false);
          let pred = match *operator {
              BinaryOp::Equal | BinaryOp::Is => IntPredicate::EQ,
              BinaryOp::NotEqual | BinaryOp::IsNot => IntPredicate::NE,
              _ => return Err(CodegenError::new("unsupported string comparison operator")),
          };
          
          Ok(codegen_context.builder.build_int_compare(pred, strcmp_result, zero, "str_cmp")?
              .as_basic_value_enum())
      } else {
          // Function pointer comparison: direct pointer icmp
          let pred = match *operator {
              BinaryOp::Equal | BinaryOp::Is => IntPredicate::EQ,
              BinaryOp::NotEqual | BinaryOp::IsNot => IntPredicate::NE,
              _ => return Err(CodegenError::new("unsupported pointer comparison operator")),
          };
          
          // Cast pointers to integers for comparison
          let ptr_int_type = codegen_context.context.i64_type();
          let lhs_int = codegen_context.builder.build_ptr_to_int(lhs_ptr, ptr_int_type, "lhs_ptr_int")?;
          let rhs_int = codegen_context.builder.build_ptr_to_int(rhs_ptr, ptr_int_type, "rhs_ptr_int")?;
          
          Ok(codegen_context.builder.build_int_compare(pred, lhs_int, rhs_int, "ptr_cmp")?
              .as_basic_value_enum())
      }
  }
  ```

  **Step 2e: Add `ensure_strcmp_function` helper** (follow pattern from `expressions_string.rs:170-187`):
  ```rust
  fn ensure_strcmp_function<'context>(
      codegen_context: &CodegenContext<'context>,
  ) -> FunctionValue<'context> {
      let i8_ptr = codegen_context.context.ptr_type(AddressSpace::default());
      let i32_type = codegen_context.context.i32_type();
      let fn_type = i32_type.fn_type(&[i8_ptr.into(), i8_ptr.into()], false);
      
      codegen_context.module.get_function("strcmp").unwrap_or_else(|| {
          codegen_context.module.add_function("strcmp", fn_type, Some(Linkage::External))
      })
  }
  ```

  **Files to modify**:
  1. `src/codegen/expressions.rs` - Add `infer_operand_type`, modify `codegen_binary`
  2. `src/codegen/expressions_numeric.rs` - Modify `codegen_cmp`, add `codegen_pointer_cmp`, add `ensure_strcmp_function`

  **Must NOT do**:
  - Do NOT handle `<`, `>`, `<=`, `>=` for strings (type checker blocks these)
  - Do NOT modify existing float or integer comparison logic
  - Do NOT modify the type checker

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: Requires understanding LLVM IR generation patterns, modifying multiple files, and careful integration
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: NO
  - **Parallel Group**: Sequential
  - **Blocks**: Task 3
  - **Blocked By**: Task 1

  **References**:
  - `src/codegen/expressions_numeric.rs:216-288` - Current `codegen_cmp` function to modify
  - `src/codegen/expressions.rs:259-300` - `codegen_binary` function to modify (add operand type inference)
  - `src/codegen/expressions.rs:58-61` - `VariableBinding` struct with `core_type` field
  - `src/codegen/expressions.rs:63-70` - `CodegenEnv` with `variables` map
  - `src/codegen/statements.rs:274-286` - Existing `infer_core_type_from_expr` pattern to follow
  - `src/codegen/expressions_string.rs:170-187` - Pattern for `ensure_sprintf_function` (adapt for strcmp)
  - `src/codegen/expressions_numeric.rs:246-248` - Pattern for IntPredicate::EQ/NE usage

  **Acceptance Criteria**:
  - [ ] `cargo test test_codegen_string_is_comparison` passes
  - [ ] `cargo test test_codegen_string_is_not_comparison` passes
  - [ ] `cargo test test_codegen_function_pointer_is_comparison` passes
  - [ ] `cargo test test_codegen_function_pointer_is_not_comparison` passes
  - [ ] No compiler warnings: `cargo build --lib 2>&1 | grep -v "^warning:" | grep -c warning` returns 0
  - [ ] Generated IR contains `strcmp` call for string comparisons
  - [ ] Generated IR contains `ptrtoint` + `icmp` for function pointer comparisons (NOT strcmp)

  **QA Scenarios**:
  ```
  Scenario: String equality comparison generates strcmp call
    Tool: Bash (cargo test)
    Preconditions: Task 1 tests exist
    Steps:
      1. Run `cargo test test_codegen_string_is_comparison -- --nocapture`
      2. Verify test passes (exit code 0)
      3. Check test output confirms strcmp in IR
    Expected Result: Test passes, IR contains "strcmp"
    Evidence: .sisyphus/evidence/task-2-string-is-cmp.txt

  Scenario: String inequality comparison generates strcmp with ne
    Tool: Bash (cargo test)
    Preconditions: Task 1 tests exist
    Steps:
      1. Run `cargo test test_codegen_string_is_not_comparison -- --nocapture`
      2. Verify test passes
    Expected Result: Test passes, IR contains "strcmp" and "icmp ne"
    Evidence: .sisyphus/evidence/task-2-string-is-not-cmp.txt

  Scenario: Function pointer equality comparison generates icmp (NOT strcmp)
    Tool: Bash (cargo test)
    Preconditions: Task 1 function pointer tests exist
    Steps:
      1. Run `cargo test test_codegen_function_pointer_is_comparison -- --nocapture`
      2. Verify test passes (exit code 0)
      3. Check test output confirms ptrtoint and icmp eq in IR
      4. Verify NO strcmp in IR (function pointers don't use strcmp)
    Expected Result: Test passes, IR contains "ptrtoint" and "icmp eq", NO "strcmp"
    Failure Indicators: Test fails, or IR contains "strcmp" for function pointer comparison
    Evidence: .sisyphus/evidence/task-2-fnptr-is-cmp.txt

  Scenario: Function pointer inequality comparison generates icmp ne
    Tool: Bash (cargo test)
    Preconditions: Task 1 function pointer tests exist
    Steps:
      1. Run `cargo test test_codegen_function_pointer_is_not_comparison -- --nocapture`
      2. Verify test passes
      3. Verify IR contains "icmp ne", NO strcmp
    Expected Result: Test passes, IR contains "ptrtoint" and "icmp ne"
    Evidence: .sisyphus/evidence/task-2-fnptr-is-not-cmp.txt
  ```

  **Commit**: YES
  - Message: `fix(codegen): handle string and function pointer comparisons in codegen_cmp`
  - Files: `src/codegen/expressions_numeric.rs`, `src/codegen/expressions.rs`

---

- [x] 3. Verify basic-calculator Test Project Works

  **What to do**:
  - Compile and run the basic-calculator test project:
    ```bash
    cd /home/justi/Projects/opalescent
    cargo build --release
    ./target/release/opalescent test-projects/basic-calculator/src/main.op --run
    ```
  - The program should:
    1. Print "Enter +, -, *, /"
    2. Accept input
    3. Print the matching operation message
  - If it fails, debug and fix any remaining issues

  **Must NOT do**:
  - Do NOT modify the basic-calculator source code (it's correct)
  - Do NOT skip this verification step

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Just running verification commands
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: NO
  - **Parallel Group**: Sequential
  - **Blocks**: Task 4
  - **Blocked By**: Task 2

  **References**:
  - `test-projects/basic-calculator/src/main.op` - The source file that was failing
  - `test-projects/basic-calculator/opal.toml` - Project config

  **Acceptance Criteria**:
  - [ ] `cargo build --release` succeeds
  - [ ] `./target/release/opalescent test-projects/basic-calculator/src/main.op` compiles without panic
  - [ ] Running with `--run` and providing input works correctly

  **QA Scenarios**:
  ```
  Scenario: basic-calculator compiles without panic
    Tool: Bash
    Preconditions: Task 2 implementation complete
    Steps:
      1. Run `cargo build --release`
      2. Run `./target/release/opalescent test-projects/basic-calculator/src/main.op`
      3. Verify no panic occurs, compilation succeeds
    Expected Result: Exit code 0, no "panicked" in output
    Evidence: .sisyphus/evidence/task-3-basic-calc-compile.txt

  Scenario: basic-calculator runs and handles input
    Tool: Bash
    Preconditions: Compilation succeeds
    Steps:
      1. Run `echo "+" | ./target/release/opalescent test-projects/basic-calculator/src/main.op --run`
      2. Verify output contains "Operation is +"
    Expected Result: Output shows correct operation message
    Evidence: .sisyphus/evidence/task-3-basic-calc-run.txt
  ```

  **Commit**: NO (no code changes expected)

---

- [x] 4. Final Verification - All Tests Pass

  **What to do**:
  - Run the full test suite to ensure no regressions:
    ```bash
    cargo test --lib
    ```
  - Verify all 954+ tests pass
  - If any tests fail, investigate and fix

  **Must NOT do**:
  - Do NOT skip any failing tests
  - Do NOT mark this complete if tests fail

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Running verification commands
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: NO
  - **Parallel Group**: Sequential (final)
  - **Blocks**: None
  - **Blocked By**: Task 3

  **References**:
  - All previous tasks

  **Acceptance Criteria**:
  - [ ] `cargo test --lib` shows all tests passing
  - [ ] No new warnings introduced
  - [ ] `cargo clippy` passes (if available)

  **QA Scenarios**:
  ```
  Scenario: Full test suite passes
    Tool: Bash
    Preconditions: Tasks 1-3 complete
    Steps:
      1. Run `cargo test --lib 2>&1 | tail -5`
      2. Verify output shows "test result: ok"
      3. Verify no "FAILED" in output
    Expected Result: All tests pass, "test result: ok. X passed"
    Evidence: .sisyphus/evidence/task-4-full-test-suite.txt
  ```

  **Commit**: NO (verification only, unless fixes needed)

---

## Commit Strategy

| Task | Commit | Message | Files |
|------|--------|---------|-------|
| 1 | YES | `test(codegen): add failing tests for string and function pointer comparisons` | `src/codegen/tests.rs` |
| 2 | YES | `fix(codegen): handle string and function pointer comparisons in codegen_cmp` | `src/codegen/expressions_numeric.rs`, `src/codegen/expressions.rs` |
| 3 | NO | - | - |
| 4 | NO | - | - |

---

## Success Criteria

### Verification Commands
```bash
# All new string tests pass
cargo test test_codegen_string_is  # Expected: 2 tests pass

# All new function pointer tests pass
cargo test test_codegen_function_pointer  # Expected: 2 tests pass

# basic-calculator works
./target/release/opalescent test-projects/basic-calculator/src/main.op  # Expected: no panic

# Full suite passes
cargo test --lib  # Expected: 958+ tests pass (954 existing + 4 new)
```

### Final Checklist
- [ ] String `is` comparison works (uses strcmp)
- [ ] String `is not` comparison works (uses strcmp)
- [ ] Function pointer `is` comparison works (uses icmp eq)
- [ ] Function pointer `is not` comparison works (uses icmp ne)
- [ ] basic-calculator test project compiles and runs
- [ ] All existing tests still pass
- [ ] No new compiler warnings
