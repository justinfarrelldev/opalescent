# Operator Accuracy Verification & Comprehensive Test Suite

## TL;DR

> **Quick Summary**: Verify all operators (comparison, bitwise, logical, arithmetic) are correctly implemented across the lexer→parser→AST pipeline, then add comprehensive tests covering every operator, all 12 pairwise precedence relationships (Assignment excluded — statement-level only), associativity rules, and edge cases.
> 
> **Deliverables**:
> - Comprehensive operator tests added to `src/parser/tests.rs` (~30+ new test functions)
> - Code review documenting operator pipeline correctness
> - Single commit with all changes
> 
> **Estimated Effort**: Short
> **Parallel Execution**: NO - sequential (single file + review + commit)
> **Critical Path**: Task 1 (tests) → Task 2 (code review) → Task 3 (commit)

---

## Context

### Original Request
Double-check all operators for accuracy (especially `>=` and `<=`, as well as bitwise). Create intense, extensive tests for them if they are not already in place. Tests should validate order-of-operations with many cases. When done, do a code review and then commit.

### Interview Summary
**Key Discussions**:
- Operator audit performed: All operators are correctly implemented (no bugs found)
- Lexer correctly tokenizes `<=` and `>=` as 2-char tokens (`src/lexer.rs:260-276`)
- Precedence table is correct with 14 levels from Assignment to Primary
- Power (`^`) is correctly right-associative, all others left-associative
- Existing test coverage is MINIMAL: only `+`, `<`, `and`, `-` (unary), `not`, and `1 + 2 * 3` precedence

**Research Findings**:
- `Equal`/`NotEqual` variants in `BinaryOp` are dead code (never constructed by parser) — flag only, do not fix
- `<` has special-case handling for generic calls (`foo<T>(x)`) but falls back correctly to comparison
- Bitwise ops are keyword-based (`band`, `bor`, etc.), not symbol-based
- `is not` is a two-keyword operator requiring special tokenization

### Metis Review
**Identified Gaps** (addressed):
- Missing tests for 15+ binary operators and 2 unary operators
- Zero precedence chain tests beyond `+ vs *`
- Zero associativity tests
- No edge case tests for `<=` vs `<` + `=`, `is not` vs `not` + `is`, etc.
- Dead `Equal`/`NotEqual` variants should be flagged in review but NOT fixed (scope boundary)

---

## Work Objectives

### Core Objective
Add a comprehensive, exhaustive test suite for ALL operators and their precedence relationships, validating the parser produces correct AST structure for every operator and order-of-operations scenario.

### Concrete Deliverables
- ~30+ new test functions in `src/parser/tests.rs`
- Code review findings documented in commit message

### Definition of Done
- [ ] `cargo test --lib parser::tests` passes with 0 failures
- [ ] `cargo clippy --all-targets -- -D warnings` produces 0 warnings
- [ ] All 21 binary operators have at least one parse test
- [ ] All 4 unary operators have at least one parse test
- [ ] At least 12 pairwise precedence relationships tested (Assignment excluded — not an expression operator)
- [ ] Power right-associativity tested
- [ ] Left-associativity tested for at least one representative
- [ ] Edge cases for `<=`, `>=`, `is not`, `bnot` tested

### Must Have
- Individual parse tests for every binary operator not currently tested
- Individual parse tests for every unary operator not currently tested
- Pairwise precedence tests for all adjacent levels in the precedence table
- Multi-level precedence chain tests
- Right-associativity test for power
- Left-associativity tests
- Unary vs binary interaction tests
- Edge case tests for multi-char/multi-keyword operators

### Must NOT Have (Guardrails)
- DO NOT modify any existing code outside `src/parser/tests.rs`
- DO NOT fix the `Equal`/`NotEqual` dead variants — only flag in review
- DO NOT add tests for semantic correctness (runtime evaluation) — parser/AST level only
- DO NOT touch type_system, codegen, formatter, or lexer code
- DO NOT add new dependencies or modify `Cargo.toml`
- DO NOT modify existing tests — only ADD new tests
- DO NOT add unnecessary comments, docstrings beyond what's required by clippy
- DO NOT create helper abstractions that obscure what each test validates

