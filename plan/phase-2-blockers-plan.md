# Phase 2 Blockers — Detailed Implementation Plan

This plan expands the Phase 2 Blockers in PLAN.md into actionable, test-driven checklists. It prioritizes Error Handling language features while sketching the surrounding blockers for integration points. All items must pass strict linting, compile on no_std-capable cores, and maintain ABI metadata for hot reload.

## 1) Error Handling Language Features (Highest Priority)

### Overview

Add first-class error declarations on functions and lambdas, a guard construct for branching on success/error, and a propagate construct to bubble errors upward when the current function declares compatible error types. Integrate into the type system: function types carry error type sets, type inference must understand these, and diagnostics must be precise with spans and help.

### Syntax (examples)

- Function declaration with errors:
  - `let parse = f(s: string): int32 errors ParseError => { ... }`
  - `public let read_file = f(path: string): string errors IoError, ParseError => { ... }`
- Lambda with errors:
  - `let map_try = f<T, U>(arr: [T], f: f(T): U errors E): [U] errors E => { ... }`
- Guard expression:
  - `guard read_line() into line else handle_line_error(line_error)`
  - `guard parse(line) into value else return default_value` (handler is an expression or block)
- Propagate expression:
  - `let n = propagate string_to_int32(s)`
  - `let data = propagate read_file(path)`

### Grammar additions (parser)

- Function type/declaration:
  - After return type, accept optional: `errors <TypeName> ( , <TypeName> )*`
  - Applies to `Decl::Function` and `Expr::Lambda`
- Guard expression (as an `Expr` or a statement form returning unit):
  - `guard <expr> into <binding_name> [ : <Type> ] [ mutable ] else <handler>`
  - Handler can be an expression or a block `{ ... }`
- Propagate (as an `Expr` wrapping a call expression):
  - `propagate <call_expr>`

### AST changes

- [x] Add `error_types: Vec<String>` to `Decl::Function` and `Expr::Lambda` nodes (stored as names; resolved during type checking).
- [x] Add `Expr::Guard { expr: Box<Expr>, binding_name: String, binding_type: Option<Type>, is_mutable: bool, else_branch: Box<Stmt> | Box<Expr>, span, id }`
  - Representation decision: encode `else_branch` as `LambdaBody`-like union: prefer `Stmt::Block` via `Box<Stmt>` for blocks and wrap single expressions in `Stmt::Expression` for uniformity.
- [x] Add `Expr::Propagate { call: Box<Expr>, span, id }` (require `call` to be an `Expr::Call`).
- [x] Add `errors: Option<Vec<Type>>` to `Type::Function` (AST type nodes) for pretty-printing and doc generation; keep as names at parse level to avoid tight coupling.

### Parser tasks

- [x] Token support: add keywords `errors`, `guard`, `into`, `else`, `propagate`.
- [x] Function/lambda parsing:
  - [x] Extend existing function signature parsing to optionally parse `errors` clause and attach to AST nodes.
  - [x] Accept zero or more error type names, comma-separated.
- [ ] Guard parsing:
  - [ ] Parse the `guard` keyword, an expression, `into` name, optional `: Type`, optional `mutable`, `else`, and then either a block `{ ... }` or an expression.
  - [ ] Produce `Expr::Guard` with captured span for all parts.
- [ ] Propagate parsing:
  - [ ] Parse `propagate` followed by a call expression; error if next node is not a call.
  - [ ] Produce `Expr::Propagate` with inner call.
- [x] Error recovery:
  - [x] If `errors` is present without any types, emit a specific parse error with suggestion.
  - [ ] If guard is missing `into` or `else`, emit error and attempt to synchronize at `;` or block end.

### Type system changes

- [ ] Core types:
  - [ ] Extend `CoreType::Function` to include `error_types: Vec<CoreType>` in addition to `parameters` and `return_type`.
  - [ ] Update `fmt::Display` for function types to print `-> ReturnType errors E1, E2` when non-empty.
- [ ] Environments and symbols:
  - [ ] Function symbols carry error types in their signature for lookup and checking.
- [ ] Inference and unification:
  - [ ] Unify functions only if parameter types, return types, and error type sets are pairwise compatible:
    - Compatibility rule: callee.error_types ⊆ caller.allowed_error_types for calls within a function body without propagate/guard; otherwise require explicit guard/propagate.
    - Equality of error sets when comparing function types directly.
  - [ ] Extend substitution to traverse function error types.
- [ ] Checking `errors` clause names:
  - [ ] Resolve names to `CoreType` (nominal) using the type environment (must exist and be a valid error kind; initially any named type allowed; refine later in ADT phase).

### Expression typing semantics

- [ ] `Expr::Propagate`:
  - [ ] Only valid inside a function (or lambda) that declares `errors`.
  - [ ] Inner callee must be a function call whose function type has error types.
  - [ ] Require: inner.error_types ⊆ current_fn.error_types.
  - [ ] Result type is the inner call's return type; error flow bubbles to caller.
  - [ ] Diagnostics: if not subset or used outside error-declaring function, emit precise error with spans on both the `propagate` and the function signature.
- [ ] `Expr::Guard`:
  - [ ] The guarded expression must be a call (or expression) with error-carrying type context.
  - [ ] Success path binds the success value to `binding_name` with the declared type (if present) or inferred from the expression's success type.
  - [ ] Else branch is type-checked against the error type(s):
    - If multiple error types exist, else must handle a union — for Phase 2, require else type to be compatible with all declared error types (exact or supertype once ADTs land). For now, require identical names; future Phase 3 will allow sum types.
  - [ ] The guard as an expression results in the success type; as a statement in a block, the else branch must produce `unit` unless used in an expression position.
  - [ ] Symbol table: register `binding_name` in the subsequent scope after the guard (then-branch scope).

