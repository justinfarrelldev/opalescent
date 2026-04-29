# op-cat Guard/Continue Codegen Fixes

## TL;DR

> **Quick Summary**: Fix two semantic codegen bugs blocking `test-projects/op-cat`: (1) `guard` on user-defined `void errors ...` functions is a no-op because user-defined error-bearing functions today emit a plain LLVM signature ignoring `error_types` entirely; (2) `continue`/`break` inside nested codegen scopes (e.g. guard else inside while) silently no-op instead of branching to the loop targets.
>
> **Deliverables**:
> - Real `errors`-aware ABI for user-defined functions: 2-field `{T, i8*}` (or `{i8*, i8*}` for void) returned by value, mirroring stdlib `fs_void_result_type`, Windows-compatible.
> - LoopContext stack on `CodegenEnv` so `Stmt::Break`/`Stmt::Continue` work uniformly regardless of nesting depth.
> - Eliminate `aggregate_result_runtime_name` whitelist; replace with signature-driven detection.
> - Rust integration test for `test-projects/op-cat` covering happy + error paths.
> - All existing test-projects continue to pass; atomic commits at each task.
>
> **Estimated Effort**: Large
> **Parallel Execution**: YES - 4 waves
> **Critical Path**: T1 (failing test) → T2 (LoopContext) → T3 (ErrorAbi types) → T6 (build_function_type) → T7 (return aggregate) → T11 (whitelist removal) → T13 (regression) → F1-F4 → user okay

---

## Context

### Original Request
Fix two semantic codegen bugs in the Opalescent compiler so `test-projects/op-cat` runs as expected:

1. `guard` on a user-defined `void errors ...` callee is a no-op — the else block is never emitted to IR because the LLVM ABI for user-defined Unit-returning functions ignores `error_types` and the call site receives a zero-field struct sentinel; `codegen_guard_statement`'s `field_count >= 2` check fails and silently falls through.
2. `continue` inside a `guard` else block inside `while` does NOT branch to `while.header`; it falls through to the merge block, causing double-increment on the error path. `Stmt::Break/Continue` are no-ops in `codegen_statement`, and the loop's `continue_target` is only honored for top-level Break/Continue at the loop body level.

User mandates: atomic commits, Windows-compatible ABI, integration-test verification mirroring existing test-project tests, automatic Momus high-accuracy review (no user prompt).

### Interview Summary
**Key Discussions**:
- Verification approach: integration tests under `tests/integration_e2e/` gated by `--features integration`, mirroring `project_execution.rs` and `fs_read_text.rs`.
- Atomic commits required at every point of progress. Commit message style: `type(scope): description`.
- Windows compatibility (MSVC + MinGW) is non-negotiable; chosen ABI must reuse `apply_sret_attr_if_needed` pattern.
- High-accuracy mode YES — Momus review loop runs automatically; do NOT prompt the user.

**Research Findings** (consolidated from explore agents):
- `codegen_guard_statement` lives at `src/codegen/statements.rs:278-448`. Field-count check at L286-296 (`>= 2`); else body lowered at L388 via plain `codegen_statement` (no loop target context); fallback at L431-445 silently skips else when `field_count < 2`.
- `emit_loop_body_with_targets` at `src/codegen/control_flow.rs:543-604`. Inline Break/Continue interception at L567-575 (only for top-level Block statements); default delegation at L602 through `codegen_statement` discards the targets.
- `Stmt::Break | Stmt::Continue | Stmt::Comment => Ok(())` at `src/codegen/statements.rs:89` — intentional no-op; loop lowering owns these today.
- `build_function_type` at `src/codegen/functions_call.rs:704-728`. Unit fast-path at L709-714 returns plain LLVM `void`, IGNORING `error_types`.
- `codegen_function_declaration` at `src/codegen/functions.rs:53-79` builds the `returns` vector from declared return types only and never threads `error_types` into the function type.
- Stdlib reference ABI: `fs_void_result_type = { i8*, i8* }` at `src/codegen/functions_stdlib.rs:44-57`. `apply_sret_attr_if_needed` is platform-aware. Authoritative C ABI: `runtime/opal_runtime.h:125-141` declares `FsVoidResult { void* value; const char* error; }`.
- `codegen_propagate_expression` at `src/codegen/functions_call.rs:346-388` extracts error field at index `(field_count >= 3) ? 2 : 1` and tests via `is_not_null` (pointer) or non-zero (int); on error calls `emit_function_default_return` (currently emits runtime-error + unreachable, NOT a forwarded error aggregate).
- `aggregate_result_runtime_name` whitelist at `functions_call.rs:311-321` — currently only `"read_lines_sync" | "list_directory_sync"`. Needs replacement with signature-driven detection so user-defined error-bearing functions are treated identically.
- `codegen_return_statement` at `src/codegen/control_flow.rs:466-511` handles empty/single/multi return expression lists; no error-aware path. Sum-variant constructor (e.g. `return err: NotFound`) lowers via `src/codegen/adts.rs:402-443` to `{ i64 tag, [i8 x 64] payload }` — orthogonal to function-return ABI.
- AST: `Stmt::Break { span, id }` and `Stmt::Continue { span, id }` defined in `src/ast.rs` (no labels today). Lambdas exist (`Expr::Lambda`) and need loop-stack isolation (snapshot/restore).
- Latent void-errors callers: 1 internal call site (op-cat guard), plus stdlib runtime entry mains in 7 fs test-projects already use the stdlib ABI — low regression risk for the user-defined ABI shift.

### Metis Review
**Identified Gaps** (addressed in this plan):
- Gap 1: Need an explicit ABI specification (struct shape, field order, encoding) — addressed by T3 (ErrorAbi module) + Verification Strategy.
- Gap 2: `return err: VariantName` lowering site for user-defined error-bearing functions — addressed by T7 (codegen_return_statement) + T8 (error variant pointer encoding helper).
- Gap 3: Propagate must FORWARD the inner callee's error to the caller's return aggregate, not call the runtime error helper — addressed by T9 (codegen_propagate_expression rewrite).
- Gap 4: Lambdas inside loops must NOT inherit outer `LoopContext` — addressed by T2 (snapshot/restore via `with_loop_isolated`).
- Gap 5: Whitelist removal must not break `read_lines_sync`/`list_directory_sync` — addressed by T11 (signature-driven dispatch + stdlib whitelist preserved internally for explicit sret intrinsics).

---

## Work Objectives

### Core Objective
Make `test-projects/op-cat` execute correctly (happy path: prints file contents; error path: reports error and continues to next argument without skipping or double-iterating) by giving user-defined `errors`-bearing functions a real LLVM ABI and threading loop control-flow targets through nested codegen scopes — without regressing any existing test-project or unit test.

### Concrete Deliverables
- Modified files (exhaustive):
  - `src/codegen/functions.rs` (thread `error_types` into `build_function_type`)
  - `src/codegen/functions_call.rs` (build_function_type honors error_types; remove/replace `aggregate_result_runtime_name`; rewrite propagate forwarding; remove zero-field void synthesis)
  - `src/codegen/functions_call_helpers.rs` (replace `emit_function_default_return` runtime-error path with error-aware default-return emission)
  - `src/codegen/control_flow.rs` (codegen_return_statement: error-aware aggregate construction; emit_loop_body_with_targets: push/pop LoopContext; remove inline Break/Continue interception once stack drives it)
  - `src/codegen/statements.rs` (Stmt::Break/Continue read top-of-stack; codegen_guard_statement consumes the new ABI uniformly)
  - `src/codegen/expressions.rs` (CodegenEnv definition site, line 40 — LoopContext stack field + helpers added here)
  - `src/codegen/error_abi.rs` (NEW: small module centralizing error-ABI shape, field indices, helpers)
  - `src/codegen/adts.rs` (only if a helper for "convert sum-variant to error-pointer encoding" must live next to constructor lowering; otherwise unchanged)
- New file: `tests/integration_e2e/op_cat.rs` (integration test for op-cat happy + error paths)
- New file: `test-projects/op-cat/expected/` (optional, only if fixture data needed) — most likely just inline test harness suffices.
- Plan file (this file).
- All commits atomic, message style verified against `git log --oneline -20`.

### Definition of Done
- [ ] `cargo build --release` passes.
- [ ] `cargo test --features integration` passes (all existing + new op-cat test).
- [ ] `target/release/opalescent run test-projects/op-cat/src/main.op -- test-projects/op-cat/sample.txt missing.txt test-projects/op-cat/sample.txt` exits 0; stdout contains the file contents twice and one error line for `missing.txt`; loop did NOT skip the second valid file (i.e., output ordering matches expectation: file → error → file).
- [ ] All other `test-projects/*` continue to compile and run (regression sweep T13).
- [ ] LLVM IR for a user-defined `void errors E` function shows a 2-field aggregate return (no plain `void`), verified via `opt -S` or `llvm-dis` on the emitted `.o` (T6 QA scenario).
- [ ] Final review wave F1-F4 all APPROVE; user gives explicit "okay" before plan is marked complete.

### Must Have
- User-defined `errors`-bearing functions emit an LLVM aggregate return matching the canonical shape understood by guard/propagate (`{T, i8*}` or `{i8*, i8*}` for void).
- `LoopContext` stack on `CodegenEnv` driving Break/Continue regardless of nesting depth.
- Lambdas snapshot/restore the loop stack so a `break` inside a nested lambda does NOT branch to the outer loop's break_target.
- Atomic commits at each task; each commit passes `cargo build`.
- Reuse `apply_sret_attr_if_needed` pattern; no new platform-specific branching outside that helper.
- Integration test for op-cat covering happy path AND error path.
- All existing test-projects pass after changes (regression suite).

### Must NOT Have (Guardrails)
- NO refactoring of unrelated codegen modules (e.g., adts.rs sum-variant constructor lowering must remain unchanged unless a small helper is genuinely required).
- NO changes to `.op` language surface (`guard`, `propagate`, `errors`, `break`, `continue` syntax remains identical).
- NO changes to stdlib ABI (`fs_void_result_type`, `read_lines_sync`, etc.) — user functions align TO stdlib, not the other way around.
- NO labeled break/continue support (out of scope; AST has no labels today).
- NO Windows-only or Linux-only branching outside `apply_sret_attr_if_needed`.
- NO premature abstraction: do NOT introduce a generic "control-flow effect" trait or similar; LoopContext stack is the entire mechanism.
- NO over-validation: do NOT add 15 error-path checks where 2 suffice.
- NO documentation bloat: doc comments on new public Rust items only (≤ 3 lines each); zero JSDoc-style ceremony.
- NO commits that fail `cargo build`.
- NO skipping the Momus loop. NO asking the user for review-mode confirmation.
- NO touching `runtime/opal_runtime.h` or runtime C sources (the runtime ABI is already correct).

---

## Verification Strategy (MANDATORY)

> **ZERO HUMAN INTERVENTION** - all verification is agent-executed via Bash/curl/cargo.

