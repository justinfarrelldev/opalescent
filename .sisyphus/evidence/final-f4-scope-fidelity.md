# Final F4 Scope Fidelity Check

## Audit basis
- Plan reviewed: `.sisyphus/plans/guard-error-propagation.md`
- Notepads reviewed:
  - `.sisyphus/notepads/guard-error-propagation/learnings.md`
  - `.sisyphus/notepads/guard-error-propagation/issues.md`
- Migration evidence reviewed: `.sisyphus/evidence/task-11-migration-checklist.md`
- Core implementation/tests reviewed:
  - `src/parser/statements.rs`
  - `src/parser/statements_guard.rs`
  - `src/parser/expressions.rs`
  - `src/type_system/checker/expressions.rs`
  - `src/type_system/checker/expressions_guard.rs`
  - `src/codegen/statements.rs`
  - `src/parser/tests.rs`
  - `src/type_system/tests.rs`
  - `tests/integration_e2e/guard_stmt.rs`
  - `tests/integration_e2e/guard_optional_binding.rs`

## Required scope checks

### 1) `return err` is rejected in guard error clauses
**Status:** PASS

**Evidence:**
- Type checker rejects direct forwarding in `src/type_system/checker/expressions_guard.rs:522-528` with exact diagnostic:
  - `return err is not valid in a guard error clause; use propagate err to forward the guard error`
- Unit coverage asserts the same rule in `src/type_system/tests.rs:3398-3408`.
- Integration coverage retains compile-fail verification in:
  - `tests/integration_e2e/guard_stmt.rs:235-255`
  - `tests/integration_e2e/guard_optional_binding.rs:390-469`
- Fixture coverage intentionally keeps the banned form only as a compile-fail proof in:
  - `test-projects/guard-stmt-return-err-banned/src/main.op`

### 2) `propagate err` is guard-clause-only final terminal behavior
**Status:** PASS

**Evidence:**
- Parser only creates the dedicated statement form when the current `propagate` token matches the active guard error binding inside an active guard-error clause:
  - `src/parser/statements.rs:831-855`
- The implementation uses statement-only AST `Stmt::PropagateGuardError`, not ordinary `Expr::Propagate`:
  - `src/ast.rs` (`PropagateGuardError` variant)
  - `src/codegen/statements.rs:87-91, 902-949`
  - `src/type_system/checker/expressions_guard.rs:510-518, 603-662`
- Final-statement enforcement lives in `src/type_system/checker/expressions_guard.rs:453-475, 618-625` with exact diagnostic:
  - `propagate err is only valid as the final statement of a guard error clause`
- Parser/unit coverage:
  - `src/parser/tests.rs:643-679`
- Typechecker coverage:
  - `src/type_system/tests.rs:3411-3475`

### 3) Ordinary shorthand `propagate <call>()` remains valid and unchanged
**Status:** PASS

**Evidence:**
- Ordinary propagate parsing remains call-only in `src/parser/expressions.rs:241-263`:
  - parser still requires `propagate` to be followed by a function call expression
  - no global widening to identifier propagation in expression parsing
- Ordinary propagate type checking still routes through `Expr::Propagate` in `src/type_system/checker/expressions.rs:341-343`.
- Regression coverage remains explicit in `src/type_system/tests.rs:3710-3715`:
  - `ordinary propagate <call> behavior should remain valid`

### 4) Long-form only `propagate err` clause is rejected with shorthand guidance
**Status:** PASS

**Evidence:**
- Dedicated diagnostic exists in `src/type_system/checker/expressions_guard.rs:345-353, 466-471, 406-413`:
  - `guard error clause must perform handling before propagating; replace this guard with shorthand propagate <call>() when no handling is needed`
- Unit coverage asserts the exact rule in `src/type_system/tests.rs:3478-3533`.
- Integration compile-fail fixture and assertion exist in:
  - `test-projects/guard-stmt-only-propagate/src/main.op`
  - `tests/integration_e2e/guard_stmt.rs:212-233`

### 5) Tests were migrated rather than skipped/deleted
**Status:** PASS

