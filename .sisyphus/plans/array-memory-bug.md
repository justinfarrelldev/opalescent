# Array Memory Bug: RC-Backed Arrays and Ergonomics

## TL;DR
> **Summary**: Replace Opalescent's split raw-pointer array lowering with one RC-backed heap array representation that carries `len`/`cap` in the array payload. Preserve functional `append`, implement `.push` and identifier-backed indexed assignment as copy-on-write rebinding, add `array_filled`/`reserve`/`clear`, and require TDD plus sanitizer-backed memory verification.
> **Deliverables**:
> - RC-backed array payload ABI using existing `opal_rc` header/runtime
> - Heap-backed literals, functional `append`, COW `.push`, COW `xs[i] = value`
> - `array_filled(length, value)`, `reserve(xs, capacity)`, and `clear(xs)` ergonomics
> - Sidecar metadata retirement for arrays (`pending_array_metadata`, array `_len`/`_cap` bindings)
> - RED-GREEN-REFACTOR tests, ASAN/LSAN memory checks, atomic commits, final clean git status
> **Effort**: XL
> **Parallel**: YES - 8 waves after the initial representation spike; testing and docs/audit tasks can run alongside implementation slices once the ABI is pinned
> **Critical Path**: T1 layout/spec tests → T2 runtime ABI → T3 literal lowering → T4 append migration → T5 push COW → T6 indexed assignment → T7 ergonomics → T8 sidecar retirement → T9 sanitizer automation → final verification

## Context
### Original Request
The user asked to fix the array memory bug using the most robust design that scales as the language grows, specifically deciding between an RC-backed array header and a `{ptr, len, cap, ownership/refcount}` system based on the language spec and test projects. The user confirmed that adjacent ergonomics are in scope and that the implementation must use TDD with RED-GREEN-REFACTOR. The user also required atomic commits and a final completely clean `git status`.

### Interview Summary
- Scope: include the core array memory model fix plus indexed assignment codegen and `array_filled`/`reserve`/`clear` ergonomics.
- Test strategy: TDD, with RED-GREEN-REFACTOR in each feature slice.
- Git strategy: atomic commits after coherent green slices; final `git status --porcelain` must be empty.
- Architecture: choose RC-backed heap array payload via existing `opal_rc`; reject a parallel sidecar ownership/refcount model.
- Semantics: keep `append(xs, value)` functional; lower `.push` and identifier-backed `xs[i] = value` as copy-on-write rebinding in v1.

### Metis Review (gaps addressed)
- Pinned exact layout: `T[]` ABI is a single pointer to an `opal_rc`-managed payload whose bytes begin with `size_t len; size_t cap;` followed by flexible element storage `T elems[]` with element alignment handled by runtime/codegen helpers.
- Pinned zero-length representation: allocate a real RC array object with `len=0`, `cap=0` in v1; no NULL/static sentinel.
- Pinned COW policy: `.push`, indexed assignment, `reserve`, and `clear` clone/rebind unconditionally in v1. Unique-owner/spare-capacity in-place mutation is explicitly out of scope and must be a follow-up.
- Pinned ergonomic signatures: `array_filled(length, value) -> T[]`; `reserve(xs, capacity) -> T[]`; `clear(xs) -> T[]`.
- Pinned indexed assignment scope: only identifier-backed `xs[i] = value` is in scope; `obj.field[i] = value`, `xs[i][j] = value`, and slice assignment are out.
- Added hard-cutover guardrail: no feature flag and no dual array lowering path.
- Added sanitizer automation, sidecar-retirement audit, `.gitignore` audit, and final clean git status gates.
- Added RED+GREEN commit policy: keep TDD discipline locally, but commit only green, atomic slices; do not commit failing RED states.

## Work Objectives
### Core Objective
Make Opalescent arrays memory-safe, ownership-aware, and scalable by replacing raw element-pointer plus compiler-side metadata with one RC-backed heap array representation integrated with existing `opal_rc` runtime/codegen infrastructure.

### Deliverables
- Runtime C ABI for RC-managed arrays with payload layout:
  ```c
  typedef struct OpalArrayPayloadHeader {
      size_t len;
      size_t cap;
      /* elems follow, aligned for element type */
  } OpalArrayPayloadHeader;
  ```
  `T[]` values are a single pointer to this payload header, and the `opal_rc` header remains immediately before the payload as implemented by `opal_rc_alloc`.
- Compiler/codegen support for:
  - heap-backed array literals
  - array length/capacity reads from payload header
  - functional `append(xs, value)`
  - COW `.push(value)` rebinding mutable identifier receivers
  - COW identifier-backed indexed assignment `xs[i] = value`
  - `array_filled(length, value)`, `reserve(xs, capacity)`, `clear(xs)` returning arrays
- Element ownership rules:
  - Retain RC-bearing elements when inserted into an array literal, copied during append/push/reserve/COW clone, or repeated by `array_filled`.
  - Release old RC-bearing elements on indexed assignment overwrite and during array drop.
  - Release all live RC-bearing elements when `clear(xs)` returns a cleared copy.
  - Drop arrays through `opal_rc_dec` / `opal_rc_drop_iterative` with array-specific child release behavior.
- Tests:
  - Rust unit/codegen tests and Opalescent integration test-projects added before implementation in each slice.
  - Sanitizer-backed memory checks for array churn and RC-bearing element scenarios.
- Git:
  - Atomic conventional commits per green slice.
  - Final `git status --porcelain` returns empty.

### Definition of Done (verifiable conditions with commands)
- `cargo test --features integration --test array_integration` exits 0.
- `cargo test` exits 0.
- New array memory test-projects run successfully via the repository's existing integration harness.
- ASAN/LSAN run for the new array memory test-projects exits 0 and prints no sanitizer error report on stderr.
- `rg --type rust 'pending_array_metadata' src/codegen src/type_system` returns no lines.
- `rg --type rust '(_len|_cap)' src/codegen` returns no array sidecar metadata references; unrelated string length/capacity locals may remain only if not tied to array bindings and are documented in the audit evidence.
- `git status --porcelain` returns an empty string after final commit.

