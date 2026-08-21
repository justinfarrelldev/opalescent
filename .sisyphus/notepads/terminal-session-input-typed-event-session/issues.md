# Issues

- 2026-08-15: Librarian background research jobs for Cargo gates and terminal backends failed immediately due unavailable configured model `opencode/gpt-5-nano`; replaced their function with direct Context7/webfetch/websearch official-doc queries. Explore agents remain usable.

## Task 8 formatter/doc preservation - 2026-08-15

- First RED command accidentally passed two Cargo test filters; use a shared substring such as `cargo test proposal` or run separate commands.
- `cargo make lint` is stricter than targeted tests and required new type-form rendering helpers to be `const fn` under `clippy::missing_const_for_fn`.

- 2026-08-20: Task 9 commit hook also enforces the 1050-line cap on central Rust files; moving existing TypeError span helpers and TypeChecker defaults/test-import configuration into focused private modules reduced both files without semantic changes. New private modules require module-level documentation under the strict lint profile.

## Task 10 ABI validation - 2026-08-20

- First `cargo make lint` after Task 10 changes hit rustc incremental ICE `shallow_lint_levels_on(...)`; `cargo clean -p opalescent` cleared the cache, and the subsequent strict lint run passed. Keep this recovery path if Clippy reports the same compiler-known incremental fingerprint issue.

## Task 11 symbol/typechecker scaffold - 2026-08-20

- The strict line-count hook can fail after adding enum variants or checker fields even when tests and lint pass. For Task 11, splitting `Warning::suppression_annotation` into `src/type_system/errors/warning_impl.rs` and `AstTypeMappingError -> TypeError` into `src/type_system/checker/type_mapping_error.rs` brought `errors.rs` and `checker.rs` back under 1050 lines.
- New private helper modules need explicit module docs for `cargo make lint` because the profile denies `clippy::missing_docs_in_private_items`; run `cargo fmt --all -- --check` before lint because rustfmt may split one-line module docs.

## Task 13 line-count/rustfmt gotcha - 2026-08-20

- `src/type_system/checker.rs` and `src/type_system/checker/statements.rs` are exactly at their line-count limits after Task 13 (1050 and 1000 respectively). Rustfmt may expand long one-line calls and re-break the hook; prefer moving shared helper wrappers into focused submodules such as `ref_rules.rs` rather than compacting statements by hand.
- Task 13 follow-up: `ref_rules.rs` is also exactly at 1000 lines after imported-borrow metadata repair. Do not remove helper docs to satisfy line-count because `cargo make lint` denies missing private docs; trim blank separators or extract a helper module instead.


## Task 13 retry review findings - 2026-08-21

- Imported function-return owner checks can pass when parameter borrow metadata is fixed but still miss affine behavior if compiler-registered resource type names are only registered through type imports; add inferred-return owner regressions whenever terminal proposal function imports are tested.
- Local lambda borrow enforcement can regress separately from imported/top-level function enforcement because lambda initializers go through `type_check_let_statement`; test both direct function declarations and let-bound lambda initializers.


## Task 14 using cleanup gotchas - 2026-08-21

- `src/type_system/checker.rs` remains exactly at its 1050-line cap after adding the `using_cleanup` module; keeping it green required removing two blank separator lines inside an existing helper. Avoid adding root checker lines without extracting or trimming first.
- `src/codegen/statements.rs` exceeded its 1250-line cap when `using` lowering was inline; keep using-specific lowering in `src/codegen/statements/using_cleanup.rs`.
- Strict `cargo make lint` enforces `clippy::pattern_type_mismatch`; destructuring borrowed AST/type values in new helpers should use explicit `&Pattern { ref field, .. }` style.
- After changing a borrowed `Stmt::Using` pattern to copy `Span`, stale `*span` dereferences caused a compile error; watch for this when converting patterns for Clippy.


## Task 14 retry review findings - 2026-08-21

- Task 14 can look complete while still being shallow if codegen only piggybacks on lexical scope cleanup; tests must assert emitted cleanup-operation scaffolds, reverse order, early-exit cleanup, and consumed-obligation duplicate suppression.
- Strict lint can reject close-recognition helper code for `clippy::shadow_unrelated` when both callee and target identifiers are destructured as `name`; use distinct binding names such as `callee_name` and `binding_name`.
- Evidence counts should be refreshed after retry tests are added: `cargo test using` is now 12 passed, and full `cargo test` is 1438 passed / 0 failed / 8 ignored with doc-tests 2 passed / 12 ignored.


## Task 14 retry 2 rejection fixes - 2026-08-21

- A test that only checks consumed cleanup after manually calling `env.consume_using_cleanup_obligation` misses the real bug: `codegen_call_expression` can consume before fallible branching. Add propagate-level IR tests when validating explicit close.
- Fallible cleanup scaffolds returning `i8*` are insufficient unless the result is loaded/tested/stored into cleanup flow; assert IR names like `using.cleanup.primary`, `using.cleanup.failed`, and `using.cleanup.primary.select`.
- Transfer logic can become dead if it is only a private predicate tested directly. Keep a production-facing call-shape inspector and codegen transfer branch tied to the registered operation/error/variant tuple.
- Line-count regressions are likely when adding propagation helpers to `functions_call.rs`; extract helper modules before running full verification to avoid rework.


## Task 14 retry 3 rejection findings - 2026-08-21

- Retry 2 fixed propagate but missed guard statement/expression lowering; both paths bypassed `consume_using_cleanup_obligation_after_success`, allowing successful guarded close to be followed by duplicate lexical cleanup.
- A naive guard fix that mutates `CodegenEnv` after success block emission would be wrong for recovered failure/fallback paths, because subsequent lexical cleanup would be skipped globally. Use runtime path flags for guard-controlled exits.
- `type_check_propagate_expr` had a dead `_has_cleanup_transfer` integration; strict review treats computed-and-discarded transfer recognition as no behavior, so remove no-op reads or connect them to real state.
- External librarian research still fails with unavailable `opencode/gpt-5-nano`; rely on local explore/direct searches for this codebase task unless the model configuration changes.

## Task 14 retry 4 guard transfer findings - 2026-08-21

- Retry 3 still missed transfer failures in guard else/fallback paths: success flags alone left `CloseRestorePending` with lexical cleanup authority. Add exact-transfer assertions for both guard statement and expression tests.
- `src/codegen/statements.rs` has too little headroom for inline guard-transfer branches. Move statement-side helper wrappers into `src/codegen/statements/using_cleanup.rs`, but avoid wildcard imports because `cargo make lint` denies them.
