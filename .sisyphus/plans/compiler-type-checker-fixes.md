# Fix Missing Standard Module Exports & Expression Loop Break-Value Typing

## TL;DR

> **Quick Summary**: Fix two compiler bugs: (1) 19 conversion builtins missing from the `standard` module's export list in the module resolver, and (2) `let x = loop => ... break x: value` fails because `Expr::Loop` doesn't track break-value types and codegen rejects `Expr::Loop` in `Stmt::Let` initializer position.
> 
> **Deliverables**:
> - All 19 conversion functions importable via `import X from standard`
> - `let x = loop => ... break x: value` compiles and runs correctly end-to-end
> 
> **Estimated Effort**: Medium
> **Parallel Execution**: YES - 2 waves + verification
> **Critical Path**: T2 → T3 → T4

---

## Context

### Original Request
User identified two compiler bugs:
1. 19 conversion functions (8 `string_to_*` and 11 `*_to_string`) are registered as global builtins but missing from the `standard` module's export list, so `import X from standard` fails for them
2. Single-binding `let x = loop => ... break x: value` is broken at multiple layers: type-checker returns `Unit` instead of break-value type, and codegen explicitly rejects `Expr::Loop` in expression position

### Research Findings

**Issue 1 — Module Resolver Gap:**
- `register_standard_module()` in `module_resolver.rs` only exports: `print`, `println`, `take_input`, `string_to_int32`, `string_to_int64`
- `register_size_specific_builtins()` in `size_specific_builtins.rs` registers all 19 conversion functions as global builtins with correct signatures
- Codegen layer (`resolve_imported_runtime_name`) already maps all 19 under `("standard", "symbol_name")` — no codegen changes needed
- Fix is purely additive: append 19 entries to the `standard_symbols` array

**Issue 2 — Expression Loop Break-Value Typing (broken at 3 layers):**

| Layer | Current Behavior | Required Behavior |
|-------|-----------------|-------------------|
| **Parser** | `let x = loop =>` → `Stmt::Let { init: Expr::Loop }` | Keep as-is (correct parse) |
| **Type-checker** | `Expr::Loop` → `CoreType::Unit` (no stack push/pop) | Push/pop `loop_break_type_stack`, infer break type, return it |
| **Codegen** | `codegen_expression(Expr::Loop)` → ERROR "lowered in statement context" | `codegen_let_statement` must detect `Expr::Loop` and route to loop-lowering logic |

Key discovery from parser analysis: `let x = loop =>` produces `Stmt::Let` (single binding), NOT `Stmt::LetDestructure`. The destructuring path (`let a, b = loop =>`) already works correctly at all layers. The fix must either:
- Route single-binding loop-lets through the existing LetDestructure codegen, OR
- Add direct `Expr::Loop` handling in `codegen_let_statement`

**Chosen approach**: Extend `codegen_let_statement` to detect `Expr::Loop` initializer and delegate to the existing loop-lowering logic from `codegen_let_destructure_statement`. This reuses proven codegen, avoids changing the parser, and is the most surgical fix.

### Metis Review
**Identified Gaps** (addressed):
- Codegen layer also broken (not just type-checker) — included as Task 3
- `infer_loop_break_types` returns `Vec<CoreType>`, expression needs scalar — handled: single value → return type, multiple → arity error, none → Unit
- Backward compat for bare breaks — preserved: no-value breaks still type as Unit

---

## Work Objectives

### Core Objective
Make all conversion builtins importable from `standard` and make single-binding `let x = loop => break x: val` work end-to-end (type-check + compile + run).

### Concrete Deliverables
- `src/type_system/module_resolver.rs` — 19 new entries in `standard_symbols` array
- `src/type_system/checker/expressions.rs` — `Expr::Loop` match arm pushes/pops `loop_break_type_stack`, returns break-value type
- `src/codegen/statements.rs` — `codegen_let_statement` handles `Expr::Loop` initializer via loop-lowering

### Definition of Done
- [x] `cargo test` passes with zero regressions
- [x] `import string_to_float64 from standard` compiles without error
- [x] `let x = loop => ... break x: 42` type-checks x as integer, compiles, runs correctly

### Must Have
- All 19 conversion functions importable from standard
- `Expr::Loop` type-checking returns break-value type (not Unit) when break carries a value
- `let x = loop => ... break x: val` compiles and runs end-to-end
- Backward compat: existing `let a, b = loop =>` destructuring still works
- Backward compat: bare `break` (no value) in expression loop still types as Unit
- Backward compat: `loop => ... break` (statement loop) unchanged