### Must Have
- One hard-cutover RC-backed array representation; no feature flag.
- All array literals heap-backed through `opal_rc_alloc` or a runtime wrapper that calls it.
- Value-carried `len`/`cap`; array bounds checks read from the payload header.
- `append` pure-functional and alias-preserving.
- `.push` and `xs[i] = value` alias-preserving through unconditional COW rebinding in v1.
- Identifier-backed indexed assignment codegen only.
- Exact retain/release handling for RC-bearing elements at every array operation site.
- TDD sequence preserved in work logs/evidence; commits contain green slices only.
- Atomic commits after each slice with conventional messages.

### Must NOT Have (guardrails, AI slop patterns, scope boundaries)
- Must NOT keep mixed stack-backed and RC-backed arrays.
- Must NOT keep array sidecar `_len`/`_cap` or `pending_array_metadata` paths after migration.
- Must NOT introduce a separate array-only ownership/refcount system.
- Must NOT implement a feature flag or dual array codegen path.
- Must NOT use `opal_rc_reuse` or unique-owner in-place fast paths in this plan.
- Must NOT expand indexed assignment to `obj.field[i] = value`, `xs[i][j] = value`, slice assignment, generic container traits, or stable external ABI work.
- Must NOT commit failing RED tests; RED+GREEN+REFACTOR are squashed/committed as one green atomic slice.
- Must NOT rely on manual verification.

## Verification Strategy
> ZERO HUMAN INTERVENTION - all verification is agent-executed.
- Test decision: TDD / RED-GREEN-REFACTOR using existing Cargo unit/integration tests and new test-projects.
- QA policy: Every task has agent-executed scenarios.
- Evidence: `.sisyphus/evidence/task-{N}-{slug}.{ext}`.
- Sanitizer policy: add executable sanitizer script or documented command wrapper under repo conventions; generated logs must be stored as evidence and not left untracked at completion.
- Git policy: after every committed slice, run `git status --porcelain`; final task requires empty output.

## Execution Strategy
### Parallel Execution Waves
> Target: 5-8 tasks per wave. <3 per wave (except final) = under-splitting.
> Extract shared dependencies as Wave-1 tasks for max parallelism.

Wave 1: T1-T3 (layout tests/spec, runtime ABI, literal lowering foundation) - mostly sequential with T1 blocking implementation, but T2/T3 can split after T1 red tests are written.
Wave 2: T4-T6 (append, push, indexed assignment COW) - sequential by dependency on new representation, with test writing and implementation paired per task.
Wave 3: T7-T9 (ergonomics, element ownership, sidecar retirement) - parallel once T4-T6 compile.
Wave 4: T10-T12 (sanitizer automation, integration audit, docs/evidence/git cleanup) - parallel after feature slices.
Wave 5: Final verification F1-F4 in parallel after all implementation tasks and commits.

### Dependency Matrix (full, all tasks)
- T1 blocks T2-T12.
- T2 blocks T3-T9 and T10 sanitizer runtime checks.
- T3 blocks T4-T9.
- T4 blocks T5, T7, T8, T10.
- T5 blocks T6, T7, T8, T10.
- T6 blocks T8, T10.
- T7 blocks T8, T10.
- T8 blocks T9.
- T9 blocks T11 and final verification.
- T10 blocks T11 and final verification.
- T11 blocks T12.
- T12 blocks final verification.

### Agent Dispatch Summary (wave → task count → categories)
- Wave 1 → 3 tasks → deep, unspecified-high
- Wave 2 → 3 tasks → deep, unspecified-high
- Wave 3 → 3 tasks → unspecified-high, deep
- Wave 4 → 3 tasks → unspecified-high, quick
- Final → 4 review tasks → oracle, unspecified-high, unspecified-high, deep

## TODOs
> Implementation + Test = ONE task. Never separate.
> EVERY task MUST have: Agent Profile + Parallelization + QA Scenarios.

- [x] 1. Pin RC array layout and add RED layout/literal/alias tests

  **What to do**: Add failing tests before implementation for the new representation. Pin this ABI in test names/comments: `T[]` is one pointer to an `opal_rc`-managed payload; payload bytes begin with `size_t len`, `size_t cap`, then aligned element storage. Add test-projects for heap-backed empty/non-empty literals and alias-preserving mutation expectations even before the implementation passes. Add codegen/runtime tests that fail under current stack-literal plus sidecar metadata behavior.
  **Must NOT do**: Do not implement the layout in this task before tests are present. Do not choose a fat `{ptr,len,cap}` value ABI. Do not introduce a NULL/static empty-array sentinel.

  **Recommended Agent Profile**:
  - Category: `deep` - Reason: crosses tests, codegen assumptions, runtime ABI, and language semantics.
  - Skills: [] - no browser/UI skill needed.
  - Omitted: [`frontend-ui-ux`] - no UI work.

  **Parallelization**: Can Parallel: NO | Wave 1 | Blocks: [2,3,4,5,6,7,8,9,10,11,12] | Blocked By: []

  **References** (executor has NO interview context - be exhaustive):
  - Pattern: `src/codegen/expressions_array.rs` - current literal lowering, including stack-backed literal path and row-value struct helpers.
  - Pattern: `src/codegen/expressions.rs` - `ArrayMetadata`, `VariableBinding`, and `CodegenEnv.pending_array_metadata` definitions to be made obsolete by later tasks.
  - Test: `tests/array_integration.rs` - existing integration harness for `test-projects/array-*`.
  - Test: `test-projects/array-push/src/main.op` and `test-projects/array-append/src/main.op` - current behavior fixtures to mirror.
  - API/Runtime: `runtime/opal_rc.h`, `runtime/opal_rc.c` - existing RC header-before-payload ABI.

  **Acceptance Criteria** (agent-executable only):
  - [ ] New tests exist for heap-backed literals, empty arrays, alias-preserving `.push`, alias-preserving indexed assignment, and functional `append` purity.
  - [ ] Running `cargo test --features integration --test array_integration array_rc_layout -- --nocapture` exits non-zero before implementation and exits 0 after tasks 2-6.
  - [ ] Running `cargo test array_rc_layout -- --nocapture` exits non-zero before implementation and exits 0 after tasks 2-6.
  - [ ] Evidence of the initial RED failure and later GREEN pass is saved to `.sisyphus/evidence/task-1-rc-layout-tests.txt`.

  **QA Scenarios** (MANDATORY - task incomplete without these):
  ```
  Scenario: RED tests expose current split representation
    Tool: Bash
    Steps: Run `cargo test --features integration --test array_integration array_rc_layout -- --nocapture`.
    Expected: Command exits non-zero before implementation because current arrays use stack/raw pointer plus sidecar metadata paths.
    Evidence: .sisyphus/evidence/task-1-rc-layout-tests-red.txt

  Scenario: Commit history stays green
    Tool: Bash
    Steps: After implementation tasks make these tests pass, run `cargo test --features integration --test array_integration array_rc_layout -- --nocapture`.
    Expected: Command exits 0; no failing RED commit is made separately.
    Evidence: .sisyphus/evidence/task-1-rc-layout-tests-green.txt
  ```

  **Commit**: YES | Message: `test(array): cover rc-backed array layout` | Files: [`tests/array_integration.rs`, `test-projects/array-rc-layout/**`, relevant Rust unit test files]

