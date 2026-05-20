# Array COW RC Game of Life Memory Optimization

## TL;DR
> **Summary**: Implement RC-correct rebinding and uniqueness-aware array COW fast paths so array updates preserve value semantics while avoiding full clone churn for unique arrays. Add a machine-checkable Game of Life memory probe proving a 100x100 board stays under 100KB of Opal runtime heap and remains leak-free/stable over updates.
> **Deliverables**:
> - RC-safe overwrite/rebinding helper used by general assignments and array rebinding paths.
> - Runtime uniqueness/reuse eligibility helpers that account for weak refs.
> - Uniqueness-aware indexed assignment, chained multidimensional assignment, and `.push`/`.pop`/`clear`/`reserve` lowering.
> - Game of Life memory probe and integration tests with executable byte/leak/stability assertions.
> - Final 3-verifier PASS artifacts, committed with all staged/unstaged changes and clean git status.
> **Effort**: Large
> **Parallel**: YES - 6 waves
> **Critical Path**: Task 1 → Task 2 → Task 3 → Task 4 → Task 5 → Task 8 → F1-F3 → Task 12

## Context
### Original Request
The user asked to verify and implement a supplied plan for array RC/COW optimization, with a definition of done that includes 3 additional final verifiers. Each verifier must independently approve that a 100x100 Game of Life board uses under 100KB of memory and that memory over board updates is reasonable/workable for a modern language, including no leaks. After confirmations, all staged and unstaged changes, including Sisyphus artifacts, must be committed and `git status` must be entirely clean.

Prometheus constraint: this file is the execution plan. Implementation must be performed by Sisyphus via `/start-work`.

### Interview Summary
No interview was needed because the user supplied a detailed sequence and explicit final verification/commit requirements. Defaults applied:
- Memory budget measures peak live Opal runtime heap attributable to Game of Life board state and update temporaries, excluding process RSS, compiler process memory, sanitizer inflation, and test harness overhead.
- Leak/stability verification is separate from the 100KB measurement: release/non-sanitized build for byte budget, sanitizer/Valgrind for leaks.
- Perceus compile-time reuse is not a primary dependency; implement the limited branch only if the runtime uniqueness/COW work still fails the 100KB probe.
- CI workflow changes are out of scope unless required to make existing commands pass; do not add sanitizer to `.github/workflows/ci.yml`.
- Final commit is a single semantic commit after verifier PASS files exist.

### Metis Review (gaps addressed)
Metis identified methodology drift and RC store discipline as the highest risks. This plan addresses them by making measurement harness/baseline Task 1, language-wide RC-bearing overwrite discipline Task 2, and runtime uniqueness predicate Task 3 before any fast path consumes uniqueness. Metis also required verifier outputs to be machine-checkable, a Perceus escalation branch, a `.gitignore` audit before committing, and executable acceptance criteria for the 100KB, steady-state, sanitizer, test-suite, and clean-status gates.

### Momus High Accuracy Review
Momus reviewed this plan and returned `OKAY`. Evidence is recorded in `.sisyphus/evidence/plan-review-array-cow-rc-game-of-life.md`. Momus noted Task 6 requires careful reading around `clear`/`reserve`, and the plan now explicitly preserves current public call shapes and existing tests.

## Work Objectives
### Core Objective
Make Opalescent array mutation efficient for practical 100x100 Game of Life workloads without breaking the language’s value/COW semantics or RC memory safety.

### Deliverables
- Measurement harness command: `cargo run --release --bin gol_memory_probe -- --size 100 --ticks 10` prints `peak_live_bytes: N` and exits 0 when `N < 102400`.
- Stability command: `cargo run --release --bin gol_memory_probe -- --size 100 --ticks 100 --report-per-tick` prints per-tick live-byte values and proves the post-warmup spread is ≤ 1024 bytes.
- Sanitizer command: `bash scripts/array_memory_sanitizer.sh` exits 0 after new array/GoL fixtures are included in coverage.
- Existing suite command: `timeout 900 cargo test --all-features` exits 0.
- Final verifier artifacts: `.sisyphus/verification/verifier-1.md`, `verifier-2.md`, `verifier-3.md`, each containing `STATUS: PASS`.
- Final commit includes all tracked/untracked non-ignored work products and leaves `git status --porcelain` empty.

### Definition of Done (verifiable conditions with commands)
- `cargo run --release --bin gol_memory_probe -- --size 100 --ticks 10` exits 0 and stdout contains `peak_live_bytes: <N>` with `N < 102400`.
- `cargo run --release --bin gol_memory_probe -- --size 100 --ticks 100 --report-per-tick` exits 0 and its own assertion confirms `steady_state_spread_bytes <= 1024` after tick 50.
- `bash scripts/array_memory_sanitizer.sh` exits 0 and the log contains no `AddressSanitizer`, `LeakSanitizer`, `heap-use-after-free`, `double-free`, or `detected memory leaks` failure marker.
- `timeout 900 cargo test --all-features` exits 0.
- `cargo clippy --all-targets --all-features -- -D warnings` exits 0.
- `cargo fmt --all -- --check` exits 0.
- `grep -c "^STATUS: PASS$" .sisyphus/verification/verifier-*.md` returns `3`.
- After the final commit, `git status --porcelain` prints nothing.

### Must Have
- Preserve `append(xs, value)` as logically pure/non-mutating.
- Preserve current public call shapes and semantics: `.push`/`.pop` as member rebind operations, and `clear`/`reserve` as the existing array intrinsics unless implementation proves they already have receiver-method aliases.
- Preserve existing alias tests in `tests/array_integration.rs:226`, `:314`, and `:336`.
- Retain replacement RC-bearing values before releasing overwritten values.
- Runtime reuse eligibility predicate must be `refcount == 1 && weak_count == 0` for objects whose header could be reused or whose storage could move.
- Chained multidimensional assignment must uniquify both outer and inner arrays as needed.
- Measurement must be non-sanitized release mode; sanitizer is only for leak/lifetime validation.

