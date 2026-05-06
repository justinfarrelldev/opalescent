# Array Functionality TDD Implementation

## TL;DR
> **Summary**: Implement spec-compliant Opalescent array growth, mutation, higher-order operations, `append`, and `T[][]` behavior using strict red-green-refactor slices. The plan prioritizes array representation/member-call foundations before user-facing functions to avoid building on fragile hidden-length side channels.
> **Deliverables**:
> - `append` free function imported from `standard`
> - `.push`, `.pop`, `.map`, `.filter`, `.reduce`, `.zip` for `T[]`
> - Double-array literal/access/length support for uniform and jagged `T[][]`
> - One red-first test project per function slice plus a double-array test project
> - Per-slice local commits and one final branch push
> **Effort**: XL
> **Parallel**: NO for implementation slices; YES only for final verification agents
> **Critical Path**: Baseline → array representation foundation → member-call lowering → append → push → pop → map → filter → reduce → zip → double arrays → final gate/push

## Context
### Original Request
Implement the array functionality in `ARRAY_FEATURES.md`; also implement `push`, `pop`, `map`, `filter`, `reduce`, and `zip`. Create failing test projects before implementation for each function, use red-green-refactor, commit after each function, and after all functions create a double-array test project covering multiple sizes and jagged/uniform arrays. User selected local commits per slice and a single final push.

### Interview Summary
- Include `append` because it is specified in `ARRAY_FEATURES.md`.
- Empty `pop()` traps/panics in v1 with a clear runtime error; return type remains `T`.
- `map`, `filter`, and `reduce` callbacks are infallible only in v1; error-propagating variants are future `try_*` APIs and out of scope.
- Commit locally after each function slice; push once after all slices and final verification pass.

### Decision Lock
- Array semantics: `T[]` has value semantics. `append(xs, value)` returns a new array and does not mutate `xs`. `.push(value)` and `.pop()` mutate the receiver only when the receiver is a mutable local binding; mutation through immutable bindings or temporaries must be rejected by checker/codegen with a clear compile-time error.
- Representation: converge new compiler-lowered array operations on one growable value representation carrying data pointer, length, and capacity together. Do not expand use of `CodegenEnv.pending_array_length` except as compatibility during migration.
- `append`: v1 may use naïve O(n) copy; document/TODO that COW/Perceus reuse is future work.
- `push`: infallible; OOM may trap/abort consistently with existing fatal runtime behavior.
- `pop`: mutates receiver, returns `T`; empty receiver exits through the same runtime error/trap path used for array bounds failures with message containing `pop on empty array`.
- `map`: `xs.map(f)` returns `U[]`; callback `f(T): U` is infallible.
- `filter`: `xs.filter(pred)` returns `T[]`; callback `pred(T): boolean` is infallible.
- `reduce`: seeded signature `xs.reduce(initial, f)` where `f(acc, item): Acc`; empty array returns `initial`.
- `zip`: `left.zip(right)` uses the existing `Pair<T, U>` shape registered by array signatures if and only if a language-visible `Pair` constructor/field-access path already exists. If `Pair` is not language-visible, STOP THE ENTIRE PLAN before creating the zip fixture or implementation, present evidence to the user, and request a plan update for language-visible `Pair` support. Do not proceed to T12 or F1-F4 on this plan until that update exists. Unequal lengths truncate to the shorter input. The zip test project is intentionally deferred until Task 11 after this proof.
- Higher-order lowering: compiler-lower specialized loops per call site; do not implement generic runtime function-pointer dispatch in v1.
- Double arrays: support literal construction, `grid.length`, `grid[row].length`, and read access `grid[row][col]` for `T[][]`. Indexed writes (`grid[row][col] = value`) are out of scope unless already needed by tests.
- Branch/push: do local commits per slice; push once to the current non-main feature branch. If current branch is `main`/`master` or has no upstream, create/push branch `array-functionality-tdd` with `git push -u origin array-functionality-tdd`.

### Research Findings
- `ARRAY_FEATURES.md:9-59` defines pure `append`; `ARRAY_FEATURES.md:155-241` defines `.push`; `ARRAY_FEATURES.md:245-484` defines jagged `T[][]` and implementation gaps.
- Array type/method signatures: `src/type_system/checker/collections/collections_array.rs:11`, `:50`, `:128`, `:150`; `src/type_system/checker/collections.rs` contains collection registration/resolution entrypoints.
- Array literal/access codegen: `src/codegen/expressions.rs:638` (`codegen_array_literal`), `src/codegen/expressions.rs:690` (`codegen_array_access`), `src/codegen/expressions.rs:68` (`pending_array_length`), `src/codegen/expressions_loop.rs:35-41` (`set/take_pending_array_length`).
- Call codegen: `src/codegen/functions_call.rs:37` (`codegen_call_expression`) and length side-channel handling around `src/codegen/functions_call.rs:351`, `:421`, `:426`.
- Runtime representation: `src/runtime/memory.rs:120` (`OpalArray<T>`), `src/runtime/arrays.rs:9` (`allocate_array`), `:22` (`array_length`), `:31` (`array_index`), `src/runtime/stdlib.rs:163` (`opal_array_slice`).
- Existing mutable stdlib vector: `src/stdlib/collections/array.rs:48` (`OpalVec<T>`); tests at `src/stdlib/collections/tests.rs:31-199` cover push/pop/map/filter/reduce semantics.
- Integration/golden pattern: `tests/fmt_integration.rs:7-41` helper layout and `:51-84` command/golden comparison pattern.
- Runtime array tests: `src/runtime/tests.rs:127-156` covers allocation/indexing/bounds; `src/runtime/tests.rs:284` and `:312` cover slicing.

### Metis Review (gaps addressed)
- Added explicit Decision Lock for mutability, `zip`, `reduce`, callback ABI, branch/push, and refactor bounds.
- Added baseline verification and STOP gates for missing closures/generics/tuples or failing base tests.
- Added foundation slices for representation and member-call lowering before function work.
- Added per-slice RED/GREEN/GATE/COMMIT/VERIFY requirements.
- Added rollback policy and refactor scope restrictions.

### Rollback and STOP Policy
- If any pre-slice baseline command fails, STOP before changing source and report evidence.
- If a slice's GREEN phase reveals a foundation flaw, do not amend or rewrite previous function commits. Create a fix-forward foundation commit if contained; if the flaw invalidates prior slice semantics, STOP and report the rollback point from Task 1.
- If representation unification requires more than 8 source files, STOP and ask for scope confirmation.
- If generic method or closure/lambda support is absent where required by `map`, `filter`, or `reduce`, STOP and report the missing prerequisite with exact files searched. Do not invent unrelated language features.
- If `Pair<T, U>` support is absent for `zip`, STOP THE ENTIRE PLAN before T12/F1-F4, write `.sisyphus/evidence/task-11-zip-pair-stop.md`, present the missing prerequisite to the user, and wait for an updated plan.

