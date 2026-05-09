## 2026-05-08T00:35:00Z Task: 1
Baseline characterization confirmed current semantics: statement-guard success binding is visible in else clause, and return-err behavior remains non-finalized for guard-specific semantics. Existing parser/typechecker/codegen references are mapped in task-1 impact evidence.

## 2026-05-09 03:54:55Z
- Clippy `needless_borrowed_reference` is satisfied in `src/type_system/tests.rs` by matching iterator items as `matches!(*error, TypeError::TypeMismatch { .. })` instead of `matches!(error, &TypeError::TypeMismatch { .. })`; this matches the existing deref-first test style already used elsewhere in the repo.
- The required `cargo test --all-features` gate can be blocked by Wine host crashes surfacing as `Err(...)` from `run_under_wine`, not just non-zero exit codes inside a successful `WineRun`.
- Parser red captures for guard-error-propagation slice 2 must use fully qualified `parser::tests::...` names; filtering by the bare function name can return zero tests even when the unit is registered.
- Task 2 RED capture confirmed the targeted parser tests are now runnable and fail at the intended gaps: typed/mutable guard binding currently errors with `MissingToken { expected: "keyword 'else'" }`, and guard-only `propagate err` currently errors with `MissingToken { expected: "dedent" }`.
- The task-2 slice stayed test-only: `src/parser/statements_guard.rs`, `src/parser/expressions.rs`, and `src/ast.rs` were not modified.

## 2026-05-09 Task 3
- New RED coverage was added for guard error-clause semantics in `src/type_system/tests.rs`: scope-leak protection, `return err` rejection, terminal `propagate err` handling, only-propagate rejection, missing handler rejection, and an unchanged ordinary `propagate <call>` control.
- The current baseline still allows the guard success binding to leak into the error clause, so the scope-leak test fails with a plain `()` result instead of the intended diagnostic.
- Ordinary `propagate <call>` remains valid and passes unchanged, which is the key regression-control test for the later green slice.
- Exact selector form that worked for evidence capture was `cargo test --lib type_system::tests::<name> -- --exact`.

## 2026-05-09T04:36:47Z Task 4
- Statement guards now mirror expression guards in the AST and parser: `Stmt::Guard` carries optional `success_binding_type` plus `success_binding_is_mutable`, and `src/parser/statements_guard.rs` accepts `into <name>: <Type> mutable` before `else`.
- Guard error clauses now parse terminal `propagate <active-error-binding>` into the new statement-only `Stmt::PropagateGuardError`, driven by a narrow parser stack in `src/parser.rs`; bare `propagate err` outside an active guard error clause still fails unchanged.
- Exhaustiveness updates were required in formatter, naming, capture analysis, RC analysis, checker statement dispatch, parser AST walkers, and codegen/tests so the new statement variant and extra guard metadata compile cleanly without placeholder paths.
- The parser test for typed/mutable statement guards still uses the normal indented-block representation for `=>` bodies; even a single handler statement remains wrapped in `Stmt::Block`, matching existing statement-guard parsing conventions.


## 2026-05-09T04:49:53Z Task 5
- Statement guards in `src/type_system/checker/statements.rs` now build `GuardBindingInfo` from `success_binding`, `success_binding_type`, `success_binding_is_mutable`, and the statement span, then delegate to the shared `type_check_guard_expr(..., GuardUsage::Statement, ...)` path used by expression guards.
- The old statement-only success-binding registration was removed, so the else/error clause now follows the shared scope behavior instead of pre-registering the success binding before or inside the handler scope.
- To keep Task 5 scoped and preserve current pre-Task-6 error-clause behavior, `src/type_system/checker/expressions_guard.rs` now accepts an optional statement error-binding name and registers the current string-typed `err` binding inside the shared else-scope only for statement guards.
- Expression guard regression coverage stayed green via `cargo test --lib type_system::tests::test_guard_else_expression_allows_matching_success_type -- --exact`.

## 2026-05-09T05:09:50Z Task 6
- Statement guard else-scopes now hide the pending success binding via checker context, so references to the guard result inside the error clause fail with `success binding is not available inside guard error clause` while same-name outer lexical bindings still resolve normally.
- Statement guard error bindings now use the guarded call's actual error type information instead of hardcoded `CoreType::String`; for single-error builtins like `string_to_int32`, the error binding now type-checks as `ParseError` inside the clause and remains unavailable after it.
- Updating the older characterization tests was necessary to keep Task 6 scoped cleanly: the former success-binding leak baseline now asserts the new scope error, and the interim `return err` baseline now records a `ParseError`-to-`unit` mismatch until Task 7 installs the dedicated diagnostic.

