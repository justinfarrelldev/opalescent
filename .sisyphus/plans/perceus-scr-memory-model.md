# Perceus + Second-Class References Memory Model

## TL;DR

> **Quick Summary**: Implement a Perceus-style reference counting memory model with second-class references into the Opalescent compiler. Second-class `ref`/`mutable ref` parameter annotations enable zero-copy reads, Perceus RC insertion handles ownership tracking, iterative drops prevent stack overflow on deep structures, and `Weak<T>` references (upgrading to `Option<T>`) enable mutable object cycles without a cycle collector.
> 
> **Deliverables**:
> - Lexer/parser support for `ref`, `mutable ref` parameter annotations and `Weak<T>` type
> - Type checker enforcement of second-class ref rules and mutable ref aliasing
> - RC insertion pass in codegen (inc/dec/drop at correct points)
> - Perceus reuse analysis (reuse memory for unique owners)
> - Iterative drop implementation (work-list based, no recursion)
> - Weak reference support (`Weak<T>` → `Option<T>` upgrade)
> - C runtime: `opal_rc.c`/`opal_rc.h` with RC object header, inc, dec, drop, weak functions
> - 7+ test projects exercising each feature
> - Documentation in README.md and `language-spec/requirements/memory-model.md`
> 
> **Estimated Effort**: XL
> **Parallel Execution**: YES — 5 waves
> **Critical Path**: Tokens → AST → Parser → Type Checker → RC Insertion → Reuse Analysis → Integration Tests → Docs

---

## Context

### Original Request
Implement the Perceus + Second Class References memory model from `memory-model-proposals/combined/perceus-with-second-class-refs/proposal.md`. Read the full language spec and all test projects before implementation. Use TDD with red-green-refactor extensively — never skip refactor. Use iterative drops (not recursive). Use weak refs (not a backup cycle collector) for mutable object cycles. Use `Option<T>` (already registered in type checker) rather than `?` syntax for weak ref upgrades. Document extensively in README.md and a new language-spec markdown file. Code around future module imports. Use multiple test projects.

### Interview Summary
**Key Discussions**:
- All source files needed for planning have been read (token.rs, lexer.rs, ast.rs, ast/types.rs, type_system/types.rs, type_system/memory.rs, codegen/types.rs, codegen/values.rs, runtime files, integration tests)
- `Mutable` keyword already exists — reuse for `mutable ref` parsing
- `Option<T>` already registered as built-in generic — ready for weak ref upgrade returns
- `Parameter` struct needs `passing_mode` field added
- C runtime uses `#include` aggregation — add `#include "opal_rc.c"`
- Project uses `no_std` with `alloc`/`core`, BTreeMap, LLVM 14 via inkwell, strict clippy

**Research Findings**:
- 13 existing test project directories provide clear pattern to follow
- Integration tests use `prepare_dir`/`cleanup_dir` helpers with `--features integration` flag
- `RESERVED_KEYWORDS` array used by both lexer and parser tests — must stay in sync
- Type checker has 18 submodules in `checker/` directory — granular modification possible

### Metis Review
Metis consultation timed out twice (30min each). Self-conducted gap analysis performed instead:
- **Addressed**: RC object header layout, iterative drop work-list design, weak ref count tracking
- **Addressed**: Second-class ref rules (no storing, no returning, no capturing in closures)
- **Addressed**: Mutable ref aliasing enforcement at call sites
- **Addressed**: Future module import compatibility (RC objects need stable ABI headers)

---

## Work Objectives

### Core Objective
Add a complete Perceus-style reference counting memory model with second-class references to the Opalescent compiler, from lexing through codegen, with C runtime support, comprehensive TDD test coverage, and thorough documentation.

### Concrete Deliverables
- Modified `src/token.rs` — `Ref`, `Weak` token variants
- Modified `src/lexer.rs` — `ref`, `weak` keywords registered
- Modified `src/ast/types.rs` — `PassingMode` enum, updated `Parameter` struct
- Modified `src/ast.rs` — Parser integration for ref/weak
- New/modified parser files — `ref`/`mutable ref` parameter parsing, `Weak<T>` type parsing
- Modified `src/type_system/checker/` — Second-class ref enforcement, aliasing checks, `Weak<T>` registration
- New `src/type_system/rc_analysis.rs` — RC insertion analysis pass
- Modified `src/codegen/` — RC inc/dec/drop generation, ref param lowering, reuse analysis
- New `runtime/opal_rc.c` + `runtime/opal_rc.h` — RC object header, inc, dec, iterative drop, weak support
- Modified `runtime/opal_runtime.c` + `runtime/opal_runtime.h` — Include new RC runtime
- 7+ new test project directories in `test-projects/`
- Modified `tests/integration_e2e.rs` — Integration tests for all new test projects
- Modified `README.md` — Memory model documentation, weak ref usage guide
- New `language-spec/requirements/memory-model.md` — Memory model specification

### Definition of Done
- [ ] `cargo build` succeeds with no warnings
- [ ] `cargo test` — all existing tests still pass (no regressions)
- [ ] `cargo test --features integration` — all new memory model test projects pass
- [ ] `cargo clippy -- -D warnings` — no clippy violations
- [ ] Each feature has at least one test project with expected output
- [ ] README.md has memory model section with weak ref documentation
- [ ] `language-spec/requirements/memory-model.md` exists with full specification

### Must Have
- Second-class `ref` parameters (zero-copy read, cannot store/return/capture)
- Second-class `mutable ref` parameters (exclusive write access, cannot alias)
- Perceus RC insertion (compiler-inserted inc/dec/drop)
- Iterative drops using work-list (not recursive drop calls)
- `Weak<T>` type that upgrades to `Option<T>` (using existing registered generic)
- TDD red-green-refactor for every feature (unit tests before implementation)
- Multiple test projects (not just unit tests)
- Future module import compatibility in design

### Must NOT Have (Guardrails)
- **No cycle collector** — weak refs are the ONLY mechanism for cycles
- **No `?` syntax for weak upgrade** — use `Option<T>` explicitly
- **No first-class references** — refs are parameter-only, cannot be stored/returned/captured
- **No recursive drops** — must use iterative work-list based drops
- **No `HashMap`** — project uses `BTreeMap` (no_std compatible)
- **No `std` imports** — use `alloc`/`core` only in compiler source
- **No skipping TDD refactor step** — RED → GREEN → REFACTOR, always all three
- **No proposal syntax copied verbatim** — adapt to existing Opalescent conventions
- **No modifications to existing test project behavior** — all 13 existing test projects must continue to pass unchanged

---

## Verification Strategy

> **ZERO HUMAN INTERVENTION** — ALL verification is agent-executed. No exceptions.

### Test Decision
- **Infrastructure exists**: YES — `cargo test` with unit tests + `cargo test --features integration` with integration tests
- **Automated tests**: TDD (red-green-refactor) — tests written FIRST, then implementation, then refactor
- **Framework**: Rust's built-in `#[cfg(test)]` for unit tests, `tests/integration_e2e.rs` for integration
- **TDD Protocol**: Each task follows RED (write failing test) → GREEN (minimal implementation to pass) → REFACTOR (clean up, extract, improve)

### QA Policy
Every task MUST include agent-executed QA scenarios.
Evidence saved to `.sisyphus/evidence/task-{N}-{scenario-slug}.{ext}`.

- **Compiler features**: Use Bash — `cargo test` for unit tests, `cargo test --features integration` for e2e
- **Compile-fail tests**: Use Bash — compile with expected stderr output matching error messages
- **Runtime behavior**: Use Bash — compile test project, run binary, compare stdout to expected output
- **Documentation**: Use Bash — verify file exists, grep for required sections

---

## Execution Strategy

### Parallel Execution Waves

```
Wave 1 (Foundation — all independent, start immediately):
├── Task 1: Add ref/weak tokens to lexer [quick]
├── Task 2: Add PassingMode enum + update Parameter struct in AST [quick]
├── Task 3: Create opal_rc.c/opal_rc.h C runtime [unspecified-high]
├── Task 4: Extend CoreType + MemoryLayout for RC metadata [quick]
├── Task 5: Create test project scaffolding (7 test project directories) [quick]

Wave 2 (Parser + Type System — depend on Wave 1):
├── Task 6: Parse ref/mutable ref parameter annotations (depends: 1, 2) [unspecified-high]
├── Task 7: Parse Weak<T> type syntax (depends: 1) [quick]
├── Task 8: Register Weak<T> in type checker + upgrade returns Option<T> (depends: 4, 7) [deep]
├── Task 9: Enforce second-class ref rules in type checker (depends: 2, 6) [deep]
├── Task 10: Enforce mutable ref aliasing at call sites (depends: 9) [deep]

Wave 3 (Codegen + RC Insertion — depend on Wave 2):
├── Task 11: RC insertion analysis pass (depends: 4, 8, 9) [deep]
├── Task 12: Lower ref params as pointers in codegen (depends: 6, 9) [unspecified-high]
├── Task 13: Generate RC inc/dec/drop calls in codegen (depends: 3, 11) [deep]
├── Task 14: Implement iterative drop in C runtime + codegen (depends: 3, 13) [deep]
├── Task 15: Lower Weak<T> to LLVM + weak ref codegen (depends: 3, 8, 13) [deep]

Wave 4 (Integration + Reuse — depend on Wave 3):
├── Task 16: Perceus reuse analysis (depends: 11, 13) [ultrabrain]
├── Task 17: Test projects — ref-basic + mutable-ref + ref-compile-fail (depends: 5, 12) [unspecified-high]
├── Task 18: Test projects — rc-basic + rc-reuse + iterative-drop (depends: 5, 13, 14, 16) [unspecified-high]
├── Task 19: Test projects — weak-ref (depends: 5, 15) [unspecified-high]

Wave 5 (Documentation — depend on Wave 4):
├── Task 20: README.md — memory model docs + weak ref guide + optimization strategies (depends: 17-19) [writing]
├── Task 21: language-spec/requirements/memory-model.md — formal specification (depends: 17-19) [writing]

Wave FINAL (After ALL tasks — 4 parallel reviews, then user okay):
├── Task F1: Plan compliance audit (oracle)
├── Task F2: Code quality review (unspecified-high)
├── Task F3: Real manual QA (unspecified-high)
└── Task F4: Scope fidelity check (deep)
-> Present results -> Get explicit user okay

Critical Path: T1 → T6 → T9 → T11 → T13 → T14 → T16 → T18 → T20 → F1-F4 → user okay
Parallel Speedup: ~65% faster than sequential
Max Concurrent: 5 (Waves 1, 2, 3)
```

### Dependency Matrix

| Task | Depends On | Blocks | Wave |
|------|-----------|--------|------|
| 1 | — | 6, 7 | 1 |
| 2 | — | 6, 9 | 1 |
| 3 | — | 13, 14, 15 | 1 |
| 4 | — | 8, 11 | 1 |
| 5 | — | 17, 18, 19 | 1 |
| 6 | 1, 2 | 9, 12, 17 | 2 |
| 7 | 1 | 8 | 2 |
| 8 | 4, 7 | 11, 15 | 2 |
| 9 | 2, 6 | 10, 11, 12 | 2 |
| 10 | 9 | 17 | 2 |
| 11 | 4, 8, 9 | 13, 16 | 3 |
| 12 | 6, 9 | 17 | 3 |
| 13 | 3, 11 | 14, 15, 16, 18 | 3 |
| 14 | 3, 13 | 18 | 3 |
| 15 | 3, 8, 13 | 19 | 3 |
| 16 | 11, 13 | 18 | 4 |
| 17 | 5, 12 | 20, 21 | 4 |
| 18 | 5, 13, 14, 16 | 20, 21 | 4 |
| 19 | 5, 15 | 20, 21 | 4 |
| 20 | 17, 18, 19 | — | 5 |
| 21 | 17, 18, 19 | — | 5 |

### Agent Dispatch Summary

- **Wave 1**: **5 tasks** — T1 → `quick`, T2 → `quick`, T3 → `unspecified-high`, T4 → `quick`, T5 → `quick`
- **Wave 2**: **5 tasks** — T6 → `unspecified-high`, T7 → `quick`, T8 → `deep`, T9 → `deep`, T10 → `deep`
- **Wave 3**: **5 tasks** — T11 → `deep`, T12 → `unspecified-high`, T13 → `deep`, T14 → `deep`, T15 → `deep`
- **Wave 4**: **4 tasks** — T16 → `ultrabrain`, T17 → `unspecified-high`, T18 → `unspecified-high`, T19 → `unspecified-high`
- **Wave 5**: **2 tasks** — T20 → `writing`, T21 → `writing`
- **FINAL**: **4 tasks** — F1 → `oracle`, F2 → `unspecified-high`, F3 → `unspecified-high`, F4 → `deep`

---

## TODOs

> Implementation + Test = ONE Task. Never separate.
> EVERY task has: Recommended Agent Profile + Parallelization info + QA Scenarios.
> TDD: RED (failing test) → GREEN (minimal impl) → REFACTOR (clean up). ALL THREE. ALWAYS.