- [x] 2. Implement runtime RC array payload ABI and helper surface

  **What to do**: Add the runtime helper surface for RC arrays around the existing `opal_rc` runtime. Define the payload header layout as `size_t len; size_t cap; elems...`; allocation must call `opal_rc_alloc` and return a pointer to the payload header. Add helpers for reading/writing len/cap, computing element storage address with correct alignment, creating arrays with `len/cap`, retaining/releasing RC-bearing elements via codegen-provided element operations, and dropping live elements through an array-specific drop callback. Keep the `opal_rc` header immediately before the payload.
  **Must NOT do**: Do not create a second array-only refcount header. Do not make `T[]` a fat value. Do not use a NULL sentinel for empty arrays.

  **Recommended Agent Profile**:
  - Category: `deep` - Reason: C runtime ABI and compiler codegen must agree exactly.
  - Skills: [] - no special skill required.
  - Omitted: [`frontend-ui-ux`] - not UI.

  **Parallelization**: Can Parallel: NO | Wave 1 | Blocks: [3,4,5,6,7,8,9,10] | Blocked By: [1]

  **References** (executor has NO interview context - be exhaustive):
  - Runtime: `runtime/opal_rc.h` - declare array helper APIs near existing `opal_rc_*` declarations.
  - Runtime: `runtime/opal_rc.c` - implement helpers using existing `opal_rc_alloc`, `opal_rc_inc`, `opal_rc_dec`, and iterative drop behavior.
  - Codegen API: `src/codegen/rc_emitter.rs` - existing `emit_alloc`, `emit_inc`, `emit_dec`; extend only if needed for array helper declarations.
  - Runtime tests: `src/runtime/tests.rs` - add helper-level tests where possible.

  **Acceptance Criteria** (agent-executable only):
  - [ ] `cargo test opal_array_rc -- --nocapture` exits 0.
  - [ ] `cargo test --features integration --test array_integration array_rc_layout -- --nocapture` advances from layout/runtime failures to codegen-only failures.
  - [ ] `rg 'opal_array_' runtime src/codegen` shows helper declarations/usages and no separate `opal_array_inc`/`opal_array_dec` refcount system.

  **QA Scenarios** (MANDATORY - task incomplete without these):
  ```
  Scenario: Runtime helper allocates empty and non-empty arrays
    Tool: Bash
    Steps: Run `cargo test opal_array_rc_alloc -- --nocapture`.
    Expected: Command exits 0; tests assert len/cap are read from payload and empty arrays are real allocations.
    Evidence: .sisyphus/evidence/task-2-runtime-rc-array.txt

  Scenario: Runtime helper rejects side refcount design
    Tool: Bash
    Steps: Run `rg 'opal_array_(inc|dec)|array_refcount|array_ownership' runtime src/codegen`.
    Expected: Command exits non-zero or returns zero lines; ownership uses `opal_rc_*` only.
    Evidence: .sisyphus/evidence/task-2-no-parallel-refcount.txt
  ```

  **Commit**: YES | Message: `feat(runtime): add rc-backed array payload helpers` | Files: [`runtime/opal_rc.h`, `runtime/opal_rc.c`, `src/runtime/tests.rs`, any build registration files needed for runtime C sources]

- [x] 3. Migrate array literals and length/capacity reads to heap-backed value metadata

  **What to do**: Rewrite array literal lowering so every array literal, including `[]`, allocates an RC-backed array payload and stores len/cap in the payload header. Update array indexing and length/capacity access to read from the payload instead of `CodegenEnv.pending_array_metadata`, `VariableBinding.static_array_length`, or separate `name_len`/`name_cap` allocas. Preserve nested array row behavior by storing array value pointers to RC array payloads, not `{ptr,len,cap}` row structs, unless a temporary transition struct is immediately eliminated in this task.
  **Must NOT do**: Do not leave stack-backed literals in `expressions_array.rs`. Do not keep a working array sidecar path for literals. Do not implement append/push yet except enough to keep existing literal tests compiling.

  **Recommended Agent Profile**:
  - Category: `deep` - Reason: central compiler representation refactor.
  - Skills: [] - no special skill required.
  - Omitted: [`git-master`] - git commands are listed but no special git skill required unless executor chooses.

  **Parallelization**: Can Parallel: NO | Wave 1 | Blocks: [4,5,6,7,8,9,10] | Blocked By: [1,2]

  **References** (executor has NO interview context - be exhaustive):
  - Pattern: `src/codegen/expressions_array.rs` - `codegen_array_literal`, `codegen_nested_array_literal`, `array_value_struct_type`, `build_array_value_struct`, `publish_array_metadata`, `resolve_array_access_base_and_length`.
  - Pattern: `src/codegen/expressions.rs` - `ArrayMetadata`, `VariableBinding`, `pending_array_metadata`.
  - Pattern: `src/codegen/statements.rs` - `codegen_let_statement` stores `_len`/`_cap` for arrays today.
  - Runtime: `runtime/opal_rc.h`, `runtime/opal_rc.c` - array helper ABI from task 2.
  - Test: `tests/array_integration.rs`, `test-projects/array-bounds/src/main.op`, `test-projects/array-double/src/main.op`.

  **Acceptance Criteria** (agent-executable only):
  - [ ] `cargo test array_literal -- --nocapture` exits 0.
  - [ ] `cargo test --features integration --test array_integration array_literals -- --nocapture` exits 0 if such filtered tests exist; otherwise `cargo test --features integration --test array_integration -- --nocapture` exits 0 for literal/bounds/nested cases.
  - [ ] `rg --type rust 'build_alloca\(.*array|pending_array_metadata' src/codegen/expressions_array.rs src/codegen/statements.rs` returns no stack-literal or literal metadata path lines; any unrelated hits are documented in `.sisyphus/evidence/task-3-sidecar-literal-audit.txt`.

  **QA Scenarios** (MANDATORY - task incomplete without these):
  ```
  Scenario: Empty and non-empty literals are heap-backed
    Tool: Bash
    Steps: Run `cargo test --features integration --test array_integration array_rc_layout -- --nocapture`.
    Expected: Command exits 0 for empty literal and non-empty literal cases; tests assert no stack-backed literal assumptions.
    Evidence: .sisyphus/evidence/task-3-heap-literals.txt

  Scenario: Bounds checks read payload length
    Tool: Bash
    Steps: Run `cargo test --features integration --test array_integration array_bounds -- --nocapture`.
    Expected: Command exits 0 and preserves existing bounds error behavior while reading length from array payload.
    Evidence: .sisyphus/evidence/task-3-payload-bounds.txt
  ```

  **Commit**: YES | Message: `feat(codegen): lower literals to rc arrays` | Files: [`src/codegen/expressions_array.rs`, `src/codegen/expressions.rs`, `src/codegen/statements.rs`, tests and fixtures]

