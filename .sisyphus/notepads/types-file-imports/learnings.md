# Learnings — types-file-imports

## [2026-04-21] Session ses_252d7f2b9ffeASYrOJz8Ky2dvJ — Atlas initialization

### Key codebase facts
- `compile_to_module` at `src/compiler.rs:124` — current sig: `(context: &'context Context, source: &str)`
- `compile_program` at `src/compiler.rs:590` — current sig: `(source: &str, output_dir: &Path)`
- `run_check_command` at `src/app.rs:523-562` — runs own lex/parse/typecheck pipeline, bypasses compile_to_module
- `validate_entry_declarations_for_module` at `src/compiler.rs:648` — hook point for new validator
- Stdlib sentinel paths: `__stdlib__/standard`, `__stdlib__/math` — must bypass validator
- Module loader resolves `./foo.types` → `foo.types.op` at `src/module_loader.rs:48-61`
- `is_type_import` flag at `src/module_loader.rs:286-295`
- Type checker cross-file at `src/type_system/checker/module_checking.rs:63-147`
- `compile_project` codegen loop at `src/compiler.rs:780-802` — currently emits .o for ALL modules

### 23 compile_to_module call sites to update in T4:
- `src/compiler.rs:594` (prod), `:819, :841, :873, :895` (4 tests)
- `src/codegen/tests.rs:700, 1222, 1260, 1296, 1328, 1360, 1392, 1425, 1459, 1502` (10 sites)
- `src/errors/tests.rs:272, 297, 322, 347, 369, 377` (6 sites)
- `tests/integration_e2e/tests.rs:55, 98` (2 sites)

### Error accumulation idiom (from src/compiler.rs ~line 154):
```rust
report.extend_type_errors(vec![role_error]);
return Err((report, normalized_source));
```

### Fixture type syntax: use colon-block form (e.g. `type Person:\n    name: string`)
### Only .types.op in repo: `language-spec/types_example.types.op` — some tests are #[ignore], do NOT touch

## [2026-04-21] T3 GREEN — validate_module_file_role

### Implemented behavior
- Added `validate_module_file_role(path: &Path, program: &Program) -> Result<(), TypeError>` in `src/module_loader.rs`.
- Stdlib sentinels are bypassed early: paths starting with `__stdlib__/` return `Ok(())`.
- `.types.op` rules enforced:
  - Allowed: `Decl::Type`, `Decl::Import`
  - Rejected: non-type decls (`Decl::Let`, `Decl::Function` entry/non-entry, comments)
  - Error: `TypeError::NonTypeDeclarationInTypesFile { decl_kind, decl_name, file_path, span }`
- Regular `.op` rules enforced:
  - Rejected: `Decl::Type`
  - Error: `TypeError::TypeDeclarationOutsideTypesFile { type_name, file_path, span }`
- Uses declaration spans via `TypeError::span_from_span(...)`.
- `let` declarations (including function-typed lets) report `decl_kind = "let"` as required by tests.

### Follow-up fix discovered during verification
- Two RED tests used compact type fixture syntax (`type User: name: string`) that failed parse in current parser; restored compact single-line type declaration support in `src/parser/declarations.rs` while preserving colon-block support.
- Fixed `is_types_file` doctest import path to `use opalescent::module_loader::is_types_file;` so full `cargo test` (including doctests) is green.


## [2026-04-21] T4 GREEN — wire file-role validation into single-file compilation paths

### What changed
- `compile_to_module` now takes `source_path: &Path` and calls `validate_module_file_role(source_path, &program)` immediately after parse success and before type checking.
- Validator failures in `compile_to_module` follow existing report accumulation idiom:
  - `report.extend_type_errors(vec![role_error]);`
  - `return Err((report, normalized_source));`
- `compile_program` now takes `source_path: &Path` and threads it into `compile_to_module`.
- `run_check_command` now validates parsed AST with `validate_module_file_role(file_path, &program)` before type checking and renders errors via existing `render_report` flow.

### Call-site migration notes
- All targeted `compile_to_module` call sites were updated to include path arg.
- Test call sites use `Path::new("test.op")` as required.
- All `compile_program` call sites in `src/app.rs` now pass `Path::new(source_path)`.
- Additional integration call sites using `compile_program` across `tests/` were updated to preserve build/test compatibility with the new signature.

