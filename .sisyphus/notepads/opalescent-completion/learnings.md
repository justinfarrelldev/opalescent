# Opalescent Completion — Learnings

## [2026-04-10] Session Start

### Project Baseline
- 259 tests passing, lint clean, clean working tree
- Lexer and Parser 100% complete
- Type System Core ~80% — error handling fully implemented (guard/propagate/errors)
- Phase 2 Blockers #2-#9 NOT STARTED

### Key Patterns
- no_std compatible: use `alloc::collections::BTreeMap` not `std::collections::HashMap`
- All files under 500 lines (test files under 1000)
- `#[expect(..., reason = "...")]` instead of `#[allow(...)]`
- No `unwrap()`, `expect()`, `panic!()`, `todo!()`, `unimplemented!()`, `unreachable!()`
- No `as` conversions (use TryFrom/TryInto)
- No `str.to_string()` (use `to_owned()` or `String::from()`)
- No single-char lifetime names
- TDD: RED test first → GREEN impl → REFACTOR
- Always `cargo make lint-fix` before committing
- Output to temp.log via `2>&1 | tee temp.log`
- `scripts/check-line-count.sh` before each commit

### Module Structure
- `src/ast.rs` + `src/ast/` — AST definitions
- `src/parser/` — Parser modules
- `src/type_system/` — Type system modules
- `src/type_system/environment.rs` — TypeEnvironment
- `src/type_system/errors.rs` — TypeError (Warning should mirror this)
- `src/type_system/checker.rs` — TypeChecker struct
- `src/type_system/checker/expressions.rs` — Expression type checking
- `src/type_system/checker/declarations.rs` — Declaration type checking
- `src/type_system/checker/statements.rs` — Statement type checking
- `src/type_system/types.rs` — CoreType definitions
- `src/type_system/constraints.rs` — TypeConstraint enum
- `src/type_system/substitution.rs` — Substitution system
- `src/type_system/symbol_table.rs` — SymbolTable, SymbolInfo

## [2026-04-10] Task 1: Multiple Return Values - COMPLETE
- Added `return_types: Vec<Type>` to Type::Function, Decl::Function, Expr::Lambda
- Added `LabeledValue` struct for labeled return values
- Modified Stmt::Return to support `values: Vec<LabeledValue>`
- Added return-label semantic enforcement in checker (label-mode tracking)
- Added TypeError::ReturnLabelMismatch
- 263 tests passing
- Files modified: src/ast.rs, src/ast/types.rs, src/parser/declarations.rs, src/parser/expressions.rs, src/parser/helpers.rs, src/parser/statements.rs, src/parser/tests.rs, src/parser/types.rs, src/type_system/checker.rs, src/type_system/checker/declarations.rs, src/type_system/checker/expressions.rs, src/type_system/checker/statements.rs, src/type_system/checker/unification.rs, src/type_system/errors.rs, src/type_system/substitution.rs, src/type_system/tests.rs, src/type_system/types.rs

## [2026-04-10] Task 2: Standard Library Built-ins - COMPLETE
- Added TypeEnvironment built-in registry with register_builtin()/lookup_builtin().
- Pre-registered built-ins in TypeChecker::new()/with_environment(): print<T>, take_input, string_to_int32 errors ParseError, random_int32.
- Added call-site generic instantiation helper to support polymorphic built-in calls (print<T> across multiple concrete types).
- Added parser/test maintenance for Expr::If coverage to keep compile/lint clean.
- Added built-in TDD tests plus hello_world type-check test with whitespace-normalized fixture helper.
- Added stdlib/prelude.op as documentation-only signature listing (no runtime behavior).
- Validation: cargo make lint PASS, cargo make test PASS (276 passed), line-count PASS after extracting call_resolution module.

