# Final Wave F2 — Code Quality Review (unspecified-high)

**Verdict: APPROVE**

Date: 2026-05-22
Reviewer: Sisyphus-Junior
Scope reviewed:
- `src/codegen/binding_store.rs`
- `src/codegen/statements.rs`
- `src/codegen/functions_call.rs`
- `src/codegen/functions_call/array/helpers.rs`
- `src/codegen/functions_call/array/intrinsics.rs`
- `src/codegen/scope_tracker.rs`
- `tests/integration_e2e/tests.rs`
- `tests/integration_e2e/call_temp_leak_regressions.rs`
- `tests/integration_e2e/rc_store_leak_regressions.rs`
- `tests/integration_e2e/game_of_life_full_memory_stress.rs`
- `scripts/array_memory_sanitizer.sh`

## Required quality checks

### 1) `StoreMode` usage explicit and conservative by default
**PASS**
- `StoreMode` is introduced in `binding_store.rs` with explicit modes `Retain` and `TakeOwned`.
- Default store path remains conservative (`store_binding_overwrite_rc_safe` delegates to `StoreMode::Retain`).
- Array helper default remains conservative (`store_array_binding` delegates to `StoreMode::Retain`).
- Assignment lowering in `statements.rs` defaults to `Retain` and only uses `TakeOwned` for explicit `Expr::Array { .. }` fresh-literal assignment path.

### 2) `TakeOwned` usage whitelist-only at provably fresh lowering sites
**PASS**
- `TakeOwned` use sites are narrow and explicit:
  - `statements.rs`: array-literal assignment path only (`assignment_store_mode`).
  - `functions_call/array/intrinsics.rs`: `push` grow/fallback branches where replacement arrays are freshly allocated before rebinding.
- No broad/global switch to `TakeOwned` detected.
- `reserve`/`clear` store paths remain retain-based through `store_array_binding`, with noop reserve returning an independent array value to avoid aliasing one owner.

### 3) Call-temp cleanup exits and double-free resilience
**PASS (runtime exits); NOTE (codegen-error branch semantics)**
- `functions_call.rs` introduces per-call temporary scope + cleanup records.
- Cleanup is invoked immediately after call lowering via `cleanup_call_argument_temporaries(...)`, including propagate-driven call scenarios (runtime early-return paths execute after this emitted cleanup point).
- Transfer-exempt path is explicit: transferred names are excluded from malloc-string free list and then removed from env bookkeeping, preventing caller-side double free.
- Ownership-transfer whitelist is currently closed/explicit: `call_argument_takes_owned_value(...)` match has no transfer arms (`_ => false`).
- Note: compile/codegen `Err` branch returns without applying `cleanup_result`; this is a compile-time failure path (not a runtime exit regression in tested behavior).

### 4) Test meaningfulness (including five call-temp regressions + RC store regressions)
**PASS**
- `tests/integration_e2e/call_temp_leak_regressions.rs` contains five non-trivial sanitizer-backed cases:
  - normal-return cleanup
  - propagate early-exit cleanup
  - mixed borrowed/owned dispositions
  - later failure cleanup after prior call-temp allocations
  - transfer/no-double-free guard
- `tests/integration_e2e/rc_store_leak_regressions.rs` covers direct assignment, push no-grow/grow, self-overwrite, alias safety, second-class-ref-adjacent overwrite.
- Stress harness remains opt-in, timeout-bounded, and bounded-sampling based (`game_of_life_full_memory_stress.rs`).

### 5) Sanitizer hook wiring deterministic and exact-selector based
**PASS**
- `scripts/array_memory_sanitizer.sh` uses fully-qualified integration selectors with `--exact` for memory verification hooks.
- Includes deterministic ordered test list for RC store and call-temp regressions.
- Stress hook is explicit opt-in via `OPAL_RUN_STRESS=1` and ignored test selector, preserving deterministic default lane.

## Verification evidence run

### Git scope evidence
- Ran: `git diff --stat`
- Ran scoped diff on all required paths.

### Anti-pattern scan evidence
- Ran grep/AST scans on scoped changed files for `TODO|FIXME|HACK|as any|@ts-ignore|unwrap(` equivalents.
- Result: no scoped matches in target files.

### Diagnostics evidence
- Ran `lsp_diagnostics` on all changed Rust/test files in scope.
- Result: **no errors** on changed Rust files; integration test files returned expected rust-analyzer `unlinked-file` hints only.

### Targeted regression execution
Executed each exact selector (all passed):
- `tests::rc_store_leak_regressions::rc_store_direct_assignment`
- `tests::rc_store_leak_regressions::rc_store_push_no_grow`
- `tests::rc_store_leak_regressions::rc_store_push_grow`
- `tests::rc_store_leak_regressions::rc_store_self_overwrite`
- `tests::rc_store_leak_regressions::rc_store_aliased_source_safety`
- `tests::rc_store_leak_regressions::rc_store_second_class_ref_adjacent`
- `tests::call_temp_leak_regressions::call_temp_owned_arg_freed_on_return`
- `tests::call_temp_leak_regressions::call_temp_owned_arg_freed_on_propagate`
- `tests::call_temp_leak_regressions::call_temp_mixed_disposition`
- `tests::call_temp_leak_regressions::call_temp_nested_later_failure_cleanup`
- `tests::call_temp_leak_regressions::call_temp_take_owned_no_double_free`

## Final judgment
Given explicit conservative defaults, constrained `TakeOwned` usage, deterministic sanitizer/test wiring, and passing targeted RC-store + call-temp regressions, the implementation quality bar for this final-wave gate is met.

**Final reviewer verdict: APPROVE.**
