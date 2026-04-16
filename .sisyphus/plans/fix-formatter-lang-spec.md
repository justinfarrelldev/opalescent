# Fix Formatter to Adhere to Opalescent Language Spec

## TL;DR

> **Quick Summary**: Fix the Opalescent code formatter to output colon-block syntax (`:` + indented body) instead of brace syntax (`{...}`) for control flow, matching the language specification.
> 
> **Deliverables**:
> - TDD regression tests that detect braces/semicolons in formatter output
> - Fixed `print_stmt` methods in `printer.rs` for If, While, For, Loop, Block
> - Updated existing tests to expect colon-block syntax
> - Documentation comments noting language spec adherence
> 
> **Estimated Effort**: Medium
> **Parallel Execution**: YES - 2 waves
> **Critical Path**: Task 1 (regression tests) → Tasks 2-6 (formatter fixes) → Task 7 (update tests) → Task 8 (verify)

---

## Context

### Original Request
The formatter outputs braces `{...}` where the Opalescent language specification uses colon-block syntax (`:` followed by indented body). The user wants TDD-style regression tests to prevent future violations and the formatter fixed to match the spec.

### Research Findings

**Language Spec Examples** (from `language-spec/*.op`):
```opal
# if-statement: colon + indented body
if n is 0:
    return 0

# while-loop: colon + indented body
while i <= n:
    result = a + b
    i = i + 1

# for-loop: colon + indented body  
for x in xs:
    out.push(fn(x))

# loop expression: arrow + indented body
loop =>
    let s = take_input()
    break user_input: s, user_number: n
```

**Current Formatter Output** (incorrect):
```opal
if x is 1 {
    return void
}

while cond {
    continue
}

for x in items {
    continue
}

loop {
    break
}
```

**Key Files**:
- `src/formatter/printer.rs` - The `print_stmt` method generates the incorrect output
- `src/formatter/tests.rs` - Contains tests expecting brace syntax that need updating

### Interview Summary
- **Test Strategy**: TDD approach - write failing tests first, then fix formatter
- **Scope**: Only control flow statements (if, while, for, loop, block)
- **Exclusion**: Match expressions still use brace syntax per existing tests

---

## Work Objectives

### Core Objective
Make the formatter output syntactically valid Opalescent code per the language specification, using colon-block syntax for control flow instead of braces.

### Concrete Deliverables
- 8+ regression tests in `src/formatter/tests.rs` detecting braces/semicolons
- Fixed `print_stmt` in `src/formatter/printer.rs` for Block, If, While, For, Loop
- Updated 6 existing tests that expected brace syntax
- Documentation comments in printer.rs noting spec adherence

### Definition of Done
- [ ] `cargo test --package opalescent` passes (all formatter tests)
- [ ] No formatter output contains `if ... {`, `while ... {`, `for ... {`, or `loop {`
- [ ] All control flow uses colon-block or arrow syntax
- [ ] Formatted output re-parses without errors

### Must Have
- Regression tests that FAIL if braces appear in control flow output
- Regression tests that FAIL if semicolons appear in control flow output
- Colon syntax for if/while/for statements
- Arrow syntax for loop expressions
- All existing tests pass after updates

### Must NOT Have (Guardrails)
- No changes to match expression formatting (still uses braces per spec)
- No changes to lambda/function body syntax (still uses arrow)
- No semicolons in output (language uses newlines)
- No breaking changes to the public formatter API

---

## Verification Strategy

> **ZERO HUMAN INTERVENTION** - ALL verification is agent-executed. No exceptions.

### Test Decision
- **Infrastructure exists**: YES
- **Automated tests**: TDD (write failing tests first)
- **Framework**: cargo test (Rust built-in)
- **If TDD**: Each task follows RED (failing test) → GREEN (minimal impl) → REFACTOR

### QA Policy
Every task includes agent-executed QA scenarios.
Evidence saved to `.sisyphus/evidence/task-{N}-{scenario-slug}.{ext}`.

---

## Execution Strategy

### Parallel Execution Waves