### Must NOT Have
- Do not mutate shared arrays in place.
- Do not remove clone/rebind fallback.
- Do not use `opal_rc_reuse` on objects with `weak_count > 0`.
- Do not make `append` destructive unless compile-time last-use proof is implemented in the limited contingency task.
- Do not expose refcount/capacity APIs as normal Opalescent language features.
- Do not add `.github/workflows/ci.yml` sanitizer gates without explicit separate approval.
- Do not commit ignored/generated `target/` artifacts.
- Do not proceed to final commit unless all 3 verifier files contain `STATUS: PASS`.

## Verification Strategy
> ZERO HUMAN INTERVENTION - all verification is agent-executed.
- Test decision: tests-after with Rust integration tests and runtime probe; add regression tests before changing each risky path where feasible, but do not block on strict TDD for runtime ABI changes.
- QA policy: Every task has agent-executed scenarios.
- Evidence: `.sisyphus/evidence/task-{N}-{slug}.{ext}`.
- Memory accounting policy: byte budget counts live Opal runtime heap bytes for RC headers + array payload headers + payload data allocated by the generated program during the probe. It excludes process RSS, compiler memory, stdout buffers, sanitizer redzones, and Cargo/test harness memory.
- Feasibility math to keep in probe docs: nested `Bool[100][100]` double-buffered board is expected under 100KB because payload is ~20KB for two 10,000-cell byte boards plus row/outer headers; `Int64` cells or 3+ live full boards are out-of-budget and must not be used for the acceptance fixture.

## Execution Strategy
### Parallel Execution Waves
> Target: 5-8 tasks per wave where possible. Some waves are intentionally narrow because RC correctness and memory methodology are strict dependencies.

Wave 1: Task 1 (measurement/baseline) and Task 2 (RC overwrite discipline audit/fix) may start in parallel only if Task 2 does not consume probe results.
Wave 2: Task 3 (runtime uniqueness/helpers) after Task 2 contracts are defined.
Wave 3: Task 4 (indexed assignment) and Task 5 (multidimensional reads/writes) after Task 3.
Wave 4: Task 6 (push/pop/clear/reserve) and Task 7 (append purity/regression hardening) after Tasks 4-5.
Wave 5: Task 8 (GoL fixture/probe enforcement), Task 9 (sanitizer integration), and conditional Task 10 (limited reuse escalation if needed).
Wave 6: Task 11 (full local verification), F1-F3 final verifiers in parallel, then Task 12 (commit/clean status).

### Dependency Matrix (full, all tasks)
- Task 1 blocks Tasks 8, 10, 11, F1-F3, Task 12.
- Task 2 blocks Tasks 3-7, 9-11, F1-F3, Task 12.
- Task 3 blocks Tasks 4-7, 9-11, F1-F3, Task 12.
- Task 4 blocks Tasks 5, 8-11, F1-F3, Task 12.
- Task 5 blocks Task 8 if the GoL fixture uses `board[r][c] = value`; otherwise still blocks F1-F3.
- Task 6 blocks Tasks 8-11, F1-F3, Task 12.
- Task 7 blocks Task 11, F1-F3, Task 12.
- Task 8 blocks Task 10 decision, Task 11, F1-F3, Task 12.
- Task 9 blocks Task 11, F3, Task 12.
- Task 10 is conditional and blocks Task 11 only if Task 8 reports `peak_live_bytes >= 102400` after Tasks 1-9.
- Task 11 blocks F1-F3.
- F1-F3 block Task 12.

### Agent Dispatch Summary (wave → task count → categories)
- Wave 1 → 2 tasks → `deep`, `unspecified-high`.
- Wave 2 → 1 task → `deep`.
- Wave 3 → 2 tasks → `unspecified-high`, `deep`.
- Wave 4 → 2 tasks → `unspecified-high`, `quick`.
- Wave 5 → 3 tasks → `deep`, `unspecified-high`, `deep` conditional.
- Wave 6 → 5 tasks including final verifiers → `unspecified-high`, `oracle`, `deep`, `unspecified-high`, `quick` with `git-master` for commit if available.

## TODOs
> Implementation + Test = ONE task. Never separate.
> EVERY task MUST have: Agent Profile + Parallelization + QA Scenarios.

