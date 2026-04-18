# Pure Keyword: Complete Enforcement Implementation

## TL;DR

> **Quick Summary**: Finish the `pure` keyword enforcement in the type checker — expand impure builtin detection, add transitive purity checking for user-defined functions, reject `pure entry`, and produce clear diagnostic errors via a dedicated `PurityViolation` error variant.
>
> **Deliverables**:
> - Dedicated `PurityViolation` error variant with miette diagnostics (error code, help text, source labels)
> - Complete impure stdlib list (20 builtins: all print_*, random_*, take_input)
> - `is_pure` tracking on `SymbolInfo` for user-defined function purity lookup
> - Transitive purity enforcement (pure function cannot call non-pure user function)
> - `pure entry` combination rejected at type-check time
> - Lambda purity inheritance documented and tested
> - VSCode syntax highlighting for `pure` keyword
> - Comprehensive TDD tests (unit + integration) for every violation and allowed case
>
> **Estimated Effort**: Medium
> **Parallel Execution**: YES — 3 waves
> **Critical Path**: Task 1 (error variant) → Task 3 (SymbolInfo) → Task 5 (transitive enforcement) → Task 7 (integration tests) → F1-F4

---

## Context

### Original Request
"Finish implementation of the pure keyword. It should be usable and enforced with clear errors on purity violations."

### Interview Summary
**Key Discussions**:
- **Purity semantics resolved from spec**: Language spec examples (`automatic_regions.op:49`, `value_semantics.op:67`, `array_helpers.op`) definitively show `pure` functions using `let mutable`, assignments, and `.push()`. **Pure = no external side effects (I/O, calling impure functions). Local mutation is fully allowed.**
- **Scope reduction**: Removed 3 originally-proposed tasks (reject `let mutable`, reject assignments, reject array mutation in pure functions) — spec contradicts these.

**Research Findings**:
- Foundation exists: lexer, parser, AST, and basic type checker enforcement (3 impure builtins checked)
- `IMPURE_STDLIB_FUNCTIONS` at `call_resolution.rs:22` only has `["print", "take_input", "random_int32"]` — 17 impure builtins missing
- `TypeChecker` has working `function_modifier_stack` and `current_function_is_pure()` — infrastructure is solid
- `SymbolInfo` lacks purity tracking — needed for transitive enforcement
- Collection member calls (`.push()`, `.len()`) are resolved through a different code path (`resolve_collection_member_call`) and must NOT be blocked
- Lambda expressions do NOT push their own modifier context — they implicitly inherit the enclosing function's purity. This is correct behavior but fragile.
- `CoreType::Function` has 131 construction sites across 21 files — modifying it is unacceptably risky

### Metis Review
**Identified Gaps** (addressed):
- **CoreType::Function modification removed** — Use `SymbolInfo`-based tracking (~15 sites) instead of type-level purity (131 sites). Covers all named-function enforcement with 10% of the effort.
- **Higher-order purity deferred** — Function type syntax `f(T): U` has no purity annotation. Enforcing purity on callback parameters would require syntax changes. Only named-call enforcement is in scope.
- **Collection member calls explicitly safe** — `.push()`, `.pop()`, `.len()` etc. go through `resolve_collection_member_call()`, not `type_check_call_expr_impl()`. They are local-mutation-only and must NOT be blocked. Guardrail test required.
- **Lambda purity inheritance is implicit** — `type_check_lambda_expr` doesn't call `enter_function_modifier_context()`. Need code comment + test to formalize.
- **Complete impure list provided** — 20 entries verified against `size_specific_builtins.rs`
- **Error pattern documented** — `TypeError` uses `thiserror` + `miette` with `#[error()]`, `#[diagnostic(code(), help())]`, `#[label()]`

---

## Work Objectives

### Core Objective
Complete `pure` keyword enforcement so that purity violations produce clear, actionable compiler errors — covering impure stdlib calls, transitive impurity through user-defined functions, and the `pure entry` conflict.

### Concrete Deliverables
- `src/type_system/errors.rs`: New `PurityViolation` variant on `TypeError`
- `src/type_system/symbol_table.rs`: `is_pure: bool` field on `SymbolInfo`
- `src/type_system/checker/declarations.rs`: Set `is_pure` during registration + reject `pure entry`
- `src/type_system/checker/call_resolution.rs`: Expanded impure list + transitive enforcement
- `src/type_system/checker/expressions.rs`: Code comment documenting lambda purity inheritance
- `src/type_system/tests.rs`: Unit tests for all violation types + allowed cases
- `src/type_system/test_integration.rs`: Integration tests through full parse pipeline
- `vscode-extension/syntaxes/opalescent.tmLanguage.json`: `pure` in keyword pattern

### Definition of Done
- [ ] `cargo test` — all existing + new unit tests pass
- [ ] `cargo test --features integration` — all integration tests pass
- [ ] `cargo clippy --all-targets -- -D warnings` — zero warnings
- [ ] Pure function calling `print` → `PurityViolation` error with code + help text
- [ ] Pure function calling all 20 impure builtins → rejected
- [ ] Pure function calling non-pure user function → `PurityViolation` error
- [ ] Pure function calling pure user function → compiles successfully
- [ ] `pure entry main` → clear error rejecting the combination
- [ ] Lambda inside pure function calling `print` → rejected
- [ ] Pure function using `let mutable`, assignments, `.push()` → compiles successfully

### Must Have
- Dedicated `PurityViolation` error variant (not reuse of `InvalidOperation`)
- Complete impure stdlib list (all 20 builtins)
- Transitive purity enforcement for named user-defined function calls
- `pure entry` rejection
- Tests for every positive and negative case
- VSCode highlighting

### Must NOT Have (Guardrails)
- **DO NOT modify `CoreType::Function`** — purity is tracked via `SymbolInfo`, NOT via type system. Do not touch `types.rs`, `unification.rs`, `substitution.rs`, or `hot_reload/abi.rs`.
- **DO NOT block collection member calls** (`.push()`, `.pop()`, `.len()`, `.contains()`, etc.) inside pure functions — spec explicitly allows local mutation
- **DO NOT enforce higher-order purity** — do not block calls through function-typed parameters/variables (e.g., a callback passed to a pure function). This requires syntax changes and is out of scope.
- **DO NOT add purity inference** — only explicitly `pure`-annotated functions are pure
- **DO NOT add purity to type equality/hashing** — this would cascade into unification and ABI hashing
- **DO NOT over-comment or over-abstract** — keep changes minimal and focused

