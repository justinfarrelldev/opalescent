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