```
Wave 1 (Start Immediately - TDD tests):
├── Task 1: Write regression tests for language spec compliance [quick]

Wave 2 (After Wave 1 - parallel formatter fixes):
├── Task 2: Fix Stmt::Block to use colon syntax [quick]
├── Task 3: Fix Stmt::If to use colon syntax [quick]
├── Task 4: Fix Stmt::While to use colon syntax [quick]
├── Task 5: Fix Stmt::For to use colon syntax [quick]
├── Task 6: Fix Stmt::Loop to use arrow syntax [quick]

Wave 3 (After Wave 2 - test updates and verification):
├── Task 7: Update existing tests expecting brace syntax [quick]
├── Task 8: Add documentation comments and final verification [quick]

Critical Path: Task 1 → Tasks 2-6 → Task 7 → Task 8
Parallel Speedup: ~50% faster than sequential
Max Concurrent: 5 (Wave 2)
```

### Dependency Matrix
- **1**: - → 2-6, 1
- **2-6**: 1 → 7, 2
- **7**: 2-6 → 8, 3
- **8**: 7 → -, 3

### Agent Dispatch Summary
- **Wave 1**: **1** - T1 → `quick`
- **Wave 2**: **5** - T2-T6 → `quick` (all parallel)
- **Wave 3**: **2** - T7-T8 → `quick`

---

## TODOs