- [x] 1. Lock memory methodology and baseline Game of Life probe

  **What to do**: Add a runtime memory accounting harness that can measure peak live Opal runtime heap bytes attributable to generated-program RC/array allocations. Implement a `gol_memory_probe` binary or equivalent existing-bin subcommand chosen by executor only if the repo already has a clearer binary convention; the final command must be exactly `cargo run --release --bin gol_memory_probe -- --size 100 --ticks 10` and support `--report-per-tick`. Add a 100x100 double-buffered `Bool` Game of Life fixture using the simplest currently supported array syntax. Record baseline current behavior in `.sisyphus/evidence/task-1-baseline-memory.txt` before optimizing.
  **Must NOT do**: Do not measure process RSS. Do not run the 100KB assertion under ASAN/LSAN. Do not use `Int64` cells for the acceptance board. Do not change `.github/workflows/ci.yml`.

  **Recommended Agent Profile**:
  - Category: `deep` - Reason: Requires runtime allocation accounting design plus benchmark/probe integration.
  - Skills: [] - No specialized skill required.
  - Omitted: [`frontend-ui-ux`] - No UI work.

  **Parallelization**: Can Parallel: YES with Task 2 contract audit | Wave 1 | Blocks: Tasks 8, 10, 11, F1-F3, Task 12 | Blocked By: none

  **References**:
  - Runtime ABI: `runtime/opal_rc.h:38` - `OpalRcHeader` size/layout.
  - Runtime allocation: `runtime/opal_rc.c:78` - `opal_rc_alloc` implementation to instrument or wrap.
  - Runtime arrays: `runtime/opal_rc.c:201` - `opal_array_alloc` computes payload size.
  - Bench structs: `src/benchmarks/memory.rs` - existing metrics are descriptive only, not authoritative measurement.
  - Test harness pattern: `tests/array_integration.rs:486` - existing memory churn sanitizer fixture.

  **Acceptance Criteria**:
  - [ ] `cargo run --release --bin gol_memory_probe -- --size 100 --ticks 10` exists, exits 0, and prints `peak_live_bytes: N`.
  - [ ] `cargo run --release --bin gol_memory_probe -- --size 100 --ticks 100 --report-per-tick` prints per-tick live bytes and `steady_state_spread_bytes: N`.
  - [ ] `.sisyphus/evidence/task-1-baseline-memory.txt` contains the command outputs and states whether current baseline passes or fails before optimizations.

  **QA Scenarios**:
  ```
  Scenario: 100x100 probe records peak bytes
    Tool: Bash
    Steps: cargo run --release --bin gol_memory_probe -- --size 100 --ticks 10 | tee .sisyphus/evidence/task-1-gol-100x100.txt
    Expected: Exit code 0; output has line matching ^peak_live_bytes: [0-9]+$; evidence file exists.
    Evidence: .sisyphus/evidence/task-1-gol-100x100.txt

  Scenario: Invalid size rejected
    Tool: Bash
    Steps: cargo run --release --bin gol_memory_probe -- --size 0 --ticks 10 2>&1 | tee .sisyphus/evidence/task-1-invalid-size.txt
    Expected: Nonzero exit or explicit error; no panic backtrace; error mentions size must be positive.
    Evidence: .sisyphus/evidence/task-1-invalid-size.txt
  ```

  **Commit**: NO | Message: n/a | Files: runtime/probe/test files plus `.sisyphus/evidence/*` staged later by Task 12

- [x] 2. Implement RC-safe overwrite/rebinding contract for RC-bearing locals

  **What to do**: Define and implement one helper for storing an RC-bearing value into a binding: load old value, retain new when needed, store new, release old when needed, and clear any cached array length/capacity metadata. Use it for normal identifier assignment in `src/codegen/statements.rs`, relevant parameter/local binding initialization in `src/codegen/functions.rs`, and `store_array_binding` in `src/codegen/functions_call/array/helpers.rs:67`. Add tests that prove aliases and self-assignment do not leak or double-free.
  **Must NOT do**: Do not implement full Perceus. Do not broadly refactor unrelated codegen. Do not release an old value before the new value is safely retained.

  **Recommended Agent Profile**:
  - Category: `unspecified-high` - Reason: Cross-cutting compiler/runtime correctness change.
  - Skills: [] - No specialized skill required.
  - Omitted: [`git-master`] - No commit yet.

  **Parallelization**: Can Parallel: YES with Task 1 only during audit; implementation should be serialized before Task 3 | Wave 1 | Blocks: Tasks 3-7, 9-11, F1-F3, Task 12 | Blocked By: none

  **References**:
  - Array store helper: `src/codegen/functions_call/array/helpers.rs:67` - current `store_array_binding` target.
  - RC emission: `src/codegen/rc_emitter.rs:33` and `:42` - existing inc/dec emitters.
  - Indexed assignment current clone/rebind: `src/codegen/expressions_array.rs:95`.
  - Existing alias tests: `tests/array_integration.rs:226`, `:314`, `:336`.

  **Acceptance Criteria**:
  - [ ] All RC-bearing binding overwrite paths use one shared helper or a documented wrapper around it.
  - [ ] Existing alias tests still pass: `cargo test --features integration --test array_integration array_push_cow_alias array_index_assignment_cow_alias array_index_assignment_rc_nested_row_rebind -- --nocapture`.
  - [ ] New tests cover `xs = xs`, assignment over an array, and function parameter/local alias mutation without leaks.

  **QA Scenarios**:
  ```
  Scenario: Self-assignment preserves array and does not double free
    Tool: Bash
    Steps: cargo test --features integration --test array_integration array_self_assignment_rc_safe -- --nocapture | tee .sisyphus/evidence/task-2-self-assignment.txt
    Expected: Exit code 0; stdout confirms values unchanged; no sanitizer markers in stderr.
    Evidence: .sisyphus/evidence/task-2-self-assignment.txt

  Scenario: Rebinding releases old value without mutating aliases
    Tool: Bash
    Steps: cargo test --features integration --test array_integration array_rebind_releases_old_preserves_alias -- --nocapture | tee .sisyphus/evidence/task-2-rebind-alias.txt
    Expected: Exit code 0; alias observes original array; rebound variable observes new array.
    Evidence: .sisyphus/evidence/task-2-rebind-alias.txt
  ```

  **Commit**: NO | Message: n/a | Files: codegen and tests staged later by Task 12