### Must NOT Have (Guardrails)
- Do NOT duplicate `string_to_int32` or `string_to_int64` in standard_symbols (already present)
- Do NOT add `random_*`, `print_*`, or other non-conversion builtins to standard (out of scope)
- Do NOT change `register_size_specific_builtins()` in `checker/size_specific_builtins.rs`
- Do NOT change `register_standard_builtins()` in `checker.rs`
- Do NOT change `Stmt::Loop` handler in `statements.rs`
- Do NOT change `type_check_let_destructure` in `statements.rs`
- Do NOT change the parser (`parser/statements.rs` or `parser/expressions.rs`)
- Do NOT add new fields to the `TypeChecker` struct — use existing `loop_break_type_stack`
- Do NOT refactor duplicate signature definitions between module_resolver and size_specific_builtins (separate tech debt)
- Do NOT make `Expr::Loop` work in general expression positions (e.g., `foo(loop => break x: 42)`) — that's a future enhancement beyond the current bug scope

---

## Verification Strategy

> **ZERO HUMAN INTERVENTION** — ALL verification is agent-executed. No exceptions.

### Test Decision
- **Infrastructure exists**: YES (Rust project with `cargo test`)
- **Automated tests**: YES (tests-after)
- **Framework**: `cargo test` (built-in Rust test framework)

### QA Policy
Every task MUST include agent-executed QA scenarios.
Evidence saved to `.sisyphus/evidence/task-{N}-{scenario-slug}.{ext}`.

- **Compiler**: Use Bash (`cargo test`, `cargo build`) — build project, run tests, assert pass
- **End-to-end**: Write `.op` test programs, compile with `cargo run --release -- <file.op> --run`, verify output

---

## Execution Strategy

### Parallel Execution Waves

```
Wave 1 (Start Immediately — independent fixes, MAX PARALLEL):
├── Task 1: Add 19 conversion functions to standard module exports [quick]
└── Task 2: Fix Expr::Loop type-checking to track break-value types [deep]

Wave 2 (After Wave 1 — codegen fix depends on T2):
└── Task 3: Fix codegen_let_statement to handle Expr::Loop initializer [deep]

Wave 3 (After Wave 2 — end-to-end verification):
└── Task 4: Add tests and end-to-end verification [unspecified-high]

Wave FINAL (After ALL tasks — 4 parallel reviews, then user okay):
├── Task F1: Plan compliance audit (oracle)
├── Task F2: Code quality review (unspecified-high)
├── Task F3: Real manual QA (unspecified-high)
└── Task F4: Scope fidelity check (deep)
-> Present results -> Get explicit user okay

Critical Path: T2 → T3 → T4 → F1-F4 → user okay
Parallel Speedup: T1 runs alongside T2 (Wave 1)
Max Concurrent: 2 (Wave 1)
```

### Dependency Matrix

| Task | Depends On | Blocks | Wave |
|------|-----------|--------|------|
| T1 | — | T4 | 1 |
| T2 | — | T3, T4 | 1 |
| T3 | T2 | T4 | 2 |
| T4 | T1, T2, T3 | F1-F4 | 3 |

### Agent Dispatch Summary

- **Wave 1**: **2 tasks** — T1 → `quick`, T2 → `deep`
- **Wave 2**: **1 task** — T3 → `deep`
- **Wave 3**: **1 task** — T4 → `unspecified-high`
- **FINAL**: **4 tasks** — F1 → `oracle`, F2 → `unspecified-high`, F3 → `unspecified-high`, F4 → `deep`

---

## TODOs