---

## Verification Strategy (MANDATORY)

> **ZERO HUMAN INTERVENTION** - ALL verification is agent-executed. No exceptions.

### Test Decision
- **Infrastructure exists**: YES
- **Automated tests**: Tests-after (this task IS writing the tests)
- **Framework**: Rust built-in `#[test]` + existing proptest

### QA Policy
Every task MUST include agent-executed QA scenarios.
Evidence saved to `.sisyphus/evidence/task-{N}-{scenario-slug}.{ext}`.

- **Tests**: Use Bash - Run `cargo test`, capture output, verify pass counts

---

## Execution Strategy

### Parallel Execution Waves

```
Wave 1 (Sequential — single concern):
├── Task 1: Add comprehensive operator & precedence tests [deep]

Wave 2 (After Wave 1):
├── Task 2: Code review of operator pipeline [deep]

Wave 3 (After Wave 2):
├── Task 3: Commit all changes [quick]

Wave FINAL (After ALL tasks — reviews):
├── Task F1: Plan compliance audit (oracle)
├── Task F2: Code quality review (unspecified-high)
├── Task F3: Real manual QA (unspecified-high)
├── Task F4: Scope fidelity check (deep)
-> Present results -> Get explicit user okay

Critical Path: Task 1 → Task 2 → Task 3 → F1-F4 → user okay
```

### Dependency Matrix

| Task | Depends On | Blocks | Wave |
|------|-----------|--------|------|
| 1 | - | 2, 3 | 1 |
| 2 | 1 | 3 | 2 |
| 3 | 1, 2 | F1-F4 | 3 |

### Agent Dispatch Summary

- **1**: **1** - T1 → `deep`
- **2**: **1** - T2 → `deep`
- **3**: **1** - T3 → `quick` (git-master skill)
- **FINAL**: **4** - F1 → `oracle`, F2 → `unspecified-high`, F3 → `unspecified-high`, F4 → `deep`

---

## TODOs