---

## Verification Strategy (MANDATORY)

> **ZERO HUMAN INTERVENTION** — ALL verification is agent-executed. No exceptions.

### Test Decision
- **Infrastructure exists**: YES (`cargo test`, extensive test suite)
- **Automated tests**: TDD (RED-GREEN-REFACTOR)
- **Framework**: Rust built-in `#[test]` with `cargo test`
- **Each task**: Write failing test FIRST → implement to make it pass → refactor

### QA Policy
Every task MUST include agent-executed QA scenarios.
Evidence saved to `.sisyphus/evidence/task-{N}-{scenario-slug}.{ext}`.

- **Type checker changes**: Use Bash (`cargo test`) — run specific test, assert pass/fail
- **Error messages**: Use Bash (`cargo test`) — assert error variant and message content
- **VSCode extension**: Use Bash (grep/jq) — verify JSON structure contains `pure` keyword

---

## Execution Strategy

### Parallel Execution Waves

```
Wave 1 (Foundation — all independent, start immediately):
├── Task 1: Add PurityViolation error variant [quick]
├── Task 2: Expand IMPURE_STDLIB_FUNCTIONS to 20 entries [quick]
├── Task 3: Add is_pure to SymbolInfo + set during registration [quick]
└── Task 4: VSCode syntax highlighting for pure [quick]

Wave 2 (Core enforcement — depends on Wave 1):
├── Task 5: Transitive purity enforcement (depends: 1, 2, 3) [deep]
├── Task 6: Reject pure entry combination (depends: 1) [quick]
└── Task 7: Lambda purity inheritance test + comment (depends: 1, 2) [quick]

Wave 3 (Integration tests — depends on Wave 2):
└── Task 8: Integration tests through full parse pipeline (depends: 5, 6, 7) [unspecified-high]

Wave FINAL (After ALL tasks — 4 parallel reviews, then user okay):
├── Task F1: Plan compliance audit (oracle)
├── Task F2: Code quality review (unspecified-high)
├── Task F3: Real manual QA (unspecified-high)
└── Task F4: Scope fidelity check (deep)
-> Present results -> Get explicit user okay
```

### Dependency Matrix

| Task | Depends On | Blocks | Wave |
|------|-----------|--------|------|
| 1 | — | 5, 6, 7 | 1 |
| 2 | — | 5, 7 | 1 |
| 3 | — | 5 | 1 |
| 4 | — | — | 1 |
| 5 | 1, 2, 3 | 8 | 2 |
| 6 | 1 | 8 | 2 |
| 7 | 1, 2 | 8 | 2 |
| 8 | 5, 6, 7 | F1-F4 | 3 |
| F1-F4 | 8 | — | FINAL |

### Agent Dispatch Summary

- **Wave 1**: **4 tasks** — T1 → `quick`, T2 → `quick`, T3 → `quick`, T4 → `quick`
- **Wave 2**: **3 tasks** — T5 → `deep`, T6 → `quick`, T7 → `quick`
- **Wave 3**: **1 task** — T8 → `unspecified-high`
- **Wave FINAL**: **4 tasks** — F1 → `oracle`, F2 → `unspecified-high`, F3 → `unspecified-high`, F4 → `deep`

---

## TODOs

- [x] 1. Add `PurityViolation` Error Variant to `TypeError`

  **What to do**:
  - **RED**: Write a unit test in `src/type_system/tests.rs` that constructs a `TypeError::PurityViolation` variant with fields `callee_name`, `reason`, and `span`. Assert the variant exists, has the expected `Display` output, and the diagnostic code is `opalescent::type_system::purity_violation`. This test will fail to compile because the variant doesn't exist yet.
  - **GREEN**: Add the `PurityViolation` variant to the `TypeError` enum in `src/type_system/errors.rs`. Follow the exact pattern of existing variants (e.g., `InvalidOperation` at line ~300). Include:
    - `#[error("cannot call impure function '{callee_name}' from pure function context")]`
    - `#[diagnostic(code(opalescent::type_system::purity_violation), help("pure functions cannot perform I/O or call impure functions — remove the 'pure' modifier or move the impure call outside"))]`
    - Fields: `callee_name: String`, `reason: String`, `#[label("{reason}")] span: SourceSpan`
  - **REFACTOR**: Verify the variant compiles cleanly. Run `cargo clippy`.

  **Must NOT do**:
  - Do NOT modify any existing error variant
  - Do NOT add more than one new variant
  - Do NOT touch `CoreType` or `types.rs`

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Single-file addition of one enum variant following established pattern
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1 (with Tasks 2, 3, 4)
  - **Blocks**: Tasks 5, 6, 7
  - **Blocked By**: None (can start immediately)

  **References**:

  **Pattern References** (existing code to follow):
  - `src/type_system/errors.rs:10-24` — `TypeError` enum declaration with `#[derive(Error, Debug, Clone, PartialEq, Eq, Diagnostic)]`; follow the derive chain exactly
  - `src/type_system/errors.rs:296-312` — `InvalidOperation` variant is the closest pattern match for the new `PurityViolation` variant; copy its structure (fields + miette attributes)

  **Test References** (testing patterns to follow):
  - `src/type_system/tests.rs:500-553` — `test_type_check_pure_function_rejects_print_call` shows how to construct a pure function AST, call `type_check_program`, and pattern-match on the resulting `TypeError` variant

  **WHY Each Reference Matters**:
  - `errors.rs:10-24` — The derive macros are critical; missing `Diagnostic` would break miette rendering, missing `PartialEq`/`Eq` would break test assertions
  - `errors.rs:296-312` — Shows the exact attribute syntax for `#[diagnostic(code(...), help(...))]` and `#[label("...")]` that miette requires
  - `tests.rs:500-553` — Shows the assertion pattern `errors.iter().any(|error| matches!(error, &TypeError::...))` needed for new tests

  **Acceptance Criteria**:

  **TDD:**
  - [ ] Test `test_purity_violation_variant_exists` compiles and passes
  - [ ] `cargo test test_purity_violation` → PASS
  - [ ] `cargo clippy --all-targets -- -D warnings` → 0 warnings

  **QA Scenarios (MANDATORY):**

  ```
  Scenario: PurityViolation variant compiles and formats correctly
    Tool: Bash (cargo test)
    Preconditions: New variant added to TypeError enum
    Steps:
      1. Run `cargo test test_purity_violation -- --nocapture 2>&1`
      2. Assert exit code is 0
      3. Assert output contains "test ... ok"
    Expected Result: Test passes, variant exists with correct Display output
    Failure Indicators: Compilation error mentioning "PurityViolation", test failure
    Evidence: .sisyphus/evidence/task-1-variant-compiles.txt

  Scenario: Clippy accepts new variant without warnings
    Tool: Bash (cargo clippy)
    Preconditions: New variant added
    Steps:
      1. Run `cargo clippy --all-targets -- -D warnings 2>&1`
      2. Assert exit code is 0
    Expected Result: Zero warnings, clean clippy pass
    Failure Indicators: Any warning about unused fields, missing derives, etc.
    Evidence: .sisyphus/evidence/task-1-clippy-clean.txt
  ```

  **Commit**: YES (groups with Wave 1)
  - Message: `feat(type_system): add PurityViolation error variant with miette diagnostics`
  - Files: `src/type_system/errors.rs`, `src/type_system/tests.rs`
  - Pre-commit: `cargo test`

