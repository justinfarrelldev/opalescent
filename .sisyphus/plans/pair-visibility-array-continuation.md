# Pair Visibility and Array Functionality Continuation

## TL;DR
> **Summary**: Expose `Pair<T, U>` as a predefined language-visible generic product type using existing ADT metadata paths, then resume the paused array functionality plan from `.zip` through double-array verification and the final review wave.
> **Deliverables**:
> - Language-visible `Pair<T, U>` with fields `first: T` and `second: U`
> - `.zip` implementation resumed from the prior STOP gate and committed as `feat(array): implement zip`
> - Double-array Task 12 completed exactly from the original array plan
> - Final verification wave completed only after Task 12 is green and user approval gate is respected
> **Effort**: Medium
> **Parallel**: NO for implementation tasks; YES only for final verification agents
> **Critical Path**: Pair prerequisite → Pair smoke tests → `.zip` RED/GREEN → double arrays → final verification

## Context
### Original Request
The user chose direction #1 after the previous array plan stopped at `.zip`: expose a language-visible `Pair<T, U>` and then allow the rest of the original array work to finish. The user explicitly wants all remaining actions, including Task 12 double arrays, exactly as previously planned but with `Pair<T, U>` implemented first.

### Interview Summary
- Prior work completed append, push, pop, map, filter, and reduce.
- Prior Task 11 stopped because `.zip` was typed as `Pair<T,U>[]` but no language-visible `Pair<T,U>` existed.
- User selected the `Pair<T,U>` exposure path rather than changing `.zip` semantics.
- No further user preferences are required: preserve the original `.zip` contract (`Pair<T,U>[]`, fields `first`/`second`, unequal lengths truncate) and original Task 12 double-array scope.

### Research Findings
- STOP evidence: `.sisyphus/evidence/task-11-zip-pair-stop.md` proves no built-in/prelude/standard language-visible `Pair<T,U>` currently exists.
- `.zip` return shape: `src/type_system/checker/collections/collections_array.rs` registers `.zip` as returning `Pair<T,U>[]`.
- Generic ADT precedent: `src/type_system/test_integration_generics.rs` declares `type Pair<T, U>:` with fields `first` and `second`, and proves generic product construction works for user-declared ADTs.
- Constructor parsing: `src/parser/new_expression.rs` supports `new Type:` product constructor syntax.
- Constructor checking: `src/type_system/checker/constructors.rs` handles product field checking and generic constructor inference.
- ADT codegen: `src/codegen/adts.rs` lowers product constructors, generic ADT names, and field access.
- Built-in/predefined type bootstrap: `src/type_system/environment.rs` and `src/type_system/checker.rs` are the relevant registration points for language-visible predefined types.
- Module-visible ADT metadata: `src/type_system/module_resolver/standard_modules.rs`, `src/type_system/module_resolver.rs`, and `src/type_system/checker/module_checking.rs` show how module ADT fields are exported/imported, but research found no existing implicit prelude mechanism.
- Runtime stdlib iterator `zip`: `src/stdlib/collections/iter.rs` returns Rust tuples for `OpalIter::opal_zip`; this plan keeps iterator zip out of scope because array `.zip` is compiler-lowered like map/filter/reduce.

### Metis Review (gaps addressed)
- Added explicit name-collision policy: predefined `Pair` is reserved; user redeclaration fails deterministically.
- Added constructor syntax lock: use existing `new Pair:` product constructor syntax, matching current parser/tests.
- Added Task 12 definition lock: double arrays means nested arrays `T[][]`, not floating-point arrays.
- Added guardrail that iterator `zip` in `src/stdlib/collections/iter.rs` is out of scope.
- Added smoke tests for direct `Pair<int32,string>` construction/field access before `.zip` implementation.
- Added acceptance criteria for length mismatch, empty zips, nested `Pair<Pair<...>, ...>`, and no tuple syntax.