- [x] 1. Add comprehensive operator parsing tests (individual operators)

  **What to do**:
  - Add individual parse tests for every binary operator not currently tested. Each test should call `parse_expression_from_string` and verify the resulting `Expr::Binary` has the correct `BinaryOp` variant. Missing operators to test:
    - Arithmetic: `Subtract` (`a - b`), `Divide` (`a / b`), `Modulo` (`a % b`), `Power` (`a ^ b`)
    - Comparison: `LessEqual` (`a <= b`), `Greater` (`a > b`), `GreaterEqual` (`a >= b`)
    - Equality: `Is` (`a is b`), `IsNot` (`a is not b`)
    - Logical: `Or` (`a or b`), `Xor` (`a xor b`)
    - Bitwise: `BitAnd` (`a band b`), `BitOr` (`a bor b`), `BitXor` (`a bxor b`), `BitShiftLeft` (`a bshl b`), `BitShiftRight` (`a bshr b`), `BitUnsignedShiftRight` (`a bushr b`)
  - Add individual parse tests for missing unary operators:
    - `Plus` (`+x`), `BitNot` (`bnot x`)
  - Each test function should verify both the operator AND the operands (left/right for binary, operand for unary)
  - Use descriptive test function names: `test_binary_op_subtract`, `test_binary_op_less_equal`, `test_unary_op_bit_not`, etc.

  **Must NOT do**:
  - Do NOT modify existing tests (`test_binary_expressions`, `test_unary_expressions`, etc.)
  - Do NOT add runtime/semantic evaluation tests
  - Do NOT touch any file other than `src/parser/tests.rs`

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: Requires understanding the parser's AST structure and writing many precise, correct test assertions
  - **Skills**: `[]`
    - No special skills needed — standard Rust testing

  **Parallelization**:
  - **Can Run In Parallel**: NO
  - **Parallel Group**: Wave 1 (sole task)
  - **Blocks**: Tasks 2, 3
  - **Blocked By**: None

  **References**:

  **Pattern References** (existing code to follow):
  - `src/parser/tests.rs:746-774` — `test_binary_expressions`: Shows the pattern for parsing an expression string and matching on `Expr::Binary { operator: BinaryOp::X, .. }`. Follow this exact style.
  - `src/parser/tests.rs:776-795` — `test_unary_expressions`: Shows the pattern for unary op tests using `Expr::Unary { operator: UnaryOp::X, .. }`.
  - `src/parser/tests.rs:26-30` — `parse_expression_from_string` helper function: Use this for all expression parsing in tests.

  **API/Type References**:
  - `src/ast/operators.rs:10-69` — `BinaryOp` enum: All 21 binary operator variants with doc comments
  - `src/ast/operators.rs:72-82` — `UnaryOp` enum: All 4 unary operator variants
  - `src/ast/operators.rs:126-156` — `TryFrom<TokenType> for BinaryOp`: Shows which `TokenType` maps to which `BinaryOp`

  **WHY Each Reference Matters**:
  - `test_binary_expressions` (line 746): Copy this exact assertion style — `matches!` macro with `Expr::Binary { operator: BinaryOp::X, .. }`
  - `BinaryOp` enum (line 10): Need to know exact variant names to write assertions
  - `parse_expression_from_string` (line 26): The test entry point — takes a string, returns `ParseResult<Expr>`

  **Acceptance Criteria**:

  **QA Scenarios (MANDATORY):**

  ```
  Scenario: All new operator parse tests pass
    Tool: Bash
    Preconditions: Working directory is project root, `cargo build` succeeds
    Steps:
      1. Run `cargo test --lib parser::tests::test_binary_op_ -- --nocapture 2>&1`
      2. Run `cargo test --lib parser::tests::test_unary_op_ -- --nocapture 2>&1`
      3. Count test results: grep for "test result:" lines
    Expected Result: All tests pass with 0 failures. At minimum 18 new binary op tests + 2 new unary op tests = 20 new tests
    Failure Indicators: Any line containing "FAILED" or "panicked"
    Evidence: .sisyphus/evidence/task-1-operator-parse-tests.txt

  Scenario: No existing tests broken
    Tool: Bash
    Preconditions: Same as above
    Steps:
      1. Run `cargo test --lib parser::tests 2>&1`
      2. Grep for "test result:" and verify 0 failures
      3. Verify test count is >= previous count (was ~135 parser tests)
    Expected Result: 0 failures, test count >= 155
    Failure Indicators: Any failure count > 0
    Evidence: .sisyphus/evidence/task-1-no-regressions.txt

  Scenario: Clippy passes on new code
    Tool: Bash
    Preconditions: Same
    Steps:
      1. Run `cargo clippy --all-targets -- -D warnings 2>&1`
    Expected Result: Exit code 0, no warnings
    Failure Indicators: Any warning or error output
    Evidence: .sisyphus/evidence/task-1-clippy.txt
  ```

  **Commit**: NO (groups with Task 3)

