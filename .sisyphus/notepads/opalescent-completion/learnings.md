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

## [2026-04-10] Task 13: Control Flow Completion
- Added `TypeError::MissingElseBranch` (`opalescent::type_system::missing_else_branch`) for non-exhaustive `if` expressions used in non-unit value contexts.
- Added `Warning::UnreachableCode` emission in statement blocks after `return`/`break`/`continue`; warning span points at the first unreachable statement while still type-checking subsequent statements.
- Implemented branch-local narrowing for `if x is SomeType` in both expression and statement control-flow paths by shadow-registering `x` with the narrowed type inside the true-branch scope.
- Enabled parser support for primitive type tokens (`int32`, `string`, etc.) in expression position so `x is int32` parses as intended for narrowing.
- Updated binary `is`/`is not` typing to permit identifier-vs-identifier type-test form used by narrowing predicates.
- Added integration tests in `src/type_system/test_integration.rs` for unreachable warning, missing-else exhaustiveness error, and true-branch type narrowing.
- Updated existing type-system test expectation for else-less `if` from generic `TypeMismatch` to `TypeError::MissingElseBranch`.
- Validation: `timeout 30 cargo test` PASS (321 passed, 3 ignored), `cargo make lint-fix` PASS, `cargo make lint` PASS, `scripts/check-line-count.sh` PASS, LSP diagnostics clean on all changed files.

## [2026-04-11] Task 14: Arithmetic & Logic Completion
- Reduced `src/type_system/checker.rs` line count by extracting return-shape context helpers into new `checker/returns.rs`; `checker.rs` is now below limit while keeping `expressions.rs` unchanged at 999 lines.
- Extended compile-time shift bounds validation with `check_shift_bounds(op, lhs_type, rhs_expr, span)` and enriched `TypeError::InvalidShiftCount` metadata (`reason`, `count_value`) while preserving existing fields for compatibility.
- Added `fold_constant_binary_op` helper in `checker/helpers.rs` and wired arithmetic overflow helper to use it for integer constant operations (`+`, `-`, `*`, `/`, `%`).
- Registered masked shift intrinsics per spec names `bshl_masked` and `bshr_masked` (kept existing `masked_*` variants), and treated `*_masked` intrinsic calls as wrapping arithmetic mode.
- Added integration coverage for negative/out-of-range constant shifts (including reason metadata) and for `bshl_masked`/`bshr_masked` type-check success in `src/type_system/test_integration.rs`.
- Validation: `cargo make lint-fix` PASS, `cargo make lint` PASS, `scripts/check-line-count.sh` PASS (`checker.rs` 929), `timeout 30 cargo test` PASS (329 total tests, 3 ignored), LSP diagnostics clean on all changed files.

## [2026-04-11] Task 16 line-count extraction fix
- Extracted ADT constructor typing from `checker/expressions.rs` into new `checker/constructors.rs` (`type_check_constructor_expr`, `type_check_constructor_fields`) and wired module inclusion in `checker.rs`.
- Moved `TypeChecker::validate_adt_type` from `checker.rs` into `checker/constructors.rs` to keep top-level checker under enforced line limit.
- Post-extraction counts: `checker/expressions.rs` 965 lines, `checker.rs` 978 lines, `checker/constructors.rs` 178 lines; `scripts/check-line-count.sh` passes.
- Strict verification passes after refactor: `cargo make lint-fix`, `timeout 30 cargo test` (339 passed, 0 failed), and `cargo make lint`.

## [2026-04-11] Task 17: Array & Collection Type Completion
- Added new integration suite `src/type_system/test_integration_collections.rs` (registered in `src/type_system.rs`) with RED→GREEN coverage for array methods (`length`, `push`, `pop`, `map`, `filter`, `reduce`, `zip`), string methods (`length`, `split`, `join`, `contains`, `starts_with`, `ends_with`, `slice`, `to_upper`, `to_lower`), iterable for-loop support, and collection method chaining inference.
- Implemented collection intrinsics as built-in method signatures during `register_standard_builtins` via new checker modules:
  - `src/type_system/checker/collections.rs`
  - `src/type_system/checker/collections_array.rs`
  - `src/type_system/checker/collections_string.rs`