- [x] 3. Add runtime uniqueness, reuse eligibility, and array helper ABI

  **What to do**: Add C runtime helpers in `runtime/opal_rc.h`/`.c`: `opal_rc_is_unique(obj)` returning true only when `refcount == 1`, `opal_rc_is_reuse_eligible(obj)` returning true only when `refcount == 1 && weak_count == 0`, and test-only `opal_rc_strong_count_for_test(obj)`/`opal_rc_weak_count_for_test(obj)` behind an internal/testing guard if feasible. Add codegen declarations in `src/codegen/rc_emitter.rs` and array helper wrappers in `src/codegen/functions_call/array/helpers.rs`. If adding `opal_array_clone`, `opal_array_set_unique`, `opal_array_push_unique`, or `opal_array_realloc_unique_or_null`, document ownership rules in the header and keep clone/rebind fallback.
  **Must NOT do**: Do not reuse/move headers when `weak_count > 0`. Do not expose these helpers as user-facing language APIs. Do not change existing RC header size without updating static asserts and all offsets.

  **Recommended Agent Profile**:
  - Category: `deep` - Reason: Runtime ABI and compiler declaration changes require careful ownership semantics.
  - Skills: [] - No specialized skill required.
  - Omitted: [`frontend-ui-ux`] - No UI work.

  **Parallelization**: Can Parallel: NO | Wave 2 | Blocks: Tasks 4-7, 9-11, F1-F3, Task 12 | Blocked By: Task 2

  **References**:
  - RC header: `runtime/opal_rc.h:38` - fields include refcount and weak_count.
  - Weak ref note: `runtime/opal_rc.h:20` - weak refs hold header pointers.
  - Existing reuse: `runtime/opal_rc.h:108`, `runtime/opal_rc.c:95` - current `opal_rc_reuse`.
  - RC emitter: `src/codegen/rc_emitter.rs:33`, `:42`, `:51`, `:106`.

  **Acceptance Criteria**:
  - [ ] Runtime unit/integration tests prove `opal_rc_is_unique` differs from `opal_rc_is_reuse_eligible` when weak refs exist.
  - [ ] Codegen can emit calls to uniqueness/reuse helpers without duplicate declarations.
  - [ ] `bash scripts/array_memory_sanitizer.sh` still exits 0 after helper additions.

  **QA Scenarios**:
  ```
  Scenario: Strong unique object reports unique and reuse eligible
    Tool: Bash
    Steps: cargo test rc_uniqueness_strong_only -- --nocapture | tee .sisyphus/evidence/task-3-strong-unique.txt
    Expected: Exit code 0; test asserts unique=true and reuse_eligible=true for new RC object.
    Evidence: .sisyphus/evidence/task-3-strong-unique.txt

  Scenario: Weak reference blocks reuse eligibility
    Tool: Bash
    Steps: cargo test rc_uniqueness_weak_blocks_reuse -- --nocapture | tee .sisyphus/evidence/task-3-weak-blocks-reuse.txt
    Expected: Exit code 0; test asserts unique=true but reuse_eligible=false when weak_count > 0.
    Evidence: .sisyphus/evidence/task-3-weak-blocks-reuse.txt
  ```

  **Commit**: NO | Message: n/a | Files: runtime/codegen/tests staged later by Task 12

- [x] 4. Implement uniqueness-aware indexed assignment fast path

  **What to do**: Replace unconditional clone behavior in `codegen_identifier_indexed_array_assignment` with a branch: bounds check first; if receiver is unique, mutate the slot in place; if shared, retain clone/rebind fallback. For RC-bearing elements, retain the replacement before storing and release the overwritten element after it is no longer reachable. Preserve current OOB diagnostics and alias behavior.
  **Must NOT do**: Do not skip bounds checks. Do not mutate shared arrays. Do not release overwritten element before retaining replacement.

  **Recommended Agent Profile**:
  - Category: `unspecified-high` - Reason: Targeted but high-risk codegen control-flow change.
  - Skills: [] - No specialized skill required.
  - Omitted: [`git-master`] - No commit yet.

  **Parallelization**: Can Parallel: NO | Wave 3 | Blocks: Tasks 5, 8-11, F1-F3, Task 12 | Blocked By: Task 3

  **References**:
  - Indexed assignment: `src/codegen/expressions_array.rs:95`.
  - Array access/bounds pattern: `src/codegen/expressions_array.rs:58`.
  - Allocation fallback: `src/codegen/expressions_array.rs:267`.
  - Existing alias test: `tests/array_integration.rs:314`.

  **Acceptance Criteria**:
  - [ ] Unique array indexed assignment does not allocate a replacement payload according to probe/test instrumentation.
  - [ ] Shared array indexed assignment still clones and leaves aliases unchanged.
  - [ ] Bounds failures remain unchanged from existing integration tests.

  **QA Scenarios**:
  ```
  Scenario: Unique array index assignment mutates in place
    Tool: Bash
    Steps: cargo test --features integration --test array_integration array_index_assignment_unique_in_place -- --nocapture | tee .sisyphus/evidence/task-4-unique-index.txt
    Expected: Exit code 0; allocation counter unchanged for payload replacement; final value updated.
    Evidence: .sisyphus/evidence/task-4-unique-index.txt

  Scenario: Shared array index assignment preserves alias
    Tool: Bash
    Steps: cargo test --features integration --test array_integration array_index_assignment_cow_alias -- --nocapture | tee .sisyphus/evidence/task-4-shared-index.txt
    Expected: Exit code 0; alias sees original value; mutated binding sees replacement value.
    Evidence: .sisyphus/evidence/task-4-shared-index.txt
  ```

  **Commit**: NO | Message: n/a | Files: codegen/tests/evidence staged later by Task 12

- [x] 5. Implement chained multidimensional reads and writes with nested COW

  **What to do**: Support `rows[r][c]` read lowering as load row array then load cell. Support `rows[r][c] = value` as: bounds-check outer; load row; bounds-check inner; if inner unique mutate slot else clone row and mutate clone; then rebind updated row into outer array using the same uniqueness/COW path as Task 4. Add tests for outer unique/inner unique, outer unique/inner shared, outer shared/inner unique, and bounds failures.
  **Must NOT do**: Do not treat jagged arrays as rectangular. Do not replace this with Matrix-only semantics. Do not mutate an inner row shared by another outer slot.

  **Recommended Agent Profile**:
  - Category: `deep` - Reason: Nested l-value semantics and COW interaction are subtle.
  - Skills: [] - No specialized skill required.
  - Omitted: [`frontend-ui-ux`] - No UI work.

  **Parallelization**: Can Parallel: NO | Wave 3 | Blocks: Task 8 if GoL uses nested assignment; F1-F3; Task 12 | Blocked By: Task 4

  **References**:
  - Nested literal support: `src/codegen/expressions_array.rs:34`.
  - Array access: `src/codegen/expressions_array.rs:58`.
  - Nested row rebind test: `tests/array_integration.rs:336`.
  - Type checker array signatures: `src/type_system/checker/collections/collections_array.rs`.

  **Acceptance Criteria**:
  - [ ] `rows[r][c]` reads correct values for jagged arrays.
  - [ ] `rows[r][c] = value` updates only the intended row/cell and preserves aliases.
  - [ ] Existing nested bounds tests continue to report row length, not outer length.

  **QA Scenarios**:
  ```
  Scenario: Nested unique row assignment updates one cell
    Tool: Bash
    Steps: cargo test --features integration --test array_integration array_nested_assignment_unique_row -- --nocapture | tee .sisyphus/evidence/task-5-nested-unique.txt
    Expected: Exit code 0; only rows[r][c] changes; other rows unchanged.
    Evidence: .sisyphus/evidence/task-5-nested-unique.txt

  Scenario: Shared inner row clones before write
    Tool: Bash
    Steps: cargo test --features integration --test array_integration array_nested_assignment_shared_inner_row_cow -- --nocapture | tee .sisyphus/evidence/task-5-nested-shared.txt
    Expected: Exit code 0; other alias to same inner row remains unchanged.
    Evidence: .sisyphus/evidence/task-5-nested-shared.txt
  ```

  **Commit**: NO | Message: n/a | Files: codegen/tests/evidence staged later by Task 12