### Continuation State From Prior Plan
- Source of truth before this continuation: `.sisyphus/plans/array-functionality.md`.
- Completed and must not be reimplemented: Tasks 1-10 (baseline/harness/foundation/member calls/append/push/pop/map/filter/reduce).
- Task 11 STOP branch completed with `.sisyphus/evidence/task-11-zip-pair-stop.md`; this continuation supersedes the stop by adding the missing prerequisite.
- Task 12 and F1-F4 remain uncompleted and must run after `.zip` succeeds.

## Work Objectives
### Core Objective
Make `Pair<T,U>` language-visible and compatible with existing generic ADT constructor/field access, then finish `.zip`, nested `T[][]`, and final verification without duplicating prior completed array slices.

### Deliverables
- Predefined `Pair<T,U>` type available without user declaration/import.
- Product fields: `first: T`, `second: U`.
- Deterministic duplicate-definition error for user-declared `type Pair<...>`.
- Direct Pair smoke test project/evidence.
- Resumed `.zip` RED/GREEN evidence and local commit.
- Original Task 12 double-array project/evidence and local commit.
- Final verification wave evidence and explicit user approval gate.

### Definition of Done (verifiable conditions with commands)
- `cargo test --features integration array_pair_runs -- --nocapture --test-threads=1` passes after Pair exposure.
- `cargo test --features integration array_zip_runs -- --nocapture --test-threads=1` passes after `.zip` implementation.
- `cargo test --features integration array_double_runs -- --nocapture --test-threads=1` passes after double-array implementation.
- `cargo test --all-features` passes after all implementation tasks.
- `cargo clippy --all-targets --all-features -- -D warnings` passes after all implementation tasks.
- `cargo fmt --all -- --check` passes after all implementation tasks.
- `cargo build --release` passes in final verification.
- `git log --oneline` includes `feat(array): expose Pair` (or `feat(types): expose Pair`) plus `feat(array): implement zip` plus `feat(array): support double arrays` after this continuation.
- Final branch push occurs only after final verification and explicit user approval per the original plan.

### Must Have
- Pair is a predefined nominal generic product type, not tuple syntax.
- Pair construction uses existing `new Pair:` product constructor syntax.
- Pair field access uses `.first` and `.second`.
- Existing user-declared generic ADT behavior remains intact for non-`Pair` types.
- `.zip` truncates to the shorter input length.
- `T[][]` supports literal construction, `grid.length`, `grid[row].length`, and `grid[row][col]` reads for uniform and jagged arrays.
- RED evidence must be captured before `.zip` and double-array implementation.

### Must NOT Have (guardrails)
- No tuple syntax `(a, b)` or tuple field syntax `.0`/`.1`.
- No destructuring or tuple patterns.
- No `Triple`, `Tuple`, `Either`, new `Option`, or generalized product-type feature expansion.
- No iterator `zip` changes in `src/stdlib/collections/iter.rs`.
- No new implicit prelude or module-system mechanism.
- No reimplementation/refactor of completed append/push/pop/map/filter/reduce slices unless a regression fix is strictly required.
- No Pair equality/display/pattern-matching/operator overloads beyond construction and field access.
- No final verification before `.zip` and Task 12 are green.

## Verification Strategy
> ZERO HUMAN INTERVENTION - all verification is agent-executed.
- Test decision: TDD RED-GREEN-REFACTOR using Cargo/Rust tests plus Opalescent test projects.
- QA policy: Every task has agent-executed scenarios.
- Evidence: `.sisyphus/evidence/pair-continuation-task-{N}-{slug}.{ext}` and original task evidence names for resumed Task 11/12 where specified.
- Integration commands that run Opalescent projects must be serialized with `--test-threads=1` because prior evidence shows shared `target/program` races across concurrent runs.

## Execution Strategy
### Parallel Execution Waves
> Target: serial implementation because each step depends on the prior language/type surface.

Wave 1: Task 1 — Pair prerequisite foundation
Wave 2: Task 2 — Pair smoke tests and prior-work sanity checks
Wave 3: Task 3 — Resume `.zip` Task 11
Wave 4: Task 4 — Resume double-array Task 12
Wave 5: Final verification F1-F4 and final push/approval gate

