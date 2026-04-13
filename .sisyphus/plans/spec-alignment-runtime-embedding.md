# Spec Alignment, Runtime Embedding & Colon-Block Parser

## TL;DR

> **Quick Summary**: Replace all test-project `.op` files with exact copies from `language-spec/`, then iteratively fix the compiler (parser, lexer, codegen, type checker, runtime linking) so these spec files compile and run correctly. Embed the C runtime in the compiler binary to eliminate the `runtime/` folder dependency. Align the VS Code extension and documentation with the language spec.
> 
> **Deliverables**:
> - Test projects use byte-for-byte copies of language-spec `.op` files
> - Parser supports colon-based indentation blocks (`if cond:`, `while cond:`, `for x in y:`)
> - Lexer emits `Indent`/`Dedent` tokens for colon-block scoping
> - `int32` type support in type checker and codegen
> - Entry function accepts `args: string[]` parameter
> - `loop` statement with `break` (multi-value) and `continue`
> - `guard...into...else` error handling syntax
> - `if...else:` chains
> - `import ... from ...` syntax (parser recognition, runtime binding)
> - `is` operator for equality comparison
> - C runtime embedded in compiler binary via `include_str!` (no `runtime/` folder needed)
> - VS Code extension aligned with language-spec keywords
> - README.md and PLAN.md updated
> 
> **Estimated Effort**: XL
> **Parallel Execution**: YES — 5 waves
> **Critical Path**: T1 → T2 → T3 → T5 → T7 → T9 → T11 → T13 → T15 → T16 → T17 → F1-F4

---

## Context

### Original Request
The user identified that test-project `.op` files diverge significantly from the language-spec examples. They use brace syntax, wrong types (`int64` vs `int32`), wrong function signatures, and miss features like colon-blocks, `guard`, `loop`, and `import`. The user wants:
1. Test project files replaced with **exact copies** from `language-spec/`
2. Compiler fixed iteratively (TDD) until these spec files compile and run
3. C runtime embedded in the compiler binary (no `runtime/` folder dependency)
4. VS Code extension aligned with language-spec syntax
5. Documentation updated

### Interview Summary
**Key Discussions**:
- Test files must be byte-for-byte copies of `language-spec/*.op` — no modifications
- Runtime embedding should follow Cargo's approach: self-contained binary, no external files
- All 4 test projects (hello_world, fib_recursive, fib_iterative, simple_quiz) are in scope
- TDD approach: replace files, run tests, fix failures, iterate
- `include_str!` is already used in the codebase for test fixtures — established pattern

**Research Findings (6 explore agents)**:
- Parser uses brace-only blocks for `if`, `while`, `for` — `parse_if_statement()`, `parse_while_statement()`, `parse_for_statement()` all expect `LeftBrace`/`RightBrace` in `src/parser/statements.rs`
- `Indent`/`Dedent` token types exist in `src/token.rs` but lexer never emits them
- `src/compiler.rs:194` hardcodes `Path::new("runtime/opal_runtime.c")` — loaded from disk
- `link_object_file()` uses `Command::new("cc")` to compile + link C runtime with generated `.o`
- Integration tests in `tests/integration_e2e.rs` assert specific output strings ("Hello world", "55")
- Entry functions use `__opalescent_entry_main` naming with C main wrapper via `emit_c_main_wrapper()`
- The C runtime is 43 lines — trivial to embed

### Metis Review
**Identified Gaps** (addressed):
- Brace coexistence: Colon-blocks for control flow (`if`, `while`, `for`, `loop`), braces retained for other constructs (struct literals, guard-else blocks). Auto-resolved — spec is clear.
- int32 scope: Must be implemented — spec files use `int32`, test files are exact copies
- Entry function args: Must handle `f(args: string[]): void` — codegen needs to accept (and possibly ignore) the args parameter
- simple_quiz features: `loop`, `break` (multi-value), `continue`, `guard...into...else`, `import...from`, `if...else:` — all must be implemented
- Portability: Linux + macOS (both have `cc`). Windows noted as future work.
- Edge cases: Empty colon-blocks, nested colon-blocks, mixed tabs/spaces, EOF after colon, comments in blocks

---

## Work Objectives

### Core Objective
Make the Opalescent compiler fully spec-compliant for the 4 test-project programs (hello_world, fib_recursive, fib_iterative, simple_quiz) by implementing all missing language features, embedding the runtime, and aligning tooling.

### Concrete Deliverables
- `test-projects/hello-world/src/main.op` — exact copy of `language-spec/hello_world.op`
- `test-projects/fib-recursive/src/main.op` — exact copy of `language-spec/fib_recursive.op`
- `test-projects/fib-iterative/src/main.op` — exact copy of `language-spec/fib_iterative.op`
- `test-projects/simple-quiz/src/main.op` — exact copy of `language-spec/simple_quiz.op`
- Compiler binary that works without `runtime/` folder
- VS Code extension with corrected keyword highlighting
- Updated README.md and PLAN.md

### Definition of Done
- [ ] `cargo test` — all unit tests pass
- [ ] `cargo test --features integration` — all 4 integration tests pass
- [ ] Test project `.op` files are byte-for-byte identical to `language-spec/` originals (verified with `diff`)
- [ ] Compiler works from any directory without `runtime/` folder present
- [ ] `diff test-projects/hello-world/src/main.op language-spec/hello_world.op` returns empty
- [ ] `diff test-projects/fib-recursive/src/main.op language-spec/fib_recursive.op` returns empty
- [ ] `diff test-projects/fib-iterative/src/main.op language-spec/fib_iterative.op` returns empty
- [ ] `diff test-projects/simple-quiz/src/main.op language-spec/simple_quiz.op` returns empty

### Must Have
- Exact spec file copies in test projects
- Colon-based indentation blocks for `if`, `while`, `for`, `loop`
- `int32` type support
- Entry function with `args: string[]` parameter
- `loop` statement with `break` and `continue`
- `guard...into...else` syntax
- `if...else:` chains
- `import ... from ...` syntax
- `is` operator for equality
- Embedded C runtime (no external files)
- VS Code extension keyword alignment
- All integration tests passing

### Must NOT Have (Guardrails)
- **No removal of brace syntax** — braces must still work for backward compatibility and for constructs like struct literals, guard-else blocks
- **No `.types.op` parser changes** — deferred (no test project uses type definition files)
- **No new language features beyond what the 4 spec files require** — no generics, no algebraic data types, no pattern matching implementation
- **No changes to the package manager, formatter, or LSP** — out of scope
- **No Windows-specific linker changes** — Linux + macOS only for now
- **No modification of `language-spec/` files** — these are the source of truth
- **No over-engineering the indent/dedent system** — implement the minimum needed for the 4 spec files to compile
- **No changing the C runtime's ABI** — the int32 support should map to existing int64 C functions (widen on call, narrow on return) unless absolutely necessary

---

## Verification Strategy (MANDATORY)

> **ZERO HUMAN INTERVENTION** — ALL verification is agent-executed. No exceptions.

### Test Decision
- **Infrastructure exists**: YES — `cargo test` and `cargo test --features integration`
- **Automated tests**: TDD — write/fix tests alongside implementation
- **Framework**: Rust's built-in test framework + integration tests gated behind `integration` feature
- **TDD flow**: Replace spec files → run tests → fix parser/codegen → run tests → repeat

### QA Policy
Every task MUST include agent-executed QA scenarios.
Evidence saved to `.sisyphus/evidence/task-{N}-{scenario-slug}.{ext}`.

- **Compiler features**: Use Bash — `cargo test`, `cargo test --features integration`, compile spec files directly
- **VS Code extension**: Use Bash — validate JSON syntax, check keyword lists
- **File identity**: Use Bash — `diff` commands to verify spec file copies

---

## Execution Strategy

### Parallel Execution Waves