- [x] 6. Make `.push`, `.pop`, `clear`, and `reserve` uniqueness-aware

  **What to do**: In `src/codegen/functions_call/array/intrinsics.rs`, split operations into unique fast paths and shared clone/rebind fallbacks while preserving current public call shape. `.push`: member rebind operation; if unique and capacity > length, write at length and increment length; if unique and reuse-eligible/growable, grow safely; else allocate/copy fallback. `.pop`: member rebind operation; transfer ownership of returned RC-bearing element before shrinking/rebinding; release abandoned slot correctly; never return a dangling element. `clear`: keep existing intrinsic shape; if operating on a mutable binding and unique, release all live elements then set len 0, otherwise create an empty replacement preserving aliases. `reserve`: keep existing intrinsic shape; no-op when requested capacity ≤ current; unique grow when safe; clone/rebind fallback when shared.
  **Must NOT do**: Do not change `append` purity. Do not change `clear`/`reserve` from intrinsics into member methods unless existing parser/type-checker already supports that shape and tests require it. Do not shrink length before transferring/releasing removed RC-bearing elements. Do not use realloc-like movement when weak refs exist.

  **Recommended Agent Profile**:
  - Category: `unspecified-high` - Reason: Multiple array operations with distinct ownership rules.
  - Skills: [] - No specialized skill required.
  - Omitted: [`git-master`] - No commit yet.

  **Parallelization**: Can Parallel: YES with Task 7 after Task 5 if touching disjoint tests; code changes should be serialized | Wave 4 | Blocks: Tasks 8-11, F1-F3, Task 12 | Blocked By: Tasks 3-5

  **References**:
  - Reserve: `src/codegen/functions_call/array/intrinsics.rs:182`.
  - Clear: `src/codegen/functions_call/array/intrinsics.rs:239`.
  - Append helper: `src/codegen/functions_call/array/intrinsics.rs:276`.
  - Push: `src/codegen/functions_call/array/intrinsics.rs:321`.
  - Pop: `src/codegen/functions_call/array/intrinsics.rs:343`.
  - Capacity helper: `src/codegen/functions_call/array/helpers.rs:299`.
  - Length setter: `src/codegen/functions_call/array/helpers.rs:390`.
  - Existing tests: `tests/array_integration.rs:420`, `:443`, `:547`, `:552`.

  **Acceptance Criteria**:
  - [ ] Unique `.push` within capacity performs no full payload clone.
  - [ ] Shared `.push` preserves alias behavior from `tests/array_integration.rs:226`.
  - [ ] `.pop` returning RC-bearing elements remains valid after receiver rebind/drop.
  - [ ] `clear` releases live RC-bearing elements exactly once.
  - [ ] `reserve(0)` and `reserve(< current_capacity)` do not allocate.

  **QA Scenarios**:
  ```
  Scenario: Unique push reuses capacity
    Tool: Bash
    Steps: cargo test --features integration --test array_integration array_push_unique_reuses_capacity -- --nocapture | tee .sisyphus/evidence/task-6-push-unique.txt
    Expected: Exit code 0; payload allocation count does not increase for within-capacity push.
    Evidence: .sisyphus/evidence/task-6-push-unique.txt

  Scenario: Pop returned RC element survives receiver mutation
    Tool: Bash
    Steps: cargo test --features integration --test array_integration array_pop_rc_element_ownership_transfer -- --nocapture | tee .sisyphus/evidence/task-6-pop-rc.txt
    Expected: Exit code 0; popped nested array/string remains readable after source array is mutated/dropped.
    Evidence: .sisyphus/evidence/task-6-pop-rc.txt
  ```

  **Commit**: NO | Message: n/a | Files: codegen/tests/evidence staged later by Task 12