### Dependency Matrix (full, all tasks)
- T1 blocks T2-T4 and final verification.
- T2 blocked by T1; blocks T3.
- T3 blocked by T2; blocks T4.
- T4 blocked by T3; blocks final verification.
- Final verification blocked by T4.

### Agent Dispatch Summary (wave → task count → categories)
- Wave 1 → 1 task → `deep`
- Wave 2 → 1 task → `deep`
- Wave 3 → 1 task → `deep`
- Wave 4 → 1 task → `deep`
- Wave 5 → 4 review agents/tools → oracle, unspecified-high, unspecified-high, deep

## TODOs
> Implementation + Test = ONE task. Never separate.
> EVERY task MUST have: Agent Profile + Parallelization + QA Scenarios.

- [x] 1. Expose predefined generic `Pair<T, U>`

  **What to do**: Register `Pair<T,U>` as a predefined language-visible nominal generic product type using existing ADT infrastructure. Add field metadata for `first: T` and `second: U`, register generic parameters `T` and `U`, and reserve the name `Pair` so user code declaring `type Pair<...>` fails with a deterministic duplicate/reserved type diagnostic. Use the builtin/checker bootstrap path only: `src/type_system/environment.rs` for the visible type name and `src/type_system/checker.rs` for `adt_fields` + `adt_generic_params` registration. Do not expose `Pair` through `standard` module exports in this plan; `Pair` must be available without import. Keep `src/stdlib/collections/iter.rs` unchanged.
  **Must NOT do**: Do not add tuple syntax, implicit prelude loading, standard module `Pair` export, standard iterator zip changes, or generalized tuple/product features.

  **Recommended Agent Profile**:
  - Category: `deep` - Reason: Type-system bootstrap, generic ADT metadata, and name-resolution behavior.
  - Skills: [] - No special skill required.
  - Omitted: [`playwright`] - No UI/browser work.

  **Parallelization**: Can Parallel: NO | Wave 1 | Blocks: T2-T4 | Blocked By: none

  **References** (executor has NO interview context - be exhaustive):
  - STOP proof: `.sisyphus/evidence/task-11-zip-pair-stop.md` - missing prerequisite this task resolves.
  - Existing zip signature: `src/type_system/checker/collections/collections_array.rs` - `.zip` already returns `Pair<T,U>[]`; do not change this public contract.
  - Generic Pair precedent: `src/type_system/test_integration_generics.rs` - user-declared `Pair<T,U>` shape uses `first` and `second`.
  - Built-in type registration: `src/type_system/environment.rs` - predefined language-visible type names.
  - Checker bootstrap/ADT maps: `src/type_system/checker.rs` - `adt_fields` and `adt_generic_params` initialization/registration.
  - Module metadata reference only: `src/type_system/module_resolver/standard_modules.rs`, `src/type_system/module_resolver.rs`, `src/type_system/checker/module_checking.rs` - inspect for ADT field-map shape, but do not add a `standard.Pair` export in this plan.
  - Constructor checking: `src/type_system/checker/constructors.rs` - existing generic product constructor inference path.
  - Constructor parsing: `src/parser/new_expression.rs` - required syntax is `new Pair:` with indented fields.
  - ADT codegen: `src/codegen/adts.rs` - product constructor and field access lowering.

  **Acceptance Criteria** (agent-executable only):
  - [ ] `cargo test type_system::test_integration_generics::tests::test_generic_product_constructor_infers_multiple_type_args` still passes or is intentionally updated to avoid reserved `Pair` while preserving equivalent non-`Pair` generic ADT coverage.
  - [ ] A new checker/unit test proves `Pair<int32,string>` or equivalent `Pair<int32, string>` is accepted without a user declaration.
  - [ ] A new checker/unit test proves `pair.first` is `int32` and `pair.second` is `string` for a predefined `Pair<int32,string>`.
  - [ ] A new negative checker/unit test proves user `type Pair<T, U>:` redeclaration fails deterministically with a diagnostic containing `Pair` and either `reserved` or `duplicate`.
  - [ ] `cargo test --workspace` passes.

  **QA Scenarios** (MANDATORY - task incomplete without these):
  ```
  Scenario: predefined Pair type-checks without declaration
    Tool: Bash
    Steps: Run targeted Rust checker test for source that constructs `new Pair:` with `first: 1 as int32` and `second: 'x'`, then accesses both fields.
    Expected: Test exits 0; inferred field types are int32 and string; no local `type Pair` declaration exists in the source.
    Evidence: .sisyphus/evidence/pair-continuation-task-1-pair-typecheck.txt

  Scenario: Pair name is reserved
    Tool: Bash
    Steps: Run targeted Rust checker test with a source-level `type Pair<T, U>:` declaration.
    Expected: Test exits 0 while asserting the compiler rejects the source with diagnostic containing `Pair` and `reserved` or `duplicate`.
    Evidence: .sisyphus/evidence/pair-continuation-task-1-pair-reserved.txt
  ```

  **Commit**: YES | Message: `feat(array): expose Pair` | Files: `src/type_system/environment.rs`, `src/type_system/checker.rs`, optional module resolver/checker files if needed, relevant tests