## Work Objectives
### Core Objective
Make Opalescent arrays usable end-to-end from source programs through typechecking, codegen, runtime execution, and test projects for array growth, mutation, transformations, zipping, and nested reads.

### Deliverables
- Test harness and integration tests for array test projects.
- Compiler/runtime support for `append`, `.push`, `.pop`, `.map`, `.filter`, `.reduce`, `.zip`.
- `T[][]` literal/access/length support for uniform and jagged arrays.
- Local git commit after each function slice and final push after all gates pass.

### Definition of Done (verifiable conditions with commands)
- `cargo test --all-features` passes.
- `cargo clippy --all-targets --all-features -- -D warnings` passes.
- `cargo fmt --all -- --check` passes.
- `cargo test --features integration array_` passes all new array integration tests.
- `cargo test --all-features` passes after the final slice.
- `git log --oneline` shows separate local commits for `append`, `push`, `pop`, `map`, `filter`, `reduce`, `zip`, and `double-arrays`.
- Final branch is pushed once to remote and `git status` reports clean.

### Must Have
- Red test project created and observed failing before implementation for each function slice.
- Every red failure must be new and attributable to the current slice, not pre-existing baseline failure.
- Each slice must include runtime/compiler unit coverage plus end-to-end test-project coverage when source-level behavior is involved.
- Panic/trap paths must be automated with expected nonzero exit/status or Rust `#[should_panic]`/error assertion.

### Must NOT Have
- No human-inspection acceptance criteria.
- No invented semantics beyond Decision Lock.
- No pushing after each function; only local commits until final gate.
- No changing `pop` to `Optional<T>` in v1.
- No error-propagating `map/filter/reduce` callbacks in v1.
- No indexed write support unless already present and required by a failing test.
- No formatter/lint-only broad refactors outside files touched by a slice.
- No force push, no skipped hooks, no destructive git operations.

## Verification Strategy
> ZERO HUMAN INTERVENTION - all verification is agent-executed.
- Test decision: TDD RED-GREEN-REFACTOR using Cargo/Rust tests plus Opalescent test projects.
- QA policy: Every task has agent-executed scenarios.
- Evidence: `.sisyphus/evidence/task-{N}-{slug}.{ext}`
- Per-slice gate shape:
  ```
  RED:    cargo test --features integration <test-name>  → expect failure containing "<slice-specific substring>"
  GREEN:  cargo test --features integration <test-name>  → expect test pass
  GATE:   cargo test --workspace && cargo clippy --all-targets --all-features -- -D warnings && cargo fmt --all -- --check
  COMMIT: git commit -m "<exact message>"
  VERIFY: git log -1 --format='%s' → expect "<exact message>"; git status --short → expect clean
  ```

## Execution Strategy
### Parallel Execution Waves
> Target: 5-8 tasks per wave. This plan is intentionally serial for implementation because every slice depends on representation/member-call foundations and user requested sequential RGR commits.

Wave 1: Tasks 1-3 — baseline, harness, representation foundation
Wave 2: Task 4 — member-call lowering
Wave 3: Tasks 5-11 — serial function slices (`append`, `push`, `pop`, `map`, `filter`, `reduce`, `zip`)
Wave 4: Task 12 — double arrays
Wave 5: Final verification F1-F4 and final push

### Dependency Matrix (full, all tasks)
- T1 blocks all tasks.
- T2 blocked by T1; blocks T5-T10 and T12. T11 creates its own zip fixture after Pair proof.
- T3 blocked by T1; blocks T4-T12.
- T4 blocked by T3; blocks T6-T11.
- T5 (`append`) blocked by T2-T3.
- T6 (`push`) blocked by T2-T4 and should run after T5.
- T7 (`pop`) blocked by T6.
- T8 (`map`) blocked by T7 and closure/generic STOP gates.
- T9 (`filter`) blocked by T8.
- T10 (`reduce`) blocked by T9.
- T11 (`zip`) blocked by T10 and Pair proof gate.
- T12 blocked by successful T11 GREEN commit. If T11 Pair proof fails, the entire plan stops before T12.
- Final verification blocked by T12.

### Agent Dispatch Summary (wave → task count → categories)
- Wave 1 → 3 tasks → `deep`, `quick`, `deep`
- Wave 2 → 1 task → `deep`
- Wave 3 → 7 tasks → `deep` for each serial function slice
- Wave 4 → 1 task → `deep`
- Wave 5 → 4 review agents/tools → oracle, oracle, Bash/general, oracle

## TODOs
> Implementation + Test = ONE task. Never separate.
> EVERY task MUST have: Agent Profile + Parallelization + QA Scenarios.