## [2026-04-21] T5 GREEN — wire file-role validation into `compile_project` and skip `.types.op` codegen

### Key implementation notes
- In `compile_project` module parse loop, file-role validation now runs immediately after `validate_entry_declarations_for_module(...)` via `validate_module_file_role(module_path, &program)`.
- Validator failures follow report accumulation style in project compile path by constructing `CompilationErrorReport`, calling `report.extend_type_errors(vec![role_error])`, and returning `CompileError::Report { report, normalized_source }`.
- Multi-module codegen loop now skips type-definition-only files with `if is_types_file(module_path) { continue; }`, preventing `.o` emission for `.types.op` modules.
- Added `is_types_file` to `src/compiler.rs` module_loader imports; no changes made to `src/module_loader.rs`, `src/app.rs`, `compile_to_module`, or `compile_program`.

## [2026-04-21] T8a/b/c + T9a/b — integration tests for types-file imports

### Added integration coverage
- Added 3 success execution tests in `tests/integration_e2e/project_execution.rs`:
  - `import_types_basic_compiles_and_runs`
  - `import_types_aliased_compiles_and_runs`
  - `import_types_multiple_compiles_and_runs`
- Each test follows the existing project-style pattern: resolve `cwd`, set `project_dir`/`target`, call `compile_project(&project_dir, &temp_dir)`, execute produced binary, assert exact stdout via `trim_end()`.

### Added compile-failure coverage
- Added 2 failure tests in `tests/integration_e2e/compile_failures.rs`:
  - `type_declaration_in_regular_file_is_rejected`
  - `value_declaration_in_types_file_is_rejected`
- Both tests assert `Err(CompileError::Report { report, .. })` and scan `report.entries()` for expected variants:
  - `TypeError::TypeDeclarationOutsideTypesFile`
  - `TypeError::NonTypeDeclarationInTypesFile`

### Verification notes
- `lsp_diagnostics` on changed test files reports only rust-analyzer `unlinked-file` hints (no actionable compile diagnostics).
- `cargo test --features integration --test integration_e2e` runs and the two new compile-failure tests pass.
- The three new success tests currently fail at runtime with `TypeError::PrivateSymbolAccess` for imported types from `.types.op` fixtures, indicating feature/runtime semantics are not yet aligned with fixture expectations.

## [2026-04-21] ADT field propagation fix for `.types.op` imports

### Cross-module type-checking propagation
- `ModuleInterface` now carries `adt_fields: BTreeMap<String, BTreeMap<String, CoreType>>` so ADT layout metadata is exportable across modules instead of being checker-local only.
- `ModuleResolver` gained `register_adt_fields_for_module(...)` to attach ADT schemas to the module interface storage.
- `TypeChecker::register_module_interface(...)` now hydrates local `adt_fields` from incoming interface metadata before registering into resolver.
- Added `sync_current_module_adt_fields()` and invoked it at the end of `type_check_program()` so every checked module publishes its ADT layout into its interface.

### Import alias handling required for constructors
- Constructor checks for imported types required local-name ADT ownership keys (e.g. `Account`) in addition to source-name keys (`User`).
- `register_import_declaration(...)` now calls `register_imported_type_adt_fields(...)` for imported `SymbolType::Type` symbols, copying both product and variant field schemas from source module interface and remapping owner names for aliases.

### Codegen follow-up discovered by success tests
- After type-checking fix, integration success fixtures surfaced codegen assumptions that rejected user-defined nominal annotations (`unsupported type 'Person'`, etc.).
- Let-annotation lowering now treats unknown AST basic types as nominal `CoreType::Generic { name, type_args: [] }` rather than hard-failing.
- Nested field access in print interpolation (`r.top_left.x`) needed codegen support:
  - member parsing now handles one nested level,
  - field-access lowering can resolve alias mappings from constructor-initialized product fields (e.g. mapping `r.top_left` -> `origin`) via `variable_field_aliases` tracked in `CodegenEnv`.

### Verification outcome
- `cargo test --features integration --test integration_e2e import_types` passes with all 3 success tests green.
- `cargo test` passes (unit + doc tests).
- `cargo build` passes.