- [x] 2. Add Pair smoke project and sanity-check completed prior array slices

  **What to do**: Add `test-projects/array-pair` and a corresponding integration test `array_pair_runs` in `tests/array_integration.rs`. The project must construct `Pair<int32,string>` with `new Pair:` and print `first 7`, `second seven`. Fixture source must be exactly:
  ```op
  ##
    Description: Verifies predefined Pair construction and field access.
  ##
  entry main = f(args: string[]): void =>
      let pair = new Pair:
          first: 7 as int32
          second: 'seven'
      print('first {pair.first}')
      print('second {pair.second}')
      return void
  ```
  Expected stdout: `first 7`, `second seven`. Also run a focused sanity suite for completed prior work: append, push, pop, map, filter, and reduce integration tests serialized. Capture evidence without reimplementing those slices.
  **Must NOT do**: Do not edit append/push/pop/map/filter/reduce implementation unless a regression is discovered; if a regression is found, fix it as a minimal fix-forward commit and record it.

  **Recommended Agent Profile**:
  - Category: `deep` - Reason: End-to-end smoke coverage plus regression confirmation across prior compiler-lowered array features.
  - Skills: [] - No special skill required.
  - Omitted: [`playwright`] - No UI.

  **Parallelization**: Can Parallel: NO | Wave 2 | Blocks: T3 | Blocked By: T1

  **References**:
  - Harness: `tests/array_integration.rs` - existing helpers for `test-projects/array-*`.
  - Test project conventions: `test-projects/array-reduce/src/main.op`, `test-projects/array-reduce/expected/stdout.txt` - latest function fixture style.
  - Pair constructor parser: `src/parser/new_expression.rs` - use `new Pair:` syntax.
  - Prior STOP evidence: `.sisyphus/evidence/task-11-zip-pair-stop.md` - should be superseded, not deleted.

  **Acceptance Criteria**:
  - [ ] `test-projects/array-pair` exists with `opal.toml`, `.gitignore`, `README.md`, `src/main.op`, and `expected/stdout.txt`.
  - [ ] `cargo test --features integration array_pair_runs -- --nocapture --test-threads=1` passes and stdout exactly matches expected file.
  - [ ] Serialized prior sanity commands pass: `array_append_runs`, `array_push_runs`, `array_pop_runs`, `array_map_runs`, `array_filter_runs`, `array_reduce_runs`.
  - [ ] Evidence captures Pair smoke GREEN and prior sanity results.
  - [ ] `cargo test --workspace` passes after adding smoke project.

  **QA Scenarios**:
  ```
  Scenario: Pair project constructs and reads fields
    Tool: Bash
    Steps: Run `cargo test --features integration array_pair_runs -- --nocapture --test-threads=1`.
    Expected: stdout exactly `first 7` and `second seven` after stripping the CLI `target/program` line.
    Evidence: .sisyphus/evidence/pair-continuation-task-2-pair-smoke-green.txt

  Scenario: Prior array slices still pass
    Tool: Bash
    Steps: Run serialized targeted integration tests for append, push, pop, map, filter, and reduce with `--test-threads=1`.
    Expected: All commands exit 0; no source edits to prior function implementations are required.
    Evidence: .sisyphus/evidence/pair-continuation-task-2-prior-sanity.txt
  ```

  **Commit**: YES | Message: `test(array): add Pair smoke coverage` | Files: `tests/array_integration.rs`, `test-projects/array-pair/*`, evidence

