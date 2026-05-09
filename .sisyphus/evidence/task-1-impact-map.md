# Task 1 Impact Map — Guard Error Propagation Baseline

## Scope
Baseline-only slice for `guard-error-propagation`: map current parser/AST/checker/codegen/diagnostic touchpoints and add characterization tests without changing semantics.

## Required reference files reviewed
- `src/parser/statements_guard.rs`
- `src/parser/expressions.rs`
- `src/type_system/checker/statements.rs`
- `src/type_system/checker/expressions_guard.rs`
- `src/type_system/checker/control_flow.rs`
- `src/codegen/statements.rs`
- `tests/integration_e2e/guard_shorthand.rs`
- `tests/integration_e2e/guard_optional_binding.rs`
- supporting baseline test/diagnostic files: `src/type_system/tests.rs`, `src/type_system/errors.rs`, `src/parser/tests.rs`, `src/ast.rs`

## Parser / AST impact
### Statement guard parser
- `src/parser/statements_guard.rs:13` — `parse_guard_statement`
  - parses `guard <expr> [into <success_binding>] else <error_binding> => <indent-body>`
  - statement guards currently store only `success_binding: Option<String>` and `error_binding: String`
  - no typed/mutable success-binding metadata is parsed here yet
- `src/parser/statements_guard.rs:82` — constructs `Stmt::Guard { expression, success_binding, error_binding, else_body, span, id }`
- `src/ast.rs:682` — `Stmt::Guard`
  - current fields: `expression`, `success_binding`, `error_binding`, `else_body`, `span`, `id`
  - no statement-only propagate terminal representation yet

### Expression guard parser
- `src/parser/expressions.rs:130` — `parse_guard_expression`
  - parses `guard <expr> into <name> [: Type] [mutable] else <handler>`
  - expression guards already carry `binding_type` and `is_mutable`
- `src/ast.rs:482` — `Expr::Guard`
  - fields include `binding_name`, `binding_type`, `is_mutable`, `else_branch`

### Ordinary propagate parser
- `src/parser/expressions.rs:241` — `parse_propagate_expression`
  - only accepts `propagate <call_expr>`
  - rejects non-call forms today
- `src/ast.rs:524` — `Expr::Propagate { call, span, id }`

### Parser tests covering current baseline
- `src/parser/tests.rs:324` — expression guard with expression else
- `src/parser/tests.rs:350` — expression guard typed/mutable binding
- `src/parser/tests.rs:376` — ordinary `propagate <call>` parses
- `src/parser/tests.rs:390` — ordinary `propagate` rejects non-call
- `src/parser/tests.rs:498` — parenthesized `if` subject in statement guard
- `src/parser/tests.rs:524` — statement guard shorthand parses
- `src/parser/tests.rs:547` — `guard ... into _ else ...` parses
- `src/parser/tests.rs:2702` — `Stmt::Guard` statement parse smoke test

## Type checker impact
### Current statement-guard path (diverges from expression guard path)
- `src/type_system/checker/statements.rs:151` — `Stmt::Guard` dispatch
- `src/type_system/checker/statements.rs:600` — `type_check_guard_statement`
  - type-checks the guarded expression directly
  - registers the success binding in outer scope before the else body
  - registers the success binding again inside the else-body scope
  - registers `error_binding` as `CoreType::String`
  - this is the current source of the success-binding leak into the error clause and string-typed error baseline
- `src/type_system/checker/statements.rs:796` — wrapper `type_check_guard_stmt_with_return`

### Shared expression-guard path (future refactor target)
- `src/type_system/checker/expressions_guard.rs:35` — `type_check_guard_expr`
  - validates guarded call signature and optional binding annotation
  - checks guard error stack compatibility
  - types the else branch in isolated scope via `type_check_guard_else_with_scope`
  - registers success binding after else-branch handling