- Added intrinsic iterable protocol marker methods (`__iter_element_type`) for arrays and strings and a helper `iterable_element_type_for` used by statement type checking.
- Extended `Stmt::For` typing to accept any registered iterable protocol type (arrays and strings now both type-check) instead of array-only matching.
- Extended member resolution in expression typing to resolve receiver-specialized collection intrinsics before nominal/member fallback.
- Improved call-site inference in `checker/call_resolution.rs` by composing local argument unification substitutions and applying them to return types, enabling `map/filter` and chained collection method inference to produce concrete return types.
- Validation: `cargo make lint-fix` PASS, `timeout 30 cargo test` PASS (355 passed, 0 failed, 3 ignored), `scripts/check-line-count.sh` PASS, `cargo make lint` PASS, LSP diagnostics clean on all changed files.
- Follow-up note: Collection intrinsics were split into dedicated checker submodules to keep core files under enforced line-count limits while preserving member-call typing behavior.

## [2026-04-11] Task 18: Generic System Completion
- Implemented generic ADT declaration plumbing end-to-end: parser now captures type-level generic params/constraints on `Decl::Type`, AST stores them, and type registration resolves ADT field types using generic bindings.
- Completed generic constructor inference: constructor field checking now instantiates per-call fresh type vars for ADT generics, unifies field values to infer concrete type args, validates declared generic constraints, and returns concrete `CoreType::Generic` (e.g. `Node<int64>`, `Pair<string, boolean>`).
- Added Phase 5-prep metadata tracking in `TypeChecker`: generic declarations per ADT plus `generic_instantiations: BTreeMap<String, Vec<Vec<CoreType>>>`, with helpers in new `checker/generics.rs` and recording from both function-call inference and ADT constructor instantiation.
- Added new integration suite `src/type_system/test_integration_generics.rs` and module registration in `src/type_system.rs`, covering constructor inference, generic call inference (`identity(42)`), constraint failure, and unique instantiation metadata recording.
- Validation: `cargo make lint-fix` PASS, `timeout 30 cargo test` PASS (360 passed, 0 failed, 3 ignored), `scripts/check-line-count.sh` PASS, `cargo make lint` PASS, LSP diagnostics clean on changed files.

## [2026-04-11] Task 19: Import/Export Resolution
- Added `src/type_system/module_resolver.rs` implementing a mockable module registry (`ModuleInterface` + `ModuleResolver`) with built-in stdlib modules (`standard`, `math`), import symbol resolution, export/public validation, and dependency graph cycle detection.
- Added checker integration in new `src/type_system/checker/module_checking.rs` and wired declaration handling so `Decl::Import` resolves imported symbols, rejects private symbol access, and checks cycles through the module dependency graph.
- Extended diagnostics with `TypeError::CircularDependency`, `TypeError::UnresolvedImport`, and `TypeError::PrivateSymbolAccess`; kept existing `TypeMismatch` flow for cross-module call type checking.
- Added and registered new integration tests in `src/type_system/test_integration_modules.rs` for standard imports, unknown imports, circular dependencies, private access enforcement, and cross-module type mismatch.
- Validation: `cargo make lint-fix` PASS, `timeout 30 cargo test` PASS (365 passed, 0 failed, 3 ignored), `scripts/check-line-count.sh` PASS (all non-test files <=1000 lines), `cargo make lint` PASS.

## [2026-04-11] Task 20: Module Validation
- Added import-name clash validation in module import processing with per-module local-binding tracking; conflicting imports now produce `TypeError::ImportNameConflict` carrying name, first/second source modules, and import span.
- Extended alias import behavior so `import math as m from math` registers `m` as a module alias and also registers qualified exported members (`m.sqrt`, etc.), enabling member access/type checking through aliases.
- Added `ModuleResolver::generate_module_interface(...)` and completed module-interface coverage in integration tests to ensure public exports and private symbol buckets are populated/usable for cross-module checks.
- Added integration suite `src/type_system/test_integration_module_validation.rs` and registration in `src/type_system.rs` covering import conflicts, private access enforcement, aliased module member resolution, aliased mismatch (`TypeMismatch`), and alias disambiguation across modules.
- Validation: `cargo make lint-fix` PASS, `timeout 30 cargo test` PASS (371 passed, 0 failed, 3 ignored), `scripts/check-line-count.sh` PASS, `cargo make lint` PASS.