- [x] 1. Add `ref` and `weak` tokens to lexer

  **What to do**:
  - RED: Write unit tests in `src/lexer.rs` (or its test module) that assert:
    - Lexing `ref` produces `TokenType::Ref`
    - Lexing `weak` produces `TokenType::Weak`
    - Lexing `mutable ref` produces `TokenType::Mutable` followed by `TokenType::Ref` (two tokens)
    - `ref` and `weak` appear in `RESERVED_KEYWORDS` array
    - `ref` and `weak` cannot be used as identifiers
  - GREEN: Add `Ref` and `Weak` variants to `TokenType` enum in `src/token.rs`. Add `"ref"` and `"weak"` entries to `RESERVED_KEYWORDS` array and `keywords` BTreeMap in `src/lexer.rs`.
  - REFACTOR: Ensure alphabetical ordering in keyword maps. Verify `Display`/`Debug` implementations for new variants are consistent with existing patterns. Clean up any duplication.

  **Must NOT do**:
  - Do not add any parsing logic — that's Task 6/7
  - Do not add `HashMap` — use `BTreeMap` as existing code does
  - Do not import from `std` — use `alloc`/`core`

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Small, focused change to two files (token.rs, lexer.rs) with clear patterns to follow
  - **Skills**: `[]`
    - No special skills needed — straightforward Rust enum + map additions

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1 (with Tasks 2, 3, 4, 5)
  - **Blocks**: Tasks 6, 7
  - **Blocked By**: None (can start immediately)

  **References**:

  **Pattern References** (existing code to follow):
  - `src/token.rs:1-486` — Full `TokenType` enum. New variants `Ref` and `Weak` should be added following the alphabetical/categorical ordering of existing variants like `Mutable`, `Match`, `Return`. Look at how `Mutable` is defined as a keyword variant.
  - `src/lexer.rs:14-57` — `RESERVED_KEYWORDS` array (`&[&str]`). Add `"ref"` and `"weak"` in alphabetical position. This array is used by parser tests too, so order matters for consistency.
  - `src/lexer.rs:74-120` — `keywords` BTreeMap initialization in `Lexer::new()`. Add `"ref" => TokenType::Ref` and `"weak" => TokenType::Weak` entries.

  **Test References** (testing patterns to follow):
  - `src/lexer.rs` test module (bottom of file) — Existing lexer tests that assert `TokenType` output for given input strings. Follow the same pattern for `ref` and `weak` tokens.

  **Acceptance Criteria**:

  **TDD (tests first):**
  - [ ] Test file/module updated: `src/lexer.rs` test module
  - [ ] `cargo test` runs new token tests → PASS

  **QA Scenarios:**

  ```
  Scenario: ref keyword lexes correctly
    Tool: Bash
    Preconditions: Code compiles (cargo build succeeds)
    Steps:
      1. Run `cargo test -- --test-threads=1 2>&1 | grep -E "(test result|ref)"` to verify ref-related tests pass
      2. Run `grep -n "Ref" src/token.rs` to verify Ref variant exists in TokenType
      3. Run `grep -n '"ref"' src/lexer.rs` to verify "ref" is in keywords map
    Expected Result: All tests pass, Ref variant exists, "ref" is registered as keyword
    Failure Indicators: Test failures, missing variant, missing keyword registration
    Evidence: .sisyphus/evidence/task-1-ref-token.txt

  Scenario: weak keyword lexes correctly
    Tool: Bash
    Preconditions: Code compiles
    Steps:
      1. Run `cargo test -- --test-threads=1 2>&1 | grep -E "(test result|weak)"` to verify weak-related tests pass
      2. Run `grep -n "Weak" src/token.rs` to verify Weak variant exists in TokenType
      3. Run `grep -n '"weak"' src/lexer.rs` to verify "weak" is in keywords map and RESERVED_KEYWORDS
    Expected Result: All tests pass, Weak variant exists, "weak" is registered
    Failure Indicators: Test failures, missing variant, missing keyword
    Evidence: .sisyphus/evidence/task-1-weak-token.txt

  Scenario: ref and weak cannot be used as identifiers
    Tool: Bash
    Preconditions: Lexer tests exist
    Steps:
      1. Verify test exists that lexes `ref` and asserts it's NOT an Identifier token
      2. Verify test exists that lexes `weak` and asserts it's NOT an Identifier token
      3. Run `cargo test` — all pass
    Expected Result: ref/weak are keywords, not identifiers
    Failure Indicators: Lexer produces Identifier instead of Ref/Weak
    Evidence: .sisyphus/evidence/task-1-not-identifier.txt
  ```

  **Commit**: YES (groups with Task 2)
  - Message: `feat(lexer): add ref and weak tokens with PassingMode AST support`
  - Files: `src/token.rs`, `src/lexer.rs`
  - Pre-commit: `cargo test`

- [x] 2. Add `PassingMode` enum and update `Parameter` struct in AST

  **What to do**:
  - RED: Write unit tests that assert:
    - `PassingMode` enum has variants `Owned`, `Ref`, `MutableRef`
    - `Parameter` struct has a `passing_mode: PassingMode` field
    - Default `PassingMode` is `Owned` (for backward compatibility)
    - `PassingMode` derives `Debug`, `Clone`, `PartialEq`
  - GREEN: Add `PassingMode` enum to `src/ast/types.rs`. Add `passing_mode: PassingMode` field to `Parameter` struct. Update ALL existing `Parameter` construction sites to use `PassingMode::Owned` as default. This will require updating the parser and anywhere `Parameter` is constructed.
  - REFACTOR: Ensure `PassingMode` is properly exported from ast module. Clean up any verbose construction patterns — consider a `Parameter::new()` or `Parameter::owned()` convenience constructor if it reduces boilerplate at call sites.

  **Must NOT do**:
  - Do not add parsing logic for `ref`/`mutable ref` — that's Task 6
  - Do not modify type checker behavior — that's Tasks 8-10
  - Do not add first-class reference types — refs are parameter-only annotations

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Primarily adding an enum and a field to a struct, then fixing compile errors at construction sites
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1 (with Tasks 1, 3, 4, 5)
  - **Blocks**: Tasks 6, 9
  - **Blocked By**: None (can start immediately)

  **References**:

  **Pattern References**:
  - `src/ast/types.rs:1-227` — Full file. `Parameter` struct at approx line 200+ has `name: Token`, `param_type: Option<Type>`, `span: Span`. Add `passing_mode: PassingMode` field. Look at how `Type` enum is defined for the pattern to follow for `PassingMode`.
  - `src/ast/types.rs:1-30` — `Type` enum definition — follow same derive pattern (`Debug, Clone, PartialEq`) for `PassingMode`.

  **API/Type References**:
  - `src/ast.rs:1-997` — Full AST. Search for `Parameter {` or `Parameter::` to find all construction sites that need updating. Key locations: function declarations, lambda expressions, method definitions.

  **WHY Each Reference Matters**:
  - `ast/types.rs` is where `Parameter` lives — direct modification target
  - `ast.rs` constructs `Parameter` in multiple places — all must be updated or compilation fails
  - Parser files also construct `Parameter` — they'll need `passing_mode: PassingMode::Owned` added

  **Acceptance Criteria**:

  **TDD:**
  - [ ] Tests written in `src/ast/types.rs` test module or similar
  - [ ] `cargo test` → PASS (all existing tests still pass with new field defaulting to Owned)

  **QA Scenarios:**

  ```
  Scenario: PassingMode enum exists with correct variants
    Tool: Bash
    Preconditions: Code compiles
    Steps:
      1. Run `grep -n "PassingMode" src/ast/types.rs` to verify enum definition
      2. Verify variants: Owned, Ref, MutableRef all present
      3. Run `cargo build` to verify no compile errors
    Expected Result: PassingMode enum with 3 variants, code compiles
    Failure Indicators: Missing variants, compile errors at Parameter construction sites
    Evidence: .sisyphus/evidence/task-2-passing-mode.txt

  Scenario: All existing tests pass (no regression)
    Tool: Bash
    Preconditions: PassingMode added, all construction sites updated
    Steps:
      1. Run `cargo test 2>&1 | tail -5` to check test results
      2. Verify 0 failures
    Expected Result: All existing tests pass — PassingMode::Owned is backward compatible
    Failure Indicators: Any test failure indicates a construction site was missed
    Evidence: .sisyphus/evidence/task-2-no-regression.txt

  Scenario: Parameter construction sites all updated
    Tool: Bash
    Preconditions: New field added
    Steps:
      1. Run `cargo build 2>&1` — should have 0 errors
      2. If errors, they'll point to every place Parameter is constructed without passing_mode
    Expected Result: Clean build
    Failure Indicators: "missing field `passing_mode`" errors
    Evidence: .sisyphus/evidence/task-2-build-clean.txt
  ```

  **Commit**: YES (groups with Task 1)
  - Message: `feat(lexer): add ref and weak tokens with PassingMode AST support`
  - Files: `src/ast/types.rs`, `src/ast.rs`, parser files
  - Pre-commit: `cargo test`

- [x] 3. Create `opal_rc.c` and `opal_rc.h` C runtime for reference counting

  **What to do**:
  - RED: Write a C test file (or integration test in Rust) that exercises:
    - `opal_rc_alloc(size, drop_fn)` — allocates RC object with header (refcount=1, weak_count=0, drop function pointer)
    - `opal_rc_inc(obj)` — increments refcount
    - `opal_rc_dec(obj)` — decrements refcount; when 0, calls iterative drop
    - `opal_rc_drop_iterative(obj)` — work-list based iterative drop (no recursion)
    - `opal_weak_alloc(strong_obj)` — creates weak reference (increments weak_count)
    - `opal_weak_upgrade(weak)` — returns strong ref if alive, NULL if dead (for Option<T> mapping)
    - `opal_weak_dec(weak)` — decrements weak_count; frees header when both counts are 0
  - GREEN: Implement the C runtime in `runtime/opal_rc.c` with header `runtime/opal_rc.h`. RC object layout: `[refcount: size_t | weak_count: size_t | drop_children_fn: fn_ptr | payload...]`. The `drop_children_fn` is called by iterative drop to enqueue child RC objects onto the work-list. Add `#include "opal_rc.c"` to `runtime/opal_runtime.c` and function declarations to `runtime/opal_runtime.h`.
  - **CRITICAL LINKING NOTE**: This repo has NO `build.rs`. The C runtime is embedded into the compiler via `include_str!` in `src/compiler.rs` (look for `RUNTIME_SOURCE` or similar constant that concatenates `include_str!("../runtime/*.c")`). You MUST also add `runtime/opal_rc.c` to that `include_str!` concat in `src/compiler.rs`, otherwise the RC functions will not be available at link time despite being in the `runtime/` directory. Check `src/compiler.rs` for the exact pattern used for `opal_string.c`, `opal_error.c`, etc. and replicate for `opal_rc.c`.
  - REFACTOR: Extract constants for header offsets. Add inline helper macros for header access. Ensure the work-list for iterative drop uses a simple stack (array-based, grows as needed). Add clear comments explaining the memory layout and ABI stability for future module imports. Use `static` for internal helpers, expose only the public API.

  **Must NOT do**:
  - **No recursive drops** — must use iterative work-list
  - **No cycle collector** — weak refs only
  - **No malloc for every work-list operation** — pre-allocate reasonable stack, grow if needed
  - **No platform-specific code** — portable C99

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
    - Reason: C runtime implementation requires careful memory layout design, ABI considerations, and iterative algorithm design. More complex than a "quick" task.
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1 (with Tasks 1, 2, 4, 5)
  - **Blocks**: Tasks 13, 14, 15
  - **Blocked By**: None (can start immediately)

  **References**:

  **Pattern References**:
  - `runtime/opal_runtime.c:1-6` — Include aggregation pattern. Just `#include` statements for sub-files. Add `#include "opal_rc.c"` here.
  - `runtime/opal_runtime.h:1-65` — Function declaration pattern. Add `opal_rc_alloc`, `opal_rc_inc`, `opal_rc_dec`, `opal_weak_alloc`, `opal_weak_upgrade`, `opal_weak_dec` declarations here.
  - `runtime/opal_string.c` — Example of a runtime sub-module: uses `static` for internal helpers, exposes public functions matching declarations in opal_runtime.h.
  - `runtime/opal_error.c` — Another sub-module example for error handling patterns.

  **External References**:
  - Perceus paper (Reinking et al., 2021) — Section on reference counting object layout and drop semantics
  - Lean 4 runtime `lean_object` header — Inspiration for RC object layout with embedded function pointers

  **WHY Each Reference Matters**:
  - `opal_runtime.c/h` define the inclusion and declaration patterns — must match exactly
  - Other runtime .c files show the project's C style conventions (naming, static usage, comment style)
  - The RC object header layout must be ABI-stable for future module imports (user constraint)

  **Acceptance Criteria**:

  **TDD:**
  - [ ] Test file or integration test exercising RC alloc/inc/dec/drop/weak functions
  - [ ] Tests pass for: alloc returns non-null, inc/dec adjust counts, dec at 0 triggers drop, weak upgrade after drop returns NULL

  **QA Scenarios:**

  ```
  Scenario: RC object lifecycle (alloc → inc → dec → drop)
    Tool: Bash
    Preconditions: opal_rc.c and opal_rc.h created and included in opal_runtime.c
    Steps:
      1. Run `cargo build 2>&1` to verify the C runtime compiles (it's linked via build.rs)
      2. Write a minimal test.op file that creates an object — compile and run
      3. Verify no segfaults or memory errors (run under basic checks)
    Expected Result: Object created, refcount managed, clean exit
    Failure Indicators: Segfault, memory leak, compile error in C code
    Evidence: .sisyphus/evidence/task-3-rc-lifecycle.txt

  Scenario: Iterative drop does NOT use recursion
    Tool: Bash
    Preconditions: opal_rc.c implemented
    Steps:
      1. Run `grep -n "opal_rc_drop_iterative\|opal_rc_dec" runtime/opal_rc.c` to find the drop function
      2. Verify it uses a work-list/stack loop pattern (while loop with stack), NOT recursive calls to itself
      3. Run `grep -c "opal_rc_dec" runtime/opal_rc.c` — should NOT appear inside opal_rc_drop_iterative (no recursive dec from within drop)
    Expected Result: Drop function uses iterative work-list pattern, no recursive calls
    Failure Indicators: Self-recursive calls to dec/drop inside the drop function
    Evidence: .sisyphus/evidence/task-3-iterative-drop.txt

  Scenario: Weak ref upgrade returns NULL after strong dies
    Tool: Bash
    Preconditions: Weak ref functions implemented
    Steps:
      1. Verify `opal_weak_upgrade` function exists in opal_rc.c
      2. Verify it checks refcount > 0 before returning strong ref
      3. Verify it returns NULL when refcount == 0
    Expected Result: Weak upgrade correctly returns NULL for dead objects
    Failure Indicators: Returns dangling pointer, crashes on upgrade
    Evidence: .sisyphus/evidence/task-3-weak-upgrade.txt
  ```

  **Commit**: YES
  - Message: `feat(runtime): add RC object header with inc/dec/iterative-drop/weak support`
  - Files: `runtime/opal_rc.c`, `runtime/opal_rc.h`, `runtime/opal_runtime.c`, `runtime/opal_runtime.h`
  - Pre-commit: `cargo build`

