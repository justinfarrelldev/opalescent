# Opalescent Compiler Improvements: Doc Comments, `opal run`, Multi-File Imports

## TL;DR

> **Quick Summary**: Add three compiler features: (1) doc comment validation that rejects public/entry functions without ≥30-char doc comments, (2) `opal run` (no args) shortcut that auto-compiles `src/main.op`, and (3) multi-file project compilation with working local imports between `.op` files.
> 
> **Deliverables**:
> - Doc comment validation in type checker with new error types
> - `opal run` (no args) project-aware compilation shortcut
> - Module loader for file-based import resolution
> - Multi-object linker support
> - `compile_project()` orchestrator for multi-file compilation
> - Local import codegen (beyond stdlib-only)
> - `@scope/package` import "not yet supported" error
> - Entry-in-main-only validation for multi-file projects
> - New integration tests for all features
> - Multi-file test project demonstrating imports
> 
> **Estimated Effort**: Large
> **Parallel Execution**: YES — 4 waves
> **Critical Path**: Task 1 → Task 6 → Task 8 → Task 9

---

## Context

### Original Request
The user requested three improvements to the Opalescent compiler:
1. Make `opal run` (no args) automatically find and compile `src/main.op`
2. Make multi-file projects work with real imports between `.op` files
3. Make the `no-doc-comments` test project fail to compile (doc comment validation is completely missing)

### Interview Summary
**Key Discussions**:
- `opal run` (no args) should look for `src/main.op` in CWD — no recursive search needed
- `entry` keyword must ONLY appear in `src/main.op`; other files get a new compile error
- Multi-file: use per-module compile + link approach (not single-LLVM-module merge)
- `@scope/package` imports: parser supports syntax but should emit "not yet supported" at compile time
- Doc comments: ≥30 characters required on all public and entry functions (entry implies public per spec)

**Research Findings**:
- Import system partially implemented: parser done, ModuleResolver works with pre-registered interfaces only, codegen only handles stdlib. No file I/O module loader exists.
- Doc comment `Documentation` struct exists in AST with `raw`, `sections`, `attributes`, `span` fields — parsed and stored but NEVER validated by type checker
- `no-doc-comments/src/main.op` has `entry main` at line 9 WITHOUT doc comments. Comment at top says "This file should fail to compile"
- `should-print-final-result/src/main.op` has entry at line 8 without doc comments — needs fixing to not break once validation is added
- All other test projects with integration tests already have valid doc comments on their entry functions
- `compile_program()` takes a single source string — no multi-file orchestration exists
- `link_object_file()` handles only a single `.o` file
- Linker uses `-no-pie` flag on Linux x86_64

### Metis Review
**Identified Gaps** (all addressed):
- `opal run` (no args) + multi-file conflict: resolved by making `opal run` project-aware via `compile_project()`
- Doc comment audit risk: audited all test projects — only `should-print-final-result` needs a doc comment added to its entry function
- Package import handling: will emit "not yet supported" error
- Entry validation across files: new `EntryNotInMainModule` error type added
- Missing integration tests for fail-to-compile projects: added to plan

---

## Work Objectives

### Core Objective
Add doc comment validation, `opal run` shortcut, and multi-file project compilation to the Opalescent compiler, making the language spec's module and documentation requirements enforceable.

### Concrete Deliverables
- `src/type_system/errors.rs` — 5 new `TypeError` variants
- `src/type_system/checker/declarations.rs` — doc comment validation logic
- `src/app.rs` — `opal run` (no args) handling
- `src/module_loader.rs` — new file for import path resolution and dependency graph
- `src/compiler.rs` — `compile_project()` function, multi-object linker
- `src/codegen/functions.rs` — local import codegen, `@scope/package` error
- `test-projects/multi-file/` — new test project demonstrating imports
- `test-projects/should-print-final-result/src/main.op` — doc comment fix
- `tests/integration_e2e.rs` — new integration tests

### Definition of Done
- [ ] `cargo test --features integration` passes with all existing + new tests
- [ ] `cargo build --release` succeeds without warnings
- [ ] `no-doc-comments` test project fails to compile with `MissingDocComment` error
- [ ] `opal run` (no args) in a project directory compiles and runs `src/main.op`
- [ ] Multi-file test project compiles and runs correctly with cross-file imports
- [ ] `@scope/package` imports produce "not yet supported" compile error
- [ ] `entry` in non-`src/main.op` files produces `EntryNotInMainModule` compile error

### Must Have
- Doc comment validation: `MissingDocComment` error when public/entry function lacks `##` block
- Doc comment validation: `DocCommentTooShort` error when doc comment `raw` field (trimmed) is < 30 characters
- `opal run` (no args): looks for `src/main.op` in CWD, error if not found
- Module loader: resolves `./path` imports to `.op` files on disk
- Module loader: handles `.types.op` extension for type imports
- Module loader: detects circular imports (integrate with existing `validate_no_cycles_from`)
- `compile_project()`: compiles each discovered file to its own `.o`, links all together
- Entry-in-main-only: `entry` keyword only valid in `src/main.op` during project compilation
- Per-module compile + link (NOT single LLVM module merge)

