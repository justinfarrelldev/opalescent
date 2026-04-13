# Opalescent Correctness Fixes

## TL;DR

> **Quick Summary**: Fix all CRITICAL and MAJOR correctness bugs, stubs, and spec mismatches found during the full-project audit. Covers lexer (UTF-8 offsets, missing operators), parser (cast/array parsing, stale #[ignore] tests), type system (builtin signature alignment, generic lambdas, constraint solver), codegen (string interpolation UB, power operator, entry main args, stubs), C runtime (rename functions, add size-specific variants, align types), formatter (quote handling, output correctness), hot reload (production implementations, recovery), LSP (stdio transport, document sync), testing framework (compile errors), and README accuracy.
>
> **Deliverables**:
> - All CRITICAL bugs fixed (UTF-8 offsets, dangling pointer UB, compile errors)
> - All MAJOR stubs completed (closure capture, generic lambdas, constraint solver, hot reload production impls)
> - C runtime fully aligned with size-specific function variants, "opal_" prefix removed
> - All stale `#[ignore]` tests un-ignored and passing
> - Spec updated (int64 as default literal type)
> - README aligned with actual implementation
> - Serena learnings updated
>
> **Estimated Effort**: XL
> **Parallel Execution**: YES - 5 waves
> **Critical Path**: Baseline → Lexer/Parser fixes → Type System fixes → Codegen fixes → C Runtime + Tooling + Docs

---

## Context

### Original Request
Full correctness audit of the Opalescent programming language compiler. Verify no remaining stubs/TODOs, everything matches the language spec, README is up-to-date. Surface all maintainability issues, bugs, and performance problems. All explicitly-sized numeric types (int8-64, uint8-64, float32/64) must be available with NO shorthand types.

### Interview Summary
**Key Discussions**:
- Default integer literal type: int64 (current behavior is correct, update spec to match)
- Phase scope: ALL phases (4 and 5 are supposed to have passed — fix all stubs)
- Colon-block syntax: Parser DOES support it — stale `#[ignore]` markers need removing
- C runtime: Drop "opal_" prefix, add size-specific variants for all int types, align types
- Test strategy: Strict TDD with RED-GREEN-REFACTOR, test baseline first
- CLI UX: Explicitly OUT OF SCOPE (separate pass later)

**Research Findings**:
- 8 parallel audit agents covered every subsystem
- Numeric type matrix: all 10 types supported lexer→codegen, C runtime is the gap
- No shorthand types found (design constraint met)
- `ast_type_to_core_type` is duplicated between type system and codegen — divergence risk
- Parser supports colon-block syntax but type system tests have stale `#[ignore]` reasons

### Metis Review
**Identified Gaps** (addressed):
- int64→int32 cascade risk: AVOIDED (user chose to keep int64, update spec)
- Phase conflation: RESOLVED (user says all phases should be complete)
- Colon-block ambiguity: RESOLVED (parser supports it, stale ignores)
- Duplicated ast_type_to_core_type: included as centralization task
- Test baseline: included as first task

---

## Work Objectives

### Core Objective
Fix all correctness bugs, complete all stubs, align spec/docs with implementation, and ensure the entire Opalescent compiler is production-correct.

### Concrete Deliverables
- Zero `TODO`/`FIXME` stubs remaining in production code
- Zero stale `#[ignore]` test markers
- C runtime with size-specific functions and no "opal_" prefix
- README accurately describes implemented features only
- Language spec states int64 as default integer literal type
- All tests passing (unit + integration)

### Definition of Done
- [ ] `cargo test` — all tests pass, zero ignored tests (except explicitly phase-gated future work)
- [ ] `cargo test --features integration` — all e2e tests pass
- [ ] `cargo build --release` — clean build, zero warnings
- [ ] No `TODO`, `FIXME`, `HACK`, `STUB` in production code (test code allowed)
- [ ] README sections match actual CLI behavior

### Must Have
- All CRITICAL and MAJOR findings fixed
- Strict TDD: RED (write failing test) → GREEN (minimal fix) → REFACTOR
- Test baseline established before any changes
- C runtime functions renamed (drop "opal_" prefix) with size-specific variants

### Must NOT Have (Guardrails)
- NO CLI UX changes (out of scope — separate pass)
- NO new language features beyond what the spec defines
- NO changing default integer literal type (keep int64)
- NO shorthand types (int, float, uint) — always explicit sizes
- NO "nice to have" improvements (cosmetic refactors, extra docs, style changes)
- NO skipping RED or REFACTOR phases in TDD cycle

---

## Verification Strategy (MANDATORY)

> **ZERO HUMAN INTERVENTION** - ALL verification is agent-executed. No exceptions.

### Test Decision
- **Infrastructure exists**: YES (cargo test, cargo test --features integration)
- **Automated tests**: YES — TDD (RED-GREEN-REFACTOR for every change)
- **Framework**: Rust built-in test framework + integration feature flag
- **TDD Protocol**: Each task follows RED (write failing test first) → GREEN (minimal implementation to pass) → REFACTOR (clean up while tests stay green)

### QA Policy
Every task MUST include agent-executed QA scenarios.
Evidence saved to `.sisyphus/evidence/task-{N}-{scenario-slug}.{ext}`.

- **Compiler changes**: Use Bash — `cargo test`, `cargo test --features integration`
- **C Runtime**: Use Bash — compile test program, run, verify output
- **README/Spec**: Use Bash — `grep` for removed/added content, verify accuracy

---

## Execution Strategy

### Parallel Execution Waves

```
Wave 0 (Baseline — sequential, single task):
└── Task 1: Establish test baseline [quick]

Wave 1 (Foundation — parallel, no dependencies):
├── Task 2: Fix UTF-8 byte offset bug in lexer [deep]
├── Task 3: Add div_euclid/mod_euclid operator tokens [quick]
├── Task 4: Resolve As/Cast token duplication [quick]
├── Task 5: Fix IsNot token inconsistency [quick]
├── Task 6: Centralize ast_type_to_core_type [deep]
├── Task 7: Update language spec — int64 default [quick]
└── Task 8: Fix Option::is_none_or compile errors [quick]

Wave 2 (Parser + Type System — after Wave 1):
├── Task 9: Implement cast "as" parsing (depends: 4) [deep]
├── Task 10: Implement array literal + indexing parsing (depends: none from W1) [deep]
├── Task 11: Fix parser unreachable!() panics (depends: none) [quick]
├── Task 12: Un-ignore stale colon-block tests (depends: none) [quick]
├── Task 13: Fix builtin function signatures (depends: 6) [deep]
├── Task 14: Implement generic lambda type checking (depends: 6) [deep]
├── Task 15: Complete constraint solver (depends: 6) [deep]
└── Task 16: Implement closure capture analysis (depends: none) [deep]

Wave 3 (Codegen + Runtime — after Wave 2):
├── Task 17: Fix string interpolation dangling pointer UB (depends: none from W2) [deep]
├── Task 18: Implement power (^) operator codegen (depends: none) [deep]
├── Task 19: Fix entry main argv construction (depends: none) [deep]
├── Task 20: Remove "task 22" stubs in codegen (depends: 9, 10, 17, 18) [deep]
├── Task 21: Overhaul C runtime — rename + size-specific variants (depends: 6) [deep]
├── Task 22: Fix memory layout placeholders (depends: 6) [quick]
└── Task 23: Implement hot reload production components (depends: none) [deep]

Wave 4 (Tooling + Docs — after Wave 3):
├── Task 24: Fix formatter quote handling + output correctness [unspecified-high]
├── Task 25: Fix package manager transitive deps + version parsing [unspecified-high]
├── Task 26: LSP stdio transport + document sync [deep]
├── Task 27: Update README to match reality [writing]
├── Task 28: Fix Makefile.toml broken task [quick]
├── Task 29: Clean up remaining TODOs/stale comments [quick]
└── Task 30: Track learnings to Serena memory [quick]

Wave FINAL (After ALL tasks — 4 parallel reviews, then user okay):
├── Task F1: Plan compliance audit (oracle)
├── Task F2: Code quality review (unspecified-high)
├── Task F3: Real manual QA (unspecified-high)
└── Task F4: Scope fidelity check (deep)
-> Present results -> Get explicit user okay

Critical Path: T1 → T6 → T13 → T20 → T21 → F1-F4 → user okay
Parallel Speedup: ~65% faster than sequential
Max Concurrent: 8 (Wave 2)
```

### Dependency Matrix

| Task | Depends On | Blocks | Wave |
|------|-----------|--------|------|
| 1 | - | ALL | 0 |
| 2 | 1 | - | 1 |
| 3 | 1 | - | 1 |
| 4 | 1 | 9 | 1 |
| 5 | 1 | - | 1 |
| 6 | 1 | 13, 14, 15, 21, 22 | 1 |
| 7 | 1 | - | 1 |
| 8 | 1 | - | 1 |
| 9 | 4 | 20 | 2 |
| 10 | 1 | 20 | 2 |
| 11 | 1 | - | 2 |
| 12 | 1 | - | 2 |
| 13 | 6 | - | 2 |
| 14 | 6 | - | 2 |
| 15 | 6 | - | 2 |
| 16 | 1 | - | 2 |
| 17 | 1 | 20 | 3 |
| 18 | 1 | 20 | 3 |
| 19 | 1 | - | 3 |
| 20 | 9, 10, 17, 18 | - | 3 |
| 21 | 6 | - | 3 |
| 22 | 6 | - | 3 |
| 23 | 1 | - | 3 |
| 24 | 1 | - | 4 |
| 25 | 1 | - | 4 |
| 26 | 1 | - | 4 |
| 27 | ALL impl tasks | - | 4 |
| 28 | 1 | - | 4 |
| 29 | ALL impl tasks | - | 4 |
| 30 | ALL | - | 4 |

### Agent Dispatch Summary

- **Wave 0**: **1** — T1 → `quick`
- **Wave 1**: **7** — T2 → `deep`, T3 → `quick`, T4 → `quick`, T5 → `quick`, T6 → `deep`, T7 → `quick`, T8 → `quick`
- **Wave 2**: **8** — T9 → `deep`, T10 → `deep`, T11 → `quick`, T12 → `quick`, T13 → `deep`, T14 → `deep`, T15 → `deep`, T16 → `deep`
- **Wave 3**: **7** — T17 → `deep`, T18 → `deep`, T19 → `deep`, T20 → `deep`, T21 → `deep`, T22 → `quick`, T23 → `deep`
- **Wave 4**: **7** — T24 → `unspecified-high`, T25 → `unspecified-high`, T26 → `deep`, T27 → `writing`, T28 → `quick`, T29 → `quick`, T30 → `quick`
- **FINAL**: **4** — F1 → `oracle`, F2 → `unspecified-high`, F3 → `unspecified-high`, F4 → `deep`

---

## TODOs

- [x] 1. Establish Test Baseline

  **What to do**:
  - Run `cargo test` and capture full output — record total pass/fail/ignore counts
  - Run `cargo test --features integration` and capture full output
  - Run `cargo build --release 2>&1` and capture any warnings
  - Save all outputs to `.sisyphus/evidence/task-1-baseline.txt`
  - This baseline is referenced by ALL subsequent tasks to detect regressions

  **Must NOT do**:
  - Do NOT fix any issues yet — observation only
  - Do NOT modify any source files

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: NO
  - **Parallel Group**: Wave 0 (solo)
  - **Blocks**: ALL other tasks
  - **Blocked By**: None

  **References**:
  - `Cargo.toml` — has `[features] integration = []` flag for e2e tests
  - `tests/integration_e2e.rs` — e2e test file
  - `src/type_system/test_integration.rs` — type system integration tests with `#[ignore]` markers

  **Acceptance Criteria**:
  - [ ] `.sisyphus/evidence/task-1-baseline.txt` exists with full test output
  - [ ] Baseline counts documented (e.g., "142 passed, 3 failed, 12 ignored")

  ```
  Scenario: Capture test baseline
    Tool: Bash
    Steps:
      1. Run `cargo test 2>&1` — capture full output
      2. Run `cargo test --features integration 2>&1` — capture full output
      3. Run `cargo build --release 2>&1` — capture warnings
      4. Save all to `.sisyphus/evidence/task-1-baseline.txt`
    Expected Result: File exists with complete test output and summary counts
    Evidence: .sisyphus/evidence/task-1-baseline.txt
  ```

  **Commit**: NO (no changes to commit)

- [x] 2. Fix UTF-8 Byte Offset Bug in Lexer

  **What to do**:
  - RED: Write a test in `src/lexer/tests.rs` that lexes a string containing multi-byte UTF-8 characters (e.g., `let π = 42`) and asserts that `token.span.start` and `token.span.end` are correct byte offsets (not char offsets). This test MUST fail before the fix.
  - GREEN: Fix `advance()` and `advance_line()` in `src/lexer.rs` (lines ~784-800) to increment `position.offset` by the byte length of the character (`ch.len_utf8()`) instead of always incrementing by 1.
  - REFACTOR: Ensure no redundant offset calculations remain. Verify that `lexeme()` slicing (which uses byte offsets into the source string) produces correct results for multi-byte chars.

  **Must NOT do**:
  - Do NOT change the `Position` struct layout
  - Do NOT break existing tests

  **Recommended Agent Profile**:
  - **Category**: `deep`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1 (with Tasks 3-8)
  - **Blocks**: None
  - **Blocked By**: Task 1

  **References**:
  - `src/lexer.rs:784-800` — `advance()` and `advance_line()` methods with the bug
  - `src/lexer.rs` — `lexeme()` method that slices source using byte offsets
  - `src/lexer/tests.rs` — existing lexer tests to follow patterns
  - `src/lexer.rs` — `Position` struct definition (offset, line, column fields)

  **Acceptance Criteria**:
  - [ ] New test exists that uses multi-byte UTF-8 chars and asserts correct byte offsets
  - [ ] `cargo test lexer` → all tests pass including new UTF-8 test
  - [ ] No regressions in `cargo test`

  ```
  Scenario: UTF-8 byte offsets are correct
    Tool: Bash
    Steps:
      1. Run `cargo test lexer::tests::test_utf8 -- --nocapture` (or similar test name)
      2. Verify test passes — byte offsets match expected values for multi-byte chars
    Expected Result: Test passes, byte offsets are `ch.len_utf8()` increments not char-count increments
    Evidence: .sisyphus/evidence/task-2-utf8-offsets.txt

  Scenario: No regressions
    Tool: Bash
    Steps:
      1. Run `cargo test` — full test suite
      2. Compare pass count with baseline (Task 1)
    Expected Result: Pass count >= baseline, no new failures
    Evidence: .sisyphus/evidence/task-2-no-regression.txt
  ```

  **Commit**: YES (groups with Wave 1)
  - Message: `fix(lexer): use byte length for UTF-8 offset tracking`
  - Files: `src/lexer.rs`, `src/lexer/tests.rs`

- [x] 3. Add div_euclid / mod_euclid Operator Tokens

  **What to do**:
  - RED: Write tests in lexer tests that tokenize `div_euclid` and `mod_euclid` as operators and assert correct TokenType variants. Tests MUST fail first.
  - GREEN: Add `TokenType::DivEuclid` and `TokenType::ModEuclid` variants to `src/token.rs`. Add keyword mappings in `src/lexer.rs`. Add corresponding `BinaryOp` variants in `src/ast.rs` if not present.
  - REFACTOR: Ensure Display/Debug impls are consistent for new variants.
  - Reference `language-spec/requirements/math.md` for the spec requirements on these operators.

  **Must NOT do**:
  - Do NOT implement codegen for these operators in this task (that's part of the codegen wave)

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1 (with Tasks 2, 4-8)
  - **Blocks**: None directly (codegen support is separate)
  - **Blocked By**: Task 1

  **References**:
  - `language-spec/requirements/math.md` — spec for euclidean division/modulo
  - `src/token.rs` — TokenType enum where new variants go
  - `src/lexer.rs` — keyword mapping section (look for `keywords.insert`)
  - `src/ast.rs` — BinaryOp enum

  **Acceptance Criteria**:
  - [ ] `TokenType::DivEuclid` and `TokenType::ModEuclid` exist in token.rs
  - [ ] Lexer maps `div_euclid` and `mod_euclid` to correct token types
  - [ ] `cargo test lexer` passes with new operator tests

  ```
  Scenario: div_euclid/mod_euclid lex correctly
    Tool: Bash
    Steps:
      1. Run `cargo test lexer::tests` — verify new operator token tests pass
    Expected Result: New tests pass, operators tokenize as DivEuclid/ModEuclid
    Evidence: .sisyphus/evidence/task-3-euclid-ops.txt
  ```

  **Commit**: YES (groups with Wave 1)
  - Message: `feat(lexer): add div_euclid and mod_euclid operator tokens`
  - Files: `src/token.rs`, `src/lexer.rs`, `src/lexer/tests.rs`, `src/ast.rs`

- [x] 4. Resolve As/Cast Token Duplication

  **What to do**:
  - RED: Write a test that lexes `x as int32` and asserts the token type is `TokenType::Cast` (not `TokenType::As`). Test MUST fail first (currently lexer maps "as" to `TokenType::As`).
  - GREEN: Decide on ONE canonical token for the cast operator. Since `Cast` is used by the AST (`Expr::Cast`), unify on `TokenType::Cast`. Update lexer keyword mapping to map `"as"` → `TokenType::Cast`. Remove `TokenType::As` if unused elsewhere, or alias it.
  - REFACTOR: Search for all references to `TokenType::As` and `TokenType::Cast` and ensure consistency.

  **Must NOT do**:
  - Do NOT implement the parser handling of cast in this task (that's Task 9)

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1
  - **Blocks**: Task 9 (cast parsing depends on consistent token)
  - **Blocked By**: Task 1

  **References**:
  - `src/token.rs:~line 203` — both `As` and `Cast` variants exist
  - `src/lexer.rs:155` — maps `"as"` to `TokenType::As`
  - `src/ast.rs` — `Expr::Cast` uses the cast concept
  - Use `lsp_find_references` on both `TokenType::As` and `TokenType::Cast` to map all usages

  **Acceptance Criteria**:
  - [ ] Only ONE token variant for cast remains (prefer `Cast`)
  - [ ] `"as"` keyword maps to that variant in lexer
  - [ ] `cargo test` passes with no regressions

  ```
  Scenario: "as" keyword produces Cast token
    Tool: Bash
    Steps:
      1. Run `cargo test lexer::tests` — verify cast token test passes
      2. Run `cargo test` — full suite, no regressions
    Expected Result: "as" lexes to Cast, no As/Cast ambiguity
    Evidence: .sisyphus/evidence/task-4-cast-token.txt
  ```

  **Commit**: YES (groups with Wave 1)
  - Message: `fix(tokens): unify As/Cast into single Cast token`
  - Files: `src/token.rs`, `src/lexer.rs`, `src/lexer/tests.rs`

- [x] 5. Fix IsNot Token Inconsistency

  **What to do**:
  - RED: Write a test that lexes `x is not None` and asserts that the lexer produces a single `TokenType::IsNot` token (or the intended token sequence). Test MUST fail first.
  - GREEN: Either (a) make the lexer emit `TokenType::IsNot` when it sees `is` followed by `not`, or (b) remove `TokenType::IsNot` and have the parser handle `Is` + `Not` as a compound operator. Choose the approach consistent with the spec.
  - REFACTOR: Ensure pattern matching and comparison code handles whichever approach is chosen.

  **Must NOT do**:
  - Do NOT change the semantics of `is` or `not` individually

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1
  - **Blocks**: None
  - **Blocked By**: Task 1

  **References**:
  - `src/token.rs:203-206` — `TokenType::IsNot` variant exists
  - `src/lexer.rs` — currently emits `Is` and `Not` as separate tokens
  - `src/parser/expressions.rs` — how `is` / `is not` are parsed in conditions
  - `language-spec/requirements/overview.md` — spec for comparison operators

  **Acceptance Criteria**:
  - [ ] `is not` handling is consistent between lexer and parser
  - [ ] Test for `is not` operator passes
  - [ ] `cargo test` — no regressions

  ```
  Scenario: "is not" operator handled consistently
    Tool: Bash
    Steps:
      1. Run `cargo test lexer::tests` and `cargo test parser::tests` — verify is_not tests pass
    Expected Result: is not operator lexes and parses correctly
    Evidence: .sisyphus/evidence/task-5-is-not.txt
  ```

  **Commit**: YES (groups with Wave 1)
  - Message: `fix(lexer): resolve IsNot token inconsistency`
  - Files: `src/token.rs`, `src/lexer.rs`, `src/lexer/tests.rs`

- [ ] 6. Centralize ast_type_to_core_type

  **What to do**:
  - RED: Write a test (or modify existing) that asserts the centralized function handles all 10 numeric types correctly, including edge cases. Verify the test fails if the function is missing/wrong.
  - GREEN: Extract `ast_type_to_core_type` into a shared module (e.g., `src/type_system/type_mapping.rs` or similar canonical location). Remove the duplicate from codegen. Update all call sites (type_system/checker.rs and codegen modules) to use the single implementation.
  - REFACTOR: Verify both the type checker and codegen produce identical results for all type mappings. Add a cross-reference test if needed.

  **Must NOT do**:
  - Do NOT change the behavior of the mapping — only centralize it
  - Do NOT add new type mappings in this task

  **Recommended Agent Profile**:
  - **Category**: `deep`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1
  - **Blocks**: Tasks 13, 14, 15, 21, 22 (all type system/codegen work depends on centralized mapping)
  - **Blocked By**: Task 1

  **References**:
  - `src/type_system/checker.rs:668-687` — one copy of `ast_type_to_core_type`
  - `src/codegen/statements.rs` and `src/codegen/functions.rs` — duplicated copies in codegen
  - `src/type_system/types.rs` — `CoreType` enum definition
  - `src/ast/types.rs` — `Type` enum definition
  - Use `serena_find_symbol` with `ast_type_to_core_type` to find ALL occurrences

  **Acceptance Criteria**:
  - [ ] Single authoritative `ast_type_to_core_type` function exists
  - [ ] No duplicate implementations remain in codegen
  - [ ] `cargo test` — all tests pass, no regressions

  ```
  Scenario: Centralized type mapping works
    Tool: Bash
    Steps:
      1. Run `cargo test` — verify all type system and codegen tests pass
      2. Use `grep -rn "ast_type_to_core_type" src/` — verify only ONE definition exists
    Expected Result: Single definition, all tests pass
    Evidence: .sisyphus/evidence/task-6-centralize-types.txt
  ```

  **Commit**: YES (groups with Wave 1)
  - Message: `refactor(types): centralize ast_type_to_core_type into single implementation`
  - Files: `src/type_system/checker.rs`, `src/codegen/statements.rs`, `src/codegen/functions.rs`, new shared module

- [x] 7. Update Language Spec — int64 as Default Literal Type

  **What to do**:
  - Review `language-spec/requirements/overview.md` and `language-spec/requirements/math.md` for any mention of default integer literal type
  - If the spec says int32 is default, update it to say int64
  - Verify consistency across all spec files and example `.op` files
  - Update any comments in source code that reference "default int32" to say "default int64"

  **Must NOT do**:
  - Do NOT change any compiler behavior — only update documentation/spec
  - Do NOT modify test expectations

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1
  - **Blocks**: None
  - **Blocked By**: Task 1

  **References**:
  - `language-spec/requirements/overview.md` — main spec document
  - `language-spec/requirements/math.md` — math operations spec
  - `src/type_system/checker/helpers.rs:12-19` — `literal_to_core_type` returns Int64

  **Acceptance Criteria**:
  - [ ] Spec states int64 as default integer literal type
  - [ ] No source comments contradict this

  ```
  Scenario: Spec states int64 default
    Tool: Bash
    Steps:
      1. `grep -n "int32.*default\|default.*int32" language-spec/requirements/*.md` — should return 0 results
      2. `grep -n "int64.*default\|default.*int64" language-spec/requirements/*.md` — should find the updated text
    Expected Result: Spec consistently says int64 is default
    Evidence: .sisyphus/evidence/task-7-spec-update.txt
  ```

  **Commit**: YES (groups with Wave 1)
  - Message: `docs(spec): update default integer literal type to int64`
  - Files: `language-spec/requirements/overview.md`, `language-spec/requirements/math.md`

- [x] 8. Fix Option::is_none_or Compile Errors

  **What to do**:
  - RED: Run `cargo test` and confirm that `Option::is_none_or` causes compile errors (or that it only works on nightly). Capture the error.
  - GREEN: Replace all uses of `Option::is_none_or` with equivalent stable Rust: `opt.as_ref().map_or(true, |v| predicate(v))` or `opt.is_none() || predicate(opt.unwrap())` (prefer the map_or pattern).
  - REFACTOR: Verify the replacement is semantically identical. Check for similar nightly-only API uses.
  - Locations: `testing/discovery.rs:62-64`, `codegen/expressions_numeric.rs`, `type_system/checker/hot_reload.rs`

  **Must NOT do**:
  - Do NOT change behavior — exact semantic equivalence required

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1
  - **Blocks**: None
  - **Blocked By**: Task 1

  **References**:
  - `src/testing/discovery.rs:62-64` — uses `is_none_or`
  - `src/codegen/expressions_numeric.rs` — uses `is_none_or`
  - `src/type_system/checker/hot_reload.rs` — uses `is_none_or`
  - Rust docs: `is_none_or` stabilized in Rust 1.82, but may not be available in all toolchains

  **Acceptance Criteria**:
  - [ ] No uses of `Option::is_none_or` remain in codebase
  - [ ] Replacement is semantically identical
  - [ ] `cargo test` — no compile errors from this API

  ```
  Scenario: No is_none_or usage remains
    Tool: Bash
    Steps:
      1. `grep -rn "is_none_or" src/` — should return 0 results
      2. `cargo build` — should compile cleanly
    Expected Result: Zero occurrences, clean build
    Evidence: .sisyphus/evidence/task-8-is-none-or.txt
  ```

  **Commit**: YES (groups with Wave 1)
  - Message: `fix: replace nightly-only Option::is_none_or with stable equivalent`
  - Files: `src/testing/discovery.rs`, `src/codegen/expressions_numeric.rs`, `src/type_system/checker/hot_reload.rs`

- [ ] 9. Implement Cast "as" Parsing

  **What to do**:
  - RED: Write parser tests for `x as int32`, `value as float64`, `(a + b) as int64` that assert `Expr::Cast` nodes are produced. Tests MUST fail first.
  - GREEN: Add cast parsing in `src/parser/expressions.rs`. Handle `TokenType::Cast` (the unified token from Task 4) as an infix operator in the precedence table. Parse `<expr> as <type>` into `Expr::Cast { expr, target_type, span }`. Assign appropriate precedence (typically same as comparison or just above).
  - REFACTOR: Verify cast works with complex expressions (nested casts, casts in function args, etc.). Clean up any dead code related to the old `As` token.

  **Must NOT do**:
  - Do NOT implement type-checking or codegen for casts (those already exist or are separate tasks)

  **Recommended Agent Profile**:
  - **Category**: `deep`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2 (with Tasks 10-16)
  - **Blocks**: Task 20 (codegen stubs removal depends on parser completeness)
  - **Blocked By**: Task 4 (unified Cast token)

  **References**:
  - `src/ast.rs` — `Expr::Cast { expr, target_type, span }` already defined
  - `src/parser/expressions.rs` — `parse_infix` and `parse_primary` where cast should be added
  - `src/parser/precedence.rs` — precedence table to add Cast entry
  - `src/parser/tests.rs:2126` — TODO comment about adding cast tests

  **Acceptance Criteria**:
  - [ ] `x as int32` parses to `Expr::Cast`
  - [ ] Parser tests for cast expressions pass
  - [ ] `cargo test parser` — all pass

  ```
  Scenario: Cast expressions parse correctly
    Tool: Bash
    Steps:
      1. Run `cargo test parser::tests` — verify cast parsing tests pass
      2. Run `cargo test` — no regressions
    Expected Result: Cast expressions produce Expr::Cast AST nodes
    Evidence: .sisyphus/evidence/task-9-cast-parsing.txt

  Scenario: Nested cast expressions
    Tool: Bash
    Steps:
      1. Verify test for `(a + b) as int32` parses correctly (nested expression in cast)
    Expected Result: Test passes, cast binds at correct precedence
    Evidence: .sisyphus/evidence/task-9-nested-cast.txt
  ```

  **Commit**: YES (groups with Wave 2)
  - Message: `feat(parser): implement cast "as" expression parsing`
  - Files: `src/parser/expressions.rs`, `src/parser/precedence.rs`, `src/parser/tests.rs`

- [ ] 10. Implement Array Literal + Indexing Parsing

  **What to do**:
  - RED: Write parser tests for array literals (`[1, 2, 3]`), empty arrays (`[]`), typed empty arrays (`int32[]`), and indexing (`arr[0]`, `arr[i + 1]`). Tests MUST fail first.
  - GREEN: Add array literal parsing in `parse_primary` — when `LeftBracket` is encountered, parse comma-separated expressions until `RightBracket`. Produce `Expr::Array`. Add indexing in `parse_infix` — when `LeftBracket` follows an expression, parse the index expression and produce `Expr::Index`. Handle precedence correctly (indexing should bind tighter than most operators).
  - REFACTOR: Verify edge cases (empty arrays, nested indexing `arr[0][1]`, indexing function results `f()[0]`).

  **Must NOT do**:
  - Do NOT implement array type checking or codegen (separate concerns)

  **Recommended Agent Profile**:
  - **Category**: `deep`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2
  - **Blocks**: Task 20
  - **Blocked By**: Task 1

  **References**:
  - `src/ast.rs` — `Expr::Array` and `Expr::Index` already defined in AST
  - `src/parser/expressions.rs` — `parse_primary` (add LeftBracket case) and `parse_infix` (add indexing)
  - `src/parser/precedence.rs` — `LeftBracket` already mapped to `Call` precedence but not implemented
  - `src/parser/tests.rs:2564` — TODO comment about adding array/index tests

  **Acceptance Criteria**:
  - [ ] `[1, 2, 3]` parses to `Expr::Array`
  - [ ] `arr[0]` parses to `Expr::Index`
  - [ ] `cargo test parser` — all pass including new array tests

  ```
  Scenario: Array literals and indexing parse
    Tool: Bash
    Steps:
      1. Run `cargo test parser::tests` — verify array/index tests pass
    Expected Result: Array literals and indexing produce correct AST nodes
    Evidence: .sisyphus/evidence/task-10-arrays.txt
  ```

  **Commit**: YES (groups with Wave 2)
  - Message: `feat(parser): implement array literal and indexing parsing`
  - Files: `src/parser/expressions.rs`, `src/parser/precedence.rs`, `src/parser/tests.rs`

- [ ] 11. Fix Parser unreachable!() Panics

  **What to do**:
  - RED: Write tests that feed the parser token sequences that would trigger each `unreachable!()` in production code. Verify tests panic (fail). Locations: `parser/expressions.rs:229`, `parser/statements.rs:565`, `parser/statements.rs:582`.
  - GREEN: Replace each `unreachable!()` with appropriate `ParseError` variant returns. Follow existing `ParseError` patterns in `src/parser/errors.rs`.
  - REFACTOR: Search for any other `unreachable!()` in non-test parser code. Ensure error messages are descriptive.

  **Must NOT do**:
  - Do NOT change `unreachable!()` in test code — those are intentional

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2
  - **Blocks**: None
  - **Blocked By**: Task 1

  **References**:
  - `src/parser/expressions.rs:229` — unreachable in production code
  - `src/parser/statements.rs:565` — unreachable in production code
  - `src/parser/statements.rs:582` — unreachable in production code
  - `src/parser/errors.rs` — existing ParseError variants to follow

  **Acceptance Criteria**:
  - [ ] Zero `unreachable!()` in non-test parser code
  - [ ] New tests verify graceful error returns instead of panics
  - [ ] `cargo test parser` — all pass

  ```
  Scenario: Parser returns errors instead of panicking
    Tool: Bash
    Steps:
      1. Run `cargo test parser::tests` — verify error-path tests pass (no panics)
      2. `grep -n "unreachable!" src/parser/*.rs | grep -v test` — should return 0 results
    Expected Result: No unreachable in production parser code, tests pass
    Evidence: .sisyphus/evidence/task-11-no-unreachable.txt
  ```

  **Commit**: YES (groups with Wave 2)
  - Message: `fix(parser): replace unreachable!() with proper ParseError returns`
  - Files: `src/parser/expressions.rs`, `src/parser/statements.rs`

- [ ] 12. Un-ignore Stale Colon-Block Tests

  **What to do**:
  - Remove the `#[ignore = "...colon-block syntax..."]` attribute from ALL tests in `src/type_system/test_integration.rs` and `src/type_system/test_integration_ecosystem.rs` that have stale reasons about colon-block syntax not being supported by the parser.
  - The parser DOES support colon-block syntax (verified: `parse_if_statement` line 390, `parse_for` line 484, `parse_while` line 527 all handle `TokenType::Colon` → `parse_indent_block`).
  - Run each un-ignored test individually to verify it passes. If any fail, the failure is in the TYPE CHECKER (not the parser) — fix the type checker issue or file it as a new finding with accurate description.
  - Also fix any test that uses `parse_pipeline` instead of `parse_pipeline_with_spaces` when processing spec files with tabs.

  **Known stale `#[ignore]` tests**:
  - `test_integration.rs:148` — `test_fib_recursive_spec_file_parses_and_type_checks`
  - `test_integration.rs:195` — `test_fib_iterative_spec_file_parses_and_type_checks`
  - `test_integration_ecosystem.rs:87` — types_example.types.op test
  - `test_integration_ecosystem.rs:99` — array_helpers.op test
  - `test_integration_ecosystem.rs:114` — partition.op test
  - `test_integration_ecosystem.rs:129` — unique_adjacent_sorted.op test
  - `test_integration_ecosystem.rs:144` — simple_quiz.op test

  **Must NOT do**:
  - Do NOT remove ignores that have ACCURATE reasons (non-stale)
  - Do NOT remove tests — only remove the `#[ignore]` attribute
  - Do NOT silence test failures — if a test fails after un-ignoring, diagnose and fix the root cause

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2
  - **Blocks**: None
  - **Blocked By**: Task 1

  **References**:
  - `src/type_system/test_integration.rs:147-158` — fib_recursive stale ignore
  - `src/type_system/test_integration.rs:194-205` — fib_iterative stale ignore
  - `src/type_system/test_integration_ecosystem.rs:83-145` — ecosystem tests with stale ignores
  - `src/parser/statements.rs:388-393` — PROOF that parser supports colon-block (if → Colon → parse_indent_block)
  - `tests/integration_e2e.rs:250-393` — PROOF that fib files compile and run end-to-end

  **Acceptance Criteria**:
  - [ ] All stale `#[ignore = "...colon-block..."]` markers removed
  - [ ] Each un-ignored test either passes or has its failure diagnosed and fixed
  - [ ] `cargo test type_system::test_integration` — all pass (or failures are NEW findings with accurate descriptions)

  ```
  Scenario: Un-ignored tests pass
    Tool: Bash
    Steps:
      1. Run `cargo test test_fib_recursive_spec_file -- --nocapture` — should pass
      2. Run `cargo test test_fib_iterative_spec_file -- --nocapture` — should pass
      3. Run `cargo test test_integration_ecosystem -- --nocapture` — check results
    Expected Result: All un-ignored tests pass; if any fail, failure is diagnosed accurately
    Evidence: .sisyphus/evidence/task-12-un-ignore.txt

  Scenario: No stale ignores remain
    Tool: Bash
    Steps:
      1. `grep -n "colon-block syntax.*not yet supported" src/type_system/*.rs` — should return 0
    Expected Result: Zero stale ignore markers
    Evidence: .sisyphus/evidence/task-12-no-stale.txt
  ```

  **Commit**: YES (groups with Wave 2)
  - Message: `fix(tests): un-ignore stale colon-block tests — parser supports this syntax`
  - Files: `src/type_system/test_integration.rs`, `src/type_system/test_integration_ecosystem.rs`

- [ ] 13. Fix Builtin Function Signatures

  **What to do**:
  - RED: Write type checker tests that call `string_to_int64`, `random_int64` (the corrected names from Task 21) and assert they return `CoreType::Int64`. Also test `print` with various types. Tests MUST fail first.
  - GREEN: Fix builtin registrations in `src/type_system/checker.rs`:
    - Lines 212-221: `string_to_int32` currently returns `CoreType::Int64` — rename to `string_to_int64` and keep return type `Int64`, OR keep name and fix return to `Int32`. Coordinate with Task 21 (C runtime rename).
    - Lines 241-249: `random_int32` returns `Int64` — same fix needed.
    - Lines 172-179: `print` uses TypeVar id=0 not declared in `generic_params` — add proper generic_params.
  - REFACTOR: Add builtins for other sizes where the C runtime will provide them (coordinate with Task 21). Ensure TypeVar IDs don't collide with auto-generated ones.

  **Must NOT do**:
  - Do NOT change the C runtime in this task (that's Task 21)

  **Recommended Agent Profile**:
  - **Category**: `deep`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2
  - **Blocks**: None
  - **Blocked By**: Task 6 (centralized type mapping)

  **References**:
  - `src/type_system/checker.rs:172-179` — print builtin with TypeVar issue
  - `src/type_system/checker.rs:212-221` — string_to_int32 builtin returning Int64
  - `src/type_system/checker.rs:241-249` — random_int32 builtin returning Int64
  - `src/type_system/checker.rs` — `register_builtins` method
  - `runtime/opal_runtime.c` — C runtime function signatures (for alignment reference)

  **Acceptance Criteria**:
  - [ ] Builtin function names match their return types
  - [ ] `print` generic parameter is properly declared
  - [ ] TypeVar IDs don't collide with auto-generated ones
  - [ ] `cargo test type_system` — all pass

  ```
  Scenario: Builtins have correct signatures
    Tool: Bash
    Steps:
      1. Run `cargo test type_system` — all tests pass
      2. Verify builtin registration matches C runtime signatures
    Expected Result: No name/type mismatches in builtins
    Evidence: .sisyphus/evidence/task-13-builtins.txt
  ```

  **Commit**: YES (groups with Wave 2)
  - Message: `fix(types): align builtin function signatures with actual types`
  - Files: `src/type_system/checker.rs`

- [ ] 14. Implement Generic Lambda Type Checking

  **What to do**:
  - RED: Write type checker tests for generic lambdas (e.g., `let id = f<T>(x: T): T => return x`) that assert successful type checking. Tests MUST fail first (currently returns `NotImplementedYet`).
  - GREEN: Implement the generic lambda type checking logic in `src/type_system/checker/expressions.rs:643-657`. This needs to: instantiate type variables for generic parameters, type-check the lambda body with those type variables in scope, and return the lambda's function type with generic parameters.
  - REFACTOR: Remove the `NotImplementedYet` return. Ensure generic lambdas work when passed as arguments to higher-order functions.

  **Must NOT do**:
  - Do NOT remove `TypeError::NotImplementedYet` variant entirely (may be used elsewhere)

  **Recommended Agent Profile**:
  - **Category**: `deep`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2
  - **Blocks**: None
  - **Blocked By**: Task 6

  **References**:
  - `src/type_system/checker/expressions.rs:643-657` — the `NotImplementedYet` stub
  - `src/type_system/checker/expressions.rs` — non-generic lambda type checking (for pattern reference)
  - `src/type_system/types.rs` — `Type::Function` and generic parameter handling
  - `src/type_system/environment.rs` — type environment for scoping type variables

  **Acceptance Criteria**:
  - [ ] Generic lambdas type-check successfully
  - [ ] `NotImplementedYet` no longer returned for generic lambdas
  - [ ] `cargo test type_system` — all pass

  ```
  Scenario: Generic lambda type checks
    Tool: Bash
    Steps:
      1. Run `cargo test type_system` — generic lambda tests pass
    Expected Result: Generic lambdas produce correct function types
    Evidence: .sisyphus/evidence/task-14-generic-lambda.txt
  ```

  **Commit**: YES (groups with Wave 2)
  - Message: `feat(types): implement generic lambda type checking`
  - Files: `src/type_system/checker/expressions.rs`

- [ ] 15. Complete Constraint Solver

  **What to do**:
  - RED: Write tests for constraint solving scenarios: unifying type variables, applying substitutions, resolving generic function calls with inferred types. Tests MUST fail first.
  - GREEN: Complete the "Phase 2" implementation in `src/type_system/checker.rs:448-476`. This needs to: process collected constraints, perform unification, apply resulting substitutions to the AST/type environment. Also fix the TODO about `unknown_span` at line 465.
  - REFACTOR: Remove "Phase 2 - not yet implemented" comments. Ensure constraint solving integrates with the existing type checking flow.

  **Must NOT do**:
  - Do NOT redesign the constraint system — complete the existing design

  **Recommended Agent Profile**:
  - **Category**: `deep`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2
  - **Blocks**: None
  - **Blocked By**: Task 6

  **References**:
  - `src/type_system/checker.rs:448-476` — incomplete constraint solver
  - `src/type_system/constraints.rs` — constraint types and definitions
  - `src/type_system/substitution.rs` — substitution application logic
  - `src/type_system/checker.rs:465` — TODO about unknown_span

  **Acceptance Criteria**:
  - [ ] Constraint solver processes all constraint types
  - [ ] No "Phase 2" or "not yet implemented" comments remain
  - [ ] `cargo test type_system` — all pass

  ```
  Scenario: Constraint solver resolves types
    Tool: Bash
    Steps:
      1. Run `cargo test type_system` — constraint tests pass
    Expected Result: Type variables are resolved through constraint solving
    Evidence: .sisyphus/evidence/task-15-constraints.txt
  ```

  **Commit**: YES (groups with Wave 2)
  - Message: `feat(types): complete constraint solver implementation`
  - Files: `src/type_system/checker.rs`, `src/type_system/constraints.rs`

- [ ] 16. Implement Closure Capture Analysis

  **What to do**:
  - RED: Write parser + type checker tests for closures that capture outer variables (e.g., `let x = 5; let add_x = f(y: int64): int64 => return x + y`). Assert that `captured_variables` is populated correctly in the AST. Tests MUST fail first.
  - GREEN: Implement closure capture analysis in `src/parser/expressions.rs:641` (the TODO). This needs to: when parsing a lambda/closure body, track which identifiers reference variables from enclosing scopes, and populate `captured_variables` in the `Expr::Lambda` node. Also update `src/ast.rs:429,436,437` related TODOs.
  - REFACTOR: Verify captures work for nested closures, mutable captures, and closures inside loops.

  **Must NOT do**:
  - Do NOT implement closure codegen (lowering captured variables to a struct) — that's part of codegen tasks

  **Recommended Agent Profile**:
  - **Category**: `deep`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2
  - **Blocks**: None (codegen closure support is a separate concern)
  - **Blocked By**: Task 1

  **References**:
  - `src/parser/expressions.rs:641` — `captured_variables: Vec::new(), // TODO: Implement closure capture analysis`
  - `src/ast.rs:429,436,437` — captured_variables field and Phase 4-5 TODOs
  - `src/parser/expressions.rs` — lambda parsing logic (context for where captures happen)
  - `src/type_system/symbol_table.rs` — symbol table for looking up enclosing scope variables

  **Acceptance Criteria**:
  - [ ] `captured_variables` is populated for closures that reference outer scope
  - [ ] Empty `captured_variables` for closures that don't capture
  - [ ] TODO at expressions.rs:641 is removed
  - [ ] `cargo test parser` — all pass

  ```
  Scenario: Closure captures detected
    Tool: Bash
    Steps:
      1. Run `cargo test parser::tests` — closure capture tests pass
    Expected Result: captured_variables correctly identifies outer scope references
    Evidence: .sisyphus/evidence/task-16-closure-capture.txt
  ```

  **Commit**: YES (groups with Wave 2)
  - Message: `feat(parser): implement closure capture analysis`
  - Files: `src/parser/expressions.rs`, `src/ast.rs`

- [ ] 17. Fix String Interpolation Dangling Pointer UB

  **What to do**:
  - RED: Write a codegen test that generates and runs code with string interpolation where the interpolated string is returned from a function or stored in a variable that outlives the function scope. Test MUST demonstrate the UB (crash, corrupt output, or sanitizer error).
  - GREEN: Fix `src/codegen/expressions_string.rs:26-73`. Replace the fixed 256-byte stack alloca with a heap allocation (malloc + format + return pointer) or use the runtime's string allocation mechanism. Ensure the returned pointer is valid for the string's lifetime. Also fix the buffer overflow risk — dynamically size the buffer based on actual interpolation result length.
  - REFACTOR: Remove the hardcoded 256-byte limit. Ensure the allocation strategy is consistent with how other strings are managed in the runtime.

  **Must NOT do**:
  - Do NOT change the string interpolation syntax or semantics
  - Do NOT introduce memory leaks — if heap allocating, ensure cleanup path exists

  **Recommended Agent Profile**:
  - **Category**: `deep`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 3 (with Tasks 18-23)
  - **Blocks**: Task 20
  - **Blocked By**: Task 1

  **References**:
  - `src/codegen/expressions_string.rs:26-73` — the dangling pointer code
  - `src/codegen/values.rs` — string value helpers
  - `runtime/opal_runtime.c` — C runtime string functions for allocation patterns
  - `src/codegen/expressions.rs` — how other expressions handle string results

  **Acceptance Criteria**:
  - [ ] String interpolation returns valid pointer (not stack alloca)
  - [ ] No fixed buffer size limit
  - [ ] Integration test with string interpolation passes without sanitizer errors
  - [ ] `cargo test --features integration` — fib tests still pass (they use string interpolation)

  ```
  Scenario: String interpolation doesn't crash
    Tool: Bash
    Steps:
      1. Run `cargo test --features integration` — fib tests use string interpolation
      2. Verify fib-recursive outputs "fib(10) = 55" correctly
    Expected Result: String interpolation produces correct output, no crashes
    Evidence: .sisyphus/evidence/task-17-string-interp.txt

  Scenario: Long interpolated strings work
    Tool: Bash
    Steps:
      1. Write a test with interpolation exceeding 256 chars
      2. Verify it doesn't buffer overflow
    Expected Result: Long strings work correctly
    Evidence: .sisyphus/evidence/task-17-long-string.txt
  ```

  **Commit**: YES (groups with Wave 3)
  - Message: `fix(codegen): heap-allocate interpolated strings to prevent UB`
  - Files: `src/codegen/expressions_string.rs`

- [ ] 18. Implement Power (^) Operator Codegen

  **What to do**:
  - RED: Write a codegen/integration test that uses the power operator (`2 ^ 10`) and asserts the result is 1024. Test MUST fail first (currently returns "unsupported in task 22" error).
  - GREEN: Implement `BinaryOp::Power` codegen in `src/codegen/expressions.rs:291`. For integer types, use a loop or call to `powi`. For float types, use LLVM's `pow` intrinsic or `powf`. Handle edge cases (0^0, negative exponents).
  - REFACTOR: Remove the "task 22" error message. Ensure power works for all numeric types (int32, int64, float32, float64).

  **Must NOT do**:
  - Do NOT implement power for non-numeric types

  **Recommended Agent Profile**:
  - **Category**: `deep`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 3
  - **Blocks**: Task 20
  - **Blocked By**: Task 1

  **References**:
  - `src/codegen/expressions.rs:291` — `BinaryOp::Power` returning error
  - `src/codegen/expressions_numeric.rs` — numeric operation codegen patterns
  - `language-spec/requirements/math.md` — spec for exponentiation
  - LLVM docs: `powi`, `pow` intrinsics

  **Acceptance Criteria**:
  - [ ] `2 ^ 10` computes to 1024
  - [ ] Power works for int32, int64, float32, float64
  - [ ] "task 22" error message removed for power operator
  - [ ] `cargo test codegen` — all pass

  ```
  Scenario: Power operator works
    Tool: Bash
    Steps:
      1. Run `cargo test codegen` — power operator tests pass
    Expected Result: 2^10 = 1024, float powers work
    Evidence: .sisyphus/evidence/task-18-power-op.txt
  ```

  **Commit**: YES (groups with Wave 3)
  - Message: `feat(codegen): implement power (^) operator`
  - Files: `src/codegen/expressions.rs`, `src/codegen/expressions_numeric.rs`

- [ ] 19. Fix Entry Main argv Construction

  **What to do**:
  - RED: Write an integration test that compiles a program accessing `args[0]` and verifies it receives the program name. Test MUST fail first (currently uses placeholder zero/undef values).
  - GREEN: Fix `src/codegen/functions.rs:682-727`. The C ABI wrapper receives `argc: i32` and `argv: **c_char`. Implement proper conversion: iterate argv, convert each `*c_char` to the runtime's string representation, build a string array, and pass it to the Opalescent `main(args: string[])`.
  - REFACTOR: Ensure the argv construction handles edge cases (no args, unicode args, empty strings).

  **Must NOT do**:
  - Do NOT change the entry function signature
  - Do NOT change the C ABI wrapper's external interface

  **Recommended Agent Profile**:
  - **Category**: `deep`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 3
  - **Blocks**: None
  - **Blocked By**: Task 1

  **References**:
  - `src/codegen/functions.rs:682-727` — entry main C ABI wrapper with placeholder values
  - `src/codegen/functions.rs` — how other functions are generated (for pattern reference)
  - `runtime/opal_runtime.c` — C runtime string handling

  **Acceptance Criteria**:
  - [ ] Programs can access command-line arguments via `args` parameter
  - [ ] No placeholder/undef values remain in entry wrapper
  - [ ] `cargo test --features integration` — all pass

  ```
  Scenario: argv is accessible
    Tool: Bash
    Steps:
      1. Run `cargo test --features integration` — existing tests pass (they don't heavily use args, but should not crash)
    Expected Result: Entry wrapper constructs valid string[] from argv
    Evidence: .sisyphus/evidence/task-19-argv.txt
  ```

  **Commit**: YES (groups with Wave 3)
  - Message: `fix(codegen): implement proper argv construction for entry main`
  - Files: `src/codegen/functions.rs`

- [ ] 20. Remove "task 22" Stubs in Codegen

  **What to do**:
  - RED: Search for ALL occurrences of "task 22" in codegen. For each, write a test that exercises the code path. Tests MUST fail with the "unsupported in task 22" error.
  - GREEN: Implement each stubbed code path. These include various expression kinds, operations, and casts that were deferred. Use the AST definitions and type system as guides for what each should produce.
  - REFACTOR: Remove ALL "task 22" error messages. Verify no unsupported operations remain for spec-defined features.

  **Must NOT do**:
  - Do NOT implement features NOT in the spec
  - Do NOT leave any "task 22" stubs

  **Recommended Agent Profile**:
  - **Category**: `deep`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: NO (depends on Tasks 9, 10, 17, 18 completing first)
  - **Parallel Group**: Wave 3 (sequential after dependencies)
  - **Blocks**: None
  - **Blocked By**: Tasks 9, 10, 17, 18

  **References**:
  - `src/codegen/expressions.rs` — multiple "task 22" stubs
  - `src/codegen/values.rs:12-19` — `build_string_placeholder` returns None
  - Search: `grep -rn "task 22\|Task 22" src/codegen/` to find all occurrences

  **Acceptance Criteria**:
  - [ ] `grep -rn "task 22" src/codegen/` returns 0 results
  - [ ] All previously-stubbed code paths now work
  - [ ] `cargo test` — all pass

  ```
  Scenario: No task 22 stubs remain
    Tool: Bash
    Steps:
      1. `grep -rn "task 22\|Task 22" src/codegen/` — should return 0
      2. `cargo test` — all pass
    Expected Result: Zero stubs, all tests pass
    Evidence: .sisyphus/evidence/task-20-no-stubs.txt
  ```

  **Commit**: YES (groups with Wave 3)
  - Message: `fix(codegen): implement all remaining "task 22" stubs`
  - Files: `src/codegen/expressions.rs`, `src/codegen/values.rs`, and others as discovered

- [ ] 21. Overhaul C Runtime — Rename + Size-Specific Variants

  **What to do**:
  - RED: Write integration tests that call the renamed runtime functions (e.g., `random_int32()`, `random_int64()`, `print_int32()`, `string_to_int64()`). Tests MUST fail first (functions don't exist yet).
  - GREEN:
    1. Rename ALL C runtime functions — drop the `opal_` prefix:
       - `opal_take_input` → `take_input`
       - `opal_random_int32` → `random_int32` (and fix to return `int32_t`)
       - `opal_string_to_int32` → `string_to_int32` (and fix to return `int32_t`)
       - `opal_print_string` → `print_string`
       - `opal_print_int` → `print_int64` (rename to be explicit about size)
    2. Add size-specific variants for ALL supported integer types:
       - `random_int8`, `random_int16`, `random_int32`, `random_int64`
       - `random_uint8`, `random_uint16`, `random_uint32`, `random_uint64`
       - `string_to_int8`, `string_to_int16`, `string_to_int32`, `string_to_int64`
       - `string_to_uint8`, `string_to_uint16`, `string_to_uint32`, `string_to_uint64`
       - `print_int8`, `print_int16`, `print_int32`, `print_int64`
       - `print_uint8`, `print_uint16`, `print_uint32`, `print_uint64`
       - `print_float32`, `print_float64`
    3. Each function MUST use the correct C type (int8_t, int16_t, int32_t, int64_t, uint8_t, etc.)
    4. Update ALL codegen references to use new function names (drop `opal_` prefix)
    5. Update ALL type system builtin registrations to match new names
    6. Update the Rust runtime wrapper (`src/runtime.rs` / `src/runtime/`) to match
  - REFACTOR: Verify all call sites are updated. Run integration tests to confirm linking works.

  **Must NOT do**:
  - Do NOT add float random functions (not in spec)
  - Do NOT change the runtime's behavior — only rename and add size variants

  **Recommended Agent Profile**:
  - **Category**: `deep`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 3
  - **Blocks**: None
  - **Blocked By**: Task 6 (centralized type mapping)

  **References**:
  - `runtime/opal_runtime.c` — C runtime source file
  - `src/codegen/functions.rs` — where codegen calls runtime functions by name
  - `src/codegen/expressions.rs` — codegen references to runtime functions
  - `src/type_system/checker.rs` — builtin registrations referencing runtime names
  - `src/runtime.rs` / `src/runtime/` — Rust runtime wrapper
  - `src/stdlib/types.rs` — Rust stdlib helpers (for reference on size-specific patterns)

  **Acceptance Criteria**:
  - [ ] No "opal_" prefix on any runtime function
  - [ ] Size-specific variants exist for all integer types
  - [ ] Each function uses correct C type (int8_t, int32_t, uint64_t, etc.)
  - [ ] `cargo test --features integration` — all e2e tests pass (confirms linking works)
  - [ ] `grep -rn "opal_" runtime/opal_runtime.c` — only in filename, not function names

  ```
  Scenario: Runtime functions renamed and sized
    Tool: Bash
    Steps:
      1. `grep -n "opal_" runtime/opal_runtime.c | grep -v "^.*:.*//\|^.*:.*\*"` — check no opal_ prefixed functions
      2. `cargo test --features integration` — all pass
    Expected Result: No opal_ prefix, all sizes covered, linking works
    Evidence: .sisyphus/evidence/task-21-c-runtime.txt

  Scenario: Correct C types used
    Tool: Bash
    Steps:
      1. `grep -n "random_int32" runtime/opal_runtime.c` — verify signature contains `int32_t` return type, NOT `int64_t`
      2. `grep -n "random_int64" runtime/opal_runtime.c` — verify signature contains `int64_t` return type
      3. `grep -n "print_int32" runtime/opal_runtime.c` — verify parameter type is `int32_t`
      4. `grep -n "print_uint8" runtime/opal_runtime.c` — verify parameter type is `uint8_t`
      5. `grep -n "string_to_int32" runtime/opal_runtime.c` — verify return type is `int32_t`
      6. For each function, assert: the C type in the signature matches the size suffix in the function name (e.g., `_int32` → `int32_t`, `_uint64` → `uint64_t`)
    Expected Result: Every function's C type matches its name suffix. Zero mismatches.
    Failure Indicators: Any function where the size suffix doesn't match the C type (e.g., `random_int32` returning `int64_t`)
    Evidence: .sisyphus/evidence/task-21-types.txt
  ```

  **Commit**: YES (groups with Wave 3)
  - Message: `refactor(runtime): rename C functions, add size-specific variants, align types`
  - Files: `runtime/opal_runtime.c`, `src/codegen/functions.rs`, `src/codegen/expressions.rs`, `src/type_system/checker.rs`, `src/runtime.rs`

- [ ] 22. Fix Memory Layout Placeholders

  **What to do**:
  - RED: Write tests for `memory_layout()` that assert correct sizes for each CoreType (e.g., Int8 = 1 byte, Int32 = 4 bytes, Int64 = 8 bytes, Float64 = 8 bytes). Tests MUST fail first (currently returns placeholder values).
  - GREEN: Fix `src/type_system/memory.rs:29-46` to return accurate sizes and alignments for all CoreType variants. Use standard C/LLVM type sizes.
  - REFACTOR: Verify the hot-reload ABI checker uses these correct values. Remove "placeholder" comments.

  **Must NOT do**:
  - Do NOT add platform-specific sizes — use LLVM's standard sizes

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 3
  - **Blocks**: None
  - **Blocked By**: Task 6

  **References**:
  - `src/type_system/memory.rs:29-46` — placeholder memory layout
  - `src/type_system/types.rs` — CoreType enum (all variants that need sizes)
  - `src/hot_reload/` — ABI checker that uses memory layout

  **Acceptance Criteria**:
  - [ ] `memory_layout()` returns correct sizes for all types
  - [ ] No "placeholder" comments remain
  - [ ] `cargo test type_system` — all pass

  ```
  Scenario: Memory layout sizes correct
    Tool: Bash
    Steps:
      1. Run `cargo test memory` — layout tests pass
    Expected Result: Int8=1, Int16=2, Int32=4, Int64=8, Float32=4, Float64=8
    Evidence: .sisyphus/evidence/task-22-memory.txt
  ```

  **Commit**: YES (groups with Wave 3)
  - Message: `fix(types): implement correct memory layout sizes`
  - Files: `src/type_system/memory.rs`

- [ ] 23. Implement Hot Reload Production Components

  **What to do**:
  - RED: Write tests for production FileWatcher, ModuleLoader, and recovery handler. Tests MUST fail first (only mocks exist).
  - GREEN:
    1. Implement production `FileWatcher` (use `notify` crate or similar file system watcher) — `src/hot_reload/`
    2. Implement production `ModuleLoader` that actually loads `.so`/`.dylib`/`.dll` files — use `libloading` crate or `dlopen`
    3. Fix `versioned_module_name` to be platform-aware (`.so` on Linux, `.dylib` on macOS, `.dll` on Windows)
    4. Implement recovery handler in `src/hot_reload/recovery.rs` — currently a stub returning `Ok(())`
    5. Ensure the recovery handler actually rolls back to the previous module version on failure
  - REFACTOR: Remove mock-only implementations if production ones exist. Ensure platform detection is centralized.

  **Must NOT do**:
  - Do NOT remove mock implementations — they're needed for tests
  - Do NOT implement hot reload CLI integration (CLI is out of scope)

  **Recommended Agent Profile**:
  - **Category**: `deep`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 3
  - **Blocks**: None
  - **Blocked By**: Task 1

  **References**:
  - `src/hot_reload/` — hot reload module directory
  - `src/hot_reload/recovery.rs` — stub recovery handler
  - `src/hot_reload/` — MockFileWatcher, MockModuleLoader (for interface reference)
  - `Cargo.toml` — check if `notify` or `libloading` dependencies exist

  **Acceptance Criteria**:
  - [ ] Production FileWatcher implementation exists
  - [ ] Production ModuleLoader implementation exists
  - [ ] Recovery handler implements actual rollback logic
  - [ ] `versioned_module_name` uses correct extension per platform
  - [ ] `cargo test hot_reload` — all pass

  ```
  Scenario: Platform-aware module names
    Tool: Bash
    Steps:
      1. Run `cargo test hot_reload` — platform extension tests pass
    Expected Result: Linux=.so, macOS=.dylib, Windows=.dll
    Evidence: .sisyphus/evidence/task-23-hot-reload.txt
  ```

  **Commit**: YES (groups with Wave 3)
  - Message: `feat(hot-reload): implement production file watcher, module loader, and recovery`
  - Files: `src/hot_reload/recovery.rs`, `src/hot_reload/` (new files), `Cargo.toml` (if new deps needed)

- [ ] 24. Fix Formatter Quote Handling + Output Correctness

  **What to do**:
  - RED: Write formatter tests that format code with single-quoted strings and verify the output uses single quotes (not double). Write tests for match expression formatting that produces parseable output. Tests MUST fail first.
  - GREEN:
    1. Fix `src/formatter/rules.rs:83-175` — operator spacing rule currently only protects double-quoted strings but language uses single quotes. Change string detection to use single quotes.
    2. Fix `src/formatter/printer.rs` — match expressions currently output brace+comma syntax instead of colon-block. String literals output double quotes instead of single quotes. Fix both to produce valid, parseable Opalescent syntax.
  - REFACTOR: Add a round-trip test: format → parse → format → compare (output should be stable).

  **Must NOT do**:
  - Do NOT change the formatter's configuration options
  - Do NOT add new formatting rules beyond fixing existing ones

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 4 (with Tasks 25-30)
  - **Blocks**: None
  - **Blocked By**: Task 1

  **References**:
  - `src/formatter/rules.rs:83-175` — operator spacing with wrong string detection
  - `src/formatter/printer.rs` — match + string literal output issues
  - `src/formatter/` — formatter test files

  **Acceptance Criteria**:
  - [ ] Formatter outputs single-quoted strings
  - [ ] Match expressions format to parseable syntax
  - [ ] `cargo test formatter` — all pass

  ```
  Scenario: Single quotes preserved
    Tool: Bash
    Steps:
      1. Run `cargo test formatter` — quote handling tests pass
    Expected Result: Single quotes in, single quotes out
    Evidence: .sisyphus/evidence/task-24-formatter.txt
  ```

  **Commit**: YES (groups with Wave 4)
  - Message: `fix(formatter): use single quotes, fix match output to parseable syntax`
  - Files: `src/formatter/rules.rs`, `src/formatter/printer.rs`

- [ ] 25. Fix Package Manager Transitive Deps + Version Parsing

  **What to do**:
  - RED: Write tests for multi-clause version constraints (`>=0.5.0 <1.0.0`) and transitive dependency resolution. Tests MUST fail first.
  - GREEN:
    1. Fix `src/package_manager/resolver.rs:161-200` — version constraint parsing to support multiple clauses (e.g., `>=0.5.0 <1.0.0`).
    2. Fix `src/package_manager/resolver.rs:83-116` — implement transitive dependency resolution (resolve deps of deps).
    3. Fix `src/build_system/config.rs:172-209` — accept bare version strings (e.g., `"1.0.0"` without operator).
    4. Reconcile version constraint parsing between package manager and build system — ideally share the same parser.
  - REFACTOR: Add error messages for invalid version constraints.

  **Must NOT do**:
  - Do NOT implement a full SAT solver — simple highest-compatible-version resolution is sufficient
  - Do NOT implement real network requests

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 4
  - **Blocks**: None
  - **Blocked By**: Task 1

  **References**:
  - `src/package_manager/resolver.rs:161-200` — single-clause version parsing
  - `src/package_manager/resolver.rs:83-116` — top-level only dependency resolution
  - `src/build_system/config.rs:172-209` — version constraint parsing
  - `language-spec/requirements/modules.md` — module/package spec

  **Acceptance Criteria**:
  - [ ] `>=0.5.0 <1.0.0` parses as combined constraint
  - [ ] Transitive deps are resolved
  - [ ] Bare version `"1.0.0"` accepted by build system
  - [ ] `cargo test package_manager` and `cargo test build_system` — all pass

  ```
  Scenario: Multi-clause version constraints work
    Tool: Bash
    Steps:
      1. Run `cargo test package_manager` — version constraint tests pass
    Expected Result: Combined constraints parse and resolve correctly
    Evidence: .sisyphus/evidence/task-25-pkg-manager.txt
  ```

  **Commit**: YES (groups with Wave 4)
  - Message: `fix(pkg): support multi-clause version constraints and transitive deps`
  - Files: `src/package_manager/resolver.rs`, `src/build_system/config.rs`

- [ ] 26. LSP Stdio Transport + Document Sync

  **What to do**:
  - RED: Write tests for JSON-RPC message framing (Content-Length header parsing, message reading/writing) and document sync (DidOpen, DidChange, DidClose handlers). Tests MUST fail first.
  - GREEN:
    1. Implement JSON-RPC stdio transport — read `Content-Length` headers from stdin, parse JSON-RPC messages, write responses with proper headers to stdout.
    2. Implement document sync — DidOpen (store document), DidChange (update document), DidClose (remove document). Maintain a document store (HashMap<URI, DocumentState>).
    3. Fix Go-to-Definition to use actual document URI instead of hardcoded "file:///main.op".
    4. Fix Rename to work across all open documents (not single-file only).
  - REFACTOR: Ensure the LSP server can handle multiple documents. Add proper error responses for malformed requests.

  **Must NOT do**:
  - Do NOT implement additional LSP features beyond what the spec already claims
  - Do NOT implement LSP CLI wiring (CLI is out of scope)

  **Recommended Agent Profile**:
  - **Category**: `deep`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 4
  - **Blocks**: None
  - **Blocked By**: Task 1

  **References**:
  - `src/lsp.rs` and `src/lsp/` — LSP module directory
  - LSP spec: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/
  - `src/lsp/` — existing completion, hover, diagnostics implementations

  **Acceptance Criteria**:
  - [ ] JSON-RPC stdio transport reads/writes messages correctly
  - [ ] DidOpen/DidChange/DidClose maintain document state
  - [ ] Go-to-Definition uses actual document URI
  - [ ] `cargo test lsp` — all pass

  ```
  Scenario: LSP message framing works
    Tool: Bash
    Steps:
      1. Run `cargo test lsp` — transport and sync tests pass
    Expected Result: Messages are framed with Content-Length, documents are tracked
    Evidence: .sisyphus/evidence/task-26-lsp.txt
  ```

  **Commit**: YES (groups with Wave 4)
  - Message: `feat(lsp): implement stdio transport and document synchronization`
  - Files: `src/lsp.rs`, `src/lsp/` (multiple files)

- [ ] 27. Update README to Match Reality

  **What to do**:
  - Review each section of README.md against actual implementation state
  - Remove or mark as "planned" any features that don't work yet
  - Specifically:
    1. CLI Reference: `opal fmt` and `opal pkg` are not wired in app.rs — note this accurately
    2. LSP section: Note current limitations (after Task 26 fixes some)
    3. Hot Reload section: Note current state (after Task 23 fixes some)
    4. VS Code extension: Verify instructions match actual directory/file state
    5. Update Types section to list ALL 10 numeric types (int8-64, uint8-64, float32/64)
    6. Remove any claims about features that don't exist
  - After changes, verify every code example in README is valid Opalescent syntax

  **Must NOT do**:
  - Do NOT rewrite sections that are already accurate
  - Do NOT add marketing language or aspirational claims
  - Do NOT change the README structure/organization

  **Recommended Agent Profile**:
  - **Category**: `writing`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 4
  - **Blocks**: None
  - **Blocked By**: All implementation tasks (README should reflect final state)

  **References**:
  - `README.md` — current README
  - `src/app.rs` — actual CLI dispatch (to verify what commands work)
  - `src/lsp/` — LSP state (after Task 26)
  - `src/hot_reload/` — hot reload state (after Task 23)
  - `vscode-extension/` — verify directory exists and contents match README

  **Acceptance Criteria**:
  - [ ] Every feature claim in README is verifiably true
  - [ ] All 10 numeric types listed in Types section
  - [ ] No false claims about unimplemented CLI commands
  - [ ] Code examples use valid syntax

  ```
  Scenario: README accuracy — CLI claims
    Tool: Bash
    Steps:
      1. `grep -n "fmt\|format" src/app.rs` — check if `opal fmt` command is dispatched. If not found, README must NOT claim `opal fmt` works.
      2. `grep -n "pkg\|package" src/app.rs` — check if `opal pkg` command is dispatched. If not found, README must NOT claim `opal pkg` works.
      3. `grep -c "int8\|int16\|int32\|int64\|uint8\|uint16\|uint32\|uint64\|float32\|float64" README.md` — verify all 10 numeric types are mentioned
      4. `test -d vscode-extension && ls vscode-extension/package.json` — verify vscode-extension directory exists and has package.json as README claims
    Expected Result: All claims match. If fmt/pkg not dispatched in app.rs, README notes them as "planned". All 10 types listed. vscode-extension/ exists.
    Failure Indicators: README claims `opal fmt` works but grep finds no dispatch in app.rs. README omits any of the 10 types. vscode-extension/ directory missing.
    Evidence: .sisyphus/evidence/task-27-readme-cli.txt

  Scenario: README accuracy — code examples
    Tool: Bash
    Steps:
      1. `grep -n "opal lsp --stdio" README.md` — if present, verify `grep -n "lsp\|stdio" src/app.rs` shows this is dispatched, else README must note as "planned"
      2. `grep -c "int32\|int64\|float64\|string\|boolean\|void" README.md` — verify built-in types section is comprehensive
      3. `grep -n "opal_" README.md` — should return zero matches (old prefix should be gone after Task 21)
    Expected Result: Zero false claims about working CLI commands. Zero references to `opal_` prefix. All types documented.
    Failure Indicators: README references `opal_` prefix functions. README claims unimplemented CLI commands work without caveats.
    Evidence: .sisyphus/evidence/task-27-readme-examples.txt
  ```

  **Commit**: YES (groups with Wave 4)
  - Message: `docs(readme): align with actual implementation state`
  - Files: `README.md`

- [ ] 28. Fix Makefile.toml Broken Task

  **What to do**:
  - RED: Run `cargo make --list-all-steps 2>&1 | grep -i "install-deps"` to verify the task is listed, then run `cargo make --print-steps 2>&1` to see if it parses correctly. If `&&` in args causes a parse error, capture it. Do NOT run `cargo make install-deps-debian` itself (it requires sudo and mutates system state).
  - GREEN: Fix `Makefile.toml:1-15` — the `install-deps-debian` task uses `&&` in the `args` array which doesn't work with cargo-make. Change to use `script` or `command` with proper shell invocation.
  - REFACTOR: Verify the fixed task parses correctly via `cargo make --list-all-steps`.

  **Must NOT do**:
  - Do NOT add new make tasks
  - Do NOT execute `cargo make install-deps-debian` (requires sudo, mutates system)

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 4
  - **Blocks**: None
  - **Blocked By**: Task 1

  **References**:
  - `Makefile.toml:1-15` — broken task definition
  - cargo-make docs for correct task syntax

  **Acceptance Criteria**:
  - [ ] `install-deps-debian` task definition is syntactically valid for cargo-make
  - [ ] No `&&` in args array
  - [ ] `cargo make --list-all-steps` succeeds and lists `install-deps-debian`

  ```
  Scenario: Makefile task is valid
    Tool: Bash
    Steps:
      1. `cargo make --list-all-steps 2>&1 | grep -i "install-deps"` — verify task is listed without parse errors
      2. `grep -A5 "install-deps-debian" Makefile.toml` — verify no `&&` in args array
      3. `grep -c "script\|command" Makefile.toml | head -5` — verify proper cargo-make syntax is used (script or command key, not bare args with shell operators)
    Expected Result: Task listed in `--list-all-steps` output. No `&&` in args. Uses `script` or `command` key.
    Failure Indicators: `--list-all-steps` shows parse error. `&&` still in args array. Missing script/command key.
    Evidence: .sisyphus/evidence/task-28-makefile.txt
  ```

  **Commit**: YES (groups with Wave 4)
  - Message: `fix(build): fix cargo-make install-deps-debian task syntax`
  - Files: `Makefile.toml`

- [ ] 29. Clean Up Remaining TODOs/Stale Comments

  **What to do**:
  - Search ALL production source files for `TODO`, `FIXME`, `HACK`, `STUB`, `placeholder`, "not yet implemented", "Phase [0-5]" comments
  - For each occurrence:
    - If the TODO is now DONE (feature implemented): remove the comment
    - If the TODO is genuinely future work (Phase 6+): leave it but update the phase number
    - If the TODO is misleading (like `declarations.rs:319` "placeholder" comment on full implementation): remove or correct
  - Remove `ast.rs:429,436,437` Phase 4-5 TODOs if closure capture is implemented (Task 16)
  - Remove `type_system/errors.rs:376` `NotImplementedYet` if no longer used (after Task 14)
  - Remove `codegen/values.rs:12` placeholder comment if fixed (after Task 20)

  **Must NOT do**:
  - Do NOT remove TODOs for genuinely unimplemented future features
  - Do NOT add new TODOs

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 4
  - **Blocks**: None
  - **Blocked By**: All implementation tasks (need to know what's been fixed)

  **References**:
  - Full TODO inventory from audit (see draft)
  - `src/parser/declarations.rs:319` — misleading "placeholder" comment
  - `src/ast.rs:429,436,437` — Phase 4-5 TODOs
  - `src/type_system/checker.rs:448,465` — Phase 2 TODOs

  **Acceptance Criteria**:
  - [ ] `grep -rn "TODO\|FIXME\|HACK\|STUB" src/ --include="*.rs" | grep -v test | grep -v "Phase [6-9]"` — returns 0 or near-0 results
  - [ ] No misleading comments remain
  - [ ] All genuinely-future TODOs have accurate phase labels

  ```
  Scenario: Stale TODOs removed
    Tool: Bash
    Steps:
      1. `grep -rn "TODO\|FIXME" src/ --include="*.rs" | grep -v test` — count results
    Expected Result: Zero or minimal results (only genuine future work)
    Evidence: .sisyphus/evidence/task-29-todo-cleanup.txt
  ```

  **Commit**: YES (groups with Wave 4)
  - Message: `chore: clean up stale TODOs and misleading comments`
  - Files: multiple

- [ ] 30. Track Learnings to Serena Memory

  **What to do**:
  - Write learnings from this audit to Serena memory for future reference
  - Topics to cover:
    1. Audit methodology (8 parallel agents, subsystem-by-subsystem)
    2. Common issues found (stale ignores, duplicated code, naming mismatches)
    3. Design decisions made (int64 default, no shorthand types, C runtime naming)
    4. Architecture insights (ast_type_to_core_type duplication risk, parser supports more than tests assume)
  - Update existing memories if relevant (project_overview, codebase_structure)

  **Must NOT do**:
  - Do NOT overwrite existing memories without reading them first

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 4
  - **Blocks**: None
  - **Blocked By**: All tasks (needs final state to write accurate learnings)

  **References**:
  - `.sisyphus/drafts/correctness-audit.md` — audit findings
  - Existing Serena memories: project_overview, codebase_structure, style_and_conventions

  **Acceptance Criteria**:
  - [ ] Serena memory updated with audit learnings
  - [ ] Existing memories updated where relevant

  ```
  Scenario: Learnings persisted
    Tool: Bash / serena_list_memories
    Steps:
      1. Verify learnings memory exists
      2. Read it to confirm content is accurate
    Expected Result: Audit learnings are accessible via Serena
    Evidence: .sisyphus/evidence/task-30-learnings.txt
  ```

  **Commit**: NO (no source code changes)

---

## Final Verification Wave (MANDATORY — after ALL implementation tasks)

> 4 review agents run in PARALLEL. ALL must APPROVE. Present consolidated results to user and get explicit "okay" before completing.

- [ ] F1. **Plan Compliance Audit** — `oracle`
  Read the plan end-to-end. For each "Must Have": verify implementation exists (read file, run command). For each "Must NOT Have": search codebase for forbidden patterns — reject with file:line if found. Check evidence files exist in .sisyphus/evidence/. Compare deliverables against plan.
  Output: `Must Have [N/N] | Must NOT Have [N/N] | Tasks [N/N] | VERDICT: APPROVE/REJECT`

- [ ] F2. **Code Quality Review** — `unspecified-high`
  Run `cargo build --release 2>&1`, `cargo test 2>&1`, `cargo test --features integration 2>&1`. Review all changed files for: `as any` equivalent casts, empty error handlers, commented-out code, unused imports, `unreachable!()` in non-test code. Check AI slop: excessive comments, over-abstraction, generic variable names.
  Output: `Build [PASS/FAIL] | Tests [N pass/N fail] | Integration [N pass/N fail] | Files [N clean/N issues] | VERDICT`

- [ ] F3. **Real Manual QA** — `unspecified-high`
  Start from clean state. Compile and run each test-project (hello-world, fib-recursive, fib-iterative, simple-quiz). Verify outputs match expectations. Test edge cases: empty source file, syntax errors, type errors. Save evidence to `.sisyphus/evidence/final-qa/`.
  Output: `Test Projects [N/N pass] | Edge Cases [N tested] | VERDICT`

- [ ] F4. **Scope Fidelity Check** — `deep`
  For each task: read "What to do", read actual diff (git log/diff). Verify 1:1 — everything in spec was built, nothing beyond spec was built. Check "Must NOT do" compliance. Detect cross-task contamination. Flag unaccounted changes.
  Output: `Tasks [N/N compliant] | Contamination [CLEAN/N issues] | Unaccounted [CLEAN/N files] | VERDICT`

---

## Commit Strategy

- Commit after each wave completes with all tests passing
- Wave 0: `chore: establish test baseline`
- Wave 1: `fix(foundation): UTF-8 offsets, token cleanup, type centralization, spec update`
- Wave 2: `fix(parser+types): cast/array parsing, generic lambdas, constraint solver, closure capture`
- Wave 3: `fix(codegen+runtime): string interp UB, power op, argv, C runtime overhaul, hot reload`
- Wave 4: `fix(tooling+docs): formatter, pkg manager, LSP, README, cleanup`

---

## Success Criteria

### Verification Commands
```bash
cargo test                          # Expected: ALL pass, 0 ignored (or only future-phase ignores)
cargo test --features integration   # Expected: ALL pass
cargo build --release 2>&1          # Expected: 0 errors, 0 warnings
grep -rn "TODO\|FIXME\|HACK\|STUB" src/ --include="*.rs" | grep -v test | grep -v "Phase [6-9]"  # Expected: 0 results
```

### Final Checklist
- [ ] All "Must Have" present
- [ ] All "Must NOT Have" absent
- [ ] All tests pass (unit + integration)
- [ ] No stale `#[ignore]` markers
- [ ] README sections match actual CLI behavior
- [ ] C runtime functions have no "opal_" prefix
- [ ] All 10 numeric types available end-to-end
- [ ] Language spec states int64 as default