## 2026-05-09T06:xx:xxZ Task 7
- Task 7 green required two passes: the initial checker implementation satisfied the dedicated unit diagnostics, but `cargo test --all-features` exposed integration regressions that only appeared in real guard-handler programs.
- Named statement guard error clauses must distinguish between true no-op fallthrough and ordinary local handling. A bare `void` expression still triggers `guard error clause must handle or propagate the bound error`, but side-effecting unit handlers like fixed-message `print(...)` are valid local handling even when they do not read `err` directly.
- Multi-error guard bindings cannot be treated as a purely synthetic non-displayable wrapper in all expression contexts. Existing programs rely on `print('{err}')`/`print('Error: {e}')` inside guard handlers, so interpolation must accept both single error types like `ParseError` and multi-error `GuardErrorContext<...>` values.
- Nested statement guards inside a named guard error clause should not inherit the outer clause's chained-error-set requirement when they fully handle their own errors locally. Keeping the old chained-set check for expression guards was correct, but statement-guard cleanup flows like `_fs_write_text_atomic` only pass when locally handled nested guards are typed independently.
- Reliable integration reruns for this repo need both `--all-features` and fully qualified test names such as `tests::guard_shorthand::guard_shorthand_project_compiles_links_and_runs`; bare names or missing features can report `0 tests` and give a false green.

## 2026-05-09T06:03:52Z Task 8
- `src/codegen/statements.rs` now lowers `Stmt::PropagateGuardError` by loading the active guard error binding from the else-clause scope and returning the canonical two-field error aggregate through the normal propagation ABI, instead of any direct `return err` fallback.
- The new integration coverage in `tests/integration_e2e/guard_optional_binding.rs` proves a long-form guard handler can run side effects (`INNER_GUARD_SEEN=...`) before final `propagate err`, and that the same original parse-error message reaches the outer guard handler unchanged.
- The dedicated compile-fail integration check confirms `return err` remains rejected in the type checker after the codegen change, so Task 8 narrows lowering only and does not broaden source semantics.

## 2026-05-09T06:18:17Z Task 8 follow-up
- Review-driven hardening showed name-based lookup for `propagate err` in codegen was insufficient because `CodegenEnv` keeps a flat variable map. The fix was to track the active guard error slot separately in `CodegenEnv` and have `Stmt::PropagateGuardError` lower from that slot instead of resolving the current `err` binding by name.
- Added `guard_error_clause_shadowed_err_still_propagates_original_guard_error` in `tests/integration_e2e/guard_optional_binding.rs` to prove a local `let err = 'shadowed-local-value'` inside the guard clause does not replace the original guarded error forwarded to the outer handler.

## 2026-05-09T04:53:47-04:00 — Task 9: propagation-only proposal aligned with implemented caveat
- Implemented semantics confirmed in src/type_system/tests.rs are now reflected in error-handler-proposals/propagation-only/proposal.md:
  1. propagate err is only valid as the FINAL top-level statement of an active guard error clause (also rejected outside a guard error clause).
  2. A long-form guard error clause whose body is only propagate err is rejected; the diagnostic suggests the shorthand propagate <call>() form.
  3. Use shorthand propagate <call>() when no per-call handling (logging, metrics, cleanup) is needed.
  4. Direct return err inside a guard error clause is rejected with: "return err is not valid in a guard error clause; use propagate err to forward the guard error". Typed wrapper returns (return new Wrapper: source: err) remain valid because they return a value, not the bare err binding.
- Proposal Handler Set, Syntax Design, Keywords table, Example Applications, and Must NOT Have sections were rewritten so all valid examples follow the implemented rules. The only return-err snippets that remain are explicitly-marked "Rejected" examples used to document the diagnostic.
- cargo test --all-features was green except for the known Wine host flake (wine_msvc_guard_shorthand: unhandled page fault inside Wine itself); see .sisyphus/evidence/task-9-green.txt.