- [x] 3. Resume Task 11 and implement `.zip` returning `Pair<T, U>[]`

  **What to do**: Resume original Task 11 from `.sisyphus/plans/array-functionality.md` now that `Pair<T,U>` is language-visible. Create `test-projects/array-zip` as RED first with this exact source:
  ```op
  ##
    Description: Runs array zip over unequal arrays and reads Pair fields.
  ##
  entry main = f(args: string[]): void =>
      let left: int32[] = [1 as int32, 2 as int32, 3 as int32]
      let right: string[] = ['a', 'b']
      let pairs = left.zip(right)
      print('length {pairs.length}')
      print('first {pairs[0].first} {pairs[0].second}')
      print('second {pairs[1].first} {pairs[1].second}')
      return void
  ```
  Expected stdout: `length 2`, `first 1 a`, `second 2 b`. Add equal-length and empty-left/empty-right integration cases. Implement compiler-lowered `.zip` as a specialized loop like `.map`/`.filter`/`.reduce`, allocating result length as `min(left.length, right.length)`, constructing each output element as a normal `Pair<T,U>` product with fields `first` and `second`, and publishing correct pending array metadata. Commit locally with exact message `feat(array): implement zip`.
  **Must NOT do**: Do not invent tuple syntax, pad values, require equal lengths, mutate source arrays, or modify iterator `OpalIter::opal_zip`.

  **Recommended Agent Profile**:
  - Category: `deep` - Reason: Higher-order/binary array lowering, generic product construction, and metadata propagation.
  - Skills: [] - No special skill required.
  - Omitted: [`playwright`] - No UI.

  **Parallelization**: Can Parallel: NO | Wave 3 | Blocks: T4 | Blocked By: T2

  **References**:
  - Original Task 11: `.sisyphus/plans/array-functionality.md` lines for `.zip` - preserve behavior and fixture intent.
  - Zip type signature: `src/type_system/checker/collections/collections_array.rs` - already declares `Pair<T,U>[]`.
  - Array HOF lowering patterns: `src/codegen/functions_call/array.rs` - follow map/filter/reduce loop and metadata style.
  - Array helper module: `src/codegen/functions_call/array/helpers.rs` - use existing allocation/copy/metadata helpers where applicable.
  - Pair codegen: `src/codegen/adts.rs` - construct product layout compatible with `new Pair:` and field access.
  - Integration harness: `tests/array_integration.rs` - add `array_zip_runs`, `array_zip_equal_lengths`, and `array_zip_empty_side`.
  - Runtime race warning: `.sisyphus/notepads/array-functionality/issues.md` - integration commands must be serialized.

  **Acceptance Criteria**:
  - [ ] RED: `cargo test --features integration array_zip_runs -- --nocapture --test-threads=1` fails before implementation due `.zip` unimplemented/codegen failure, not Pair visibility failure.
  - [ ] GREEN: same command passes and stdout exactly matches `length 2`, `first 1 a`, `second 2 b`.
  - [ ] Equal-length zip preserves all pairs in order.
  - [ ] Empty left and empty right each return zipped length 0.
  - [ ] Result elements support `.first` and `.second` field access at typecheck and runtime.
  - [ ] Gate passes: `cargo test --workspace`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo fmt --all -- --check`.
  - [ ] Local commit exists with message `feat(array): implement zip`.

  **QA Scenarios**:
  ```
  Scenario: zip truncates unequal arrays
    Tool: Bash
    Steps: Run `cargo test --features integration array_zip_runs -- --nocapture --test-threads=1`.
    Expected: stdout exactly `length 2`, `first 1 a`, `second 2 b`.
    Evidence: .sisyphus/evidence/task-11-zip-green.txt

  Scenario: zip empty side
    Tool: Bash
    Steps: Run empty-left and empty-right integration cases with `--test-threads=1`.
    Expected: Both print `length 0`.
    Evidence: .sisyphus/evidence/task-11-zip-empty.txt
  ```

  **Commit**: YES | Message: `feat(array): implement zip` | Files: zip type/codegen/runtime helper files plus `test-projects/array-zip`, `tests/array_integration.rs`

- [x] 4. Resume Task 12 double-array literals, lengths, and reads

  **What to do**: Execute original Task 12 from `.sisyphus/plans/array-functionality.md` exactly after `.zip` is green. Create `test-projects/array-double` as RED first. Cover uniform 3x3, jagged with empty row, single row, single column, and empty outer array. Update array literal/access/length handling so nested `T[][]` carries row-specific metadata through `grid[row].length` and `grid[row][col]` reads. Add nested out-of-bounds negative test for `jagged[1][0]`. Commit locally with exact message `feat(array): support double arrays`.
  **Must NOT do**: Do not implement indexed writes, rectangular-only arrays, or mutable row aliasing changes outside the original Task 12 scope.

  **Recommended Agent Profile**:
  - Category: `deep` - Reason: Nested aggregate codegen and row-specific length metadata.
  - Skills: [] - No special skill required.
  - Omitted: [`playwright`] - No UI.

  **Parallelization**: Can Parallel: NO | Wave 4 | Blocks: Final verification | Blocked By: T3

  **References**:
  - Original Task 12: `.sisyphus/plans/array-functionality.md` - use fixture and expected stdout exactly.
  - Spec: `ARRAY_FEATURES.md` sections for double arrays/jagged arrays.
  - Literal codegen: `src/codegen/expressions.rs` - `codegen_array_literal`.
  - Access codegen: `src/codegen/expressions.rs` - `codegen_array_access`.
  - Length metadata helpers: `src/codegen/expressions_loop.rs` and current array metadata conventions.
  - Existing array integration harness: `tests/array_integration.rs`.

  **Acceptance Criteria**:
  - [ ] RED failure captured before double-array implementation.
  - [ ] `cargo test --features integration array_double_runs -- --nocapture --test-threads=1` passes.
  - [ ] Uniform, jagged, single-row, single-column, and empty-outer stdout lines match exactly from original Task 12.
  - [ ] `grid[row].length` uses row-specific length, including jagged empty row length 0.
  - [ ] Nested out-of-bounds test reports index 0 out of bounds for length 0.
  - [ ] Gate passes: `cargo test --workspace`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo fmt --all -- --check`.
  - [ ] Local commit exists with message `feat(array): support double arrays`.

  **QA Scenarios**:
  ```
  Scenario: double-array varieties
    Tool: Bash
    Steps: Run `cargo test --features integration array_double_runs -- --nocapture --test-threads=1`.
    Expected: All expected stdout lines match exactly, including jagged row lengths 2,0,3.
    Evidence: .sisyphus/evidence/task-12-double-arrays-green.txt

  Scenario: nested bounds failure
    Tool: Bash
    Steps: Run nested out-of-bounds integration case accessing `jagged[1][0]`.
    Expected: Nonzero exit or runtime error; diagnostic reports index 0 out of bounds for length 0.
    Evidence: .sisyphus/evidence/task-12-double-arrays-bounds.txt
  ```

  **Commit**: YES | Message: `feat(array): support double arrays` | Files: nested array codegen/runtime/type files plus `test-projects/array-double`, `tests/array_integration.rs`