- [x] 4. Migrate `append(xs, value)` to functional RC array construction

  **What to do**: Rewrite `append` lowering so it allocates a new RC array payload with `len = old_len + 1`, `cap = max(old_len + 1, growth_policy(old_cap))`, copies all existing elements into the new payload, retains RC-bearing copied elements, retains the appended RC-bearing element, stores the appended element, and returns the new array value. Preserve functional semantics: the input array is not modified even when uniquely owned. Add explicit tests proving append purity and alias non-visibility.
  **Must NOT do**: Do not lower `append` to mutate/reuse its input. Do not skip retaining copied RC-bearing elements. Do not call raw `malloc` for the array buffer.

  **Recommended Agent Profile**:
  - Category: `unspecified-high` - Reason: focused but tricky codegen/runtime ownership update.
  - Skills: [] - no special skill required.
  - Omitted: [`frontend-ui-ux`] - no UI.

  **Parallelization**: Can Parallel: NO | Wave 2 | Blocks: [5,7,8,10] | Blocked By: [1,2,3]

  **References** (executor has NO interview context - be exhaustive):
  - Pattern: `src/codegen/functions_call/array.rs` - `lower_array_append_operation`, append dispatch sites.
  - Pattern: `src/codegen/functions_call/array/helpers.rs` - `allocate_array_buffer`, `copy_existing_array_elements`, `compute_next_array_capacity`; replace raw-buffer assumptions.
  - Test: `test-projects/array-append/src/main.op` and `tests/array_integration.rs` append tests.
  - Runtime: `runtime/opal_rc.c/.h` array helpers from task 2.

  **Acceptance Criteria** (agent-executable only):
  - [ ] `cargo test --features integration --test array_integration array_append -- --nocapture` exits 0.
  - [ ] New append purity test exits 0 and proves mutating the appended result does not affect the original.
  - [ ] `rg --type rust 'malloc|allocate_array_buffer' src/codegen/functions_call/array.rs src/codegen/functions_call/array/helpers.rs` shows no raw malloc append path; any helper name retained must allocate through RC array helper only.

  **QA Scenarios** (MANDATORY - task incomplete without these):
  ```
  Scenario: append remains pure functional
    Tool: Bash
    Steps: Run `cargo test --features integration --test array_integration array_append_purity -- --nocapture`.
    Expected: Command exits 0; fixture proves `let b = append(a, 3); b.push(4)` leaves `a` unchanged.
    Evidence: .sisyphus/evidence/task-4-append-purity.txt

  Scenario: append handles RC-bearing elements
    Tool: Bash
    Steps: Run `cargo test --features integration --test array_integration array_append_rc_elements -- --nocapture`.
    Expected: Command exits 0 and sanitizer task later reports no leak/UAF for copied string/array elements.
    Evidence: .sisyphus/evidence/task-4-append-rc-elements.txt
  ```

  **Commit**: YES | Message: `feat(array): make append build rc arrays` | Files: [`src/codegen/functions_call/array.rs`, `src/codegen/functions_call/array/helpers.rs`, tests and fixtures]

- [x] 5. Implement `.push(value)` as unconditional COW rebinding

  **What to do**: Rewrite `.push` lowering for mutable identifier receivers so it creates a new RC array payload, copies/retains existing elements, appends/retains the new element when RC-bearing, and stores the new array pointer back into the receiver binding. This is unconditional COW in v1 even when uniquely owned and even when capacity is available. Add tests proving alias preservation.
  **Must NOT do**: Do not mutate the existing array in place. Do not use unique-owner checks or spare-capacity fast paths. Do not allow `.push` on immutable receivers.

  **Recommended Agent Profile**:
  - Category: `unspecified-high` - Reason: method lowering plus ownership semantics.
  - Skills: [] - no special skill required.
  - Omitted: [`frontend-ui-ux`] - no UI.

  **Parallelization**: Can Parallel: NO | Wave 2 | Blocks: [6,7,8,10] | Blocked By: [1,2,3,4]

  **References** (executor has NO interview context - be exhaustive):
  - Pattern: `src/codegen/functions_call/array.rs` - push lowering call sites and `lower_array_append_operation` replacement.
  - Pattern: `src/type_system/checker/collections/collections_array.rs` - mutable receiver/type rules for array methods.
  - Test: `test-projects/array-push/src/main.op`, `tests/array_integration.rs` push tests.
  - Runtime: `runtime/opal_rc.c/.h` array helpers.

  **Acceptance Criteria** (agent-executable only):
  - [ ] `cargo test --features integration --test array_integration array_push -- --nocapture` exits 0.
  - [ ] New alias-safety fixture exits 0 for `let a = [1,2]; let mutable b = a; b.push(3);` and asserts `a == [1,2]`, `b == [1,2,3]`.
  - [ ] Typechecker/compiler still rejects `.push` on immutable receivers.

  **QA Scenarios** (MANDATORY - task incomplete without these):
  ```
  Scenario: push is alias-preserving COW
    Tool: Bash
    Steps: Run `cargo test --features integration --test array_integration array_push_cow_alias -- --nocapture`.
    Expected: Command exits 0; fixture proves original alias remains unchanged after pushing through mutable binding.
    Evidence: .sisyphus/evidence/task-5-push-cow-alias.txt

  Scenario: immutable push rejected
    Tool: Bash
    Steps: Run `cargo test --features integration --test array_integration array_push_immutable_rejected -- --nocapture`.
    Expected: Command exits 0; compiler emits the existing mutable-receiver error pattern.
    Evidence: .sisyphus/evidence/task-5-push-immutable.txt
  ```

  **Commit**: YES | Message: `feat(array): lower push as cow rebind` | Files: [`src/codegen/functions_call/array.rs`, `src/codegen/functions_call/array/helpers.rs`, typechecker tests if needed, integration fixtures]