### Test Decision
- **Infrastructure exists**: YES (`Cargo.toml:13-16` declares `[features] integration = []`; `tests/integration_e2e/` already populated).
- **Automated tests**: YES (TDD-lite: failing integration test created in T1, made green by subsequent tasks).
- **Framework**: `cargo test --features integration`.
- **TDD shape per task**: T1 emits a RED integration test; T6/T7/T9/T10/T11 progressively shift it toward GREEN; T13 confirms regression-free.

### QA Policy
Every task MUST include agent-executed QA scenarios. Evidence saved to `.sisyphus/evidence/task-{N}-{slug}.{ext}`.

- **Compiler/codegen tasks**: `Bash` — `cargo build --release`; `target/release/opalescent run <fixture>.op -- <args>`; `llvm-dis program.ll || llc program.ll`; assert exit codes, stdout substrings, IR shape.
- **Library/module tasks**: `Bash` — `cargo test --features integration <test_name>` capturing pass/fail and error message.
- **Regression sweep**: `Bash` — iterate all `test-projects/*/opal.toml`, run `opal run`, assert exit 0 and expected output.

### ABI Specification (canonical reference for all tasks)
Aligned with `runtime/opal_runtime.h:125-141` and `src/codegen/functions_stdlib.rs:44-57`:

- **`void errors E1, E2, ...`** → LLVM return type `{ i8*, i8* }` = `{ value_placeholder, error_ptr }`.
  - Success encoding: both fields `null`.
  - Error encoding: field 0 `null`; field 1 `non-null i8*` = pointer to error description (interned string literal of the variant name initially, e.g. `"NotFound"`).
- **`T errors E1, E2, ...`** (T scalar/pointer) → `{ T, i8* }` = `{ value, error_ptr }`.
  - Success: field 0 = real value, field 1 = null.
  - Error: field 0 = `T::default_zero` (`zeroinitializer`), field 1 = non-null error string ptr.