---

- [x] 2. Expand `IMPURE_STDLIB_FUNCTIONS` to Complete 20-Entry List

  **What to do**:
  - **RED**: Write unit tests in `src/type_system/tests.rs` that construct pure functions calling `print_int32`, `random_uint64`, and `print_string`. Assert each produces an error. These will fail because these builtins aren't in the impure list yet.
  - **GREEN**: Replace the 3-entry `IMPURE_STDLIB_FUNCTIONS` array at `src/type_system/checker/call_resolution.rs:22` with the complete 20-entry list:
    ```rust
    const IMPURE_STDLIB_FUNCTIONS: &[&str] = &[
        "print", "take_input",
        "print_int8", "print_int16", "print_int32", "print_int64",
        "print_uint8", "print_uint16", "print_uint32", "print_uint64",
        "print_float32", "print_float64", "print_string",
        "random_int8", "random_int16", "random_int32",
        "random_uint8", "random_uint16", "random_uint32", "random_uint64",
    ];
    ```
  - **REFACTOR**: Verify the list is alphabetically grouped (print_*, random_*) for maintainability. Ensure no pure builtins are accidentally included (all `string_to_*` and `*_to_string` conversions must NOT be listed — they are pure data transformations).

  **Must NOT do**:
  - Do NOT include `string_to_*` or `*_to_string` builtins — these are pure
  - Do NOT change the purity check logic itself (just the list)
  - Do NOT modify `size_specific_builtins.rs`

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Single constant replacement + a few test additions
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1 (with Tasks 1, 3, 4)
  - **Blocks**: Tasks 5, 7
  - **Blocked By**: None (can start immediately)

  **References**:

  **Pattern References**:
  - `src/type_system/checker/call_resolution.rs:22` — Current `IMPURE_STDLIB_FUNCTIONS` array (3 entries); replace this entirely
  - `src/type_system/checker/call_resolution.rs:100-114` — The purity check that reads this list; no modification needed here but understand how the list is consumed

  **API/Type References**:
  - `src/type_system/checker/size_specific_builtins.rs` — All size-specific builtins registered here; cross-reference to verify the complete list of print_* and random_* function names

  **Test References**:
  - `src/type_system/tests.rs:500-553` — Existing `test_type_check_pure_function_rejects_print_call`; new tests follow this exact pattern but call different builtins

  **WHY Each Reference Matters**:
  - `call_resolution.rs:22` — This is the exact line to modify; the constant name and type must stay the same
  - `size_specific_builtins.rs` — Authoritative source for builtin names; verifying against this prevents typos in the impure list
  - `tests.rs:500-553` — Provides the exact AST construction pattern for testing impure builtin rejection

  **Acceptance Criteria**:

  **TDD:**
  - [ ] Tests for `print_int32`, `random_uint64`, `print_string` in pure context → FAIL initially
  - [ ] After expanding list → all 3 new tests PASS
  - [ ] Existing `test_type_check_pure_function_rejects_print_call` still passes (no regression)

  **QA Scenarios (MANDATORY):**

  ```
  Scenario: Expanded impure list catches size-specific print builtins
    Tool: Bash (cargo test)
    Preconditions: IMPURE_STDLIB_FUNCTIONS expanded to 20 entries
    Steps:
      1. Run `cargo test test_type_check_pure_function_rejects_print -- --nocapture 2>&1`
      2. Assert exit code is 0
      3. Assert output contains multiple "test ... ok" lines
    Expected Result: All print-variant tests pass (pure function calling print_int32/print_string etc. → error)
    Failure Indicators: Any test failure, "not found" for test names
    Evidence: .sisyphus/evidence/task-2-impure-list-expanded.txt

  Scenario: Pure builtins NOT blocked (string_to_int32 etc.)
    Tool: Bash (cargo test)
    Preconditions: IMPURE_STDLIB_FUNCTIONS does NOT include string_to_* builtins
    Steps:
      1. Write a test: pure function calling `string_to_int32` should compile successfully
      2. Run `cargo test test_type_check_pure_function_allows_string_to -- --nocapture 2>&1`
      3. Assert exit code is 0
    Expected Result: Pure function calling conversion builtins compiles without error
    Failure Indicators: Error about purity violation for string_to_int32
    Evidence: .sisyphus/evidence/task-2-pure-builtins-allowed.txt
  ```

  **Commit**: YES (groups with Wave 1)
  - Message: `feat(type_system): expand impure stdlib list to 20 builtins`
  - Files: `src/type_system/checker/call_resolution.rs`, `src/type_system/tests.rs`
  - Pre-commit: `cargo test`