## [2026-04-11] Task 21: LLVM Backend Setup
- Added LLVM backend scaffold with `inkwell` 0.8.0 and LLVM 14 wiring for Phase 5 codegen infrastructure.
- LLVM version: 14, dependency configured as `inkwell = { version = "0.8.0", features = ["llvm14-0", "llvm14-0-prefer-dynamic"] }` to avoid missing static Polly on this environment while keeping LLVM 14 API compatibility.
- Env var: `LLVM_SYS_140_PREFIX=/usr/lib/llvm-14` used for all cargo build/test/lint commands.
- Validation: `cargo make lint-fix` PASS, `timeout 30 cargo test` PASS (374 passed, 0 failed, 3 ignored), `scripts/check-line-count.sh` PASS, `cargo make lint` PASS, LSP diagnostics clean on changed files (one expected rust-analyzer unlinked hint on required `src/codegen/mod.rs`).

## [2026-04-11] Task 22: Codegen Expressions + Statements (Phase 5)
- Added `src/codegen/expressions.rs` implementing literal lowering (int/float/bool/string/unit), identifier loads, binary/unary operators, explicit numeric casts, array literal allocation/stores, and array index access via GEP+load.
- Added `CodegenEnv` variable map (`alloc::collections::BTreeMap`) and `VariableBinding` to thread alloca slots + static types through lowering.
- Implemented debug-mode integer overflow trapping for `+`, `-`, `*` via LLVM overflow intrinsics (`llvm.s/u{add,sub,mul}.with.overflow.iN`) with conditional trap blocks.
- Implemented integer division/modulo runtime zero checks that branch to `llvm.trap` before `sdiv/udiv/srem/urem`.
- Added `src/codegen/statements.rs` implementing `Stmt::Let` (alloca + optional initializer store + env registration) and identifier assignment statements (`store` to existing alloca).
- Wired new modules in both `src/codegen/mod.rs` and `src/codegen.rs` to satisfy module resolution in current crate layout.
- Expanded `src/codegen/tests.rs` with RED→GREEN tests covering literals, unary/cast, let/assignment, array literal/access, overflow trap intrinsic presence, and division-by-zero trap presence using in-memory LLVM IR assertions.
- Inkwell 0.8.0 note: `build_in_bounds_gep` is `unsafe` and requires safety comments on preceding lines under strict lint configuration.
- Validation: `timeout 30 LLVM_SYS_140_PREFIX=/usr/lib/llvm-14 cargo test` PASS (379 passed, 0 failed, 3 ignored), `LLVM_SYS_140_PREFIX=/usr/lib/llvm-14 cargo make lint-fix` PASS, `LLVM_SYS_140_PREFIX=/usr/lib/llvm-14 cargo make lint` PASS, `scripts/check-line-count.sh` PASS, LSP diagnostics clean on changed files.

## [2026-04-11] Task 23: Codegen Functions + Control Flow (Phase 5)
- Added `src/codegen/functions.rs` with lowering for function declarations, call expressions (including lambda capture argument threading), guard/propagate control paths, and entry-wrapper (`main`) emission.
- Added `src/codegen/control_flow.rs` with lowering for `if` statements, `if` expressions (phi merge), loop forms (`while`/`for`/`loop`) and multi-value return handling.
- Wired dispatch integration in `src/codegen/expressions.rs` and `src/codegen/statements.rs`, and module exports in both `src/codegen/mod.rs` and `src/codegen.rs`.
- Expanded `src/codegen/tests.rs` coverage for function declarations/calls, lambda closure calls, guard/propagate error flow, if/loop lowering, and multi-return behavior via in-memory LLVM IR checks.
- Clippy-specific learning: `pattern_type_mismatch` with `&Expr` inputs required explicit reference patterns (`if let &Expr::... { ref ... } = expr_ref`) in helper lowering paths.
- Final verification: `LLVM_SYS_140_PREFIX=/usr/lib/llvm-14 cargo make lint-fix` PASS (auto-fixes applied), `LLVM_SYS_140_PREFIX=/usr/lib/llvm-14 cargo make lint` PASS, `timeout 30 env LLVM_SYS_140_PREFIX=/usr/lib/llvm-14 cargo test` PASS (385 passed, 0 failed, 3 ignored), `scripts/check-line-count.sh` PASS, LSP diagnostics clean on changed files (one expected rust-analyzer unlinked-file hint for `src/codegen/mod.rs`).