- [x] 7. Preserve and harden pure `append` semantics

  **What to do**: Keep `append(xs, value)` conservative and logically pure. Add regression tests proving `append` returns a new logical array and leaves `xs` unchanged even when `xs` is otherwise unique. Ensure any helper refactoring from Tasks 2-6 does not accidentally route append through destructive mutation. Document in code comments that compile-time last-use optimization is deferred to Task 10 only if needed.
  **Must NOT do**: Do not make append secretly destructive. Do not depend on runtime uniqueness to mutate append’s receiver.

  **Recommended Agent Profile**:
  - Category: `quick` - Reason: Mostly regression tests and small guard comments.
  - Skills: [] - No specialized skill required.
  - Omitted: [`git-master`] - No commit yet.

  **Parallelization**: Can Parallel: YES with Task 6 tests if code edits do not conflict | Wave 4 | Blocks: Task 11, F1-F3, Task 12 | Blocked By: Task 3

  **References**:
  - Append entry: `src/codegen/functions_call/array/intrinsics.rs:74`.
  - Append lowerer: `src/codegen/functions_call/array/intrinsics.rs:276`.
  - Existing append tests: `tests/array_integration.rs` search `array_append_runs`.

  **Acceptance Criteria**:
  - [ ] `append(xs, value)` leaves `xs` unchanged in unique and shared cases.
  - [ ] `.push` and `append` tests demonstrate different mutability semantics.

  **QA Scenarios**:
  ```
  Scenario: Append does not mutate unique input
    Tool: Bash
    Steps: cargo test --features integration --test array_integration array_append_unique_input_pure -- --nocapture | tee .sisyphus/evidence/task-7-append-unique.txt
    Expected: Exit code 0; original array unchanged; returned array includes appended value.
    Evidence: .sisyphus/evidence/task-7-append-unique.txt

  Scenario: Append does not mutate shared input
    Tool: Bash
    Steps: cargo test --features integration --test array_integration array_append_shared_input_pure -- --nocapture | tee .sisyphus/evidence/task-7-append-shared.txt
    Expected: Exit code 0; all aliases to input remain unchanged.
    Evidence: .sisyphus/evidence/task-7-append-shared.txt
  ```

  **Commit**: NO | Message: n/a | Files: tests/comments/evidence staged later by Task 12

- [x] 8. Enforce 100x100 Game of Life memory target

  **What to do**: Update the Game of Life probe/fixture from Task 1 to use the optimized operations naturally. The acceptance board must be 100x100 `Bool`, double-buffered, run 10 ticks for peak budget and 100 ticks for stability. Make the probe fail with nonzero exit when peak live bytes ≥ 102400 or when post-tick-50 steady-state spread exceeds 1024 bytes. Store run outputs in `.sisyphus/evidence/task-8-gol-memory.txt` and `.sisyphus/evidence/task-8-gol-stability.txt`.
  **Must NOT do**: Do not lower the board size. Do not change the memory definition to pass. Do not suppress transient allocations unless they are genuinely outside generated-program RC/array heap.

  **Recommended Agent Profile**:
  - Category: `deep` - Reason: Acceptance-critical memory validation tied to language runtime behavior.
  - Skills: [] - No specialized skill required.
  - Omitted: [`frontend-ui-ux`] - No UI work.

  **Parallelization**: Can Parallel: NO | Wave 5 | Blocks: Task 10 decision, Task 11, F1-F3, Task 12 | Blocked By: Tasks 1, 4, 6 and Task 5 if nested assignment is used

  **References**:
  - Probe from Task 1.
  - Array push/index lowering from Tasks 4 and 6.
  - Runtime allocation accounting from `runtime/opal_rc.c:78`, `:201`.

  **Acceptance Criteria**:
  - [ ] `cargo run --release --bin gol_memory_probe -- --size 100 --ticks 10` exits 0 with `peak_live_bytes < 102400`.
  - [ ] `cargo run --release --bin gol_memory_probe -- --size 100 --ticks 100 --report-per-tick` exits 0 with `steady_state_spread_bytes <= 1024`.
  - [ ] Evidence files contain the exact commands and outputs.

  **QA Scenarios**:
  ```
  Scenario: 100x100 board under 100KB
    Tool: Bash
    Steps: cargo run --release --bin gol_memory_probe -- --size 100 --ticks 10 | tee .sisyphus/evidence/task-8-gol-memory.txt
    Expected: Exit code 0; peak_live_bytes captured value is < 102400.
    Evidence: .sisyphus/evidence/task-8-gol-memory.txt

  Scenario: 100-tick memory remains stable
    Tool: Bash
    Steps: cargo run --release --bin gol_memory_probe -- --size 100 --ticks 100 --report-per-tick | tee .sisyphus/evidence/task-8-gol-stability.txt
    Expected: Exit code 0; output includes steady_state_spread_bytes <= 1024.
    Evidence: .sisyphus/evidence/task-8-gol-stability.txt
  ```

  **Commit**: NO | Message: n/a | Files: probe/tests/evidence staged later by Task 12

- [x] 9. Integrate leak and sanitizer coverage for new array paths

  **What to do**: Extend existing array integration/sanitizer coverage so `bash scripts/array_memory_sanitizer.sh` exercises new index assignment, nested assignment, push/pop/clear/reserve, and Game of Life churn fixtures. Prefer adding tests to `tests/array_integration.rs` and existing fixture conventions instead of creating a separate sanitizer script. Run sanitizer/Valgrind script and save output.
  **Must NOT do**: Do not weaken sanitizer marker checks. Do not add broad suppressions for new leaks. Do not require sanitizer for the 100KB release byte budget.

  **Recommended Agent Profile**:
  - Category: `unspecified-high` - Reason: Memory safety validation across runtime/codegen changes.
  - Skills: [] - No specialized skill required.
  - Omitted: [`git-master`] - No commit yet.

  **Parallelization**: Can Parallel: YES with Task 8 after operation changes land | Wave 5 | Blocks: Task 11, F3, Task 12 | Blocked By: Tasks 2-6

  **References**:
  - Sanitizer script: `scripts/array_memory_sanitizer.sh`.
  - Churn fixture: `tests/array_integration.rs:486`.
  - Existing array tests: `tests/array_integration.rs`.

  **Acceptance Criteria**:
  - [ ] `bash scripts/array_memory_sanitizer.sh` exits 0.
  - [ ] Script output covers at least one fixture/test for each changed operation category.
  - [ ] `.sisyphus/evidence/task-9-sanitizer.txt` contains full command output and no sanitizer failure markers.

  **QA Scenarios**:
  ```
  Scenario: Sanitizer covers array churn and new fast paths
    Tool: Bash
    Steps: bash scripts/array_memory_sanitizer.sh 2>&1 | tee .sisyphus/evidence/task-9-sanitizer.txt
    Expected: Exit code 0; output has no AddressSanitizer/LeakSanitizer failure markers.
    Evidence: .sisyphus/evidence/task-9-sanitizer.txt

  Scenario: Deliberate sanitizer marker grep stays sensitive
    Tool: Bash
    Steps: grep -E "AddressSanitizer|LeakSanitizer|heap-use-after-free|double-free|detected memory leaks" .sisyphus/evidence/task-9-sanitizer.txt > .sisyphus/evidence/task-9-sanitizer-markers.txt || true
    Expected: Marker file is empty.
    Evidence: .sisyphus/evidence/task-9-sanitizer-markers.txt
  ```

  **Commit**: NO | Message: n/a | Files: tests/script/evidence staged later by Task 12