- [x] 1. Write TDD regression tests for language spec compliance

  **What to do**:
  - Add new test section at end of `src/formatter/tests.rs` titled "Language Spec Compliance Tests (Regression Prevention)"
  - Add comment block explaining these tests enforce Opalescent language spec
  - Write `test_spec_compliance_if_no_braces` - asserts output contains `if x is 1:` NOT `if x is 1 {`
  - Write `test_spec_compliance_while_no_braces` - asserts output contains `while cond:` NOT `while cond {`
  - Write `test_spec_compliance_for_no_braces` - asserts output contains `for x in items:` NOT `for x in items {`
  - Write `test_spec_compliance_loop_no_braces` - asserts output contains `loop =>` NOT `loop {`
  - Write `test_spec_compliance_no_semicolons_in_control_flow` - counts semicolons, asserts 0
  - Write `test_spec_compliance_function_body_arrow_syntax` - asserts function bodies use `=> { ... }` (braces kept for functions, colon only for control flow)
  - Write `test_spec_compliance_nested_control_flow_no_braces` - tests deep nesting
  - Write `test_spec_compliance_formatted_output_parses_cleanly` - formats inline test source, then lex+parse (pattern from `test_formatter_comments_reparse_clean`)
  - Each test comment should state "REGRESSION TEST" and reference "language spec"

  **Test implementation pattern** (follow existing codebase pattern from `test_formatter_comments_reparse_clean`):
  ```rust
  #[test]
  fn test_spec_compliance_if_no_braces() {
      // REGRESSION TEST: Ensures if-statements use colon-block syntax per language spec.
      let source = "entry main = f(): void =>\n    if true:\n        return void\n    return void\n";
      let formatted = Formatter::with_defaults()
          .format_source(source)
          .expect("should format");
      assert!(formatted.contains("if true:"), "if-statement should use colon syntax");
      assert!(!formatted.contains("if true {"), "if-statement must NOT use brace syntax");
  }
  
  #[test]
  fn test_spec_compliance_formatted_output_parses_cleanly() {
      // REGRESSION TEST: Formatted control flow must lex and parse without errors.
      let source = "entry main = f(): void =>\n    if true:\n        return void\n    return void\n";
      let formatted = Formatter::with_defaults()
          .format_source(source)
          .expect("should format");
      let lexer = crate::lexer::Lexer::new(&formatted);
      let (tokens, lex_errors) = lexer.tokenize();
      assert!(lex_errors.errors.is_empty(), "should lex: {lex_errors:?}");
      let parser = crate::parser::Parser::new(tokens);
      let (_program, parse_errors) = parser.parse();
      assert!(parse_errors.errors.is_empty(), "should parse: {parse_errors:?}");
  }
  ```

  **Must NOT do**:
  - Do not modify any existing tests yet
  - Do not change the formatter code yet

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Adding tests to an existing test file, straightforward additions
  - **Skills**: `[]`
    - No special skills needed

  **Parallelization**:
  - **Can Run In Parallel**: NO (must complete before formatter fixes)
  - **Parallel Group**: Wave 1 (alone)
  - **Blocks**: Tasks 2, 3, 4, 5, 6
  - **Blocked By**: None (can start immediately)

  **References**:

  **Pattern References**:
  - `src/formatter/tests.rs:897-1056` - Existing comment preservation tests show test structure pattern
  - `src/formatter/tests.rs:1058-1193` - Recent body leading comment tests show assertion patterns

  **API/Type References**:
  - `src/formatter/printer.rs:221-237` - Formatter struct and with_defaults()
  - `crate::lexer::Lexer` and `crate::parser::Parser` - For re-parse verification

  **External References**:
  - `language-spec/fib_iterative.op` - Shows `if n is 0:` and `while i <= n:` colon syntax
  - `language-spec/array_helpers.op` - Shows `for x in xs:` colon syntax
  - `language-spec/simple_quiz.op` - Shows `loop =>` arrow syntax

  **WHY Each Reference Matters**:
  - Test structure patterns ensure new tests follow existing conventions
  - Language spec files show EXACTLY what syntax the formatter should output

  **Acceptance Criteria**:
  - [ ] 8+ new test functions added to `src/formatter/tests.rs`
  - [ ] Each test has "REGRESSION TEST" in comment
  - [ ] Tests currently FAIL when run (expected - formatter not fixed yet)
  - [ ] `cargo test test_spec_compliance --package opalescent` exits non-zero (tests fail in RED phase)

  **QA Scenarios**:

  ```
  Scenario: Verify regression tests exist and fail (TDD RED phase)
    Tool: Bash
    Preconditions: Tests written but formatter not yet fixed
    Steps:
      1. Run: cargo test test_spec_compliance --package opalescent 2>&1
      2. Check exit code is non-zero (tests should FAIL at this stage)
      3. Verify output contains "FAILED" text
    Expected Result: Exit code non-zero, tests fail (this is expected in TDD RED phase)
    Failure Indicators: Exit code 0 (tests pass, means tests don't detect the problem) or compile errors
    Evidence: .sisyphus/evidence/task-1-regression-tests-fail.txt

  Scenario: Verify test file compiles
    Tool: Bash
    Preconditions: Tests written
    Steps:
      1. Run: cargo check --package opalescent 2>&1
      2. Check exit code is 0
    Expected Result: Compilation succeeds
    Evidence: .sisyphus/evidence/task-1-compile-check.txt
  ```

  **Commit**: YES
  - Message: `test(formatter): add regression tests for language spec compliance`
  - Files: `src/formatter/tests.rs`
  - Pre-commit: `cargo check --package opalescent`