**Evidence:**
- Migration checklist documents the concrete migrated runtime fixture and rationale in `.sisyphus/evidence/task-11-migration-checklist.md:16-38`.
- The prior broken direct-`return err` runtime fixture was migrated, not removed:
  - `test-projects/fs-path-manipulation/src/path_ops/absolute.op` changed from direct `return err` to `return '{err}'` per checklist.
- Guard-semantic compile-fail fixtures were retained and added as explicit proofs instead of deleting coverage:
  - `guard-stmt-success-binding-leak`
  - `guard-stmt-only-propagate`
  - `guard-stmt-return-err-banned`
- Search over `tests/integration_e2e` found no `#[ignore]` entries in the guard integration area.
- Existing repo-wide skips are limited to unrelated pre-existing/environmental cases (not introduced here), especially Wine and older ecosystem tests; no evidence shows guard tests were skipped/commented/deleted to get green.

## Additional semantic boundary evidence
- Parser test proves bare `propagate err` outside a guard clause stays invalid:
  - `src/parser/tests.rs:672-679`
- Guard-only propagation is lowered via dedicated codegen path that loads the active guard error slot and returns the canonical two-field error aggregate, rather than any `return err` fallback:
  - `src/codegen/statements.rs:902-949`
- Core Rust diagnostics sanity:
  - `src/parser/statements_guard.rs` → no LSP diagnostics
  - `src/type_system/checker/expressions_guard.rs` → no LSP diagnostics
  - integration test files only reported rust-analyzer `unlinked-file` hints, not semantic errors

## Verification results

### Targeted semantic tests
**Status:** PASS

Executed:
```bash
cargo test --lib type_system::tests::test_guard_error_clause_return_err_is_rejected -- --exact
cargo test --lib type_system::tests::test_guard_error_clause_propagate_err_must_be_terminal -- --exact
cargo test --lib type_system::tests::test_guard_error_clause_only_propagate_is_rejected -- --exact
cargo test --lib type_system::tests::test_guard_error_clause_side_effect_then_propagate_err_is_allowed -- --exact
cargo test --features integration guard_stmt
```

Observed results:
- 4/4 targeted unit tests passed.
- `cargo test --features integration guard_stmt` passed:
  - `guard_stmt_only_propagate_project_emits_shorthand_guidance`
  - `guard_stmt_return_err_banned_project_emits_return_err_diagnostic`
  - `guard_stmt_success_binding_leak_project_emits_scope_diagnostic`
  - `guard_stmt_propagate_err_project_compiles_links_and_runs`
  - `guard_stmt_typed_binding_project_compiles_links_and_runs`

### Full gate
**Status:** PASS with known environment flake classification

Executed:
```bash
cargo fmt --all -- --check && cargo clippy --all-targets --all-features -- -D warnings && cargo test --all-features
```

Observed results:
- `cargo fmt --all -- --check` ✅
- `cargo clippy --all-targets --all-features -- -D warnings` ✅
- `cargo test --all-features` ❌ only at known Wine host flake:
  - failing test: `tests::windows_wine::tests::wine_msvc_guard_shorthand`
  - failure mode: Wine host page fault + 120s timeout
  - representative stderr: `wine: Unhandled page fault on write access ... starting debugger...`

Classification:
- This matches the already-documented known external Wine host flake from Task 9 / Task 12 notes and does **not** indicate a scope or semantics regression in the guard error propagation work.
- All targeted guard semantics tests and non-Wine broad-gate coverage passed.

## Scope verdict
**VERDICT: APPROVE**

The delivered implementation matches the requested plan boundaries:
- direct `return err` in guard error clauses is rejected,
- `propagate err` exists only as a guard-clause terminal statement form,
- ordinary `propagate <call>()` remains the normal shorthand path,
- long-form propagate-only clauses are rejected with shorthand guidance,
- and the affected tests/fixtures were migrated instead of skipped or deleted.

The only broad-gate red is the known external Wine crash/timeout already classified in prior evidence, so it is not grounds to reject scope fidelity.