## [2026-04-11] Task 4: Warning Infrastructure
- Added `Warning` enum parallel to `TypeError` with miette diagnostics and stable warning codes.
- Added `TypeChecker` warning collection (`warnings`, `warnings()`, `clear_warnings()`, `push_warning()`) and reset in `type_check_program`.
- Converted unsafe-cast flow to warning collection path via `validate_cast_with_warnings`, while keeping invalid casts as hard errors.
- Added suppression placeholder plumbing on warning variants (`suppression_annotation: Option<String>`) to support future suppression annotations.
- Added TDD coverage for warning creation, unsafe-cast warning behavior (non-fatal), and warning collection accumulation.

## [2026-04-11] Task 3: If Expression Semantics - COMPLETE
- Added `Expr::If` AST variant while preserving existing `Stmt::If` for statement-position control flow.
- Parser now recognizes `if` in expression position (`parse_if_expression`) and supports both with-else and else-less forms.
- Type checker now infers `if` expression type by branch unification when `else` is present; mismatch raises `TypeError::TypeMismatch` with spans for both branches.
- Else-less `if` expressions now evaluate to `unit` type semantics.
- Added TDD coverage in parser/type-system tests for inference success, branch mismatch failure, and else-less behavior.
- Extracted control-flow typing into new `src/type_system/checker/control_flow.rs` to satisfy line-count constraints while sharing guard/if helpers.
- Validation: cargo make lint-fix PASS, cargo make lint PASS, cargo make test PASS (276 passed), scripts/check-line-count.sh PASS.

## [2026-04-11] Task 7: Member Access Type Checking - COMPLETE
- Implemented `Expr::Member` handling in `type_check_expr` with receiver-first typing and support for chained member access.
- Added module member resolution via qualified symbol lookup (`module.member`) and basic struct-like field resolution via nominal lookup (`Type.member`).
- Missing members now emit `TypeError::SymbolNotFound` with source span preserved, and tests assert qualified names in diagnostics.
- Added RED→GREEN tests for module member success, struct-like field success, missing member error/span, and chained access success.
- Validation: cargo make test PASS (287 passed, 1 ignored), cargo make lint PASS, cargo make lint-fix PASS, scripts/check-line-count.sh PASS.

## [2026-04-11] Task 9: Division by Zero Detection - COMPLETE
- Added `TypeError::DivisionByZero` with diagnostic code `opalescent::type_system::division_by_zero` and labeled divisor span for clear compile-time reporting.
- Added helper `zero_divisor_operation_name` in `checker/helpers.rs` to detect constant zero divisors for `/` and `%` via existing integer-constant extraction.
- Updated binary expression checking to hard-error on compile-time-known zero RHS for division and modulo, while preserving non-constant divisor behavior as runtime-only.
- Added RED→GREEN tests for division-by-zero and modulo-by-zero (including non-constant LHS + literal zero RHS), plus a non-constant divisor no-error case.
- Validation: `timeout 30 cargo test` PASS, `timeout 30 cargo make test` PASS (299 passed, 1 ignored), `cargo make lint` PASS, `cargo make lint-fix` PASS, `scripts/check-line-count.sh` PASS, LSP diagnostics clean on changed files.

## [2026-04-10] Task 10: Integration Tests — COMPLETE (commit 096625e)

### What Was Created
- `src/type_system/test_integration.rs` — new file (430 lines, excluded from line-count check)
- `src/type_system.rs` — added `#[cfg(test)] mod test_integration;`
- 10 tests total: 8 active + 2 `#[ignore]`d spec-file tests

### Tests Added
1. `test_hello_world_full_pipeline_parses_and_type_checks` — full pipeline against `language-spec/hello_world.op`
2. `test_fib_recursive_equivalent_parses_and_type_checks` — brace-syntax recursive fib (green)
3. `test_fib_recursive_spec_file_parses_and_type_checks` — `#[ignore]`: colon-block syntax not yet supported
4. `test_fib_iterative_equivalent_parses_and_type_checks` — brace-syntax iterative fib (green)
5. `test_fib_iterative_spec_file_parses_and_type_checks` — `#[ignore]`: colon-block syntax not yet supported
6. `test_multi_error_reporting_returns_all_errors` — ≥2 errors collected, not short-circuited
7. `test_multi_error_reporting_errors_have_type_mismatch_kind` — error kind preserved through pipeline
8. `test_multi_error_correct_and_bad_declarations_all_checked` — mixed valid/invalid, all checked
9. `test_error_span_is_non_zero_for_type_mismatch` — span accuracy for TypeMismatch
10. `test_error_span_for_undefined_symbol_is_non_zero` — span accuracy for SymbolNotFound
11. `test_parse_succeeds_on_semantically_invalid_program` — parser/type-checker stage isolation
12. `test_clean_program_produces_zero_errors_and_zero_warnings` — zero-noise golden path