- [x] 1. Add 19 Missing Conversion Functions to Standard Module Exports

  **What to do**:
  - Open `src/type_system/module_resolver.rs`, find the `register_standard_module()` function
  - Locate the `standard_symbols` array (currently contains 5 entries: `print`, `println`, `take_input`, `string_to_int32`, `string_to_int64`)
  - Append 19 new entries to the array, grouped into two categories:

  **8 `string_to_*` functions (fallible, `errors ParseError`):**
  Each uses the pattern:
  ```rust
  (
      String::from("string_to_XXXX"),
      CoreType::Function {
          generic_params: Vec::new(),
          parameters: vec![CoreType::String],
          return_types: vec![CoreType::XXXX],  // target type
          error_types: vec![CoreType::Generic {
              name: "ParseError".to_owned(),
              type_args: Vec::new(),
          }],
      },
      SymbolType::Function,
  ),
  ```

  Add these 8 entries (copy the exact `CoreType` variant names from the `register_string_to_int` calls in `size_specific_builtins.rs`):

  | Function Name | Return CoreType |
  |---|---|
  | `string_to_int8` | `CoreType::Int8` |
  | `string_to_int16` | `CoreType::Int16` |
  | `string_to_uint8` | `CoreType::UInt8` |
  | `string_to_uint16` | `CoreType::UInt16` |
  | `string_to_uint32` | `CoreType::UInt32` |
  | `string_to_uint64` | `CoreType::UInt64` |
  | `string_to_float32` | `CoreType::Float32` |
  | `string_to_float64` | `CoreType::Float64` |

  **11 `*_to_string` functions (infallible, no errors):**
  Each uses the pattern:
  ```rust
  (
      String::from("XXXX_to_string"),
      CoreType::Function {
          generic_params: Vec::new(),
          parameters: vec![CoreType::XXXX],  // source type
          return_types: vec![CoreType::String],
          error_types: Vec::new(),
      },
      SymbolType::Function,
  ),
  ```

  Add these 11 entries (copy from the `register_to_string` calls in `size_specific_builtins.rs`):

  | Function Name | Parameter CoreType |
  |---|---|
  | `int8_to_string` | `CoreType::Int8` |
  | `int16_to_string` | `CoreType::Int16` |
  | `int32_to_string` | `CoreType::Int32` |
  | `int64_to_string` | `CoreType::Int64` |
  | `uint8_to_string` | `CoreType::UInt8` |
  | `uint16_to_string` | `CoreType::UInt16` |
  | `uint32_to_string` | `CoreType::UInt32` |
  | `uint64_to_string` | `CoreType::UInt64` |
  | `float32_to_string` | `CoreType::Float32` |
  | `float64_to_string` | `CoreType::Float64` |
  | `bool_to_string` | `CoreType::Boolean` |

  **Must NOT do**:
  - Do NOT add `string_to_int32` or `string_to_int64` again (they are already in the array at positions 4 and 5)
  - Do NOT add `random_*`, `print_*`, or any non-conversion builtins
  - Do NOT modify `checker.rs` or `size_specific_builtins.rs`
  - Do NOT change any other function in `module_resolver.rs` — only modify the `standard_symbols` array inside `register_standard_module()`

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Single-file additive change following an existing pattern — append 19 array entries
  - **Skills**: `[]`
  - **Skills Evaluated but Omitted**:
    - None applicable — this is a straightforward Rust code addition

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1 (with Task 2)
  - **Blocks**: Task 4
  - **Blocked By**: None (can start immediately)

  **References**:

  **Pattern References** (existing code to follow):
  - `src/type_system/module_resolver.rs` — `register_standard_module()` function — the existing `standard_symbols` array with 5 entries. Follow the EXACT tuple structure `(String::from("name"), CoreType::Function { ... }, SymbolType::Function)`. Each new entry must be identical in shape.
  - `src/type_system/module_resolver.rs` — The existing `string_to_int32` and `string_to_int64` entries — these are the direct template for `string_to_*` entries (fallible with ParseError)
  - `src/type_system/module_resolver.rs` — The existing `println` entry — template for infallible functions (empty `error_types`), though `*_to_string` functions take a typed parameter not `CoreType::String`

  **API/Type References** (contracts to implement against):
  - `src/type_system/checker/size_specific_builtins.rs` — `register_string_to_int()` helper — shows the EXACT `CoreType::Function` signature for each `string_to_*` function: `parameters: vec![CoreType::String]`, `return_types: vec![target_type]`, `error_types: vec![CoreType::Generic { name: "ParseError" ... }]`
  - `src/type_system/checker/size_specific_builtins.rs` — `register_to_string()` helper — shows the EXACT `CoreType::Function` signature for each `*_to_string` function: `parameters: vec![source_type]`, `return_types: vec![CoreType::String]`, `error_types: Vec::new()`
  - `src/type_system/checker/size_specific_builtins.rs` — `register_size_specific_builtins()` function — the authoritative list of ALL conversion functions and their CoreType variants (Int8, Int16, UInt8, etc.)

  **WHY Each Reference Matters**:
  - `module_resolver.rs:register_standard_module()` — This is the ONLY file you edit. The existing entries show the exact data structure shape to follow.
  - `size_specific_builtins.rs` — Do NOT edit this file, but READ it to get the exact `CoreType` variant names for each function. The signatures MUST match exactly between the module export and the global builtin registration.

  **Acceptance Criteria**:

  **QA Scenarios (MANDATORY):**

  ```
  Scenario: Verify all 19 new standard module exports compile
    Tool: Bash (cargo)
    Preconditions: Working compiler build
    Steps:
      1. Run `cargo build` in the project root
      2. Assert exit code 0
      3. Run `cargo test` in the project root
      4. Assert exit code 0, no test failures
    Expected Result: Clean build and all existing tests pass
    Failure Indicators: Compilation error in module_resolver.rs, duplicate export error, test failure
    Evidence: .sisyphus/evidence/task-1-build-and-test.txt

  Scenario: Verify no duplicate exports
    Tool: Bash (grep)
    Preconditions: Task 1 changes applied
    Steps:
      1. Search the `standard_symbols` array for duplicate function name strings
      2. Specifically verify `string_to_int32` and `string_to_int64` appear exactly ONCE each
      3. Count total entries in standard_symbols — should be exactly 24 (5 existing + 19 new)
    Expected Result: No duplicates, exactly 24 entries
    Failure Indicators: Any function name appearing more than once, count != 24
    Evidence: .sisyphus/evidence/task-1-no-duplicates.txt

  Scenario: Verify signature consistency between module export and builtin registration
    Tool: Bash (grep + manual inspection)
    Preconditions: Task 1 changes applied
    Steps:
      1. For each of the 19 new functions, compare the CoreType::Function signature in module_resolver.rs against the corresponding registration in size_specific_builtins.rs
      2. Specifically check: parameters vec, return_types vec, error_types vec, generic_params vec
      3. Assert all 19 signatures match exactly
    Expected Result: All 19 signatures identical between module export and builtin registration
    Failure Indicators: Any mismatch in parameter types, return types, or error types
    Evidence: .sisyphus/evidence/task-1-signature-consistency.txt
  ```

  **Evidence to Capture:**
  - [x] task-1-build-and-test.txt — cargo build + cargo test output
  - [x] task-1-no-duplicates.txt — duplicate check results
  - [x] task-1-signature-consistency.txt — signature comparison results

  **Commit**: YES (Commit 1)
  - Message: `fix(type-system): add 19 missing conversion builtins to standard module exports`
  - Files: `src/type_system/module_resolver.rs`
  - Pre-commit: `cargo test`