- [x] 10. Conditional limited Perceus/last-use reuse escalation

  **What to do**: Execute this task only if Task 8 fails with `peak_live_bytes >= 102400` after Tasks 1-9 pass. Implement the narrowest compile-time reuse needed for patterns in the Game of Life probe, prioritizing `xs = append(xs, value)` and `next = step(current)`-style last-use rebinds. Use `src/type_system/rc_analysis.rs` only after validating its outputs; if stale, implement a local, explicit last-use check for the probe pattern with tests. If Task 8 already passes, mark this task completed as "not needed" with evidence from Task 8.
  **Must NOT do**: Do not implement full Perceus. Do not make append destructive without last-use proof. Do not broaden representation to packed bits or Matrix unless the user explicitly approves a new plan.

  **Recommended Agent Profile**:
  - Category: `deep` - Reason: Conditional compiler analysis/codegen optimization with correctness risk.
  - Skills: [] - No specialized skill required.
  - Omitted: [`git-master`] - No commit yet.

  **Parallelization**: Can Parallel: NO | Wave 5 | Blocks: Task 11 if executed | Blocked By: Task 8 failure

  **References**:
  - Reuse analysis definitions: `src/type_system/rc_analysis.rs`.
  - Append lowerer: `src/codegen/functions_call/array/intrinsics.rs:276`.
  - RC reuse runtime: `runtime/opal_rc.c:95`.

  **Acceptance Criteria**:
  - [ ] If skipped: `.sisyphus/evidence/task-10-skipped.txt` states Task 8 passed and includes `peak_live_bytes`.
  - [ ] If executed: `append` last-use reuse has tests proving no visible mutation when aliases exist.
  - [ ] After execution or skip, Task 8 commands pass.

  **QA Scenarios**:
  ```
  Scenario: Conditional task skipped because memory target already passes
    Tool: Bash
    Steps: test -f .sisyphus/evidence/task-8-gol-memory.txt && grep "peak_live_bytes" .sisyphus/evidence/task-8-gol-memory.txt | tee .sisyphus/evidence/task-10-skipped.txt
    Expected: Evidence shows peak_live_bytes < 102400 and no code changes are made for Task 10.
    Evidence: .sisyphus/evidence/task-10-skipped.txt

  Scenario: Last-use append reuse remains semantically pure
    Tool: Bash
    Steps: cargo test --features integration --test array_integration array_append_last_use_reuse_alias_guard -- --nocapture | tee .sisyphus/evidence/task-10-last-use.txt
    Expected: Exit code 0; reuse occurs only when no observable alias exists; alias guard case clones.
    Evidence: .sisyphus/evidence/task-10-last-use.txt
  ```

  **Commit**: NO | Message: n/a | Files: optional codegen/tests/evidence staged later by Task 12

- [x] 11. Run full local verification and consolidate evidence

  **What to do**: Run required project-wide verification commands and store outputs. Ensure the final byte, stability, sanitizer, tests, clippy, and fmt gates pass before launching final verifiers. Create `.sisyphus/evidence/final-local-verification.md` summarizing command, exit status, and evidence file path for each gate.
  **Must NOT do**: Do not launch final verifiers if any command fails. Do not ignore clippy/fmt failures. Do not mark this task complete based on partial runs.

  **Recommended Agent Profile**:
  - Category: `unspecified-high` - Reason: Hands-on verification and evidence consolidation.
  - Skills: [] - No specialized skill required.
  - Omitted: [`git-master`] - Commit occurs in Task 12.

  **Parallelization**: Can Parallel: NO | Wave 6 | Blocks: F1-F3 | Blocked By: Tasks 1-10 as applicable

  **References**:
  - CI commands: `.github/workflows/ci.yml`.
  - Sanitizer script: `scripts/array_memory_sanitizer.sh`.
  - Probe from Task 1/8.

  **Acceptance Criteria**:
  - [ ] `timeout 900 cargo test --all-features` exits 0.
  - [ ] `cargo clippy --all-targets --all-features -- -D warnings` exits 0.
  - [ ] `cargo fmt --all -- --check` exits 0.
  - [ ] Task 8 and Task 9 commands pass.
  - [ ] Consolidated evidence markdown exists.

  **QA Scenarios**:
  ```
  Scenario: Full all-features tests pass
    Tool: Bash
    Steps: timeout 900 cargo test --all-features | tee .sisyphus/evidence/task-11-cargo-test-all-features.txt
    Expected: Exit code 0; output contains test result: ok and no FAILED summary.
    Evidence: .sisyphus/evidence/task-11-cargo-test-all-features.txt

  Scenario: Formatting and lint gates pass
    Tool: Bash
    Steps: cargo fmt --all -- --check | tee .sisyphus/evidence/task-11-fmt.txt && cargo clippy --all-targets --all-features -- -D warnings | tee .sisyphus/evidence/task-11-clippy.txt
    Expected: Both commands exit 0.
    Evidence: .sisyphus/evidence/task-11-fmt.txt; .sisyphus/evidence/task-11-clippy.txt
  ```

  **Commit**: NO | Message: n/a | Files: evidence staged later by Task 12