- [x] 6. Implement identifier-backed indexed assignment `xs[i] = value`

  **What to do**: Extend assignment lowering beyond identifiers for the specific case of mutable identifier-backed array index assignment. For `xs[i] = value`, bounds-check `i` using payload `len`, allocate a COW clone with same len/cap, retain copied RC-bearing elements except at overwritten slot as appropriate, release the old overwritten RC-bearing element, retain the new RC-bearing value, write the new element, and rebind `xs`. Preserve existing parser/typechecker behavior; codegen should no longer emit `assignment target must be an identifier` for this supported case.
  **Must NOT do**: Do not support `obj.field[i] = value`, `xs[i][j] = value`, non-identifier bases, slice assignment, or in-place overwrite in v1.

  **Recommended Agent Profile**:
  - Category: `deep` - Reason: assignment lowering, lvalue semantics, bounds checks, ownership.
  - Skills: [] - no special skill required.
  - Omitted: [`frontend-ui-ux`] - no UI.

  **Parallelization**: Can Parallel: NO | Wave 2 | Blocks: [8,10] | Blocked By: [1,2,3,5]

  **References** (executor has NO interview context - be exhaustive):
  - Pattern: `src/parser/statements.rs` - parser already recognizes indexed assignment.
  - Pattern: `src/type_system/checker/statements.rs` - typechecker accepts identifiers/members/indexes as assignment targets.
  - Pattern: `src/codegen/statements.rs` - `codegen_assignment` currently accepts only identifier targets.
  - Pattern: `src/codegen/expressions_array.rs` - array index/bounds helper logic.
  - Test: `src/parser/tests.rs` indexed assignment parser tests.

  **Acceptance Criteria** (agent-executable only):
  - [ ] `cargo test indexed_assignment -- --nocapture` exits 0.
  - [ ] `cargo test --features integration --test array_integration array_index_assignment -- --nocapture` exits 0.
  - [ ] A negative test for `xs[i][j] = value` or `obj.field[i] = value` exits 0 by asserting a clear unsupported-codegen/type error.

  **QA Scenarios** (MANDATORY - task incomplete without these):
  ```
  Scenario: identifier-backed indexed assignment works
    Tool: Bash
    Steps: Run `cargo test --features integration --test array_integration array_index_assignment -- --nocapture`.
    Expected: Command exits 0; fixture proves `let mutable a = [1,2,3]; a[1] = 99;` prints/asserts `[1,99,3]`.
    Evidence: .sisyphus/evidence/task-6-index-assignment.txt

  Scenario: indexed assignment is alias-preserving
    Tool: Bash
    Steps: Run `cargo test --features integration --test array_integration array_index_assignment_cow_alias -- --nocapture`.
    Expected: Command exits 0; fixture proves alias remains original after assignment through mutable binding.
    Evidence: .sisyphus/evidence/task-6-index-assignment-cow.txt
  ```

  **Commit**: YES | Message: `feat(array): support identifier index assignment` | Files: [`src/codegen/statements.rs`, `src/codegen/expressions_array.rs`, parser/typechecker tests if needed, integration fixtures]

- [x] 7. Add `array_filled`, `reserve`, and `clear` ergonomics with explicit COW semantics

  **What to do**: Add array ergonomics following existing builtin/method registration patterns discovered in the typechecker/codegen. Implement exact signatures: `array_filled(length, value) -> T[]`; `reserve(xs, capacity) -> T[]`; `clear(xs) -> T[]`. `array_filled` creates a new RC array with len/cap equal to `length` and retains RC-bearing `value` once per slot. `reserve` returns a new array with `len = xs.len`, `cap = max(xs.cap, capacity)`, copies/retains live elements, and does not mutate `xs`. `clear` returns a new array with `len = 0`, `cap = xs.cap`, releases all live RC-bearing elements in the discarded copy semantics where applicable, and does not mutate aliases.
  **Must NOT do**: Do not make `reserve` or `clear` void mutating methods in v1. Do not add `Array.filled` unless existing naming conventions clearly require it; default spelling is free function `array_filled` plus free functions or methods only if current array method dispatch already supports them without design work.

  **Recommended Agent Profile**:
  - Category: `unspecified-high` - Reason: builtin registration plus codegen and tests.
  - Skills: [] - no special skill required.
  - Omitted: [`frontend-ui-ux`] - no UI.

  **Parallelization**: Can Parallel: NO | Wave 3 | Blocks: [8,10] | Blocked By: [1,2,3,4,5]

  **References** (executor has NO interview context - be exhaustive):
  - Pattern: `src/codegen/functions_call.rs` - builtin function dispatch.
  - Pattern: `src/codegen/functions_call/array.rs` - array method/function lowering.
  - Pattern: `src/type_system/checker/collections/collections_array.rs` - array method/type registration.
  - Pattern: `ARRAY_FEATURES.md` - documented push/append/array ergonomics.
  - Test: `tests/array_integration.rs` - add integration coverage.

  **Acceptance Criteria** (agent-executable only):
  - [ ] `cargo test --features integration --test array_integration array_filled -- --nocapture` exits 0.
  - [ ] `cargo test --features integration --test array_integration array_reserve -- --nocapture` exits 0.
  - [ ] `cargo test --features integration --test array_integration array_clear -- --nocapture` exits 0.
  - [ ] RC-bearing `array_filled(3, s)` scenario is covered and later passes sanitizer checks.

  **QA Scenarios** (MANDATORY - task incomplete without these):
  ```
  Scenario: array_filled repeats values and preserves RC ownership
    Tool: Bash
    Steps: Run `cargo test --features integration --test array_integration array_filled -- --nocapture`.
    Expected: Command exits 0; fixture prints/asserts three repeated values and sanitizer later reports no leaks/UAF.
    Evidence: .sisyphus/evidence/task-7-array-filled.txt

  Scenario: reserve and clear are functional COW helpers
    Tool: Bash
    Steps: Run `cargo test --features integration --test array_integration array_reserve -- --nocapture` and `cargo test --features integration --test array_integration array_clear -- --nocapture`.
    Expected: Both commands exit 0; fixtures prove aliases are unchanged and returned arrays have expected len/cap behavior.
    Evidence: .sisyphus/evidence/task-7-reserve-clear.txt
  ```

  **Commit**: YES | Message: `feat(array): add filled reserve clear helpers` | Files: [`src/codegen/functions_call.rs`, `src/codegen/functions_call/array.rs`, `src/type_system/checker/collections/collections_array.rs`, tests and fixtures]