- [x] 3. Add `is_pure` Field to `SymbolInfo` and Set During Function Registration

  **What to do**:
  - **RED**: Write a unit test that registers a function with `FunctionModifier::Pure`, looks it up in the symbol table, and asserts `symbol_info.is_pure == true`. Also test that a non-pure function has `is_pure == false`. These tests will fail because the field doesn't exist.
  - **GREEN**:
    1. Add `pub is_pure: bool` field to `SymbolInfo` struct in `src/type_system/symbol_table.rs` (after `read_count: usize` at line 64).
    2. Update ALL existing `SymbolInfo { ... }` construction sites to include `is_pure: false`. There are ~15 sites across `declarations.rs` and potentially other files. Use `ast_grep_search` with pattern `SymbolInfo {` to find them all.
    3. In `src/type_system/checker/declarations.rs`, in the `register_declaration_signature` method at line ~275, set `is_pure: true` when the function's modifiers contain `FunctionModifier::Pure`. The modifiers are available through the `Decl::Function` match arm (line 178). Access them via the matched `modifiers` field (which is NOT currently destructured — you'll need to add `ref modifiers` to the pattern).
  - **REFACTOR**: Verify all construction sites are updated (compilation will catch any missed ones). Run full test suite.

  **Must NOT do**:
  - Do NOT add purity to `CoreType::Function` — only `SymbolInfo`
  - Do NOT modify `unification.rs`, `substitution.rs`, `types.rs`, or `hot_reload/abi.rs`
  - Do NOT change the behavior of `current_function_is_pure()` — that uses the modifier stack for the *current* function, not the callee

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Adding one boolean field to a struct + updating construction sites (mechanical)
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1 (with Tasks 1, 2, 4)
  - **Blocks**: Task 5
  - **Blocked By**: None (can start immediately)

  **References**:

  **Pattern References**:
  - `src/type_system/symbol_table.rs:47-65` — `SymbolInfo` struct definition; add `is_pure: bool` here following the pattern of existing boolean fields (`is_let_binding`, `is_mutable`)
  - `src/type_system/checker/declarations.rs:274-284` — Function signature registration that creates `SymbolInfo`; this is the PRIMARY site where `is_pure: true` must be set for pure functions
  - `src/type_system/checker/declarations.rs:178-188` — `Decl::Function` pattern match in `register_declaration_signature`; need to add `ref modifiers` to destructure

  **API/Type References**:
  - `src/ast/modifiers.rs` — `FunctionModifier::Pure` variant definition; use `.contains(&FunctionModifier::Pure)` or `.iter().any(|m| matches!(m, FunctionModifier::Pure))` to check

  **Test References**:
  - `src/type_system/tests.rs:500-553` — Shows how to create `Decl::Function` with `modifiers: vec![FunctionModifier::Pure]` and call `type_check_program`

  **WHY Each Reference Matters**:
  - `symbol_table.rs:47-65` — Must add the field in the right position and with correct visibility (`pub`)
  - `declarations.rs:274-284` — This is where function-level `SymbolInfo` is created; `is_pure` must be set here from modifiers
  - `declarations.rs:178-188` — The pattern match doesn't currently extract `modifiers`; need to add it to access purity info

  **Acceptance Criteria**:

  **TDD:**
  - [ ] Test `test_symbol_info_tracks_purity` compiles and passes
  - [ ] `cargo test` → all existing tests still pass (no regressions from adding `is_pure: false` everywhere)
  - [ ] `cargo clippy --all-targets -- -D warnings` → 0 warnings

  **QA Scenarios (MANDATORY):**

  ```
  Scenario: SymbolInfo.is_pure correctly set for pure functions
    Tool: Bash (cargo test)
    Preconditions: is_pure field added to SymbolInfo
    Steps:
      1. Run `cargo test test_symbol_info_tracks_purity -- --nocapture 2>&1`
      2. Assert exit code is 0
    Expected Result: Pure function registration sets is_pure=true, non-pure sets is_pure=false
    Failure Indicators: Test failure, compilation error about missing field
    Evidence: .sisyphus/evidence/task-3-symbol-info-purity.txt

  Scenario: No regression from adding is_pure field
    Tool: Bash (cargo test)
    Preconditions: All SymbolInfo construction sites updated with is_pure: false
    Steps:
      1. Run `cargo test 2>&1 | tail -5`
      2. Assert "test result: ok" in output
      3. Assert zero failures
    Expected Result: All existing tests pass unchanged
    Failure Indicators: Any test failure, compilation error about missing field
    Evidence: .sisyphus/evidence/task-3-no-regression.txt
  ```

  **Commit**: YES (groups with Wave 1)
  - Message: `feat(type_system): add is_pure tracking to SymbolInfo`
  - Files: `src/type_system/symbol_table.rs`, `src/type_system/checker/declarations.rs`, `src/type_system/tests.rs` (+ any other files with SymbolInfo construction)
  - Pre-commit: `cargo test`

---

- [x] 4. Add `pure` to VSCode Syntax Highlighting

  **What to do**:
  - Add `pure` to the keyword declaration pattern in `vscode-extension/syntaxes/opalescent.tmLanguage.json` at line 69.
  - Change `"\\b(f|type|module|import|export|entry|public)\\b"` to `"\\b(f|type|module|import|export|entry|public|pure)\\b"`.

  **Must NOT do**:
  - Do NOT modify any other pattern in the tmLanguage file
  - Do NOT add `pure` to multiple pattern groups — it belongs with declaration keywords alongside `entry` and `public`

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Single regex change in one JSON file
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1 (with Tasks 1, 2, 3)
  - **Blocks**: None
  - **Blocked By**: None (can start immediately)

  **References**:

  **Pattern References**:
  - `vscode-extension/syntaxes/opalescent.tmLanguage.json:65-78` — The `keyword` section with three pattern groups: declaration, control, and other. `pure` goes in the declaration group (line 69) alongside `entry` and `public`.

  **WHY Each Reference Matters**:
  - `tmLanguage.json:65-78` — The exact line to modify; `pure` is a declaration modifier like `entry` and `public`, not a control flow keyword or an `other` keyword

  **Acceptance Criteria**:

  **TDD:** N/A (JSON config, no Rust tests)

  **QA Scenarios (MANDATORY):**

  ```
  Scenario: pure keyword appears in tmLanguage declaration pattern
    Tool: Bash (grep)
    Preconditions: tmLanguage.json modified
    Steps:
      1. Run `grep -o 'pure' vscode-extension/syntaxes/opalescent.tmLanguage.json`
      2. Assert output contains "pure"
      3. Run `grep 'keyword.declaration' vscode-extension/syntaxes/opalescent.tmLanguage.json -A 1`
      4. Assert the match line contains `pure`
    Expected Result: `pure` present in keyword.declaration pattern
    Failure Indicators: grep returns empty, `pure` in wrong pattern group
    Evidence: .sisyphus/evidence/task-4-vscode-highlighting.txt

  Scenario: tmLanguage JSON is valid
    Tool: Bash (python/jq)
    Preconditions: tmLanguage.json modified
    Steps:
      1. Run `python3 -c "import json; json.load(open('vscode-extension/syntaxes/opalescent.tmLanguage.json'))" 2>&1`
      2. Assert exit code is 0
    Expected Result: Valid JSON (no syntax errors from the edit)
    Failure Indicators: JSONDecodeError
    Evidence: .sisyphus/evidence/task-4-json-valid.txt
  ```

  **Commit**: YES (groups with Wave 1)
  - Message: `feat(vscode): add pure keyword to syntax highlighting`
  - Files: `vscode-extension/syntaxes/opalescent.tmLanguage.json`
  - Pre-commit: N/A (JSON file, no build step)