- [x] 1. Establish baseline, branch safety, and rollback points

  **What to do**: Verify the current repository state before adding red tests. Run `git status --short`, `git branch --show-current`, `git log --oneline -5`, `cargo test --workspace`, `cargo test --features integration`, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo fmt --all -- --check`. If baseline tests fail before any new test, STOP and report the failing command/output. If on `main` or `master`, create branch `array-functionality-tdd`; otherwise stay on current feature branch. Record current HEAD SHA as rollback point in `.sisyphus/evidence/task-1-baseline.txt`.
  **Must NOT do**: Do not add tests or edit source before baseline is green. Do not push. Do not use destructive git commands.

  **Recommended Agent Profile**:
  - Category: `deep` - Reason: Coordinates repo-wide verification and git safety before a large serial plan.
  - Skills: [] - No special skill required; follow git safety protocol.
  - Omitted: [`git-master`] - Not loaded directly because plan execution can use built-in git commands and must not skip hooks.

  **Parallelization**: Can Parallel: NO | Wave 1 | Blocks: all tasks | Blocked By: none

  **References**:
  - CI gate: `.github/workflows/ci.yml` - use CI-equivalent commands from research.
  - Test infra: `Cargo.toml` - feature-gated integration tests.
  - TDD precedent: `src/formatter/tests.rs` - existing expected-red test comments.

  **Acceptance Criteria**:
  - [ ] `cargo test --workspace` exits 0 before new tests.
  - [ ] `cargo test --features integration` exits 0 before new tests.
  - [ ] Clippy and fmt gates exit 0 before new tests.
  - [ ] Evidence file records branch, HEAD SHA, and command results.
  - [ ] If current branch was `main`/`master`, branch is now `array-functionality-tdd`.

  **QA Scenarios**:
  ```
  Scenario: Clean baseline
    Tool: Bash
    Steps: Run baseline commands exactly as listed; write stdout/stderr summaries to .sisyphus/evidence/task-1-baseline.txt.
    Expected: All commands exit 0; git status shows no source changes before T2.
    Evidence: .sisyphus/evidence/task-1-baseline.txt

  Scenario: Pre-existing failure
    Tool: Bash
    Steps: Run baseline commands; if any command exits nonzero, capture full failure.
    Expected: Execution stops before adding red tests; evidence contains failing command and output substring.
    Evidence: .sisyphus/evidence/task-1-baseline-failure.txt
  ```

  **Commit**: NO | Message: N/A | Files: `.sisyphus/evidence/task-1-baseline.txt` only

- [x] 2. Add reusable array integration-test harness and first red fixtures

  **What to do**: Create an integration harness modeled after `tests/fmt_integration.rs:7-84`, but for compiling/running array test projects. Add helpers in a new Rust integration test file such as `tests/array_integration.rs`: `binary_path()`, `array_project_src(project, filename)`, `run_opal_project(project)`, and `assert_stdout(project, expected)`. Create test-project directories before implementation for these function slices only: `test-projects/array-append`, `array-push`, `array-pop`, `array-map`, `array-filter`, `array-reduce`, each with `opal.toml`, `.gitignore`, `README.md`, `src/main.op`, and `expected/stdout.txt`. Do not create `test-projects/array-zip` in this task; Task 11 creates it only after proving the existing `Pair<T, U>` source syntax. Mark tests initially active, not ignored, so they fail RED. Use exact expected outputs below.
  **Must NOT do**: Do not implement compiler/runtime behavior in this task. Do not weaken tests to pass.

  **Recommended Agent Profile**:
  - Category: `quick` - Reason: Adds test scaffolding and failing fixtures only.
  - Skills: [] - No special skill required.
  - Omitted: [`playwright`] - No browser/UI involved.

  **Parallelization**: Can Parallel: NO | Wave 1 | Blocks: T5-T10, T12 | Blocked By: T1

  **References**:
  - Pattern: `tests/fmt_integration.rs:7-41` - helper functions for binary/test-project paths.
  - Pattern: `tests/fmt_integration.rs:51-84` - command invocation and assertion style.
  - Pattern: `test-projects/fmt-test/expected/*.expected.op` - checked-in golden files.
  - Convention: `README.md` Testing section - test project layout.

  **Acceptance Criteria**:
  - [ ] `tests/array_integration.rs` exists and compiles once array features are implemented.
  - [ ] Each listed `test-projects/array-*` directory for append/push/pop/map/filter/reduce exists with `src/main.op` and `expected/stdout.txt`.
  - [ ] `test-projects/array-zip` does not exist yet unless Task 11 Pair proof has already passed.
  - [ ] Running `cargo test --features integration array_append_runs` fails before `append` implementation with unknown symbol/member/codegen error.
  - [ ] Running `cargo test --features integration array_push_runs` fails before `.push` implementation with member-call/codegen error.
  - [ ] Evidence captures RED failures for all function projects.

  **QA Scenarios**:
  ```
  Scenario: RED fixtures fail for missing implementation
    Tool: Bash
    Steps: Run `cargo test --features integration array_ -- --nocapture` immediately after adding fixtures.
    Expected: Command exits nonzero; output contains at least one of `append`, `push`, `unsupported call callee`, or `unknown symbol`.
    Evidence: .sisyphus/evidence/task-2-array-fixtures-red.txt

  Scenario: Test project layout is complete
    Tool: Bash
    Steps: Run a script/check that verifies each of array-append,array-push,array-pop,array-map,array-filter,array-reduce has opal.toml, README.md, .gitignore, src/main.op, expected/stdout.txt, and verifies array-zip is absent/deferred.
    Expected: All required files exist; no implementation files changed except integration tests and test projects.
    Evidence: .sisyphus/evidence/task-2-layout.txt
  ```

  **Commit**: DEFER | Message: combine with first green foundation commit `test(array): add array integration fixtures` | Files: `tests/array_integration.rs`, `test-projects/array-{append,push,pop,map,filter,reduce}/*`

- [x] 3. Unify array representation and length/capacity metadata for new operations

  **What to do**: Before adding function semantics, inspect references with `lsp_find_references`/search for `OpalArray`, `pending_array_length`, `codegen_array_literal`, `codegen_array_access`, and array length side bindings. Implement the smallest representation bridge that lets compiler-lowered arrays carry data pointer + length + capacity together for new operations. Prefer extending runtime helpers in `src/runtime/memory.rs` / `src/runtime/arrays.rs` and codegen helpers in `src/codegen/expressions.rs` rather than continuing ad-hoc side channels. Preserve existing indexing behavior and tests. Add Rust unit tests for length/capacity invariants and bounds behavior.
  **Must NOT do**: Do not implement `append`, `.push`, `.pop`, or higher-order operations here except helper primitives needed by later slices. If full representation rewrite would touch more than 8 source files, STOP and ask for scope confirmation.

  **Recommended Agent Profile**:
  - Category: `deep` - Reason: Cross-cutting compiler/runtime foundation with high regression risk.
  - Skills: [] - No special skill required.
  - Omitted: [`frontend-ui-ux`] - No UI work.

  **Parallelization**: Can Parallel: NO | Wave 1 | Blocks: T4-T12 | Blocked By: T1

  **References**:
  - Runtime: `src/runtime/memory.rs:120` - current immutable `OpalArray<T>`.
  - Runtime helpers: `src/runtime/arrays.rs:9`, `:22`, `:31` - allocation, length, index.
  - Codegen side channel: `src/codegen/expressions.rs:68`, `src/codegen/expressions_loop.rs:35-41`, `src/codegen/statements.rs:166-183`.
  - Runtime tests: `src/runtime/tests.rs:127-156` - preserve current allocation/indexing/bounds semantics.

  **Acceptance Criteria**:
  - [ ] Existing `src/runtime/tests.rs:127-156` still passes.
  - [ ] New tests prove arrays retain correct length after allocation and helper-level growth/shrink operations.
  - [ ] No new operation relies solely on `pending_array_length` without carrying length with the value.
  - [ ] `cargo test --workspace` passes after foundation change.

  **QA Scenarios**:
  ```
  Scenario: Representation preserves existing indexing
    Tool: Bash
    Steps: Run `cargo test array_runtime_supports_allocation_indexing_and_bounds_checks`.
    Expected: Test passes; out-of-bounds remains `RuntimeError::IndexOutOfBounds { index: 8, length: 4 }`.
    Evidence: .sisyphus/evidence/task-3-existing-runtime-index.txt

  Scenario: Growth metadata invariant
    Tool: Bash
    Steps: Run new Rust unit test for helper allocation/growth/shrink metadata.
    Expected: Length updates exactly, capacity is never less than length, and stale side-channel length is not used.
    Evidence: .sisyphus/evidence/task-3-metadata-invariant.txt
  ```

  **Commit**: YES | Message: `refactor(array): unify array representation metadata` | Files: `src/runtime/memory.rs`, `src/runtime/arrays.rs`, `src/codegen/expressions.rs`, `src/codegen/expressions_loop.rs`, `src/codegen/statements.rs`, relevant tests

- [x] 4. Implement compiler lowering for array member calls

  **What to do**: Add RED test in `tests/array_integration.rs` using `test-projects/array-push/src/main.op` that currently fails at member-call codegen. Update `src/codegen/functions_call.rs:37` and internal `resolve_callee_function` so `Expr::Member` callees on arrays resolve through `resolve_collection_member_call` / `resolve_array_member_call` and dispatch to compiler-lowered array intrinsic handlers. Preserve identifier/lambda call behavior. Enforce mutable receiver requirement for mutating methods (`push`, `pop`) here or in checker with a clear diagnostic.
  **Must NOT do**: Do not implement individual method semantics beyond dispatch stubs that return clear unimplemented errors for methods not yet sliced.

  **Recommended Agent Profile**:
  - Category: `deep` - Reason: Compiler call resolution and type/codegen integration.
  - Skills: [] - No special skill required.
  - Omitted: [`playwright`] - No browser/UI involved.

  **Parallelization**: Can Parallel: NO | Wave 2 | Blocks: T6-T11 | Blocked By: T3

  **References**:
  - Type signatures: `src/type_system/checker/collections/collections_array.rs:128` (`resolve_array_member_call`).
  - Call codegen: `src/codegen/functions_call.rs:37` (`codegen_call_expression`).
  - Existing member registrations: `src/type_system/checker/collections/collections_array.rs:11-50`.

  **Acceptance Criteria**:
  - [ ] Member-call callee no longer fails with `unsupported call callee expression` for array methods.
  - [ ] Non-array member calls keep existing behavior.
  - [ ] Calling mutating array methods on immutable receiver fails with deterministic compile-time diagnostic.
  - [ ] Function slices still RED because semantics are not implemented yet.

  **QA Scenarios**:
  ```
  Scenario: Member call dispatch reaches array handler
    Tool: Bash
    Steps: Run `cargo test --features integration array_push_runs -- --nocapture` after member-call lowering but before push semantics.
    Expected: Failure changes from `unsupported call callee expression` to a slice-specific unimplemented/semantic failure mentioning `push`.
    Evidence: .sisyphus/evidence/task-4-member-dispatch-red-shift.txt

  Scenario: Immutable receiver rejected
    Tool: Bash
    Steps: Add/check a compile-fail fixture `let xs: int32[] = []; xs.push(1)`.
    Expected: Compiler exits nonzero with diagnostic containing `mutable` and `push`.
    Evidence: .sisyphus/evidence/task-4-immutable-receiver.txt
  ```

  **Commit**: YES | Message: `feat(array): lower array member calls` | Files: `src/codegen/functions_call.rs`, collection checker/codegen dispatch files, `tests/array_integration.rs`, plus deferred fixture files from T2

- [x] 5. Red-green-refactor `append`

  **What to do**: Use `test-projects/array-append` as RED first. Fixture source must include:
  ```op
  import append from standard

  entry main = f(args: string[]): void =>
      let original: int32[] = [1, 2]
      let grown = append(original, 3)
      print('original length {original.length}')
      print('grown length {grown.length}')
      print('grown values {grown[0]} {grown[1]} {grown[2]}')
      return void
  ```
  Expected stdout: `original length 2`, `grown length 3`, `grown values 1 2 3`. Register `append` in standard module resolution and typechecking, then lower it as a pure copy+append operation. Add a negative type test: `append([1], 'x')` fails with type mismatch. Refactor only helper duplication introduced in this slice. Commit locally with exact message below.
  **Must NOT do**: Do not optimize with COW/Perceus in v1. Do not mutate the original array.

  **Recommended Agent Profile**:
  - Category: `deep` - Reason: Standard import, typechecking, codegen, runtime behavior.
  - Skills: [] - No special skill required.
  - Omitted: [`playwright`] - No browser/UI involved.

  **Parallelization**: Can Parallel: NO | Wave 3 | Blocks: T6 | Blocked By: T2, T3

  **References**:
  - Spec: `ARRAY_FEATURES.md:9-59` and examples `ARRAY_FEATURES.md:61-153`.
  - Standard module resolver: `src/type_system/module_resolver/standard_modules.rs:20-26`, `src/type_system/module_resolver/standard_symbols_core_io_and_bytes.rs:894-895`, `src/type_system/module_resolver/standard_symbols_filesystem_operations.rs:605-606`.
  - Stdlib codegen declarations: `src/codegen/functions_stdlib.rs:474`, `src/codegen/functions_stdlib.rs:535-553`.
  - Runtime helpers: `src/runtime/arrays.rs:9` and representation helpers from T3.

  **Acceptance Criteria**:
  - [ ] RED: `cargo test --features integration array_append_runs -- --nocapture` fails before implementation with `append` unknown/unimplemented.
  - [ ] GREEN: same command passes and stdout exactly matches expected file.
  - [ ] Type mismatch negative test fails at check/type phase with clear incompatible element diagnostic.
  - [ ] Gate passes: `cargo test --workspace && cargo clippy --all-targets --all-features -- -D warnings && cargo fmt --all -- --check`.
  - [ ] Local commit exists with message `feat(array): implement append`.

  **QA Scenarios**:
  ```
  Scenario: append pure happy path
    Tool: Bash
    Steps: Run `cargo test --features integration array_append_runs -- --nocapture`.
    Expected: Test passes; stdout contains exactly original length 2, grown length 3, grown values 1 2 3.
    Evidence: .sisyphus/evidence/task-5-append-green.txt

  Scenario: append type mismatch
    Tool: Bash
    Steps: Run compile/check negative fixture using `append([1], 'x')`.
    Expected: Nonzero exit; diagnostic contains incompatible array element/value types.
    Evidence: .sisyphus/evidence/task-5-append-type-error.txt
  ```

  **Commit**: YES | Message: `feat(array): implement append` | Files: append resolver/typecheck/codegen/runtime files plus `test-projects/array-append`, `tests/array_integration.rs`

- [x] 6. Red-green-refactor `.push`

  **What to do**: Use `test-projects/array-push` as RED first. Fixture source:
  ```op
  entry main = f(args: string[]): void =>
      let mutable xs: int32[] = []
      xs.push(10)
      xs.push(25)
      xs.push(8)
      print('length {xs.length}')
      print('values {xs[0]} {xs[1]} {xs[2]}')
      return void
  ```
  Expected stdout: `length 3`, `values 10 25 8`. Implement mutating push on mutable array locals, updating value and length/capacity atomically. Add compile-fail fixture for immutable receiver. Refactor only push/helper code. Commit locally.
  **Must NOT do**: Do not allow `.push` on immutable bindings or temporaries. Do not make `.push` failable.

  **Recommended Agent Profile**:
  - Category: `deep` - Reason: Mutating array codegen and receiver mutability semantics.
  - Skills: [] - No special skill required.
  - Omitted: [`playwright`] - No UI.

  **Parallelization**: Can Parallel: NO | Wave 3 | Blocks: T7 | Blocked By: T4, T5

  **References**:
  - Spec: `ARRAY_FEATURES.md:155-241`.
  - Type registration: `src/type_system/checker/collections/collections_array.rs:11`.
  - Existing stdlib tests: `src/stdlib/collections/tests.rs:38-44` (`test_vec_push_increments_length`).

  **Acceptance Criteria**:
  - [ ] RED failure captured before implementation with member/unimplemented/codegen error for `.push`.
  - [ ] GREEN integration test prints exact length/values.
  - [ ] `.push` returns `void` and cannot be used where `int32`/array is expected.
  - [ ] Immutable receiver negative test fails.
  - [ ] Gate passes and local commit message is `feat(array): implement push`.

  **QA Scenarios**:
  ```
  Scenario: push grows empty array
    Tool: Bash
    Steps: Run `cargo test --features integration array_push_runs -- --nocapture`.
    Expected: stdout exactly `length 3` and `values 10 25 8`.
    Evidence: .sisyphus/evidence/task-6-push-green.txt

  Scenario: push rejects immutable receiver
    Tool: Bash
    Steps: Run compile-fail fixture with `let xs: int32[] = []; xs.push(1)`.
    Expected: Nonzero exit; diagnostic contains `mutable` and `push`.
    Evidence: .sisyphus/evidence/task-6-push-immutable-error.txt
  ```

  **Commit**: YES | Message: `feat(array): implement push` | Files: array method codegen/typecheck/runtime files plus `test-projects/array-push`, `tests/array_integration.rs`

- [x] 7. Red-green-refactor `.pop`

  **What to do**: Use `test-projects/array-pop` as RED first. Fixture source:
  ```op
  entry main = f(args: string[]): void =>
      let mutable xs: int32[] = [4, 5, 6]
      let last = xs.pop()
      print('last {last}')
      print('length {xs.length}')
      print('remaining {xs[0]} {xs[1]}')
      return void
  ```
  Expected stdout: `last 6`, `length 2`, `remaining 4 5`. Add process-level negative fixture `test-projects/array-pop-empty` or Rust integration case that runs `let mutable xs: int32[] = []; xs.pop()` and expects nonzero exit/stderr containing `pop on empty array`. Implement trap via existing runtime error/reporting path; if no reusable panic/reporting path exists, add one minimal runtime helper and test it.
  **Must NOT do**: Do not change return type to optional. Do not silently return zero/default on empty.

  **Recommended Agent Profile**:
  - Category: `deep` - Reason: Mutating operation plus runtime trap path.
  - Skills: [] - No special skill required.
  - Omitted: [`playwright`] - No UI.

  **Parallelization**: Can Parallel: NO | Wave 3 | Blocks: T8 | Blocked By: T6

  **References**:
  - Type registration: `src/type_system/checker/collections/collections_array.rs:11`.
  - Existing stdlib tests: `src/stdlib/collections/tests.rs:46-61` (`test_vec_pop_returns_last`, `test_vec_pop_empty_returns_none`) but language `T[]` semantics differ on empty by Decision Lock.
  - Runtime error pattern: `src/runtime/tests.rs:146-155` and `src/runtime/tests.rs:188-218`.

  **Acceptance Criteria**:
  - [ ] RED failure captured before implementation.
  - [ ] Happy-path pop integration test passes.
  - [ ] Empty pop negative test exits nonzero or panics with `pop on empty array`.
  - [ ] Length decrements and popped slot cannot be read through valid index.
  - [ ] Gate passes and local commit message is `feat(array): implement pop`.

  **QA Scenarios**:
  ```
  Scenario: pop returns last and shrinks
    Tool: Bash
    Steps: Run `cargo test --features integration array_pop_runs -- --nocapture`.
    Expected: stdout exactly `last 6`, `length 2`, `remaining 4 5`.
    Evidence: .sisyphus/evidence/task-7-pop-green.txt

  Scenario: pop empty traps
    Tool: Bash
    Steps: Run empty-pop integration test.
    Expected: Process/test observes nonzero exit or panic; stderr/message contains `pop on empty array`.
    Evidence: .sisyphus/evidence/task-7-pop-empty-trap.txt
  ```

  **Commit**: YES | Message: `feat(array): implement pop` | Files: pop codegen/runtime/typecheck files plus `test-projects/array-pop*`, `tests/array_integration.rs`

- [x] 8. Red-green-refactor `.map`

  **What to do**: First verify closure/lambda and generic return support. If compiler cannot represent `map<U>(f(T): U): U[]` without new generic method infrastructure, STOP and report. Use `test-projects/array-map` as RED first. Fixture source:
  ```op
  entry main = f(args: string[]): void =>
      let xs: int32[] = [1, 2, 3]
      let doubled = xs.map(f(x: int32): int32 => return x * 2)
      print('length {doubled.length}')
      print('values {doubled[0]} {doubled[1]} {doubled[2]}')
      return void
  ```
  Expected stdout: `length 3`, `values 2 4 6`. Add empty array fixture: `let xs: int32[] = []; let out = xs.map(...); print(out.length)` expects `0`. Lower map as a compiler-generated loop, not generic runtime function-pointer dispatch.
  **Must NOT do**: Do not support error-declaring callbacks in v1. Do not add broad closure-capture semantics if absent.

  **Recommended Agent Profile**:
  - Category: `deep` - Reason: Higher-order compiler lowering and generic result arrays.
  - Skills: [] - No special skill required.
  - Omitted: [`playwright`] - No UI.

  **Parallelization**: Can Parallel: NO | Wave 3 | Blocks: T9 | Blocked By: T7

  **References**:
  - Transform signatures: `src/type_system/checker/collections/collections_array.rs:50`.
  - Existing stdlib test: `src/stdlib/collections/tests.rs:139-162` (`test_vec_map_doubles`).
  - Call lowering: `src/codegen/functions_call.rs:37`.

  **Acceptance Criteria**:
  - [ ] RED failure captured before implementation.
  - [ ] Happy-path map prints expected doubled values.
  - [ ] Empty map returns empty array.
  - [ ] Callback returning wrong type relative to expected use produces deterministic type error.
  - [ ] Gate passes and local commit message is `feat(array): implement map`.

  **QA Scenarios**:
  ```
  Scenario: map doubles integers
    Tool: Bash
    Steps: Run `cargo test --features integration array_map_runs -- --nocapture`.
    Expected: stdout exactly `length 3` and `values 2 4 6`.
    Evidence: .sisyphus/evidence/task-8-map-green.txt

  Scenario: map empty input
    Tool: Bash
    Steps: Run empty-map integration case.
    Expected: stdout exactly `length 0`.
    Evidence: .sisyphus/evidence/task-8-map-empty.txt
  ```

  **Commit**: YES | Message: `feat(array): implement map` | Files: map type/codegen/runtime helper files plus `test-projects/array-map`, `tests/array_integration.rs`

- [x] 9. Red-green-refactor `.filter`

  **What to do**: Use `test-projects/array-filter` as RED first. Fixture source:
  ```op
  entry main = f(args: string[]): void =>
      let xs: int32[] = [1, 2, 3, 4]
      let evens = xs.filter(f(x: int32): boolean => return x % 2 == 0)
      print('length {evens.length}')
      print('values {evens[0]} {evens[1]}')
      let none = xs.filter(f(x: int32): boolean => return x > 10)
      print('none length {none.length}')
      return void
  ```
  Expected stdout: `length 2`, `values 2 4`, `none length 0`. Add all-pass fixture or assertion for `x > 0` returning length 4. Lower as compiler-generated loop using `.push`/growth helper internally.
  **Must NOT do**: Do not accept non-boolean predicates. Do not mutate source array.

  **Recommended Agent Profile**:
  - Category: `deep` - Reason: Higher-order predicate lowering and dynamic result size.
  - Skills: [] - No special skill required.
  - Omitted: [`playwright`] - No UI.

  **Parallelization**: Can Parallel: NO | Wave 3 | Blocks: T10 | Blocked By: T8

  **References**:
  - Transform signatures: `src/type_system/checker/collections/collections_array.rs:50`.
  - Existing stdlib test: `src/stdlib/collections/tests.rs:164-178` (`test_vec_filter_evens`).

  **Acceptance Criteria**:
  - [ ] RED failure captured before implementation.
  - [ ] Filter keeps matching values in original order.
  - [ ] All-pass, none-pass, and empty-input cases pass.
  - [ ] Non-boolean predicate negative test fails at typecheck.
  - [ ] Gate passes and local commit message is `feat(array): implement filter`.

  **QA Scenarios**:
  ```
  Scenario: filter evens and none
    Tool: Bash
    Steps: Run `cargo test --features integration array_filter_runs -- --nocapture`.
    Expected: stdout exactly `length 2`, `values 2 4`, `none length 0`.
    Evidence: .sisyphus/evidence/task-9-filter-green.txt

  Scenario: filter predicate type error
    Tool: Bash
    Steps: Run compile-fail fixture where predicate returns int32.
    Expected: Nonzero exit; diagnostic contains `boolean` and `filter`.
    Evidence: .sisyphus/evidence/task-9-filter-type-error.txt
  ```

  **Commit**: YES | Message: `feat(array): implement filter` | Files: filter type/codegen/runtime helper files plus `test-projects/array-filter`, `tests/array_integration.rs`

- [x] 10. Red-green-refactor `.reduce`

  **What to do**: Use seeded signature from Decision Lock. Use `test-projects/array-reduce` as RED first. Fixture source:
  ```op
  entry main = f(args: string[]): void =>
      let xs: int32[] = [1, 2, 3]
      let sum = xs.reduce(0, f(acc: int32, x: int32): int32 => return acc + x)
      print('sum {sum}')
      let empty: int32[] = []
      let seed = empty.reduce(99, f(acc: int32, x: int32): int32 => return acc + x)
      print('empty {seed}')
      return void
  ```
  Expected stdout: `sum 6`, `empty 99`. Add negative fixture where reducer returns wrong accumulator type. Lower as compiler-generated loop over source array.
  **Must NOT do**: Do not implement unseeded reduce. Do not trap on empty seeded reduce.

  **Recommended Agent Profile**:
  - Category: `deep` - Reason: Higher-order accumulator semantics and type consistency.
  - Skills: [] - No special skill required.
  - Omitted: [`playwright`] - No UI.

  **Parallelization**: Can Parallel: NO | Wave 3 | Blocks: T11 | Blocked By: T9

  **References**:
  - Transform signatures: `src/type_system/checker/collections/collections_array.rs:50`.
  - Existing stdlib tests: `src/stdlib/collections/tests.rs:180-199` (`test_vec_reduce_sum`, `test_vec_reduce_empty_returns_initial`).

  **Acceptance Criteria**:
  - [ ] RED failure captured before implementation.
  - [ ] Seeded reduce sums `[1,2,3]` to `6`.
  - [ ] Empty seeded reduce returns seed unchanged.
  - [ ] Reducer return/accumulator mismatch fails typecheck.
  - [ ] Gate passes and local commit message is `feat(array): implement reduce`.

  **QA Scenarios**:
  ```
  Scenario: reduce sum and empty seed
    Tool: Bash
    Steps: Run `cargo test --features integration array_reduce_runs -- --nocapture`.
    Expected: stdout exactly `sum 6` and `empty 99`.
    Evidence: .sisyphus/evidence/task-10-reduce-green.txt

  Scenario: reduce accumulator mismatch
    Tool: Bash
    Steps: Run compile-fail fixture where reducer returns string for int32 seed.
    Expected: Nonzero exit; diagnostic contains accumulator/return type mismatch.
    Evidence: .sisyphus/evidence/task-10-reduce-type-error.txt
  ```

  **Commit**: YES | Message: `feat(array): implement reduce` | Files: reduce type/codegen/runtime helper files plus `test-projects/array-reduce`, `tests/array_integration.rs`

- [x] 11. Red-green-refactor `.zip`

  **What to do**: Before creating any zip fixture or implementation, prove the existing `Pair<T, U>` return shape is language-visible by searching `src/type_system/checker/collections/collections_array.rs:110`, `src/type_system/test_integration_generics.rs:120-136`, and parser/codegen support for constructing/accessing generic ADTs from source. If `Pair` is not language-visible without adding a new built-in product type, STOP and update the plan/user before creating `test-projects/array-zip` or changing zip implementation. If supported, create `test-projects/array-zip` as RED first in this task with `opal.toml`, `.gitignore`, `README.md`, `src/main.op`, and `expected/stdout.txt`. Fixture source must use the proven existing Pair field-access syntax. Intended behavior below assumes the generic ADT precedent fields `first` and `second`; replace only if the proof finds different existing field names.
  ```op
  entry main = f(args: string[]): void =>
      let left: int32[] = [1, 2, 3]
      let right: string[] = ['a', 'b']
      let pairs = left.zip(right)
      print('length {pairs.length}')
      print('first {pairs[0].first} {pairs[0].second}')
      print('second {pairs[1].first} {pairs[1].second}')
      return void
  ```
  Expected stdout: `length 2`, `first 1 a`, `second 2 b`. Implement truncate-to-shorter semantics. Add equal-length and empty-input cases.
  **Must NOT do**: Do not pad, error, or require equal lengths. Do not invent tuple syntax/type if missing.

  **Recommended Agent Profile**:
  - Category: `deep` - Reason: Binary array operation plus possible tuple dependency.
  - Skills: [] - No special skill required.
  - Omitted: [`playwright`] - No UI.

  **Parallelization**: Can Parallel: NO | Wave 3 | Blocks: T12 | Blocked By: T10

  **References**:
  - Transform signatures: `src/type_system/checker/collections/collections_array.rs:50` and `src/type_system/checker/collections/collections_array.rs:110` (`Pair` return shape).
  - Generic ADT precedent: `src/type_system/test_integration_generics.rs:120-136` defines and constructs `Pair<T, U>`.
  - Tuple-pattern-only warning: `src/ast/patterns.rs:43-44` and `src/parser/patterns.rs:156-176`; do not assume tuple expression support from tuple pattern support.
  - Iterator precedent: `src/stdlib/collections/tests.rs:13` mentions iterator `zip`; inspect nearby tests before implementation.

  **Acceptance Criteria**:
  - [ ] Prerequisite proof captured: either `Pair` is language-visible with exact syntax, or execution stops before zip implementation with evidence and user-facing plan update.
  - [ ] RED failure captured before implementation if prerequisite proof passes.
  - [ ] Unequal lengths truncate to shorter length.
  - [ ] Equal length preserves all pairs in order.
  - [ ] Empty left or right returns empty zipped array.
  - [ ] If Pair proof passes: gate passes and local commit message is `feat(array): implement zip`.
  - [ ] If Pair proof fails: `.sisyphus/evidence/task-11-zip-pair-stop.md` exists, no zip fixture/implementation commit exists, and execution stops before T12/F1-F4.

  **QA Scenarios**:
  ```
  Scenario: Pair prerequisite missing STOP path
    Tool: Bash
    Steps: Search the referenced Pair/generic ADT/parser/codegen files; if no language-visible Pair construction/access path exists, write `.sisyphus/evidence/task-11-zip-pair-stop.md` with files searched and missing capability, then stop execution before creating `test-projects/array-zip`.
    Expected: Evidence exists; no zip fixture directory exists; no `feat(array): implement zip` commit exists; T12 and F1-F4 are not started.
    Evidence: .sisyphus/evidence/task-11-zip-pair-stop.md

  Scenario: zip truncates unequal arrays
    Tool: Bash
    Steps: Run `cargo test --features integration array_zip_runs -- --nocapture`.
    Expected: stdout exactly `length 2`, `first 1 a`, `second 2 b`.
    Evidence: .sisyphus/evidence/task-11-zip-green.txt

  Scenario: zip empty side
    Tool: Bash
    Steps: Run empty-left and empty-right integration cases.
    Expected: Both print `length 0`.
    Evidence: .sisyphus/evidence/task-11-zip-empty.txt
  ```

  **Commit**: YES only if Pair prerequisite proof passes; otherwise STOP with no commit | Message: `feat(array): implement zip` | Files: zip type/codegen/runtime helper files plus `test-projects/array-zip`, `tests/array_integration.rs`

- [ ] 12. Red-green-refactor double-array literals, lengths, and reads

  **What to do**: Create `test-projects/array-double` only after all function slices pass, including successful Task 11 zip implementation. If Task 11 stopped for missing language-visible `Pair`, do not start this task. Add RED integration test `array_double_runs`. Fixture source must cover uniform 3x3, jagged with empty row, single row, single column, and empty outer array:
  ```op
  entry main = f(args: string[]): void =>
      let uniform: int32[][] = [[1,2,3],[4,5,6],[7,8,9]]
      print('uniform outer {uniform.length}')
      print('uniform inner {uniform[1].length}')
      print('uniform value {uniform[2][1]}')

      let jagged: int32[][] = [[1,2], [], [3,4,5]]
      print('jagged outer {jagged.length}')
      print('jagged row0 {jagged[0].length}')
      print('jagged row1 {jagged[1].length}')
      print('jagged row2 {jagged[2].length}')
      print('jagged value {jagged[2][2]}')

      let single_row: int32[][] = [[10,11,12]]
      print('single row {single_row.length} {single_row[0].length} {single_row[0][2]}')

      let single_col: int32[][] = [[20],[21],[22]]
      print('single col {single_col.length} {single_col[2].length} {single_col[2][0]}')

      let empty_outer: int32[][] = []
      print('empty outer {empty_outer.length}')
      return void
  ```
  Expected stdout lines: `uniform outer 3`, `uniform inner 3`, `uniform value 8`, `jagged outer 3`, `jagged row0 2`, `jagged row1 0`, `jagged row2 3`, `jagged value 5`, `single row 1 3 12`, `single col 3 1 22`, `empty outer 0`. Update `codegen_array_literal` recursively for nested `CoreType::Array`, `codegen_array_access` to preserve/load per-row length, and `.length` on extracted rows. Add out-of-bounds nested access negative test.
  **Must NOT do**: Do not implement indexed write. Do not enforce rectangular arrays. Do not share mutable row aliases in a way that violates value semantics.

  **Recommended Agent Profile**:
  - Category: `deep` - Reason: Nested aggregate codegen, recursive literals, per-row length tracking.
  - Skills: [] - No special skill required.
  - Omitted: [`playwright`] - No UI.

  **Parallelization**: Can Parallel: NO | Wave 4 | Blocks: Final verification | Blocked By: T11

  **References**:
  - Spec: `ARRAY_FEATURES.md:245-295`, `ARRAY_FEATURES.md:445-484`.
  - Literal codegen: `src/codegen/expressions.rs:638`.
  - Access codegen: `src/codegen/expressions.rs:690`.
  - Length side-channel helpers: `src/codegen/expressions_loop.rs:35-41`.

  **Acceptance Criteria**:
  - [ ] RED failure captured before double-array implementation.
  - [ ] Uniform, jagged, single-row, single-column, and empty-outer cases pass exactly.
  - [ ] `grid[row].length` uses row-specific length, not outer/static length.
  - [ ] Nested out-of-bounds test reports bounds error on correct dimension.
  - [ ] Gate passes and local commit message is `feat(array): support double arrays`.
  - [ ] Final push occurs only after this task and all gates pass.

  **QA Scenarios**:
  ```
  Scenario: double-array varieties
    Tool: Bash
    Steps: Run `cargo test --features integration array_double_runs -- --nocapture`.
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
- [ ] F1. Plan Compliance Audit — oracle
  - Verify every Decision Lock item is implemented or explicitly stopped with evidence.
  - Verify every function slice has RED evidence before implementation evidence.

  **QA Scenarios**:
  ```
  Scenario: Decision Lock compliance audit
    Tool: task(subagent_type="oracle")
    Steps: Invoke oracle with the final diff, `.sisyphus/plans/array-functionality.md`, and evidence directory. Ask it to check each Decision Lock bullet against implementation and evidence.
    Expected: Oracle returns APPROVE with zero unmet Decision Lock items, or returns a concrete numbered rejection list that must be fixed before completion.
    Evidence: .sisyphus/evidence/f1-plan-compliance-audit.md

  Scenario: RED-before-GREEN evidence audit
    Tool: Bash + task(subagent_type="oracle")
    Steps: List `.sisyphus/evidence/task-*-red*.txt` and `.sisyphus/evidence/task-*-green*.txt`; provide listing plus git log to oracle for ordering verification.
    Expected: Every function slice has RED evidence timestamped/committed before its GREEN evidence and local implementation commit.
    Evidence: .sisyphus/evidence/f1-red-green-order.txt
  ```

- [ ] F2. Code Quality Review — oracle
  - Review compiler/runtime changes for representation consistency, no duplicated side-channel hacks, clear diagnostics, and minimal refactor scope.

  **QA Scenarios**:
  ```
  Scenario: Static gate and diff quality review
    Tool: Bash + task(subagent_type="oracle")
    Steps: Run `cargo test --all-features`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo fmt --all -- --check`, and `git diff --check`; provide command outputs and full branch diff to reviewer.
    Expected: All commands exit 0; reviewer returns APPROVE with no representation-consistency, diagnostic-quality, or scope-creep blockers.
    Evidence: .sisyphus/evidence/f2-code-quality-review.md

  Scenario: Array representation consistency check
    Tool: task(subagent_type="oracle")
    Steps: Ask reviewer to inspect all changed references to `OpalArray`, `pending_array_length`, `codegen_array_literal`, `codegen_array_access`, and array method lowering.
    Expected: Reviewer confirms no new user-facing operation depends solely on stale side-channel length metadata and no duplicated ad-hoc array representation was introduced.
    Evidence: .sisyphus/evidence/f2-representation-consistency.md
  ```

- [ ] F3. Real Manual QA — Bash + general
  - Run all array test projects via CLI, not only Cargo assertions; capture stdout/stderr into `.sisyphus/evidence/f3-array-cli-qa.txt`.

  **QA Scenarios**:
  ```
  Scenario: CLI run all positive array projects
    Tool: Bash
    Steps: First assert `.sisyphus/evidence/task-11-zip-pair-stop.md` does not exist. Build debug binary, then run CLI execution for `test-projects/array-append/src/main.op`, `array-push/src/main.op`, `array-pop/src/main.op`, `array-map/src/main.op`, `array-filter/src/main.op`, `array-reduce/src/main.op`, `array-zip/src/main.op`, and `array-double/src/main.op`; compare stdout to each project's `expected/stdout.txt`.
    Expected: Stop evidence is absent; every positive project exits 0 and stdout exactly equals its expected file.
    Evidence: .sisyphus/evidence/f3-array-cli-qa.txt

  Scenario: CLI run negative array projects
    Tool: Bash
    Steps: Run CLI/check commands for negative fixtures: append type mismatch, immutable push, empty pop, map callback mismatch, filter non-boolean predicate, reduce accumulator mismatch, zip tuple STOP evidence if applicable, and nested bounds failure.
    Expected: Each negative fixture exits nonzero with the exact diagnostic substring specified in its task evidence; empty pop includes `pop on empty array`; nested bounds includes index 0 out of bounds for length 0.
    Evidence: .sisyphus/evidence/f3-array-negative-cli-qa.txt
  ```

- [ ] F4. Scope Fidelity Check — oracle
  - Confirm no indexed writes, no error-propagating HOF callbacks, no broad tuple invention, no COW/Perceus scope creep, no final completion before user approval.

  **QA Scenarios**:
  ```
  Scenario: Out-of-scope feature audit
    Tool: task(subagent_type="oracle")
    Steps: Provide final branch diff and plan Decision Lock to deep reviewer; ask it to search for indexed assignment implementation, `try_map`/`try_filter`/`try_reduce`, error-propagating HOF callback support, COW/Perceus implementation, force-push usage, and tuple/product-type invention outside zip's existing prerequisites.
    Expected: Reviewer returns APPROVE confirming none of the out-of-scope features were implemented; any finding is a blocker to fix or explicitly present to user.
    Evidence: .sisyphus/evidence/f4-scope-fidelity.md

  Scenario: User-approval gate audit
    Tool: Bash + task(subagent_type="oracle")
    Steps: Inspect final task status/evidence and git push evidence; verify F1-F4 were not marked complete before consolidated results were presented to user and explicit user `okay` was received.
    Expected: No final completion marker exists before user approval; if absent approval, execution stops after presenting verification results.
    Evidence: .sisyphus/evidence/f4-user-approval-gate.txt
  ```

## Commit Strategy
- Pre-slice foundation tasks may be committed as needed only after green gates, using messages:
  - `test(array): add array integration fixtures`
  - `refactor(array): unify array representation metadata`
  - `feat(array): lower array member calls`
- Required local function commits:
  - `feat(array): implement append`
  - `feat(array): implement push`
  - `feat(array): implement pop`
  - `feat(array): implement map`
  - `feat(array): implement filter`
  - `feat(array): implement reduce`
  - `feat(array): implement zip` — required after Task 11 proves `Pair<T, U>` is language-visible. If Task 11 proves it is not language-visible, STOP THE ENTIRE PLAN before T12/final verification and do not fabricate this commit.
  - `feat(array): support double arrays`
- Before each commit: run slice GREEN command, full gate, `git diff --check`, then commit. Never use `--no-verify`.
- After final verification and user approval: push once. If branch has upstream: `git push`. If branch was created as `array-functionality-tdd`: `git push -u origin array-functionality-tdd`.

## Success Criteria
- All array behavior in Decision Lock is covered by automated tests. The only allowed exception is the Task 11 Pair-prerequisite STOP path, which stops the entire plan before T12/final verification and requires user-facing evidence plus a plan update before work continues.
- All new test projects fail RED before relevant implementation and pass GREEN after.
- `cargo test --all-features`, clippy, and fmt all pass.
- Double arrays support jagged/uniform read/length behavior with per-row lengths.
- Final verification agents approve and user explicitly approves consolidated results before work is considered complete.