## [2026-04-11] Task 24: Runtime System Foundation
- Added Phase 5 runtime surface in `src/runtime.rs` with path-based module wiring to `src/runtime/{memory,strings,arrays,io,errors}.rs` and top-level re-exports used by generated programs.
- Added `src/runtime/mod.rs` compatibility root as required by task structure, re-exporting runtime submodules through `crate::runtime` while keeping clippy `mod_module_files` clean.
- Implemented runtime memory layer: `RuntimeAllocator` trait, `DefaultRuntimeAllocator`, `OpalString` as `Arc<str>`, and generic `OpalArray<T>` as `Arc<[T]>` with required `Debug + Clone + PartialEq` derives.
- Implemented runtime operations modules: string alloc/concat/compare/len, array alloc/index/len with `RuntimeError::IndexOutOfBounds` bounds checks, and pure-Rust `print`/`take_input` using injectable `IoHandler` for test mocking.
- Implemented error runtime model in `src/runtime/errors.rs`: `RuntimeResult<T> = Result<T, RuntimeError>`, required `RuntimeError` variants, stable error-code helper (`error_code`), message rendering, and `RuntimeResultExt` propagation helper.
- Added TDD runtime tests in `src/runtime/tests.rs` with `MockAllocator` and `MockIoHandler` only (no real stdin/stdout in tests), covering string ops, array bounds behavior, I/O injection, and error propagation helpers.
- Updated crate wiring in `src/main.rs` with `#[path = "runtime.rs"] pub mod runtime;` so runtime module is registered in crate root.
- Verification: `LLVM_SYS_140_PREFIX=/usr/lib/llvm-14 cargo make lint-fix` PASS, `LLVM_SYS_140_PREFIX=/usr/lib/llvm-14 cargo make lint` PASS, `timeout 30 bash -lc 'LLVM_SYS_140_PREFIX=/usr/lib/llvm-14 cargo test'` PASS (389 passed, 0 failed, 3 ignored), `scripts/check-line-count.sh` PASS, LSP diagnostics clean on changed files (only expected unlinked-file hint on `src/runtime/mod.rs`).
- Additional note: strict clippy profile rejects `Arc<String>` and `Arc<Vec<T>>` (`clippy::rc_buffer`), so runtime containers intentionally use `Arc<str>` and `Arc<[T]>` for equivalent reference-counted semantics.

## [2026-04-11] Task 25: Codegen ADTs + Monomorphization
- Added `src/codegen/adts.rs` with codegen for ADT constructors (`Expr::Constructor`), product field access (`Expr::Member` on constructor-backed values), and match lowering (`Expr::Match`) to LLVM `switch` + phi merge blocks.
- Added `src/codegen/monomorphization.rs` with deterministic specialization naming (`name__Type...`) and specialization cache integration using `CodegenEnv::emitted_specializations: BTreeMap<(String, Vec<String>), FunctionValue>`.
- Extended `CodegenEnv` with ADT/runtime metadata (`variable_field_indices`) and monomorphization state; wired expression dispatch for constructor/match/member plus generic call arguments.
- Updated function call codegen to accept explicit generic args and dispatch to monomorphized declarations; extended AST type conversion in codegen to support `Type::Generic`.
- Updated let-statement lowering to preserve constructor value layout (alloca based on lowered initializer type) and track product field indices for later member GEP loads.
- Added RED→GREEN tests in `src/codegen/tests.rs` for sum ADT tagged-union constructor IR shape, switch-based match lowering, deterministic monomorphized naming, generic ADT name instantiation, and product field-access GEP generation.
- Validation: `LLVM_SYS_140_PREFIX=/usr/lib/llvm-14 timeout 30 cargo test` PASS (393 passed, 3 ignored), `LLVM_SYS_140_PREFIX=/usr/lib/llvm-14 cargo make lint-fix` PASS, `LLVM_SYS_140_PREFIX=/usr/lib/llvm-14 cargo make lint` PASS, `scripts/check-line-count.sh` PASS, and LSP diagnostics show zero errors on `src/codegen`.