- [x] 8. Complete RC-bearing element retain/release coverage

  **What to do**: Audit all array operation sites and ensure RC-bearing elements are retained and released exactly once according to the ownership checklist. Required sites: literal construction, append copy, append new element, push copy, push new element, indexed assignment copied elements, indexed assignment old overwritten element release, indexed assignment new element retain, COW clone, `array_filled`, `reserve`, `clear`, and array drop. Add focused tests with strings and nested arrays because they are RC-bearing and exercise child drops.
  **Must NOT do**: Do not defer element ownership to a follow-up. Do not assume raw pointer copies are safe for RC-bearing elements.

  **Recommended Agent Profile**:
  - Category: `deep` - Reason: memory safety across all operations.
  - Skills: [] - no special skill required.
  - Omitted: [`frontend-ui-ux`] - no UI.

  **Parallelization**: Can Parallel: YES | Wave 3 | Blocks: [9,10] | Blocked By: [1,2,3,4,5,6,7]

  **References** (executor has NO interview context - be exhaustive):
  - Runtime: `runtime/opal_rc.c/.h` - retain/release/drop mechanics.
  - Codegen: `src/codegen/rc_emitter.rs` - `emit_inc`, `emit_dec`, possible helper additions.
  - Analysis: `src/type_system/rc_analysis.rs` - existing RC type/reuse analysis.
  - Array codegen: `src/codegen/functions_call/array.rs`, `src/codegen/functions_call/array/helpers.rs`, `src/codegen/expressions_array.rs`, `src/codegen/statements.rs`.
  - String codegen: `src/codegen/expressions_string.rs` - existing temp free/ownership patterns for string-like values.

  **Acceptance Criteria** (agent-executable only):
  - [ ] `cargo test --features integration --test array_integration array_rc_elements -- --nocapture` exits 0.
  - [ ] `cargo test --features integration --test array_integration array_nested_rc_drop -- --nocapture` exits 0.
  - [ ] Sanitizer task 10 reports no leaks/UAF for string arrays, nested arrays, append/push/index overwrite, `array_filled`, `reserve`, and `clear`.

  **QA Scenarios** (MANDATORY - task incomplete without these):
  ```
  Scenario: indexed overwrite releases old RC element and retains new
    Tool: Bash
    Steps: Run `cargo test --features integration --test array_integration array_index_assignment_rc_elements -- --nocapture`.
    Expected: Command exits 0; sanitizer later shows no leaked old string and no UAF for new string.
    Evidence: .sisyphus/evidence/task-8-index-rc-elements.txt

  Scenario: nested array drop releases children
    Tool: Bash
    Steps: Run `cargo test --features integration --test array_integration array_nested_rc_drop -- --nocapture`.
    Expected: Command exits 0; sanitizer later reports no leaks for nested arrays.
    Evidence: .sisyphus/evidence/task-8-nested-rc-drop.txt
  ```

  **Commit**: YES | Message: `fix(array): retain and release rc elements` | Files: [`runtime/opal_rc.h`, `runtime/opal_rc.c`, `src/codegen/rc_emitter.rs`, array codegen files, tests and fixtures]

- [x] 9. Retire array sidecar metadata and raw malloc array paths

  **What to do**: Remove array-specific uses of `pending_array_metadata`, `ArrayMetadata`, `static_array_length`, `static_array_capacity`, `name_len`, and `name_cap` sidecar metadata. Remove or rewrite helpers that allocate raw element buffers. If non-array code still uses similarly named variables, document why they are unrelated. Use LSP references and grep audits before deletion.
  **Must NOT do**: Do not leave compatibility fallback paths. Do not keep dead code “for safety.” Do not remove unrelated non-array functionality just to satisfy broad grep output.

  **Recommended Agent Profile**:
  - Category: `unspecified-high` - Reason: cleanup/refactor requiring careful audits.
  - Skills: [] - no special skill required.
  - Omitted: [`frontend-ui-ux`] - no UI.

  **Parallelization**: Can Parallel: YES | Wave 3 | Blocks: [11,12] | Blocked By: [1,2,3,4,5,6,7,8]

  **References** (executor has NO interview context - be exhaustive):
  - Pattern: `src/codegen/expressions.rs` - `ArrayMetadata`, `VariableBinding` fields, `CodegenEnv.pending_array_metadata`.
  - Pattern: `src/codegen/statements.rs` - `codegen_let_statement` and `codegen_assignment` sidecar stores.
  - Pattern: `src/codegen/expressions_array.rs` - `publish_array_metadata`, nested row structs, array access metadata.
  - Pattern: `src/codegen/functions_call/array/helpers.rs` - `store_array_binding_with_metadata`, `allocate_array_buffer`.
  - Tool instruction: use `lsp_find_references` on `allocate_array_buffer`, `copy_existing_array_elements`, `lower_array_append_operation`, and `pending_array_metadata` before deletion.

  **Acceptance Criteria** (agent-executable only):
  - [ ] `rg --type rust 'pending_array_metadata' src/codegen src/type_system` returns zero lines.
  - [ ] `rg --type rust 'ArrayMetadata|static_array_length|static_array_capacity|store_array_binding_with_metadata' src/codegen src/type_system` returns zero lines or only documented non-array-obsolete references in `.sisyphus/evidence/task-9-sidecar-audit.txt`.
  - [ ] `rg --type rust 'malloc' src/codegen/functions_call/array.rs src/codegen/functions_call/array/helpers.rs` returns zero raw array allocation lines.
  - [ ] `cargo test --features integration --test array_integration -- --nocapture` exits 0.

  **QA Scenarios** (MANDATORY - task incomplete without these):
  ```
  Scenario: sidecar metadata is gone
    Tool: Bash
    Steps: Run `rg --type rust 'pending_array_metadata|ArrayMetadata|static_array_length|static_array_capacity|store_array_binding_with_metadata' src/codegen src/type_system`.
    Expected: Command returns zero lines, or documented unrelated lines only with explanation in evidence.
    Evidence: .sisyphus/evidence/task-9-sidecar-audit.txt

  Scenario: no raw array malloc remains
    Tool: Bash
    Steps: Run `rg --type rust 'malloc' src/codegen/functions_call/array.rs src/codegen/functions_call/array/helpers.rs`.
    Expected: Command returns zero raw array allocation lines.
    Evidence: .sisyphus/evidence/task-9-raw-malloc-audit.txt
  ```

  **Commit**: YES | Message: `refactor(array): retire sidecar metadata` | Files: [`src/codegen/expressions.rs`, `src/codegen/expressions_array.rs`, `src/codegen/statements.rs`, `src/codegen/functions_call/array.rs`, `src/codegen/functions_call/array/helpers.rs`]