## Final Verification Wave (MANDATORY — after ALL implementation tasks)
> 4 review agents run in PARALLEL. ALL must APPROVE. Present consolidated results to user and get explicit "okay" before completing.
> **Do NOT auto-proceed after verification. Wait for user's explicit approval before marking work complete.**
> **Never mark F1-F4 as checked before getting user's okay.** Rejection or user feedback -> fix -> re-run -> present again -> wait for okay.

- [x] F1. Plan Compliance Audit — oracle
  - Verify this continuation plan and `.sisyphus/plans/array-functionality.md` are both satisfied: Pair prerequisite resolved, `.zip` implemented, Task 12 implemented, and prior STOP evidence superseded rather than deleted.

  **QA Scenarios**:
  ```
  Scenario: Decision Lock compliance audit
    Tool: task(subagent_type="oracle")
    Steps: Invoke oracle with this plan, original array plan, final diff, git log, and evidence directory. Ask it to check every Decision Lock and continuation task.
    Expected: Oracle returns APPROVE with zero unmet items, or returns a concrete numbered rejection list to fix.
    Evidence: .sisyphus/evidence/f1-plan-compliance-audit.md
  ```

- [x] F2. Code Quality Review — unspecified-high
  - Review Pair registration, zip lowering, nested arrays, metadata consistency, diagnostics, and scope guardrails.

  **QA Scenarios**:
  ```
  Scenario: Static gate and diff quality review
    Tool: Bash + task(category="unspecified-high")
    Steps: Run `cargo test --all-features`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo fmt --all -- --check`, `cargo build --release`, and `git diff --check`; provide outputs and full branch diff to reviewer.
    Expected: All commands exit 0; reviewer APPROVES with no blockers.
    Evidence: .sisyphus/evidence/f2-code-quality-review.md
  ```

- [x] F3. Real Manual QA — unspecified-high
  - Run all positive array CLI projects via the debug binary, including `array-pair`, `array-zip`, and `array-double`, comparing stdout to expected files.

  **QA Scenarios**:
  ```
  Scenario: CLI run all positive array projects
    Tool: Bash
    Steps: Build debug binary, then run CLI execution for `array-append`, `array-push`, `array-pop`, `array-map`, `array-filter`, `array-reduce`, `array-pair`, `array-zip`, and `array-double`; compare stdout to each project's `expected/stdout.txt` after stripping `target/program`.
    Expected: Every positive project exits 0 and stdout exactly equals its expected file.
    Evidence: .sisyphus/evidence/f3-array-cli-qa.txt
  ```

- [x] F4. Scope Fidelity Check — deep
  - Confirm no tuple syntax, no iterator zip changes, no Pair equality/display/pattern matching, no broad prelude/module mechanism, no reimplementation of completed prior slices, and no final completion before user approval.

  **QA Scenarios**:
  ```
  Scenario: Out-of-scope feature audit
    Tool: task(category="deep")
    Steps: Provide final branch diff and Must NOT Have guardrails. Ask reviewer to search for tuple syntax support, `Triple`/`Tuple`, iterator zip changes, Pair operator/display additions, and broad prelude/module-system changes.
    Expected: Reviewer APPROVES confirming no out-of-scope features were implemented; any finding is a blocker.
    Evidence: .sisyphus/evidence/f4-scope-fidelity.md
  ```

## Commit Strategy
- Do not rewrite prior commits for append/push/pop/map/filter/reduce.
- Required continuation commits:
  - `feat(array): expose Pair`
  - `test(array): add Pair smoke coverage`
  - `feat(array): implement zip`
  - `feat(array): support double arrays`
- If Pair exposure requires only type-system tests and no integration project yet, the Pair smoke coverage commit still follows immediately after.
- Before each implementation commit: run targeted GREEN command, relevant negative tests, `cargo test --workspace`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo fmt --all -- --check`, and `git diff --check`.
- Push once only after final verification and explicit user approval, following the original branch policy from `.sisyphus/plans/array-functionality.md`.

## Success Criteria
- `Pair<T,U>` is language-visible without user declaration and supports `new Pair:` plus `.first`/`.second`.
- `.zip` returns `Pair<T,U>[]`, truncates unequal arrays, and supports field access on results.
- Task 12 double arrays pass exactly as originally planned.
- Completed prior tasks are not duplicated or regressed.
- All gates and final review agents approve.
- User explicitly approves consolidated final verification before final completion/push.
