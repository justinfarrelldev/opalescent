
## 2026-04-28T23:05:00Z Task: sample-fixture-op-cat
- `test-projects/op-cat/sample.txt` is intentionally a plain ASCII fixture with exactly three newline-terminated lines: `line one`, `line two`, `line three`.
- Verification was kept simple and deterministic: content log, `wc -l`, and `file` output all point to a stable text fixture.

## 2026-04-28T23:10:00Z Task: task-5-commit-style
- Recent history is best treated as Conventional Commits (`type: subject`) with short lowercase subjects.
- Verbatim examples used for evidence: `chore: append final cleanliness audit notes`, `refactor: fs-markdown-roundtrip scenario`, `green: fs-markdown-roundtrip scenario`.

## 2026-04-28T23:55:00Z Task: t6-build-function-type-error-types
- `build_function_type` now needs the function's `error_types` at every signature-lowering entry point, not just declared functions: user declarations, lambdas, local imports, and imported signatures all participate in the same ABI decision.
- Declaration/lambda AST nodes carry `error_types` as names, so codegen-side lowering can conservatively map them to `CoreType::Generic { name, type_args: [] }` to preserve the existing signature model without expanding T6 into type-resolution work.
- The fast-path split is stable when treated as: no errors => old ABI unchanged; errors + Unit => `{ i8*, i8* }`; errors + scalar/pointer => `{ T, i8* }`; errors + aggregate => early `CodegenError`.
- A focused unit test on `build_function_type` is enough to pin the Unit error ABI shape without pulling in later return/propgate work from T7/T9.

## 2026-04-28T00:00:00Z Task: t7-codegen-return-error-abi
- `codegen_return_statement` now branches on the enclosing LLVM function return type, so the old non-errors lowering stays untouched while 2-field error ABI functions wrap returns through `error_abi` helpers.
- `return err: VariantName` is represented as a normal labeled return payload with `label == "err"`; the lowering extracts the variant name from identifier/member/constructor callee shapes and interns it as the error pointer.
- The reliable T7 IR evidence path in this repo is a focused Rust unit test plus `Module::print_to_string()`, because the CLI does not expose an LLVM IR emit flag.

## 2026-04-28T23:59:00Z Task: t8-emit-function-default-return-forwarded-error
- `emit_function_default_return` now accepts `forwarded_error: Option<PointerValue<'ctx>>`, centralizing the future forwarding hook without changing current callers yet.
- When a forwarded error is provided and the enclosing LLVM return type is the canonical 2-field error ABI struct, the helper now returns `error_abi::build_error_aggregate(...)`; otherwise it preserves the existing runtime-error + unreachable fallback.
- Current callers remain explicit and conservative: the sole propagate call site now passes `None`, which keeps T8 behavior-neutral while preparing T9 to thread the extracted error pointer.

## 2026-04-28T23:20:49Z Task: t9-propagate-forward-inner-error
- `codegen_propagate_expression` can forward an inner error safely by reusing the extracted pointer only when the callee result field is pointer-valued and the current function returns the canonical 2-field error aggregate.
- Keeping the existing `field_count >= 2` gate and success extraction from field `0` preserves prior non-error and success-path behavior while swapping the error-slot lookup over to `error_abi::error_field_index(field_count)`.

## 2026-04-28T23:46:29Z Task: t10-loopcontext-push-pop-loop-emitter
- `emit_loop_body_with_targets` now pushes a `LoopContext` before lowering and pops it after lowering regardless of success or error, so loop-stack symmetry lives in one helper.
- The old direct `break_target`/`continue_target` interception was removed from `emit_loop_body_with_targets`; loop-scoped break/continue branching now reads the active frame from `env.current_loop()` inside local helper recursion in `control_flow.rs`.

## 2026-04-29T01:32:32Z Task: t14-break-continue-loopstack-lambda-isolation
- `src/codegen/statements.rs` now treats `Stmt::Break` and `Stmt::Continue` as active control-flow: break stores any loop-expression payloads into the current loop frame slots, then branches to `break_target`; continue branches to `continue_target`; both reposition the builder on a fresh post-branch block.
- Lambda lowering in `src/codegen/functions_call.rs` now wraps the actual body-emission closure in `env.with_loop_isolated(...)`, and the focused codegen test confirms generated lambda IR does not reference outer loop targets even when the caller env has an active loop frame.

