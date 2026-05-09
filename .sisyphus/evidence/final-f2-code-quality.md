# F2 Code Quality Review — Guard Error Propagation Final Gate

Date: 2026-05-09
Reviewer: Sisyphus-Junior

## Verification Run (fresh)

- `git diff --stat` executed on current workspace state (35 changed files total; includes compiler/test/docs/evidence files).
- `cargo clippy --all-targets --all-features -- -D warnings` executed successfully.
  - Result: `Finished 'dev' profile [unoptimized + debuginfo] target(s) in 0.12s`
  - No warnings/errors emitted under `-D warnings`.

## Scope Discipline Assessment

### Scope target: guard semantics only

**Finding: PASS (scoped, intentional cross-cutting updates only where required by AST/semantic changes).**

Observed compiler changes are concentrated in guard parsing/type-check/codegen plumbing, with required exhaustiveness updates in formatter/capture/analysis layers:

- AST extension for statement-guard metadata and statement-only terminal propagation:
  - `src/ast.rs` (`Stmt::Guard` now carries type/mutability metadata; new `Stmt::PropagateGuardError`).
  - `src/ast/node_impls.rs` exhaustiveness support.
- Parser changes constrained to active guard-error clause context:
  - `src/parser/statements_guard.rs` lines 30-72 and 95-119 (typed/mutable `into` parsing and guard binding stack push/pop).
  - `src/parser/statements.rs` lines 831-856 (special-case parse to `Stmt::PropagateGuardError` only when inside active guard error binding stack).
  - `src/parser.rs` adds `active_guard_error_bindings` parser context stack.
- Type checker guard enforcement and scoping:
  - `src/type_system/checker/expressions_guard.rs` (guard error clause validation path, terminal propagate checks, explicit `return err` rejection, handling requirement).
  - `src/type_system/checker/expressions.rs` lines 476-505 (success-binding hidden in else clause; active guard error binding typed resolution).
  - `src/type_system/checker/statements.rs` lines 154-179, 226-228 (statement guards now use shared guard request flow; statement-only propagate arm checked).
  - `src/type_system/checker/control_flow.rs` introduces `GuardCheckRequest` request object (reduces argument-sprawl maintainability risk).
  - `src/type_system/checker.rs` adds explicit context stacks for pending success bindings and active error bindings.
- Codegen lowering for statement-only `propagate err` uses canonical error ABI:
  - `src/codegen/statements.rs` lines 902-949 (`codegen_guard_error_propagation_statement` loads active guard error slot and returns canonical two-field aggregate via `build_error_aggregate`).
  - `src/codegen/expressions.rs` + `src/codegen/expressions_loop.rs` add explicit active guard error slot stack APIs.

No unrelated redesign themes were found (no package manager/runtime/LSP architecture changes tied into this slice).

## Prohibited Pattern Checks

### 1) Direct `return err` enablement hack

**Finding: PASS (not introduced).**

- Checker explicitly rejects forwarding active guard error through bare return:
  - `src/type_system/checker/expressions_guard.rs` line 524 diagnostic:
    - `"return err is not valid in a guard error clause; use propagate err to forward the guard error"`
- Integration and unit tests assert this remains rejected:
  - `src/type_system/tests.rs` (`test_guard_error_clause_return_err_is_rejected`, `test_guard_statement_return_err_uses_dedicated_guard_diagnostic`).
  - `tests/integration_e2e/guard_optional_binding.rs` (`guard_error_clause_return_err_stays_rejected`).

### 2) Broad/general redesign of normal `Expr::Propagate`

**Finding: PASS (no broad redesign detected).**

- Standard expression propagate path remains in place and unchanged in role:
  - `src/type_system/checker/expressions.rs` line 341 dispatches `Expr::Propagate { ref call, .. }` to `type_check_propagate_expr`.
- New behavior is additive and scoped to statement guard error clauses via distinct AST variant (`Stmt::PropagateGuardError`), not a global rewrite of `Expr::Propagate` semantics.
- Regression control exists:
  - `src/type_system/tests.rs` includes `test_propagate_call_remains_valid_unmodified`.

### 3) Skip-marker / bypass style additions

**Finding: PASS (no suspicious skip hacks added in changed guard paths).**

- Targeted grep showed no new guard-path skip bypass patterns (`#[ignore]` removals occurred in parser tests; no new test bypass introduced in reviewed guard files).

## Maintainability / Readability Review

### Positive maintainability signals

- Request-bundle refactor (`GuardCheckRequest`) reduces long positional argument passing and clarifies intent at call sites (`control_flow.rs`, `expressions.rs`, `statements.rs`).
- Explicit context stacks (`pending_guard_success_bindings`, `active_guard_error_bindings`) make scope rules auditable and localized.
- Parser and codegen both use push/pop stack discipline with debug assertions to protect LIFO invariants (`parser/statements_guard.rs`, `codegen/statements.rs`).

### Complexity risk noted (non-blocking)

- `src/type_system/checker/expressions_guard.rs` is large and now includes extensive statement/expression identifier traversal and guard clause validation logic in one module.
- This is currently acceptable (clippy clean, behavior backed by tests), but future edits should prefer further extraction into smaller helper modules to limit cognitive load.

## Semantic Correctness Checks (file-backed)

- Guard success binding no longer leaks into error clause:
  - checker enforcement in `src/type_system/checker/expressions.rs` lines 476-487.
  - corresponding tests updated/added in `src/type_system/tests.rs`.
- Guard error binding has precise typed behavior in clause context:
  - `src/type_system/checker/expressions.rs` lines 489-503.
- Terminal `propagate err` rule enforced with explicit diagnostics:
  - `src/type_system/checker/expressions_guard.rs` lines 603-625 and 458-472.
- Guard-only `propagate err` without handling explicitly rejected:
  - `src/type_system/checker/expressions_guard.rs` lines 466-471.

## Binary Verdict

**VERDICT: APPROVE**

Rationale: final changed compiler code is maintainable enough for current scope, semantically aligned to guard-specific behavior, and does not introduce prohibited `return err` enablement or broad `Expr::Propagate` redesign. Clippy strict gate is green with fresh run.