### Must NOT Have (Guardrails)
- NO package manager wiring or `@scope/package` import resolution — only "not yet supported" error
- NO recursive file search for entry point — only check `src/main.op`
- NO changes to the parser — import syntax is already fully implemented
- NO changes to the lexer or tokenizer
- NO hot reload changes
- NO formatter changes (fmt-test inputs without doc comments are fine — they're not compiled)
- NO over-abstraction of the module loader — keep it simple, file-based, no plugin system
- NO changes to the `Documentation` struct or doc comment parsing — only add validation
- NO `opal.toml` changes — project config format stays the same

---

## Verification Strategy

> **ZERO HUMAN INTERVENTION** — ALL verification is agent-executed. No exceptions.

### Test Decision
- **Infrastructure exists**: YES — Cargo test framework with integration feature flag
- **Automated tests**: YES (tests-after) — integration tests verify end-to-end behavior
- **Framework**: `cargo test` (Rust built-in) + `cargo test --features integration` for E2E
- **Approach**: Each feature verified by integration tests that compile test projects and check outcomes

### QA Policy
Every task includes agent-executed QA scenarios.
Evidence saved to `.sisyphus/evidence/task-{N}-{scenario-slug}.{ext}`.

- **Compiler features**: Use Bash — `cargo test`, `cargo build`, run compiled binaries
- **CLI features**: Use Bash — invoke `opal run` and check stdout/stderr/exit codes
- **Fail-to-compile features**: Use Bash — attempt compilation and verify error output

---

## Execution Strategy

### Parallel Execution Waves

```
Wave 1 (Foundation — 4 parallel tasks):
├── Task 1: Add new TypeError variants to errors.rs [quick]
├── Task 2: `opal run` (no args) shortcut in app.rs [quick]
├── Task 3: Module loader (new src/module_loader.rs) [deep]
└── Task 4: Multi-object linker support in compiler.rs [quick]

Wave 2 (Core features — 3 parallel tasks):
├── Task 5: Doc comment validation + fix test projects (depends: 1) [unspecified-high]
├── Task 6: compile_project() orchestrator + entry validation (depends: 1, 3, 4) [deep]
└── Task 7: Local import codegen + @scope/package error (depends: 1, 3) [unspecified-high]

Wave 3 (Integration — 2 parallel tasks):
├── Task 8: Wire `opal run` to compile_project() (depends: 2, 6) [quick]
└── Task 9: Multi-file test project + integration tests (depends: 5, 6, 7) [unspecified-high]

Wave FINAL (After ALL tasks — 4 parallel reviews, then user okay):
├── Task F1: Plan compliance audit (oracle)
├── Task F2: Code quality review (unspecified-high)
├── Task F3: Real manual QA (unspecified-high)
└── Task F4: Scope fidelity check (deep)
→ Present results → Get explicit user okay

Critical Path: Task 1 → Task 6 → Task 8 → Task 9 → F1-F4 → user okay
Parallel Speedup: ~60% faster than sequential
Max Concurrent: 4 (Wave 1)
```

### Dependency Matrix

| Task | Depends On | Blocks | Wave |
|------|-----------|--------|------|
| 1 | — | 5, 6, 7 | 1 |
| 2 | — | 8 | 1 |
| 3 | — | 6, 7 | 1 |
| 4 | — | 6 | 1 |
| 5 | 1 | 9 | 2 |
| 6 | 1, 3, 4 | 8, 9 | 2 |
| 7 | 1, 3 | 9 | 2 |
| 8 | 2, 6 | 9 | 3 |
| 9 | 5, 6, 7, 8 | F1-F4 | 3 |

### Agent Dispatch Summary

- **Wave 1**: **4 tasks** — T1 → `quick`, T2 → `quick`, T3 → `deep`, T4 → `quick`
- **Wave 2**: **3 tasks** — T5 → `unspecified-high`, T6 → `deep`, T7 → `unspecified-high`
- **Wave 3**: **2 tasks** — T8 → `quick`, T9 → `unspecified-high`
- **FINAL**: **4 tasks** — F1 → `oracle`, F2 → `unspecified-high`, F3 → `unspecified-high`, F4 → `deep`

---

## TODOs

- [x] 1. Add New TypeError Variants

  **What to do**:
  - Add 5 new `TypeError` variants to `src/type_system/errors.rs`, following the existing pattern (derive `Error`, `Debug`, `Clone`, `PartialEq`, `Eq`, `Diagnostic` via thiserror + miette):
    1. `MissingDocComment` — public/entry function has no `##` doc comment block
       - Fields: `name: String` (function name), `span: SourceSpan`
       - Error message: `"Public function '{name}' is missing a documentation comment"`
       - Diagnostic code: `opalescent::type_system::missing_doc_comment`
       - Help: `"Add a ## documentation block with at least 30 characters before this function"`
    2. `DocCommentTooShort` — doc comment exists but `raw` content (trimmed) is < 30 characters
       - Fields: `name: String`, `found_length: usize`, `min_length: usize`, `span: SourceSpan`
       - Error message: `"Documentation comment for '{name}' is too short ({found_length} characters, minimum {min_length})"`
       - Diagnostic code: `opalescent::type_system::doc_comment_too_short`
       - Help: `"Expand the documentation to at least {min_length} characters"`
    3. `EntryNotInMainModule` — `entry` keyword found in a file that is not `src/main.op`
       - Fields: `file_path: String`, `span: SourceSpan`
       - Error message: `"The 'entry' keyword is only allowed in src/main.op, found in '{file_path}'"`
       - Diagnostic code: `opalescent::type_system::entry_not_in_main_module`
       - Help: `"Move the entry function to src/main.op — only one entry point is allowed per project"`
    4. `ModuleNotFound` — import path doesn't resolve to a file on disk
       - Fields: `path: String`, `span: SourceSpan`
       - Error message: `"Module '{path}' not found"`
       - Diagnostic code: `opalescent::type_system::module_not_found`
       - Help: `"Check the import path — expected file at '{path}.op' or '{path}.types.op'"`
    5. `PackageImportNotSupported` — `@scope/name` import used
       - Fields: `path: String`, `span: SourceSpan`
       - Error message: `"Package imports are not yet supported: '{path}'"`
       - Diagnostic code: `opalescent::type_system::package_import_not_supported`
       - Help: `"Package imports (@scope/name) will be available once the package manager is implemented. Use local imports (./path) instead."`
  - Ensure each variant follows the exact pattern of existing variants (see `PurityViolation`, `UnresolvedImport` as templates)
  - Add `#[label("...")]` annotations on span fields matching existing conventions

  **Must NOT do**:
  - Do NOT modify any other files — only `src/type_system/errors.rs`
  - Do NOT add validation logic — only define the error types
  - Do NOT change existing error variants

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Single-file change adding enum variants following a clear existing pattern
  - **Skills**: `[]`
    - No special skills needed — straightforward Rust enum extension

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1 (with Tasks 2, 3, 4)
  - **Blocks**: Tasks 5, 6, 7
  - **Blocked By**: None (can start immediately)

  **References**:

  **Pattern References** (existing code to follow):
  - `src/type_system/errors.rs:673-689` — `PurityViolation` variant: follow this exact pattern for new variants (Error + Diagnostic derives, `#[error(...)]`, `#[diagnostic(code(...), help(...))]`, `#[label("...")]` on span)
  - `src/type_system/errors.rs:623-635` — `UnresolvedImport` variant: template for import-related errors
  - `src/type_system/errors.rs:108-134` — `MissingEntryPoint` and `DuplicateEntryPoint`: template for entry-related errors

  **API/Type References**:
  - `src/type_system/errors.rs:10` — `TypeError` enum definition: add new variants here
  - `miette::SourceSpan` — used for all span fields
  - `alloc::string::String` — used for all string fields

  **Acceptance Criteria**:

  - [ ] `cargo build` succeeds with the new variants
  - [ ] `cargo clippy -- -D warnings` produces no warnings on errors.rs
  - [ ] All 5 new variants are present in the `TypeError` enum
  - [ ] Each variant has `#[error(...)]`, `#[diagnostic(code(...), help(...))]`, and `#[label(...)]` annotations

  **QA Scenarios (MANDATORY):**

  ```
  Scenario: Build succeeds with new error variants
    Tool: Bash
    Preconditions: Clean working directory
    Steps:
      1. Run `cargo build 2>&1`
      2. Check exit code is 0
      3. Grep output for "error" — should find none
    Expected Result: Exit code 0, no compilation errors
    Failure Indicators: Non-zero exit code, "error[E" in output
    Evidence: .sisyphus/evidence/task-1-build-success.txt

  Scenario: Clippy passes without warnings
    Tool: Bash
    Preconditions: Successful build
    Steps:
      1. Run `cargo clippy -- -D warnings 2>&1`
      2. Check exit code is 0
    Expected Result: Exit code 0, no clippy warnings
    Failure Indicators: Non-zero exit code, "warning:" in output
    Evidence: .sisyphus/evidence/task-1-clippy-clean.txt
  ```

  **Commit**: YES
  - Message: `feat(type-system): add error variants for doc comments, modules, and entry validation`
  - Files: `src/type_system/errors.rs`
  - Pre-commit: `cargo build`

- [x] 2. `opal run` (No Args) Shortcut — Single-File Initial Implementation

  **What to do**:
  - Modify `run_run_command()` in `src/app.rs` (around line 251) to handle the case where no source file argument is provided:
    1. Check `args.len()` — if no file argument after `run`:
       - Construct path: `CWD/src/main.op`
       - Check if file exists using `std::path::Path::exists()`
       - If exists: call existing `compile_and_run()` with that path
       - If not exists: print error message to stderr and exit with code 1:
         `"Error: No source file specified and no src/main.op found in the current directory.\nUsage: opal run <file.op>  or  run from a project directory with src/main.op"`
    2. Keep existing behavior for `opal run <file.op>` — this is only for the no-args case
  - NOTE: This initial implementation uses single-file compilation. Task 8 will upgrade it to project-aware compilation.

  **Must NOT do**:
  - Do NOT implement recursive file search — only check `src/main.op`
  - Do NOT modify `compile_and_run()` itself
  - Do NOT change any other CLI commands
  - Do NOT add `opal.toml` reading logic here (that's Task 6/8)

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Small modification to one function in one file, clear logic
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1 (with Tasks 1, 3, 4)
  - **Blocks**: Task 8
  - **Blocked By**: None (can start immediately)

  **References**:

  **Pattern References**:
  - `src/app.rs:251` — `run_run_command()`: the function to modify — currently expects `args[2]` as source file
  - `src/app.rs:206` — `compile_and_run()`: the function to call when `src/main.op` is found — takes a file path, reads source, calls `compile_program()`, executes binary
  - `src/app.rs:522` — `run_build_command()`: reference for how `opal build` finds `src/main.op` from CWD via `opal.toml` — useful pattern to follow

  **API/Type References**:
  - `std::env::current_dir()` — get CWD for constructing `src/main.op` path
  - `std::path::Path::exists()` — check if `src/main.op` exists

  **Acceptance Criteria**:

  - [ ] `cargo build` succeeds
  - [ ] When run from a directory with `src/main.op`, `opal run` (no args) compiles and runs it
  - [ ] When run from a directory without `src/main.op`, `opal run` (no args) prints error to stderr and exits with code 1
  - [ ] Existing `opal run <file.op>` behavior is unchanged

  **QA Scenarios (MANDATORY):**

  ```
  Scenario: opal run (no args) finds and runs src/main.op
    Tool: Bash
    Preconditions: `cargo build --release` succeeds. CWD is `test-projects/hello-world/`
    Steps:
      1. Run `cd test-projects/hello-world && ../../target/release/opalescent run 2>&1`
      2. Check stdout contains "Hello world"
      3. Check exit code is 0
    Expected Result: Binary compiles and runs, stdout contains "Hello world", exit code 0
    Failure Indicators: "Error:" in stderr, non-zero exit code, no "Hello world" in stdout
    Evidence: .sisyphus/evidence/task-2-run-no-args-success.txt

  Scenario: opal run (no args) errors when no src/main.op
    Tool: Bash
    Preconditions: `cargo build --release` succeeds. CWD is `/tmp/` (no src/main.op)
    Steps:
      1. Run `cd /tmp && /home/justi/Projects/opalescent/target/release/opalescent run 2>&1`
      2. Check output contains "No source file specified" or "src/main.op"
      3. Check exit code is non-zero
    Expected Result: Error message about missing src/main.op, non-zero exit
    Failure Indicators: Exit code 0, no error message, or crash/panic
    Evidence: .sisyphus/evidence/task-2-run-no-args-error.txt

  Scenario: opal run <file.op> still works normally
    Tool: Bash
    Preconditions: `cargo build --release` succeeds
    Steps:
      1. Run `./target/release/opalescent run test-projects/hello-world/src/main.op 2>&1`
      2. Check stdout contains "Hello world"
      3. Check exit code is 0
    Expected Result: Existing behavior unchanged, "Hello world" in stdout
    Failure Indicators: Error or different behavior than before
    Evidence: .sisyphus/evidence/task-2-run-with-file-unchanged.txt
  ```

  **Commit**: YES
  - Message: `feat(cli): add opal run shortcut for src/main.op`
  - Files: `src/app.rs`
  - Pre-commit: `cargo build`

- [x] 3. Module Loader — File-Based Import Resolution

  **What to do**:
  - Create new file `src/module_loader.rs` with the following components:
    1. **`ModuleLoader` struct** — orchestrates file discovery and dependency resolution
       - Fields: `project_root: PathBuf`, `discovered_modules: HashMap<PathBuf, ParsedModule>`
       - `ParsedModule` struct: `path: PathBuf`, `source: String`, `ast: Program`, `imports: Vec<ImportInfo>`
       - `ImportInfo` struct: `source_path: String`, `resolved_path: PathBuf`, `is_type_import: bool`, `span: Span`
    2. **`resolve_import_path(from_file: &Path, import_source: &str) -> Result<PathBuf, TypeError>`**
       - For `./path`: resolve relative to the importing file's directory
       - Append `.op` extension if not present
       - For `./path.types`: resolve to `.types.op` file
       - For stdlib names (`standard`, `math`): return a sentinel/marker path (these are handled by existing codegen)
       - For `@scope/name`: return `PackageImportNotSupported` error
       - If resolved file doesn't exist on disk: return `ModuleNotFound` error
    3. **`discover_all_modules(entry_path: &Path) -> Result<Vec<PathBuf>, TypeError>`**
       - Start from `entry_path` (src/main.op)
       - Parse the file to get its AST (reuse existing `parse()` from parser)
       - Extract all `Decl::Import` declarations
       - Resolve each import path
       - Recursively discover imports from each resolved file
       - Detect circular imports using the existing `ModuleResolver::validate_no_cycles_from` pattern
       - Return files in topological order (dependencies before dependents)
    4. **`get_module_source(path: &Path) -> Result<String, CompileError>`**
       - Read file from disk via `std::fs::read_to_string`
       - Cache results to avoid re-reading
  - Register the module in `src/lib.rs` with `pub mod module_loader;`
  - The module loader does NOT do type checking or codegen — it only discovers files, parses for import info, and resolves paths

  **Must NOT do**:
  - Do NOT integrate with type checker or codegen — that's Task 6
  - Do NOT handle stdlib imports specially beyond returning a sentinel path
  - Do NOT modify the parser — use it as-is
  - Do NOT add recursive directory scanning — only follow explicit import paths
  - Do NOT create a plugin or extension system

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: New module with graph algorithms (cycle detection, topological sort), file I/O, and AST interaction — requires careful design
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1 (with Tasks 1, 2, 4)
  - **Blocks**: Tasks 6, 7
  - **Blocked By**: None (can start immediately)

  **References**:

  **Pattern References**:
  - `src/type_system/module_resolver.rs` — `ModuleResolver` struct and `validate_no_cycles_from()`: existing cycle detection pattern to follow or integrate with. Note: ModuleResolver works with pre-registered `ModuleInterface` objects, NOT files. The module loader will FEED data into ModuleResolver.
  - `src/type_system/module_resolver.rs` — `ModuleInterface` struct: the interface type that must be constructed from parsed files (has `exports: BTreeMap<String, SymbolInfo>`, `private_symbols: BTreeMap<String, SymbolInfo>`, `module_path: String`). Use `register_symbol()` to add symbols.
  - `src/parser/declarations.rs` — `parse_import_declaration()`: the parser function that creates `Decl::Import` nodes — module loader will parse files and extract these

  **API/Type References**:
  - `src/ast.rs` — `Decl::Import { items, source, span }`: the AST node for imports — `source` is the import path string, `items` contains what's imported
  - `src/ast.rs:900-926` — `ImportItem` enum has exactly 3 variants: `Named { name, alias, span }`, `Glob { span }`, `Type { name, alias, span }` — module loader must handle all three. `Named` covers both plain and aliased imports (alias is `Option<String>`). `Type` covers both plain and aliased type imports. There is no separate `Aliased`, `TypeImport`, `TypeAliased`, or `TypeGlob` variant.
  - `src/ast.rs` — `Program` struct: the top-level AST type returned by the parser
  - `src/parser.rs` — `Parser::new(tokens: Vec<Token>)` then `.parse() -> (Option<Program>, ParseErrors)`: to parse a source file, first tokenize with `Lexer::new(source).tokenize() -> (Vec<Token>, LexErrors)`, then pass tokens to `Parser::new(tokens).parse()`. Note: `parse()` consumes `self`.
  - `src/compiler.rs` — `CompileError` enum: error type to use for file I/O failures
  - `src/compiler.rs:122-231` — `compile_to_module()`: reference implementation showing the full lex → parse → typecheck → codegen pipeline — module loader should follow the same lex+parse steps

  **External References**:
  - Standard Rust: `std::fs::read_to_string`, `std::path::Path`, `std::collections::HashMap`
  - Topological sort: Kahn's algorithm or DFS-based — implement inline, no external crate needed

  **Acceptance Criteria**:

  - [ ] `cargo build` succeeds with new module_loader.rs
  - [ ] Module is registered in `src/lib.rs`
  - [ ] `resolve_import_path` correctly handles: `./utils` → `{dir}/utils.op`, `./types.types` → `{dir}/types.types.op`, `standard` → sentinel, `@scope/pkg` → error
  - [ ] `discover_all_modules` finds all transitively imported files
  - [ ] Circular imports are detected and produce an error

  **QA Scenarios (MANDATORY):**

  ```
  Scenario: Module loader compiles and is accessible
    Tool: Bash
    Preconditions: Module loader code written
    Steps:
      1. Run `cargo build 2>&1`
      2. Check exit code is 0
      3. Grep for "module_loader" in src/lib.rs to verify registration
    Expected Result: Build succeeds, module registered
    Failure Indicators: Compilation errors, missing module registration
    Evidence: .sisyphus/evidence/task-3-build-success.txt

  Scenario: Import path resolution handles all cases
    Tool: Bash
    Preconditions: Write a focused unit test in module_loader.rs
    Steps:
      1. Add `#[cfg(test)] mod tests` in module_loader.rs with tests for:
         - `./utils` resolves to `{base}/utils.op`
         - `./models.types` resolves to `{base}/models.types.op`
         - `standard` returns sentinel path
         - `@scope/pkg` returns PackageImportNotSupported error
         - Non-existent `./missing` returns ModuleNotFound error
      2. Run `cargo test module_loader 2>&1`
      3. Check all tests pass
    Expected Result: All resolution tests pass
    Failure Indicators: Test failures, panics
    Evidence: .sisyphus/evidence/task-3-resolution-tests.txt
  ```

  **Commit**: YES
  - Message: `feat(compiler): add module loader for file-based import resolution`
  - Files: `src/module_loader.rs`, `src/lib.rs`
  - Pre-commit: `cargo test module_loader`

- [x] 4. Multi-Object Linker Support

  **What to do**:
  - Modify `link_object_file()` in `src/compiler.rs` to accept multiple object files:
    1. Change signature from `link_object_file(object_path: &Path, output_path: &Path)` to `link_object_files(object_paths: &[PathBuf], output_path: &Path)` — or add a new function alongside the existing one
    2. Update `build_linker_command()` similarly to accept multiple `.o` file paths
    3. Each `.o` file path gets added as an argument to the linker command
    4. Keep backward compatibility: the existing single-file `link_object_file()` can call the new multi-file version with a single-element slice
  - Ensure the `-no-pie` flag and all other linker flags are preserved
  - The embedded C runtime object file must still be included in the link

  **Must NOT do**:
  - Do NOT change the compilation pipeline — only the linker step
  - Do NOT modify how object files are generated (that's `emit_object_file`)
  - Do NOT change the output binary name convention (`program`)
  - Do NOT remove or break the existing single-file linking behavior

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Small modification to existing linker functions — change from single path to path slice
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1 (with Tasks 1, 2, 3)
  - **Blocks**: Task 6
  - **Blocked By**: None (can start immediately)

  **References**:

  **Pattern References**:
  - `src/compiler.rs` — `link_object_file()`: current implementation linking single `.o` to binary — extend to multiple
  - `src/compiler.rs` — `build_linker_command()`: constructs the `cc` command with flags — must add multiple `.o` file arguments
  - `src/compiler.rs` — embedded C runtime handling: the runtime `.o` file is extracted from `RUNTIME_OBJECT` and written to a temp file, then included in link. This must still work.

  **API/Type References**:
  - `src/compiler.rs` — `CompileError` enum: error type for linker failures
  - `std::process::Command` — used to invoke `cc` for linking

  **Acceptance Criteria**:

  - [ ] `cargo build` succeeds
  - [ ] Existing single-file linking still works (all existing integration tests pass)
  - [ ] New function/signature accepts `&[PathBuf]` for multiple object files
  - [ ] `cargo test --features integration` passes (existing tests use single-file linking)

  **QA Scenarios (MANDATORY):**

  ```
  Scenario: Existing single-file linking unchanged
    Tool: Bash
    Preconditions: Modified compiler.rs builds
    Steps:
      1. Run `cargo test --features integration smoke_void_program_compiles_links_and_runs 2>&1`
      2. Run `cargo test --features integration hello_world_compiles_links_and_runs 2>&1`
      3. Check both pass
    Expected Result: Both tests pass — backward compatibility maintained
    Failure Indicators: Test failures, linker errors
    Evidence: .sisyphus/evidence/task-4-backward-compat.txt

  Scenario: Multi-object link function signature exists
    Tool: Bash
    Preconditions: Build succeeds
    Steps:
      1. Grep src/compiler.rs for the new function accepting `&[PathBuf]` or `&[&Path]`
      2. Verify it includes all existing linker flags including `-no-pie`
    Expected Result: New function found with correct signature
    Failure Indicators: Function missing, flags missing
    Evidence: .sisyphus/evidence/task-4-multi-object-signature.txt
  ```

  **Commit**: YES
  - Message: `feat(compiler): extend linker for multiple object files`
  - Files: `src/compiler.rs`
  - Pre-commit: `cargo test --features integration`

- [x] 5. Doc Comment Validation in Type Checker + Fix Affected Test Projects

  **What to do**:
  - **Part A: Implement validation in type checker** — modify `src/type_system/checker/declarations.rs`:
    1. When type-checking `Decl::Function` declarations:
       - If `is_public == true` OR `is_entry == true`:
         - If `doc_comment` is `None` → emit `TypeError::MissingDocComment { name, span }`
         - If `doc_comment` is `Some(doc)` and `doc.raw.trim().len() < 30` → emit `TypeError::DocCommentTooShort { name, found_length: doc.raw.trim().len(), min_length: 30, span }`
    2. When type-checking `Decl::Let` declarations that are public:
       - Same validation as above — public `let` bindings (function values) also need doc comments
    3. The `span` used in errors should be the span of the function/let declaration itself (not the doc comment span)
    4. Validation should happen AFTER the function is registered in scope (so existing type checking still works) but BEFORE moving to the next declaration
  - **Part B: Fix `should-print-final-result` test project** — edit `test-projects/should-print-final-result/src/main.op`:
    - Add a `##` doc comment block before the `entry main` on line 8:
      ```
      ##
          Description: Entry point for print result test
      ##
      ```
    - This ensures the test project continues to compile after validation is added
  - **Part C: Verify `no-doc-comments` now fails** — the `no-doc-comments` test project should now fail to compile because `entry main` at line 9 has no doc comment. Do NOT modify this file.
  - **Part D: Add integration tests** — add to `tests/integration_e2e.rs`:
    1. `no_doc_comments_fails_to_compile` test: read `test-projects/no-doc-comments/src/main.op`, call `compile_program()`, assert it returns `Err` (similar to `immutability_compile_error` test pattern)
    2. `multiple_entry_fails_to_compile` test: read `test-projects/multiple-entry/src/main.op`, call `compile_program()`, assert it returns `Err` (this likely already fails due to `DuplicateEntryPoint`, but having the test is valuable)

  **Must NOT do**:
  - Do NOT modify the `Documentation` struct or doc comment parsing
  - Do NOT validate doc comments on private (non-public, non-entry) functions
  - Do NOT modify `no-doc-comments/src/main.op` — it must remain invalid
  - Do NOT change the formatter test inputs (`fmt-test/src/input-*.op`) — they are not compiled
  - Do NOT add doc comment validation to type declarations (only functions and let bindings)

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
    - Reason: Touches type checker logic, test projects, and integration tests — moderate complexity with cross-cutting concerns
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2 (with Tasks 6, 7)
  - **Blocks**: Task 9
  - **Blocked By**: Task 1 (needs new TypeError variants)

  **References**:

  **Pattern References**:
  - `src/type_system/checker/declarations.rs` — declaration type-checking logic: find where `Decl::Function` and `Decl::Let` are handled, add doc comment checks there
  - `tests/integration_e2e.rs:806-849` — `immutability_compile_error` test: exact pattern to follow for `no_doc_comments_fails_to_compile` — reads file, calls `compile_program()`, asserts `Err`

  **API/Type References**:
  - `src/ast.rs` — `Decl::Function { name, doc_comment, is_public, is_entry, span, ... }`: fields to check
  - `src/ast.rs` — `Decl::Let { name, doc_comment, is_public, span, ... }`: fields to check for public let bindings
  - `src/ast/documentation.rs:13-22` — `Documentation { raw, sections, attributes, span }`: the `raw` field is what to measure for length (after `.trim()`)
  - `src/type_system/errors.rs` — `TypeError::MissingDocComment`, `TypeError::DocCommentTooShort` (from Task 1)

  **Test References**:
  - `test-projects/no-doc-comments/src/main.op` — test file that MUST fail: entry at line 9 without doc comment
  - `test-projects/should-print-final-result/src/main.op` — test file that must be FIXED: add doc comment to entry at line 8
  - `test-projects/hello-world/src/main.op:10-13` — example of valid doc comment on entry function

  **Acceptance Criteria**:

  - [ ] `cargo build` succeeds
  - [ ] `compile_program()` on `no-doc-comments/src/main.op` returns `Err` with `MissingDocComment`
  - [ ] `compile_program()` on `should-print-final-result/src/main.op` (with added doc comment) returns `Ok`
  - [ ] All existing integration tests pass (`cargo test --features integration`)
  - [ ] New `no_doc_comments_fails_to_compile` test passes
  - [ ] New `multiple_entry_fails_to_compile` test passes

  **QA Scenarios (MANDATORY):**

  ```
  Scenario: no-doc-comments project fails to compile
    Tool: Bash
    Preconditions: Doc comment validation implemented
    Steps:
      1. Run `cargo test --features integration no_doc_comments_fails_to_compile 2>&1`
      2. Check test passes (compilation correctly rejected)
    Expected Result: Test passes — compilation fails with MissingDocComment error
    Failure Indicators: Test failure — either compilation succeeds or wrong error type
    Evidence: .sisyphus/evidence/task-5-no-doc-comments-fails.txt

  Scenario: Existing test projects still compile
    Tool: Bash
    Preconditions: Validation implemented, should-print-final-result fixed
    Steps:
      1. Run `cargo test --features integration hello_world_compiles_links_and_runs 2>&1`
      2. Run `cargo test --features integration fib_recursive_compiles_links_and_runs 2>&1`
      3. Run `cargo test --features integration lambda_basic_compiles_and_returns_correct_value 2>&1`
      4. Check all pass
    Expected Result: All existing tests pass — doc comment validation doesn't break valid code
    Failure Indicators: Any test failure
    Evidence: .sisyphus/evidence/task-5-existing-tests-pass.txt

  Scenario: Doc comment too short is caught
    Tool: Bash
    Preconditions: Validation implemented, cargo build --release succeeds
    Steps:
       1. Create a temporary .op file using a heredoc:
          ```bash
          cat <<'EOF' > /tmp/test_short_doc.op
          ## Short ##
          entry main = f(): void =>
              return void
          EOF
          ```
       2. Run the compiler on it:
          `./target/release/opalescent /tmp/test_short_doc.op 2>&1`
      3. Check that stderr/output contains "too short" or "DocCommentTooShort" or "minimum"
      4. Check exit code is non-zero
      5. Clean up: `rm -f /tmp/test_short_doc.op`
    Expected Result: Compilation fails with error about doc comment being too short (< 30 chars)
    Failure Indicators: Compilation succeeds, or error message doesn't mention doc comment length
    Evidence: .sisyphus/evidence/task-5-short-doc-comment-rejected.txt
  ```

  **Commit**: YES
  - Message: `feat(type-system): add doc comment validation for public/entry functions`
  - Files: `src/type_system/checker/declarations.rs`, `test-projects/should-print-final-result/src/main.op`, `tests/integration_e2e.rs`
  - Pre-commit: `cargo test --features integration`

- [x] 6. `compile_project()` Orchestrator + Entry-in-Main-Only Validation

  **What to do**:
  - **Part A: Create `compile_project()` function** in `src/compiler.rs`:
    1. New public function: `compile_project(project_dir: &Path, output_dir: &Path) -> Result<PathBuf, CompileError>`
    2. Implementation steps:
       a. Construct path to `src/main.op` from `project_dir`
       b. If `src/main.op` doesn't exist → return error: `"No src/main.op found in project directory"`
       c. Use `ModuleLoader::discover_all_modules(main_path)` to find all imported files
       d. For each discovered file (in topological order):
          - Read source with `ModuleLoader::get_module_source()`
          - Parse with existing `parse()` function
          - Type-check with existing type checker (need to set up shared `ModuleResolver` across files)
          - Compile to LLVM module with `compile_to_module()`
          - Emit object file with `emit_object_file()` — each file gets its own `.o` (e.g., `main.o`, `utils.o`)
       e. Collect all `.o` file paths
       f. Call `link_object_files()` (from Task 4) to link all `.o` files into final binary
       g. Return path to final binary
     3. For type checking across files:
        - Create a shared `ModuleResolver` instance
        - After type-checking each file, extract its public symbols into a `ModuleInterface` and register it with `ModuleResolver`
        - When type-checking a file that imports from a previously-checked file, the `ModuleInterface` will already be registered
        - This is why topological ordering matters — dependencies are checked first
     4. **Build and pass an imported-symbol signature map for codegen** (CRITICAL for Task 7):
        - After type-checking all modules and before codegen of each file, build a `BTreeMap<String, SymbolInfo>` (or `BTreeMap<String, CoreType>`) mapping each imported symbol name to its full type signature
        - This map is populated from the `ModuleResolver`'s registered `ModuleInterface.exports` for each imported module
        - Pass this map into `compile_to_module()` (or a new variant `compile_to_module_with_imports()`) so that `CodegenEnv` can use it when generating `extern` declarations for imported functions
        - The `CodegenEnv` struct already has `imported_functions: BTreeMap<String, String>` (name → module) — extend this or add a parallel `imported_signatures: BTreeMap<String, CoreType>` field to carry the resolved function types
        - This is essential: without it, `codegen_import_declaration()` in Task 7 cannot determine the correct LLVM function type for extern declarations
  - **Part B: Entry-in-main-only validation**:
    1. During project compilation (NOT single-file compilation), after parsing each non-main file:
       - Scan its AST for `Decl::Function` with `is_entry == true`
       - If found → return `TypeError::EntryNotInMainModule { file_path, span }` error
    2. This check only applies in `compile_project()`, not in `compile_program()` (single-file mode)
  - **Part C: Wire into `opal build`**:
    - Modify `run_build_command()` in `src/app.rs` to use `compile_project()` instead of the current single-file approach
    - Read `opal.toml` for project name, use project directory as `project_dir`

  **Must NOT do**:
  - Do NOT break single-file `compile_program()` — it must continue to work independently
  - Do NOT merge LLVM modules — each file gets its own LLVM context and module (per-module compile + link)
  - Do NOT implement cross-module function inlining or link-time optimization
  - Do NOT add support for nested module directories beyond what import paths specify

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: Core compilation pipeline change requiring careful orchestration of parsing, type-checking, codegen, and linking across multiple files — the most complex task in this plan
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2 (with Tasks 5, 7)
  - **Blocks**: Tasks 8, 9
  - **Blocked By**: Tasks 1, 3, 4 (needs new error types, module loader, multi-object linker)

  **References**:

  **Pattern References**:
  - `src/compiler.rs` — `compile_program(source: &str, output_dir: &Path)`: existing single-file pipeline — follow its structure but extend to multiple files
  - `src/compiler.rs` — `compile_to_module(context: &Context, source: &str)`: compiles source to LLVM module — call this per file
  - `src/compiler.rs` — `emit_object_file(module: &Module, path: &Path)`: emits `.o` file — call this per file
  - `src/app.rs:522` — `run_build_command()`: current `opal build` implementation — modify to use `compile_project()`

  **API/Type References**:
  - `src/module_loader.rs` — `ModuleLoader` (from Task 3): `discover_all_modules()`, `resolve_import_path()`, `get_module_source()`
  - `src/compiler.rs` — `link_object_files()` (from Task 4): links multiple `.o` files
  - `src/type_system/module_resolver.rs` — `ModuleResolver::new()`, `register_module_interface(interface)`, `register_symbol_for_module(module_path, symbol)`, `resolve_symbol(source, name, span)`, `resolve_all_exports(source, span)`, `validate_no_cycles_from(module, span)`: shared across files for cross-module type checking
  - `src/type_system/module_resolver.rs` — `ModuleInterface { exports: BTreeMap<String, SymbolInfo>, private_symbols: BTreeMap<String, SymbolInfo>, module_path: String }`: must be constructed from each file's type-checked AST. Use `ModuleInterface::new(module_path)` then `register_symbol(symbol_info)` to populate exports.
  - `src/type_system/checker.rs:82` — `TypeChecker` struct definition and `new()` constructor. The `type_check_program(&program)` method is defined in `src/type_system/checker/declarations.rs:674`. May need to accept an optional `ModuleResolver` for cross-module awareness. After type-checking a file, extract its registered symbols to build a `ModuleInterface` for that module.
  - `src/type_system/symbol_table.rs` — `SymbolInfo { name, symbol_type, core_type, visibility, ... }`: the symbol record type used in `ModuleInterface.exports`. The `core_type: CoreType` field contains the full type signature (including `CoreType::Function { parameters, return_types, ... }`) needed by codegen.
  - `inkwell::context::Context` — LLVM context: create one per file (they can't be shared across threads, but since we compile sequentially this is fine)

  **Acceptance Criteria**:

  - [ ] `cargo build` succeeds
  - [ ] `compile_project()` function exists and is public
  - [ ] Single-file `compile_program()` still works unchanged
  - [ ] Entry keyword in non-main file produces `EntryNotInMainModule` error
  - [ ] `opal build` uses `compile_project()` for multi-file compilation
  - [ ] Object files are generated per-source-file (not merged into one)

  **QA Scenarios (MANDATORY):**

  ```
  Scenario: compile_project() handles single-file project (no imports)
    Tool: Bash
    Preconditions: compile_project() implemented, cargo build --release succeeds
    Steps:
      1. Run `cd test-projects/hello-world && ../../target/release/opalescent build 2>&1`
         (opal build uses compile_project() as wired in Part C)
      2. Check exit code is 0
      3. Run the produced binary: `test-projects/hello-world/target/program 2>&1`
      4. Check stdout contains "Hello world"
    Expected Result: Single-file projects compile identically through compile_project() via `opal build`
    Failure Indicators: Compilation failure, linker errors, incorrect output
    Evidence: .sisyphus/evidence/task-6-single-file-project.txt

  Scenario: Entry in wrong file produces error
    Tool: Bash
    Preconditions: compile_project() implemented, cargo build --release succeeds
    Steps:
      1. Create temporary project directory:
         `mkdir -p /tmp/test-entry-wrong/src`
       2. Create opal.toml using a heredoc:
          ```bash
          cat <<'EOF' > /tmp/test-entry-wrong/opal.toml
          name = "test-entry-wrong"
          version = "1.0.0"
          EOF
          ```
      3. Create src/main.op with valid entry and import:
         Write to /tmp/test-entry-wrong/src/main.op:
         ```
         import add from ./math
         ##
             Description: Entry point for testing entry validation
         ##
         entry main = f(args: string[]): void =>
             return void
         ```
      4. Create src/math.op with INVALID entry keyword:
         Write to /tmp/test-entry-wrong/src/math.op:
         ```
         ##
             Description: This entry should not be allowed here
         ##
         entry bad = f(): void =>
             return void
         ```
      5. Run `cd /tmp/test-entry-wrong && /home/justi/Projects/opalescent/target/release/opalescent build 2>&1`
      6. Check exit code is non-zero
      7. Check output contains "entry" and "src/main.op" (EntryNotInMainModule error message)
      8. Clean up: `rm -rf /tmp/test-entry-wrong`
    Expected Result: EntryNotInMainModule error produced with clear message
    Failure Indicators: Compilation succeeds, or error doesn't mention entry restriction
    Evidence: .sisyphus/evidence/task-6-entry-wrong-file-error.txt
  ```

  **Commit**: YES
  - Message: `feat(compiler): add compile_project for multi-file compilation`
  - Files: `src/compiler.rs`, `src/app.rs`
  - Pre-commit: `cargo build`

- [x] 7. Local Import Codegen + `@scope/package` Error

  **What to do**:
  - **Part A: Local import codegen** — modify `src/codegen/functions.rs` `codegen_import_declaration()`:
    1. Currently, codegen only handles stdlib imports (`"standard"` and `"math"` modules). For local imports (`./path`), it currently does nothing or errors.
     2. For local imports in a multi-file project:
        - The imported functions exist in separate `.o` files that will be linked together
        - Codegen must generate `extern` function declarations in the current module's LLVM IR for each imported symbol
        - This tells LLVM "this function exists but is defined elsewhere" — the linker resolves it
        - Use `module.add_function(name, fn_type, Some(Linkage::External))` to declare external functions
        - **Getting the correct function type**: The `CodegenEnv` will carry an `imported_signatures: BTreeMap<String, CoreType>` map (provided by `compile_project()` in Task 6). For each imported symbol, look up its `CoreType::Function { parameters, return_types, error_types, ... }` from this map, then convert the `CoreType` parameter/return types to LLVM types using the same type-mapping logic used in `codegen_function_declaration()` (e.g., `CoreType::Int32` → `context.i32_type()`, `CoreType::String` → `context.ptr_type(...)`, etc.)
        - If the signature is not found in the map (should not happen if type-checking passed), emit a `CodegenError`
    3. For type imports (`import type X from ./path.types`):
       - Types are compile-time only — no codegen needed, just ensure the type info is available from the type checker
       - Codegen can skip these entirely (types have no runtime representation beyond their use in function signatures)
  - **Part B: `@scope/package` error** — add handling in codegen or type checker:
    1. When encountering an import with source starting with `@`:
       - Emit `TypeError::PackageImportNotSupported { path, span }` error
    2. This should be caught early in the compilation pipeline — ideally during type checking in `register_import_declaration()` in `src/type_system/checker/module_checking.rs`
    3. Since the module loader (Task 3) already returns this error during path resolution, this may already be handled. If so, just verify the error propagates correctly.
  - **Part C: Ensure imported function symbols use correct linkage**:
    1. Functions in non-main files that are `public` must be compiled with `External` linkage (default) so the linker can see them
    2. Functions that are NOT public should use `Internal` linkage (prevents symbol leaks)
    3. Check that `codegen_function_declaration()` sets linkage based on `is_public`

  **Must NOT do**:
  - Do NOT implement package resolution — only emit "not yet supported" error
  - Do NOT change stdlib import codegen — it works fine
  - Do NOT add dynamic linking or shared library support
  - Do NOT modify the parser's import handling

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
    - Reason: Requires understanding of LLVM IR linkage semantics and how cross-module symbol resolution works during linking
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2 (with Tasks 5, 6)
  - **Blocks**: Task 9
  - **Blocked By**: Tasks 1, 3 (needs error types and module loader for path resolution)

  **References**:

  **Pattern References**:
  - `src/codegen/functions.rs` — `codegen_import_declaration()`: current implementation handling stdlib imports only. This is the main function to extend.
  - `src/codegen/functions_stdlib.rs` — stdlib function mapping: shows how stdlib functions are declared as LLVM externals — similar pattern needed for local imports
  - `src/codegen/functions.rs` — `codegen_function_declaration()`: how functions are compiled — check linkage setting here

  **API/Type References**:
  - `inkwell::module::Module::add_function(name, fn_type, linkage)` — for declaring external functions from other modules
  - `inkwell::module::Linkage::External` — for public/imported functions
  - `inkwell::module::Linkage::Internal` — for private functions (not exported)
  - `src/ast.rs` — `Decl::Import { items, source, span }` — `source` field contains the import path
  - `src/type_system/checker/module_checking.rs` — `register_import_declaration()`: where import paths are resolved during type checking
  - `src/codegen/expressions.rs:39-46` — `CodegenEnv` struct: has `imported_functions: BTreeMap<String, String>` (name → module). Extend with `imported_signatures: BTreeMap<String, CoreType>` to carry resolved function types from Task 6's `compile_project()` pipeline
  - `src/type_system/types.rs` — `CoreType::Function { generic_params, parameters, return_types, error_types }`: the type representation to convert to LLVM types for extern declarations
  - `src/type_system/module_resolver.rs` — `ModuleInterface.exports`: `BTreeMap<String, SymbolInfo>` where each `SymbolInfo.core_type` contains the function's `CoreType` signature

  **Acceptance Criteria**:

  - [ ] `cargo build` succeeds
  - [ ] Local imports generate `extern` function declarations in LLVM IR
  - [ ] `@scope/package` imports produce `PackageImportNotSupported` error
  - [ ] Public functions use External linkage, private functions use Internal linkage
  - [ ] Stdlib imports continue to work unchanged

  **QA Scenarios (MANDATORY):**

  ```
  Scenario: @scope/package import produces error
    Tool: Bash
    Preconditions: Package import error handling implemented
    Steps:
      1. Create temporary .op file with `import foo from @test/pkg`
      2. Attempt to compile
      3. Verify error message contains "not yet supported" or "package"
    Expected Result: Compilation fails with PackageImportNotSupported error
    Failure Indicators: Compilation succeeds, wrong error message
    Evidence: .sisyphus/evidence/task-7-package-import-error.txt

  Scenario: Stdlib imports still work
    Tool: Bash
    Preconditions: Codegen changes applied
    Steps:
      1. Run `cargo test --features integration fib_recursive_compiles_links_and_runs 2>&1`
      2. This test project uses `import int64_to_string from standard` — verifies stdlib isn't broken
    Expected Result: Test passes — stdlib imports unaffected
    Failure Indicators: Test failure
    Evidence: .sisyphus/evidence/task-7-stdlib-unchanged.txt
  ```

  **Commit**: YES
  - Message: `feat(codegen): support local imports and reject package imports`
  - Files: `src/codegen/functions.rs`, `src/type_system/checker/module_checking.rs`
  - Pre-commit: `cargo test --features integration`

- [x] 8. Wire `opal run` (No Args) to Project-Aware Compilation

  **What to do**:
  - Modify the `opal run` (no args) path in `src/app.rs` (implemented in Task 2) to use `compile_project()` instead of single-file `compile_and_run()`:
    1. When `opal run` (no args) detects `src/main.op` exists:
       - Determine the project directory (CWD)
       - Create output directory (e.g., `{CWD}/target/`)
       - Call `compile_project(project_dir, output_dir)` instead of `compile_and_run()`
       - Execute the resulting binary
    2. This makes `opal run` (no args) handle projects with imports — `src/main.op` can import from `./utils`, and it will all compile correctly
    3. Keep `opal run <file.op>` (explicit file) using single-file compilation as before (users may want to compile individual files)

  **Must NOT do**:
  - Do NOT change `opal run <file.op>` behavior — only the no-args path
  - Do NOT add new CLI flags
  - Do NOT modify compile_project() — just use it

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Small wiring change in app.rs — replace one function call with another
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 3 (with Task 9)
  - **Blocks**: Task 9 (integration tests)
  - **Blocked By**: Tasks 2, 6 (needs initial shortcut + compile_project)

  **References**:

  **Pattern References**:
  - `src/app.rs` — `run_run_command()`: the function modified in Task 2 — update the no-args branch to use compile_project()
  - `src/app.rs` — `run_build_command()`: already wired to compile_project() in Task 6 — follow same pattern

  **API/Type References**:
  - `src/compiler.rs` — `compile_project(project_dir: &Path, output_dir: &Path) -> Result<PathBuf, CompileError>` (from Task 6)
  - `std::process::Command` — for executing the compiled binary after project compilation

  **Acceptance Criteria**:

  - [ ] `cargo build` succeeds
  - [ ] `opal run` (no args) uses `compile_project()` for project-aware compilation
  - [ ] Projects with imports compile correctly via `opal run` (no args)
  - [ ] `opal run <file.op>` (explicit file) still uses single-file compilation

  **QA Scenarios (MANDATORY):**

  ```
  Scenario: opal run (no args) uses project compilation
    Tool: Bash
    Preconditions: compile_project() working, multi-file test project exists (from Task 9)
    Steps:
      1. cd to multi-file test project directory
      2. Run `../../target/release/opalescent run 2>&1`
      3. Verify it compiles successfully (both files) and runs
    Expected Result: Multi-file project compiles and runs via opal run
    Failure Indicators: Import errors, linker errors, wrong output
    Evidence: .sisyphus/evidence/task-8-project-aware-run.txt

  Scenario: opal run <file.op> still works in single-file mode
    Tool: Bash
    Preconditions: Changes applied
    Steps:
      1. Run `./target/release/opalescent run test-projects/hello-world/src/main.op 2>&1`
      2. Check stdout contains "Hello world"
    Expected Result: Single-file mode unchanged
    Failure Indicators: Different behavior than before
    Evidence: .sisyphus/evidence/task-8-single-file-unchanged.txt
  ```

  **Commit**: YES
  - Message: `feat(cli): wire opal run to project-aware compilation`
  - Files: `src/app.rs`
  - Pre-commit: `cargo build`

- [x] 9. Multi-File Test Project + Integration Tests

  **What to do**:
  - **Part A: Create multi-file test project** at `test-projects/multi-file/`:
    1. `opal.toml`:
       ```toml
       name = "multi-file"
       version = "1.0.0"
       ```
    2. `src/main.op` — imports from `./math` and uses the imported function:
       ```opal
       import add from ./math
       import int32_to_string from standard

       ##
           Description: Entry point demonstrating multi-file imports
       ##
       entry main = f(args: string[]): void =>
           let result = add(3, 4)
           println(int32_to_string(result))
           return void
       ```
       (Note: adjust `int32_to_string` vs `int64_to_string` based on what stdlib actually provides for the return type of `add`. Check existing test projects for the correct stdlib function.)
    3. `src/math.op` — exports a public `add` function:
       ```opal
       ##
           Description: Adds two integers and returns the sum
       ##
       public let add = f(a: int32, b: int32): int32 =>
           return a + b
       ```
    4. `.gitignore`: `target/`
  - **Part B: Add integration tests** to `tests/integration_e2e.rs`:
    1. `multi_file_compiles_and_runs` test:
       - Use `compile_project()` on `test-projects/multi-file/`
       - Execute the binary
       - Assert stdout contains `"7"` (3 + 4)
       - Assert exit code is 0
       - Follow the exact pattern of existing tests (prepare_dir, execution_result closure, cleanup_dir)
    2. `opal_run_no_args_finds_main` test (optional — may require running the binary as subprocess):
       - This tests the CLI path, which is harder to test in-process
       - If too complex, skip this test and rely on QA scenarios
  - **Part C: Ensure complete integration test coverage**:
    - Verify all new and existing integration tests pass: `cargo test --features integration`

  **Must NOT do**:
  - Do NOT create overly complex test project — minimal example demonstrating imports
  - Do NOT add package imports to the test project
  - Do NOT modify existing test projects (except what Task 5 already does)

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
    - Reason: Creates test infrastructure and validates the entire compilation pipeline end-to-end across all features
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 3 (with Task 8)
  - **Blocks**: F1-F4 (Final Verification)
  - **Blocked By**: Tasks 5, 6, 7, 8 (needs all features implemented)

  **References**:

  **Pattern References**:
  - `tests/integration_e2e.rs:176-247` — `hello_world_compiles_links_and_runs`: exact test pattern to follow — prepare_dir, read source, compile, run, check output, cleanup
  - `tests/integration_e2e.rs:806-849` — `immutability_compile_error`: pattern for fail-to-compile tests
  - `test-projects/hello-world/` — reference project structure (opal.toml, .gitignore, src/main.op)
  - `test-projects/fib-recursive/src/main.op` — example of project with stdlib imports and public function + entry with doc comments

  **API/Type References**:
  - `src/compiler.rs` — `compile_project(project_dir: &Path, output_dir: &Path)` (from Task 6): use this for multi-file test
  - `src/compiler.rs` — `compile_program(source: &str, output_dir: &Path)` — still used for single-file tests

  **Acceptance Criteria**:

  - [ ] `test-projects/multi-file/` directory exists with correct structure
  - [ ] `cargo test --features integration multi_file_compiles_and_runs` passes
  - [ ] All existing integration tests still pass
  - [ ] `cargo test --features integration` passes with zero failures

  **QA Scenarios (MANDATORY):**

  ```
  Scenario: Multi-file test project compiles and produces correct output
    Tool: Bash
    Preconditions: All features implemented
    Steps:
      1. Run `cargo test --features integration multi_file_compiles_and_runs 2>&1`
      2. Check test passes
      3. Verify test asserts stdout contains "7"
    Expected Result: Multi-file project compiles, links, runs, outputs "7"
    Failure Indicators: Test failure, linker errors, wrong output
    Evidence: .sisyphus/evidence/task-9-multi-file-test.txt

  Scenario: Full integration test suite passes
    Tool: Bash
    Preconditions: All features implemented, all test projects fixed
    Steps:
      1. Run `cargo test --features integration 2>&1`
      2. Check all tests pass
      3. Count total tests: should be original count + 3 new tests (no_doc_comments, multiple_entry, multi_file)
    Expected Result: All tests pass, zero failures
    Failure Indicators: Any test failure
    Evidence: .sisyphus/evidence/task-9-full-suite.txt

  Scenario: Circular import detected
    Tool: Bash
    Preconditions: Module loader and compile_project() working, cargo build --release succeeds
    Steps:
      1. Create temporary project directory:
         `mkdir -p /tmp/test-circular/src`
       2. Create opal.toml using a heredoc:
          ```bash
          cat <<'EOF' > /tmp/test-circular/opal.toml
          name = "test-circular"
          version = "1.0.0"
          EOF
          ```
      3. Create src/main.op:
         Write to /tmp/test-circular/src/main.op:
         ```
         import foo from ./a
         ##
             Description: Entry point for circular import test
         ##
         entry main = f(args: string[]): void =>
             return void
         ```
      4. Create src/a.op:
         Write to /tmp/test-circular/src/a.op:
         ```
         import bar from ./b
         ##
             Description: Exports foo function from module a
         ##
         public let foo = f(): void =>
             return void
         ```
      5. Create src/b.op:
         Write to /tmp/test-circular/src/b.op:
         ```
         import foo from ./a
         ##
             Description: Exports bar function from module b
         ##
         public let bar = f(): void =>
             return void
         ```
      6. Run `cd /tmp/test-circular && /home/justi/Projects/opalescent/target/release/opalescent build 2>&1`
      7. Check exit code is non-zero
      8. Check output contains "circular" or "cycle"
      9. Clean up: `rm -rf /tmp/test-circular`
    Expected Result: CircularDependency error produced mentioning the cycle path
    Failure Indicators: Compilation succeeds, infinite loop, hang, or wrong error
    Evidence: .sisyphus/evidence/task-9-circular-import-error.txt
  ```

  **Commit**: YES
  - Message: `test: add multi-file test project and integration tests`
  - Files: `test-projects/multi-file/opal.toml`, `test-projects/multi-file/.gitignore`, `test-projects/multi-file/src/main.op`, `test-projects/multi-file/src/math.op`, `tests/integration_e2e.rs`
  - Pre-commit: `cargo test --features integration`

---

## Final Verification Wave (MANDATORY — after ALL implementation tasks)

> 4 review agents run in PARALLEL. ALL must APPROVE. Present consolidated results to user and get explicit "okay" before completing.
>
> **Do NOT auto-proceed after verification. Wait for user's explicit approval before marking work complete.**

- [x] F1. **Plan Compliance Audit** — `oracle`
  Read the plan end-to-end. For each "Must Have": verify implementation exists (read file, run `cargo test`, check error output). For each "Must NOT Have": search codebase for forbidden patterns — reject with file:line if found. Check evidence files exist in `.sisyphus/evidence/`. Compare deliverables against plan.
  Output: `Must Have [N/N] | Must NOT Have [N/N] | Tasks [N/N] | VERDICT: APPROVE/REJECT`

- [x] F2. **Code Quality Review** — `unspecified-high`
  Run `cargo build --release 2>&1`, `cargo clippy -- -D warnings 2>&1`, `cargo test --features integration 2>&1`. Review all changed files for: `unsafe` blocks without justification, `unwrap()`/`expect()` in non-test code without error context, `todo!()` or `unimplemented!()` macros, unused imports, dead code warnings. Check for AI slop: excessive comments, over-abstraction, generic variable names.
  Output: `Build [PASS/FAIL] | Clippy [PASS/FAIL] | Tests [N pass/N fail] | Files [N clean/N issues] | VERDICT`

- [x] F3. **Real Manual QA** — `unspecified-high`
  Start from clean state (`cargo build --release`). Execute EVERY QA scenario from EVERY task — follow exact steps, capture evidence. Test cross-task integration: multi-file project with `opal run` (no args). Test edge cases: empty `src/main.op`, file with only comments, circular imports, entry in wrong file. Save to `.sisyphus/evidence/final-qa/`.
  Output: `Scenarios [N/N pass] | Integration [N/N] | Edge Cases [N tested] | VERDICT`

- [x] F4. **Scope Fidelity Check** — `deep`
  For each task: read "What to do", read actual diff (`git log --oneline`, `git diff main...HEAD`). Verify 1:1 — everything in spec was built (no missing), nothing beyond spec was built (no creep). Check "Must NOT do" compliance. Detect cross-task contamination. Flag unaccounted changes.
  Output: `Tasks [N/N compliant] | Contamination [CLEAN/N issues] | Unaccounted [CLEAN/N files] | VERDICT`

---

## Commit Strategy

| After Task(s) | Commit Message | Key Files |
|---------------|---------------|-----------|
| 1 | `feat(type-system): add error variants for doc comments, modules, and entry validation` | `src/type_system/errors.rs` |
| 2 | `feat(cli): add opal run shortcut for src/main.op` | `src/app.rs` |
| 3 | `feat(compiler): add module loader for file-based import resolution` | `src/module_loader.rs`, `src/lib.rs` |
| 4 | `feat(compiler): extend linker for multiple object files` | `src/compiler.rs` |
| 5 | `feat(type-system): add doc comment validation for public/entry functions` | `src/type_system/checker/declarations.rs`, `test-projects/should-print-final-result/src/main.op` |
| 6 | `feat(compiler): add compile_project for multi-file compilation` | `src/compiler.rs` |
| 7 | `feat(codegen): support local imports and reject package imports` | `src/codegen/functions.rs` |
| 8 | `feat(cli): wire opal run to project-aware compilation` | `src/app.rs` |
| 9 | `test: add multi-file test project and integration tests` | `test-projects/multi-file/`, `tests/integration_e2e.rs` |

Pre-commit for all: `cargo test --features integration`

---

## Success Criteria

### Verification Commands
```bash
cargo build --release           # Expected: success, no warnings
cargo test                      # Expected: all unit tests pass
cargo test --features integration  # Expected: all integration tests pass (including new ones)
cargo clippy -- -D warnings     # Expected: no warnings
```

### Final Checklist
- [ ] All "Must Have" requirements implemented and verified
- [ ] All "Must NOT Have" guardrails respected
- [ ] All existing integration tests still pass
- [ ] New integration tests added for doc comments, multi-file, opal run
- [ ] `no-doc-comments` test project fails to compile
- [ ] `should-print-final-result` test project still compiles (doc comment added to entry)
- [ ] Multi-file test project compiles and runs with cross-file imports
- [ ] `opal run` (no args) works from project root directory