- `src/type_system/checker/expressions_guard.rs:158` — `type_check_guard_else_with_scope`
  - pushes/pops guard error context and increments `guard_else_depth`
- `src/type_system/checker/expressions_guard.rs:202` — `type_check_guard_else_branch`
  - statement usage currently accepts `Expr::Propagate` or unit-typed handler expressions
- `src/type_system/checker/control_flow.rs:13` — `GuardUsage`
- `src/type_system/checker/control_flow.rs:23` — `GuardBindingInfo`
- `src/type_system/checker/control_flow.rs:182` — expression guard wrapper `type_check_guard_expression`

### Current diagnostics relevant to the slice
- `src/type_system/errors.rs:529` — `GuardOnNonErrorExpression`
- `src/type_system/errors.rs:543` — `GuardBindingTypeMismatch`
- `src/type_system/errors.rs:563` — `GuardElseIncompatibleError`
- `src/type_system/errors.rs:583` — `GuardChainedErrorMismatch`
- no guard-specific diagnostic exists yet for:
  - success binding hidden inside guard error clause
  - `return err` banned in guard error clause
  - guard-only `propagate err` placement / shorthand guidance

### Existing type-system baseline tests found
- `src/type_system/tests.rs:2113` — current statement guard binds success + string error types
- `src/type_system/tests.rs:2133` — shorthand guard preserves string error binding
- `src/type_system/tests.rs:2177` — named success binding available after guard
- `src/type_system/tests.rs:2197` — `into _` remains valid
- `src/type_system/tests.rs:2887` — statement guard else currently allows ordinary `propagate <call>` when error sets match
- `src/type_system/tests.rs:2950` — statement guard else rejects ordinary propagate when error sets differ

## Codegen impact
- `src/codegen/statements.rs:69` — `Stmt::Guard` lowers through `codegen_guard_statement`
- `src/codegen/statements.rs:428` — `codegen_guard_statement`
  - lowers guard result aggregate
  - allocates success slot before branching and exposes success binding after merge
  - allocates error binding in else block as `CoreType::String`
  - executes else body with current `err` binding in scope
  - no statement-only guard-error propagation terminal exists yet
- `src/codegen/statements.rs:655` — `infer_guard_success_core_type`
  - current success-type inference helper for `guard ... into`

## Formatting / traversal / auxiliary exhaustiveness touchpoints
`Stmt::Guard` is also referenced in:
- `src/parser/captures.rs:194`
- `src/formatter/printer.rs:687`
- `src/formatter/naming.rs:253`
- `src/type_system/rc_analysis.rs:277`

These files are part of the impact surface for later semantic representation changes, but are unchanged in Task 1.

## Integration / characterization surface
- `tests/integration_e2e/guard_shorthand.rs` — end-to-end project for shorthand + named guard success path/runtime markers
- `tests/integration_e2e/guard_optional_binding.rs` — inline compile/run guard shorthand and named-binding coverage
- `tests/integration_e2e/compile_failures.rs` — compile-fail integration pattern for future guard diagnostic tasks

## Characterization gaps filled in Task 1
Added baseline tests in `src/type_system/tests.rs` for:
- current leak of statement-guard success binding into the else clause
- current string-typed error binding assumption in statement guards
- ordinary `propagate <call>` baseline remains valid (existing coverage reused)
- current `return err` behavior in a guard error clause, documented as today’s failure mode

## Baseline conclusions for downstream tasks
1. Statement guards still use a separate checker path instead of the shared expression-guard path.
2. Statement-guard success bindings are incorrectly visible inside the error clause today.
3. Statement-guard error bindings are currently modeled as `string` in both checker and codegen.
4. Ordinary `propagate <call>` already has coverage and must remain unchanged.
5. No dedicated guard-only `propagate err` AST/parser/type-check/codegen path exists yet.
6. No current diagnostic matches the planned future guard error propagation messages; later tasks must add them without disturbing ordinary propagate behavior.