- [x] 12. Commit all changes and leave git status clean after verifier PASS

  **What to do**: After F1-F3 all produce `STATUS: PASS`, inspect `git status --porcelain`, `.gitignore`, and `git diff` to ensure no ignored/generated junk is staged. Stage all intended tracked and untracked changes, including `.sisyphus/plans/array-cow-rc-game-of-life.md`, `.sisyphus/evidence/*`, and `.sisyphus/verification/verifier-*.md`. Commit as a single semantic commit with message `fix(array): add rc-safe cow reuse for game of life memory`. Verify clean status after commit.
  **Must NOT do**: Do not commit before all 3 verifier files pass. Do not commit `target/` or unrelated local/IDE files. Do not force-push. Do not skip hooks.

  **Recommended Agent Profile**:
  - Category: `quick` - Reason: Final git hygiene after verification.
  - Skills: [`git-master`] - Required for git operations discipline.
  - Omitted: [`frontend-ui-ux`] - No UI work.

  **Parallelization**: Can Parallel: NO | Wave 6 | Blocks: completion | Blocked By: F1-F3 unanimous PASS

  **References**:
  - User requirement: commit ALL unstaged and staged changes including Sisyphus artifacts.
  - Final verification files: `.sisyphus/verification/verifier-1.md`, `verifier-2.md`, `verifier-3.md`.

  **Acceptance Criteria**:
  - [ ] `grep -c "^STATUS: PASS$" .sisyphus/verification/verifier-*.md` returns `3` before staging.
  - [ ] `git status --porcelain` is reviewed before staging and after commit.
  - [ ] Commit exists with all intended source/test/runtime/probe/Sisyphus artifacts.
  - [ ] Final `git status --porcelain` output is empty.

  **QA Scenarios**:
  ```
  Scenario: Verifier PASS gate blocks commit until unanimous
    Tool: Bash
    Steps: grep -c "^STATUS: PASS$" .sisyphus/verification/verifier-*.md | tee .sisyphus/evidence/task-12-verifier-count.txt
    Expected: Output is exactly 3.
    Evidence: .sisyphus/evidence/task-12-verifier-count.txt

  Scenario: Repository clean after final commit
    Tool: Bash
    Steps: git status --porcelain | tee .sisyphus/evidence/task-12-post-commit-status.txt
    Expected: Evidence file is empty; command exits 0.
    Evidence: .sisyphus/evidence/task-12-post-commit-status.txt
  ```

  **Commit**: YES | Message: `fix(array): add rc-safe cow reuse for game of life memory` | Files: all intended staged/unstaged source, tests, runtime, probe, `.sisyphus/plans/*`, `.sisyphus/evidence/*`, `.sisyphus/verification/*`

## Final Verification Wave (MANDATORY — after ALL implementation tasks)
> 3 review agents run in PARALLEL. ALL must APPROVE. Each verifier must independently confirm that the 100x100 Game of Life board is under 100KB of measured Opal runtime heap and memory over updates is stable/no-leak. Create `.sisyphus/verification/verifier-N.md` with `STATUS: PASS|FAIL`. Unanimous PASS is required before Task 12 commit. After Task 12 commit, present consolidated results to the user and wait for explicit okay before marking the work complete.
> **Do NOT auto-proceed after post-commit reporting. Wait for user's explicit approval before marking work complete.**
> **Never mark F1-F3 as checked before their PASS files exist.** Rejection or FAIL -> fix -> re-run Task 11 -> re-run F1-F3 -> Task 12 as needed.

- [x] F1. Correctness and RC Invariant Audit — oracle

  **Verifier Prompt**: Review the completed changes and evidence. Independently verify that RC-safe overwrite/rebinding, uniqueness predicates, indexed assignment, nested assignment, push/pop/clear/reserve, and append purity preserve semantics and alias contracts. Run or inspect evidence for `timeout 900 cargo test --all-features`, targeted array integration tests, and memory probe outputs. Write `.sisyphus/verification/verifier-1.md` with `STATUS: PASS` only if correctness and RC invariants are satisfied and the 100x100 memory/stability evidence passes.

- [x] F2. Memory Target Reproduction — deep

  **Verifier Prompt**: Independently reproduce the 100x100 Game of Life memory commands in release mode: `cargo run --release --bin gol_memory_probe -- --size 100 --ticks 10` and `cargo run --release --bin gol_memory_probe -- --size 100 --ticks 100 --report-per-tick`. Confirm `peak_live_bytes < 102400` and `steady_state_spread_bytes <= 1024`. Write `.sisyphus/verification/verifier-2.md` with command outputs and `STATUS: PASS` only if both thresholds pass.

- [x] F3. Leak/Lifetime and Practicality Review — unspecified-high

  **Verifier Prompt**: Run `bash scripts/array_memory_sanitizer.sh`, inspect `.sisyphus/evidence/task-9-sanitizer.txt`, and review allocation/lifetime behavior over the 100-tick probe. Confirm no sanitizer/Valgrind leak/use-after-free/double-free markers and no unbounded memory growth. Write `.sisyphus/verification/verifier-3.md` with `STATUS: PASS` only if leak/lifetime behavior is reasonable and the 100x100 memory evidence passes.

## Commit Strategy
- Single final semantic commit after F1-F3 PASS because the user explicitly requires all staged/unstaged changes, including Sisyphus artifacts, to be committed with clean status.
- Before commit: inspect `git status --porcelain`, `git diff`, and `.gitignore`; stage all intended source/test/runtime/probe/Sisyphus artifacts; exclude ignored/generated junk such as `target/`.
- Commit message: `fix(array): add rc-safe cow reuse for game of life memory`.
- After commit: run `git status --porcelain`; it must be empty. If not empty, either commit remaining intended files or remove/ignore generated junk, then re-check.

## Success Criteria
- Functional semantics preserved: existing and new alias/COW tests pass.
- Runtime memory safety preserved: sanitizer/Valgrind script exits 0 with no failure markers.
- 100x100 Game of Life target passes in release mode: `peak_live_bytes < 102400` and post-warmup spread ≤ 1024 bytes.
- Three independent verifier files contain `STATUS: PASS`.
- All staged and unstaged intended changes, including Sisyphus artifacts, are committed.
- `git status --porcelain` is empty after the final commit.