- **`T errors ...`** for aggregate T (struct return) — out of scope for op-cat; document as future work in plan epilogue. (op-cat only needs `void errors`; we do NOT implement the 3-field array variant for user-defined returns in this plan.)
- **Field indices** (consumed by guard/propagate, must remain consistent with current code): error_field_index = `(field_count >= 3) ? 2 : 1`. For our 2-field shape this is `1`. ✅
- **Encoding choice**: pointer-null based (matches existing guard/propagate `is_null`/`is_not_null` checks; matches `runtime/opal_runtime.h`'s `const char* error`).
- **Error string source**: variant name as a `build_global_string_ptr` (e.g., variant `NotFound` → `"NotFound\0"`). This matches the no-payload variant case used in op-cat (`return err: NotFound`); payload-bearing variants are out of scope for this fix and will trigger a typecheck-level "unsupported" error if encountered (see T8).

### Loop Context Specification
```rust
// src/codegen/expressions.rs (CodegenEnv lives here, line 40)
pub struct LoopContext<'ctx> {
    pub continue_target: BasicBlock<'ctx>,
    pub break_target: BasicBlock<'ctx>,
    pub break_slots: Vec<...>,  // existing type if any
    pub break_labels: Vec<String>, // for parity with current emit_loop_body_with_targets state
}

impl CodegenEnv {
    pub fn push_loop(&mut self, ctx: LoopContext<'ctx>);
    pub fn pop_loop(&mut self) -> Option<LoopContext<'ctx>>;
    pub fn current_loop(&self) -> Option<&LoopContext<'ctx>>;
    /// Lambdas: snapshot the stack, run f with empty stack, restore.
    pub fn with_loop_isolated<R>(&mut self, f: impl FnOnce(&mut Self) -> R) -> R;
}
```

---

## Execution Strategy

### Parallel Execution Waves

```
Wave 1 (Foundation - START IMMEDIATELY):
├── T1: Add failing integration test for op-cat (RED) [quick]
├── T2: LoopContext type + CodegenEnv stack helpers (no consumers yet) [unspecified-low]
├── T3: New error_abi.rs module: shape constants + helpers (no consumers yet) [quick]
├── T4: Sample fixture file for op-cat tests (test-projects/op-cat/sample.txt) [quick]
└── T5: Audit & document atomic-commit message style from git log [quick]

Wave 2 (Core ABI + control flow plumbing - depends on Wave 1):
├── T6: build_function_type honors error_types (Unit + scalar fast paths) + codegen_function_declaration threading [deep]
├── T7: codegen_return_statement error-aware aggregate construction (handles `return err: VariantName`) [deep]
├── T8: Replace emit_function_default_return error path with forwarded-error emission [unspecified-high]
├── T9: Rewrite codegen_propagate_expression to forward inner error into caller's aggregate [deep]
├── T10: emit_loop_body_with_targets pushes/pops LoopContext; remove inline Break/Continue interception [unspecified-high]
└── T11: Replace aggregate_result_runtime_name whitelist with signature-driven detection [unspecified-high]

Wave 3 (Integration + regression - depends on Wave 2):
├── T12: Call-site lowering: synthesize aggregate alloca for errors-bearing user-fn calls (drives codegen_guard_statement onto new uniform ABI) [unspecified-high]
├── T13: op-cat regression: tests turn GREEN [unspecified-high]
├── T14: Stmt::Break/Continue read top-of-stack; lambda isolation wired (with_loop_isolated for Expr::Lambda) [unspecified-high]
└── T15: Documentation: README + .op error-handling notes for new ABI [writing]

Wave FINAL (Review - 4 parallel agents, all must APPROVE):
├── F1: Plan compliance audit (oracle)
├── F2: Code quality review (unspecified-high)
├── F3: Real manual QA (unspecified-high)
└── F4: Scope fidelity check (deep)
→ Present results → Get explicit user okay

Critical Path: T1 → T2 → T3 → T6 → T7 → T9 → T10 → T11 → T12 → T13 → F1-F4 → user okay
Parallel Speedup: ~50% vs sequential
Max Concurrent: 5 (Wave 1)
```

### Dependency Matrix

- **T1**: blocks: T13, F3 / blocked-by: none
- **T2**: blocks: T10, T14 / blocked-by: none
- **T3**: blocks: T6, T7, T8, T9, T11, T12 / blocked-by: none
- **T4**: blocks: T1, T13, F3 / blocked-by: none
- **T5**: blocks: every commit / blocked-by: none
- **T6**: blocks: T7, T8, T9, T11, T12 / blocked-by: T3
- **T7**: blocks: T8, T9, T12 / blocked-by: T3, T6
- **T8**: blocks: T9, T12, T13 / blocked-by: T3, T6, T7
- **T9**: blocks: T11, T12, T13 / blocked-by: T3, T6, T7, T8
- **T10**: blocks: T13, T14 / blocked-by: T2
- **T11**: blocks: T12, T13 / blocked-by: T3, T6, T9
- **T12**: blocks: T13 / blocked-by: T3, T6, T7, T8, T9, T11
- **T13**: blocks: F1-F4 / blocked-by: T1, T4, T6, T7, T8, T9, T10, T11, T12
- **T14**: blocks: F1-F4 / blocked-by: T2, T10
- **T15**: blocks: F1-F4 / blocked-by: T6, T7, T8, T9 (documentation reflects landed ABI)
- **F1-F4**: blocks: user-okay / blocked-by: T13, T14, T15

### Agent Dispatch Summary

- **Wave 1**: 5 agents — T1 → `quick`, T2 → `unspecified-low`, T3 → `quick`, T4 → `quick`, T5 → `quick`
- **Wave 2**: 6 agents — T6 → `deep`, T7 → `deep`, T8 → `unspecified-high`, T9 → `deep`, T10 → `unspecified-high`, T11 → `unspecified-high`
- **Wave 3**: 4 agents — T12 → `unspecified-high`, T13 → `unspecified-high`, T14 → `unspecified-high`, T15 → `writing`
- **Wave FINAL**: 4 agents — F1 → `oracle`, F2 → `unspecified-high`, F3 → `unspecified-high`, F4 → `deep`

---

## TODOs

- [x] 1. **Add failing integration test for op-cat (RED)**

  **What to do**:
  - Create `tests/integration_e2e/op_cat.rs`.
  - Mirror the structure of `tests/integration_e2e/project_execution.rs` (use `compile_program(source_path, source_str, temp_dir, &TargetTriple::host())` then `Command::new(binary).args(...).output()`).
  - Test fn 1 `op_cat_happy_path_prints_file_contents` — args = single valid path → assert exit 0 and stdout contains the sample file's contents.
  - Test fn 2 `op_cat_error_path_continues_to_next_arg` — args = `[valid, missing, valid]` → assert exit 0; stdout contains the valid file contents TWICE; stdout/stderr contains the error line ("Error reading file: …" or whatever the .op source prints) ONCE; assert no double-printing of any file.
  - Register the new file in `tests/integration_e2e/main.rs` (or whatever the integration harness root is — verify path with `ls tests/integration_e2e/`).
  - Tests will FAIL initially — that is intentional (RED). Mark with no `#[ignore]`; the grading is "this test must pass after T13".

  **Must NOT do**:
  - Do NOT modify any compiler source.
  - Do NOT add `#[ignore]` — the test must run and fail visibly.
  - Do NOT depend on platform-specific paths; use `tempfile`/`tempdir` patterns from `fs_helpers.rs`.

  **Recommended Agent Profile**:
  - **Category**: `quick` — Reason: Test scaffolding, single new file, mirrors existing pattern with no architectural decisions.
  - **Skills**: [] (no special skills required; standard Rust + cargo test).

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1 (with T2, T3, T4, T5)
  - **Blocks**: T13 (regression confirms RED→GREEN), F3
  - **Blocked By**: None

  **References**:
  - **Pattern References**:
    - `tests/integration_e2e/project_execution.rs` — Use this file's `compile_program` + `Command::new` pattern verbatim (function signatures, assertion style, temp dir handling).
    - `tests/integration_e2e/fs_read_text.rs` — Reference for happy-path + error-path dual test layout.
    - `tests/integration_e2e/fs_helpers.rs` — Use `unique_probe_target_dir`, `prepare_dir`, `cleanup_dir` for fixture isolation.
  - **Fixture References**:
    - `test-projects/op-cat/src/main.op:1-22` — The .op source under test; do NOT modify it in this task.
    - `test-projects/op-cat/sample.txt` — Created by T4; this test depends on T4 finishing. If T4 not yet done, declare path constant pointing where T4 will create it.
  - **Cargo References**:
    - `Cargo.toml:13-16` — `[features] integration = []`; tests gated behind `--features integration`.

  **WHY Each Reference Matters**:
  - `project_execution.rs`: canonical integration-test recipe; deviation will fail the harness setup.
  - `fs_read_text.rs`: shows how to assert on both stdout substrings and exit code in the same test fn.
  - `op-cat/src/main.op`: defines exact expected output strings (e.g., the `print` calls inside `cat_file` and the else block) — assertions must match these strings precisely.

  **Acceptance Criteria**:
  - [ ] File `tests/integration_e2e/op_cat.rs` exists; compiles via `cargo build --tests --features integration`.
  - [ ] `cargo test --features integration op_cat` runs both tests; both FAIL (RED) with assertion messages clearly indicating expected-vs-actual stdout.
  - [ ] Test file does NOT contain `#[ignore]`, `#[should_panic]`, or `unwrap()` swallowing real errors.

  **QA Scenarios**:
  ```
  Scenario: New test file compiles and is discovered by cargo
    Tool: Bash
    Preconditions: Repo on a clean branch off main; test-projects/op-cat/sample.txt may or may not exist (T4 may run in parallel).
    Steps:
      1. Run `cargo build --tests --features integration 2>&1 | tee .sisyphus/evidence/task-1-build.log`
      2. Run `cargo test --features integration op_cat -- --list 2>&1 | tee .sisyphus/evidence/task-1-list.log`
      3. Assert `.sisyphus/evidence/task-1-list.log` contains both `op_cat_happy_path_prints_file_contents` and `op_cat_error_path_continues_to_next_arg`.
    Expected Result: Build exits 0; cargo lists exactly the 2 new tests.
    Failure Indicators: Build fails with rustc errors; cargo does not list one or both tests.
    Evidence: .sisyphus/evidence/task-1-build.log, .sisyphus/evidence/task-1-list.log

  Scenario: Tests fail visibly (RED state expected)
    Tool: Bash
    Preconditions: Test file compiled (above scenario passed); T4 sample.txt exists OR test creates it inline.
    Steps:
      1. Run `cargo test --features integration op_cat 2>&1 | tee .sisyphus/evidence/task-1-redrun.log; echo "exit=$?"`
      2. Assert exit code != 0.
      3. Assert log contains `FAILED` or `assertion failed`.
    Expected Result: Tests run, both FAIL with clear assertion messages naming the missing/incorrect stdout.
    Evidence: .sisyphus/evidence/task-1-redrun.log
  ```

  **Commit**: YES — single commit
  - Message: `test(integration): add failing op-cat integration tests (RED)`
  - Files: `tests/integration_e2e/op_cat.rs`, `tests/integration_e2e/main.rs` (or harness root) if module wiring required.
  - Pre-commit: `cargo build --tests --features integration`

- [x] 2. **LoopContext type + CodegenEnv stack helpers**

  **What to do**:
  - Edit `src/codegen/expressions.rs` where `pub struct CodegenEnv<'context>` is defined (line 40). Confirm via `rg -n 'pub struct CodegenEnv' src/` (expect: `src/codegen/expressions.rs:40`).
  - Add `LoopContext<'ctx>` struct in the same file (above CodegenEnv) with fields: `continue_target: BasicBlock<'ctx>`, `break_target: BasicBlock<'ctx>`, plus any break_slots / break_labels currently used by `emit_loop_body_with_targets` (mirror existing locals so T10 can fully replace them).
  - Add private field `loop_stack: Vec<LoopContext<'ctx>>` to `CodegenEnv` (initialize empty in constructor; locate constructor via `rg -n 'impl.*CodegenEnv' src/codegen/expressions.rs`).
  - Add four methods on `impl<'context> CodegenEnv<'context>`: `push_loop`, `pop_loop`, `current_loop` (returns `Option<&LoopContext<'context>>`), `with_loop_isolated<R>(&mut self, f: impl FnOnce(&mut Self) -> R) -> R` (snapshots `loop_stack` into a temp Vec, replaces with empty, runs `f`, restores).
  - Doc comments ≤ 3 lines each.
  - NO consumers in this task — this is foundation only. T10 / T14 wire them in.

  **Must NOT do**:
  - Do NOT touch `emit_loop_body_with_targets` yet (that is T10's job).
  - Do NOT touch `Stmt::Break/Continue` arms (T10).
  - Do NOT make any field public unless an existing convention requires it (verify by inspecting peer fields).
  - Do NOT add labels to LoopContext (no labeled loops in AST today; out of scope).

  **Recommended Agent Profile**:
  - **Category**: `unspecified-low` — Reason: Pure additive type definition + tiny helper methods; no consumers, low risk.
  - **Skills**: [] — Standard Rust.

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1 (with T1, T3, T4, T5)
  - **Blocks**: T10, T14
  - **Blocked By**: None

  **References**:
  - **Pattern References**:
    - `src/codegen/control_flow.rs:543-604` (`emit_loop_body_with_targets`) — copy the parameter shape (`continue_target`, `break_target`, plus any break_slots locals) into LoopContext so T10 is a mechanical substitution.
    - `src/codegen/expressions.rs:40` (`pub struct CodegenEnv<'context>`) — Add `LoopContext<'ctx>` struct above this line; add `loop_stack` field; add four helper methods on the existing `impl<'context> CodegenEnv<'context>` block. Follow existing convention (`pub fn next_name`, etc.).
  - **API References**:
    - `inkwell::basic_block::BasicBlock<'ctx>` — Lifetime-parameterized; LoopContext must carry the same `'ctx`.

  **WHY Each Reference Matters**:
  - `emit_loop_body_with_targets` defines the EXACT state to migrate; mismatched fields will block T10.
  - `src/codegen/expressions.rs` already has lifetime-parameterized helpers (e.g. `next_name`); LoopContext must follow same lifetime convention to compile.

  **Acceptance Criteria**:
  - [ ] `cargo build --release` passes.
  - [ ] `cargo build --tests --features integration` passes.
  - [ ] `rg 'pub fn push_loop|pub fn pop_loop|pub fn current_loop|pub fn with_loop_isolated' src/codegen/expressions.rs` returns 4 matches.
  - [ ] `rg 'loop_stack' src/codegen/expressions.rs` shows definition + 4 method bodies; ZERO consumer call sites elsewhere (`rg 'loop_stack' src/codegen/ -g '!expressions.rs'` returns 0).

  **QA Scenarios**:
  ```
  Scenario: Compile with new types, no usage
    Tool: Bash
    Preconditions: Wave 1 branch.
    Steps:
      1. Run `cargo build --release 2>&1 | tee .sisyphus/evidence/task-2-build.log; echo "exit=$?"`
      2. Assert exit 0.
      3. Run `rg -n 'pub fn (push_loop|pop_loop|current_loop|with_loop_isolated)' src/codegen/expressions.rs | tee .sisyphus/evidence/task-2-symbols.log`
      4. Assert 4 lines.
    Expected Result: Build clean; all 4 helpers present.
    Evidence: .sisyphus/evidence/task-2-build.log, .sisyphus/evidence/task-2-symbols.log

  Scenario: with_loop_isolated correctly snapshots/restores
    Tool: Bash (via inline rust unit test)
    Preconditions: Add a `#[cfg(test)] mod loop_stack_tests { ... }` block at the bottom of `src/codegen/expressions.rs` with a unit test exercising push → with_loop_isolated(assert empty inside) → pop. (This test module IS part of T2's deliverable.)
    Steps:
      1. Run `cargo test --lib codegen::expressions::loop_stack_tests 2>&1 | tee .sisyphus/evidence/task-2-unittest.log`
      2. Assert exit 0; assert "1 passed" in log.
    Expected Result: Unit test passes proving snapshot/restore semantics.
    Evidence: .sisyphus/evidence/task-2-unittest.log
  ```

  **Commit**: YES — single commit
  - Message: `feat(codegen): add LoopContext stack to CodegenEnv (no consumers)`
  - Files: `src/codegen/expressions.rs` (CodegenEnv + new LoopContext type live here at line 40).
  - Pre-commit: `cargo build --release && cargo test --lib codegen::expressions::loop_stack_tests`

- [x] 3. **error_abi.rs module: shape constants + helpers (no consumers)**

  **What to do**:
  - Create `src/codegen/error_abi.rs`.
  - Register the module in the codegen crate root `src/codegen.rs` (this project does NOT use `mod.rs` — confirm via `ls src/codegen.rs && ls src/codegen/mod.rs 2>&1`; expect the latter to error). Add `pub(super) mod error_abi;` (or `pub(crate) mod error_abi;` if peer modules in `src/codegen.rs` use that — inspect existing `mod` declarations there and match the dominant convention).
  - Define helpers (signatures only listed — bodies follow Inkwell idioms used in `functions_stdlib.rs`):
    - `pub fn build_error_return_type<'ctx>(ctx: &'ctx Context, success_llvm_type: Option<BasicTypeEnum<'ctx>>) -> StructType<'ctx>` — Returns `{ T, i8* }` if `Some(T)`, else `{ i8*, i8* }` for void.
    - `pub fn error_field_index(field_count: u32) -> u32` — Centralizes the `(field_count >= 3) ? 2 : 1` rule already used by guard/propagate; returns 1 for our 2-field shape.
    - `pub fn build_success_aggregate<'ctx>(builder, struct_ty, value: BasicValueEnum<'ctx>) -> StructValue<'ctx>` — Inserts value at index 0, null `i8*` at index 1.
    - `pub fn build_error_aggregate<'ctx>(builder, struct_ty, error_ptr: PointerValue<'ctx>) -> StructValue<'ctx>` — Inserts zeroinitializer at index 0, error_ptr at index 1.
    - `pub fn build_void_success_aggregate<'ctx>(builder, struct_ty) -> StructValue<'ctx>` — both fields null.
    - `pub fn build_void_error_aggregate<'ctx>(builder, struct_ty, error_ptr) -> StructValue<'ctx>` — field 0 null, field 1 error_ptr.
    - `pub fn intern_variant_name<'ctx>(codegen_context, env, variant_name: &str) -> PointerValue<'ctx>` — Wraps `build_global_string_ptr` for variant-name encoding.
  - Each helper ≤ 15 lines body. Doc comments ≤ 3 lines.
  - NO consumers in this task.

  **Must NOT do**:
  - Do NOT modify `build_function_type` (T6).
  - Do NOT modify `codegen_return_statement` (T7).
  - Do NOT introduce a trait abstraction; pure free functions.
  - Do NOT support 3-field aggregate variant (`{T, length, error}`) — that's stdlib's shape; user-defined functions use 2-field only in this plan.

  **Recommended Agent Profile**:
  - **Category**: `quick` — Reason: Pure additive helpers; no behavioral change.
  - **Skills**: [] — Standard Inkwell.

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1 (with T1, T2, T4, T5)
  - **Blocks**: T6, T7, T8, T9, T11, T12
  - **Blocked By**: None

  **References**:
  - **Pattern References**:
    - `src/codegen/functions_stdlib.rs:44-57` — `fs_void_result_type` is the EXACT shape to match for void+errors.
    - `src/codegen/functions_stdlib.rs:285-329` — `read_lines_sync` & `write_text_sync` show how struct types are built and used in calls.
    - `src/codegen/functions_call_helpers.rs:46-49` — `build_global_string_ptr` usage pattern for `intern_variant_name`.
  - **API References**:
    - `runtime/opal_runtime.h:125-141` — Authoritative C ABI (`FsVoidResult { void* value; const char* error; }`).

  **WHY Each Reference Matters**:
  - `functions_stdlib.rs`: misalignment between user ABI and stdlib ABI breaks the unified guard/propagate path.
  - `opal_runtime.h`: any future C-ABI interop with runtime must agree on field order and types.

  **Acceptance Criteria**:
  - [ ] `cargo build --release` passes.
  - [ ] `rg -n 'pub fn (build_error_return_type|error_field_index|build_success_aggregate|build_error_aggregate|build_void_success_aggregate|build_void_error_aggregate|intern_variant_name)' src/codegen/error_abi.rs` returns 7 matches.
  - [ ] No consumer file imports `error_abi::*` yet (other than module declaration in `src/codegen.rs`).

  **QA Scenarios**:
  ```
  Scenario: Build green, helpers exist
    Tool: Bash
    Steps:
      1. Run `cargo build --release 2>&1 | tee .sisyphus/evidence/task-3-build.log`
      2. Run `rg -nc 'pub fn ' src/codegen/error_abi.rs | tee .sisyphus/evidence/task-3-helpers.log`
      3. Assert helper count == 7.
    Expected: Build 0; 7 helpers.
    Evidence: .sisyphus/evidence/task-3-build.log, .sisyphus/evidence/task-3-helpers.log

  Scenario: Module wired into codegen crate root
    Tool: Bash
    Preconditions: `src/codegen.rs` is the crate root (not `src/codegen/mod.rs`, which does not exist in this project).
    Steps:
      1. Run `ls src/codegen.rs && ls src/codegen/mod.rs 2>&1 | tee .sisyphus/evidence/task-3-rootcheck.log` — expect first line success, second line "No such file".
      2. Run `rg -n '^\s*(pub(\(.*\))?\s+)?mod\s+error_abi\b' src/codegen.rs | tee .sisyphus/evidence/task-3-modwire.log`
      3. Assert exactly 1 match in step 2.
    Expected: Module declared in `src/codegen.rs`.
    Evidence: .sisyphus/evidence/task-3-rootcheck.log, .sisyphus/evidence/task-3-modwire.log
  ```

  **Commit**: YES — single commit
  - Message: `feat(codegen): add error_abi module with shape helpers (no consumers)`
  - Files: `src/codegen/error_abi.rs` (new), `src/codegen.rs` (mod declaration added — this project's codegen crate root, NOT `src/codegen/mod.rs`).
  - Pre-commit: `cargo build --release`

- [x] 4. **Sample fixture for op-cat tests**

  **What to do**:
  - Create `test-projects/op-cat/sample.txt` with deterministic content: 3 lines, e.g., `line one\nline two\nline three\n` (no trailing whitespace beyond final newline).
  - Update `test-projects/op-cat/.gitignore` if needed to NOT ignore `sample.txt`.
  - If a `README.md` exists for op-cat, append one line documenting the fixture's purpose; otherwise skip.

  **Must NOT do**:
  - Do NOT modify `src/main.op`.
  - Do NOT include OS-specific line endings (use `\n` only).
  - Do NOT make the file empty — tests need non-empty stdout to assert on.

  **Recommended Agent Profile**:
  - **Category**: `quick` — Reason: Trivial fixture creation.
  - **Skills**: [] — Standard file ops.

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1 (with T1, T2, T3, T5)
  - **Blocks**: T1 (test asserts on this file's contents), T13, F3
  - **Blocked By**: None

  **References**:
  - **Pattern References**:
    - `test-projects/hello-world/` — Reference layout (gitignore + README + src/main.op).
    - `test-projects/op-cat/src/main.op:3-9` — `cat_file` reads a file path arg; this fixture is what gets passed.

  **WHY Each Reference Matters**:
  - The op-cat .op program's `print` of file contents is asserted character-for-character by T1's tests; fixture content must be deterministic and stable.

  **Acceptance Criteria**:
  - [ ] `test-projects/op-cat/sample.txt` exists, non-empty.
  - [ ] `file test-projects/op-cat/sample.txt` reports `ASCII text` (no Windows CRLF).
  - [ ] `wc -l test-projects/op-cat/sample.txt` == 3.

  **QA Scenarios**:
  ```
  Scenario: Fixture content deterministic
    Tool: Bash
    Steps:
      1. Run `cat test-projects/op-cat/sample.txt | tee .sisyphus/evidence/task-4-content.log`
      2. Run `wc -l test-projects/op-cat/sample.txt | tee .sisyphus/evidence/task-4-lines.log`
      3. Run `file test-projects/op-cat/sample.txt | tee .sisyphus/evidence/task-4-filetype.log`
      4. Assert content matches `line one\nline two\nline three\n`; lines == 3; file type "ASCII text".
    Expected: 3-line ASCII fixture.
    Evidence: .sisyphus/evidence/task-4-content.log, .sisyphus/evidence/task-4-lines.log, .sisyphus/evidence/task-4-filetype.log
  ```

  **Commit**: YES — single commit
  - Message: `test(op-cat): add deterministic sample.txt fixture`
  - Files: `test-projects/op-cat/sample.txt`, optionally `.gitignore`, `README.md`.
  - Pre-commit: `git status` (no build needed).

- [x] 5. **Audit & document atomic-commit message style**

  **What to do**:
  - Run `git log --oneline -30` and `git log --pretty=format:'%s' -50`.
  - Identify dominant prefix style (e.g. `feat(scope):`, `fix(scope):`, `test(scope):`, `refactor(scope):`).
  - Save findings to `.sisyphus/evidence/task-5-commit-style.md` with a 5-bullet summary + 3 example commits verbatim.
  - If style is inconsistent, choose Conventional Commits (`type(scope): subject`) and document the chosen convention in the same file. All subsequent task commits MUST follow the documented convention.

  **Must NOT do**:
  - Do NOT modify any source code.
  - Do NOT rewrite git history.
  - Do NOT introduce a commit hook or CI gate (out of scope).

  **Recommended Agent Profile**:
  - **Category**: `quick` — Reason: Read-only audit + tiny markdown output.
  - **Skills**: [] — git CLI.

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1 (with T1, T2, T3, T4)
  - **Blocks**: Every commit in subsequent tasks (style reference)
  - **Blocked By**: None

  **References**:
  - **External References**:
    - https://www.conventionalcommits.org/en/v1.0.0/ — Fallback convention if repo style is ambiguous.

  **WHY Each Reference Matters**:
  - Commit messages must be uniform so F1's audit can grep for "atomic commits per task" without false positives.

  **Acceptance Criteria**:
  - [ ] File `.sisyphus/evidence/task-5-commit-style.md` exists with detected style + 3 verbatim examples.
  - [ ] File documents the EXACT convention to use for tasks 1-15 (e.g., "Use `type(scope): subject`; types in use: feat, fix, refactor, test, chore.").

  **QA Scenarios**:
  ```
  Scenario: Audit produces actionable style guide
    Tool: Bash
    Steps:
      1. Run `git log --oneline -30 | tee .sisyphus/evidence/task-5-recent.log`
      2. Verify `.sisyphus/evidence/task-5-commit-style.md` was created.
      3. Run `wc -l .sisyphus/evidence/task-5-commit-style.md` — assert > 5 lines.
      4. Run `grep -c '^type:' .sisyphus/evidence/task-5-commit-style.md` OR equivalent assertion that the chosen convention is explicitly stated.
    Expected: Style guide present and non-trivial.
    Evidence: .sisyphus/evidence/task-5-recent.log, .sisyphus/evidence/task-5-commit-style.md
  ```

  **Commit**: NO — `.sisyphus/evidence/` is documentation-only, not committed to repo.
  - (The audit file is local; it informs subsequent commits but is not itself a code change.)

---

- [x] 6. **build_function_type honors error_types (Unit + scalar fast paths)**

  **What to do**:
  - In `src/codegen/functions_call.rs:704-728` modify `build_function_type` to accept (or already-receive) the `error_types: &[Type]` slice and:
    - If `error_types.is_empty()`: behavior UNCHANGED (return existing void/scalar/aggregate type).
    - If `!error_types.is_empty()` and return is `Unit`: build `{ i8*, i8* }` via `error_abi::build_error_return_type(ctx, None)`; function fn_type returns this struct by value (NOT void).
    - If `!error_types.is_empty()` and return is a scalar/pointer type T: build `{ T, i8* }` via `error_abi::build_error_return_type(ctx, Some(T_llvm))`; fn_type returns by value.
    - If `!error_types.is_empty()` and return is aggregate (already a struct/array > 1 field): emit a clear typecheck-style error message at codegen time `"errors-bearing functions returning aggregate types are not yet supported (use a wrapper)"` and bail with `CodegenError`. (Out-of-scope to support; explicit guardrail.)
  - In `src/codegen/functions.rs:53-79` (`codegen_function_declaration`): thread `decl.error_types` (or equivalent field — verify exact name via `rg 'error_types' src/ast.rs`) through to `build_function_type`. If the helper signature didn't take `error_types` before, update it and ALL call sites in this PR (use `rg 'build_function_type\(' src/`).
  - Reuse `apply_sret_attr_if_needed` exactly as stdlib does for the new aggregate return; do NOT introduce a new sret path.

  **Must NOT do**:
  - Do NOT change the ABI for functions WITHOUT errors (zero behavioral change for non-errors functions).
  - Do NOT change call-site lowering yet (T8 / T11 jobs).
  - Do NOT introduce a 3-field aggregate path for user-defined functions.
  - Do NOT silently fall back when aggregate-T is encountered — error explicitly.

  **Recommended Agent Profile**:
  - **Category**: `deep` — Reason: Touches central type-construction; cascades to every call-site lowering.
  - **Skills**: [] — Inkwell/LLVM domain knowledge.

  **Parallelization**:
  - **Can Run In Parallel**: NO (foundational; downstream T7-T11 depend on it).
  - **Parallel Group**: Wave 2 (sequenced first; T7, T8, T9 follow once it lands).
  - **Blocks**: T7, T8, T9, T10 (loop integration unrelated but commits in same branch), T11.
  - **Blocked By**: T2, T3.

  **References**:
  - **Pattern References**:
    - `src/codegen/functions_stdlib.rs:44-57` — `fs_void_result_type`: identical shape to emit for `void errors`.
    - `src/codegen/functions_stdlib.rs:285-329` — `apply_sret_attr_if_needed` consumer pattern.
    - `src/codegen/functions_call.rs:704-728` — current `build_function_type` (Unit fast-path L709-714 is the EXACT site to branch on `error_types`).
  - **API References**:
    - `src/codegen/error_abi.rs::build_error_return_type` (introduced in T3).
    - `inkwell::types::FunctionType::fn_type(args, is_var_args)`.
  - **AST References**:
    - `src/ast.rs` `Decl::Function` — confirm field name (`error_types`, `errors`, etc.).

  **WHY Each Reference Matters**:
  - Stdlib already uses this exact ABI; mismatch breaks the unified call-site path in T11.
  - `apply_sret_attr_if_needed` is the only Windows-correct path; bypassing it would re-introduce platform bugs.

  **Acceptance Criteria**:
  - [ ] `cargo build --release` passes.
  - [ ] `cargo test --features integration` passes for ALL non-op_cat tests (no regressions; op_cat tests still RED at this point).
  - [ ] `rg -n 'fn build_function_type' src/codegen/functions_call.rs` shows updated signature with `error_types` param threaded.
  - [ ] Manual: a simple `let f = f(): void errors X => ...` declaration emits IR returning `{ ptr, ptr }` (verify with `cargo run -- some_test_project/src/main.op` and `--emit=llvm-ir` if available, OR a unit test using `Module::print_to_string`).

  **QA Scenarios**:
  ```
  Scenario: Existing test-projects still compile + run
    Tool: Bash
    Preconditions: T2 + T3 merged.
    Steps:
      1. Run `cargo test --features integration -- --skip op_cat 2>&1 | tee .sisyphus/evidence/task-6-regression.log; echo "exit=$?"`
      2. Assert exit 0; assert no FAILED lines.
    Expected Result: All non-op_cat integration tests still pass.
    Evidence: .sisyphus/evidence/task-6-regression.log

  Scenario: Errors-bearing void function emits 2-field struct return type
    Tool: Bash (unit test in src/codegen/functions_call.rs `#[cfg(test)]`)
    Preconditions: Add a unit test that calls build_function_type with a fake AST decl having `error_types = [SomeErr]` and `return = Unit`.
    Steps:
      1. Run `cargo test --lib codegen::functions_call::tests::build_function_type_void_errors 2>&1 | tee .sisyphus/evidence/task-6-unit.log`
      2. Assert exit 0.
      3. Assert log contains `1 passed`.
    Expected: Unit test asserts return type is StructType with 2 pointer fields.
    Evidence: .sisyphus/evidence/task-6-unit.log

  Scenario: Aggregate-T errors-bearing function bails with clear error
    Tool: Bash (small .op file with `let f = f(): SomeStruct errors E => ...` where SomeStruct is a record)
    Preconditions: Add a fixture `.sisyphus/fixtures/aggregate_errors.op`.
    Steps:
      1. Run `./target/release/opalescent .sisyphus/fixtures/aggregate_errors.op 2>&1 | tee .sisyphus/evidence/task-6-aggregate.log; echo "exit=$?"`
      2. Assert exit != 0.
      3. Assert log contains `aggregate types are not yet supported`.
    Expected: Clear actionable error, no panic.
    Evidence: .sisyphus/evidence/task-6-aggregate.log
  ```

  **Commit**: YES
  - Message: `feat(codegen): build_function_type honors error_types for Unit and scalar returns`
  - Files: `src/codegen/functions_call.rs`, `src/codegen/functions.rs`, `.sisyphus/fixtures/aggregate_errors.op` (test fixture).
  - Pre-commit: `cargo build --release && cargo test --features integration -- --skip op_cat`

- [x] 7. **codegen_return_statement: error-aware aggregate construction**

  **What to do**:
  - At `src/codegen/control_flow.rs:466-511` (`codegen_return_statement`):
    - Detect: is the enclosing function's LLVM return type a struct created by `error_abi`? (Inspect `function.get_type().get_return_type()` and check if it's a 2-field struct.)
    - If NO: behavior UNCHANGED.
    - If YES + return expression is a normal value (success): wrap via `error_abi::build_success_aggregate(builder, struct_ty, value)` (or `build_void_success_aggregate` if void). `builder.build_return(Some(&aggregate))`.
    - If YES + return expression is `return err: VariantName` form: detect this AST shape (find existing handling — `rg 'return.*err' src/ -t rust` and `rg 'Stmt::Return' src/codegen/`); compute error pointer via `error_abi::intern_variant_name(ctx, env, variant_name_str)`; wrap via `error_abi::build_error_aggregate(builder, struct_ty, err_ptr)` (or void variant). `builder.build_return(Some(&aggregate))`.
    - If YES + return expression is implicit (no expr, function is `void errors X`): emit `build_void_success_aggregate` and return.
  - Verify: every existing `Stmt::Return` codegen path in non-errors functions stays bit-identical (run `cargo test --features integration` post-change).

  **Must NOT do**:
  - Do NOT support payload-bearing variants (`err: NotFound { reason: "..." }`) yet — emit clear `CodegenError` with message `"payload-bearing error variants not yet supported in user-defined functions"`. Scope guardrail per Metis Gap 5.
  - Do NOT change the AST.
  - Do NOT call into `emit_function_default_return` from this function (that's the propagate path's tool, separate task).

  **Recommended Agent Profile**:
  - **Category**: `deep` — Reason: Branches on function ABI; cross-cuts return-statement codegen.
  - **Skills**: [] — Inkwell + AST.

  **Parallelization**:
  - **Can Run In Parallel**: YES with T8, T9 (different functions, same file but non-overlapping lines).
  - **Parallel Group**: Wave 2.
  - **Blocks**: T13.
  - **Blocked By**: T3, T6.

  **References**:
  - **Pattern References**:
    - `src/codegen/functions_stdlib.rs:285-329` — How stdlib constructs and returns the 2-field aggregate (`read_lines_sync` / `write_text_sync`).
    - `src/codegen/control_flow.rs:466-511` — Existing return-statement lowering.
    - `src/codegen/adts.rs:402-443` — Sum-variant constructor lowering (orthogonal but reference for variant-name extraction).
  - **API References**:
    - `error_abi::build_success_aggregate`, `build_error_aggregate`, `intern_variant_name` (T3).

  **WHY Each Reference Matters**:
  - `functions_stdlib.rs` is the canonical pattern; deviating introduces ABI drift.
  - `control_flow.rs` existing logic must not regress — non-errors path must be byte-identical.

  **Acceptance Criteria**:
  - [ ] `cargo build --release` passes.
  - [ ] All non-op_cat integration tests pass (`cargo test --features integration -- --skip op_cat`).
  - [ ] A function `let f = f(): void errors X => return err: NotFound` compiles, emits IR returning `{ null, %"NotFound\0" }`.
  - [ ] Payload-bearing return (e.g., `return err: NotFound { reason: "x" }`) emits clear `CodegenError`, no panic.

  **QA Scenarios**:
  ```
  Scenario: Non-errors return unchanged
    Tool: Bash
    Steps:
      1. Run `cargo test --features integration -- --skip op_cat 2>&1 | tee .sisyphus/evidence/task-7-regression.log; echo "exit=$?"`
      2. Assert exit 0.
    Expected: Zero regressions.
    Evidence: .sisyphus/evidence/task-7-regression.log

  Scenario: void errors function — success and error returns
    Tool: Bash (with .sisyphus/fixtures/void_errors_return.op containing both return paths)
    Steps:
      1. Run `./target/release/opalescent .sisyphus/fixtures/void_errors_return.op --emit=llvm-ir 2>&1 | tee .sisyphus/evidence/task-7-ir.log` (if --emit flag exists; else use compile + objdump). If no emit flag, use `cargo test --lib codegen::control_flow::tests::return_void_errors`.
      2. Assert IR/log contains both `ret { ptr, ptr } { ptr null, ptr null }` (success) AND a non-null error pointer constant.
    Expected: Both forms emit correctly.
    Evidence: .sisyphus/evidence/task-7-ir.log

  Scenario: Payload-bearing variant rejected with clear error
    Tool: Bash (.sisyphus/fixtures/payload_err.op with `return err: Foo { msg: "x" }`)
    Steps:
      1. Run `./target/release/opalescent .sisyphus/fixtures/payload_err.op 2>&1 | tee .sisyphus/evidence/task-7-payload.log; echo "exit=$?"`
      2. Assert exit != 0; log contains `payload-bearing error variants not yet supported`.
    Expected: Clear actionable error.
    Evidence: .sisyphus/evidence/task-7-payload.log
  ```

  **Commit**: YES
  - Message: `feat(codegen): error-aware return aggregate construction in codegen_return_statement`
  - Files: `src/codegen/control_flow.rs`, fixtures under `.sisyphus/fixtures/`.
  - Pre-commit: `cargo build --release && cargo test --features integration -- --skip op_cat`

- [x] 8. **Replace emit_function_default_return error path with forwarded-error emission**

  **What to do**:
  - In `src/codegen/functions_call_helpers.rs` (`emit_function_default_return`):
    - Current behavior: emits a runtime-error call + `unreachable`. This path is invoked from `codegen_propagate_expression` when the inner callee returned an error.
    - New behavior: if the ENCLOSING function's return type is an `error_abi` 2-field struct, emit a forwarded-error aggregate (`build_error_aggregate(builder, struct_ty, propagated_error_ptr)`) and `builder.build_return(Some(&aggregate))`. The error_ptr to forward is the inner callee's error field, threaded in by T9's propagate rewrite (this task introduces the helper signature change to accept `forwarded_error: PointerValue<'ctx>`).
    - If the enclosing function is NOT errors-bearing: KEEP the existing runtime-error + unreachable behavior (this is the legitimate "propagate from a non-errors function" — already a typecheck error, but defensive codegen should not crash).
  - Update the helper's signature: add `forwarded_error: Option<PointerValue<'ctx>>` parameter.
  - Update all call sites: `rg 'emit_function_default_return\(' src/codegen/`. T9 will pass `Some(error_ptr)`; legacy callers pass `None`.

  **Must NOT do**:
  - Do NOT remove the runtime-error path; preserve as fallback for non-errors-function context.
  - Do NOT inline the forwarding logic into propagate; keep it in this helper for symmetry.
  - Do NOT introduce new branches for aggregate-T (out-of-scope per T6).

  **Recommended Agent Profile**:
  - **Category**: `deep` — Reason: Cross-cutting helper signature change; cascades to all callers.
  - **Skills**: [] — Inkwell.

  **Parallelization**:
  - **Can Run In Parallel**: YES with T7 (different files); coordinate with T9.
  - **Parallel Group**: Wave 2.
  - **Blocks**: T9.
  - **Blocked By**: T3, T6.

  **References**:
  - **Pattern References**:
    - `src/codegen/functions_call_helpers.rs::emit_function_default_return` (current impl).
    - `src/codegen/functions_call.rs:346-388` — propagate caller (will be rewritten in T9 to pass forwarded_error).
    - `src/codegen/error_abi.rs::build_error_aggregate` (T3).

  **WHY Each Reference Matters**:
  - The forwarding semantics MUST match: caller's error field index, struct shape — or runtime UB.

  **Acceptance Criteria**:
  - [ ] Build passes.
  - [ ] All non-op_cat integration tests pass.
  - [ ] `rg 'fn emit_function_default_return' src/` shows updated signature.
  - [ ] All call sites updated to pass `None` (T9 will switch propagate to `Some`).

  **QA Scenarios**:
  ```
  Scenario: Helper signature compiles across all call sites
    Tool: Bash
    Steps:
      1. Run `cargo build --release 2>&1 | tee .sisyphus/evidence/task-8-build.log; echo "exit=$?"`
      2. Assert exit 0.
      3. Run `rg 'emit_function_default_return\(' src/ -c | tee .sisyphus/evidence/task-8-callers.log`
      4. Manually inspect log; verify all callers pass an explicit `None` or `Some(...)`.
    Expected: Clean build; all callers updated.
    Evidence: .sisyphus/evidence/task-8-build.log, .sisyphus/evidence/task-8-callers.log

  Scenario: Existing tests unchanged (None path)
    Tool: Bash
    Steps:
      1. Run `cargo test --features integration -- --skip op_cat 2>&1 | tee .sisyphus/evidence/task-8-regression.log; echo "exit=$?"`
      2. Assert exit 0.
    Expected: No regressions (Some-path not yet exercised; T9 enables it).
    Evidence: .sisyphus/evidence/task-8-regression.log
  ```

  **Commit**: YES
  - Message: `refactor(codegen): emit_function_default_return accepts forwarded_error`
  - Files: `src/codegen/functions_call_helpers.rs`, all call sites discovered via rg.
  - Pre-commit: `cargo build --release && cargo test --features integration -- --skip op_cat`

- [x] 9. **Rewrite codegen_propagate_expression to forward inner error into caller's aggregate**

  **What to do**:
  - At `src/codegen/functions_call.rs:346-388` (`codegen_propagate_expression`):
    - Replace the on-error branch's call to `emit_function_default_return(...)` with `emit_function_default_return(..., Some(extracted_error_ptr))` so the caller's return aggregate is constructed by forwarding the inner error pointer (T8 implements the actual aggregate build).
    - The extraction logic at field index `error_field_index(field_count)` is preserved; only the on-error action changes.
    - Replace the literal `(field_count >= 3) ? 2 : 1` expression with `error_abi::error_field_index(field_count)` so all sites stay in sync (no behavior change; refactor for safety).
    - When the caller is NOT errors-bearing (e.g., propagate inside a non-errors fn — typechecker should reject earlier, but be defensive): fall back to the old runtime-error path by passing `None`.
  - Verify field-count gating remains `>= 2` (untouched).

  **Must NOT do**:
  - Do NOT modify the success path.
  - Do NOT change callees' ABI assumptions.
  - Do NOT remove the field_count gate.

  **Recommended Agent Profile**:
  - **Category**: `deep` — Reason: Subtle semantics; downstream depends on correct error forwarding.
  - **Skills**: [] — Inkwell + control flow.

  **Parallelization**:
  - **Can Run In Parallel**: NO with T8 (signature dependency); coordinate sequentially.
  - **Parallel Group**: Wave 2 (after T8).
  - **Blocks**: T13.
  - **Blocked By**: T3, T6, T8.

  **References**:
  - **Pattern References**:
    - `src/codegen/functions_call.rs:346-388` — current `codegen_propagate_expression`.
    - `src/codegen/error_abi.rs::error_field_index` (T3).
    - `src/codegen/functions_call_helpers.rs::emit_function_default_return` (post-T8).
  - **External References**:
    - LLVM `extractvalue` instruction reference: https://llvm.org/docs/LangRef.html#extractvalue-instruction

  **WHY Each Reference Matters**:
  - Field index drift between propagate and stdlib breaks the unified ABI.
  - `emit_function_default_return` is now the single forwarding path; bypassing it would diverge.

  **Acceptance Criteria**:
  - [ ] Build passes.
  - [ ] All non-op_cat integration tests pass (no regressions in stdlib propagate users like `read_lines_sync` callers).
  - [ ] `rg 'field_count >= 3' src/codegen/` returns ZERO matches (replaced with `error_abi::error_field_index`).
  - [ ] `rg 'emit_function_default_return\(.*Some' src/codegen/functions_call.rs` returns ≥1 match.

  **QA Scenarios**:
  ```
  Scenario: Stdlib propagate users still work
    Tool: Bash (existing fs test-projects use propagate against stdlib aggregates).
    Steps:
      1. Run `cargo test --features integration fs_read_text 2>&1 | tee .sisyphus/evidence/task-9-stdlib.log; echo "exit=$?"`
      2. Run `cargo test --features integration list_directory 2>&1 | tee .sisyphus/evidence/task-9-listdir.log; echo "exit=$?"`
      3. Assert both exit 0.
    Expected: Stdlib propagate paths unaffected by the refactor.
    Evidence: .sisyphus/evidence/task-9-stdlib.log, .sisyphus/evidence/task-9-listdir.log

  Scenario: User-defined errors function propagate forwards error
    Tool: Bash (.sisyphus/fixtures/user_propagate.op: outer fn calls inner fn that returns err; outer should forward, not abort).
    Steps:
      1. Run `./target/release/opalescent .sisyphus/fixtures/user_propagate.op --run 2>&1 | tee .sisyphus/evidence/task-9-userpropagate.log; echo "exit=$?"`
      2. Assert exit 0 (program completes); log contains the outer fn's error-handling output (e.g., a print of "got error: NotFound" from a guard at top level).
    Expected: Error propagates cleanly without runtime abort.
    Evidence: .sisyphus/evidence/task-9-userpropagate.log
  ```

  **Commit**: YES
  - Message: `feat(codegen): propagate forwards inner error into caller aggregate`
  - Files: `src/codegen/functions_call.rs`, fixture.
  - Pre-commit: `cargo build --release && cargo test --features integration -- --skip op_cat`

- [x] 10. **emit_loop_body_with_targets pushes/pops LoopContext; remove inline Break/Continue interception**

  **What to do**:
  - At `src/codegen/control_flow.rs:543-604` (`emit_loop_body_with_targets`):
    - At loop-body entry: build a `LoopContext { continue_target, break_target, ... }` and call `env.push_loop(ctx)`.
    - Lower the body via `codegen_statement` (the standard recursion). Remove the inline interception block at L567-575 — `Stmt::Break/Continue` will now be handled in `codegen_statement` (T14) by reading top-of-stack.
    - At loop-body exit (and on early returns): `env.pop_loop()`. Use a guard pattern (e.g., a small RAII-style scope guard or explicit `pop_loop` before every return path).
  - All four loop forms (`while`, `for`, `loop`, `do-while` if present — verify with `rg 'emit_loop_body_with_targets' src/codegen/`) must push/pop their LoopContext.
  - For lambda bodies encountered during recursion: the standard `codegen_statement` path is unaffected; lambdas are emitted in their own LLVM function via `with_loop_isolated` (T2's helper) — this isolation is invoked at the lambda-codegen site (T14 wires it).

  **Must NOT do**:
  - Do NOT keep the inline Break/Continue interception (it would short-circuit nested cases).
  - Do NOT push/pop in the wrong order (push BEFORE body, pop AFTER body — including all early-return paths).
  - Do NOT special-case nested loops; the stack handles them naturally.

  **Recommended Agent Profile**:
  - **Category**: `deep` — Reason: Control-flow surgery; small but high-stakes.
  - **Skills**: [] — Inkwell control flow.

  **Parallelization**:
  - **Can Run In Parallel**: YES with T6/T7/T8/T9 (different file scope).
  - **Parallel Group**: Wave 2.
  - **Blocks**: T13, T14.
  - **Blocked By**: T2.

  **References**:
  - **Pattern References**:
    - `src/codegen/control_flow.rs:543-604` — current `emit_loop_body_with_targets`.
    - `src/codegen/expressions.rs::CodegenEnv::push_loop`/`pop_loop` (T2 added these on the CodegenEnv `impl` block in expressions.rs:40+).

  **WHY Each Reference Matters**:
  - Symmetry between push/pop is critical; an early-return that skips pop corrupts the stack and breaks subsequent loops.

  **Acceptance Criteria**:
  - [ ] Build passes.
  - [ ] All non-op_cat integration tests pass.
  - [ ] `rg 'continue_target|break_target' src/codegen/control_flow.rs` shows the inline interception block REMOVED (only push/pop and parameter usage remain).
  - [ ] Stack pop count == push count along ALL control paths in the loop body emitter (manual review or unit test counting).

  **QA Scenarios**:
  ```
  Scenario: Existing loop tests still pass
    Tool: Bash
    Steps:
      1. Run `cargo test --features integration fib_iterative 2>&1 | tee .sisyphus/evidence/task-10-fibloop.log; echo "exit=$?"`
      2. Run `cargo test --features integration -- --skip op_cat 2>&1 | tee .sisyphus/evidence/task-10-regression.log; echo "exit=$?"`
      3. Assert both exit 0.
    Expected: All existing loop-using projects compile + run.
    Evidence: .sisyphus/evidence/task-10-fibloop.log, .sisyphus/evidence/task-10-regression.log

  Scenario: Nested loop break/continue works
    Tool: Bash (.sisyphus/fixtures/nested_loop.op: outer for, inner for, continue from inner — assert outer still iterates).
    Steps:
      1. Run `./target/release/opalescent .sisyphus/fixtures/nested_loop.op --run 2>&1 | tee .sisyphus/evidence/task-10-nested.log; echo "exit=$?"`
      2. Assert exit 0; log shows outer iteration count == expected.
    Expected: Nested control flow correct.
    Evidence: .sisyphus/evidence/task-10-nested.log
  ```

  **Commit**: YES
  - Message: `refactor(codegen): drive loop break/continue via LoopContext stack`
  - Files: `src/codegen/control_flow.rs`, fixture.
  - Pre-commit: `cargo build --release && cargo test --features integration -- --skip op_cat`

- [x] 11. **Replace aggregate_result_runtime_name whitelist with signature-driven detection**

  **What to do**:
  - Locate `aggregate_result_runtime_name` at `src/codegen/functions_call.rs:311-321`.
  - Find ALL call sites: `rg 'aggregate_result_runtime_name' src/codegen/`.
  - Replace each call with a check on the callee's LLVM `FunctionType` return type: `if return_type.is_struct_type() && struct.count_fields() >= 2 { /* aggregate result path */ }`.
  - Ensure existing stdlib runtimes (`read_lines_sync`, `list_directory_sync`) still take the aggregate path because their return type is now also a 2+ field struct (they already are — verified in research).
  - Delete the whitelist function once unused (`cargo build` + `rg` to confirm zero references).
  - Remove any zero-field void synthesis at call sites (search `rg 'count_fields\(\) == 0|empty struct' src/codegen/`); if present, replace with `is_void_type()` checks.

  **Must NOT do**:
  - Do NOT preserve the whitelist as a fallback — its existence is the bug.
  - Do NOT change the actual aggregate-result lowering logic, just the dispatch criterion.

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high` — Reason: Mechanical replacement but high-impact (every error-bearing call site now flows through one path).
  - **Skills**: [] — Inkwell.

  **Parallelization**:
  - **Can Run In Parallel**: NO (must come after T6 because callee return types only become 2-field structs once T6 lands).
  - **Parallel Group**: Wave 3 (after Wave 2 stabilizes).
  - **Blocks**: T13.
  - **Blocked By**: T6, T7, T8, T9.

  **References**:
  - **Pattern References**:
    - `src/codegen/functions_call.rs:311-321` — current whitelist.
    - `src/codegen/functions_call.rs:346-388` — propagate (uses field_count check, similar pattern).
  - **API References**:
    - `inkwell::types::AnyTypeEnum::into_struct_type`, `StructType::count_fields`.

  **WHY Each Reference Matters**:
  - The whitelist's removal is precisely what makes user-defined errors-bearing calls work; without it, op-cat's guard remains broken.

  **Acceptance Criteria**:
  - [ ] Build passes.
  - [ ] `rg 'aggregate_result_runtime_name' src/` returns ZERO matches.
  - [ ] All fs test-projects (`fs_read_text`, `list_directory`, `write_text`, etc.) still pass.
  - [ ] All non-op_cat integration tests pass.

  **QA Scenarios**:
  ```
  Scenario: Whitelist function deleted
    Tool: Bash
    Steps:
      1. Run `rg 'aggregate_result_runtime_name' src/ 2>&1 | tee .sisyphus/evidence/task-11-whitelist.log`
      2. Assert log empty (no matches).
    Expected: Function fully removed.
    Evidence: .sisyphus/evidence/task-11-whitelist.log

  Scenario: All stdlib fs propagate users still work
    Tool: Bash
    Steps:
      1. Run `cargo test --features integration -- --skip op_cat 2>&1 | tee .sisyphus/evidence/task-11-regression.log; echo "exit=$?"`
      2. Assert exit 0; assert no FAILED.
    Expected: Zero regressions.
    Evidence: .sisyphus/evidence/task-11-regression.log
  ```

  **Commit**: YES
  - Message: `refactor(codegen): replace aggregate-result whitelist with signature-driven dispatch`
  - Files: `src/codegen/functions_call.rs` (and any other site found via rg).
  - Pre-commit: `cargo build --release && cargo test --features integration -- --skip op_cat`

- [x] 12. **Call-site lowering: synthesize aggregate alloca for errors-bearing user-fn calls**

  **What to do**:
  - At every call-site lowering for user-defined functions (find via `rg 'build_call' src/codegen/functions_call.rs`):
    - If callee's LLVM return type is an `error_abi` 2-field struct: the call returns the struct by value; bind it (or its alloca-stored copy if downstream consumers expect a pointer) to the result SSA value.
    - For `void errors X` callees: do NOT synthesize a zero-field void value (current bug); instead, the call's return is the 2-field struct, and the success-vs-error dispatch flows through the existing `field_count >= 2` propagate/guard paths.
  - Audit and update any place that currently produces a "fake" Unit value for void-returning calls (`rg 'unit_value\(\)|build_struct_value\(\[\]' src/codegen/`).
  - Do NOT change call-site lowering for non-errors functions.

  **Must NOT do**:
  - Do NOT break the call-site for non-errors void functions (they still return LLVM `void`).
  - Do NOT introduce sret manually — `apply_sret_attr_if_needed` handles it.

  **Recommended Agent Profile**:
  - **Category**: `deep` — Reason: Cross-cutting; touches every call instruction.
  - **Skills**: [] — Inkwell.

  **Parallelization**:
  - **Can Run In Parallel**: YES with T11 (different concerns; coordinate).
  - **Parallel Group**: Wave 3.
  - **Blocks**: T13.
  - **Blocked By**: T6, T11.

  **References**:
  - **Pattern References**:
    - `src/codegen/functions_call.rs::*build_call*` (find via rg).
    - `src/codegen/functions_stdlib.rs:285-329` — How stdlib call sites consume `{ T, ptr }` returns.

  **WHY Each Reference Matters**:
  - Call-site shape MUST match the new function-type shape from T6; mismatch is undefined behavior.

  **Acceptance Criteria**:
  - [ ] Build passes.
  - [ ] All non-op_cat integration tests pass.
  - [ ] `rg 'unit_value\(\)' src/codegen/functions_call.rs` shows ZERO synthesis at void-call lowering (or only in legitimate non-errors paths).

  **QA Scenarios**:
  ```
  Scenario: void errors call site returns the 2-field struct
    Tool: Bash (.sisyphus/fixtures/void_errors_call.op: caller invokes `void errors X` callee with guard).
    Steps:
      1. Run `./target/release/opalescent .sisyphus/fixtures/void_errors_call.op --run 2>&1 | tee .sisyphus/evidence/task-12-callsite.log; echo "exit=$?"`
      2. Assert exit 0; output matches expected guard branches.
    Expected: Guard works on user void-errors fn.
    Evidence: .sisyphus/evidence/task-12-callsite.log

  Scenario: Non-errors void calls unchanged
    Tool: Bash (hello-world test-project).
    Steps:
      1. Run `cargo test --features integration hello_world 2>&1 | tee .sisyphus/evidence/task-12-helloworld.log; echo "exit=$?"`
      2. Assert exit 0.
    Expected: Plain void calls unaffected.
    Evidence: .sisyphus/evidence/task-12-helloworld.log
  ```

  **Commit**: YES
  - Message: `feat(codegen): consume errors-bearing aggregate at call sites`
  - Files: `src/codegen/functions_call.rs`, fixture.
  - Pre-commit: `cargo build --release && cargo test --features integration -- --skip op_cat`

- [x] 13. **op-cat regression: tests turn GREEN**

  **What to do**:
  - Re-run the integration tests authored in T1: `cargo test --features integration op_cat`.
  - If both tests pass: nothing to do here beyond verification.
  - If a test fails: triage the failure, identify which earlier task missed a case, fix it in that task's file (or open a small follow-up commit if the gap is genuinely cross-task), and re-run.
  - Do NOT modify T1's test file unless an assertion was empirically wrong (e.g., the .op program prints with a different prefix than assumed); record any test-only edits with rationale.

  **Must NOT do**:
  - Do NOT modify `test-projects/op-cat/src/main.op` — the .op program is the spec.
  - Do NOT add `#[ignore]`.
  - Do NOT commit a passing-but-vacuous assertion (e.g. weakening `assert_eq!` to `assert!(true)`).

  **Recommended Agent Profile**:
  - **Category**: `deep` — Reason: May require triage across multiple codegen layers.
  - **Skills**: [] — General codegen + test debugging.

  **Parallelization**:
  - **Can Run In Parallel**: NO (gating verification).
  - **Parallel Group**: Wave 3 final.
  - **Blocks**: T15, F1-F4.
  - **Blocked By**: T1, T7, T9, T10, T11, T12, T14.

  **References**:
  - **Test References**:
    - `tests/integration_e2e/op_cat.rs` (T1).
    - `test-projects/op-cat/src/main.op` (the spec).

  **WHY Each Reference Matters**:
  - The .op source is the canonical behavioral spec; tests assert on its print output verbatim.

  **Acceptance Criteria**:
  - [ ] `cargo test --features integration op_cat` exits 0 with both tests passing.
  - [ ] `cargo test --features integration` (full suite) exits 0 — zero regressions.

  **QA Scenarios**:
  ```
  Scenario: op-cat tests green
    Tool: Bash
    Steps:
      1. Run `cargo test --features integration op_cat 2>&1 | tee .sisyphus/evidence/task-13-opcat.log; echo "exit=$?"`
      2. Assert exit 0; assert log contains `2 passed; 0 failed`.
    Expected: Both tests pass.
    Evidence: .sisyphus/evidence/task-13-opcat.log

  Scenario: Full integration suite green
    Tool: Bash
    Steps:
      1. Run `cargo test --features integration 2>&1 | tee .sisyphus/evidence/task-13-fullsuite.log; echo "exit=$?"`
      2. Assert exit 0; assert no FAILED lines.
    Expected: Zero regressions.
    Evidence: .sisyphus/evidence/task-13-fullsuite.log

  Scenario: Manual run of compiled op-cat binary
    Tool: Bash
    Steps:
      1. Run `cd test-projects/op-cat && ../../target/release/opalescent run src/main.op -- sample.txt nonexistent.txt sample.txt 2>&1 | tee ../../.sisyphus/evidence/task-13-manualrun.log; echo "exit=$?"`
      2. Assert exit 0; log shows sample.txt contents twice and an error message for nonexistent.txt once.
    Expected: Behavior matches spec end-to-end.
    Evidence: .sisyphus/evidence/task-13-manualrun.log
  ```

  **Commit**: YES (only if any fix-up edits were made; otherwise tag the GREEN state with an empty commit or a tag).
  - Message: `test(integration): op-cat tests pass after codegen fixes (GREEN)` (or follow-up `fix(codegen): ...` if a real bug was triaged).
  - Files: any fix-up files only.
  - Pre-commit: `cargo test --features integration`.

- [x] 14. **Stmt::Break/Continue read top-of-stack; lambda isolation wired**

  **What to do**:
  - At `src/codegen/statements.rs:89` replace the `Stmt::Break | Stmt::Continue | Stmt::Comment => Ok(())` arm:
    - Split into separate arms.
    - For `Stmt::Comment`: keep `Ok(())`.
    - For `Stmt::Break`: read `env.current_loop()`. If `Some(ctx)`: `builder.build_unconditional_branch(ctx.break_target)`. If `None`: emit `CodegenError` with message `"break used outside of loop body"`.
    - For `Stmt::Continue`: same shape, branch to `ctx.continue_target`. `None` → `CodegenError "continue used outside of loop body"`.
    - After the unconditional branch, emit a fresh unreachable basic block (LLVM convention: positioning the builder on a new block prevents later instructions from being inserted into a terminated block).
  - At lambda-codegen site (find via `rg 'Expr::Lambda' src/codegen/`): wrap the lambda body emission in `env.with_loop_isolated(|env| { /* emit lambda body */ })` so a `break` inside a lambda does NOT branch to the outer loop's target.
  - Verify by manual reading: any place that recurses through statements while inside a loop should naturally see the stack top — including `if`, `match`, `guard else`, nested blocks.

  **Must NOT do**:
  - Do NOT silently no-op when stack is empty — emit a clear error (typechecker should reject earlier; defensive in codegen).
  - Do NOT support labeled `break label;` (no AST support).
  - Do NOT forget the unreachable-block-after-branch idiom.

  **Recommended Agent Profile**:
  - **Category**: `deep` — Reason: Subtle control-flow correctness; lambda isolation easy to miss.
  - **Skills**: [] — Inkwell + AST traversal.

  **Parallelization**:
  - **Can Run In Parallel**: YES with T11/T12 (different files).
  - **Parallel Group**: Wave 3.
  - **Blocks**: T13.
  - **Blocked By**: T2, T10.

  **References**:
  - **Pattern References**:
    - `src/codegen/statements.rs:89` — current no-op arm.
    - `src/codegen/expressions.rs::CodegenEnv::current_loop`/`with_loop_isolated` (T2 added these on the CodegenEnv `impl` block in expressions.rs:40+).
    - `src/codegen/control_flow.rs:543-604` — loop body emitter (push/pop site).
  - **AST References**:
    - `src/ast.rs` `Stmt::Break { span, id }`, `Stmt::Continue { span, id }`, `Expr::Lambda`.

  **WHY Each Reference Matters**:
  - Without lambda isolation, a closure captured then called outside its defining loop would branch to a stale block — undefined behavior.

  **Acceptance Criteria**:
  - [ ] Build passes.
  - [ ] All non-op_cat integration tests pass.
  - [ ] `Stmt::Break | Stmt::Continue` arms BOTH branch to correct targets via `current_loop()`.
  - [ ] Lambda body codegen invokes `with_loop_isolated`.
  - [ ] A test fixture with `break` outside a loop fails compilation with the new error message (no panic).

  **QA Scenarios**:
  ```
  Scenario: Break/continue inside guard else inside while
    Tool: Bash (.sisyphus/fixtures/guard_continue_in_loop.op: while loop with guard whose else does `continue`).
    Steps:
      1. Run `./target/release/opalescent .sisyphus/fixtures/guard_continue_in_loop.op --run 2>&1 | tee .sisyphus/evidence/task-14-guardcont.log; echo "exit=$?"`
      2. Assert exit 0; iteration count matches expected (no double-iteration).
    Expected: Continue correctly branches to loop header.
    Evidence: .sisyphus/evidence/task-14-guardcont.log

  Scenario: Break outside loop emits clear error
    Tool: Bash (.sisyphus/fixtures/break_outside.op: top-level `break` not in any loop).
    Steps:
      1. Run `./target/release/opalescent .sisyphus/fixtures/break_outside.op 2>&1 | tee .sisyphus/evidence/task-14-breakerr.log; echo "exit=$?"`
      2. Assert exit != 0; log contains `break used outside of loop body`.
    Expected: Diagnostic, no panic.
    Evidence: .sisyphus/evidence/task-14-breakerr.log

  Scenario: Lambda body does NOT inherit outer loop's break/continue targets
    Tool: Bash
    Preconditions:
      - Fixture file `.sisyphus/fixtures/lambda_in_loop.op` (CREATED as part of this task's commit). Contents: a `while true` loop whose body defines an inline lambda containing a bare `break` statement (the lambda is bound to a `let` but NOT called, so any error must come from compile-time codegen, not runtime). Example skeleton:
          ```
          entry main = f(args: string[]): void =>
              let mutable i: int32 = 0
              while i < 10:
                  let escape = f(): void => break
                  i = i + 1
              return void
          ```
      - This project's source-of-truth for the chosen semantics is THIS plan (T2's `with_loop_isolated` helper + T10's push/pop placement + T14's wrapping lambda body in `with_loop_isolated`). The README (lines 1-700, scanned 2026-04-28) intentionally documents `guard`/`propagate`/`match`/loops but is silent on lambda-internal `break`. AST evidence: `src/ast.rs` `Stmt::Break { span, id }` carries no label, and `Expr::Lambda` is a value-producing expression — therefore the only well-defined behavior is to treat the lambda body as its own scope with an EMPTY loop stack. Branching to the enclosing loop's `break_target` from inside a lambda value would be unsound (the lambda may be invoked after the loop has exited).
    Steps:
      1. Run `./target/release/opalescent .sisyphus/fixtures/lambda_in_loop.op 2>&1 | tee .sisyphus/evidence/task-14-lambda.log; echo "exit=$?" >> .sisyphus/evidence/task-14-lambda.log`
      2. Assert exit code is non-zero (compile-time codegen rejection): `grep -q 'exit=[1-9]' .sisyphus/evidence/task-14-lambda.log`
      3. Assert log contains the EXACT diagnostic substring: `grep -q 'break used outside of loop body' .sisyphus/evidence/task-14-lambda.log`
    Expected Result: Compile fails with a non-zero exit code AND stderr/stdout contains the literal string `break used outside of loop body`. No panic, no LLVM verifier crash, no silent branching to the outer loop's break_target.
    Failure Indicators: Exit code 0 (means the `break` was silently accepted — UNSOUND); OR a panic / LLVM verifier error in the log; OR a different error message that doesn't match the literal string above (means a different code path emitted the error).
    Evidence: .sisyphus/evidence/task-14-lambda.log
  ```

  **Commit**: YES
  - Message: `feat(codegen): break/continue use LoopContext stack with lambda isolation`
  - Files: `src/codegen/statements.rs`, lambda-codegen site (verify location), fixtures.
  - Pre-commit: `cargo build --release && cargo test --features integration -- --skip op_cat`

- [x] 15. **Documentation: README + .op error-handling notes for new ABI**

  **What to do**:
  - Append a brief subsection to `README.md` under "Error Handling" describing what compiles (void/scalar errors-bearing functions) and what doesn't yet (aggregate-T errors-bearing, payload-bearing variants), with one-line examples.
  - Add a comment block at the top of `src/codegen/error_abi.rs` summarizing the chosen ABI shape (`{T, i8*}` / `{i8*, i8*}`), null-pointer encoding semantics, field index rule, and a pointer to `runtime/opal_runtime.h:125-141`.
  - No source-code behavioral changes.

  **Must NOT do**:
  - Do NOT introduce code changes.
  - Do NOT exceed 30 lines added to README.
  - Do NOT update CHANGELOG (out of scope unless one already exists; check `ls CHANGELOG*`).

  **Recommended Agent Profile**:
  - **Category**: `writing` — Reason: Pure documentation.
  - **Skills**: [] — Markdown.

  **Parallelization**:
  - **Can Run In Parallel**: YES (independent file scope).
  - **Parallel Group**: Wave 3.
  - **Blocks**: F1, F2.
  - **Blocked By**: T3 (so the ABI module exists to document).

  **References**:
  - **Pattern References**:
    - `README.md` — existing "Error Handling" section.
    - `runtime/opal_runtime.h:125-141` — ABI source of truth.

  **WHY Each Reference Matters**:
  - Future contributors need to know the shape and the explicit unsupported cases (aggregate-T, payload variants).

  **Acceptance Criteria**:
  - [ ] `README.md` has a new subsection (≤ 30 lines added).
  - [ ] `src/codegen/error_abi.rs` has a top-of-file doc comment describing the ABI.
  - [ ] No source-behavior changes (`git diff --stat src/` shows only `error_abi.rs` modified, doc-comment-only).

  **QA Scenarios**:
  ```
  Scenario: Docs added, no behavior change
    Tool: Bash
    Steps:
      1. Run `git diff --stat README.md src/codegen/error_abi.rs | tee .sisyphus/evidence/task-15-diff.log`
      2. Run `cargo build --release 2>&1 | tee .sisyphus/evidence/task-15-build.log; echo "exit=$?"`
      3. Run `cargo test --features integration 2>&1 | tee .sisyphus/evidence/task-15-tests.log; echo "exit=$?"`
      4. Assert build exit 0; tests exit 0; diff shows only docs touched.
    Expected: Docs landed cleanly.
    Evidence: .sisyphus/evidence/task-15-diff.log, .sisyphus/evidence/task-15-build.log, .sisyphus/evidence/task-15-tests.log
  ```

  **Commit**: YES
  - Message: `docs: document errors-bearing function ABI and current limitations`
  - Files: `README.md`, `src/codegen/error_abi.rs`.
  - Pre-commit: `cargo build --release && cargo test --features integration`.

---

## Final Verification Wave (MANDATORY — after ALL implementation tasks)

> 4 review agents run in PARALLEL. ALL must APPROVE. Present consolidated results to user and get explicit "okay" before completing.
> Do NOT auto-proceed. Wait for user's explicit approval before marking work complete.
> Never mark F1-F4 as checked before user okay. Rejection or user feedback → fix → re-run → present again → wait.

- [x] F1. **Plan Compliance Audit** — `oracle`
  Read this plan end-to-end. For each "Must Have": verify implementation exists (read file, run command, inspect IR). For each "Must NOT Have": grep codebase for forbidden patterns — reject with file:line if found. Check evidence files exist in `.sisyphus/evidence/`. Compare deliverables against plan.
  Output: `Must Have [N/N] | Must NOT Have [N/N] | Tasks [N/N] | VERDICT: APPROVE/REJECT`

- [x] F2. **Code Quality Review** — `unspecified-high`
  Run `cargo build --release`, `cargo clippy --all-targets`, `cargo test --features integration`. Review changed files for: `as any`-equivalents (`unwrap()`, `expect("")` with empty msg, `.clone()` on Copy types), empty error catches, `println!`/`eprintln!` left in non-CLI codegen paths, commented-out code, unused imports. Check AI-slop: excessive comments, generic names (`data`, `result`, `temp`), over-abstraction.
  Output: `Build [PASS/FAIL] | Clippy [PASS/FAIL] | Tests [N pass/N fail] | Files [N clean/N issues] | VERDICT`

- [x] F3. **Real Manual QA** — `unspecified-high`
  Start from clean `target/`. Execute EVERY QA scenario from EVERY task — exact steps, capture evidence. Test cross-task integration: run op-cat with mixed valid/invalid args; assert printed lines match expectation. Test edge cases: zero args, all-invalid args, repeated-valid args. Save evidence to `.sisyphus/evidence/final-qa/`.
  Output: `Scenarios [N/N pass] | Integration [N/N] | Edge Cases [N tested] | VERDICT`

- [x] F4. **Scope Fidelity Check** — `deep`
  For each task: read "What to do", read actual diff (`git log <branch>..HEAD --stat`, `git diff <range>`). Verify 1:1 — everything in spec was built (no missing), nothing beyond spec (no creep). Check "Must NOT do" compliance per task. Detect cross-task contamination: Task N touching Task M's files unexpectedly. Flag unaccounted changes.
  Output: `Tasks [N/N compliant] | Contamination [CLEAN/N issues] | Unaccounted [CLEAN/N files] | VERDICT`

---

## Commit Strategy

- Atomic commits per user mandate. Every TODO → at least one commit; complex TODOs → multiple commits at logical sub-step boundaries.
- Commit message style: `type(scope): description` — verified by T5 against `git log --oneline -20`.
- Pre-commit gate: `cargo build --release` MUST succeed for every commit. Tests need not all pass mid-wave (TDD allows RED commits in T1), but the build must always link.
- Suggested commit boundaries per task are documented in each TODO's `Commit` field.

---

## Success Criteria

### Verification Commands
```bash
# Build
cargo build --release

# Unit + integration tests
cargo test --features integration

# op-cat happy + error path
target/release/opalescent run test-projects/op-cat/src/main.op -- \
  test-projects/op-cat/sample.txt missing.txt test-projects/op-cat/sample.txt
# Expected: exit 0; stdout contains sample.txt contents twice + one error line for missing.txt

# Regression sweep — all test-projects must exit 0
for proj in test-projects/*/opal.toml; do
  dir=$(dirname "$proj")
  echo "=== $dir ==="
  target/release/opalescent run "$dir/src/main.op" || echo "FAILED: $dir"
done

# IR shape check (T6 evidence)
target/release/opalescent test-projects/op-cat/src/main.op
llvm-dis-14 test-projects/op-cat/target/program.bc 2>/dev/null || \
  objdump -d test-projects/op-cat/target/program.o | head -50
```

### Final Checklist
- [ ] All "Must Have" present
- [ ] All "Must NOT Have" absent (verified by F1 grep)
- [ ] `cargo build --release` PASS
- [ ] `cargo test --features integration` PASS
- [ ] op-cat happy + error path PASS
- [ ] All other test-projects PASS regression sweep
- [ ] F1, F2, F3, F4 all APPROVE
- [ ] User gives explicit "okay"
