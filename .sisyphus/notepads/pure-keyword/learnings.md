# Learnings — pure-keyword

## [2026-04-18] Session ses_260fd3b04ffecmESbccQV6hvfs

### Purity Semantics (from language spec)
- `pure` means NO external side effects — I/O and calling impure functions are forbidden
- **Local mutation is FULLY ALLOWED**: `let mutable`, assignments, `.push()` are all valid inside pure functions
- Evidence: `automatic_regions.op:49`, `value_semantics.op:67`, `array_helpers.op`

### Architecture Decision: SymbolInfo-based purity tracking
- Do NOT modify `CoreType::Function` — 131 construction sites across 21 files (too risky)
- Use `SymbolInfo.is_pure: bool` instead — only ~15 construction sites
- Lambda purity is IMPLICIT — lambdas inherit the enclosing function's modifier stack (no separate context push)

### Collection Member Calls Are Safe
- `.push()`, `.pop()`, `.len()` go through `resolve_collection_member_call()`, NOT `type_check_call_expr_impl()`
- They bypass purity check entirely — this is CORRECT, do NOT block them

### Error Pattern
- `TypeError` uses `thiserror` + `miette` with `#[error()]`, `#[diagnostic(code(), help())]`, `#[label()]`
- Diagnostic code for new variant: `opalescent::type_system::purity_violation`

### Complete Impure Builtins List (20 entries)
```
print, take_input,
print_int8, print_int16, print_int32, print_int64,
print_uint8, print_uint16, print_uint32, print_uint64,
print_float32, print_float64, print_string,
random_int8, random_int16, random_int32,
random_uint8, random_uint16, random_uint32, random_uint64
```
- Pure builtins (`string_to_*`, `*_to_string`) must NOT be in this list

### Key File Locations
- `IMPURE_STDLIB_FUNCTIONS` at `call_resolution.rs:22` (currently only 3 entries)
- `current_function_is_pure()` at `checker.rs:923-932`
- `SymbolInfo` struct at `symbol_table.rs:47-65`
- Purity check at `call_resolution.rs:100-114`
- Lambda type-checking at `expressions.rs:~735`
- `type_check_function_declaration` at `declarations.rs:514`

## [2026-04-18] Session task-2-impure-list-expanded

### TDD + purity builtins behavior
- Adding pure-context tests first correctly exposed missing entries in `IMPURE_STDLIB_FUNCTIONS`.
- `print_int32`, `print_string`, and `random_uint64` are all blocked in pure functions once list is expanded.
- `string_to_int32` remains allowed in pure functions when used with proper fallible handling (`propagate` + declared `ParseError`).
- Purity check still emits `TypeError::InvalidOperation` in this task scope; tests must assert that variant.

- Added `SymbolInfo.is_pure` and updated every `SymbolInfo { ... }` initializer to set explicit purity defaults; function symbol registration now derives purity from `FunctionModifier::Pure` so callee purity metadata is available at lookup time.
- 2026-04-18: Updated vscode-extension/syntaxes/opalescent.tmLanguage.json keyword.declaration regex to include `pure` alongside `entry` and `public`. Verified `keyword.control` and `keyword.other` patterns remained unchanged.

## [2026-04-18] Session task-5-transitive-purity-enforcement

- Purity call checks must use `TypeError::PurityViolation` (not `InvalidOperation`) for consistency with dedicated diagnostics.
- Transitive enforcement should only apply to user-defined functions. Guard by checking `environment.lookup_builtin(name)` first so pure stdlib builtins (e.g. `string_to_int32`) remain callable from pure functions.
- Collection method checks in AST-level tests should target existing intrinsic names (`push`, `length`), not parser-level `.len()` syntax assumptions.

## [2026-04-18] Session task-6-pure-entry-rejected

- The correct enforcement point for `pure entry` is `type_check_function_declaration` before `effective_modifiers` are built, so the declaration fails fast before any entry-only modifier injection.
- A direct single-function `Program { declarations: vec![entry_fn], .. }` is the safest test shape for entry-specific declaration validation; `create_entry_program` auto-injects a separate `main` and is not suitable when the tested function itself must be the entrypoint.
- `TypeError::PurityViolation` matching in tests should assert `callee_name == "entry"` for this rule to keep diagnostics specific and future-proof.

## [2026-04-18] Session task-7-lambda-purity-inheritance

- Lambda purity inheritance is validated by embedding an `Expr::Lambda` inside a function `Stmt::Block`; the lambda body can directly call `print("hello")` and is checked under the enclosing function modifier context.
- A lambda inside a `pure` function triggers `TypeError::PurityViolation { callee_name: "print", .. }` even when the lambda itself has no modifiers, confirming implicit inheritance via `function_modifier_stack`.
- The non-pure counterpart passes with `result.is_ok()`, confirming no unintended purity enforcement leaks into impure contexts.
- Added a non-behavioral note at `type_check_lambda_expr` documenting the intentional design: lambdas do not push a separate modifier context and therefore inherit purity from their enclosing function.

## [2026-04-18] Session task-8-integration-tests-through-full-parse-pipeline

- Integration parse syntax for pure functions is `pure <name> = f(...) => ...` (without `let`); using `pure let` causes parser failures (`expected function declaration after modifiers`).
- End-to-end source-string tests for purity should assert `TypeError::PurityViolation` and match `callee_name` for precise impure call attribution (`print`, user impure fn names, `random_int32`).
- `pure entry` is validated at type-check time and remains parse-valid integration syntax; broad `PurityViolation { .. }` matching is stable for this case.
