# Task 3 — Assignment/Let Ownership Audit

Date: 2026-05-23

## `assignment_store_mode`
- Exact caller count: **1**
- Caller: `src/codegen/statements.rs:633-649` in the assignment lowering path (`codegen_assignment`), where the RHS is lowered and then passed into `assignment_store_mode(...)`.
- Pattern: the function is used only as the assignment store-mode selector; no other code path calls it.

## `initialize_binding_value`
- Exact caller count: **3**
- Callers:
  1. `src/codegen/statements.rs:236-239` — `codegen_let_statement`
  2. `src/codegen/functions.rs:148-155` — function parameter initialization
  3. `src/codegen/control_flow.rs:421-428` — `for`-loop iteration binding initialization
- Pattern: shared binding initialization helper used for `let`, function params, and loop iteration bindings.

## `codegen_let_statement`
- Decision: **leave unchanged**.
- Reason: the `let` path already distinguishes identifiers from non-identifiers via `retain_new_value = matches!(*initializer_expr, Expr::Identifier { .. })`, so call initializers are already treated as owned/fresh and are not retained again. That is the intended baseline for Task 4 to match on assignment, not something to rework here.

## Predicate choice for Task 4
- Use: **`binding_requires_rc_cleanup(&binding_type)`** (or a direct equivalent that checks the same RC-heap classification).
- Reason: `src/codegen/binding_store.rs:127-129` defines the predicate in terms of `HeapClass::ReferenceCounted`, and the new assignment call arm should be gated on whether the binding type actually needs RC cleanup, not on calls alone.
- This keeps the `TakeOwned` arm limited to RC-bearing bindings and preserves existing non-RC behavior.

## Verification note
- This audit only records the call graph and predicate choice; it does **not** change production behavior.
- The red regression remains expected at this stage and is captured separately in `task-3-no-behavior-change.txt`.