- [x] 10. Add sanitizer-backed array memory regression automation

  **What to do**: Add a reproducible command or script, following repo conventions, that builds/runs the new array memory test-projects with ASAN+LSAN or Valgrind when sanitizer toolchain is unavailable. Cover array churn, string arrays, nested arrays, append, push, indexed overwrite, `array_filled`, `reserve`, and `clear`. Ensure generated sanitizer logs are either ignored or written under `.sisyphus/evidence` during task execution and not left untracked.
  **Must NOT do**: Do not make sanitizer verification a manual note only. Do not leave generated binaries/logs untracked outside ignored/evidence paths.

  **Recommended Agent Profile**:
  - Category: `unspecified-high` - Reason: build/test harness and runtime memory tooling.
  - Skills: [] - no special skill required.
  - Omitted: [`frontend-ui-ux`] - no UI.

  **Parallelization**: Can Parallel: YES | Wave 4 | Blocks: [11,12] | Blocked By: [1,2,3,4,5,6,7,8]

  **References** (executor has NO interview context - be exhaustive):
  - Test harness: `tests/array_integration.rs` - integration execution pattern.
  - Build config: `Cargo.toml` - dev-dependencies/features.
  - Runtime C files: `runtime/opal_rc.c`, `runtime/opal_string.c`, `runtime/opal_bytes.c`, `runtime/opal_runtime.c`.
  - Existing benchmark/memory references: `src/benchmarks/memory.rs`, `src/benchmarks/runtime_bench.rs`.

  **Acceptance Criteria** (agent-executable only):
  - [ ] `cargo test --features integration --test array_integration -- --nocapture` exits 0.
  - [ ] New sanitizer command/script exits 0 for all new array memory fixtures.
  - [ ] Sanitizer stderr contains no `ERROR: AddressSanitizer`, `LeakSanitizer`, `heap-use-after-free`, `double-free`, or `detected memory leaks` lines.
  - [ ] `git status --porcelain` after sanitizer run shows no untracked generated logs/binaries except intentional tracked script/test changes.

  **QA Scenarios** (MANDATORY - task incomplete without these):
  ```
  Scenario: sanitizer reports no array leaks or UAF
    Tool: Bash
    Steps: Run the new sanitizer command/script, e.g. `./scripts/test-array-sanitizers.sh` if scripts are the repo convention chosen by implementation.
    Expected: Command exits 0; stderr has no ASAN/LSAN/Valgrind error markers.
    Evidence: .sisyphus/evidence/task-10-array-sanitizers.txt

  Scenario: sanitizer artifacts do not dirty repo
    Tool: Bash
    Steps: Run `git status --porcelain` immediately after sanitizer command.
    Expected: Output lists only intentional tracked changes before commit, and is empty after commit.
    Evidence: .sisyphus/evidence/task-10-sanitizer-git-status.txt
  ```

  **Commit**: YES | Message: `test(array): add sanitizer memory regressions` | Files: [`tests/array_integration.rs`, `test-projects/array-memory-*`, sanitizer script or harness files, `.gitignore` if needed]

- [x] 11. Run full regression suite and audit gitignore/artifacts

  **What to do**: Run the full relevant test suite, audit generated artifacts, and ensure `.gitignore` covers sanitizer/build outputs without hiding source files. Fix any regressions discovered by full-suite execution. Save evidence for cargo tests, array integration tests, sanitizer tests, sidecar grep, raw malloc grep, and git status.
  **Must NOT do**: Do not mark this task complete with failing/flaky tests. Do not add broad ignore patterns that could hide real source or fixture files.

  **Recommended Agent Profile**:
  - Category: `unspecified-high` - Reason: broad QA and artifact hygiene.
  - Skills: [] - no special skill required.
  - Omitted: [`frontend-ui-ux`] - no UI.

  **Parallelization**: Can Parallel: NO | Wave 4 | Blocks: [12] | Blocked By: [9,10]

  **References** (executor has NO interview context - be exhaustive):
  - Config: `Cargo.toml`.
  - Ignore rules: `.gitignore`.
  - Integration: `tests/array_integration.rs`.
  - Runtime: `runtime/*.c`, `runtime/*.h`.
  - Codegen: `src/codegen/**`.

  **Acceptance Criteria** (agent-executable only):
  - [ ] `cargo test` exits 0.
  - [ ] `cargo test --features integration --test array_integration -- --nocapture` exits 0.
  - [ ] Sanitizer command/script from task 10 exits 0.
  - [ ] `rg --type rust 'pending_array_metadata|ArrayMetadata|static_array_length|static_array_capacity|store_array_binding_with_metadata' src/codegen src/type_system` returns zero lines or documented unrelated lines only.
  - [ ] `git status --porcelain` before commit lists only intended tracked changes; after commit it is empty.

  **QA Scenarios** (MANDATORY - task incomplete without these):
  ```
  Scenario: full Rust and integration suites pass
    Tool: Bash
    Steps: Run `cargo test` and `cargo test --features integration --test array_integration -- --nocapture`.
    Expected: Both commands exit 0.
    Evidence: .sisyphus/evidence/task-11-full-tests.txt

  Scenario: artifact hygiene is clean
    Tool: Bash
    Steps: Run sanitizer command, then `git status --porcelain`.
    Expected: No unexpected untracked sanitizer/build artifacts appear; after commit output is empty.
    Evidence: .sisyphus/evidence/task-11-artifact-hygiene.txt
  ```

  **Commit**: YES | Message: `chore(array): verify rc array migration` | Files: [`.gitignore` if needed, fixes from regression run, evidence references if repo tracks them]