## 2026-04-29T01:41:46Z Task: t14-break-continue-loopstack-lambda-isolation-fix
- The Task 14 acceptance fixture can use a directly-invoked lambda inside the loop body to exercise the real `Expr::Lambda` lowering site while still proving loop-stack isolation for bare `break`.
- Using `set -o pipefail` on the evidence commands is necessary here because the acceptance criteria depend on true non-zero compile exits for both bare top-level `break` and isolated lambda `break`.

## 2026-04-29T01:46:01Z Task: t11-signature-driven-aggregate-detection
- Call dispatch in `src/codegen/functions_call.rs` now uses callee LLVM signature shape (`uses_aggregate_result_dispatch`) rather than any runtime-name allowlist.
- The aggregate path now supports both ABI shapes without whitelist coupling: direct struct-return (`return_type` is struct with `>=2` fields) and sret-style (`void` return + first pointer-to-struct param with `>=2` fields).
- Existing aggregate lowering mechanics were preserved; only the dispatch criterion and struct-return fast branch changed.

## 2026-04-29T01:52:46Z Task: t12-callsite-aggregate-alloca-errors-bearing
- `src/codegen/functions_call.rs` now materializes direct struct-return aggregate calls through an alloca+store+load path, matching the existing hidden-sret aggregate path so downstream guard/propagate consumers always see a real struct value.
- The zero-field fake-unit fallback remains only in the non-aggregate call path; the required grep audit shows no `unit_value()`/`count_fields() == 0` synthesis in `functions_call.rs` for the errors-bearing path.

## 2026-04-29T02:55:00Z Task: t13-op-cat-green-gate
- `let lines = propagate read_lines_sync(...)` needed the same runtime `_len` tracking as guard-bound arrays; preserving that extracted length in `CodegenEnv` and consuming it only for array `let` bindings fixed `.length` on propagated arrays without changing the `.op` source.
- Array parameters now lower consistently as element-pointer plus sidecar `i64` length across declarations, imports, lambdas, and the C `main` wrapper, which fixes real entry `args` forwarding instead of fabricating an always-empty array.
- The op-cat integration harness needed isolated probe target directories and stdout-based handled-error assertions, because `main.op` reports recoverable file errors with `print(...)` rather than stderr.

## 2026-04-29T02:21:00Z Task: t10-loopcontext-push-pop-loop-emitter-rerun
- `emit_loop_body_with_targets` now matches the task-10 wiring directly: it pushes a `LoopContext`, lowers the entire body through `codegen_statement`, then pops after capturing the result so stack symmetry is preserved on both success and error returns.
- Removing the local `Stmt::Block` unwrap keeps loop-control recursion on the standard statement path, which is the intended handoff point for break/continue semantics to read the top loop frame.

## Task 15: Error ABI Documentation
- Documented the canonical error-bearing ABI in README.md and src/codegen/error_abi.rs.
- ABI shapes: {T, i8*} for scalar T, {i8*, i8*} for void.
- Null pointer indicates success; non-null indicates interned variant name.
- Explicitly called out limitations: no aggregate returns, no payload-bearing errors.
- Pointer to runtime/opal_runtime.h:125-141 included in codegen docs.

## 2026-04-28T00:00:00Z Task: f2-reject-codegen-tests-cleanup
- Replaced the reported test-only `unwrap()` sites in `src/codegen/tests.rs` with contextual `expect(...)` messages so any future panic points to the specific allocation or function-value failure.
- Removed the stray `eprintln!("DEBUG TEST: LLVM IR:\n{ir}");` from the Windows dllexport test to keep the suite quiet.
- Verification passed: `lsp_diagnostics` reported no issues in the edited file, `cargo clippy --all-targets` completed, and `cargo test --features integration` passed.

## 2026-04-29T03:20:00Z Task: f4-scope-fidelity-remediation
- T1 alignment for `tests/integration_e2e/op_cat.rs` is strict when the happy path uses one valid input and the error path uses `[valid, missing, valid]` with explicit occurrence checks.
- The robust error-path assertion in this repo keys to `FileNotFoundError` exactly once rather than the generic handled-error prefix, because an additional handled runtime message can appear without violating the T1 missing-file scenario intent.
- Scope cleanup for F4 is safest by reverting non-target drift (`src/codegen/tests.rs`, unrelated plan checkbox flips, and untracked blocker docs) and then reapplying only the minimal allowed F2 test-quality cleanup lines.