- [x] 2. Add comprehensive precedence and order-of-operations tests

  **What to do**:
  - Add pairwise precedence tests for ALL 12 adjacent precedence level pairs (Assignment excluded — it is not parsed as an expression operator; `TokenType::Assign` is not mapped in `Precedence::from_token` and is handled at the statement level only). Each test parses an expression with two operators from adjacent levels (no parentheses) and verifies the OUTER operator is the lower-precedence one and the INNER (nested) operator is the higher-precedence one. Tests to add:
    - `or` vs `xor`: `a or b xor c` → outer `Or`, right child is `Xor`
    - `xor` vs `and`: `a xor b and c` → outer `Xor`, right child is `And`
    - `and` vs `bor`: `a and b bor c` → outer `And`, right child is `BitOr`
    - `bor` vs `bxor`: `a bor b bxor c` → outer `BitOr`, right child is `BitXor`
    - `bxor` vs `band`: `a bxor b band c` → outer `BitXor`, right child is `BitAnd`
    - `band` vs `is`: `a band 1 is b band 2` → outer `Is`, both children are `BitAnd`
    - `is` vs `<`: `a < b is c < d` → outer `Is`, both children are `Less`
    - `<` vs `bshl`: `a < b bshl c` → outer `Less`, right child is `BitShiftLeft`
    - `bshl` vs `+`: `a bshl b + c` → outer `BitShiftLeft`, right child is `Add`
    - `+` vs `*`: `a + b * c` → outer `Add`, right child is `Multiply` (extend existing)
    - `*` vs `^`: `a * b ^ c` → outer `Multiply`, right child is `Power`
    - `^` vs unary: `-a ^ b` → outer `Power`, left child is unary `Negate` (unary binds tighter)
  - Add right-associativity test for power:
    - `2 ^ 3 ^ 4` → outer `Power`, LEFT is `2`, RIGHT is `Power(3, 4)`
  - Add left-associativity tests:
    - `1 - 2 - 3` → outer `Subtract`, LEFT is `Subtract(1, 2)`, RIGHT is `3`
    - `a and b and c` → outer `And`, LEFT is `And(a, b)`, RIGHT is `c`
  - Add multi-level precedence chain tests:
    - `1 + 2 * 3 ^ 4` → `1 + (2 * (3 ^ 4))` — outer `Add`, right is `Multiply`, right-right is `Power`
    - `a or b and c < d + e * f` → deeply nested chain exercising 6 precedence levels
    - `a bor b bxor c band d bshl e + f * g` → bitwise chain exercising all bitwise + arithmetic levels
  - Add parenthesized override tests:
    - `(a + b) * c` → outer `Multiply`, left is `Parenthesized` containing `Add`
    - `(a or b) and c` → outer `And`, left is `Parenthesized` containing `Or`
  - Add mixed comparison/logical tests:
    - `a < b and c > d` → outer `And`, left is `Less`, right is `Greater`
    - `a <= b or c >= d` → outer `Or`, left is `LessEqual`, right is `GreaterEqual`
  - Add unary interaction tests:
    - `not a and b` → outer `And`, left is unary `Not` applied to `a`
    - `bnot x bor y` → outer `BitOr`, left is unary `BitNot` applied to `x`
    - `-a + b` → outer `Add`, left is unary `Negate` applied to `a`
  - Add edge case tests:
    - `a <= b` parses as single `LessEqual`, NOT `Less` then `Assign` — verify `Expr::Binary { operator: BinaryOp::LessEqual, .. }`
    - `a >= b` parses as single `GreaterEqual` — same verification
    - `a is not b` parses as single `IsNot` — verify `Expr::Binary { operator: BinaryOp::IsNot, .. }`
    - `not a is not b` — verify correct parse tree (unary `Not` on `a`, then `IsNot` with `b`)

  **Must NOT do**:
  - Do NOT modify existing `test_operator_precedence` test
  - Do NOT add runtime evaluation tests
  - Do NOT touch any file other than `src/parser/tests.rs`
  - Do NOT create abstract test helpers that hide what's being tested — each test should be self-contained and readable

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: Complex AST structure verification requiring deep understanding of Pratt parser behavior and recursive tree matching
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: NO
  - **Parallel Group**: Wave 1 (with Task 1 — same file, same agent)
  - **Blocks**: Tasks 3
  - **Blocked By**: Task 1 (same file, must be coordinated — run by same agent)

  **References**:

  **Pattern References**:
  - `src/parser/tests.rs:809-837` — `test_operator_precedence`: Shows the pattern for precedence testing — parse `1 + 2 * 3`, match outer as `Add`, verify right child is `Multiply`. Copy this exact pattern for all new precedence tests.
  - `src/parser/tests.rs:797-801` — `test_parenthesized_expressions`: Shows matching on `Expr::Parenthesized`

  **API/Type References**:
  - `src/parser/precedence.rs:9-28` — `Precedence` enum: The authoritative precedence order. Use this to verify which operator should be outer vs inner in each test.
  - `src/parser/precedence.rs:33-56` — `Precedence::from_token`: Maps each `TokenType` to its precedence level.
  - `src/parser/precedence.rs:60-77` — `Precedence::next`: The adjacency chain — each `next()` gives the immediately higher precedence level.
  - `src/parser/expressions.rs:762-768` — Right-associativity logic: Power uses `precedence` (same level) for right side, others use `precedence.next()` (higher level). This is what makes `2 ^ 3 ^ 4` parse as `2 ^ (3 ^ 4)`.

  **WHY Each Reference Matters**:
  - `test_operator_precedence` (line 809): Exact pattern to copy — `if let Expr::Binary { left, operator: BinaryOp::X, right, .. }` then assert structure of left/right
  - `Precedence` enum (line 9): Need to know exact ordering to write correct expected outcomes
  - Right-associativity code (line 762): Confirms `^` is the ONLY right-associative binary operator

  **Acceptance Criteria**:

  **QA Scenarios (MANDATORY):**

  ```
  Scenario: All precedence tests pass
    Tool: Bash
    Preconditions: Task 1 completed, working directory is project root
    Steps:
      1. Run `cargo test --lib parser::tests::test_precedence_ -- --nocapture 2>&1`
      2. Run `cargo test --lib parser::tests::test_associativity_ -- --nocapture 2>&1`
      3. Run `cargo test --lib parser::tests::test_edge_case_ -- --nocapture 2>&1`
      4. Count passing tests
    Expected Result: At least 12 pairwise precedence + 3 associativity + 5 multi-level + 3 override + 3 mixed + 3 unary interaction + 4 edge case = ~33 tests, all passing
    Failure Indicators: Any "FAILED" or "panicked"
    Evidence: .sisyphus/evidence/task-2-precedence-tests.txt

  Scenario: Full test suite passes with no regressions
    Tool: Bash
    Preconditions: All new tests added
    Steps:
      1. Run `cargo test --lib parser::tests 2>&1`
      2. Verify "test result: ok" and 0 failures
    Expected Result: All tests pass, total count >= 185 (was ~135 + ~50 new)
    Failure Indicators: failures > 0
    Evidence: .sisyphus/evidence/task-2-full-suite.txt
  ```

  **Commit**: NO (groups with Task 3)