- [x] 12. Final atomic-commit and clean-status gate

  **What to do**: Inspect git history and working tree, ensure all intended changes are committed in coherent atomic commits, and verify no staged or unstaged changes remain. If changes remain, either commit them atomically with an appropriate conventional message or revert unintended artifacts. Save final status evidence.
  **Must NOT do**: Do not leave staged changes. Do not leave unstaged changes. Do not create empty commits. Do not use force-push or amend unless explicitly instructed by the user.

  **Recommended Agent Profile**:
  - Category: `quick` - Reason: final verification and commit hygiene.
  - Skills: [`git-master`] - use if executor has access because task is explicitly git-focused.
  - Omitted: [`frontend-ui-ux`] - no UI.

  **Parallelization**: Can Parallel: NO | Wave 4 | Blocks: [F1,F2,F3,F4] | Blocked By: [11]

  **References** (executor has NO interview context - be exhaustive):
  - Git policy from user: atomic commits; final `git status` completely clean.
  - Commit Strategy section of this plan.

  **Acceptance Criteria** (agent-executable only):
  - [ ] `git status --porcelain` returns an empty string.
  - [ ] `git log --oneline -10` shows coherent conventional commits for array slices.
  - [ ] `cargo test` exits 0 after the final commit.

  **QA Scenarios** (MANDATORY - task incomplete without these):
  ```
  Scenario: final repository status is clean
    Tool: Bash
    Steps: Run `git status --porcelain`.
    Expected: Output is exactly empty.
    Evidence: .sisyphus/evidence/task-12-final-git-status.txt

  Scenario: final committed state passes tests
    Tool: Bash
    Steps: Run `cargo test`.
    Expected: Command exits 0 from the committed clean state.
    Evidence: .sisyphus/evidence/task-12-final-tests.txt
  ```

  **Commit**: YES if needed | Message: `chore(array): finalize rc array migration` | Files: [only any remaining intentional files]

## Final Verification Wave (MANDATORY — after ALL implementation tasks)
> 4 review agents run in PARALLEL. ALL must APPROVE. Present consolidated results to user and get explicit "okay" before completing.
> **Do NOT auto-proceed after verification. Wait for user's explicit approval before marking work complete.**
> **Never mark F1-F4 as checked before getting user's okay.** Rejection or user feedback -> fix -> re-run -> present again -> wait for okay.
- [ ] F1. Plan Compliance Audit — oracle

  **QA Scenario**:
  ```
  Scenario: plan compliance audit approves implementation
    Tool: task
    Steps: Run an oracle review agent with the final implementation diff and `.sisyphus/plans/array-memory-bug.md`; require it to check every Must Have, Must NOT Have, task acceptance criterion, and commit/status requirement.
    Expected: Oracle returns APPROVED with no critical or blocking findings.
    Evidence: .sisyphus/evidence/f1-plan-compliance-audit.md
  ```

- [ ] F2. Code Quality Review — unspecified-high

  **QA Scenario**:
  ```
  Scenario: code quality review approves implementation
    Tool: task
    Steps: Run an unspecified-high review agent against the final diff, focusing on compiler/runtime maintainability, ownership clarity, dead code, naming, and AI-slop patterns.
    Expected: Reviewer returns APPROVED with no required changes.
    Evidence: .sisyphus/evidence/f2-code-quality-review.md
  ```

- [ ] F3. Real Manual QA — unspecified-high

  **QA Scenario**:
  ```
  Scenario: hands-on QA executes the shipped verification commands
    Tool: task
    Steps: Run an unspecified-high QA agent to execute `cargo test`, `cargo test --features integration --test array_integration -- --nocapture`, the sanitizer command/script from T10, sidecar grep audits from T9, and `git status --porcelain`.
    Expected: QA agent reports all commands exit 0, grep audits match expected output, sanitizer has no error markers, and git status is empty.
    Evidence: .sisyphus/evidence/f3-real-qa.md
  ```

- [ ] F4. Scope Fidelity Check — deep

  **QA Scenario**:
  ```
  Scenario: scope fidelity review confirms no overreach or omissions
    Tool: task
    Steps: Run a deep review agent comparing the implementation against this plan's scope boundaries, including no feature flag, no sidecar ownership, no nested/field indexed assignment, no unique-owner fast path, and inclusion of all requested ergonomics.
    Expected: Reviewer returns APPROVED with no scope creep and no missed in-scope deliverables.
    Evidence: .sisyphus/evidence/f4-scope-fidelity.md
  ```

## Commit Strategy
- Use TDD locally, but commit only green atomic slices. Do not commit failing RED states.
- Commit after each TODO slice T1-T12 once its acceptance criteria pass and `git status --porcelain` shows only intended files before staging.
- Use conventional commit messages listed in each task.
- Before every commit: inspect `git status`, `git diff`, and `git log --oneline -10`; stage only intended files.
- After every commit: run `git status --porcelain` and save output to evidence.
- Final repository state must be clean because all intended changes have been committed.

## Success Criteria
- The generated language no longer allocates raw array buffers outside the RC array ABI.
- Array ownership/lifetime is integrated with `opal_rc`.
- `.push` and indexed assignment are alias-safe by construction.
- `append` remains functional and distinct from mutation ergonomics.
- Existing array integration behavior remains compatible unless explicitly updated by new semantics.
- Memory regression tests and sanitizer checks pass.
- Sidecar metadata paths are removed for arrays.
- Atomic commits are present and final git status is clean.