### Key Learnings
- `src/type_system/tests.rs` is 5228 lines — NEVER add more tests there; use `test_integration.rs`
- Spec files (`fib_recursive.op`, `fib_iterative.op`) use `if n is 0:` colon-block syntax — parser requires `{ }` braces; brace equivalents needed
- Integer literals (`0`, `1`, `42`) infer as `int64`; params in fib sources must match (`int64` not `int32`)
- `Decl::Let` without explicit type annotation is NOT pre-registered in first pass; use `public foo =` (`Decl::Function`) for forward-referenced functions
- **clippy `pattern_type_mismatch`**: When iterating `Vec<TypeError>` with `.iter()`, closures receive `&TypeError`. In `matches!`, use `matches!(**err, TypeError::Foo { .. })` (double-deref since `.find()` adds one more `&`); in `is_some_and`, use `if let TypeError::Foo { .. } = *err { .. }` (single-deref)
- Baseline after Task 10: **309 tests passing, 3 ignored** (1 pre-existing + 2 new spec-file ignores)

## [2026-04-10] Task 11: Function System Completion
- Entry-point cardinality validation implemented via `TypeChecker::validate_entry_points` with `TypeError::MissingEntryPoint` and `TypeError::DuplicateEntryPoint`; integrated as post-typecheck semantic validation so existing error-collection behavior remains intact.
- Generic call inference now fails explicitly with `TypeError::CannotInferGenericType` when a declared generic parameter is not constrained by call arguments and no explicit generic args are provided.
- Scope/closure behavior verified with integration coverage for lambda capture of outer variables inside function scope, plus guard/propagate with multi-return integration coverage.
- Hot-reload signature stability metadata added in new `src/type_system/checker/hot_reload.rs` and wired through declaration signature registration; `TypeChecker` now records/retrieves per-function signature snapshots (`signature_stable`, params/returns).
- Added integration tests for missing entry, duplicate entry, uninferable generic call, closure capture, and guard+multiple-returns success path.
- Validation after changes: `timeout 30 cargo test` PASS (314 passed, 3 ignored), `cargo make lint-fix` PASS, `cargo make lint` PASS, `scripts/check-line-count.sh` PASS, LSP diagnostics clean on changed files.

## [2026-04-10] Task 12: Variable System Completion
- Added mutability and usage metadata to `SymbolInfo` (`is_let_binding`, `is_mutable`, `read_count`) and mutable symbol lookup in `SymbolTable`.
- Implemented immutable reassignment hard-failure via new `TypeError::ImmutableAssignment` (`opalescent::type_system::immutable_assignment`) with both assignment and declaration spans.
- Updated assignment checking to enforce mutability only for identifier targets while preserving existing member/index assignment behavior.
- Wired identifier resolution to increment `read_count` so usage tracking is data-driven and scope-aware.
- Added unused-let warnings after successful program type-check (`Warning::UnusedVariable`, code `opalescent::type_system::unused_variable`) with underscore-prefix exemption.
- Preserved shadowing semantics: same-scope re-`let` continues to register a new symbol entry; immutability validation only applies to assignment statements.
- Added integration tests for immutable assignment failure, mutable assignment success, unused warning emission, and underscore suppression.
- Validation: `timeout 30 cargo test` PASS (318 passed, 3 ignored), `cargo make lint-fix` PASS, `cargo make lint` PASS, `scripts/check-line-count.sh` PASS, LSP diagnostics clean on changed files.