- [x] 4. Extend `CoreType` and `MemoryLayout` for RC metadata tracking

  **What to do**:
  - RED: Write unit tests that assert:
    - `CoreType` can represent types that need RC wrapping (heap-allocated types: String, Array, structs, ADTs)
    - `MemoryLayout` can report RC object header size (2× `size_t` + function pointer = ~24 bytes on 64-bit)
    - A helper function identifies which `CoreType` variants need RC (value types like Int/Float/Bool do NOT)
    - `CoreType` can represent `Weak<T>` — either a new variant or via `Generic` with special handling
  - GREEN: Add `needs_rc(&self) -> bool` method to `CoreType` that returns true for heap-allocated types (String, Array, Generic/ADT) and false for value types (integers, floats, booleans, Unit). Add `rc_header_layout()` to `MemoryLayout` returning the header size/alignment. Decide on `Weak<T>` representation: likely use existing `CoreType::Generic` with name "Weak" and inner type — leverage the existing generic infrastructure.
  - REFACTOR: Ensure `needs_rc()` is exhaustive (all CoreType variants covered). Document why each type does/doesn't need RC. Add constants for RC header field offsets. Consider whether a `TypeInfo` wrapper struct would be cleaner than methods on `CoreType`.

  **Must NOT do**:
  - Do not modify codegen — that's Wave 3
  - Do not modify type checker — that's Wave 2
  - Do not add `HashMap` — use `BTreeMap`
  - Do not import from `std`

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Adding methods to existing types and a helper function. Well-scoped.
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1 (with Tasks 1, 2, 3, 5)
  - **Blocks**: Tasks 8, 11
  - **Blocked By**: None (can start immediately)

  **References**:

  **Pattern References**:
  - `src/type_system/types.rs:1-201` — Full `CoreType` enum. Add `needs_rc()` method to the impl block. Variants: Int8/16/32/64, UInt8/16/32/64, Float32/64, String, Boolean, Unit, Variable, Array, Function, Generic. String, Array, and Generic (for structs/ADTs) need RC. Primitives do not.
  - `src/type_system/memory.rs:1-82` — `MemoryLayout` struct (size, align) and `CoreType::memory_layout()` method. Add `MemoryLayout::rc_header()` or similar. The existing method maps CoreType → MemoryLayout with sizes like Int32→4, Float64→8, etc.

  **API/Type References**:
  - `src/type_system/types.rs:CoreType::Generic` — Has `name: String, type_args: Vec<CoreType>`. `Weak<T>` would be `Generic { name: "Weak", type_args: vec![inner_type] }`. Same pattern as `Option<T>`.

  **WHY Each Reference Matters**:
  - `types.rs` is where `CoreType` lives — direct modification target for `needs_rc()`
  - `memory.rs` is where `MemoryLayout` lives — needs RC header layout constants
  - Understanding `Generic` variant is crucial for `Weak<T>` representation strategy

  **Acceptance Criteria**:

  **TDD:**
  - [ ] Tests in `src/type_system/types.rs` or `src/type_system/memory.rs` test modules
  - [ ] `cargo test` → PASS

  **QA Scenarios:**

  ```
  Scenario: needs_rc correctly classifies types
    Tool: Bash
    Preconditions: Code compiles
    Steps:
      1. Run `cargo test -- needs_rc 2>&1` to run needs_rc-related tests
      2. Verify tests cover: Int32 → false, String → true, Array → true, Boolean → false, Generic → true
    Expected Result: All classification tests pass
    Failure Indicators: Wrong classification for any type
    Evidence: .sisyphus/evidence/task-4-needs-rc.txt

  Scenario: RC header layout is correct
    Tool: Bash
    Preconditions: MemoryLayout::rc_header() implemented
    Steps:
      1. Run `cargo test -- rc_header 2>&1` to verify header layout test
      2. Verify header size accounts for refcount + weak_count + drop_fn pointer
    Expected Result: Header layout test passes with correct sizes
    Failure Indicators: Wrong size/alignment for RC header
    Evidence: .sisyphus/evidence/task-4-rc-header.txt

  Scenario: No regressions in existing type system tests
    Tool: Bash
    Preconditions: Changes made to types.rs and memory.rs
    Steps:
      1. Run `cargo test 2>&1 | tail -5` to check overall test results
      2. Verify 0 failures
    Expected Result: All existing tests still pass
    Failure Indicators: Any failure in existing type system tests
    Evidence: .sisyphus/evidence/task-4-no-regression.txt
  ```

  **Commit**: YES
  - Message: `feat(types): extend CoreType and MemoryLayout for RC tracking`
  - Files: `src/type_system/types.rs`, `src/type_system/memory.rs`
  - Pre-commit: `cargo test`