```
Wave 0 (Pre-requisite — commit current state):
└── Task 1: Commit current changes [quick]

Wave 1 (Foundation — replace files + core lexer/parser groundwork, 5 parallel):
├── Task 2: Replace test-project .op files with exact spec copies [quick]
├── Task 3: Embed C runtime in compiler binary [deep]
├── Task 4: Add int32 type support to type checker + codegen [deep]
├── Task 5: Implement Indent/Dedent token emission in lexer [deep]
├── Task 6: Add `is` operator for equality comparison [quick]

Wave 2 (Parser features — colon-blocks + control flow, 5 parallel after Wave 1):
├── Task 7: Colon-block parsing for if/while/for statements [deep]
├── Task 8: Add if...else: chain parsing [deep]
├── Task 9: Entry function with args: string[] parameter [deep]
├── Task 10: Import...from syntax parsing + binding [deep]

Wave 3 (Advanced features — loop/break/continue/guard, 3 parallel after Wave 2):
├── Task 11: Loop statement with break (multi-value) and continue [deep]
├── Task 12: Guard...into...else error handling [deep]

Wave 4 (Integration + polish, 5 parallel after Wave 3):
├── Task 13: Update integration test assertions [unspecified-high]
├── Task 14: TDD iteration — compile all 4 spec files end-to-end [deep]
├── Task 15: Align VS Code extension with language-spec [quick]
├── Task 16: Update README.md [writing]
├── Task 17: Update PLAN.md [writing]

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
| 1 | — | 2-17 | 0 |
| 2 | 1 | 13, 14 | 1 |
| 3 | 1 | 14 | 1 |
| 4 | 1 | 7, 8, 9, 14 | 1 |
| 5 | 1 | 7, 8, 10, 11, 12 | 1 |
| 6 | 1 | 7, 14 | 1 |
| 7 | 4, 5, 6 | 11, 12, 14 | 2 |
| 8 | 5, 7 | 14 | 2 |
| 9 | 4 | 14 | 2 |
| 10 | 5 | 12, 14 | 2 |
| 11 | 7 | 12, 14 | 3 |
| 12 | 7, 10, 11 | 14 | 3 |
| 13 | 2 | 14 | 4 |
| 14 | 3, 7, 8, 9, 10, 11, 12, 13 | F1-F4 | 4 |
| 15 | 1 | F1-F4 | 4 |
| 16 | 3, 14 | F1-F4 | 4 |
| 17 | 3, 14 | F1-F4 | 4 |

### Agent Dispatch Summary

- **Wave 0**: **1** — T1 → `quick`
- **Wave 1**: **5** — T2 → `quick`, T3 → `deep`, T4 → `deep`, T5 → `deep`, T6 → `quick`
- **Wave 2**: **4** — T7 → `deep`, T8 → `deep`, T9 → `deep`, T10 → `deep`
- **Wave 3**: **2** — T11 → `deep`, T12 → `deep`
- **Wave 4**: **5** — T13 → `unspecified-high`, T14 → `deep`, T15 → `quick`, T16 → `writing`, T17 → `writing`
- **FINAL**: **4** — F1 → `oracle`, F2 → `unspecified-high`, F3 → `unspecified-high`, F4 → `deep`

---

## TODOs

- [x] 1. Commit Current State

  **What to do**:
  - Stage all current changes (including the modified `test-projects/simple-quiz/src/main.op` and untracked `.sisyphus/drafts/` directory)
  - Create a commit with message: `chore: commit current state before spec alignment`
  - This preserves the current state as a rollback point

  **Must NOT do**:
  - Do not push to remote
  - Do not modify any files

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: [`git-master`]
    - `git-master`: Git operations are the sole focus of this task

  **Parallelization**:
  - **Can Run In Parallel**: NO
  - **Parallel Group**: Wave 0 (solo)
  - **Blocks**: Tasks 2-17
  - **Blocked By**: None

  **References**:
  - `git status` shows: `M test-projects/simple-quiz/src/main.op` and `?? .sisyphus/drafts/`

  **Acceptance Criteria**:

  **QA Scenarios (MANDATORY):**

  ```
  Scenario: Commit created successfully
    Tool: Bash
    Steps:
      1. Run `git log --oneline -1`
      2. Assert output contains "chore: commit current state before spec alignment"
    Expected Result: Most recent commit has the expected message
    Evidence: .sisyphus/evidence/task-1-commit-created.txt

  Scenario: Working tree is clean after commit
    Tool: Bash
    Steps:
      1. Run `git status --porcelain`
      2. Assert output is empty (no modified or untracked files)
    Expected Result: Empty output — clean working tree
    Evidence: .sisyphus/evidence/task-1-clean-tree.txt
  ```

  **Commit**: YES
  - Message: `chore: commit current state before spec alignment`
  - Files: all staged changes
  - Pre-commit: none

---

- [x] 2. Replace Test-Project .op Files with Exact Language-Spec Copies

  **What to do**:
  - Copy `language-spec/hello_world.op` → `test-projects/hello-world/src/main.op`
  - Copy `language-spec/fib_recursive.op` → `test-projects/fib-recursive/src/main.op`
  - Copy `language-spec/fib_iterative.op` → `test-projects/fib-iterative/src/main.op`
  - Copy `language-spec/simple_quiz.op` → `test-projects/simple-quiz/src/main.op`
  - Verify each copy is byte-for-byte identical using `diff`

  **Must NOT do**:
  - Do not modify the language-spec files
  - Do not add or remove any test project directories
  - Do not modify opal.toml or other project config files

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1 (with Tasks 3, 4, 5, 6)
  - **Blocks**: Tasks 13, 14
  - **Blocked By**: Task 1

  **References**:

  **Pattern References**:
  - `language-spec/hello_world.op` — Source file (20 lines). Uses `entry main = f(args: string[]): void =>` with tab indentation, string interpolation, `return void`
  - `language-spec/fib_recursive.op` — Source file (21 lines). Uses `let fib_recursive = f(n: int32): int32 =>` with `if n is 0:` colon-blocks
  - `language-spec/fib_iterative.op` — Source file (33 lines). Uses `while i <= n:` colon-block, mutable variables
  - `language-spec/simple_quiz.op` — Source file (74 lines). Uses `import...from`, `loop =>`, `guard...into...else`, `break` with labeled values, `continue`, `if...else:`

  **Acceptance Criteria**:

  **QA Scenarios (MANDATORY):**

  ```
  Scenario: All 4 files are byte-for-byte identical
    Tool: Bash
    Steps:
      1. Run `diff test-projects/hello-world/src/main.op language-spec/hello_world.op`
      2. Run `diff test-projects/fib-recursive/src/main.op language-spec/fib_recursive.op`
      3. Run `diff test-projects/fib-iterative/src/main.op language-spec/fib_iterative.op`
      4. Run `diff test-projects/simple-quiz/src/main.op language-spec/simple_quiz.op`
      5. Assert ALL diffs produce empty output
    Expected Result: All 4 diff commands return exit code 0 with no output
    Evidence: .sisyphus/evidence/task-2-diff-results.txt

  Scenario: Files have not been truncated or corrupted
    Tool: Bash
    Steps:
      1. Run `wc -l test-projects/hello-world/src/main.op` — assert 20 lines
      2. Run `wc -l test-projects/fib-recursive/src/main.op` — assert 21 lines
      3. Run `wc -l test-projects/fib-iterative/src/main.op` — assert 33 lines
      4. Run `wc -l test-projects/simple-quiz/src/main.op` — assert 74 lines
    Expected Result: Line counts match source files exactly
    Evidence: .sisyphus/evidence/task-2-line-counts.txt
  ```

  **Commit**: YES (groups with T2 standalone)
  - Message: `chore(test-projects): replace .op files with exact language-spec copies`
  - Files: `test-projects/*/src/main.op`
  - Pre-commit: `diff` verification

---

- [x] 3. Embed C Runtime in Compiler Binary

  **What to do**:
  - In `src/compiler.rs`, replace `let runtime_path = Path::new("runtime/opal_runtime.c");` (line 194) with embedded runtime using `include_str!("../runtime/opal_runtime.c")`
  - Modify `link_object_file()` or `compile_program()` to:
    1. Create a temporary file (using `tempfile` crate or `std::env::temp_dir()`)
    2. Write the embedded C runtime source to the temp file
    3. Pass the temp file path to the `cc` invocation instead of the hardcoded path
    4. Clean up the temp file after linking (or use a `tempfile::NamedTempFile` which auto-deletes)
  - Add `const RUNTIME_SOURCE: &str = include_str!("../runtime/opal_runtime.c");` near the top of `src/compiler.rs`
  - Remove the `runtime_path` parameter from `link_object_file()` signature — it should use the embedded constant
  - Update `link_object_file()` to accept `extra_c_sources: &[&str]` (content strings) instead of `extra_c_files: &[&Path]`
  - Ensure the temp file has a `.c` extension so `cc` treats it as C source
  - If using `tempfile` crate, add it to `Cargo.toml` under `[dependencies]`

  **Must NOT do**:
  - Do not delete the `runtime/` directory — it's still the source of truth for the C code
  - Do not change the C runtime's API/ABI
  - Do not change what functions the runtime provides
  - Do not use `build.rs` for this — `include_str!` is simpler and already used in the codebase

  **Recommended Agent Profile**:
  - **Category**: `deep`
  - **Skills**: []
    - No specialized skills needed — this is Rust systems programming

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1 (with Tasks 2, 4, 5, 6)
  - **Blocks**: Task 14
  - **Blocked By**: Task 1

  **References**:

  **Pattern References**:
  - `src/compiler.rs:184-196` — `compile_program()` function where `runtime_path` is hardcoded. This is where the embedding change happens.
  - `src/compiler.rs:140-176` — `link_object_file()` function that invokes `Command::new("cc")` with the runtime path. Must be modified to accept embedded source.

  **API/Type References**:
  - `runtime/opal_runtime.c` — The 43-line C runtime source that will be embedded. Contains: `opal_take_input()`, `opal_random_int32()`, `opal_string_to_int32()`, `opal_print_string()`, `opal_print_int()`

  **External References**:
  - `include_str!` macro: https://doc.rust-lang.org/std/macro.include_str.html — embeds file contents as `&str` at compile time
  - `std::env::temp_dir()`: https://doc.rust-lang.org/std/env/fn.temp_dir.html — cross-platform temp directory

  **WHY Each Reference Matters**:
  - `compiler.rs:184-196`: This is the exact code to modify — the `runtime_path` variable on line 194 must be replaced
  - `compiler.rs:140-176`: The `link_object_file()` function takes `extra_c_files: &[&Path]` and passes them to `cc` — must change to write embedded source to temp file first
  - `runtime/opal_runtime.c`: The content that will be embedded — agent needs to verify the include path is correct relative to `src/compiler.rs`

  **Acceptance Criteria**:
  - [ ] `cargo test` passes (unit tests still work)
  - [ ] `const RUNTIME_SOURCE: &str = include_str!(...)` exists in `src/compiler.rs`
  - [ ] No reference to `Path::new("runtime/opal_runtime.c")` in `src/compiler.rs`

  **QA Scenarios (MANDATORY):**

  ```
  Scenario: Compiler works without runtime folder
    Tool: Bash
    Preconditions: Project builds successfully with `cargo build`
    Steps:
      1. Run `cargo build`
      2. Run `mv runtime runtime_backup`
      3. Compile a simple program: `printf 'entry main = f(): void => {\n  print('"'"'test'"'"')\n  return void\n}\n' > /tmp/test_embed.op && ./target/debug/opalescent /tmp/test_embed.op`
      4. Assert the program compiles and runs (or at least the linker doesn't fail looking for runtime/opal_runtime.c)
      5. Run `mv runtime_backup runtime`
    Expected Result: Compilation succeeds without `runtime/` folder present
    Failure Indicators: Error message containing "runtime/opal_runtime.c" or "No such file"
    Evidence: .sisyphus/evidence/task-3-no-runtime-folder.txt

  Scenario: Existing tests still pass with embedded runtime
    Tool: Bash
    Steps:
      1. Run `cargo test`
      2. Assert all tests pass
    Expected Result: Zero test failures
    Evidence: .sisyphus/evidence/task-3-cargo-test.txt

  Scenario: Temp file cleanup after compilation
    Tool: Bash
    Steps:
      1. Count `.c` files in temp directory before: `ls /tmp/opal_runtime_*.c 2>/dev/null | wc -l`
      2. Compile a program
      3. Count `.c` files in temp directory after: `ls /tmp/opal_runtime_*.c 2>/dev/null | wc -l`
      4. Assert count did not increase (temp file was cleaned up)
    Expected Result: No lingering temp files
    Evidence: .sisyphus/evidence/task-3-temp-cleanup.txt
  ```

  **Commit**: YES
  - Message: `feat(compiler): embed C runtime in binary via include_str!`
  - Files: `src/compiler.rs`, possibly `Cargo.toml`
  - Pre-commit: `cargo test`

---

- [x] 4. Add int32 Type Support to Type Checker and Codegen

  **What to do**:
  - Add `Int32` variant to the type system (wherever `Int64` is defined — likely `src/ast.rs` or `src/type_system/types.rs`)
  - Update the lexer to recognize `int32` as a type keyword (in `src/lexer.rs`)
  - Update the parser to parse `int32` type annotations
  - Update the type checker to handle `Int32` type — arithmetic operations, comparison, assignment compatibility
  - Update codegen to emit `i32` LLVM type for `int32` (currently everything is `i64`)
  - The C runtime uses `int64_t` internally — when calling runtime functions with `int32` values, widen to `int64_t` at the call boundary (sign-extend). When receiving `int64_t` returns, truncate to `i32`.
  - Ensure `int64` still works — both types should coexist

  **Must NOT do**:
  - Do not remove `int64` support
  - Do not change the C runtime's function signatures
  - Do not implement implicit type coercion between `int32` and `int64` — explicit widening only at runtime boundaries

  **Recommended Agent Profile**:
  - **Category**: `deep`
  - **Skills**: []
    - No specialized skills needed — compiler internals work

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1 (with Tasks 2, 3, 5, 6)
  - **Blocks**: Tasks 7, 8, 9, 14
  - **Blocked By**: Task 1

  **References**:

  **Pattern References**:
  - `src/ast.rs` — AST type definitions. Search for `Int64` or `Integer` to find where numeric types are defined. The `Int32` variant should be added alongside.
  - `src/type_system/` — Type checking logic. Find where `Int64` is matched/handled and add parallel `Int32` handling.
  - `src/codegen/` — Code generation. Find where `context.i64_type()` is used for integer values and add `context.i32_type()` for `Int32`.

  **API/Type References**:
  - `src/token.rs` — Token types. Check if there's already an `Int32` keyword token or if it needs to be added.
  - `runtime/opal_runtime.c` — Runtime functions use `int64_t` parameters. `int32` values must be sign-extended to `int64_t` when passed to runtime.

  **External References**:
  - Inkwell `IntType`: The LLVM integer type API — `context.i32_type()` for 32-bit integers

  **WHY Each Reference Matters**:
  - `src/ast.rs`: Need to add `Int32` alongside existing `Int64` in type enums
  - `src/type_system/`: Every pattern match on integer types needs an `Int32` arm
  - `src/codegen/`: LLVM IR generation must use `i32` for `int32` typed values

  **Acceptance Criteria**:
  - [ ] `cargo test` passes
  - [ ] Parser accepts `int32` type annotation: `let x: int32 = 5`
  - [ ] Codegen emits `i32` LLVM type for `int32` variables

  **QA Scenarios (MANDATORY):**

  ```
  Scenario: int32 type parses and compiles
    Tool: Bash
    Steps:
      1. Run `printf 'entry main = f(): void => {\n  let x: int32 = 42\n  return void\n}\n' > /tmp/test_int32.op`
      2. Run `cargo run -- /tmp/test_int32.op`
      3. Assert exit code 0 (no compilation errors)
    Expected Result: Program compiles without type errors
    Evidence: .sisyphus/evidence/task-4-int32-compiles.txt

  Scenario: int64 still works (backward compatibility)
    Tool: Bash
    Steps:
      1. Run `cargo test` — existing tests use int64
      2. Assert all tests pass
    Expected Result: No regressions in existing int64 functionality
    Evidence: .sisyphus/evidence/task-4-int64-compat.txt
  ```

  **Commit**: YES (groups with T6)
  - Message: `feat(types): add int32 type support and is operator`
  - Files: `src/ast.rs`, `src/token.rs`, `src/lexer.rs`, `src/parser/*`, `src/type_system/*`, `src/codegen/*`
  - Pre-commit: `cargo test`

---

- [x] 5. Implement Indent/Dedent Token Emission in Lexer

  **What to do**:
  - Modify `src/lexer.rs` to emit `Indent` and `Dedent` tokens based on indentation changes
  - The lexer currently tracks `Position` (line/column) but does NOT maintain an indentation stack
  - Implement an indentation stack that tracks the current indent level
  - After a `Colon` token followed by a `Newline`, begin tracking indentation:
    - If the next line's indentation is greater than the current level → emit `Indent` token, push new level
    - If the next line's indentation is less than the current level → emit one or more `Dedent` tokens, pop levels
    - If same indentation → no indent/dedent tokens
  - Handle both tabs and spaces (the spec files use BOTH — `hello_world.op` uses tabs, `fib_recursive.op` uses 4-space indentation)
  - At EOF, emit `Dedent` tokens for all remaining open indent levels
  - **CRITICAL**: Only activate indent/dedent tracking after tokens that start blocks — NOT colons in type annotations like `n: int32`. Block-starting tokens include:
    - `Colon` after control-flow conditions: `if cond:`, `while cond:`, `for x in y:`, `else:`
    - `Arrow` (`=>`) after function/lambda/loop/guard declarations: `f(...): T =>`, `loop =>`, `else e =>`
    - `name: type` colons do NOT trigger indent tracking — disambiguate by context
  - The `Indent` and `Dedent` token types already exist in `src/token.rs` — they just need to be emitted
  - **Note**: The existing `parse_blockless_body_statements()` in `helpers.rs` already handles `=>` + indented body using column heuristics. The indent/dedent tokens provide a more robust mechanism that should eventually replace those heuristics, but both can coexist during transition.

  **Must NOT do**:
  - Do not break existing brace-based parsing — brace blocks must still work
  - Do not change how `Newline` tokens are emitted
  - Do not require ALL code to use indent/dedent — it's only for colon-blocks
  - Do not enforce consistent indentation style (tabs vs spaces) — accept both

  **Recommended Agent Profile**:
  - **Category**: `deep`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1 (with Tasks 2, 3, 4, 6)
  - **Blocks**: Tasks 7, 8, 10, 11, 12
  - **Blocked By**: Task 1

  **References**:

  **Pattern References**:
  - `src/lexer.rs` — The main lexer file. Find the `next_token()` or equivalent method. The indent tracking logic goes here.
  - `src/token.rs` — Token type definitions. `Indent` and `Dedent` are already defined but never used. Check exact variant names.

  **API/Type References**:
  - `src/token.rs:Position` — Position struct tracking line/column. Used to determine indentation level.
  - `src/token.rs:TokenType::Indent` — The existing indent token variant (unused)
  - `src/token.rs:TokenType::Dedent` — The existing dedent token variant (unused)
  - `src/token.rs:TokenType::Colon` — The colon token that triggers indent tracking
  - `src/token.rs:TokenType::Newline` — Newline token — indent/dedent emission happens after newlines

  **External References**:
  - Python lexer indent/dedent: https://docs.python.org/3/reference/lexical_analysis.html#indentation — Python's approach to indent/dedent is the gold standard. Opalescent's approach is similar but only activates after colon.

  **WHY Each Reference Matters**:
  - `src/lexer.rs`: This IS the file being modified — the core lexing logic
  - `src/token.rs`: Need to verify exact token variant names and ensure no changes needed
  - Python's approach: Model for how to handle indent stack, multiple dedents, EOF dedent emission

  **Acceptance Criteria**:
  - [ ] `cargo test` passes (existing tests unbroken)
  - [ ] Lexer emits `Indent` after `Colon + Newline + increased indentation` (for control flow)
  - [ ] Lexer emits `Indent` after `Arrow + Newline + increased indentation` (for `loop =>`, `else e =>`, function bodies)
  - [ ] Lexer emits `Dedent` when indentation decreases
  - [ ] Lexer emits multiple `Dedent` tokens for multiple-level dedent
  - [ ] Lexer emits `Dedent` tokens at EOF for open indent levels
  - [ ] Colons in type annotations (`n: int32`) do NOT trigger indent tracking

  **QA Scenarios (MANDATORY):**

  ```
  Scenario: Colon-block produces indent/dedent tokens
    Tool: Bash
    Preconditions: Unit tests for indent/dedent have been added as part of this task's implementation
    Steps:
      1. Run `cargo test lexer -- --nocapture 2>&1 | tee .sisyphus/evidence/task-5-indent-dedent-tokens.txt`
      2. Assert exit code 0 (all lexer tests pass, including new indent/dedent tests)
      3. Verify new test name appears in output (e.g., `test_colon_block_indent` or similar)
    Expected Result: All lexer tests pass, including tests verifying Indent/Dedent emission after colon-blocks
    Evidence: .sisyphus/evidence/task-5-indent-dedent-tokens.txt

  Scenario: Type annotation colons do NOT trigger indent
    Tool: Bash
    Preconditions: Unit test for type annotation colon disambiguation added in this task
    Steps:
      1. Run `cargo test lexer -- --nocapture 2>&1 | tee .sisyphus/evidence/task-5-no-indent-type-annotation.txt`
      2. Assert exit code 0
    Expected Result: Test proves `let x: int32 = 5` produces NO Indent/Dedent tokens
    Evidence: .sisyphus/evidence/task-5-no-indent-type-annotation.txt

  Scenario: Existing brace-based tests still pass
    Tool: Bash
    Steps:
      1. Run `cargo test 2>&1 | tee .sisyphus/evidence/task-5-no-regressions.txt`
      2. Assert exit code 0 and output contains "test result: ok"
    Expected Result: Zero regressions — all existing tests pass
    Evidence: .sisyphus/evidence/task-5-no-regressions.txt
  ```

  **Commit**: YES (groups with T7, T8)
  - Message: `feat(parser): add colon-block indentation parsing with indent/dedent tokens`
  - Files: `src/lexer.rs`
  - Pre-commit: `cargo test`

---

- [x] 6. Add `is` Operator for Equality Comparison

  **What to do**:
  - The spec uses `is` for equality comparison: `if n is 0:`, `if user_number is quiz_num:`
  - Check if `is` is already a keyword in `src/token.rs` — it likely is (used in parsing)
  - If not already handled as an equality operator in the parser, add it:
    - In the expression parser, treat `is` as equivalent to `==` (binary equality comparison)
    - Generate the same LLVM IR as `==` for `is` expressions
  - The type checker should validate that both sides of `is` have the same type
  - `is` should work for both integer and string comparisons

  **Must NOT do**:
  - Do not remove `==` support — both should work
  - Do not implement `is` as identity comparison (pointer comparison) — it's value equality

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1 (with Tasks 2, 3, 4, 5)
  - **Blocks**: Task 7, 14
  - **Blocked By**: Task 1

  **References**:

  **Pattern References**:
  - `src/parser/expressions.rs` — Expression parsing. Find where `==` is handled and add `is` as an alternative.
  - `src/token.rs` — Check if `TokenType::Is` exists. If not, add it.
  - `src/lexer.rs` — Keyword recognition. If `is` needs to be added as a keyword.

  **API/Type References**:
  - `src/codegen/expressions.rs` — Code generation for binary expressions. `is` should generate the same `icmp eq` instruction as `==`.

  **WHY Each Reference Matters**:
  - Expression parser: Where `is` must be recognized as a binary operator
  - Token definitions: Need to verify `is` token exists
  - Codegen: Must emit `icmp eq` for `is` just like `==`

  **Acceptance Criteria**:
  - [ ] `cargo test` passes
  - [ ] `if n is 0:` parses successfully (with colon-block from Task 7)
  - [ ] `is` generates same LLVM IR as `==`

  **QA Scenarios (MANDATORY):**

  ```
  Scenario: is operator compiles and executes correctly
    Tool: Bash
    Steps:
      1. Run `printf 'entry main = f(): void => {\n  let x = 5\n  let y = 5\n  if x is y { print('"'"'equal'"'"') }\n  return void\n}\n' > /tmp/test_is_op.op`
      2. Run `cargo run -- /tmp/test_is_op.op 2>&1 | tee .sisyphus/evidence/task-6-is-operator.txt`
      3. Assert exit code 0 and stdout contains "equal"
    Expected Result: Program compiles, runs, and prints "equal" — confirming `is` works as equality
    Failure Indicators: Parse error on `is` token, type error, or "equal" not in output
    Evidence: .sisyphus/evidence/task-6-is-operator.txt

  Scenario: == still works (backward compatibility)
    Tool: Bash
    Steps:
      1. Run `cargo test 2>&1 | tee .sisyphus/evidence/task-6-eq-compat.txt`
      2. Assert exit code 0 and output contains "test result: ok"
    Expected Result: No regressions — existing tests using == still pass
    Evidence: .sisyphus/evidence/task-6-eq-compat.txt
  ```

  **Commit**: YES (groups with T4)
  - Message: `feat(types): add int32 type support and is operator`
  - Files: `src/token.rs`, `src/lexer.rs`, `src/parser/expressions.rs`, `src/codegen/expressions.rs`
  - Pre-commit: `cargo test`

---

- [x] 7. Colon-Block Parsing for if/while/for Statements

  **What to do**:
  - Modify `parse_if_statement()` in `src/parser/statements.rs` to accept BOTH syntaxes:
    - **Existing**: `if cond { body }` (brace-based) — keep working
    - **New**: `if cond:` followed by `Indent`, body statements, `Dedent` (colon-block)
  - Similarly modify `parse_while_statement()` for `while cond:` + indent block
  - Similarly modify `parse_for_statement()` for `for x in y:` + indent block
  - Detection logic: After parsing the condition, check if next token is `LeftBrace` → use existing brace parsing. If next token is `Colon` → consume colon, expect `Newline`, `Indent`, parse body until `Dedent`.
  - The colon-block body should collect statements into the same `Block` AST node used by brace-blocks
  - Handle nested colon-blocks: `if cond1:` → indent → `if cond2:` → indent → body → dedent → dedent

  **Must NOT do**:
  - Do not remove brace-block support — both syntaxes coexist
  - Do not change the AST representation — `Block` is `Block` regardless of syntax
  - Do not modify the `parse_block_statement()` function — add a new `parse_indent_block()` function instead

  **Recommended Agent Profile**:
  - **Category**: `deep`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2 (with Tasks 8, 9, 10)
  - **Blocks**: Tasks 11, 12, 14
  - **Blocked By**: Tasks 4, 5, 6

  **References**:

  **Pattern References**:
  - `src/parser/statements.rs` — Contains `parse_if_statement()`, `parse_while_statement()`, `parse_for_statement()`. Each currently expects `LeftBrace`. Add colon-block alternative.
  - `src/parser/helpers.rs` — Contains `parse_blockless_body_statements()` which uses column-position heuristics for function bodies. Study this pattern — colon-blocks use a similar concept but with proper `Indent`/`Dedent` tokens.
  - `src/parser/declarations.rs` — Function body parsing after `=>`. The `=>` + indented body pattern is already partially supported here — study how it works.

  **API/Type References**:
  - `src/ast.rs:Block` — The AST node for blocks. Colon-blocks should produce the same `Block` node as brace-blocks.
  - `src/token.rs:TokenType::Indent` — The indent token emitted by the lexer (from Task 5)
  - `src/token.rs:TokenType::Dedent` — The dedent token marking end of indent block

  **WHY Each Reference Matters**:
  - `statements.rs`: This IS the file being modified — the `parse_if_statement()`, `parse_while_statement()`, `parse_for_statement()` functions
  - `helpers.rs`: The blockless body parsing shows existing column-based heuristics — useful pattern to understand
  - `ast.rs:Block`: Must verify the `Block` node structure to ensure colon-blocks produce compatible AST

  **Acceptance Criteria**:
  - [ ] `cargo test` passes
  - [ ] `if n is 0:\n    return 0` parses into valid AST (colon-block)
  - [ ] `while i <= n:\n    result = a + b\n    i = i + 1` parses correctly
  - [ ] Brace blocks still work: `if n is 0 { return 0 }`
  - [ ] Nested colon-blocks parse correctly

  **QA Scenarios (MANDATORY):**

  ```
  Scenario: if-statement with colon-block parses correctly
    Tool: Bash
    Preconditions: Parser unit tests for colon-block if-statements added as part of this task
    Steps:
      1. Run `cargo test parser -- --nocapture 2>&1 | tee .sisyphus/evidence/task-7-if-colon-block.txt`
      2. Assert exit code 0 (all parser tests pass, including new colon-block tests)
    Expected Result: Colon-block if tests pass — parser produces correct AST
    Evidence: .sisyphus/evidence/task-7-if-colon-block.txt

  Scenario: while-statement with colon-block parses correctly
    Tool: Bash
    Preconditions: Parser unit tests for colon-block while-statements added as part of this task
    Steps:
      1. Run `cargo test parser -- --nocapture 2>&1 | tee .sisyphus/evidence/task-7-while-colon-block.txt`
      2. Assert exit code 0
    Expected Result: Colon-block while tests pass
    Evidence: .sisyphus/evidence/task-7-while-colon-block.txt

  Scenario: Brace blocks still work (no regression)
    Tool: Bash
    Steps:
      1. Run `cargo test 2>&1 | tee .sisyphus/evidence/task-7-brace-compat.txt`
      2. Assert exit code 0 and output contains "test result: ok"
    Expected Result: Zero regressions in brace-block parsing
    Evidence: .sisyphus/evidence/task-7-brace-compat.txt
  ```

  **Commit**: YES (groups with T5, T8)
  - Message: `feat(parser): add colon-block indentation parsing with indent/dedent tokens`
  - Files: `src/parser/statements.rs`
  - Pre-commit: `cargo test`

---

- [x] 8. Add if...else: Chain Parsing

  **What to do**:
  - The spec uses `if...else:` chains: `if user_number is quiz_num:` → body → `if user_number < quiz_num:` → body → `else:` → body (see `simple_quiz.op` lines 61-68)
  - Modify `parse_if_statement()` to handle `else:` blocks after an if-block:
    - After the if-body (colon-block closes via Dedent), check if next token is `Else`
    - If `Else` followed by `Colon` → parse an else colon-block
    - If `Else` followed by `If` → parse an else-if chain
    - If `Else` followed by `LeftBrace` → existing brace-based else (keep working)
  - The `else:` block uses the same indent/dedent mechanism as `if cond:`

  **Must NOT do**:
  - Do not change how else-if chains work with brace syntax
  - Do not introduce new AST nodes — reuse existing If/Else AST structure

  **Recommended Agent Profile**:
  - **Category**: `deep`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2 (with Tasks 7, 9, 10)
  - **Blocks**: Task 14
  - **Blocked By**: Tasks 5, 7

  **References**:

  **Pattern References**:
  - `src/parser/statements.rs:parse_if_statement()` — The existing if parsing function. The else-handling logic is here — extend it.
  - `language-spec/simple_quiz.op:61-68` — The spec example of `if...else:` chains:
    ```
    if user_number is quiz_num:
        print('Wow, you guessed correctly!')
        return void
    if user_number < quiz_num:
        print('Oh no, too low!')
    else:
        print('Too high!')
    ```

  **API/Type References**:
  - `src/ast.rs` — The IfStatement AST node with optional else branch. Verify the else branch structure.

  **WHY Each Reference Matters**:
  - `parse_if_statement()`: The function being extended — else colon-block logic goes here
  - `simple_quiz.op:61-68`: The exact syntax we need to parse — reference for test cases

  **Acceptance Criteria**:
  - [ ] `cargo test` passes
  - [ ] `if cond:\n    body1\nelse:\n    body2` parses correctly
  - [ ] `if cond1:\n    body1\nif cond2:\n    body2\nelse:\n    body3` parses as two separate if-statements (second has else)

  **QA Scenarios (MANDATORY):**

  ```
  Scenario: if-else colon-block chain parses correctly
    Tool: Bash
    Preconditions: Parser unit tests for if-else colon-blocks added as part of this task
    Steps:
      1. Run `cargo test parser -- --nocapture 2>&1 | tee .sisyphus/evidence/task-8-if-else-colon.txt`
      2. Assert exit code 0 (all parser tests pass, including if-else colon-block tests)
    Expected Result: if-else with colon-blocks produces correct AST
    Evidence: .sisyphus/evidence/task-8-if-else-colon.txt

  Scenario: Standalone if (no else) still works
    Tool: Bash
    Steps:
      1. Run `cargo test 2>&1 | tee .sisyphus/evidence/task-8-no-regression.txt`
      2. Assert exit code 0 and output contains "test result: ok"
    Expected Result: No regressions
    Evidence: .sisyphus/evidence/task-8-no-regression.txt
  ```

  **Commit**: YES (groups with T5, T7)
  - Message: `feat(parser): add colon-block indentation parsing with indent/dedent tokens`
  - Files: `src/parser/statements.rs`
  - Pre-commit: `cargo test`

---

- [x] 9. Entry Function with args: string[] Parameter

  **What to do**:
  - The spec uses `entry main = f(args: string[]): void =>` — the entry function takes an `args: string[]` parameter
  - Currently the compiler expects `f(): void` for entry functions (zero parameters)
  - Modify the parser to accept parameters in entry function declarations
  - Modify the type checker to accept `string[]` parameter type for entry functions
  - Modify codegen:
    - The `emit_c_main_wrapper()` function generates a C `main()` that calls `__opalescent_entry_main()`
    - Update the wrapper to pass `argc`/`argv` to the entry function as a string array
    - OR: Accept the parameter in the signature but pass a null/empty array (simpler, still spec-compliant for now)
  - The simpler approach: Accept the `args` parameter in the function signature, bind it in the local scope, but pass an empty array or null pointer. This makes the spec files parse and compile while deferring full argv support.
  - **Key insight**: The spec files declare `args: string[]` but none of the 4 test programs actually USE the `args` variable in their body. So a dummy binding is sufficient.

  **Must NOT do**:
  - Do not break existing entry functions with `f(): void` — both should work
  - Do not implement full argv parsing if it's complex — a dummy binding is fine for now

  **Recommended Agent Profile**:
  - **Category**: `deep`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2 (with Tasks 7, 8, 10)
  - **Blocks**: Task 14
  - **Blocked By**: Task 4

  **References**:

  **Pattern References**:
  - `src/parser/declarations.rs` — Entry function declaration parsing. Find where `entry` keyword is handled and where parameter list is parsed.
  - `src/codegen/` — Search for `emit_c_main_wrapper` or `__opalescent_entry_main` to find where the entry function is generated.
  - `src/type_system/` — Search for `entry` to find where entry function type is validated.

  **API/Type References**:
  - `src/ast.rs` — Function declaration AST node. Check how parameters are represented.
  - `src/codegen/` — The C main wrapper that calls the entry function. Must be updated to match new signature.

  **WHY Each Reference Matters**:
  - Parser: Must accept `f(args: string[]): void` in addition to `f(): void` for entry functions
  - Codegen: The C main wrapper must generate a call that matches the entry function's LLVM signature
  - Type checker: Must not reject `string[]` parameter on entry functions

  **Acceptance Criteria**:
  - [ ] `cargo test` passes
  - [ ] `entry main = f(args: string[]): void =>` parses without error
  - [ ] `entry main = f(): void =>` still works (backward compat)
  - [ ] Codegen produces valid LLVM IR for entry function with string[] param

  **QA Scenarios (MANDATORY):**

  ```
  Scenario: Entry function with args parameter compiles
    Tool: Bash
    Steps:
      1. Run `printf 'entry main = f(args: string[]): void =>\n    print('"'"'hello'"'"')\n    return void\n' > /tmp/test_entry_args.op`
      2. Run `cargo run -- /tmp/test_entry_args.op 2>&1 | tee .sisyphus/evidence/task-9-entry-args.txt`
      3. Assert exit code 0 and stdout contains "hello"
    Expected Result: Program compiles and prints "hello"
    Failure Indicators: Parse error on `args: string[]`, type error, "hello" not in output
    Evidence: .sisyphus/evidence/task-9-entry-args.txt

  Scenario: Entry function without args still works
    Tool: Bash
    Steps:
      1. Run `cargo test` — existing tests use `f(): void`
      2. Assert all pass
    Expected Result: No regressions
    Evidence: .sisyphus/evidence/task-9-no-args-compat.txt
  ```

  **Commit**: YES
  - Message: `feat(codegen): entry function accepts args: string[] parameter`
  - Files: `src/parser/declarations.rs`, `src/codegen/*`, `src/type_system/*`
  - Pre-commit: `cargo test`

---

- [x] 10. Import...from Syntax Parsing and Runtime Binding

  **What to do**:
  - The spec uses: `import take_input, string_to_int32 from standard` and `import random_int32 from math`
  - Implement import statement parsing in the parser:
    - `import <name1>, <name2>, ... from <module>` syntax
    - Import statements must appear at the top of the file (before any other declarations)
    - Parse into an `ImportStatement` AST node
  - Add `ImportStatement` to the AST if it doesn't exist: fields for imported names and module name
  - Add `import` and `from` as keywords in the lexer/token types (if not already present)
  - For codegen: The imported names (`take_input`, `string_to_int32`, `random_int32`) should be treated as aliases to the existing runtime function bindings:
    - `take_input` → `opal_take_input` (already bound in codegen)
    - `string_to_int32` → `opal_string_to_int32` (already bound)
    - `random_int32` → `opal_random_int32` (already bound)
  - This is a "soft" import system — it registers name bindings that map to existing runtime functions. No module loading, no file resolution.

  **Must NOT do**:
  - Do not implement a real module system — just name aliasing for now
  - Do not implement file resolution or dependency loading
  - Do not reject programs that DON'T use imports (backward compat)

  **Recommended Agent Profile**:
  - **Category**: `deep`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2 (with Tasks 7, 8, 9)
  - **Blocks**: Tasks 12, 14
  - **Blocked By**: Task 5

  **References**:

  **Pattern References**:
  - `src/parser/` — Find where top-level declarations are parsed. Import statements go before other declarations.
  - `src/codegen/` — Search for `opal_take_input`, `opal_random_int32`, `opal_string_to_int32` to find where runtime functions are declared/bound. Imports should create local aliases to these.
  - `language-spec/simple_quiz.op:8-9` — The exact import syntax to parse:
    ```
    import take_input, string_to_int32 from standard
    import random_int32 from math
    ```

  **API/Type References**:
  - `src/ast.rs` — Need to add `ImportStatement` node with `names: Vec<String>` and `module: String`
  - `src/token.rs` — Check if `Import` and `From` keyword tokens exist

  **WHY Each Reference Matters**:
  - Parser: Must handle the `import...from` syntax at file top level
  - Codegen: Imported names must resolve to existing runtime function bindings
  - `simple_quiz.op:8-9`: The exact syntax that must parse — test case reference

  **Acceptance Criteria**:
  - [ ] `cargo test` passes
  - [ ] `import take_input from standard` parses into ImportStatement AST node
  - [ ] `import take_input, string_to_int32 from standard` handles multiple imports
  - [ ] Imported names resolve to runtime function bindings in codegen

  **QA Scenarios (MANDATORY):**

  ```
  Scenario: Import statement parses correctly
    Tool: Bash
    Preconditions: Parser unit tests for import...from syntax added as part of this task
    Steps:
      1. Run `cargo test parser -- --nocapture 2>&1 | tee .sisyphus/evidence/task-10-import-parses.txt`
      2. Assert exit code 0 (all parser tests pass, including import tests)
    Expected Result: Import parses into correct AST node
    Evidence: .sisyphus/evidence/task-10-import-parses.txt

  Scenario: Multiple imports from same module
    Tool: Bash
    Preconditions: Unit test for multi-import added in this task
    Steps:
      1. Run `cargo test parser -- --nocapture 2>&1 | tee .sisyphus/evidence/task-10-multi-import.txt`
      2. Assert exit code 0
    Expected Result: Multiple names parsed correctly
    Evidence: .sisyphus/evidence/task-10-multi-import.txt
  ```

  **Commit**: YES
  - Message: `feat(parser): add import...from syntax`
  - Files: `src/lexer.rs`, `src/token.rs`, `src/parser/*`, `src/ast.rs`
  - Pre-commit: `cargo test`

---

- [x] 11. Loop Statement with Break (Multi-Value) and Continue

  **What to do**:
  - The spec uses `loop =>` as an infinite loop construct (see `simple_quiz.op:25`):
    ```
    let user_input, user_number =
        loop =>
            let s = take_input()
            ...
            break user_input: s, user_number: n
    ```
  - Implement `loop` statement parsing:
    - `loop =>` followed by indented body (colon-block-style using `=>` instead of `:`)
    - `loop` is an expression that can return values via `break`
  - Implement `break` statement:
    - `break` alone → exit loop, no value
    - `break name1: val1, name2: val2` → exit loop with labeled return values
    - The labeled break values bind to the `let` destructuring on the left side of the `loop` assignment
  - Implement `continue` statement:
    - `continue` → jump to next loop iteration
    - Used inside `guard...else` blocks to retry on error
  - Add `loop`, `break`, `continue` as keywords (if not already)
  - Add AST nodes: `LoopStatement` (body), `BreakStatement` (optional labeled values), `ContinueStatement`
  - Codegen for `loop`:
    - Create LLVM basic blocks: `loop_body`, `loop_exit`
    - Branch to `loop_body` unconditionally
    - `break` → branch to `loop_exit` (with phi nodes for return values)
    - `continue` → branch to `loop_body`
  - **Note**: The `loop =>` syntax uses `=>` not `:` for its block. This is because `loop` is an expression (can return values) while `if`/`while`/`for` are statements. The parser should handle `loop =>` followed by `Indent`...`Dedent`.

  **Must NOT do**:
  - Do not implement labeled loops (e.g., `loop 'outer =>`) — not in the spec
  - Do not implement `for` loop break values — only `loop` supports value-returning break
  - Do not change how `while` or `for` loops work

  **Recommended Agent Profile**:
  - **Category**: `deep`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 3 (with Task 12)
  - **Blocks**: Tasks 12, 14
  - **Blocked By**: Task 7

  **References**:

  **Pattern References**:
  - `src/parser/statements.rs` — Where loop parsing goes. Study `parse_while_statement()` for similar structure.
  - `src/codegen/` — Search for `while` codegen to see how loop basic blocks are created. `loop` uses a similar pattern but with `br` (unconditional branch) instead of `br i1` (conditional).
  - `language-spec/simple_quiz.op:24-50` — The full loop example with break and continue:
    ```
    let user_input, user_number =
        loop =>
            let s = take_input()
            guard string_to_int32(s) into n else e =>
                print('Error: {e}')
                continue
            break user_input: s, user_number: n
    ```

  **API/Type References**:
  - `src/ast.rs` — Need to add `LoopExpression`, `BreakStatement`, `ContinueStatement` AST nodes

  **WHY Each Reference Matters**:
  - `statements.rs`: Where `loop` parsing logic goes
  - Codegen while-loop: Pattern to follow for loop basic block creation
  - `simple_quiz.op:24-50`: The exact syntax that must work — primary test case

  **Acceptance Criteria**:
  - [ ] `cargo test` passes
  - [ ] `loop => { ... break }` parses and generates valid LLVM IR
  - [ ] `break name: value` syntax parses correctly
  - [ ] `continue` generates branch back to loop header
  - [ ] Multi-value break with `let a, b = loop => ...` works

  **QA Scenarios (MANDATORY):**

  ```
  Scenario: Simple loop with break compiles and runs
    Tool: Bash
    Steps:
      1. Run `printf 'entry main = f(): void => {\n  let mutable i = 0\n  loop => {\n    i = i + 1\n    if i is 5 { break }\n  }\n  print('"'"'done'"'"')\n  return void\n}\n' > /tmp/test_loop.op`
      2. Run `cargo run -- /tmp/test_loop.op 2>&1 | tee .sisyphus/evidence/task-11-loop-break.txt`
      3. Assert exit code 0 and stdout contains "done"
    Expected Result: Loop executes 5 times, breaks, and prints "done"
    Failure Indicators: Parse error on `loop`, infinite loop (no output/timeout), "done" not in output
    Evidence: .sisyphus/evidence/task-11-loop-break.txt

  Scenario: Continue restarts loop iteration
    Tool: Bash
    Steps:
      1. Run `printf 'entry main = f(): void => {\n  let mutable i = 0\n  let mutable count = 0\n  loop => {\n    i = i + 1\n    if i > 10 { break }\n    if i is 3 { continue }\n    count = count + 1\n  }\n  print('"'"'{count}'"'"')\n  return void\n}\n' > /tmp/test_continue.op`
      2. Run `cargo run -- /tmp/test_continue.op 2>&1 | tee .sisyphus/evidence/task-11-continue.txt`
      3. Assert exit code 0 and stdout contains "9" (10 iterations minus 1 skipped by continue)
    Expected Result: Continue skips iteration where i==3, count ends at 9
    Failure Indicators: Parse error on `continue`, wrong count value, infinite loop
    Evidence: .sisyphus/evidence/task-11-continue.txt
  ```

  **Commit**: YES (groups with T12)
  - Message: `feat(parser): add loop/break/continue and guard...into...else`
  - Files: `src/parser/*`, `src/codegen/*`, `src/ast.rs`
  - Pre-commit: `cargo test`

---

- [x] 12. Guard...into...else Error Handling

  **What to do**:
  - The spec uses `guard...into...else` for error handling (see `simple_quiz.op:43-45`):
    ```
    guard string_to_int32(s) into n else e =>
        print('Error: {e}')
        continue
    ```
  - Implement `guard` statement parsing:
    - `guard <expr> into <binding> else <error_binding> =>` followed by indented body
    - `<expr>` is a function call that can produce errors
    - `into <binding>` binds the success value
    - `else <error_binding>` binds the error value
    - The else body executes when the expression produces an error
    - After the guard (if no error), `<binding>` is available as a local variable
  - Add `guard`, `into` keywords (if not already present)
  - AST node: `GuardStatement` with fields: `expression`, `success_binding`, `error_binding`, `else_body`
  - **Simplified codegen approach**: Since the current runtime functions don't actually return errors (e.g., `opal_string_to_int32` returns 0 on failure, not an error), implement guard as:
    - Call the function
    - Check if result indicates error (e.g., for `string_to_int32`, check if return value is some sentinel)
    - If error → execute else body
    - If success → bind to `into` variable
  - **Even simpler**: For now, treat `guard expr into binding else e => ...` as `let binding = expr` and skip the error path entirely. This makes the spec files compile. The error handling semantics can be refined later when the error type system is implemented.
  - Use the simpler approach — the goal is to make `simple_quiz.op` compile, not to implement full error handling

  **Must NOT do**:
  - Do not implement a full error type system — that's future work
  - Do not implement `propagate` keyword — not used in the 4 test files
  - Do not change function signatures to return Result types

  **Recommended Agent Profile**:
  - **Category**: `deep`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 3 (with Task 11)
  - **Blocks**: Task 14
  - **Blocked By**: Tasks 7, 10, 11

  **References**:

  **Pattern References**:
  - `src/parser/statements.rs` — Where guard statement parsing goes
  - `language-spec/simple_quiz.op:43-45` — The exact guard syntax:
    ```
    guard string_to_int32(s) into n else e =>
        print('Error: {e}')
        continue
    ```
  - `language-spec/requirements/overview.md` — May contain guard semantics description

  **API/Type References**:
  - `src/ast.rs` — Need to add `GuardStatement` AST node
  - `src/token.rs` — Need `Guard`, `Into` keywords (check if they exist)

  **WHY Each Reference Matters**:
  - `statements.rs`: Where guard parsing goes — follow the pattern of other statement parsers
  - `simple_quiz.op:43-45`: The exact syntax to parse — primary test case
  - `overview.md`: Understanding guard semantics for correct AST design

  **Acceptance Criteria**:
  - [ ] `cargo test` passes
  - [ ] `guard expr into binding else e => ...` parses into GuardStatement AST
  - [ ] Codegen produces working code (even if simplified — just calls function and binds result)
  - [ ] `continue` inside guard else body works (from Task 11)

  **QA Scenarios (MANDATORY):**

  ```
  Scenario: Guard statement parses correctly
    Tool: Bash
    Preconditions: Parser unit tests for guard...into...else syntax added as part of this task
    Steps:
      1. Run `cargo test parser -- --nocapture 2>&1 | tee .sisyphus/evidence/task-12-guard-parses.txt`
      2. Assert exit code 0 (all parser tests pass, including guard tests)
    Expected Result: Guard parses into correct AST node
    Evidence: .sisyphus/evidence/task-12-guard-parses.txt

  Scenario: Guard compiles in context of loop
    Tool: Bash
    Preconditions: Codegen for guard statement implemented in this task
    Steps:
      1. Run `cargo test 2>&1 | tee .sisyphus/evidence/task-12-guard-compiles.txt`
      2. Assert exit code 0
    Expected Result: All tests pass including guard-related tests
    Evidence: .sisyphus/evidence/task-12-guard-compiles.txt
  ```

  **Commit**: YES (groups with T11)
  - Message: `feat(parser): add loop/break/continue and guard...into...else`
  - Files: `src/parser/*`, `src/codegen/*`, `src/ast.rs`, `src/token.rs`
  - Pre-commit: `cargo test`

---

- [x] 13. Update Integration Test Assertions

  **What to do**:
  - The integration tests in `tests/integration_e2e.rs` assert specific output strings that will change when test project files are replaced with spec copies:
    - `hello_world` test: Currently asserts stdout contains `"Hello world"` → Still correct (spec also prints "Hello world")
    - `fib_recursive` test: Currently asserts stdout contains `"55"` → Must change to assert `"fib(10) = 55"` (spec uses `print('fib({n}) = {result}')`)
    - `fib_iterative` test: Currently asserts stdout contains `"55"` → Must change to assert `"fib(10) = 55"`
    - `simple_quiz` test: Currently writes `"TestUser\n3\n"` to stdin, checks for `"What is your name?"`, `"TestUser"`, `"Correct"` or `"Wrong"` → Update assertions to match spec output strings (`"Hello, {name}!"`, etc.)
  - Update the test helper functions if needed (e.g., if entry function signature changes affect how tests compile programs)
  - Update any `compile_to_module` unit tests in `src/compiler.rs` that use brace syntax — they should still compile but may need source string updates

  **Must NOT do**:
  - Do not add new test projects
  - Do not change the test infrastructure (feature gating, helper functions)
  - Do not skip any tests

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 4 (with Tasks 14, 15, 16, 17)
  - **Blocks**: Task 14
  - **Blocked By**: Task 2

  **References**:

  **Pattern References**:
  - `tests/integration_e2e.rs` — All integration tests. Read the full file to understand current assertions and update them.
  - `src/compiler.rs:198-250` — Unit tests for `compile_to_module` that use inline source strings. These may use brace syntax and need updating.

  **API/Type References**:
  - `language-spec/hello_world.op:17` — Output: `print('Hello {world}')` → prints "Hello world"
  - `language-spec/fib_recursive.op:19` — Output: `print('fib({n}) = {result}')` where n=10, result=55 → prints "fib(10) = 55"
  - `language-spec/fib_iterative.op:31` — Output: `print('fib({n}) = {result}')` where n=10, result=55 → prints "fib(10) = 55"
  - `language-spec/simple_quiz.op:15,20,52,62,66,68` — Multiple output strings for the quiz program

  **WHY Each Reference Matters**:
  - `integration_e2e.rs`: The file being modified — every assertion must be updated to match new output
  - Language spec files: Source of truth for what each program prints

  **Acceptance Criteria**:
  - [ ] `fib_recursive` test asserts `"fib(10) = 55"` (not just `"55"`)
  - [ ] `fib_iterative` test asserts `"fib(10) = 55"`
  - [ ] `simple_quiz` test assertions match spec output strings
  - [ ] All test assertion strings match the spec file output exactly

  **QA Scenarios (MANDATORY):**

  ```
  Scenario: Test assertions match spec output
    Tool: Bash
    Steps:
      1. Run `grep -c 'fib(10) = 55' tests/integration_e2e.rs` — assert ≥ 2 (one for fib_recursive, one for fib_iterative)
      2. Run `grep -c 'Hello world' tests/integration_e2e.rs` — assert ≥ 1 (hello_world test)
      3. Run `grep -c 'What is your name' tests/integration_e2e.rs` — assert ≥ 1 (simple_quiz test)
      4. Save results: `grep -n 'fib(10)\|Hello world\|What is your name' tests/integration_e2e.rs > .sisyphus/evidence/task-13-assertion-check.txt`
    Expected Result: All spec-matching assertion strings are present in the test file
    Evidence: .sisyphus/evidence/task-13-assertion-check.txt

  Scenario: No hardcoded old assertion patterns remain
    Tool: Bash
    Steps:
      1. Run `grep -n 'contains("55")' tests/integration_e2e.rs` — assert 0 matches (old bare "55" assertion should be replaced with "fib(10) = 55")
      2. Run `grep -c 'fib(10) = 55' tests/integration_e2e.rs` — assert ≥ 2 (one for fib_recursive, one for fib_iterative)
    Expected Result: Old `contains("55")` patterns replaced with `contains("fib(10) = 55")` or similar full-string assertions
    Evidence: .sisyphus/evidence/task-13-no-old-assertions.txt
  ```

  **Commit**: YES (groups with T14)
  - Message: `fix(tests): update integration test assertions and verify all spec files compile`
  - Files: `tests/integration_e2e.rs`
  - Pre-commit: `cargo test --features integration` (may fail until T14 completes)

---

- [x] 14. TDD Iteration — Compile All 4 Spec Files End-to-End

  **What to do**:
  - This is the integration task. After all parser/codegen features are implemented (Tasks 3-13), run the full test suite and fix any remaining issues.
  - Run `cargo test` — fix any unit test failures
  - Run `cargo test --features integration` — fix any integration test failures
  - For each test project, verify the compilation pipeline works end-to-end:
    1. `cargo run -- test-projects/hello-world/src/main.op` → should print "Hello world"
    2. `cargo run -- test-projects/fib-recursive/src/main.op` → should print "fib(10) = 55"
    3. `cargo run -- test-projects/fib-iterative/src/main.op` → should print "fib(10) = 55"
    4. `cargo run -- test-projects/simple-quiz/src/main.op` → should run interactive quiz
  - Debug and fix any remaining issues in:
    - Lexer (indent/dedent edge cases)
    - Parser (colon-block edge cases, guard syntax)
    - Type checker (int32 type mismatches)
    - Codegen (LLVM IR errors, runtime function calls)
    - Linker (embedded runtime issues)
  - This task is inherently iterative — expect multiple fix-test cycles
  - **CRITICAL**: The simple_quiz program has complex features (loop with multi-value break, guard, continue, string interpolation, random numbers, user input). Expect this to be the hardest to get working.

  **Must NOT do**:
  - Do not modify the spec files to work around compiler issues — fix the compiler instead
  - Do not skip any test project
  - Do not leave `todo!()` or `unimplemented!()` in production code

  **Recommended Agent Profile**:
  - **Category**: `deep`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: NO
  - **Parallel Group**: Wave 4 (sequential — depends on all previous tasks)
  - **Blocks**: F1-F4
  - **Blocked By**: Tasks 3, 7, 8, 9, 10, 11, 12, 13

  **References**:

  **Pattern References**:
  - ALL files modified in Tasks 3-13 — this task integrates everything
  - `tests/integration_e2e.rs` — The integration test suite to run
  - `src/compiler.rs` — The `compile_program()` entry point

  **WHY Each Reference Matters**:
  - This is a debugging/integration task — the agent needs access to all compiler files to fix issues

  **Acceptance Criteria**:
  - [ ] `cargo test` → ALL pass (0 failures)
  - [ ] `cargo test --features integration` → ALL pass (4/4 test projects)
  - [ ] `cargo run -- test-projects/hello-world/src/main.op` prints "Hello world"
  - [ ] `cargo run -- test-projects/fib-recursive/src/main.op` prints "fib(10) = 55"
  - [ ] `cargo run -- test-projects/fib-iterative/src/main.op` prints "fib(10) = 55"
  - [ ] `cargo run -- test-projects/simple-quiz/src/main.op` runs without crashing
  - [ ] No `todo!()` or `unimplemented!()` in non-test code

  **QA Scenarios (MANDATORY):**

  ```
  Scenario: All 4 programs compile and produce correct output
    Tool: Bash
    Preconditions: All previous tasks completed
    Steps:
      1. Run `cargo build`
      2. Run `./target/debug/opalescent test-projects/hello-world/src/main.op` — assert stdout contains "Hello world"
      3. Run `./target/debug/opalescent test-projects/fib-recursive/src/main.op` — assert stdout contains "fib(10) = 55"
      4. Run `./target/debug/opalescent test-projects/fib-iterative/src/main.op` — assert stdout contains "fib(10) = 55"
      5. Run `echo "TestUser\n3\n" | ./target/debug/opalescent test-projects/simple-quiz/src/main.op` — assert stdout contains "What is your name?"
    Expected Result: All 4 programs compile and run correctly
    Failure Indicators: Compilation errors, wrong output, crashes
    Evidence: .sisyphus/evidence/task-14-all-programs.txt

  Scenario: Compiler works from different working directory
    Tool: Bash
    Steps:
      1. Run `REPO="$(pwd)" && OPAL="$REPO/target/debug/opalescent" && PROJ="$REPO/test-projects/hello-world/src/main.op" && cd /tmp && "$OPAL" "$PROJ" 2>&1 | tee "$REPO/.sisyphus/evidence/task-14-cross-dir.txt"`
      2. Assert exit code 0 and stdout contains "Hello world"
      3. Assert stderr does NOT contain "runtime" or "No such file"
    Expected Result: Compiler works from /tmp — no "runtime not found" errors, output is "Hello world"
    Failure Indicators: Error containing "runtime/opal_runtime.c" or "No such file or directory"
    Evidence: .sisyphus/evidence/task-14-cross-dir.txt

  Scenario: Full test suite passes
    Tool: Bash
    Steps:
      1. Run `cargo test`
      2. Run `cargo test --features integration`
      3. Assert 0 failures in both
    Expected Result: All unit and integration tests pass
    Evidence: .sisyphus/evidence/task-14-full-test-suite.txt
  ```

  **Commit**: YES (groups with T13)
  - Message: `fix(tests): update integration test assertions and verify all spec files compile`
  - Files: any files that needed fixes during TDD iteration
  - Pre-commit: `cargo test && cargo test --features integration`

---

- [x] 15. Align VS Code Extension with Language-Spec

  **What to do**:
  - Update `vscode-extension/package.json`:
    - Add `.types.op` to the file extensions list (alongside `.op`)
  - Update `vscode-extension/syntaxes/opalescent.tmLanguage.json`:
    - **Remove** `fn` from keyword highlighting — spec uses `f` for function expressions
    - **Remove** `unit` keyword — spec uses `void`
    - **Add** `continue` to control flow keywords
    - **Add** `from` to keyword list (used in `import...from`)
    - **Add** `into` to keyword list (used in `guard...into...else`)
    - **Add** `void` to built-in types if not already present
    - **Add** `guard` to keyword list
    - **Add** `propagate` to keyword list
    - **Add** `loop` to control flow keywords
    - **Add** `import` to keyword list
    - Verify `is` is in the keyword list
    - Verify `int32` is in the type list
  - Review `vscode-extension/language-configuration.json`:
    - Add colon as a potential indent trigger (after `if`, `while`, `for`, `else`, `loop`)
    - No other changes expected

  **Must NOT do**:
  - Do not modify LSP server code
  - Do not change extension activation events (beyond adding `.types.op`)
  - Do not change the extension's package name or publisher

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 4 (with Tasks 13, 14, 16, 17)
  - **Blocks**: F1-F4
  - **Blocked By**: Task 1

  **References**:

  **Pattern References**:
  - `vscode-extension/package.json` — Extension manifest. Search for `.op` to find file association and add `.types.op`.
  - `vscode-extension/syntaxes/opalescent.tmLanguage.json` — TextMate grammar. Find keyword patterns and update.
  - `vscode-extension/language-configuration.json` — Language config for brackets, comments, auto-indent.

  **API/Type References**:
  - Language spec files — Source of truth for which keywords exist in the language

  **WHY Each Reference Matters**:
  - `package.json`: File association — `.types.op` files need syntax highlighting
  - `tmLanguage.json`: Keyword highlighting — must match actual language keywords
  - `language-configuration.json`: Editor behavior for new syntax (colon-blocks)

  **Acceptance Criteria**:
  - [ ] `fn` is NOT in the keyword list
  - [ ] `unit` is NOT in the keyword list
  - [ ] `f`, `void`, `continue`, `from`, `into`, `guard`, `propagate`, `loop`, `import` ARE in the keyword list
  - [ ] `.types.op` files get syntax highlighting
  - [ ] JSON files are valid (no syntax errors)

  **QA Scenarios (MANDATORY):**

  ```
  Scenario: Extension JSON files are valid
    Tool: Bash
    Steps:
      1. Run `python3 -m json.tool vscode-extension/package.json > /dev/null` (or `jq . < file > /dev/null`)
      2. Run `python3 -m json.tool vscode-extension/syntaxes/opalescent.tmLanguage.json > /dev/null`
      3. Run `python3 -m json.tool vscode-extension/language-configuration.json > /dev/null`
      4. Assert all exit code 0
    Expected Result: All JSON files parse without errors
    Evidence: .sisyphus/evidence/task-15-json-valid.txt

  Scenario: Removed keywords are gone, added keywords are present
    Tool: Bash
    Steps:
      1. Run `grep -cE '\\bfn\\b' vscode-extension/syntaxes/opalescent.tmLanguage.json` — assert 0 (fn removed from keyword regex patterns)
      2. Run `grep -cE '\\bunit\\b' vscode-extension/syntaxes/opalescent.tmLanguage.json` — assert 0 (unit removed)
      3. Run `grep -cE '\\bguard\\b' vscode-extension/syntaxes/opalescent.tmLanguage.json` — assert ≥ 1 (guard added)
      4. Run `grep -cE '\\bfrom\\b' vscode-extension/syntaxes/opalescent.tmLanguage.json` — assert ≥ 1 (from added)
      5. Run `grep -cE '\\binto\\b' vscode-extension/syntaxes/opalescent.tmLanguage.json` — assert ≥ 1 (into added)
    Expected Result: Old keywords removed from regex patterns, new keywords added
    Evidence: .sisyphus/evidence/task-15-keywords.txt
  ```

  **Commit**: YES
  - Message: `fix(vscode): align extension keywords with language-spec`
  - Files: `vscode-extension/package.json`, `vscode-extension/syntaxes/opalescent.tmLanguage.json`, `vscode-extension/language-configuration.json`
  - Pre-commit: JSON validation

---

- [x] 16. Update README.md

  **What to do**:
  - Update the "Test Project Conventions" section:
    - Remove references to brace syntax requirement (`{ }`)
    - Remove `int64` type requirement — change to `int32`
    - Remove `f(): void` entry function — change to `f(args: string[]): void`
    - Add note about colon-block syntax for control flow
  - Update the "Compiler Pipeline" section:
    - Add note that C runtime is embedded in the binary
    - Remove any mention of `runtime/` folder dependency
    - Update the escape hatches table if needed
  - Update code examples throughout to use spec syntax:
    - Entry function examples should use `f(args: string[]): void`
    - If-statement examples should use colon-blocks
  - Update the "IDE Integration" section:
    - Add `.types.op` file extension mention
    - Update keyword list to match actual language keywords
  - Do NOT remove any sections — only update content

  **Must NOT do**:
  - Do not rewrite the entire README
  - Do not add new sections
  - Do not remove information about features that still exist

  **Recommended Agent Profile**:
  - **Category**: `writing`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 4 (with Tasks 13, 14, 15, 17)
  - **Blocks**: F1-F4
  - **Blocked By**: Tasks 3, 14

  **References**:

  **Pattern References**:
  - `README.md` — The file to update. Read it fully first.
  - `language-spec/` files — Source of truth for correct syntax examples

  **WHY Each Reference Matters**:
  - README.md: The file being modified
  - Language spec: Correct syntax to use in examples

  **Acceptance Criteria**:
  - [ ] No references to brace-only syntax in test project conventions
  - [ ] Entry function examples use `f(args: string[]): void`
  - [ ] Runtime embedding mentioned (no runtime folder needed)
  - [ ] `int32` used as primary integer type in examples

  **QA Scenarios (MANDATORY):**

  ```
  Scenario: README doesn't contain outdated information
    Tool: Bash
    Steps:
      1. Run `grep -cn 'runtime/opal_runtime' README.md` — assert output is "0" (embedded now, no file path references)
      2. Run `grep -cn 'brace syntax.*required\|Use.*brace syntax\|braces.*{ }.*for block' README.md` — assert output is "0" (colon-block syntax is the standard now)
      3. Run `grep -cn 'f(): void' README.md` — assert output is "0" (entry signature updated to `f(args: string[]): void`)
      4. Run `grep -c 'f(args: string\[\]): void' README.md` — assert output is >= 1 (new entry signature is documented)
      5. Run `grep -c 'int32' README.md` — assert output is >= 1 (int32 is the primary integer type now)
    Expected Result: All 5 assertions pass — no stale references, new conventions documented
    Failure Indicators: Any grep count doesn't match expected (stale content remains or new content missing)
    Evidence: .sisyphus/evidence/task-16-readme-check.txt
  ```

  **Commit**: YES (groups with T17)
  - Message: `docs: update README.md and PLAN.md for spec alignment`
  - Files: `README.md`
  - Pre-commit: none

---

- [x] 17. Update PLAN.md

  **What to do**:
  - Read `PLAN.md` and update any sections that reference:
    - Brace-only syntax for test projects
    - `int64` as the only integer type
    - Runtime folder dependency
    - Old test project output assertions
  - Add a section or note about the spec alignment changes made
  - Update any roadmap/status sections to reflect completed work

  **Must NOT do**:
  - Do not rewrite the entire PLAN.md
  - Do not remove historical information
  - Do not add speculative future plans

  **Recommended Agent Profile**:
  - **Category**: `writing`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 4 (with Tasks 13, 14, 15, 16)
  - **Blocks**: F1-F4
  - **Blocked By**: Tasks 3, 14

  **References**:

  **Pattern References**:
  - `PLAN.md` — The file to update. Read it fully first.

  **Acceptance Criteria**:
  - [ ] PLAN.md reflects current compiler capabilities
  - [ ] No stale references to brace-only syntax for spec files

  **QA Scenarios (MANDATORY):**

  ```
  Scenario: PLAN.md has no stale references
    Tool: Bash
    Steps:
      1. Run `grep -c 'runtime/opal_runtime' PLAN.md` — assert 0 (no hardcoded runtime path references)
      2. Run `grep -ci 'brace.only\|brace syntax.*required\|must use.*brace' PLAN.md` — assert 0 (no brace-only mandates)
      3. Save results: `grep -n 'runtime/opal_runtime\|brace.only\|brace syntax.*required' PLAN.md > .sisyphus/evidence/task-17-plan-check.txt 2>&1; echo "Exit: $?" >> .sisyphus/evidence/task-17-plan-check.txt`
    Expected Result: Zero matches for stale references — all content reflects current implementation
    Evidence: .sisyphus/evidence/task-17-plan-check.txt
  ```

  **Commit**: YES (groups with T16)
  - Message: `docs: update README.md and PLAN.md for spec alignment`
  - Files: `PLAN.md`
  - Pre-commit: none

---

## Final Verification Wave (MANDATORY — after ALL implementation tasks)

> 4 review agents run in PARALLEL. ALL must APPROVE. Present consolidated results to user and get explicit "okay" before completing.
>
> **Do NOT auto-proceed after verification. Wait for user's explicit approval before marking work complete.**

- [x] F1. **Plan Compliance Audit** — `oracle`
  Read the plan end-to-end. For each "Must Have": verify implementation exists (read file, run command). For each "Must NOT Have": search codebase for forbidden patterns — reject with file:line if found. Check evidence files exist in `.sisyphus/evidence/`. Compare deliverables against plan.
  Output: `Must Have [N/N] | Must NOT Have [N/N] | Tasks [N/N] | VERDICT: APPROVE/REJECT`

- [x] F2. **Code Quality Review** — `unspecified-high`
  Run `cargo test`, `cargo test --features integration`, `cargo clippy`. Review all changed files for: `as any`/`unwrap()` in non-test code, empty match arms, `todo!()` macros, commented-out code, unused imports. Check for AI slop: excessive comments, over-abstraction, generic names.
  Output: `Build [PASS/FAIL] | Tests [N pass/N fail] | Clippy [PASS/FAIL] | Files [N clean/N issues] | VERDICT`

- [x] F3. **Real Manual QA** — `unspecified-high`
  Start from clean state. Compile each test project individually: `cargo run -- test-projects/{name}/src/main.op`. Verify output matches expected. Test that compiler works WITHOUT `runtime/` folder (rename it temporarily). Test from a different working directory. Save evidence to `.sisyphus/evidence/final-qa/`.
  Output: `Programs [4/4 compile] | Output [4/4 correct] | No-runtime [PASS/FAIL] | Cross-dir [PASS/FAIL] | VERDICT`

- [x] F4. **Scope Fidelity Check** — `deep`
  For each task: read "What to do", read actual diff (git log/diff). Verify 1:1 — everything in spec was built (no missing), nothing beyond spec was built (no creep). Check "Must NOT do" compliance. Detect cross-task contamination. Flag unaccounted changes.
  Output: `Tasks [N/N compliant] | Contamination [CLEAN/N issues] | Unaccounted [CLEAN/N files] | VERDICT`

---

## Commit Strategy

| Group | Message | Files | Pre-commit |
|-------|---------|-------|------------|
| T1 | `chore: commit current state before spec alignment` | all staged | — |
| T2 | `chore(test-projects): replace .op files with exact language-spec copies` | `test-projects/*/src/main.op` | — |
| T3 | `feat(compiler): embed C runtime in binary via include_str!` | `src/compiler.rs` | `cargo test` |
| T4+T6 | `feat(types): add int32 type support and is operator` | `src/token.rs`, `src/lexer.rs`, `src/parser/*`, `src/type_system/*`, `src/codegen/*` | `cargo test` |
| T5+T7+T8 | `feat(parser): add colon-block indentation parsing with indent/dedent tokens` | `src/lexer.rs`, `src/token.rs`, `src/parser/statements.rs`, `src/parser/helpers.rs` | `cargo test` |
| T9 | `feat(codegen): entry function accepts args: string[] parameter` | `src/parser/*`, `src/codegen/*`, `src/type_system/*` | `cargo test` |
| T10 | `feat(parser): add import...from syntax` | `src/lexer.rs`, `src/parser/*`, `src/ast.rs` | `cargo test` |
| T11+T12 | `feat(parser): add loop/break/continue and guard...into...else` | `src/parser/*`, `src/codegen/*`, `src/ast.rs` | `cargo test` |
| T13+T14 | `fix(tests): update integration test assertions and verify all spec files compile` | `tests/integration_e2e.rs` | `cargo test --features integration` |
| T15 | `fix(vscode): align extension keywords with language-spec` | `vscode-extension/*` | — |
| T16+T17 | `docs: update README.md and PLAN.md for spec alignment` | `README.md`, `PLAN.md` | — |

---

## Success Criteria

### Verification Commands
```bash
# All unit tests pass
cargo test  # Expected: all tests pass

# All integration tests pass (compiles + runs all 4 test projects)
cargo test --features integration  # Expected: 4 tests pass

# Test files are exact copies of language-spec
diff test-projects/hello-world/src/main.op language-spec/hello_world.op  # Expected: no output
diff test-projects/fib-recursive/src/main.op language-spec/fib_recursive.op  # Expected: no output
diff test-projects/fib-iterative/src/main.op language-spec/fib_iterative.op  # Expected: no output
diff test-projects/simple-quiz/src/main.op language-spec/simple_quiz.op  # Expected: no output

# Compiler works without runtime folder
mv runtime runtime_backup && cargo run -- test-projects/hello-world/src/main.op && mv runtime_backup runtime  # Expected: prints "Hello world"

# No clippy warnings
cargo clippy -- -D warnings  # Expected: no warnings
```

### Final Checklist
- [ ] All "Must Have" features implemented and working
- [ ] All "Must NOT Have" guardrails respected
- [ ] All unit tests pass (`cargo test`)
- [ ] All integration tests pass (`cargo test --features integration`)
- [ ] Test project files are byte-for-byte identical to language-spec originals
- [ ] Compiler works without `runtime/` folder
- [ ] VS Code extension keywords match language-spec
- [ ] README.md and PLAN.md reflect current state