- [x] 5. Implement Transitive Purity Enforcement + Migrate to PurityViolation Error

  **What to do**:
  - **RED**: Write unit tests in `src/type_system/tests.rs`:
    1. `test_pure_function_cannot_call_non_pure_user_function`: Pure function `a` calls non-pure function `b` → expect `PurityViolation` error
    2. `test_pure_function_can_call_pure_user_function`: Pure function `a` calls pure function `b` → expect success
    3. `test_pure_function_allows_local_mutation`: Pure function with `let mutable x = 0`, `x = x + 1`, and array `.push()` → expect success
    4. `test_pure_function_allows_collection_member_calls`: Pure function calling `.push()`, `.len()` on local array → expect success (guardrail test)
    These will fail because transitive enforcement doesn't exist yet.
  - **GREEN**:
    1. In `src/type_system/checker/call_resolution.rs`, modify `type_check_call_expr_impl()` starting at line 100:
       - **First**: Migrate the existing `IMPURE_STDLIB_FUNCTIONS` check (lines 100-114) to use `TypeError::PurityViolation` instead of `TypeError::InvalidOperation`. Set `callee_name` to the function name and `reason` to `"this function performs I/O or has side effects"`.
       - **Second**: After the stdlib check, add transitive enforcement: If `self.current_function_is_pure()` and callee is `Expr::Identifier`, look up the callee in `self.symbol_table.lookup(&name)`. If found and `!symbol_info.is_pure`, emit `PurityViolation` with `reason: "function '{name}' is not marked 'pure'"`.
       - **Important**: Only check `Expr::Identifier` callees. Do NOT check `Expr::Member` (collection methods) or calls through function-typed variables. Stdlib builtins without a symbol table entry that are NOT in `IMPURE_STDLIB_FUNCTIONS` should be assumed pure (they are math/conversion builtins).
    2. Handle the edge case: If a function is NOT in `IMPURE_STDLIB_FUNCTIONS` and NOT in the symbol table (i.e., it's a non-impure stdlib builtin), allow the call. Only reject if it IS in the symbol table AND is not pure.
  - **REFACTOR**: Ensure the check order is clear: stdlib impure check → symbol table transitive check → allow. Clean up any duplication.

  **Must NOT do**:
  - Do NOT block `Expr::Member` calls (`.push()`, `.len()`, etc.) — these are local mutation, not side effects
  - Do NOT block calls through function-typed parameters/variables — higher-order purity is out of scope
  - Do NOT modify `CoreType::Function`
  - Do NOT add purity inference — only check explicitly registered `is_pure` on `SymbolInfo`

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: Core enforcement logic with multiple edge cases, requires understanding of call resolution and symbol table interaction
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES (with Tasks 6, 7 in Wave 2)
  - **Parallel Group**: Wave 2
  - **Blocks**: Task 8
  - **Blocked By**: Tasks 1, 2, 3

  **References**:

  **Pattern References**:
  - `src/type_system/checker/call_resolution.rs:100-114` — Existing purity check; this is the code to extend. Currently checks `IMPURE_STDLIB_FUNCTIONS` and emits `InvalidOperation`. Migrate to `PurityViolation` and add transitive check after it.
  - `src/type_system/checker/call_resolution.rs:22` — `IMPURE_STDLIB_FUNCTIONS` constant (expanded to 20 entries by Task 2)
  - `src/type_system/checker.rs:923-932` — `current_function_is_pure()` method; already works correctly, just call it

  **API/Type References**:
  - `src/type_system/symbol_table.rs:47-65` — `SymbolInfo` struct with `is_pure` field (added by Task 3)
  - `src/type_system/symbol_table.rs` — `SymbolTable::lookup(&str) -> Option<&SymbolInfo>` method for looking up callee purity
  - `src/type_system/errors.rs` — `TypeError::PurityViolation` variant (added by Task 1)

  **Test References**:
  - `src/type_system/tests.rs:500-553` — Existing pure function test; new tests follow same AST construction pattern but with two functions (caller + callee)

  **WHY Each Reference Matters**:
  - `call_resolution.rs:100-114` — This is the EXACT code location to modify; must understand the `Expr::Identifier` destructure pattern to extend it
  - `checker.rs:923-932` — Confirms `current_function_is_pure()` correctly reads from modifier stack; no changes needed there
  - `symbol_table.rs` — `lookup` returns `Option<&SymbolInfo>` — need to handle the `None` case (builtin not in symbol table = assume allowed unless in impure list)

  **Acceptance Criteria**:

  **TDD:**
  - [ ] `test_pure_function_cannot_call_non_pure_user_function` → PASS (PurityViolation error)
  - [ ] `test_pure_function_can_call_pure_user_function` → PASS (no error)
  - [ ] `test_pure_function_allows_local_mutation` → PASS (no error)
  - [ ] `test_pure_function_allows_collection_member_calls` → PASS (no error)
  - [ ] Existing `test_type_check_pure_function_rejects_print_call` updated to expect `PurityViolation` instead of `InvalidOperation` → PASS
  - [ ] `cargo clippy --all-targets -- -D warnings` → 0 warnings

  **QA Scenarios (MANDATORY):**

  ```
  Scenario: Pure function calling non-pure user function produces PurityViolation
    Tool: Bash (cargo test)
    Preconditions: Transitive enforcement implemented
    Steps:
      1. Run `cargo test test_pure_function_cannot_call_non_pure_user_function -- --nocapture 2>&1`
      2. Assert exit code is 0
      3. Assert output contains "test ... ok"
    Expected Result: PurityViolation error with callee name and reason about missing pure modifier
    Failure Indicators: Test failure, wrong error type (InvalidOperation instead of PurityViolation)
    Evidence: .sisyphus/evidence/task-5-transitive-enforcement.txt

  Scenario: Pure function calling pure user function compiles successfully
    Tool: Bash (cargo test)
    Preconditions: Transitive enforcement implemented
    Steps:
      1. Run `cargo test test_pure_function_can_call_pure_user_function -- --nocapture 2>&1`
      2. Assert exit code is 0
    Expected Result: No error — pure→pure call is allowed
    Failure Indicators: False positive PurityViolation error
    Evidence: .sisyphus/evidence/task-5-pure-calls-pure.txt

  Scenario: Pure function with local mutation compiles (guardrail)
    Tool: Bash (cargo test)
    Preconditions: Transitive enforcement does NOT touch local mutation
    Steps:
      1. Run `cargo test test_pure_function_allows_local_mutation -- --nocapture 2>&1`
      2. Run `cargo test test_pure_function_allows_collection_member_calls -- --nocapture 2>&1`
      3. Assert both exit code 0
    Expected Result: Local let mutable, assignments, .push() all allowed in pure functions
    Failure Indicators: Any purity violation error for local mutation
    Evidence: .sisyphus/evidence/task-5-local-mutation-allowed.txt
  ```

  **Commit**: YES (groups with Wave 2)
  - Message: `feat(type_system): enforce transitive purity for user-defined function calls`
  - Files: `src/type_system/checker/call_resolution.rs`, `src/type_system/tests.rs`
  - Pre-commit: `cargo test`

---

- [x] 6. Reject `pure entry` Combination

  **What to do**:
  - **RED**: Write a unit test `test_pure_entry_combination_rejected` in `src/type_system/tests.rs`. Construct a `Decl::Function` with `is_entry: true` and `modifiers: vec![FunctionModifier::Pure]`. Call `type_check_program`. Assert it returns a `PurityViolation` (or `InvalidOperation`) error explaining that entry functions are implicitly impure.
  - **GREEN**: In `src/type_system/checker/declarations.rs`, in the `type_check_function_declaration` method (line 514), add a check BEFORE line 553 (before `effective_modifiers` construction):
    ```rust
    if params.is_entry && params.modifiers.iter().any(|m| matches!(m, FunctionModifier::Pure)) {
        return Err(TypeError::PurityViolation {
            callee_name: String::from("entry"),
            reason: String::from("entry functions are implicitly impure and cannot be marked 'pure'"),
            span: TypeError::span_from_span(params.span),
        });
    }
    ```
  - **REFACTOR**: Ensure the error message is clear and actionable.

  **Must NOT do**:
  - Do NOT modify the parser — `pure entry` should still parse; the error is at type-check time
  - Do NOT add this check in the lexer or parser phases

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Single conditional check + one test
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES (with Tasks 5, 7 in Wave 2)
  - **Parallel Group**: Wave 2
  - **Blocks**: Task 8
  - **Blocked By**: Task 1

  **References**:

  **Pattern References**:
  - `src/type_system/checker/declarations.rs:514-560` — `type_check_function_declaration` method; add the check before line 553 (before `effective_modifiers` construction)
  - `src/type_system/checker/declarations.rs:553-560` — Where `effective_modifiers` is built and `Untested` is auto-added for entry; the `pure entry` check should go BEFORE this

  **API/Type References**:
  - `src/type_system/errors.rs` — `TypeError::PurityViolation` variant (added by Task 1)
  - `src/ast/modifiers.rs` — `FunctionModifier::Pure` variant

  **Test References**:
  - `src/type_system/tests.rs:500-553` — AST construction pattern for pure function; adapt with `is_entry: true`

  **WHY Each Reference Matters**:
  - `declarations.rs:514-560` — Must add the check in the right place (before modifier processing, after parameter setup)
  - `declarations.rs:553-560` — Shows how `is_entry` is already used in this method; follow the same pattern

  **Acceptance Criteria**:

  **TDD:**
  - [ ] `test_pure_entry_combination_rejected` → PASS
  - [ ] `cargo test` → all existing tests still pass

  **QA Scenarios (MANDATORY):**

  ```
  Scenario: pure entry function produces clear error
    Tool: Bash (cargo test)
    Preconditions: pure entry check added
    Steps:
      1. Run `cargo test test_pure_entry_combination_rejected -- --nocapture 2>&1`
      2. Assert exit code is 0
      3. Assert output contains "test ... ok"
    Expected Result: PurityViolation error with message about entry being implicitly impure
    Failure Indicators: Test failure, no error produced, wrong error type
    Evidence: .sisyphus/evidence/task-6-pure-entry-rejected.txt

  Scenario: Non-pure entry function still works
    Tool: Bash (cargo test)
    Preconditions: pure entry check doesn't affect normal entry functions
    Steps:
      1. Run `cargo test test_type_check_non_pure_function_allows_print_call -- --nocapture 2>&1`
      2. Assert exit code is 0
    Expected Result: Normal entry function compiles without error
    Failure Indicators: False positive rejection of non-pure entry
    Evidence: .sisyphus/evidence/task-6-normal-entry-ok.txt
  ```

  **Commit**: YES (groups with Wave 2)
  - Message: `feat(type_system): reject pure entry function combination`
  - Files: `src/type_system/checker/declarations.rs`, `src/type_system/tests.rs`
  - Pre-commit: `cargo test`

- [x] 7. Lambda Purity Inheritance Test + Code Comment

  **What to do**:
  - **RED**: Write a unit test `test_lambda_inside_pure_function_inherits_purity` in `src/type_system/tests.rs`. Construct a pure function whose body contains a lambda that calls `print`. Assert this produces a `PurityViolation` error. The lambda should NOT have its own `pure` modifier — it inherits purity from the enclosing function.
  - **GREEN**: This test should ALREADY pass after Task 5 (transitive enforcement + expanded impure list), because the lambda body is type-checked while the enclosing function's modifier context is still on the stack. If it doesn't pass, investigate why `current_function_is_pure()` returns false during lambda body type-checking.
  - **REFACTOR**: Add a code comment in `src/type_system/checker/expressions.rs` at the lambda type-checking location (~line 735, near `type_check_lambda_expr`) documenting the implicit purity inheritance:
    ```rust
    // NOTE: Lambdas do NOT push their own modifier context. They inherit
    // the enclosing function's modifiers from function_modifier_stack.
    // This means lambdas inside `pure` functions are implicitly pure.
    // This is intentional — a lambda created in a pure context should
    // not be able to perform impure operations.
    ```
  - Also write a positive test: `test_lambda_inside_non_pure_function_allows_print` — lambda calling `print` in a non-pure function should succeed.

  **Must NOT do**:
  - Do NOT modify the lambda type-checking logic — just document and test the existing implicit behavior
  - Do NOT add modifier context push/pop to lambda expressions

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Writing tests + one code comment; no logic changes
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES (with Tasks 5, 6 in Wave 2)
  - **Parallel Group**: Wave 2
  - **Blocks**: Task 8
  - **Blocked By**: Tasks 1, 2

  **References**:

  **Pattern References**:
  - `src/type_system/checker/expressions.rs:735` — Lambda type-checking entry point; add comment here
  - `src/type_system/checker.rs:923-932` — `current_function_is_pure()` reads from `function_modifier_stack`; lambdas inherit because they don't push their own context

  **Test References**:
  - `src/type_system/tests.rs:500-553` — AST construction pattern; adapt to include a lambda `Expr::Lambda` that calls `print` inside a pure function's body

  **WHY Each Reference Matters**:
  - `expressions.rs:735` — This is where the lambda body is type-checked WITHOUT pushing modifier context; the comment documents this design decision
  - `checker.rs:923-932` — Confirms the mechanism: `function_modifier_stack.last()` returns the enclosing function's modifiers during lambda body checking

  **Acceptance Criteria**:

  **TDD:**
  - [ ] `test_lambda_inside_pure_function_inherits_purity` → PASS (PurityViolation for print in lambda)
  - [ ] `test_lambda_inside_non_pure_function_allows_print` → PASS (no error)
  - [ ] Comment added to `expressions.rs` at lambda type-checking

  **QA Scenarios (MANDATORY):**

  ```
  Scenario: Lambda in pure function cannot call print
    Tool: Bash (cargo test)
    Preconditions: Lambda purity inheritance working via modifier stack
    Steps:
      1. Run `cargo test test_lambda_inside_pure_function_inherits_purity -- --nocapture 2>&1`
      2. Assert exit code is 0
    Expected Result: PurityViolation error — lambda inherits enclosing pure context
    Failure Indicators: No error produced (lambda escapes purity check), wrong error type
    Evidence: .sisyphus/evidence/task-7-lambda-purity.txt

  Scenario: Lambda in non-pure function can call print
    Tool: Bash (cargo test)
    Preconditions: Lambda purity inheritance only applies in pure context
    Steps:
      1. Run `cargo test test_lambda_inside_non_pure_function_allows_print -- --nocapture 2>&1`
      2. Assert exit code is 0
    Expected Result: No error — non-pure context allows impure calls in lambda
    Failure Indicators: False positive PurityViolation
    Evidence: .sisyphus/evidence/task-7-lambda-non-pure-ok.txt
  ```

  **Commit**: YES (groups with Wave 2)
  - Message: `test(type_system): add lambda purity inheritance tests and documentation`
  - Files: `src/type_system/tests.rs`, `src/type_system/checker/expressions.rs`
  - Pre-commit: `cargo test`

---

- [x] 8. Integration Tests Through Full Parse Pipeline

  **What to do**:
  - Add integration tests in `src/type_system/test_integration.rs` that test the `pure` keyword through the complete lex → parse → type-check pipeline using source strings (not hand-built ASTs).
  - Tests to add (each uses `parse_pipeline()` helper from line 41):

    **Positive tests (should compile):**
    1. `test_integration_pure_function_with_local_mutation`: Source with `pure let compute = f(n: int32): int32 { let mutable sum = 0; ... return sum }` — compiles successfully
    2. `test_integration_pure_function_calls_pure_function`: Source with two `pure` functions where one calls the other — compiles successfully

    **Negative tests (should produce errors):**
    3. `test_integration_pure_function_rejects_print`: Source with `pure let worker = f(): void { print("hello") }` — PurityViolation error
    4. `test_integration_pure_function_rejects_non_pure_call`: Source with `let impure_fn = f(): void { print("hi") }` and `pure let caller = f(): void { impure_fn() }` — PurityViolation error for transitive impurity
    5. `test_integration_pure_entry_rejected`: Source with `pure entry main = f(args: string[]): void { ... }` — PurityViolation error
    6. `test_integration_pure_function_rejects_random`: Source with `pure let worker = f(): int32 { return random_int32() }` — PurityViolation error

  - For each negative test, assert:
    - `result.is_err()`
    - Error list contains `TypeError::PurityViolation { .. }`
    - `callee_name` field matches expected function name

  **Must NOT do**:
  - Do NOT duplicate unit tests — integration tests focus on source-string parsing, not AST construction
  - Do NOT test features already covered by unit tests at a granular level
  - Do NOT use `parse_pipeline_with_spaces` unless testing colon-block syntax

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
    - Reason: Multiple integration tests requiring source string construction matching the language's exact syntax; need to understand both parser expectations and type checker behavior
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: NO (depends on all Wave 2 tasks)
  - **Parallel Group**: Wave 3 (sequential after Wave 2)
  - **Blocks**: F1-F4
  - **Blocked By**: Tasks 5, 6, 7

  **References**:

  **Pattern References**:
  - `src/type_system/test_integration.rs:41-67` — `parse_pipeline()` and `parse_pipeline_with_spaces()` helpers; use these for all integration tests
  - `src/type_system/test_integration.rs:78-100` — `test_hello_world_type_checks` shows the canonical integration test pattern: define source string → `parse_pipeline()` → `type_check_program()` → assert result
  - `src/type_system/test_integration.rs:10-14` — Comment about brace vs colon-block syntax; tests use `{ }` brace syntax since the current parser requires it for blocks

  **API/Type References**:
  - `src/type_system/errors.rs` — `TypeError::PurityViolation` for assertion pattern matching
  - README section on Functions — Shows `pure let` syntax: `pure let name = f(params): return_type => body`

  **External References**:
  - `language-spec/array_helpers.op` — Shows `pure` function syntax in language spec (note: uses colon-block syntax which parser may not support yet — use brace equivalent)
  - `memory-model-proposals/hybrid/region-based-memory/automatic_regions.op:49-61` — Shows `pure let compute_stats = f(...)` syntax with local mutation

  **WHY Each Reference Matters**:
  - `test_integration.rs:41-67` — MUST use `parse_pipeline()` helper, not construct ASTs manually; this tests the full front-end
  - `test_integration.rs:10-14` — Critical: integration test source strings must use brace syntax `{ }`, not colon-block syntax
  - `test_integration.rs:78-100` — Shows exact assertion pattern for integration tests

  **Acceptance Criteria**:

  **TDD:**
  - [ ] All 6 integration tests pass: `cargo test test_integration_pure -- --nocapture`
  - [ ] No regressions in existing integration tests
  - [ ] `cargo clippy --all-targets -- -D warnings` → 0 warnings

  **QA Scenarios (MANDATORY):**

  ```
  Scenario: All pure keyword integration tests pass
    Tool: Bash (cargo test)
    Preconditions: All Wave 1 + Wave 2 tasks complete
    Steps:
      1. Run `cargo test test_integration_pure -- --nocapture 2>&1`
      2. Assert exit code is 0
      3. Count "test ... ok" lines — expect 6
    Expected Result: All 6 integration tests pass (2 positive, 4 negative)
    Failure Indicators: Any test failure, compilation error in source strings
    Evidence: .sisyphus/evidence/task-8-integration-tests.txt

  Scenario: Full test suite passes with no regressions
    Tool: Bash (cargo test)
    Preconditions: All implementation tasks complete
    Steps:
      1. Run `cargo test 2>&1 | tail -3`
      2. Assert "test result: ok" in output
      3. Assert "0 failed" in output
    Expected Result: Complete test suite passes with zero failures
    Failure Indicators: Any failure count > 0
    Evidence: .sisyphus/evidence/task-8-full-suite.txt

  Scenario: Integration test source strings parse correctly
    Tool: Bash (cargo test)
    Preconditions: Source strings use correct Opalescent syntax
    Steps:
      1. Run each integration test individually to isolate parse failures
      2. Check that parse errors are absent (lex_errors.is_empty(), parse_errors.is_empty())
    Expected Result: All source strings lex and parse without errors before reaching type checker
    Failure Indicators: Panic from parse_pipeline() helper with "lex errors" or "parse errors"
    Evidence: .sisyphus/evidence/task-8-parse-validation.txt
  ```

  **Commit**: YES (Wave 3)
  - Message: `test(type_system): add integration tests for pure keyword enforcement`
  - Files: `src/type_system/test_integration.rs`
  - Pre-commit: `cargo test`

---

## Final Verification Wave (MANDATORY — after ALL implementation tasks)

> 4 review agents run in PARALLEL. ALL must APPROVE. Present consolidated results to user and get explicit "okay" before completing.
>
> **Do NOT auto-proceed after verification. Wait for user's explicit approval before marking work complete.**

- [x] F1. **Plan Compliance Audit** — `oracle`
  Read the plan end-to-end. For each "Must Have": verify implementation exists (read file, run `cargo test`). For each "Must NOT Have": search codebase for forbidden patterns — reject with file:line if found. Check evidence files exist in `.sisyphus/evidence/`. Compare deliverables against plan.
  Output: `Must Have [N/N] | Must NOT Have [N/N] | Tasks [N/N] | VERDICT: APPROVE/REJECT`

- [x] F2. **Code Quality Review** — `unspecified-high`
  Run `cargo clippy --all-targets -- -D warnings` + `cargo test`. Review all changed files for: `as any` equivalent, suppressed warnings, empty error handling, commented-out code, unused imports. Check AI slop: excessive comments, over-abstraction, generic names (data/result/item/temp). Verify no_std compliance in core modules.
  Output: `Build [PASS/FAIL] | Clippy [PASS/FAIL] | Tests [N pass/N fail] | Files [N clean/N issues] | VERDICT`

- [x] F3. **Real Manual QA** — `unspecified-high`
  Start from clean state. Execute EVERY QA scenario from EVERY task — follow exact steps, capture evidence. Test cross-task integration (features working together, not isolation). Test edge cases: empty state, nested pure functions, recursive pure functions. Save to `.sisyphus/evidence/final-qa/`.
  Output: `Scenarios [N/N pass] | Integration [N/N] | Edge Cases [N tested] | VERDICT`

- [x] F4. **Scope Fidelity Check** — `deep`
  For each task: read "What to do", read actual diff (git log/diff). Verify 1:1 — everything in spec was built (no missing), nothing beyond spec was built (no creep). Check "Must NOT do" compliance: `CoreType::Function` unmodified, no collection member call blocking, no higher-order purity enforcement. Flag unaccounted changes.
  Output: `Tasks [N/N compliant] | Contamination [CLEAN/N issues] | Unaccounted [CLEAN/N files] | VERDICT`

---

## Commit Strategy

| After Wave | Message | Files | Pre-commit |
|-----------|---------|-------|------------|
| Wave 1 | `feat(type_system): add PurityViolation error and expand impure builtin list` | errors.rs, call_resolution.rs, symbol_table.rs, declarations.rs, opalescent.tmLanguage.json | `cargo test` |
| Wave 2 | `feat(type_system): enforce transitive purity and reject pure entry` | call_resolution.rs, declarations.rs, expressions.rs, tests.rs | `cargo test` |
| Wave 3 | `test(type_system): add integration tests for pure keyword enforcement` | test_integration.rs | `cargo test` |

---

## Success Criteria

### Verification Commands
```bash
cargo test                          # Expected: all tests pass (0 failures)
cargo test --features integration   # Expected: all integration tests pass
cargo clippy --all-targets -- -D warnings  # Expected: 0 warnings
```

### Final Checklist
- [x] All "Must Have" present
- [x] All "Must NOT Have" absent
- [x] All tests pass
- [x] PurityViolation error includes diagnostic code `opalescent::type_system::purity_violation`
- [x] 20 impure builtins in list
- [x] Transitive enforcement works for named calls
- [x] Collection member calls (.push etc.) still work in pure functions
- [x] Lambda purity inheritance tested and documented