## [2026-04-11] Task 26: Runtime Memory + Stdlib
- Added `src/runtime/stdlib.rs` with `string_to_int32 -> RuntimeResult<i32>` mapping parse failures to new `RuntimeError::ParseError`, deterministic-testable RNG via `RandomIntSource`, `random_int32`, interpolation helper `format_interpolated_string`, and `opal_array_slice` range slicing.
- Added `src/runtime/reporting.rs` with `format_runtime_error(&RuntimeError) -> String` that renders miette-style multiline output using diagnostic code/help metadata.
- Extended `src/runtime/errors.rs` with `ParseError { message }` variant (`opalescent::runtime::parse_error`) and stable error code `1_004`.
- Extended `src/runtime/memory.rs` memory strategy docs with cycle handling guidance (`Arc` strong refs + `Weak` back edges), and added `OpalWeakRef<str>/OpalWeakRef<[T]>` weak-upgrade API for cycle-safe graph patterns.
- Wired runtime module/re-exports in `src/runtime.rs` and `src/runtime/mod.rs` for new stdlib/reporting surfaces.
- Added RED→GREEN tests in `src/runtime/tests.rs` for parse success/failure, deterministic random range, interpolation success/mismatch, array slicing success/error, runtime error formatting, and weak-ref upgrade semantics.
- Verification: `LLVM_SYS_140_PREFIX=/usr/lib/llvm-14 timeout 30 cargo test` PASS (402 passed, 3 ignored), `LLVM_SYS_140_PREFIX=/usr/lib/llvm-14 cargo make lint-fix` PASS, `LLVM_SYS_140_PREFIX=/usr/lib/llvm-14 cargo make lint` PASS, `scripts/check-line-count.sh` PASS, `lsp_diagnostics src/runtime` clean (only expected unlinked-file hint for `src/runtime/mod.rs`).

## [2026-04-11] Task 27: Basic Optimization Passes
- Added `src/codegen/optimization.rs` with `OptimizationLevel::{Debug, Release}` and `apply_optimization_passes(module, level)` using inkwell `PassManager<Module>` + `PassManagerBuilder`.
- Wired optimization module in both `src/codegen/mod.rs` and `src/codegen.rs`, plus dedicated test module registration `tests_optimization`.
- Release pipeline now configures O2-class pass setup (`OptimizationLevel::Aggressive`) and integrates SCCP/instruction simplify, DCE-oriented cleanup (aggressive/global DCE + dead store elimination), dead arg/prototype cleanup, CFG simplify, and `AlwaysInliner`.
- Added RED→GREEN tests in new `src/codegen/tests_optimization.rs` validating: O0 vs O2 IR divergence, constant folding to immediate return, dead-store elimination, and inlining of always-inline small function calls.
- Validation: `LLVM_SYS_140_PREFIX=/usr/lib/llvm-14 timeout 30 cargo test` PASS (406 passed, 3 ignored), `LLVM_SYS_140_PREFIX=/usr/lib/llvm-14 cargo make lint-fix` PASS, `LLVM_SYS_140_PREFIX=/usr/lib/llvm-14 cargo make lint` PASS, `scripts/check-line-count.sh` PASS, and LSP diagnostics clean on changed files (with expected rust-analyzer unlinked-file hint for `src/codegen/mod.rs`).