- [x] 3. Code review and commit

  **What to do**:
  - Review the entire operator pipeline for correctness:
    - `src/lexer.rs:260-276` — Verify `<=` and `>=` tokenization is correct (greedy 2-char matching)
    - `src/ast/operators.rs:126-156` — Verify all `TryFrom<TokenType>` conversions are bidirectionally correct
    - `src/parser/precedence.rs:33-56` — Verify `from_token` maps all operators to correct levels
    - `src/parser/precedence.rs:60-77` — Verify `next()` chain has no gaps or skips
    - `src/parser/expressions.rs:703-730` — Verify `<` special-case for generics doesn't swallow comparisons
    - `src/parser/expressions.rs:762-768` — Verify only Power is right-associative
  - Flag `Equal`/`NotEqual` dead variants in `BinaryOp` as a future cleanup candidate (note in commit message, do NOT fix)
  - Run final verification: `cargo test --lib && cargo clippy --all-targets -- -D warnings`
  - Create a single commit with descriptive message including review findings
  - Commit message format: `test(parser): add comprehensive operator and precedence tests`
  - Include a note in the commit body about the `Equal`/`NotEqual` dead code observation

  **Must NOT do**:
  - Do NOT fix the `Equal`/`NotEqual` dead variants
  - Do NOT modify any source code — only `src/parser/tests.rs` should be in the commit
  - Do NOT use `--no-verify` flag

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Straightforward review + git commit
  - **Skills**: `['git-master']`
    - `git-master`: Needed for proper commit formatting and git operations

  **Parallelization**:
  - **Can Run In Parallel**: NO
  - **Parallel Group**: Sequential (Wave 3)
  - **Blocks**: F1-F4
  - **Blocked By**: Tasks 1, 2

  **References**:

  **Pattern References**:
  - `src/lexer.rs:260-276` — `<=` and `>=` tokenization: Verify greedy 2-char matching (peek for `=` before emitting single-char token)
  - `src/ast/operators.rs:9-69` — Full `BinaryOp` enum: Check for dead variants (`Equal`, `NotEqual` — lines 28-30)
  - `src/ast/operators.rs:126-156` — `TryFrom<TokenType> for BinaryOp`: Verify every `TokenType` operator variant has a mapping
  - `src/parser/precedence.rs:9-77` — Full precedence module: Verify ordering, `from_token` completeness, and `next()` chain integrity

  **WHY Each Reference Matters**:
  - Lexer tokenization: Core correctness for `<=`/`>=` — must verify `=` is consumed as part of the compound token
  - `BinaryOp` dead variants: Metis identified this as a code smell to flag (not fix)
  - Precedence chain: Any gap in `next()` would cause incorrect parsing of adjacent-level operators

  **Acceptance Criteria**:

  **QA Scenarios (MANDATORY):**

  ```
  Scenario: Commit created successfully
    Tool: Bash
    Preconditions: Tasks 1 and 2 completed, all tests pass
    Steps:
      1. Run `cargo test --lib 2>&1` — verify pass
      2. Run `cargo clippy --all-targets -- -D warnings 2>&1` — verify clean
      3. Run `git add src/parser/tests.rs`
      4. Run `git commit -m "test(parser): add comprehensive operator and precedence tests" -m "..." `
      5. Run `git log -1 --oneline` — verify commit exists
    Expected Result: Commit created with proper message, only `src/parser/tests.rs` modified
    Failure Indicators: Commit rejected by hook, or files other than tests.rs staged
    Evidence: .sisyphus/evidence/task-3-commit.txt

  Scenario: Code review findings documented
    Tool: Bash
    Preconditions: Review completed
    Steps:
      1. Verify commit message body mentions Equal/NotEqual dead variants observation
      2. Verify commit message body confirms operator pipeline correctness
    Expected Result: Commit body contains review findings
    Failure Indicators: Missing review documentation
    Evidence: .sisyphus/evidence/task-3-review.txt
  ```

  **Commit**: YES
  - Message: `test(parser): add comprehensive operator and precedence tests`
  - Files: `src/parser/tests.rs`
  - Pre-commit: `cargo test --lib && cargo clippy --all-targets -- -D warnings`