- [x] 2. Fix Expr::Loop Type-Checking to Track Break-Value Types

  **What to do**:
  - Open `src/type_system/checker/expressions.rs`, find the `Expr::Loop` match arm in `type_check_expr`
  - Current code (to be replaced):
    ```rust
    Expr::Loop { ref body, .. } => {
        self.type_check_stmt_with_return(body.as_ref(), None)?;
        Ok(CoreType::Unit)
    }
    ```
  - Replace with code that mirrors `Stmt::Loop` behavior:
    1. Push `None` onto `self.loop_break_type_stack` (enables break-value tracking for breaks inside this loop)
    2. Call `self.type_check_stmt_with_return(body.as_ref(), None)?` (type-check the loop body — breaks will now record their types into the stack)
    3. Pop from `self.loop_break_type_stack` and capture the result
     4. If pop returns `Some(break_types)` (i.e. the popped `Option<Vec<CoreType>>` is `Some(vec)`):
       - If `break_types.len() == 1`: return the single `CoreType` (this is the loop expression's value type)
       - If `break_types.len() > 1`: return a `TypeError` — single-binding expression loops cannot produce multiple values (that's what destructuring let is for)
       - If `break_types.is_empty()`: return `CoreType::Unit` (shouldn't happen but handle defensively)
     5. If pop returns `None` (no breaks with values encountered — the stack entry was never set by the break handler): return `CoreType::Unit` (backward compat)
     6. Note: `loop_break_type_stack` is `Vec<Option<Vec<CoreType>>>`, so `.pop()` returns `Option<Option<Vec<CoreType>>>`. The outer `Option` is always `Some` (we just pushed). The inner `Option` is `None` when no typed breaks were seen, `Some(vec)` when typed breaks were seen.

  - The key behavioral change: `Expr::Loop` now participates in break-value tracking via `loop_break_type_stack`, and returns the break-value type instead of unconditionally returning `Unit`

  **Must NOT do**:
  - Do NOT change `Stmt::Loop` handler in `statements.rs` (already works correctly)
  - Do NOT change `type_check_let_destructure` (already works correctly for multi-binding destructuring)
  - Do NOT add new fields to `TypeChecker` struct — use existing `loop_break_type_stack`
  - Do NOT change `infer_loop_break_types` or `collect_break_types`
  - Do NOT make expression loops support multiple break values (error on len > 1)

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: Requires understanding the loop_break_type_stack mechanism, interaction between Expr::Loop and Stmt::Break type-checking, and correct handling of edge cases (no breaks, bare breaks, typed breaks)
  - **Skills**: `[]`
  - **Skills Evaluated but Omitted**:
    - None applicable — Rust compiler internals

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1 (with Task 1)
  - **Blocks**: Task 3, Task 4
  - **Blocked By**: None (can start immediately)

  **References**:

  **Pattern References** (existing code to follow):
  - `src/type_system/checker/statements.rs:165-170` — `Stmt::Loop` match arm — the EXACT pattern to mirror: `checker.loop_break_type_stack.push(None)` before body type-check, `.pop()` after. This is your direct template.
  - `src/type_system/checker/statements.rs:171-202` — `Stmt::Break` match arm — shows how breaks interact with `loop_break_type_stack`: first break sets `Some(current_types)`, subsequent breaks validate against it. Understanding this is critical to know what the stack will contain after body type-checking.
  - `src/type_system/checker/statements.rs:371-383` — `type_check_let_destructure` — shows how destructuring let pushes/pops the stack and calls `infer_loop_break_types`. You do NOT need `infer_loop_break_types` because the stack entry itself will contain the break types after body type-checking (the break handler fills it in).

  **API/Type References**:
  - `src/type_system/checker.rs:87-89` — `loop_break_type_stack: Vec<Option<Vec<CoreType>>>` field declaration — the stack is `Vec<Option<Vec<CoreType>>>`: outer Option is None when no breaks with values seen, inner Vec holds the break-value types
  - `src/type_system/types.rs` — `CoreType::Unit` — the fallback return type when no break values are present

  **WHY Each Reference Matters**:
  - `Stmt::Loop` match arm — Direct template. Copy the push/pop pattern exactly.
  - `Stmt::Break` match arm — Explains what the stack will contain after body type-checking. Without understanding this, you won't know how to interpret the popped value.
  - `type_check_let_destructure` — Shows the precedent for using loop_break_type_stack with expression loops. Your change is a simpler version of this.

  **Acceptance Criteria**:

  **QA Scenarios (MANDATORY):**

  ```
  Scenario: Cargo build succeeds after type-checker change
    Tool: Bash (cargo)
    Preconditions: Task 2 changes applied to expressions.rs
    Steps:
      1. Run `cargo build` in project root
      2. Assert exit code 0
      3. Run `cargo test` in project root
      4. Assert exit code 0, no new test failures
    Expected Result: Clean build and all existing tests pass — this change should not break anything since Expr::Loop previously returned Unit and now returns Unit for bare breaks (backward compat)
    Failure Indicators: Compilation error, type mismatch in existing code
    Evidence: .sisyphus/evidence/task-2-build-and-test.txt

  Scenario: Verify loop_break_type_stack push/pop is balanced
    Tool: Bash (grep)
    Preconditions: Task 2 changes applied
    Steps:
      1. Read the modified Expr::Loop match arm in expressions.rs
      2. Verify there is exactly one `push(None)` call before `type_check_stmt_with_return`
      3. Verify there is exactly one `pop()` call after `type_check_stmt_with_return`
      4. Verify the pop result is captured and used to determine return type
    Expected Result: Balanced push/pop, pop result drives return type
    Failure Indicators: Unbalanced push/pop (would corrupt stack for nested loops), pop result ignored
    Evidence: .sisyphus/evidence/task-2-stack-balance.txt
  ```

  **Evidence to Capture:**
  - [x] task-2-build-and-test.txt — cargo build + cargo test output
  - [x] task-2-stack-balance.txt — code inspection showing balanced push/pop

  **Commit**: NO (groups with Task 3 in Commit 2)

- [x] 3. Fix Codegen to Handle Expr::Loop Initializer in Let Statements

  **What to do**:
  - Open `src/codegen/statements.rs`, find the `codegen_let_statement` function
  - Currently, when `initializer` is `Some(Expr::Loop { ... })`, it calls `codegen_expression(init_expr, ...)` which hits the explicit error: `"loop expressions are lowered in statement context"`
  - Add a special case: **before** calling `codegen_expression`, check if the initializer is `Expr::Loop`. If so, handle it with loop-lowering logic similar to `codegen_let_destructure_statement`

  **Implementation approach** (choose the cleanest based on existing code structure):

  **Option A — Route through existing LetDestructure codegen:**
  - In `codegen_let_statement`, detect `Expr::Loop` initializer
  - Create a single-element bindings slice from the `Let`'s binding
  - Call `codegen_let_destructure_statement(codegen_context, env, &[binding], initializer)` directly
  - This reuses ALL existing loop-lowering logic with zero duplication

  **Option B — Inline loop lowering for single binding:**
  - In `codegen_let_statement`, detect `Expr::Loop` initializer
  - Extract the `Expr::Loop { body, .. }` fields
  - Allocate a result slot (alloca) for the single binding
  - Generate the loop body using the existing loop codegen pattern (look at how `codegen_let_destructure_statement` calls into loop lowering)
  - After the loop, the break will have stored the value into the slot
  - Load from the slot and bind to the variable name

  **Recommendation**: Option A if `codegen_let_destructure_statement`'s binding type (`LetBinding`) is compatible with `Let`'s binding. Option B if there's a type mismatch. Read both functions to determine compatibility.

  **Key files to examine for implementation context:**
  - `src/codegen/statements.rs` — `codegen_let_destructure_statement()` — the FULL function body shows exactly how loop initializers are lowered: it extracts `Expr::Loop { body }`, sets up break slots, generates loop body, and binds results. Your single-binding implementation mirrors this.
  - `src/codegen/statements.rs` — `codegen_let_statement()` — lines 99-117 show where `codegen_expression` is called on the initializer. The `Expr::Loop` check should go BEFORE these calls.

  **Must NOT do**:
  - Do NOT change `codegen_expression` for `Expr::Loop` — it should continue to error for general expression positions (scope is let-initializer only)
  - Do NOT change `codegen_let_destructure_statement` — it already works
  - Do NOT change the parser
  - Do NOT modify any type-checker code (that was Task 2)

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: Requires understanding LLVM codegen patterns, alloca/store/load semantics, and how break statements store values into pre-allocated slots. Must correctly integrate with existing loop codegen infrastructure.
  - **Skills**: `[]`
  - **Skills Evaluated but Omitted**:
    - None applicable — Rust compiler codegen internals

  **Parallelization**:
  - **Can Run In Parallel**: NO
  - **Parallel Group**: Wave 2 (sequential, after Task 2)
  - **Blocks**: Task 4
  - **Blocked By**: Task 2 (type-checker must correctly type Expr::Loop before codegen can rely on it)

  **References**:

  **Pattern References** (existing code to follow):
  - `src/codegen/statements.rs` — `codegen_let_destructure_statement()` (lines ~155-165 and onward) — This is the PRIMARY reference. It shows the complete pattern: extracting `Expr::Loop { body }`, setting up pre-allocated slots for each binding, generating the loop body (where breaks store into slots), and loading results after the loop. Your single-binding version follows this exact flow but with 1 slot instead of N.
  - `src/codegen/statements.rs` — `codegen_let_statement()` (lines ~99-117) — The function you'll modify. Shows where `codegen_expression` is called on the initializer. Your `Expr::Loop` check goes before these branches.

  **API/Type References**:
  - `src/codegen/expressions.rs:143-147` — `Expr::Loop` match arm in `codegen_expression` — confirms this path REJECTS loop expressions. You are NOT fixing this; instead you're intercepting BEFORE `codegen_expression` is called.
  - `src/ast.rs` — `Expr::Loop { body: Box<Stmt>, span: Span, id: NodeId }` — the AST shape you'll pattern-match against
  - `src/ast.rs` — `LetBinding` struct — check if the `Let` variant's binding and `LetDestructure`'s bindings use the same `LetBinding` type (determines whether Option A works)

  **WHY Each Reference Matters**:
  - `codegen_let_destructure_statement` — This is the proven, working loop-lowering codegen. You MUST understand its entire flow before writing the single-binding version. If it takes `&[LetBinding]`, you can potentially call it directly with a single-element slice (Option A).
  - `codegen_let_statement` — This is where you add the interception. You need to understand the two branches (with type annotation vs inferred) to know where the Expr::Loop check fits.
  - `codegen_expression` Expr::Loop arm — Confirms you must NOT let execution reach this point for loop initializers.

  **Acceptance Criteria**:

  **QA Scenarios (MANDATORY):**

  ```
  Scenario: Cargo build and test pass after codegen changes
    Tool: Bash (cargo)
    Preconditions: Tasks 2 and 3 changes applied
    Steps:
      1. Run `cargo build` in project root
      2. Assert exit code 0
      3. Run `cargo test` in project root
      4. Assert exit code 0, no new test failures
    Expected Result: Clean build, all tests pass
    Failure Indicators: Codegen error, LLVM IR generation failure, test regression
    Evidence: .sisyphus/evidence/task-3-build-and-test.txt

  Scenario: Verify Expr::Loop initializer is intercepted before codegen_expression
    Tool: Bash (grep + code inspection)
    Preconditions: Task 3 changes applied
    Steps:
      1. Read the modified `codegen_let_statement` function
      2. Verify there is a pattern match for `Expr::Loop` on the initializer BEFORE the call to `codegen_expression`
      3. Verify the Expr::Loop branch does NOT call `codegen_expression` (which would error)
      4. Verify the Expr::Loop branch either calls `codegen_let_destructure_statement` or implements equivalent loop lowering
    Expected Result: Expr::Loop is handled before codegen_expression is reached
    Failure Indicators: codegen_expression still called on Expr::Loop, runtime crash
    Evidence: .sisyphus/evidence/task-3-interception-check.txt
  ```

  **Evidence to Capture:**
  - [x] task-3-build-and-test.txt — cargo build + cargo test output
  - [x] task-3-interception-check.txt — code inspection results

  **Commit**: YES (Commit 2)
  - Message: `fix(type-system,codegen): support break-value typing in expression loops`
  - Files: `src/type_system/checker/expressions.rs`, `src/codegen/statements.rs`
  - Pre-commit: `cargo test`

- [x] 4. Add Tests and End-to-End Verification

  **What to do**:
  - Write comprehensive tests verifying both fixes. Two categories:

  **Category A — Standard module export tests:**
  - Find the existing test infrastructure (look for test files in `src/type_system/tests/` or `tests/` directory)
  - Add tests that verify each of the 19 new conversion functions can be resolved from the standard module
  - Test approach: use the module resolver's `resolve_symbol("standard", "function_name", ...)` API directly in unit tests, OR write `.op` test programs that import each function

  **Category B — Expression loop break-value tests:**
  - Add tests verifying `Expr::Loop` type-checking returns correct types:
    - Expression loop with single break value → returns that type (not Unit)
    - Expression loop with bare break → returns Unit
    - Expression loop with mismatched break types → TypeError
    - Nested expression loops → each loop's break type is independent
  - Add end-to-end tests: write `.op` programs that use `let x = loop => ... break x: value` and verify they compile AND produce correct runtime output

  **Test programs to write (end-to-end `.op` test programs):**

  ```
  // test_standard_imports.op
  import string_to_float64 from standard
  import bool_to_string from standard
  // ... test each of the 19 functions can be imported
  ```

  ```
  // test_loop_expression.op
  let result = loop =>
    break result: 42
  println(int64_to_string(result))  // integer literals infer as int64; should print "42"
  ```

  **Must NOT do**:
  - Do NOT modify production code (only test files)
  - Do NOT write tests that require human intervention
  - Do NOT skip any of the 19 conversion functions in import tests

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
    - Reason: Requires understanding test infrastructure, writing both unit tests and end-to-end compiler tests, and verifying runtime behavior
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: NO
  - **Parallel Group**: Wave 3 (after Tasks 1, 2, 3)
  - **Blocks**: F1-F4
  - **Blocked By**: Tasks 1, 2, 3

  **References**:

  **Pattern References**:
  - Look for existing test files: `tests/` directory, `src/**/tests.rs`, `src/**/tests/` — follow existing test patterns
  - `src/type_system/module_resolver.rs` — `resolve_symbol()` method — the API to test standard module symbol resolution
  - Any existing `.op` test files in the project — follow their structure for end-to-end tests

  **Test References**:
  - Search for `#[test]` in `src/type_system/` to find existing type-checker tests — follow their patterns for new tests

  **WHY Each Reference Matters**:
  - Existing test patterns ensure new tests are consistent with the project's testing conventions
  - `resolve_symbol()` is the exact API that the import system uses — testing it directly proves the fix works

  **Acceptance Criteria**:

  **QA Scenarios (MANDATORY):**

  ```
  Scenario: All new tests pass
    Tool: Bash (cargo test)
    Preconditions: All tasks (1-4) complete
    Steps:
      1. Run `cargo test` in project root
      2. Assert exit code 0
      3. Verify new test count (should be > 0 new tests)
      4. Assert no test failures
    Expected Result: All tests pass including new ones
    Failure Indicators: Any test failure, compilation error in test files
    Evidence: .sisyphus/evidence/task-4-all-tests-pass.txt

  Scenario: End-to-end test - import conversion function from standard
    Tool: Bash
    Preconditions: All tasks complete, compiler built (`cargo build --release`)
    Steps:
      1. Write a `.op` file (e.g. `/tmp/test_import.op`) that does `import string_to_float64 from standard` and calls it
      2. Compile and run it: `cargo run --release -- /tmp/test_import.op --run`
      3. Assert exit code 0 (no compilation errors)
      4. Assert correct runtime output (the converted value prints correctly)
    Expected Result: Program compiles and runs successfully with correct output
    Failure Indicators: "unknown symbol" error on import, compilation crash, wrong runtime output
    Evidence: .sisyphus/evidence/task-4-e2e-import.txt

  Scenario: End-to-end test - let x = loop expression with break value
    Tool: Bash
    Preconditions: All tasks complete, compiler built (`cargo build --release`)
    Steps:
      1. Write a `.op` file (e.g. `/tmp/test_loop_expr.op`) with `let x = loop => break x: 42` followed by `println(int64_to_string(x))` (integer literals infer as int64)
      2. Compile and run it: `cargo run --release -- /tmp/test_loop_expr.op --run`
      3. Assert exit code 0
      4. Assert stdout contains "42"
    Expected Result: Program compiles, runs, prints "42"
    Failure Indicators: Type error during compilation, codegen error ("loop expressions are lowered in statement context"), wrong runtime value
    Evidence: .sisyphus/evidence/task-4-e2e-loop-expr.txt

  Scenario: Regression test - existing destructuring let from loop still works
    Tool: Bash (cargo test)
    Preconditions: All tasks complete
    Steps:
      1. Search for existing tests that use `let a, b = loop =>` or `LetDestructure`
      2. Run those specific tests
      3. Assert they all pass
    Expected Result: No regressions in destructuring let behavior
    Failure Indicators: Any existing test failure related to LetDestructure or loop
    Evidence: .sisyphus/evidence/task-4-regression-destructure.txt
  ```

  **Evidence to Capture:**
  - [x] task-4-all-tests-pass.txt — full cargo test output
  - [x] task-4-e2e-import.txt — end-to-end import test
  - [x] task-4-e2e-loop-expr.txt — end-to-end loop expression test
  - [x] task-4-regression-destructure.txt — regression test results

  **Commit**: YES (Commit 3)
  - Message: `test: add tests for standard module exports and expression loop break values`
  - Files: test files (location determined by existing test infrastructure)
  - Pre-commit: `cargo test`

---

## Final Verification Wave

> 4 review agents run in PARALLEL. ALL must APPROVE. Present consolidated results to user and get explicit "okay" before completing.

- [x] F1. **Plan Compliance Audit** — `oracle`
  Read the plan end-to-end. For each "Must Have": verify implementation exists (read file, run command). For each "Must NOT Have": search codebase for forbidden patterns — reject with file:line if found. Check evidence files exist in `.sisyphus/evidence/`. Compare deliverables against plan.
  Output: `Must Have [N/N] | Must NOT Have [N/N] | Tasks [N/N] | VERDICT: APPROVE/REJECT`

- [x] F2. **Code Quality Review** — `unspecified-high`
  Run `cargo test` and `cargo build --release`. Review all changed files for: unused imports, dead code, inconsistent patterns vs existing code, missing error handling. Check that new standard_symbols entries exactly match CoreType signatures from size_specific_builtins.rs. Verify no `unwrap()` on user-facing paths.
  Output: `Build [PASS/FAIL] | Tests [N pass/N fail] | Files [N clean/N issues] | VERDICT`

- [x] F3. **Real Manual QA** — `unspecified-high`
  Write and compile test `.op` programs (using `cargo run --release -- <file.op> --run`) exercising: (1) importing each of the 19 new conversion functions from standard, (2) `let x = loop => ... break x: 42` with various types, (3) nested expression loops, (4) bare break in expression loop. Capture compiler output and runtime results. Save to `.sisyphus/evidence/final-qa/`.
  Output: `Scenarios [N/N pass] | Integration [N/N] | Edge Cases [N tested] | VERDICT`

- [x] F4. **Scope Fidelity Check** — `deep`
  For each task: read "What to do", read actual diff (`git diff`). Verify 1:1 — everything in spec was built (no missing), nothing beyond spec was built (no creep). Check "Must NOT do" compliance. Flag unaccounted changes.
  Output: `Tasks [N/N compliant] | Unaccounted [CLEAN/N files] | VERDICT`

---

## Commit Strategy

- **Commit 1** (after T1): `fix(type-system): add 19 missing conversion builtins to standard module exports` — `src/type_system/module_resolver.rs`
- **Commit 2** (after T3): `fix(type-system,codegen): support break-value typing in expression loops` — `src/type_system/checker/expressions.rs`, `src/codegen/statements.rs`
- **Commit 3** (after T4): `test: add tests for standard module exports and expression loop break values` — test files

---

## Success Criteria

### Verification Commands
```bash
cargo test              # Expected: all tests pass, 0 failures
cargo build --release   # Expected: clean build, no warnings from changed files
```

### Final Checklist
- [x] All 19 conversion functions importable from standard
- [x] `let x = loop => ... break x: value` works end-to-end
- [x] Existing `let a, b = loop =>` destructuring unchanged
- [x] Statement loops (`loop => ... break`) unchanged
- [x] `cargo test` passes
- [x] No regressions in existing compiler behavior