## 2026-05-09T09:05:01Z Task 10
- Added five project-backed guard propagation fixtures under test-projects/: compile-pass `guard-stmt-typed-binding` and `guard-stmt-propagate-err`, plus compile-fail `guard-stmt-success-binding-leak`, `guard-stmt-only-propagate`, and `guard-stmt-return-err-banned`.
- Wired the new fixtures into `tests/integration_e2e/guard_stmt.rs`, reusing the existing project compile/run and CompileError assertion patterns so Task 10 stays in the E2E layer instead of inventing a new harness.
- The current compiler accepts typed statement-guard bindings in real projects and preserves the success binding after the guard, but post-guard mutation of that binding is still rejected as immutable; for this slice the typed-binding pass fixture therefore proves typed+mutable syntax acceptance and success-path availability without turning Task 10 into a compiler-semantics change.

## 2026-05-09T09:05:01Z Task 10
- Added five project-backed guard propagation fixtures under test-projects/: compile-pass `guard-stmt-typed-binding` and `guard-stmt-propagate-err`, plus compile-fail `guard-stmt-success-binding-leak`, `guard-stmt-only-propagate`, and `guard-stmt-return-err-banned`.
- Wired the new fixtures into `tests/integration_e2e/guard_stmt.rs`, reusing the existing project compile/run and CompileError assertion patterns so Task 10 stays in the E2E layer instead of inventing a new harness.
- The current compiler accepts typed statement-guard bindings in real projects and preserves the success binding after the guard, but post-guard mutation of that binding is still rejected as immutable; for this slice the typed-binding pass fixture therefore proves typed+mutable syntax acceptance and success-path availability without turning Task 10 into a compiler-semantics change.

## 2026-05-09T09:18:20Z Task 11
- Migration sweep across test-projects/**, tests/**, and src/type_system/tests.rs found one stale runtime fixture occurrence of direct guard-clause `return err` in fs-path-manipulation helper code.
- Updated `test-projects/fs-path-manipulation/src/path_ops/absolute.op` to return textual error payload (`return '{err}'`) instead of forbidden direct `return err`, preserving helper behavior while aligning with Task 7 semantics.
- Required gates: `cargo test --features integration` passed; `cargo test --all-features` failed only with known Wine host flake `tests::windows_wine::tests::wine_msvc_guard_shorthand` (timeout/page fault under Wine).

## 2026-05-09T05:36:02-04:00 Task 12
- Final gate capture is complete; the only remaining `cargo test --all-features` failure is the known Wine host flake `tests::windows_wine::tests::wine_msvc_guard_shorthand`.
- `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features -- -D warnings`, and the full test gate were executed in order for the final audit snapshot.
- `git status --short`, `git log --oneline -n 30`, and `git diff --stat` were captured for the atomic commit audit evidence.

## 2026-05-09T05:39:27-04:00 Task 12 retry
- Replaced guard-checker positional parameter threading with a bundled request object in `src/type_system/checker/control_flow.rs` and updated the expression/statement call sites.
- Flattened loop-break collection into an explicit stack walk to avoid recursion-only accumulator linting while preserving break-compatibility checks.
- Removed the obsolete guard wrapper helpers in `src/type_system/checker/statements.rs` so the guard API has one consistent entry path.

## 2026-05-09T09:55:58Z Task 12 final gate refresh
- `src/type_system/checker/expressions_guard.rs` satisfied the remaining clippy blockers with a minimal local shape change: a small `GuardElseScopeRequest` bundle replaced the 8-argument helper, the stale `GuardBindingInfo` import was removed, and the guard-error no-op helper became a static recursion helper with unchanged behavior.
- `src/type_system/checker/statements.rs` only needed rustfmt normalization; no semantic statement-dispatch changes were introduced beyond formatting the already-correct guard branch.
- Final verification on the refreshed code state is: `cargo fmt --all -- --check` ✅, `cargo clippy --all-targets --all-features -- -D warnings` ✅, and `cargo test --all-features` red only at the known Wine host crash/timeout in `tests::windows_wine::tests::wine_msvc_guard_shorthand`.

## 2026-05-09T10:16:45Z Task 4 evidence capture
- Targeted parser evidence for `parser::tests::statement_guard_parses_typed_mutable_binding_like_expression_guards`, `parser::tests::statement_guard_allows_guard_only_propagate_err_terminal`, and `parser::tests::bare_propagate_err_outside_guard_remains_invalid` all passed with fully qualified selectors.
- The broad gate still ends at the known Wine host flake `tests::windows_wine::tests::wine_msvc_guard_shorthand`; the failure is environmental page fault/timeout, not a new guard-propagation regression.