### Diagnostics & error types

- [ ] Add new `TypeError` variants (with spans):
  - [ ] `UndeclaredErrorType { name, span }`
  - [ ] `PropagateOutsideErrorFunction { span }`
  - [ ] `PropagateErrorMismatch { expected: Vec<CoreType>, found: Vec<CoreType>, span, callee_span }`
  - [ ] `GuardOnNonErrorExpression { span }`
  - [ ] `GuardBindingTypeMismatch { expected: CoreType, found: CoreType, span }`
  - [ ] `GuardElseIncompatibleError { expected: Vec<CoreType>, found: CoreType, span }`
- [ ] Parse errors (with suggestions):
  - [ ] Missing error names after `errors` clause
  - [ ] `propagate` without call expression
  - [ ] `guard` missing `into` or `else`

### Tests (TDD — minimum 3 per checkbox)

- [x] Parser tests:
  - [x] Parse functions/lambdas with 0, 1, many error types; with spacing/commas edge cases.
  - [x] Parse guard with expression and block else branches; with/without type annotation; with mutable. (AST created, parsing pending)
  - [x] Parse propagate around a call; reject non-call usage. (AST created, parsing pending)
- [ ] Type checker tests:
  - [ ] Using `propagate` inside matching error function succeeds; mismatch fails with clear error and spans.
  - [ ] Using `propagate` in a function without `errors` fails.
  - [ ] Guard binds success type correctly; else must match error types; mismatches fail.
  - [ ] Symbol table registers guard binding for subsequent statements in then scope.
- [ ] Integration samples (language-spec/*.op):
  - [ ] Add small .op examples that compile/type-check to exercise guard/propagate.

### Documentation

- [x] Update inline docs for all new AST and type system items (safety/comments, rationale for Vec ordering of error types and deterministic iteration).
- [ ] Update PLAN.md references when items are finished; cross-link to this plan.
- [x] Note architectural decision: use `alloc::collections::BTreeMap` and `Vec` for deterministic order; no_std compatibility preserved (no std-only features in core modules).

---

## 2) Multiple Return Values (Outline — detail when starting task)

- [ ] AST: modify `Type::Function`, `Decl::Function`, and `Expr::Lambda` to carry `return_types: Vec<Type>`; maintain single-return compatibility.
- [ ] Parser: allow comma-separated returns in signatures; `return label: expr, ...` with uniqueness checks.
- [ ] Type system: update `CoreType::Function` to store `return_types: Vec<CoreType>`; check arity and labels.
- [ ] Tests: arity mismatch, label mismatch, single-return back-compat.

## 3) Standard Library Built-ins (Outline)

- [ ] Register built-ins in type environment at checker creation: `print<T>(T): unit`, `take_input(): string`, `string_to_int32(string): int32 errors ParseError`, `random_int32(int32, int32): int32`.
- [ ] Add `TypeEnvironment::register_builtin()` and preload.
- [ ] Tests: type-check calls to built-ins; generic instantiation for `print<T>`; `string_to_int32` + propagate/guard paths.

## 4) Generic Type Parameter Constraints (Outline)

- [ ] AST: extend generic parameter lists to include constraints.
- [ ] Parser: parse constraints in `<T: Constraint1 + Constraint2>` style (exact syntax TBD per spec evolution).
- [ ] Type system: constraint satisfaction on instantiation; solver integration.
- [ ] Tests: satisfaction, violation, inference.

## 5) If Expression Semantics (Outline)

- [ ] Confirm Rust-style value-returning if-expr behavior; else-less if yields `unit`.
- [ ] Parser: ensure expression form emitted where used in expr position.
- [ ] Type system: branch type compatibility; inference for result type.
- [ ] Tests: inference and mismatches.

## 6) Member Access Type Checking (Outline)

- [ ] Implement `Expr::Member` typing: module member lookup, ADT field access (Phase 3), chained access.
- [ ] Tests: success and error cases; module vs struct fields.

## 7) Arithmetic Overflow Detection (Outline)

- [ ] Const-eval overflow checks for +, -, *, shifts.
- [ ] Diagnostics per math.md; tests for constant overflows.

## 8) Division by Zero Detection (Outline)

- [ ] Const-eval checks for `/` and `%` by zero; runtime trap otherwise.
- [ ] Tests per spec.

## 9) Warning System Infrastructure (Outline)

- [ ] Add `Warning` parallel to `TypeError`; collection on `TypeChecker`.
- [ ] Convert unsafe casts to warnings; display with miette.
- [ ] Tests: warning presence and suppression hooks (future).

## 10) Type System Core Plan Synchronization (Docs)

- [ ] Update `type-system-core-plan.md` with the above blockers and cross-references as they complete.

## 11) PLAN.md Integration (Docs)

- [ ] Keep PLAN.md synchronized: check boxes as we complete, add references for error handling syntax and dependencies.

## Acceptance Criteria

- [ ] Parser recognizes `errors`, `guard`, and `propagate` with robust recovery and helpful suggestions.
- [ ] AST carries error type info in functions/lambdas and includes Guard/Propagate nodes.
- [ ] Type system enforces error type compatibility rules; propagate subset checks; guard binding and else handling validated.
- [ ] Comprehensive TDD coverage for parser + type system behaviors (minimum 3 tests per checkbox item in this section).
- [ ] All lints pass; no unwrap/panic/todo; deterministic data structures; no_std-compatible core.