- [x] 2. Special-case control flow printing (keep Stmt::Block braced globally)

  **What to do**:
  - **KEEP `Stmt::Block` printing braces globally** (do NOT change Stmt::Block match arm)
  - Instead, modify `Stmt::If`, `Stmt::While`, `Stmt::For`, `Stmt::Loop` to NOT call `print_stmt` on their body block
  - Create a helper `print_block_body_indented(&self, block: &Stmt, depth: usize) -> String` that:
    - Extracts statements from a `Stmt::Block`
    - Prints each statement at `depth` indentation (no braces, no colon)
    - For empty blocks: output `# empty` comment placeholder
  - Control flow statements emit their own header (`if cond:`, `while cond:`, `for x in iter:`, `loop =>`) then call the helper for body contents
  - This approach leaves `Stmt::Block` unchanged, so function bodies (`=> { ... }`) continue to work

  **Why this approach** (critical architectural decision):
  - `Stmt::Block` is used in MANY contexts: function bodies, lambdas, match arms, control flow
  - `Decl::Function` prints `=> {body_str}` where body is a Stmt::Block - must keep working
  - Changing Stmt::Block globally would break function bodies (would become `=> :` or similar)
  - Minimal safe change: only control flow statements get special printing, Block unchanged

  **Must NOT do**:
  - Do NOT modify `Stmt::Block` match arm in print_stmt
  - Do NOT emit `pass` statement (doesn't exist in parser)
  - Do NOT break function body printing (`=> { ... }` pattern must remain)

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Adding a helper method and refactoring one match arm
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2 (with Tasks 3, 4, 5, 6)
  - **Blocks**: Task 7
  - **Blocked By**: Task 1

  **References**:

  **Pattern References**:
  - `src/formatter/printer.rs:541-551` - Current Stmt::Block implementation to refactor
  - `src/formatter/printer.rs:651-674` - Stmt::Guard shows indented statement printing pattern
  - `src/formatter/printer.rs:533-537` - print_stmt signature to follow for helper

  **Context References** (critical for avoiding the trap):
  - `src/formatter/printer.rs:Decl::Function` - Shows `=> {body_str}` pattern; must NOT become `=> :`
  - `src/formatter/tests.rs:test_formatter_comments_reparse_clean` - Shows format+reparse test pattern

  **External References**:
  - `language-spec/fib_iterative.op:6-9` - Shows block body without braces

  **WHY Each Reference Matters**:
  - Current implementation shows exact code to change
  - Guard statement shows how to output statements without braces
  - Decl::Function shows WHY we can't bake `:` into Block (would break function printing)

  **Acceptance Criteria**:
  - [ ] Helper method `print_block_body_indented` exists in printer.rs
  - [ ] `Stmt::Block` match arm UNCHANGED (still outputs braces)
  - [ ] Control flow (If/While/For/Loop) now call helper instead of print_stmt for body
  - [ ] Empty blocks output `# empty` comment, not `pass`
  - [ ] Function bodies still output `=> { ... }` correctly

  **QA Scenarios**:

  ```
  Scenario: Block helper works and control flow uses colon syntax
    Tool: Bash
    Preconditions: Task 2 complete, formatter changes applied
    Steps:
      1. Run: cargo test test_spec_compliance_nested_control_flow_no_braces --package opalescent 2>&1
      2. Check exit code is 0 and output contains "ok"
    Expected Result: Test passes, confirming control flow uses colon syntax
    Failure Indicators: Exit code non-zero or output shows "FAILED"
    Evidence: .sisyphus/evidence/task-2-no-braces.txt

  Scenario: Function bodies still use brace syntax (unchanged)
    Tool: Bash
    Preconditions: Task 2 complete
    Steps:
      1. Run: cargo test test_spec_compliance_function_body_arrow_syntax --package opalescent 2>&1
      2. Check exit code is 0 and output contains "ok"
    Expected Result: Function bodies still use `=> { ... }` (only control flow uses colon)
    Failure Indicators: Exit code non-zero or output shows "FAILED"
    Evidence: .sisyphus/evidence/task-2-function-braces.txt
  ```

  **Commit**: NO (groups with Task 6)

- [x] 3. Fix Stmt::If to use colon-block syntax

  **What to do**:
  - In `src/formatter/printer.rs`, find `print_stmt` method, `Stmt::If` match arm (lines 619-631)
  - Change from `if {cond} {then_str}` to `if {cond}:\n{indented_body}`
  - For else branch, output `else:` followed by indented body, NOT `else {`
  - Handle single-line if (inline body like `if pred(x): left.push(x) else: right.push(x)`)

  **Must NOT do**:
  - Do not change Expr::If (expression-level if, different from statement)
  - Do not break else-if chaining

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2 (with Tasks 2, 4, 5, 6)
  - **Blocks**: Task 7
  - **Blocked By**: Task 1

  **References**:

  **Pattern References**:
  - `src/formatter/printer.rs:619-631` - Current Stmt::If to modify
  - `language-spec/fib_recursive.op:6-9` - Shows `if n is 0:` + indented return
  - `language-spec/partition.op:8` - Shows inline `if pred(x): left.push(x) else: right.push(x)`

  **WHY Each Reference Matters**:
  - Current code shows exact structure to change
  - Language spec shows both multi-line and inline if syntax

  **Acceptance Criteria**:
  - [ ] `test_spec_compliance_if_no_braces` passes
  - [ ] If statements output `if condition:` not `if condition {`
  - [ ] Else branches output `else:` not `else {`

  **QA Scenarios**:

  ```
  Scenario: If statement uses colon syntax
    Tool: Bash
    Preconditions: Task 3 complete
    Steps:
      1. Run: cargo test test_spec_compliance_if_no_braces --package opalescent 2>&1
      2. Check for "ok" not "FAILED"
    Expected Result: Test passes
    Evidence: .sisyphus/evidence/task-3-if-colon.txt
  ```

  **Commit**: NO (groups with Task 6)

- [x] 4. Fix Stmt::While to use colon-block syntax

  **What to do**:
  - In `src/formatter/printer.rs`, find `print_stmt` method, `Stmt::While` match arm (lines 642-649)
  - Change from `while {cond} {body_str}` to `while {cond}:\n{indented_body}`
  - Body statements should be at depth+1 indentation

  **Must NOT do**:
  - Do not change loop or for statement handling

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2 (with Tasks 2, 3, 5, 6)
  - **Blocks**: Task 7
  - **Blocked By**: Task 1

  **References**:

  **Pattern References**:
  - `src/formatter/printer.rs:642-649` - Current Stmt::While to modify
  - `language-spec/fib_iterative.op:16-20` - Shows `while i <= n:` + indented body
  - `language-spec/array_helpers.op:61-63` - Shows `while i < end_exclusive:` pattern

  **WHY Each Reference Matters**:
  - Current code shows structure to change
  - Language spec shows expected output format

  **Acceptance Criteria**:
  - [ ] `test_spec_compliance_while_no_braces` passes
  - [ ] While loops output `while condition:` not `while condition {`

  **QA Scenarios**:

  ```
  Scenario: While loop uses colon syntax
    Tool: Bash
    Preconditions: Task 4 complete
    Steps:
      1. Run: cargo test test_spec_compliance_while_no_braces --package opalescent 2>&1
      2. Check for "ok" not "FAILED"
    Expected Result: Test passes
    Evidence: .sisyphus/evidence/task-4-while-colon.txt
  ```

  **Commit**: NO (groups with Task 6)

- [x] 5. Fix Stmt::For to use colon-block syntax

  **What to do**:
  - In `src/formatter/printer.rs`, find `print_stmt` method, `Stmt::For` match arm (lines 632-641)
  - Change from `for {var} in {iter} {body_str}` to `for {var} in {iter}:\n{indented_body}`
  - Body statements should be at depth+1 indentation

  **Must NOT do**:
  - Do not change while or loop statement handling

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2 (with Tasks 2, 3, 4, 6)
  - **Blocks**: Task 7
  - **Blocked By**: Task 1

  **References**:

  **Pattern References**:
  - `src/formatter/printer.rs:632-641` - Current Stmt::For to modify
  - `language-spec/array_helpers.op:29-30` - Shows `for x in xs:` + indented body
  - `language-spec/partition.op:7-8` - Shows `for x in xs:` inline body

  **WHY Each Reference Matters**:
  - Current code shows structure to change
  - Language spec shows both multi-line and inline for syntax

  **Acceptance Criteria**:
  - [ ] `test_spec_compliance_for_no_braces` passes
  - [ ] For loops output `for var in iter:` not `for var in iter {`

  **QA Scenarios**:

  ```
  Scenario: For loop uses colon syntax
    Tool: Bash
    Preconditions: Task 5 complete
    Steps:
      1. Run: cargo test test_spec_compliance_for_no_braces --package opalescent 2>&1
      2. Check for "ok" not "FAILED"
    Expected Result: Test passes
    Evidence: .sisyphus/evidence/task-5-for-colon.txt
  ```

  **Commit**: NO (groups with Task 6)

- [x] 6. Fix Stmt::Loop to use arrow syntax (not braces)

  **What to do**:
  - In `src/formatter/printer.rs`, find `print_stmt` method, `Stmt::Loop` match arm (lines 676-679)
  - Change from `loop {body_str}` to `loop =>\n{indented_body}`
  - Body statements should be at depth+1 indentation
  - Note: Expr::Loop at line 851-854 may also need fixing

  **Must NOT do**:
  - Do not change while or for statement handling

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2 (with Tasks 2, 3, 4, 5)
  - **Blocks**: Task 7
  - **Blocked By**: Task 1

  **References**:

  **Pattern References**:
  - `src/formatter/printer.rs:676-679` - Current Stmt::Loop to modify
  - `src/formatter/printer.rs:851-854` - Expr::Loop (may need similar fix)
  - `language-spec/simple_quiz.op:25-50` - Shows `loop =>` + indented body with break

  **WHY Each Reference Matters**:
  - Current code shows structure to change
  - Language spec shows loop uses `=>` not `:` (different from if/while/for)

  **Acceptance Criteria**:
  - [ ] `test_spec_compliance_loop_no_braces` passes
  - [ ] Loop statements output `loop =>` not `loop {`

  **QA Scenarios**:

  ```
  Scenario: Loop uses arrow syntax
    Tool: Bash
    Preconditions: Task 6 complete
    Steps:
      1. Run: cargo test test_spec_compliance_loop_no_braces --package opalescent 2>&1
      2. Check for "ok" not "FAILED"
    Expected Result: Test passes
    Evidence: .sisyphus/evidence/task-6-loop-arrow.txt
  ```

  **Commit**: YES (includes Tasks 2-6)
  - Message: `fix(formatter): use colon-block syntax per language spec`
  - Files: `src/formatter/printer.rs`
  - Pre-commit: `cargo test test_spec_compliance --package opalescent`

- [x] 7. Update existing tests expecting brace syntax to expect colon syntax

  **What to do**:
  - Find tests in `src/formatter/tests.rs` that have `expected` strings with braces
  - Update these tests:
    - `test_formatter_loop_body_leading_comment` (line 1059) - change expected from `loop {` to `loop =>`
    - `test_formatter_for_body_leading_comment` (line 1082) - change expected from `for x in items {` to `for x in items:`
    - `test_formatter_while_body_leading_comment` (line 1105) - change expected from `while cond {` to `while cond:`
    - `test_formatter_if_body_leading_comment` (line 1128) - change expected from `if cond {` to `if cond:`
    - `test_formatter_loop_body_leading_doc_comment` (line 1173) - change expected from `loop {` to `loop =>`
  - Also update function body tests if they expect `=> {` to just `=>`
  - Remove closing braces from expected strings

  **Must NOT do**:
  - Do not change match expression tests (match still uses braces)
  - Do not change the source input strings (only expected output)

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: NO
  - **Parallel Group**: Wave 3 (sequential)
  - **Blocks**: Task 8
  - **Blocked By**: Tasks 2, 3, 4, 5, 6

  **References**:

  **Pattern References**:
  - `src/formatter/tests.rs:1059-1079` - test_formatter_loop_body_leading_comment
  - `src/formatter/tests.rs:1082-1102` - test_formatter_for_body_leading_comment
  - `src/formatter/tests.rs:1105-1125` - test_formatter_while_body_leading_comment
  - `src/formatter/tests.rs:1128-1148` - test_formatter_if_body_leading_comment
  - `src/formatter/tests.rs:1173-1193` - test_formatter_loop_body_leading_doc_comment

  **WHY Each Reference Matters**:
  - These are the exact tests that need their expected strings updated

  **Acceptance Criteria**:
  - [ ] All 5+ updated tests pass
  - [ ] No test expects `{` or `}` in control flow output (except match)
  - [ ] `cargo test formatter_tests --package opalescent` passes

  **QA Scenarios**:

  ```
  Scenario: All formatter tests pass
    Tool: Bash
    Preconditions: Tasks 1-7 complete
    Steps:
      1. Run: cargo test formatter_tests --package opalescent 2>&1
      2. Verify output ends with "test result: ok"
      3. Count any FAILED tests
    Expected Result: 0 failed tests, all pass
    Failure Indicators: Any test shows "FAILED"
    Evidence: .sisyphus/evidence/task-7-all-tests-pass.txt
  ```

  **Commit**: YES
  - Message: `test(formatter): update tests to expect colon-block syntax`
  - Files: `src/formatter/tests.rs`
  - Pre-commit: `cargo test formatter_tests --package opalescent`

- [x] 8. Add documentation comments, spec file reparse test, and final verification

  **What to do**:
  - Add doc comment at top of `print_stmt` method in `printer.rs` noting language spec compliance:
    ```rust
    /// Pretty-print a statement at the given indent `depth`.
    ///
    /// # Language Spec Compliance
    /// 
    /// This method outputs control flow statements using Opalescent's colon-block
    /// syntax per the language specification:
    /// - `if condition:` followed by indented body (no braces)
    /// - `while condition:` followed by indented body (no braces)  
    /// - `for var in iter:` followed by indented body (no braces)
    /// - `loop =>` followed by indented body (no braces)
    ///
    /// See `language-spec/*.op` for canonical examples.
    ```
  - Add inline comment on each fixed match arm: `// Per language spec: colon-block syntax`
  - Add `test_spec_files_format_and_reparse` test in `src/formatter/tests.rs`:
    ```rust
    #[test]
    fn test_spec_files_format_and_reparse() {
        // REGRESSION TEST: Ensures formatted output is valid Opalescent syntax
        // per language spec. Formatted code must lex and parse without errors.
        let spec_files = [
            include_str!("../../language-spec/fib_iterative.op"),
            include_str!("../../language-spec/fib_recursive.op"),
            include_str!("../../language-spec/array_helpers.op"),
            // Add other spec files as available
        ];
        for source in spec_files {
            let formatted = Formatter::with_defaults()
                .format_source(source)
                .expect("formatter should succeed");
            let lexer = crate::lexer::Lexer::new(&formatted);
            let (tokens, lex_errors) = lexer.tokenize();
            assert!(
                lex_errors.errors.is_empty(),
                "formatted output should lex without errors: {lex_errors:?}"
            );
            let parser = crate::parser::Parser::new(tokens);
            let (_program, parse_errors) = parser.parse();
            assert!(
                parse_errors.errors.is_empty(),
                "formatted output should parse without errors: {parse_errors:?}"
            );
        }
    }
    ```
  - Run full test suite to verify no regressions

  **Must NOT do**:
  - Do not add excessive comments
  - Do not change any logic

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: NO
  - **Parallel Group**: Wave 3 (after Task 7)
  - **Blocks**: None (final task)
  - **Blocked By**: Task 7

  **References**:

  **Pattern References**:
  - `src/formatter/printer.rs:533-537` - Existing doc comment on print_stmt to extend
  - `src/formatter/printer.rs:1-25` - Module-level documentation pattern
  - `src/formatter/tests.rs:test_formatter_comments_reparse_clean` - Format+reparse test pattern to follow
  - `src/type_system/test_integration_ecosystem.rs` - Shows include_str! pattern for loading spec files

  **Acceptance Criteria**:
  - [ ] Doc comments added to `print_stmt` about language spec
  - [ ] Inline comments on each control flow match arm
  - [ ] `test_spec_files_format_and_reparse` test added and passes
  - [ ] `cargo test --package opalescent` passes (all tests)
  - [ ] `cargo clippy --package opalescent` passes

  **QA Scenarios**:

  ```
  Scenario: Full test suite passes
    Tool: Bash
    Preconditions: All previous tasks complete
    Steps:
      1. Run: cargo test --package opalescent 2>&1
      2. Capture exit code
      3. Verify "test result: ok" in output
    Expected Result: Exit code 0, all tests pass
    Failure Indicators: Non-zero exit code or any FAILED tests
    Evidence: .sisyphus/evidence/task-8-full-suite.txt

  Scenario: Clippy passes
    Tool: Bash
    Preconditions: All previous tasks complete
    Steps:
      1. Run: cargo clippy --package opalescent -- -D warnings 2>&1
      2. Verify no warnings or errors
    Expected Result: Clean clippy output
    Evidence: .sisyphus/evidence/task-8-clippy.txt

  Scenario: Language spec examples format and re-parse
    Tool: Bash
    Preconditions: All previous tasks complete
    Steps:
      1. Run: cargo test test_spec_compliance_formatted_output_parses_cleanly --package opalescent 2>&1
      2. Check output contains "ok" (not "FAILED")
      3. Verify exit code is 0
    Expected Result: The re-parse regression test passes, confirming formatted output is valid Opalescent syntax
    Failure Indicators: Test shows "FAILED" or exit code non-zero
    Evidence: .sisyphus/evidence/task-8-spec-examples.txt
  ```

  **Commit**: YES
  - Message: `docs(formatter): add language spec compliance documentation`
  - Files: `src/formatter/printer.rs`
  - Pre-commit: `cargo test --package opalescent && cargo clippy --package opalescent`

---

## Final Verification Wave

> After ALL implementation tasks, run these 4 reviews in PARALLEL. All must APPROVE.

- [x] F1. **Plan Compliance Audit** — `oracle`
  Read the plan end-to-end. Verify: all regression tests exist and pass, all control flow uses colon/arrow syntax, no braces in if/while/for/loop output. Check evidence files in `.sisyphus/evidence/`.
  Output: `Must Have [N/N] | Must NOT Have [N/N] | Tasks [8/8] | VERDICT: APPROVE/REJECT`

- [x] F2. **Code Quality Review** — `unspecified-high`
  Run `cargo test`, `cargo clippy`. Review `src/formatter/printer.rs` changes for: proper indentation handling, no regressions in other statement types, clean code.
  Output: `Build [PASS/FAIL] | Tests [N/N pass] | Clippy [PASS/FAIL] | VERDICT`

- [x] F3. **Real Manual QA** — `unspecified-high`
  Test formatter on all `language-spec/*.op` files. Verify output matches expected colon-block syntax. Test edge cases: empty blocks, nested control flow, inline if/for.
  Output: `Spec Files [N/N pass] | Edge Cases [N tested] | VERDICT`

- [x] F4. **Scope Fidelity Check** — `deep`
  Verify only `printer.rs` and `tests.rs` were modified. Ensure match expression formatting unchanged. Verify no unintended changes to other formatter components.
  Output: `Files [2/2 expected] | Match syntax [unchanged] | VERDICT`

---

## Commit Strategy

| Task | Commit | Message | Files |
|------|--------|---------|-------|
| 1 | YES | `test(formatter): add regression tests for language spec compliance` | `src/formatter/tests.rs` |
| 2-6 | YES | `fix(formatter): use colon-block syntax per language spec` | `src/formatter/printer.rs` |
| 7 | YES | `test(formatter): update tests to expect colon-block syntax` | `src/formatter/tests.rs` |
| 8 | YES | `docs(formatter): add language spec compliance documentation` | `src/formatter/printer.rs` |

---

## Success Criteria

### Verification Commands
```bash
# All tests pass
cargo test --package opalescent
# Expected: test result: ok

# Regression tests specifically pass
cargo test test_spec_compliance --package opalescent
# Expected: 8+ tests pass

# No clippy warnings
cargo clippy --package opalescent -- -D warnings
# Expected: no output (clean)
```

### Final Checklist
- [ ] 8+ regression tests exist and pass
- [ ] No `if ... {`, `while ... {`, `for ... {`, `loop {` in formatter output
- [ ] All control flow uses colon-block or arrow syntax
- [ ] No semicolons in formatted control flow output
- [ ] Formatted output re-parses without errors
- [ ] Match expressions still use brace syntax (unchanged)
- [ ] All existing tests pass
- [ ] Documentation comments added