## [2026-04-11] Task 28: Hot Reload Infrastructure
- Added path-based crate wiring in `src/hot_reload.rs` plus `#[path = "hot_reload.rs"] pub mod hot_reload;` in `src/main.rs`, mirroring runtime wiring pattern.
- Added `src/hot_reload/mod.rs` re-export compatibility module (kept intentionally minimal and currently unlinked like existing `codegen/mod.rs` pattern).
- Implemented ABI infrastructure in `src/hot_reload/abi.rs`: `ModuleVTable` narrow C ABI table, `FunctionSignature`, `ExportedFunction`, `PodLayout`, `AbiSignature`, deterministic ABI hashing, `generate_abi_signature`, and `signatures_compatible`.
- Implemented versioning in `src/hot_reload/version.rs`: `ModuleVersion(u32)` newtype with display formatting `v{:04}` and `versioned_module_name(base, version)` producing `logic_v0001.so` style names.
- Implemented host swap infrastructure in `src/hot_reload/loader.rs`: mockable `ModuleLoader` trait, `LoadedModule`, `HostProcess`, `HotReloadError`, and `hot_swap_module` with ABI compatibility gating and unload/load swap sequencing.
- Added TDD suite in `src/hot_reload/tests.rs` covering deterministic ABI hash generation, ABI incompatibility detection, version formatting/name generation, successful module swap, and ABI-incompatible swap rejection with mocked loader only (no `.so` creation).
- Critical workflow note: running `cargo make lint-fix` auto-modified pre-existing `src/codegen/adts.rs` and `src/codegen/functions.rs` into patterns that fail strict `clippy::pattern_type_mismatch`; restored explicit `&Expr` pattern style and re-verified lint/test clean.
- Verification: `LLVM_SYS_140_PREFIX=/usr/lib/llvm-14 timeout 30 cargo test` PASS (411 passed, 3 ignored), `LLVM_SYS_140_PREFIX=/usr/lib/llvm-14 cargo make lint-fix` PASS, `LLVM_SYS_140_PREFIX=/usr/lib/llvm-14 cargo make lint` PASS, `scripts/check-line-count.sh` PASS, and `lsp_diagnostics` clean on changed hot-reload files (only expected unlinked-file hint for `src/hot_reload/mod.rs`).

## [2026-04-11] Task 29: Change Detection & Hot Swap Classification
- Added new hot-reload modules: `src/hot_reload/change_detection.rs`, `classifier.rs`, `dependency_graph.rs`, and `cache.rs` with deterministic, mock-first APIs and `alloc::collections::BTreeMap`-based storage.
- Implemented `FileWatcher` abstraction and `MockFileWatcher` with in-memory queued events only (no real file watcher / file I/O), plus `ChangeDetectionError` and `FileChangeEvent` payloads.
- Implemented `ChangeClassifier::classify(old, new)` with required categories: `HotSwappable` (body-only hash delta when ABI surface unchanged), `RequiresRestart` (function signature delta), `FullRestart` (type layout hash delta), plus `ReloadDecision` helper.
- Implemented `ModuleDependencyGraph` adjacency list (`BTreeMap<String, Vec<String>>`) with `add_dependency(dependent, dependency)` and deterministic transitive invalidation traversal via `transitive_dependents(module)`.
- Implemented `AbiSignatureCache` keyed by module name (`String -> AbiSignature`) with `get/insert/remove/get_or_insert_with` to avoid recomputation.
- Wired module surface through `src/hot_reload.rs` and compatibility re-exports in `src/hot_reload/mod.rs`.
- Added RED→GREEN test coverage in `src/hot_reload/tests.rs` for body-only classification, signature-change classification, type-layout classification, dependency graph traversal, ABI cache hit behavior, and mock watcher polling.
- Verification: `LLVM_SYS_140_PREFIX=/usr/lib/llvm-14 timeout 30 cargo test` PASS (417 passed, 3 ignored), `LLVM_SYS_140_PREFIX=/usr/lib/llvm-14 cargo make lint-fix` PASS, `LLVM_SYS_140_PREFIX=/usr/lib/llvm-14 cargo make lint` PASS, `scripts/check-line-count.sh` PASS, LSP diagnostics clean on changed files (only expected unlinked-file hint for `src/hot_reload/mod.rs`).