---

## Final Verification Wave

> 4 review agents run in PARALLEL. ALL must APPROVE. Present consolidated results to user and get explicit "okay" before completing.

- [x] F1. **Plan Compliance Audit** — `oracle`
  Read the plan end-to-end. For each "Must Have": verify implementation exists (check test functions in `src/parser/tests.rs`). For each "Must NOT Have": search codebase for forbidden changes. Compare deliverables against plan.
  Output: `Must Have [N/N] | Must NOT Have [N/N] | Tasks [N/N] | VERDICT: APPROVE/REJECT`

- [x] F2. **Code Quality Review** — `unspecified-high`
  Run `cargo clippy --all-targets -- -D warnings`. Review all new test functions for: correct assertions, descriptive messages, no `unwrap()` in non-test code, proper use of `matches!` macro and `if let` patterns consistent with existing tests. Check test count increased.
  Output: `Build [PASS/FAIL] | Lint [PASS/FAIL] | Tests [N pass/N fail] | VERDICT`

- [x] F3. **Real Manual QA** — `unspecified-high`
  Run `cargo test --lib parser::tests` and capture full output. Verify 0 failures. Run `cargo test --lib ast::operators::tests` and verify. Count new test functions against plan requirement of 30+.
  Output: `Scenarios [N/N pass] | Test Count [N new] | VERDICT`

- [x] F4. **Scope Fidelity Check** — `deep`
  Verify only `src/parser/tests.rs` was modified. Verify no existing tests were changed. Verify no other source files were touched. Check git diff is clean except for the test file.
  Output: `Tasks [N/N compliant] | Files Modified [expected 1] | VERDICT`

---

## Commit Strategy

- **Wave 3**: `test(parser): add comprehensive operator and precedence tests` - `src/parser/tests.rs`, `cargo test --lib`

---

## Success Criteria

### Verification Commands
```bash
cargo test --lib parser::tests  # Expected: all tests pass, 0 failures
cargo test --lib ast::operators  # Expected: all tests pass, 0 failures
cargo clippy --all-targets -- -D warnings  # Expected: 0 warnings
```

### Final Checklist
- [ ] All "Must Have" present
- [ ] All "Must NOT Have" absent
- [ ] All tests pass
- [ ] Code review completed and findings documented