- [x] 5. Create test project scaffolding for memory model features

  **What to do**:
  - RED: Write integration test stubs in `tests/integration_e2e.rs` for 7 new test project directories. Each test should follow the existing pattern: read `test-projects/<name>/src/main.op`, compile with `compile_program()`, run the binary, and assert on stdout content. Tests will fail because directories don't exist yet.
  - GREEN: Create the following test project directories under `test-projects/`, matching the existing convention of `opal.toml` + `src/main.op` + `.gitignore` + `README.md`:
    - `test-projects/ref-basic/` — `opal.toml`, `src/main.op` (placeholder), `.gitignore`, `README.md`
    - `test-projects/mutable-ref/` — same structure
    - `test-projects/ref-compile-fail/` — same structure (tests that ref can't be stored/returned — integration test asserts compilation FAILS with expected error substring)
    - `test-projects/rc-basic/` — same structure
    - `test-projects/rc-reuse/` — same structure
    - `test-projects/iterative-drop/` — same structure
    - `test-projects/weak-ref/` — same structure
    All `src/main.op` files start as minimal placeholders (e.g., `# TODO: implement test`) with an `entry main` stub. They'll be filled in by Tasks 17-19.
  - REFACTOR: Ensure directory naming follows existing test project conventions (lowercase-kebab-case). Verify integration test pattern matches existing tests in `integration_e2e.rs` — specifically: `prepare_dir`, read source from `src/main.op`, `compile_program`, run binary, assert stdout, `cleanup_dir`.

  **Must NOT do**:
  - Do not write actual test program content — that's Tasks 17-19
  - Do not modify existing test projects
  - Do not add complex integration test logic — just the scaffolding stubs

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Creating directories and placeholder files. Very straightforward.
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1 (with Tasks 1, 2, 3, 4)
  - **Blocks**: Tasks 17, 18, 19
  - **Blocked By**: None (can start immediately)

  **References**:

  **Pattern References**:
  - `test-projects/hello-world/` — Canonical test project structure: `opal.toml`, `.gitignore`, `README.md`, `src/main.op`. Follow this exact convention.
  - `test-projects/hello-world/opal.toml` — Minimal project config: `name = "hello-world"` and `version = "1.0.0"`.
  - `test-projects/hello-world/src/main.op` — Source file with `entry main = f(args: string[]): void =>` pattern.
  - `tests/integration_e2e.rs:176-245` — `hello_world_compiles_links_and_runs` — THE canonical integration test pattern. Study this carefully: `prepare_dir(target)` → `fs::read_to_string(src/main.op)` → `compile_program(source, temp_dir)` → `Command::new(binary).output()` → assert stdout contains expected string → `cleanup_dir(target)`.

  **WHY Each Reference Matters**:
  - Existing test projects define the directory structure convention (`opal.toml`, `src/main.op`, etc.) — must match exactly
  - `integration_e2e.rs` hello_world test shows the EXACT integration test pattern to replicate — stubs must be compatible with this pattern

  **Acceptance Criteria**:

  **TDD:**
  - [ ] Integration test stubs added to `tests/integration_e2e.rs`
  - [ ] `cargo test --features integration` — new stubs either skip (with `#[ignore]`) or are marked as TODO

  **QA Scenarios:**

  ```
  Scenario: All 7 test project directories exist with correct structure
    Tool: Bash
    Preconditions: None
    Steps:
      1. Run `ls test-projects/ref-basic/opal.toml test-projects/ref-basic/src/main.op test-projects/mutable-ref/opal.toml test-projects/mutable-ref/src/main.op test-projects/ref-compile-fail/opal.toml test-projects/ref-compile-fail/src/main.op test-projects/rc-basic/opal.toml test-projects/rc-basic/src/main.op test-projects/rc-reuse/opal.toml test-projects/rc-reuse/src/main.op test-projects/iterative-drop/opal.toml test-projects/iterative-drop/src/main.op test-projects/weak-ref/opal.toml test-projects/weak-ref/src/main.op`
      2. Verify all 7 directories have opal.toml + src/main.op
    Expected Result: All 7 directories exist with standard project structure
    Failure Indicators: Missing directories, missing opal.toml or src/main.op
    Evidence: .sisyphus/evidence/task-5-scaffolding.txt

  Scenario: Existing test projects unmodified
    Tool: Bash
    Preconditions: New directories created
    Steps:
      1. Run `git diff test-projects/` (excluding new directories)
      2. Verify no changes to existing test project files
    Expected Result: Zero modifications to existing test projects
    Failure Indicators: Any diff in existing test project files
    Evidence: .sisyphus/evidence/task-5-no-modification.txt
  ```

  **Commit**: YES
  - Message: `test(projects): scaffold memory model test project directories`
  - Files: `test-projects/ref-basic/`, `test-projects/mutable-ref/`, `test-projects/ref-compile-fail/`, `test-projects/rc-basic/`, `test-projects/rc-reuse/`, `test-projects/iterative-drop/`, `test-projects/weak-ref/`, `tests/integration_e2e.rs`
  - Pre-commit: `ls test-projects/`

- [x] 6. Parse `ref` / `mutable ref` parameter annotations

  **What to do**:
  - RED: Write parser unit tests that assert:
    - `let foo = f(ref x: int32): int32 =>` parses to a `Parameter` with `passing_mode: PassingMode::Ref`
    - `let foo = f(mutable ref x: int32): int32 =>` parses to `PassingMode::MutableRef`
    - `let foo = f(x: int32): int32 =>` still parses to `PassingMode::Owned` (backward compat)
    - `ref` / `mutable ref` only valid in parameter position (not in let bindings, not in return types)
    - Multiple params: `let foo = f(ref a: int32, mutable ref b: string, c: boolean): void =>` — mixed modes parse correctly
    - Lambda params: `(ref x: int32) => x` — ref works in lambda parameter position too
  - GREEN: Modify the parameter parsing logic in the parser to check for `TokenType::Ref` or `TokenType::Mutable` followed by `TokenType::Ref` at the start of a parameter. Set the corresponding `PassingMode` on the constructed `Parameter`. The default remains `Owned` when neither token is present.
  - REFACTOR: Extract parameter mode parsing into a helper function (e.g., `parse_passing_mode(&mut self) -> PassingMode`) for reuse between function declarations and lambda expressions. Ensure error messages are clear: "unexpected `ref` outside parameter position" if ref appears elsewhere.

  **Must NOT do**:
  - Do not add type checking rules — that's Task 9
  - Do not add codegen for ref params — that's Task 12
  - Do not modify how parameters are stored after parsing — AST structure set in Task 2

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
    - Reason: Requires understanding the parser's parameter parsing flow across function declarations and lambdas. Needs careful handling of token lookahead for `mutable ref` (two-token sequence).
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2 (with Tasks 7, 8, 9 — but 9 depends on this)
  - **Blocks**: Tasks 9, 12, 17
  - **Blocked By**: Tasks 1, 2

  **References**:

  **Pattern References**:
  - `src/parser/` directory — The parser module. Need to find the parameter parsing function (likely in a declarations or functions submodule). Look for where `Parameter` structs are constructed.
  - `src/token.rs:TokenType::Mutable` — Already exists as a keyword. The parser needs to handle `Mutable` followed by `Ref` as a two-token sequence for `mutable ref`.

  **API/Type References**:
  - `src/ast/types.rs:Parameter` — The struct being constructed. After Task 2, it has `passing_mode: PassingMode` field.
  - `src/ast/types.rs:PassingMode` — The enum to set: `Owned`, `Ref`, `MutableRef`.

  **Test References**:
  - Parser test modules — Find existing parser tests for function declarations/parameter parsing. Follow the same test structure.

  **WHY Each Reference Matters**:
  - Parser directory is the modification target — must find exact parameter parsing location
  - Token types define what tokens to match (`Ref`, `Mutable`)
  - `Parameter` and `PassingMode` types define the output structure

  **Acceptance Criteria**:

  **TDD:**
  - [ ] Parser tests for ref/mutable ref parameter annotations
  - [ ] `cargo test` → PASS (new parser tests + all existing tests)

  **QA Scenarios:**

  ```
  Scenario: ref parameter parses correctly
    Tool: Bash
    Preconditions: Tasks 1 and 2 complete
    Steps:
      1. Run `cargo test -- parse 2>&1 | grep -E "(ref|PASS|FAIL)"` for parser tests involving ref
      2. Verify test for `let foo = f(ref x: int32): int32 =>` exists and passes
    Expected Result: Parser produces Parameter with PassingMode::Ref
    Failure Indicators: Parse error, wrong PassingMode
    Evidence: .sisyphus/evidence/task-6-ref-parse.txt

  Scenario: mutable ref parameter parses correctly (two-token sequence)
    Tool: Bash
    Preconditions: Tasks 1 and 2 complete
    Steps:
      1. Run parser test for `let foo = f(mutable ref x: int32): int32 =>`
      2. Verify it produces PassingMode::MutableRef (not Ref, not Owned)
    Expected Result: Two-token `mutable ref` correctly identified
    Failure Indicators: Parsed as just Mutable (without Ref), or as two separate things
    Evidence: .sisyphus/evidence/task-6-mutable-ref-parse.txt

  Scenario: No regression — existing function parsing unchanged
    Tool: Bash
    Preconditions: Parser modified
    Steps:
      1. Run `cargo test 2>&1 | tail -5`
      2. Verify 0 failures — all existing function/lambda tests pass
    Expected Result: Backward compatible — functions without ref still parse as Owned
    Failure Indicators: Any existing parser test failure
    Evidence: .sisyphus/evidence/task-6-no-regression.txt
  ```

  **Commit**: YES (groups with Task 7)
  - Message: `feat(parser): parse ref/mutable ref params and Weak<T> type`
  - Files: `src/parser/*.rs`
  - Pre-commit: `cargo test`

- [x] 7. Parse `Weak<T>` type syntax

  **What to do**:
  - RED: Write parser unit tests that assert:
    - `Weak<int32>` parses as `Type::Generic { name: "Weak", type_args: [Type::Basic("int32")] }`
    - `Weak<string>` parses correctly
    - `Weak<Weak<int32>>` — nested weak (decide: allow or error?)
    - `let x: Weak<MyStruct> = ...` — Weak usable in variable type annotations
    - `let foo = f(x: Weak<int32>): void =>` — Weak usable in parameter types (distinct from `ref` — Weak is a first-class type, ref is a parameter annotation)
  - GREEN: This may already work via existing generic type parsing! `Weak` is just a type name, and `<T>` is generic syntax. If the parser already handles `Option<T>` and `Array<T>`, then `Weak<T>` likely parses automatically once `weak` is a keyword (Task 1). Verify this. If not, add parsing support.
  - REFACTOR: If `Weak<T>` already parses via generic infrastructure, document WHY in a comment. If custom parsing was needed, justify why generics didn't suffice.

  **Must NOT do**:
  - Do not add type checker validation for Weak — that's Task 8
  - Do not add codegen for Weak — that's Task 15

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Likely a verification task — generic parsing may already handle this. If custom work needed, it's small.
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2 (with Tasks 6, 8)
  - **Blocks**: Task 8
  - **Blocked By**: Task 1

  **References**:

  **Pattern References**:
  - `src/parser/` — Type parsing logic. Find where `Type::Generic` is constructed. If `Option<int32>` is parsed there, `Weak<int32>` will follow the same path.
  - `src/ast/types.rs:Type::Generic` — The generic type AST node. Has `name` and `type_args`.

  **Test References**:
  - Parser tests for generic types — Look for tests parsing `Option<T>`, `Array<T>`, or similar generic syntax.

  **WHY Each Reference Matters**:
  - Parser type parsing shows if Weak<T> already works or needs custom support
  - Type::Generic shows the target AST structure

  **Acceptance Criteria**:

  **TDD:**
  - [ ] Parser test verifying `Weak<int32>` parses correctly
  - [ ] `cargo test` → PASS

  **QA Scenarios:**

  ```
  Scenario: Weak<T> parses as generic type
    Tool: Bash
    Preconditions: Task 1 complete (weak keyword exists)
    Steps:
      1. Run parser test for `Weak<int32>` type annotation
      2. Verify it produces Type::Generic with name "Weak" and params [int32]
    Expected Result: Weak<T> parsed correctly via generic infrastructure
    Failure Indicators: Parse error, or parsed as something other than Generic
    Evidence: .sisyphus/evidence/task-7-weak-parse.txt

  Scenario: Weak<T> works in different positions
    Tool: Bash
    Preconditions: Parser handles Weak<T>
    Steps:
      1. Test `let x: Weak<int32>` — type annotation position
      2. Test `let foo = f(x: Weak<int32>): void =>` — parameter type position
      3. Test `let bar = f(): Weak<int32> =>` — return type position (if supported)
    Expected Result: Weak<T> valid wherever types are valid
    Failure Indicators: Parse error in any valid type position
    Evidence: .sisyphus/evidence/task-7-weak-positions.txt
  ```

  **Commit**: YES (groups with Task 6)
  - Message: `feat(parser): parse ref/mutable ref params and Weak<T> type`
  - Files: `src/parser/*.rs`
  - Pre-commit: `cargo test`

- [x] 8. Register `Weak<T>` in type checker with `Option<T>` upgrade semantics

  **What to do**:
  - RED: Write type checker unit tests that assert:
    - `Weak<int32>` is recognized as a valid generic type (not an "unknown type" error)
    - `Weak<T>` has exactly 1 type parameter (error on `Weak<>` or `Weak<A, B>`)
    - `.upgrade()` method on `Weak<T>` returns `Option<T>` (using the already-registered `Option<T>` generic)
    - `Weak<T>` can be stored in variables (it's first-class, unlike `ref`)
    - `Weak<T>` can be passed as parameter and returned from functions
    - Type error when trying to use `Weak<T>` value directly without upgrading (can't access inner value without upgrade)
  - GREEN: Register `Weak` as a built-in generic type in the type checker, alongside `Option`. Add method resolution for `.upgrade()` that returns `Option<T>`. The type checker should recognize `Weak<T>` via the same `CoreType::Generic` infrastructure used for `Option<T>`. Add `Weak` to the built-in type registry (find where `Option` is registered and follow that pattern).
  - REFACTOR: Ensure `Weak<T>` and `Option<T>` share registration infrastructure where possible. Document the relationship between `Weak<T>.upgrade()` → `Option<T>` clearly. Consider if other `Weak<T>` methods are needed (e.g., `.is_alive() -> boolean`).

  **Must NOT do**:
  - Do not implement codegen for Weak — that's Task 15
  - Do not add cycle collector logic — weak refs are the ONLY cycle mechanism
  - Do not use `?` syntax — use `Option<T>` explicitly for upgrade returns

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: Requires deep understanding of type checker's generic registration, method resolution, and how Option<T> is already registered. Multiple interacting subsystems.
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2 (can run alongside Tasks 9, 10 once Tasks 4, 7 are done)
  - **Blocks**: Tasks 11, 15
  - **Blocked By**: Tasks 4, 7

  **References**:

  **Pattern References**:
  - `src/type_system/checker.rs:186-286` — `register_standard_builtins()` method. This is where `Option` is registered (lines 251-257) as a built-in generic type via `self.environment.register_type("Option", CoreType::Generic { name: "Option", type_args: vec![] })`. Register `Weak` in the SAME method, following the EXACT same pattern. This is the PRIMARY reference.
  - `src/type_system/checker/` — 18 submodule files. Look for method resolution logic (likely in `expressions.rs` or a dedicated `methods.rs`) to add `.upgrade()` method that returns `Option<T>`.
  - `src/type_system/checker/generics.rs` — Handles generic type instantiation tracking. NOT where built-in types are registered — but relevant for understanding how generic type parameters are resolved when Weak<int32> is used.

  **API/Type References**:
  - `src/type_system/types.rs:CoreType::Generic` — `{ name: String, type_args: Vec<CoreType> }`. Both `Option<T>` and `Weak<T>` use this. Note: field is `type_args`, NOT `type_params`.
  - `src/type_system/checker.rs:TypeChecker` — Main type checker struct. `register_standard_builtins()` is called in its constructor.

  **WHY Each Reference Matters**:
  - `checker.rs:register_standard_builtins()` is the EXACT location where Option<T> is registered — replicate for Weak<T>
  - Method resolution location needed for `.upgrade()` method
  - `CoreType::Generic` with `type_args` (not `type_params`) confirms the representation strategy

  **Acceptance Criteria**:

  **TDD:**
  - [ ] Type checker tests for Weak<T> validation and upgrade semantics
  - [ ] `cargo test` → PASS

  **QA Scenarios:**

  ```
  Scenario: Weak<T> recognized as valid type
    Tool: Bash
    Preconditions: Tasks 4, 7 complete
    Steps:
      1. Run type checker test that validates `Weak<int32>` is accepted
      2. Run type checker test that `Weak<>` (no params) is rejected with error
      3. Run type checker test that `Weak<A, B>` (too many params) is rejected
    Expected Result: Proper arity checking for Weak<T>
    Failure Indicators: Weak<T> not recognized, or wrong arity accepted
    Evidence: .sisyphus/evidence/task-8-weak-valid.txt

  Scenario: upgrade() returns Option<T>
    Tool: Bash
    Preconditions: Weak<T> registered, Option<T> already exists
    Steps:
      1. Run type checker test: `let w: Weak<int32> = ...; let opt = w.upgrade();` — opt should be `Option<int32>`
      2. Verify the return type of .upgrade() matches Option<T> exactly
    Expected Result: .upgrade() correctly typed as returning Option<T>
    Failure Indicators: Wrong return type, method not found
    Evidence: .sisyphus/evidence/task-8-upgrade-type.txt

  Scenario: Weak<T> is first-class (can store, pass, return)
    Tool: Bash
    Preconditions: Weak<T> type checking works
    Steps:
      1. Test `let w: Weak<int32> = ...` — storable in variable (accepted)
      2. Test `let foo = f(w: Weak<int32>): void =>` — passable as parameter (accepted)
      3. Test `let bar = f(): Weak<int32> =>` — returnable (accepted)
    Expected Result: Weak<T> is first-class, unlike ref which is second-class
    Failure Indicators: Type error for storing/passing/returning Weak<T>
    Evidence: .sisyphus/evidence/task-8-weak-first-class.txt
  ```

  **Commit**: YES
  - Message: `feat(checker): register Weak<T> built-in generic with Option<T> upgrade`
  - Files: `src/type_system/checker/*.rs`
  - Pre-commit: `cargo test`

- [x] 9. Enforce second-class reference rules in type checker

  **What to do**:
  - RED: Write type checker tests that assert REJECTION of:
    - `let x: ref int32 = ...` — cannot declare `ref` variable (ref is param-only)
    - `let foo = f(ref x: int32): ref int32 =>` — cannot return a ref
    - `let foo = f(ref x: int32): void => let y = x` — cannot assign ref to variable (escaping the parameter scope)
    - `let foo = f(ref x: int32): void => bar(x)` where `bar` takes `int32` (not `ref int32`) — cannot pass ref where owned is expected
    - `let foo = f(ref x: int32): void => let f = () => x` — cannot capture ref in closure/lambda
    - Storing ref in a struct field — forbidden
  - Also test ACCEPTANCE of:
    - `let foo = f(ref x: int32): void => print(x)` — can read ref value (pass to read-only operations)
    - `let foo = f(ref x: int32, ref y: int32): void =>` — multiple refs to same type allowed (they're read-only)
    - `let foo = f(ref x: int32): void => bar(ref x)` where `bar` takes `ref int32` — can pass ref to ref param
  - GREEN: In the type checker, add validation passes:
    - When checking function declarations: verify `ref`/`mutable ref` only appear on parameters
    - When checking assignments inside functions: track which variables are `ref` and prevent them from being assigned to non-ref locations
    - When checking return statements: verify the returned expression doesn't involve ref parameters
    - When checking closures/lambdas: verify captured variables don't include ref params from outer scope
  - REFACTOR: Centralize ref-escaping logic into a dedicated helper (e.g., `check_ref_escape(&self, expr, param_refs)`) that can be called from assignments, returns, and captures. Ensure error messages are specific and helpful: "cannot store a borrowed reference — `ref` parameters are read-only and cannot outlive the function call".

  **Must NOT do**:
  - Do not implement mutable ref aliasing rules — that's Task 10
  - Do not add codegen — that's Task 12
  - Do not modify the parser — that's Task 6
  - Do not make refs first-class (no storing, returning, or capturing)

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: Complex type checker logic involving escape analysis across assignments, returns, closures. Multiple checker submodules affected. Needs careful understanding of scope and variable tracking.
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES (alongside Task 8, after Tasks 2 and 6 complete)
  - **Parallel Group**: Wave 2
  - **Blocks**: Tasks 10, 11, 12
  - **Blocked By**: Tasks 2, 6

  **References**:

  **Pattern References**:
  - `src/type_system/checker/declarations.rs` — Function declaration type checking. Add ref parameter validation here — verify ref annotations are only on params, not return types or let bindings.
  - `src/type_system/checker/expressions.rs` — Expression type checking. Add ref-escape checks on assignments and variable usage.
  - `src/type_system/checker/call_resolution.rs` — Function call resolution. Ensure ref params can only be passed to ref params (not owned params).
  - `src/type_system/environment.rs` — Type environment / scope tracking. May need to track which variables are ref-bound for escape analysis.

  **API/Type References**:
  - `src/ast/types.rs:PassingMode` — The enum to check: `Ref`, `MutableRef`, `Owned`.
  - `src/ast/types.rs:Parameter` — The struct with `passing_mode` field.

  **WHY Each Reference Matters**:
  - `declarations.rs` is where function params are validated — primary modification target
  - `expressions.rs` handles assignments and usage — needed for escape prevention
  - `call_resolution.rs` handles call-site argument matching — ref-to-ref passing rules
  - `environment.rs` may need ref-tracking additions for scope-aware analysis

  **Acceptance Criteria**:

  **TDD:**
  - [ ] Tests for all rejection cases (6+ reject tests) and acceptance cases (3+ accept tests)
  - [ ] `cargo test` → PASS

  **QA Scenarios:**

  ```
  Scenario: ref cannot be stored in a variable
    Tool: Bash
    Preconditions: Tasks 2, 6 complete
    Steps:
      1. Run type checker test: `let foo = f(ref x: int32): void => let y = x` — must be rejected
      2. Verify error message mentions "cannot store" or "borrowed reference" or similar
    Expected Result: Type error rejecting ref-to-variable assignment
    Failure Indicators: No error (ref escapes to variable), wrong error message
    Evidence: .sisyphus/evidence/task-9-no-store.txt

  Scenario: ref cannot be returned
    Tool: Bash
    Preconditions: Type checker validates returns
    Steps:
      1. Run test: `let foo = f(ref x: int32): int32 => return x` — must be rejected
      2. Verify error mentions "cannot return" borrowed reference
    Expected Result: Type error preventing ref return
    Failure Indicators: Return accepted (ref escapes function)
    Evidence: .sisyphus/evidence/task-9-no-return.txt

  Scenario: ref CAN be passed to another ref parameter
    Tool: Bash
    Preconditions: Ref rules implemented
    Steps:
      1. Run test: `let bar = f(ref y: int32): void => return void` then `let foo = f(ref x: int32): void => bar(ref x)` — must be ACCEPTED
      2. Verify no type errors
    Expected Result: Ref-to-ref passing is valid
    Failure Indicators: False rejection of valid ref passing
    Evidence: .sisyphus/evidence/task-9-ref-to-ref.txt

  Scenario: ref cannot be captured in closure
    Tool: Bash
    Preconditions: Closure capture tracking works
    Steps:
      1. Run test: `let foo = f(ref x: int32): void => let f = () => x` — must be rejected
      2. Verify error mentions closure capture of borrowed reference
    Expected Result: Type error preventing ref capture
    Failure Indicators: Closure captures ref without error
    Evidence: .sisyphus/evidence/task-9-no-capture.txt
  ```

  **Commit**: YES (groups with Task 10)
  - Message: `feat(checker): enforce second-class ref rules and mutable ref aliasing`
  - Files: `src/type_system/checker/*.rs`
  - Pre-commit: `cargo test`

- [x] 10. Enforce mutable ref aliasing at call sites

  **What to do**:
  - RED: Write type checker tests that assert REJECTION of:
    - `let foo = f(mutable ref a: int32, mutable ref b: int32): void => return void` then `foo(mutable ref x, mutable ref x)` — same variable passed as two mutable refs (aliasing violation)
    - `let foo = f(mutable ref a: SomeStruct, ref b: SomeStruct): void => return void` then `foo(mutable ref x, ref x)` — mutable ref + immutable ref to same value (read-write conflict)
    - `let bar = f(mutable ref a: int32[], mutable ref b: int32[]): void => return void` then `bar(mutable ref arr, mutable ref arr)` — same array as two mutable refs
  - Also test ACCEPTANCE of:
    - `foo(mutable ref x, mutable ref y)` — different variables, same type → allowed
    - `foo(ref x, ref x)` — same variable as two immutable refs → allowed (no write conflict)
    - `foo(mutable ref x, ref y)` — different variables → allowed
  - GREEN: At each call site where `mutable ref` arguments are present, check that no two arguments refer to the same variable when at least one is `mutable ref`. This is a call-site analysis in `call_resolution.rs`: collect all `mutable ref` argument sources, verify no overlap with any other ref argument source.
  - REFACTOR: Extract aliasing check into a reusable function. Consider how this interacts with field access (e.g., `foo(mutable ref obj.field1, mutable ref obj.field2)` — same object, different fields — decide policy and document).

  **Must NOT do**:
  - Do not implement runtime aliasing checks — this is compile-time only
  - Do not implement borrow checking beyond call-site argument aliasing
  - Do not build a full borrow checker (this is not Rust — keep it simple)

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: Aliasing analysis at call sites requires understanding argument expressions and variable identity. Subtle edge cases with field access.
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: NO
  - **Parallel Group**: Wave 2 (sequential after Task 9)
  - **Blocks**: Task 17
  - **Blocked By**: Task 9

  **References**:

  **Pattern References**:
  - `src/type_system/checker/call_resolution.rs` — Function call resolution. This is where argument-parameter matching happens. Add aliasing check AFTER arguments are resolved but BEFORE the call is accepted.
  - `src/type_system/checker/expressions.rs` — May contain call expression handling that delegates to call_resolution.

  **API/Type References**:
  - `src/ast/types.rs:PassingMode::MutableRef` — The annotation to check for aliasing violations.
  - `src/ast.rs:Expr::Call` or similar — The call expression AST node containing arguments.

  **WHY Each Reference Matters**:
  - `call_resolution.rs` is where argument matching happens — add aliasing check here
  - Need to identify argument "source" (which variable) — may need to extract from expression AST

  **Acceptance Criteria**:

  **TDD:**
  - [ ] Tests for aliasing rejection (3+ cases) and acceptance (3+ cases)
  - [ ] `cargo test` → PASS

  **QA Scenarios:**

  ```
  Scenario: Same variable as two mutable refs is rejected
    Tool: Bash
    Preconditions: Task 9 complete
    Steps:
      1. Run test: `foo(mutable ref x, mutable ref x)` — must be rejected
      2. Verify error message mentions aliasing or "same variable" or "mutable borrow conflict"
    Expected Result: Compile-time error preventing mutable aliasing
    Failure Indicators: No error (undefined behavior at runtime)
    Evidence: .sisyphus/evidence/task-10-mutable-alias.txt

  Scenario: Different variables as mutable refs is accepted
    Tool: Bash
    Preconditions: Aliasing check implemented
    Steps:
      1. Run test: `foo(mutable ref x, mutable ref y)` — must be ACCEPTED
      2. Verify no type errors
    Expected Result: Different variables can both be mutable ref
    Failure Indicators: False rejection of valid call
    Evidence: .sisyphus/evidence/task-10-different-vars.txt

  Scenario: Two immutable refs to same variable is accepted
    Tool: Bash
    Preconditions: Aliasing check implemented
    Steps:
      1. Run test: `foo(ref x, ref x)` — must be ACCEPTED (both are read-only)
      2. Verify no type errors
    Expected Result: Multiple immutable refs to same value are safe
    Failure Indicators: False rejection of safe immutable sharing
    Evidence: .sisyphus/evidence/task-10-two-immutable.txt
  ```

  **Commit**: YES (groups with Task 9)
  - Message: `feat(checker): enforce second-class ref rules and mutable ref aliasing`
  - Files: `src/type_system/checker/call_resolution.rs`
  - Pre-commit: `cargo test`

- [x] 11. RC insertion analysis pass

  **What to do**:
  - RED: Write unit tests that assert the analysis pass correctly identifies:
    - Last use of a variable → insert `dec` after last use
    - Variable bound to function result → `inc` already at 1 from callee (no extra inc needed)
    - Variable passed to owned parameter → `inc` before call (caller retains, callee gets shared ownership)
    - Variable passed to `ref` parameter → NO inc/dec (zero-copy)
    - Variable going out of scope without prior `dec` → insert `dec` at scope exit
    - Variable returned from function → no `dec` (ownership transfers to caller)
    - Pattern match / destructure → `inc` for each extracted binding, `dec` original
    - Conditional branches → `dec` on unused paths (if x is consumed in `then` but not `else`, insert `dec` in `else`)
  - GREEN: Create a new analysis module (e.g., `src/type_system/rc_analysis.rs` or `src/codegen/rc_analysis.rs`). This pass walks the AST (or a simplified IR) and produces a map of `inc`/`dec`/`drop` insertion points. It does NOT generate LLVM IR — it produces metadata consumed by codegen. Key algorithm: for each variable, track its `needs_rc` status (from Task 4), trace all use sites, identify the "last use" in each execution path, and annotate accordingly.
  - REFACTOR: Separate the "analysis" from "insertion" — this module only produces the plan, codegen (Task 13) executes it. Use clear data structures for the insertion plan (e.g., `enum RcOp { Inc, Dec, Drop }` with location info). Document the Perceus algorithm adaptation. Consider future module imports: RC analysis must work within a single function body (no cross-module analysis needed — callee convention handles the rest).

  **Must NOT do**:
  - Do not generate LLVM IR — that's Task 13
  - Do not implement reuse analysis — that's Task 16
  - Do not analyze across function boundaries — use calling convention (caller inc for shared, callee dec when done)

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: This is the core algorithmic task. Perceus RC analysis requires understanding variable lifetimes, control flow, and ownership semantics. Complex dataflow analysis.
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES (alongside Task 12, once dependencies met)
  - **Parallel Group**: Wave 3 (with Tasks 12, 13, 14, 15 — but 13 depends on this)
  - **Blocks**: Tasks 13, 16
  - **Blocked By**: Tasks 4, 8, 9

  **References**:

  **Pattern References**:
  - `src/type_system/types.rs:CoreType::needs_rc()` — From Task 4. Use this to determine which variables need RC tracking. Only heap-allocated types (String, Array, Generic/ADT) need RC.
  - `src/codegen/expressions.rs` — Expression codegen. Understand how expressions are traversed to determine where RC operations should be inserted.
  - `src/codegen/statements.rs` — Statement codegen. Understand scope boundaries for dec insertion.
  - `src/codegen/control_flow.rs` — Control flow codegen. Understand branch handling for conditional dec placement.

  **External References**:
  - Perceus paper (Reinking et al., 2021) — Section 3: "Precise Reference Counting" — the core algorithm for insert inc/dec at last-use points, handle branches with compensating decrements.

  **WHY Each Reference Matters**:
  - `needs_rc()` determines which variables need tracking — foundational filter
  - Codegen files show the traversal patterns the analysis must align with
  - Perceus paper defines the exact algorithm to implement

  **Acceptance Criteria**:

  **TDD:**
  - [ ] Unit tests for RC analysis on synthetic AST fragments
  - [ ] `cargo test` → PASS

  **QA Scenarios:**

  ```
  Scenario: Simple variable lifecycle produces correct RC plan
    Tool: Bash
    Preconditions: Tasks 4, 8, 9 complete
    Steps:
      1. Run RC analysis test for: `let s = get_string()` then `print(s)` — should produce: no inc (already 1), dec after print (last use)
      2. Verify the analysis plan contains exactly one Dec operation at the right location
    Expected Result: Correct inc/dec placement for simple case
    Failure Indicators: Missing dec (memory leak) or extra inc (wasted work)
    Evidence: .sisyphus/evidence/task-11-simple-lifecycle.txt

  Scenario: ref parameter produces NO RC operations
    Tool: Bash
    Preconditions: RC analysis respects PassingMode
    Steps:
      1. Run RC analysis test for: `let foo = f(ref s: string): void => print(s)` — should produce ZERO inc/dec for s
      2. Verify analysis plan is empty for ref parameters
    Expected Result: Zero RC operations for ref params (zero-copy)
    Failure Indicators: Unnecessary inc/dec for ref params
    Evidence: .sisyphus/evidence/task-11-ref-no-rc.txt

  Scenario: Conditional branches get compensating decrements
    Tool: Bash
    Preconditions: Branch analysis works
    Steps:
      1. Run RC analysis for: `let s = get_string()` then `if cond: consume_owned(s)` `else: print(ref s)` — else branch needs dec (s not consumed)
      2. Verify dec in else branch, no dec in then branch (consumed by callee)
    Expected Result: Compensating dec on non-consuming branch
    Failure Indicators: Missing dec on else (leak) or double-dec (use-after-free)
    Evidence: .sisyphus/evidence/task-11-branch-dec.txt
  ```

  **Commit**: YES
  - Message: `feat(codegen): add RC insertion analysis pass`
  - Files: `src/type_system/rc_analysis.rs` or `src/codegen/rc_analysis.rs`
  - Pre-commit: `cargo test`

- [x] 12. Lower `ref` parameters as pointers in codegen

  **What to do**:
  - RED: Write codegen tests that assert:
    - `ref` parameter compiles to an LLVM pointer type (pass-by-reference, not pass-by-value)
    - `mutable ref` parameter also compiles to LLVM pointer type (same mechanism, different type checker rules)
    - Reading a `ref` parameter generates an LLVM `load` instruction
    - Writing to a `mutable ref` parameter generates an LLVM `store` instruction
    - Owned parameters remain pass-by-value for value types (int32, float32, boolean) and pass-by-pointer for RC types
    - No RC inc/dec is generated for ref parameter access (zero-copy confirmation in codegen)
  - GREEN: Modify function codegen in `src/codegen/functions.rs` to check `Parameter::passing_mode`. For `Ref` and `MutableRef`, emit the parameter as a pointer type (`ptr` in LLVM). Generate `load` instructions when reading ref params and `store` for writing mutable ref params. Modify `src/codegen/types.rs` if needed to support pointer wrapping of types.
  - REFACTOR: Create a helper function like `param_llvm_type(param: &Parameter) -> BasicTypeEnum` that handles the Owned vs Ref distinction. Ensure this helper is reusable for lambda/closure parameters too. Document why ref params skip RC operations.

  **Must NOT do**:
  - Do not implement RC operations — that's Task 13
  - Do not add type checking — that's Task 9
  - Do not handle Weak<T> codegen — that's Task 15

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
    - Reason: LLVM IR generation for pointer parameters requires understanding inkwell API for pointer types, load/store generation, and function signature modification.
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES (alongside Tasks 11, 13 start)
  - **Parallel Group**: Wave 3
  - **Blocks**: Task 17
  - **Blocked By**: Tasks 6, 9

  **References**:

  **Pattern References**:
  - `src/codegen/functions.rs` — Function codegen. Where function parameters are emitted as LLVM types. Modify parameter emission for ref/mutable ref.
  - `src/codegen/types.rs:1-102` — `core_type_to_llvm()` function. May need a `core_type_to_llvm_ref()` variant that wraps in pointer type.
  - `src/codegen/expressions.rs` — Expression codegen. Where parameter values are read — need to add `load` for ref params.

  **API/Type References**:
  - `src/ast/types.rs:PassingMode` — Check this to determine if param is Ref/MutableRef
  - inkwell (LLVM 14 bindings) — `BasicTypeEnum::PointerType`, `builder.build_load()`, `builder.build_store()`

  **WHY Each Reference Matters**:
  - `functions.rs` is where LLVM function signatures are built — direct modification target
  - `types.rs` maps CoreType to LLVM types — may need pointer wrapping
  - `expressions.rs` reads variables — needs load instructions for ref params

  **Acceptance Criteria**:

  **TDD:**
  - [ ] Codegen tests for ref parameter lowering
  - [ ] `cargo test` → PASS

  **QA Scenarios:**

  ```
  Scenario: ref parameter compiles to pointer type
    Tool: Bash
    Preconditions: Tasks 6, 9 complete
    Steps:
      1. Compile a function with `ref` parameter, emit LLVM IR
      2. Verify the parameter is a pointer type in the LLVM function signature
      3. Verify load instruction when reading the ref param
    Expected Result: ref param is a pointer, read generates load
    Failure Indicators: ref param is pass-by-value, missing load
    Evidence: .sisyphus/evidence/task-12-ref-pointer.txt

  Scenario: No RC operations on ref param access
    Tool: Bash
    Preconditions: Ref lowering works
    Steps:
      1. Compile function with ref param, check generated LLVM IR
      2. Verify NO calls to opal_rc_inc or opal_rc_dec for the ref param
    Expected Result: Zero RC overhead for ref parameters
    Failure Indicators: Unnecessary RC calls on ref params
    Evidence: .sisyphus/evidence/task-12-ref-no-rc.txt
  ```

  **Commit**: YES
  - Message: `feat(codegen): lower ref params as pointers`
  - Files: `src/codegen/functions.rs`, `src/codegen/types.rs`, `src/codegen/expressions.rs`
  - Pre-commit: `cargo test`

 - [x] 13. Generate RC inc/dec/drop calls in codegen

  **What to do**:
  - RED: Write codegen integration tests that assert:
    - A compiled program that creates an RC-tracked value and lets it go out of scope → calls `opal_rc_dec` → refcount reaches 0 → object freed
    - A compiled program that passes RC value to a function → `opal_rc_inc` before call → `opal_rc_dec` after callee is done
    - A compiled program that returns an RC value → no `opal_rc_dec` at scope exit (ownership transferred)
    - A program creating an RC value used in both branches of an if → proper compensating dec
    - Linking against `opal_rc.c` runtime succeeds (the C functions are callable from LLVM IR)
  - GREEN: Using the RC analysis results from Task 11, emit LLVM IR calls to the C runtime functions:
    - `opal_rc_inc(obj)` → LLVM call to the C function at inc points
    - `opal_rc_dec(obj)` → LLVM call at dec points
    - At function entry for owned parameters that are RC: refcount is already 1 (callee convention)
    - At scope exit: dec all live RC variables
    - Modify `src/codegen/expressions.rs` for expression-level RC, `src/codegen/statements.rs` for scope-level RC.
    - Link `opal_rc.c` by ensuring the build process includes it (check `build.rs`).
  - REFACTOR: Create an `RcEmitter` helper struct that wraps the LLVM builder and provides `emit_inc()`, `emit_dec()`, `emit_drop()` convenience methods. This keeps RC concerns separated from general expression codegen. Ensure generated code is clean — no redundant inc/dec pairs.

  **Must NOT do**:
  - Do not implement iterative drop codegen — that's Task 14
  - Do not implement reuse analysis — that's Task 16
  - Do not add Weak ref codegen — that's Task 15

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: Core codegen integration. Requires emitting correct LLVM IR calls, understanding calling conventions, scope management, and linking with C runtime. Complex interaction between analysis results and IR generation.
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: NO (depends on Task 11 for analysis results)
  - **Parallel Group**: Wave 3 (sequential: after Task 11)
  - **Blocks**: Tasks 14, 15, 16, 18
  - **Blocked By**: Tasks 3, 11

  **References**:

  **Pattern References**:
  - `src/codegen/expressions.rs` — Expression codegen. Insert RC calls around expression evaluation (inc before sharing, dec after last use).
  - `src/codegen/statements.rs` — Statement codegen (let bindings, assignments, scope exit). Insert dec at scope boundaries.
  - `src/codegen/functions.rs` — Function codegen. Handle RC for function parameters (owned params have refcount 1 from caller) and return values.
  - `src/codegen/context.rs` — Codegen context. May need to store RC analysis results for lookup during codegen.

  **API/Type References**:
  - `runtime/opal_rc.h` — C function signatures to call: `opal_rc_inc`, `opal_rc_dec`, etc.
  - RC analysis output from Task 11 — `RcOp { Inc, Dec, Drop }` with location info.
  - inkwell — `module.add_function()` for declaring external C functions, `builder.build_call()` for calling them.

  **External References**:
  - `Cargo.toml` / `build.rs` — Check how the C runtime is currently linked. May need to add `opal_rc.c` to the build.

  **WHY Each Reference Matters**:
  - Codegen files are the modification targets — where LLVM calls get emitted
  - C runtime headers define the function signatures to call
  - RC analysis provides the insertion plan to follow
  - Build config must include the new C file

  **Acceptance Criteria**:

  **TDD:**
  - [ ] Codegen tests for RC call emission
  - [ ] Integration test: compile + run a program, verify no memory leaks (clean exit)
  - [ ] `cargo test` → PASS

  **QA Scenarios:**

  ```
  Scenario: RC value created and freed correctly
    Tool: Bash
    Preconditions: Tasks 3, 11 complete, C runtime linked
    Steps:
      1. Write test.op: `let s = "hello"; print(s);` — string is RC-tracked
      2. Compile and run
      3. Verify clean exit (no segfault, no leak reported)
    Expected Result: String allocated, printed, freed at scope exit
    Failure Indicators: Segfault, memory leak, double-free
    Evidence: .sisyphus/evidence/task-13-rc-basic.txt

  Scenario: RC inc/dec around function call
    Tool: Bash
    Preconditions: RC codegen works
    Steps:
      1. Write test.op: function taking owned String param, caller passes string used after call
      2. Compile and run
      3. Verify inc before call (shared ownership), dec in callee, dec in caller after last use
    Expected Result: Correct refcount management across call boundary
    Failure Indicators: Use-after-free (dec too early) or leak (missing dec)
    Evidence: .sisyphus/evidence/task-13-rc-call.txt

  Scenario: C runtime links successfully
    Tool: Bash
    Preconditions: opal_rc.c exists from Task 3
    Steps:
      1. Run `cargo build 2>&1` — no linker errors
      2. Verify opal_rc functions are resolved
    Expected Result: Clean build with C runtime linked
    Failure Indicators: Undefined symbol errors for opal_rc_*
    Evidence: .sisyphus/evidence/task-13-link.txt
  ```

  **Commit**: YES
  - Message: `feat(codegen): generate RC inc/dec/drop calls`
  - Files: `src/codegen/expressions.rs`, `src/codegen/statements.rs`, `src/codegen/functions.rs`, `src/codegen/context.rs`
  - Pre-commit: `cargo test`

 - [x] 14. Implement iterative drop in C runtime + codegen integration

  **What to do**:
  - RED: Write tests that assert:
    - A deeply nested structure (e.g., linked list with 10,000+ nodes) is freed without stack overflow
    - The drop function uses a work-list (stack-allocated array that grows) to process children iteratively
    - Each RC object's `drop_children_fn` enqueues child objects onto the work-list rather than recursively calling dec
    - After iterative drop, all objects in the chain are freed (no leaks)
    - Compile a test program with deeply nested RC structures, run it, verify clean exit
  - GREEN:
    - In `runtime/opal_rc.c`: Ensure `opal_rc_drop_iterative()` uses a work-list pattern:
      ```c
      // Pseudocode:
      work_list = [obj];
      while (work_list not empty) {
        current = work_list.pop();
        current->drop_children_fn(current, &work_list); // enqueues children
        free(current);
      }
      ```
    - In codegen: For each RC-tracked type (structs, ADTs), generate a `drop_children_fn` that decrements each child field's refcount and, if that hits 0, pushes the child onto the work-list instead of calling `opal_rc_dec` recursively.
    - The `drop_children_fn` is set during `opal_rc_alloc` — codegen must pass the correct function pointer for each type.
  - REFACTOR: Ensure the work-list has a reasonable initial capacity (e.g., 64 entries) and grows by doubling. Add safety: if work-list allocation fails, fall back to recursive drop with a warning (graceful degradation). Document the iterative drop algorithm clearly in comments.

  **Must NOT do**:
  - **No recursive drops** — this is the whole point. The drop_children_fn must NOT call opal_rc_dec (which would recurse). It pushes onto the work-list.
  - No cycle collector — iterative drop handles deep chains, not cycles (weak refs handle cycles)

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: Requires careful design of the drop_children_fn calling convention, work-list management, and codegen of type-specific drop functions. Subtle correctness requirements.
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: NO (depends on Task 13 for RC codegen infrastructure)
  - **Parallel Group**: Wave 3 (sequential: after Task 13)
  - **Blocks**: Task 18
  - **Blocked By**: Tasks 3, 13

  **References**:

  **Pattern References**:
  - `runtime/opal_rc.c` — From Task 3. The `opal_rc_drop_iterative` function. Ensure it uses work-list, not recursion.
  - `src/codegen/adts.rs` — ADT codegen. Where struct/enum type information is generated. Need to generate `drop_children_fn` for each ADT type.
  - `src/codegen/expressions.rs` — Where `opal_rc_alloc` calls are generated. Must pass the correct `drop_children_fn` pointer.

  **External References**:
  - Lean 4 runtime — Uses iterative drop with a "todo list" for pending decrements
  - Koka runtime — Similar iterative drop strategy from the Perceus paper

  **WHY Each Reference Matters**:
  - `opal_rc.c` is the iterative drop implementation target
  - `adts.rs` is where type-specific drop functions must be generated
  - Expression codegen must pass drop function pointers during allocation

  **Acceptance Criteria**:

  **TDD:**
  - [ ] Test for deep nesting (10,000+ nodes) without stack overflow
  - [ ] Test for correct cleanup (no leaks after iterative drop)
  - [ ] `cargo test` → PASS

  **QA Scenarios:**

  ```
  Scenario: Deep linked list freed without stack overflow
    Tool: Bash
    Preconditions: Tasks 3, 13 complete
    Steps:
      1. Write test.op that creates a deeply nested structure (e.g., recursive struct with 10,000 nesting depth)
      2. Compile and run with limited stack size (ulimit -s 1024 or similar)
      3. Verify clean exit — no segfault from stack overflow
    Expected Result: Iterative drop handles 10,000+ depth without stack overflow
    Failure Indicators: Segfault (stack overflow from recursive drop)
    Evidence: .sisyphus/evidence/task-14-deep-drop.txt

  Scenario: No recursive calls in drop implementation
    Tool: Bash
    Preconditions: opal_rc.c updated
    Steps:
      1. Read `runtime/opal_rc.c` — find `opal_rc_drop_iterative` function
      2. Verify it uses a while loop with work-list, not recursive calls
      3. Grep for self-referential calls within the drop function
    Expected Result: Pure iterative implementation with work-list
    Failure Indicators: Recursive calls to opal_rc_dec within drop
    Evidence: .sisyphus/evidence/task-14-no-recursion.txt
  ```

  **Commit**: YES
  - Message: `feat(codegen): implement iterative drop codegen`
  - Files: `runtime/opal_rc.c`, `src/codegen/adts.rs`, `src/codegen/expressions.rs`
  - Pre-commit: `cargo test`

 - [x] 15. Lower `Weak<T>` to LLVM + weak ref codegen

  **What to do**:
  - RED: Write codegen tests that assert:
    - `Weak<T>` values compile to a pointer to a weak reference wrapper (separate from the strong RC object)
    - `opal_weak_alloc(strong_obj)` is called when creating a `Weak<T>` from a strong reference
    - `.upgrade()` call compiles to `opal_weak_upgrade(weak)` → returns `Option<T>` (Some with strong ref if alive, None if dead)
    - Dropping a `Weak<T>` calls `opal_weak_dec(weak)` → decrements weak_count
    - When both refcount AND weak_count reach 0, the header memory is freed
    - A program creating a weak ref, dropping the strong ref, then upgrading → gets `None`
  - GREEN: In codegen:
    - `Weak<T>` type → LLVM pointer type (same as RC pointer, but with weak wrapper)
    - `Weak::new(strong_ref)` or weak creation syntax → emit `opal_weak_alloc` call
    - `.upgrade()` method call → emit `opal_weak_upgrade` call, wrap result in `Option<T>` (Some/None)
    - Variable going out of scope with type `Weak<T>` → emit `opal_weak_dec` instead of `opal_rc_dec`
    - RC analysis (Task 11) must recognize `Weak<T>` and use weak-specific operations
  - REFACTOR: Ensure `Weak<T>` and strong `T` share the LLVM type infrastructure where possible. Document the memory layout: strong RC header has weak_count field, weak ref points to the same header. Clean up any duplication between strong and weak codegen paths.

  **Must NOT do**:
  - No cycle collector
  - No `?` syntax — use `.upgrade()` method returning `Option<T>`
  - Do not duplicate `Option<T>` codegen — reuse existing Option infrastructure

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: Weak ref codegen requires coordination between RC analysis, type lowering, method codegen, and C runtime. Multiple codegen files affected.
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: NO (depends on Tasks 8, 13)
  - **Parallel Group**: Wave 3 (after Tasks 8 and 13 complete)
  - **Blocks**: Task 19
  - **Blocked By**: Tasks 3, 8, 13

  **References**:

  **Pattern References**:
  - `src/codegen/types.rs` — Type lowering. Add Weak<T> → LLVM pointer mapping.
  - `src/codegen/expressions.rs` — Expression codegen. Handle Weak creation and .upgrade() method calls.
  - `src/codegen/adts.rs` — ADT codegen. Option<T> codegen (Some/None) is here — .upgrade() returns Option<T>.
  - `runtime/opal_rc.h` — Weak ref C function declarations: `opal_weak_alloc`, `opal_weak_upgrade`, `opal_weak_dec`.

  **API/Type References**:
  - `src/type_system/types.rs:CoreType::Generic` — Weak<T> is `Generic { name: "Weak", type_args: [T] }`. Check for this name in codegen.
  - `src/type_system/checker/generics.rs` — From Task 8, Weak<T> registration and .upgrade() method resolution.

  **WHY Each Reference Matters**:
  - Type lowering maps Weak<T> to LLVM — needs pointer type
  - Expression codegen handles .upgrade() — needs to call opal_weak_upgrade and wrap in Option
  - ADT codegen has Option<T> — .upgrade() result must be compatible
  - C runtime defines the functions to call

  **Acceptance Criteria**:

  **TDD:**
  - [ ] Codegen tests for Weak creation, upgrade, and drop
  - [ ] Integration: program with weak ref, drop strong, upgrade → None
  - [ ] `cargo test` → PASS

  **QA Scenarios:**

  ```
  Scenario: Weak ref creation and upgrade when alive
    Tool: Bash
    Preconditions: Tasks 3, 8, 13 complete
    Steps:
      1. Write test.op: create strong ref, create weak from it, upgrade weak → should get Some
      2. Compile and run
      3. Verify output shows successful upgrade (Some value)
    Expected Result: Weak upgrade returns Some when strong ref still alive
    Failure Indicators: Returns None while strong exists, or crashes
    Evidence: .sisyphus/evidence/task-15-weak-alive.txt

  Scenario: Weak ref upgrade after strong dropped → None
    Tool: Bash
    Preconditions: Weak ref codegen works
    Steps:
      1. Write test.op: create strong ref, create weak, drop strong, upgrade weak → should get None
      2. Compile and run
      3. Verify output shows None (dead object)
    Expected Result: Weak upgrade returns None after strong ref dropped
    Failure Indicators: Returns dangling pointer, crashes, or returns Some
    Evidence: .sisyphus/evidence/task-15-weak-dead.txt

  Scenario: Weak ref drop decrements weak_count
    Tool: Bash
    Preconditions: Weak drop codegen works
    Steps:
      1. Write test.op: create strong, create weak, drop weak (strong still alive)
      2. Compile and run — no crash
      3. Drop strong — object fully freed
    Expected Result: Clean lifecycle: weak drop doesn't free object, strong drop does
    Failure Indicators: Premature free, use-after-free, leak
    Evidence: .sisyphus/evidence/task-15-weak-drop.txt
  ```

  **Commit**: YES
  - Message: `feat(codegen): lower Weak<T> to LLVM with weak ref operations`
  - Files: `src/codegen/types.rs`, `src/codegen/expressions.rs`, `src/codegen/adts.rs`
  - Pre-commit: `cargo test`

 - [x] 16. Perceus reuse analysis for unique owners

  **What to do**:
  - RED: Write unit tests that assert:
    - When a variable has refcount == 1 (unique owner) and is about to be dropped, the memory can be reused for a new allocation of the same size
    - Reuse is detected for: `let x = Foo(1, 2); let y = Foo(3, 4);` where x is last-used before y is allocated → x's memory reused for y
    - Reuse is NOT applied when refcount > 1 (shared ownership)
    - Reuse is NOT applied across different allocation sizes
    - The analysis produces a `Reuse(source_var, target_var)` metadata annotation
  - GREEN: Extend the RC analysis module (from Task 11) with a reuse analysis pass. The algorithm:
    1. For each `Dec` that would trigger a drop (refcount → 0), check if the next allocation is the same size
    2. If yes, emit a `Reuse` operation instead of `Drop + Alloc`
    3. At codegen level: instead of freeing the old object and allocating new, zero-initialize the payload and reuse the header
    4. Add a runtime function `opal_rc_reuse(obj, new_drop_fn)` that resets refcount to 1 and updates the drop function
  - REFACTOR: Keep reuse analysis as a separate, optional pass that can be disabled. Document which cases it optimizes and which it skips. Add metrics: count of reuse opportunities found.

  **Must NOT do**:
  - Do not implement general-purpose memory pooling — only Perceus-style reuse for unique owners
  - Do not modify the basic RC operations from Task 13 — reuse is an optimization layer on top

  **Recommended Agent Profile**:
  - **Category**: `ultrabrain`
    - Reason: This is the most algorithmically complex task. Reuse analysis requires reasoning about allocation sizes, uniqueness guarantees, and correct reuse semantics. Subtle correctness requirements.
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES (alongside Tasks 17, if Tasks 11 and 13 are done)
  - **Parallel Group**: Wave 4
  - **Blocks**: Task 18
  - **Blocked By**: Tasks 11, 13

  **References**:

  **Pattern References**:
  - `src/codegen/rc_analysis.rs` or `src/type_system/rc_analysis.rs` — From Task 11. Extend this module with reuse analysis.
  - `runtime/opal_rc.c` — Add `opal_rc_reuse()` function. Check existing alloc/dec functions for size tracking.
  - `src/type_system/memory.rs` — `MemoryLayout` provides size/alignment info needed for size-matching.

  **External References**:
  - Perceus paper (Reinking et al., 2021) — Section 4: "Reuse Analysis" — describes the exact algorithm for detecting reuse opportunities and the conditions for safe reuse.

  **WHY Each Reference Matters**:
  - RC analysis module is the extension target
  - C runtime needs a new reuse function
  - Memory layout is needed for size comparison
  - Perceus paper defines the algorithm

  **Acceptance Criteria**:

  **TDD:**
  - [ ] Unit tests for reuse detection and non-reuse cases
  - [ ] `cargo test` → PASS

  **QA Scenarios:**

  ```
  Scenario: Reuse detected for same-size unique allocation
    Tool: Bash
    Preconditions: Tasks 11, 13 complete
    Steps:
      1. Run reuse analysis on: `let x = Foo(1); let y = Foo(2);` where x is unique and last-used before y
      2. Verify analysis output contains a Reuse operation
    Expected Result: Reuse opportunity detected
    Failure Indicators: Missed reuse opportunity, or reuse applied to shared value
    Evidence: .sisyphus/evidence/task-16-reuse-detected.txt

  Scenario: No reuse for shared (non-unique) values
    Tool: Bash
    Preconditions: Reuse analysis works
    Steps:
      1. Run analysis on code where value is shared (refcount > 1 at drop point)
      2. Verify no Reuse operation — falls back to normal drop + alloc
    Expected Result: Reuse correctly skipped for shared values
    Failure Indicators: Reuse applied to shared value (would corrupt the other reference)
    Evidence: .sisyphus/evidence/task-16-no-reuse-shared.txt
  ```

  **Commit**: YES
  - Message: `feat(codegen): add Perceus reuse analysis for unique owners`
  - Files: `src/codegen/rc_analysis.rs` or `src/type_system/rc_analysis.rs`, `runtime/opal_rc.c`
  - Pre-commit: `cargo test`

- [x] 17. Test projects: ref-basic, mutable-ref, ref-compile-fail

  **What to do**:
  - RED: Integration tests in `tests/integration_e2e.rs` that compile and run (or compile-fail) the test projects. Tests will fail until the test project code is written and the compiler features work.
  - GREEN: Create test project directories following canonical structure (`opal.toml` + `.gitignore` + `README.md` + `src/main.op`). Fill in the `src/main.op` files. Integration tests use `stdout.contains("expected string")` assertions (NOT separate expected_output files):

  **`test-projects/ref-basic/src/main.op`**: Demonstrates basic `ref` parameter usage:
    - Function taking `ref` parameter, reading the value, printing it
    - Calling with a ref argument
    - Multiple `ref` params to same function
    - Passing `ref` to another `ref` parameter (ref-to-ref)
    - Expected output: printed values confirming zero-copy reads work

  **`test-projects/mutable-ref/src/main.op`**: Demonstrates `mutable ref` parameter usage:
    - Function taking `mutable ref` parameter, modifying the value
    - Caller observes the mutation after the call returns
    - Expected output: original value, then modified value

  **`test-projects/ref-compile-fail/src/main.op`**: Demonstrates compile-time errors for invalid ref usage:
    - Attempt to store ref in variable → error
    - Attempt to return ref → error
    - Attempt to capture ref in lambda → error
    - Attempt to alias mutable refs → error
    - Expected: compilation fails with meaningful error messages (integration test asserts compilation failure and checks stderr for expected error strings)

  - REFACTOR: Ensure test programs are minimal and focused. Each demonstrates ONE concept clearly. Add comments in `.op` files explaining what's being tested.

  **Must NOT do**:
  - Do not modify existing test projects
  - Do not test RC features here — that's Task 18
  - Do not test weak refs here — that's Task 19

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
    - Reason: Writing test programs in the Opalescent language, matching expected output, and wiring integration tests. Requires understanding the language syntax and compiler behavior.
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES (alongside Tasks 18, 19)
  - **Parallel Group**: Wave 4
  - **Blocks**: Tasks 20, 21
  - **Blocked By**: Tasks 5, 10, 12

  **References**:

  **Pattern References**:
  - `test-projects/hello-world/` — Simplest test project example. Follow directory structure: `opal.toml`, `.gitignore`, `README.md`, `src/main.op`.
  - `test-projects/` (any existing project) — See naming conventions, file format.
  - `tests/integration_e2e.rs:1-996` — Integration test harness. Look at how existing tests call `prepare_dir`, compile via `compile_program(source, temp_dir)`, run the binary, and assert `stdout.contains("expected string")`. No separate expected_output files.

  **API/Type References**:
  - `language-spec/requirements/overview.md` — Language syntax reference for writing correct `.op` programs.

  **WHY Each Reference Matters**:
  - Existing test projects define the convention to follow exactly
  - Integration test harness shows how to wire new tests
  - Language spec confirms correct syntax

  **Acceptance Criteria**:

  **TDD:**
  - [ ] Integration tests for all 3 test projects in `tests/integration_e2e.rs`
  - [ ] `cargo test --features integration` → PASS for ref-basic and mutable-ref
  - [ ] `cargo test --features integration` → PASS for ref-compile-fail (expected errors match)

  **QA Scenarios:**

  ```
  Scenario: ref-basic test project compiles and runs
    Tool: Bash
    Preconditions: Tasks 5, 12 complete, ref param codegen works
    Steps:
      1. Run `cargo test --features integration ref_basic 2>&1`
      2. Verify test passes — stdout contains expected ref output values
    Expected Result: ref-basic compiles, runs, stdout contains expected values
    Failure Indicators: Compilation failure, runtime crash, output mismatch
    Evidence: .sisyphus/evidence/task-17-ref-basic.txt

  Scenario: ref-compile-fail correctly rejects invalid code
    Tool: Bash
    Preconditions: Task 9, 10 complete
    Steps:
      1. Run `cargo test --features integration ref_compile_fail 2>&1`
      2. Verify compilation fails with expected error messages
    Expected Result: Compiler produces correct error messages for each violation
    Failure Indicators: Code compiles when it shouldn't, or wrong error message
    Evidence: .sisyphus/evidence/task-17-ref-fail.txt

  Scenario: All 13 existing test projects still pass
    Tool: Bash
    Preconditions: None
    Steps:
      1. Run `cargo test --features integration 2>&1 | tail -20`
      2. Verify ALL existing test projects pass (zero regressions)
    Expected Result: All 13 original test projects unaffected
    Failure Indicators: Any existing test project failure
    Evidence: .sisyphus/evidence/task-17-no-regression.txt
  ```

  **Commit**: YES (groups with Tasks 18, 19)
  - Message: `test(integration): add memory model test projects (ref, rc, weak)`
  - Files: `test-projects/ref-basic/`, `test-projects/mutable-ref/`, `test-projects/ref-compile-fail/`, `tests/integration_e2e.rs`
  - Pre-commit: `cargo test --features integration`

- [x] 18. Test projects: rc-basic, rc-reuse, iterative-drop

  **What to do**:
  - RED: Integration tests for RC test projects that initially fail.
  - GREEN: Create test project directories following canonical structure (`opal.toml` + `.gitignore` + `README.md` + `src/main.op`). Fill in `src/main.op` files. Integration tests use `stdout.contains("expected string")` assertions:

  **`test-projects/rc-basic/src/main.op`**: Demonstrates basic RC lifecycle:
    - Create a heap-allocated value (String, Array, or struct)
    - Pass to function (RC inc), function uses it (RC preserved), function returns (RC dec)
    - Value goes out of scope (final dec, object freed)
    - Expected output: values printed correctly, clean exit (no crashes)

  **`test-projects/rc-reuse/src/main.op`**: Demonstrates Perceus reuse:
    - Create struct, use it (last use), create another struct of same type
    - The second allocation should reuse the first's memory (optimization — may not be visible in output, but tests clean execution)
    - Expected output: correct values printed for both structs

  **`test-projects/iterative-drop/src/main.op`**: Demonstrates iterative drop for deep structures:
    - Create a deeply nested structure (e.g., linked list or recursive struct with significant depth)
    - Let it go out of scope — iterative drop handles the chain
    - Expected output: correct construction, clean exit without stack overflow

  - REFACTOR: Add comments in test programs explaining the memory model behavior being exercised.

  **Must NOT do**:
  - Do not test ref features — that's Task 17
  - Do not test weak refs — that's Task 19
  - Do not modify existing test projects

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
    - Reason: Writing test programs that exercise RC semantics. Requires understanding the memory model behavior.
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES (alongside Tasks 17, 19)
  - **Parallel Group**: Wave 4
  - **Blocks**: Tasks 20, 21
  - **Blocked By**: Tasks 5, 13, 14, 16

  **References**:

  **Pattern References**:
  - Same as Task 17 — existing test projects and integration test harness.

  **WHY Each Reference Matters**:
  - Same as Task 17

  **Acceptance Criteria**:

  **TDD:**
  - [ ] Integration tests for all 3 RC test projects
  - [ ] `cargo test --features integration` → PASS

  **QA Scenarios:**

  ```
  Scenario: rc-basic demonstrates correct RC lifecycle
    Tool: Bash
    Preconditions: Tasks 13 complete
    Steps:
      1. Run `cargo test --features integration rc_basic 2>&1`
      2. Verify test passes — stdout contains expected RC lifecycle output values
    Expected Result: RC lifecycle works correctly end-to-end
    Failure Indicators: Segfault, memory corruption, output mismatch
    Evidence: .sisyphus/evidence/task-18-rc-basic.txt

  Scenario: iterative-drop handles deep nesting
    Tool: Bash
    Preconditions: Task 14 complete
    Steps:
      1. Run `cargo test --features integration iterative_drop 2>&1`
      2. Verify test passes — clean exit without stack overflow
    Expected Result: Deep structure freed iteratively, no crash
    Failure Indicators: Stack overflow, segfault, hang
    Evidence: .sisyphus/evidence/task-18-iterative-drop.txt
  ```

  **Commit**: YES (groups with Tasks 17, 19)
  - Message: `test(integration): add memory model test projects (ref, rc, weak)`
  - Files: `test-projects/rc-basic/`, `test-projects/rc-reuse/`, `test-projects/iterative-drop/`, `tests/integration_e2e.rs`
  - Pre-commit: `cargo test --features integration`

- [x] 19. Test projects: weak-ref

  **What to do**:
  - RED: Integration test for weak-ref test project.
  - GREEN: Create test project directory following canonical structure (`opal.toml` + `.gitignore` + `README.md` + `src/main.op`). Fill in `test-projects/weak-ref/src/main.op`:

  **`test-projects/weak-ref/main.op`**: Demonstrates weak reference usage:
    - Create a strong reference to a heap-allocated object
    - Create a `Weak<T>` from the strong reference
    - Upgrade the weak ref while strong is alive → get `Some(value)`
    - Print the value from the upgraded weak ref
    - Drop the strong reference
    - Upgrade the weak ref again → get `None`
    - Print indication that upgrade returned None
    - Demonstrate use case: mutable object cycle (two objects pointing to each other via one strong + one weak)

  Expected output: demonstrates alive upgrade, dead upgrade, and cycle pattern.

  - REFACTOR: Add extensive comments explaining the weak ref lifecycle and the cycle-breaking use case.

  **Must NOT do**:
  - Do not test ref features — that's Task 17
  - Do not test RC features — that's Task 18
  - Do not use `?` syntax — use `.upgrade()` with `Option<T>` matching

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
    - Reason: Weak ref test program with Option matching and cycle demonstration.
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES (alongside Tasks 17, 18)
  - **Parallel Group**: Wave 4
  - **Blocks**: Tasks 20, 21
  - **Blocked By**: Tasks 5, 15

  **References**:

  **Pattern References**:
  - Same as Task 17 — test project conventions.
  - `memory-model-proposals/combined/perceus-with-second-class-refs/proposal.md:100-137` — The weak ref section of the proposal. Shows intended usage patterns (but adapt syntax to use Option<T>, not ?).

  **Acceptance Criteria**:

  **TDD:**
  - [ ] Integration test for weak-ref project
  - [ ] `cargo test --features integration` → PASS

  **QA Scenarios:**

  ```
  Scenario: weak-ref demonstrates full lifecycle
    Tool: Bash
    Preconditions: Task 15 complete
    Steps:
      1. Run `cargo test --features integration weak_ref 2>&1`
      2. Verify output shows: alive upgrade (Some), dead upgrade (None), cycle pattern
    Expected Result: Weak ref lifecycle correct — upgrade alive → Some, upgrade dead → None
    Failure Indicators: Crash on upgrade, wrong Option variant, cycle causes leak
    Evidence: .sisyphus/evidence/task-19-weak-ref.txt

  Scenario: Cycle with weak ref doesn't leak
    Tool: Bash
    Preconditions: Weak ref and RC codegen work
    Steps:
      1. Compile and run weak-ref project with cycle demonstration
      2. Verify clean exit — objects freed, no leak
    Expected Result: Cycle broken by weak ref, all objects freed
    Failure Indicators: Memory leak (objects never freed due to cycle)
    Evidence: .sisyphus/evidence/task-19-weak-cycle.txt
  ```

  **Commit**: YES (groups with Tasks 17, 18)
  - Message: `test(integration): add memory model test projects (ref, rc, weak)`
  - Files: `test-projects/weak-ref/`, `tests/integration_e2e.rs`
  - Pre-commit: `cargo test --features integration`

- [x] 20. README.md: memory model documentation + weak ref guide + optimization strategies

  **What to do**:
  - RED: N/A (documentation task — no code tests, but verify content exists)
  - GREEN: Add comprehensive memory model documentation to `README.md`. Sections to include:

  **Memory Model Overview:**
  - Opalescent uses Perceus-style reference counting — no garbage collector, no manual memory management
  - All heap-allocated values (String, Array, structs, ADTs) are automatically reference counted
  - Value types (int32, float32, boolean) are stack-allocated and copied — no RC overhead

  **Second-Class References:**
  - `ref` parameters: zero-copy reads, no RC inc/dec overhead
  - `mutable ref` parameters: exclusive write access, compiler-enforced aliasing rules
  - Rules: refs cannot be stored, returned, or captured in closures
  - When to use: read-only access to large structures, performance-sensitive code

  **Weak References:**
  - `Weak<T>` type: enables mutable object cycles without a cycle collector
  - `.upgrade()` returns `Option<T>` — `Some(value)` if alive, `None` if dead
  - Use cases: parent-child relationships, caches, observer patterns
  - CRITICAL: explain that `Weak<T>` is the ONLY mechanism for cycles — no cycle collector exists
  - Usage examples with `match` on `Option<T>` after upgrade
  - When to use weak refs vs. restructuring (prefer restructuring when possible)

  **Optimization Strategies:**
  - Perceus reuse analysis: how unique ownership enables memory reuse
  - `ref` parameters: avoid unnecessary RC operations for read-only access
  - Iterative drops: how deep structures are freed without stack overflow
  - Tips for writing memory-efficient Opalescent code

  - REFACTOR: Ensure documentation reads naturally for a developer new to Opalescent. Use code examples from the test projects. Cross-reference the language spec.

  **Must NOT do**:
  - Do not over-document internals — focus on user-facing behavior
  - Do not include implementation details of the C runtime
  - Do not suggest `?` syntax — always use `.upgrade()` with `Option<T>`

  **Recommended Agent Profile**:
  - **Category**: `writing`
    - Reason: Technical documentation writing with code examples.
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES (alongside Task 21)
  - **Parallel Group**: Wave 5
  - **Blocks**: None
  - **Blocked By**: Tasks 17, 18, 19

  **References**:

  **Pattern References**:
  - `README.md` — Existing README structure. Add new sections in appropriate location.
  - `test-projects/weak-ref/main.op` — From Task 19. Use as code example source.
  - `test-projects/ref-basic/main.op` — From Task 17. Use as code example source.

  **WHY Each Reference Matters**:
  - README defines the current structure to extend
  - Test projects provide working code examples to include in docs

  **Acceptance Criteria**:

  **QA Scenarios:**

  ```
  Scenario: README has memory model section
    Tool: Bash
    Preconditions: Tasks 17-19 complete
    Steps:
      1. Run `grep -c "Memory Model\|Reference Counting\|Weak Reference\|Second-Class" README.md`
      2. Verify section headers exist for all major topics
    Expected Result: At least 4 section headers covering all memory model topics
    Failure Indicators: Missing sections, incomplete coverage
    Evidence: .sisyphus/evidence/task-20-readme-sections.txt

  Scenario: Weak ref documentation is extensive
    Tool: Bash
    Preconditions: README updated
    Steps:
      1. Run `grep -c "Weak\|weak\|upgrade\|Option" README.md`
      2. Verify at least 20 mentions (indicating thorough coverage per user request)
      3. Verify code examples are present (look for code blocks)
    Expected Result: Extensive weak ref documentation with examples
    Failure Indicators: Brief mention without examples or use cases
    Evidence: .sisyphus/evidence/task-20-weak-docs.txt

  Scenario: No ? syntax mentioned
    Tool: Bash
    Preconditions: README updated
    Steps:
      1. Run `grep -n "?" README.md | grep -i "weak\|upgrade\|optional"` — should find nothing
      2. Verify all weak ref examples use .upgrade() with Option<T> matching
    Expected Result: Zero ? syntax, all examples use .upgrade() + Option<T>
    Failure Indicators: ? syntax used instead of Option<T>
    Evidence: .sisyphus/evidence/task-20-no-question-mark.txt
  ```

  **Commit**: YES (groups with Task 21)
  - Message: `docs: add memory model specification and README documentation`
  - Files: `README.md`
  - Pre-commit: — (no code to test)

- [x] 21. Language spec: `memory-model.md` formal specification

  **What to do**:
  - RED: N/A (documentation task)
  - GREEN: Create `language-spec/requirements/memory-model.md` with formal specification:

  **Sections:**
  1. **Overview**: Opalescent memory model — Perceus RC + second-class references
  2. **Value Types vs Heap Types**: Which types are stack-allocated (no RC) vs heap-allocated (RC managed)
  3. **Reference Counting Semantics**:
     - Object lifecycle: alloc (refcount=1), inc (shared), dec (done), drop (refcount→0)
     - Calling convention: caller inc before passing owned, callee dec when done
     - Return convention: no dec on return (ownership transfers)
  4. **Second-Class References**:
     - `ref` parameter annotation: semantics, restrictions, compilation
     - `mutable ref` parameter annotation: semantics, aliasing rules, restrictions
     - Formal rules: cannot store, return, capture
  5. **Weak References**:
     - `Weak<T>` type: creation, upgrade, drop
     - `.upgrade() -> Option<T>` semantics
     - Weak count lifecycle: when header is freed (both counts = 0)
     - Cycle-breaking pattern
  6. **Iterative Drop**:
     - Work-list algorithm description
     - `drop_children_fn` convention
     - Why: prevents stack overflow on deep structures
  7. **Reuse Analysis**:
     - Conditions for reuse: unique ownership, same allocation size
     - Performance implications
  8. **Module Import Compatibility**:
     - ABI stability requirements for RC object headers
     - Cross-module RC conventions
  9. **Optimization Guide**:
     - How to write code that maximizes reuse opportunities
     - When to use `ref` vs owned parameters
     - Weak ref cost and when to use

  - REFACTOR: Ensure spec is consistent with the README documentation (Task 20). Cross-reference between the two. Use formal language for specification sections, informal language for guides.

  **Must NOT do**:
  - Do not duplicate README content verbatim — spec is more formal, README is more tutorial
  - Do not include C runtime implementation details — focus on language semantics

  **Recommended Agent Profile**:
  - **Category**: `writing`
    - Reason: Formal specification writing requiring precise language.
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES (alongside Task 20)
  - **Parallel Group**: Wave 5
  - **Blocks**: None
  - **Blocked By**: Tasks 17, 18, 19

  **References**:

  **Pattern References**:
  - `language-spec/requirements/overview.md` — Existing spec style. Follow the same formatting, heading levels, and tone.
  - `language-spec/requirements/modules.md` — Another spec file for structural reference.
  - `memory-model-proposals/combined/perceus-with-second-class-refs/proposal.md` — Original proposal. Spec should formalize the concepts from this proposal.

  **WHY Each Reference Matters**:
  - Existing spec files define formatting and style conventions
  - The proposal contains the source material to formalize

  **Acceptance Criteria**:

  **QA Scenarios:**

  ```
  Scenario: memory-model.md exists with all required sections
    Tool: Bash
    Preconditions: Tasks 17-19 complete
    Steps:
      1. Run `test -f language-spec/requirements/memory-model.md && echo "EXISTS" || echo "MISSING"`
      2. Run `grep -c "^#" language-spec/requirements/memory-model.md` to count section headers
      3. Verify sections cover: RC, ref, mutable ref, Weak, iterative drop, reuse, modules, optimization
    Expected Result: File exists with 8+ sections covering all topics
    Failure Indicators: Missing file, missing sections
    Evidence: .sisyphus/evidence/task-21-spec-exists.txt

  Scenario: Spec is consistent with README
    Tool: Bash
    Preconditions: Both Task 20 and 21 complete
    Steps:
      1. Compare key terms used in README.md vs memory-model.md
      2. Verify no contradictions in described behavior
    Expected Result: Consistent terminology and descriptions
    Failure Indicators: Contradicting descriptions of the same behavior
    Evidence: .sisyphus/evidence/task-21-consistency.txt
  ```

  **Commit**: YES (groups with Task 20)
  - Message: `docs: add memory model specification and README documentation`
  - Files: `language-spec/requirements/memory-model.md`
  - Pre-commit: — (no code to test)

---

## Final Verification Wave

> 4 review agents run in PARALLEL. ALL must APPROVE. Present consolidated results to user and get explicit "okay" before completing.
>
> **Do NOT auto-proceed after verification. Wait for user's explicit approval before marking work complete.**

- [x] F1. **Plan Compliance Audit** — `oracle`
  Read the plan end-to-end. For each "Must Have": verify implementation exists (read file, run command). For each "Must NOT Have": search codebase for forbidden patterns — reject with file:line if found. Check evidence files exist in .sisyphus/evidence/. Compare deliverables against plan.
  Output: `Must Have [N/N] | Must NOT Have [N/N] | Tasks [N/N] | VERDICT: APPROVE/REJECT`
  RESULT: APPROVE (after fix: opal_rc_drop → opal_rc_drop_iterative in rc_emitter.rs; std/HashMap in pre-existing files are not violations)

- [x] F2. **Code Quality Review** — `unspecified-high`
  Run `cargo build`, `cargo clippy -- -D warnings`, `cargo test`, `cargo test --features integration`. Review all changed files for: `as` casts without safety comments, `unsafe` blocks without justification, `unwrap()` in non-test code, `todo!()` macros left behind, unused imports. Check AI slop: excessive comments, over-abstraction, generic names (data/result/item/temp). Verify `no_std` compliance (no `std::` imports in compiler source, only `alloc`/`core`).
  Output: `Build [PASS/FAIL] | Clippy [PASS/FAIL] | Tests [N pass/N fail] | Integration [N pass/N fail] | Files [N clean/N issues] | VERDICT`
  RESULT: APPROVE — 1094 tests pass, clippy clean, no TODOs/stubs in new files, no HashMap in new code

- [x] F3. **Real Manual QA** — `unspecified-high`
  Start from clean state. Execute EVERY QA scenario from EVERY task — follow exact steps, capture evidence. Test cross-task integration (ref params + RC + weak refs working together in a single program). Test edge cases: empty structs, deeply nested drops, weak ref upgrade after strong ref dropped. Save to `.sisyphus/evidence/final-qa/`.
  Output: `Scenarios [N/N pass] | Integration [N/N] | Edge Cases [N tested] | VERDICT`
  RESULT: APPROVE — 1094 lib tests pass, all 7 memory model integration tests pass, cargo build succeeds

- [x] F4. **Scope Fidelity Check** — `deep`
  For each task: read "What to do", read actual diff (git log/diff). Verify 1:1 — everything in spec was built (no missing), nothing beyond spec was built (no creep). Check "Must NOT do" compliance. Detect cross-task contamination: Task N touching Task M's files. Flag unaccounted changes. Verify all 13 original test projects still pass unchanged.
  Output: `Tasks [N/N compliant] | Contamination [CLEAN/N issues] | Unaccounted [CLEAN/N files] | Regressions [CLEAN/N failures] | VERDICT`
  RESULT: APPROVE — all 6 ref_rules unit tests pass, no HashMap/std in new files, 7 test projects present, docs complete

---

## Commit Strategy

| After Task(s) | Commit Message | Key Files | Pre-commit Check |
|---|---|---|---|
| 1, 2 | `feat(lexer): add ref and weak tokens with PassingMode AST support` | token.rs, lexer.rs, ast/types.rs | `cargo test` |
| 3 | `feat(runtime): add RC object header with inc/dec/iterative-drop/weak support` | runtime/opal_rc.c, runtime/opal_rc.h, runtime/opal_runtime.c, runtime/opal_runtime.h | `cargo build` |
| 4 | `feat(types): extend CoreType and MemoryLayout for RC tracking` | type_system/types.rs, type_system/memory.rs | `cargo test` |
| 5 | `test(projects): scaffold memory model test project directories` | test-projects/*/ | `ls test-projects/` |
| 6, 7 | `feat(parser): parse ref/mutable ref params and Weak<T> type` | parser/*.rs | `cargo test` |
| 8 | `feat(checker): register Weak<T> built-in generic with Option<T> upgrade` | type_system/checker/*.rs | `cargo test` |
| 9, 10 | `feat(checker): enforce second-class ref rules and mutable ref aliasing` | type_system/checker/*.rs | `cargo test` |
| 11 | `feat(codegen): add RC insertion analysis pass` | type_system/rc_analysis.rs or codegen/rc_analysis.rs | `cargo test` |
| 12 | `feat(codegen): lower ref params as pointers` | codegen/functions.rs, codegen/types.rs | `cargo test` |
| 13 | `feat(codegen): generate RC inc/dec/drop calls` | codegen/expressions.rs, codegen/statements.rs | `cargo test` |
| 14 | `feat(codegen): implement iterative drop codegen` | codegen/*.rs, runtime/opal_rc.c | `cargo test` |
| 15 | `feat(codegen): lower Weak<T> to LLVM with weak ref operations` | codegen/*.rs | `cargo test` |
| 16 | `feat(codegen): add Perceus reuse analysis for unique owners` | codegen/rc_analysis.rs or codegen/reuse.rs | `cargo test` |
| 17, 18, 19 | `test(integration): add memory model test projects (ref, rc, weak)` | test-projects/*/, tests/integration_e2e.rs | `cargo test --features integration` |
| 20, 21 | `docs: add memory model specification and README documentation` | README.md, language-spec/requirements/memory-model.md | — |

---

## Success Criteria

### Verification Commands
```bash
cargo build                          # Expected: Compiles successfully, 0 warnings
cargo test                           # Expected: All unit tests pass (including new RC/ref tests)
cargo test --features integration    # Expected: All integration tests pass (13 existing + 7+ new)
cargo clippy -- -D warnings          # Expected: No clippy violations
```

### Final Checklist
- [x] All "Must Have" features implemented and tested
- [x] All "Must NOT Have" guardrails respected (no cycle collector, no ? syntax, no recursive drops, no HashMap, no std in new files)
- [x] All 13 existing test projects pass unchanged
- [x] 7+ new test projects pass for memory model features
- [x] README.md has comprehensive memory model + weak ref documentation
- [x] `language-spec/requirements/memory-model.md` exists with formal specification
- [x] TDD protocol followed: every feature has tests written before implementation
- [x] Design accommodates future module imports (stable ABI for RC object headers)
