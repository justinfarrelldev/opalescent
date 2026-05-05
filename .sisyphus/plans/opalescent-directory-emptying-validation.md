# Opalescent Directory Emptying Validation

## TL;DR
> **Summary**: Validate, with TDD-first end-to-end `.op` integration tests, that Opalescent can express the Linux equivalent of a Batch directory-emptying script: list a target directory, compose child paths, classify files vs directories, recursively delete directories, delete files, and prove the directory is empty. Implement only defects proven by those tests, with conditional broader path API work only if current `FilesystemPath` composition is inadequate.
> **Deliverables**:
> - Objective readiness verdict based on named green tests, not prose-only judgment
> - `.op` integration coverage for `delete_directory_recursive_sync` against a nested tree
> - `.op` integration coverage for the full empty-directory workflow: `list_directory_sync` → `join_path_components` → `is_directory_sync` → `delete_directory_recursive_sync`/`delete_file_sync`
> - Negative-path coverage exercising filesystem error ABI via a missing recursive-delete target
> - Conditional path API improvement if `FilesystemPath[]` entries cannot be used ergonomically in the workflow
> **Effort**: Medium
> **Parallel**: YES - 3 waves
> **Critical Path**: Task 1 → Task 2/3/4 → Task 5 → Task 6 → Task 7 → Final Verification

## Context
### Original Request
Validate the supplied analysis and implement all logic/tests necessary to confirm whether Opalescent can support the Linux equivalent of this Batch program:

```bat
set folder="C:\Users\justi\Downloads"
cd /d %folder%
for /F "delims=" %%i in ('dir /b') do (rmdir "%%i" /s/q || del "%%i" /s/q)
```

The operational Linux equivalent is: given a directory, delete every top-level child; if a child is a directory, delete it recursively; if it is a file, delete it as a file. Tests MUST use temporary/sandbox directories only; never operate on real `~/Downloads` or any home directory path.

### Interview Summary
- Test strategy: TDD first.
- Path API scope: if `FilesystemPath[]` entries cannot be used ergonomically for child path composition, implement a broader path API improvement rather than a one-off conversion.
- Scope is Linux readiness for the directory-emptying workflow, not Windows support or production cleanup tooling.

### Metis Review (gaps addressed)
- Readiness must be objective: named tests + commands + expected green outcomes.
- Add a diagnosis task between failing tests and fixes; do not pre-assume the defect layer.
- Path API work is conditional on a concrete failed ergonomics finding.
- Tests must derive paths from `tempfile::TempDir` or repository test helpers and must assert sandbox prefix before deletion.
- Include a negative-path test to exercise error ABI.
- Explicit non-goals: no Windows shims, no tilde expansion, no async deletion, no dry-run/progress/trash integration, no CI workflow changes.

## Work Objectives
### Core Objective
Make Opalescent's readiness for safe Linux directory-emptying work provable by automated end-to-end tests, and implement only the logic required to make those tests pass.

### Deliverables
- New or extended Rust integration tests under `tests/integration_e2e/` gated by `#[cfg(feature = "integration")]`.
- Inline `.op` test programs or fixture `.op` programs that call the actual filesystem stdlib functions from Opalescent source.
- If required by diagnosis, focused fixes in compiler/type/codegen/runtime/path API files.
- A readiness report/checklist captured in test names, assertion messages, and final execution evidence.

### Definition of Done (verifiable conditions with commands)
- `cargo test --all-features --test integration_e2e fs_recursive_delete_from_op_source -- --exact` exits 0 and reports the exact test passed.
- `cargo test --all-features --test integration_e2e fs_empty_directory_workflow_from_op_source -- --exact` exits 0 and reports the exact test passed.
- `cargo test --all-features --test integration_e2e fs_recursive_delete_missing_path_error_from_op_source -- --exact` exits 0 and reports the exact test passed.
- `cargo test --all-features` exits 0.
- `cargo clippy --all-targets --all-features -- -D warnings` exits 0.
- `cargo fmt --all -- --check` exits 0.

### Must Have
- TDD-first test additions before fixes.
- `.op` source invocation of `delete_directory_recursive_sync`.
- `.op` source invocation of the complete workflow: `path_from`, `list_directory_sync`, `join_path_components`, `is_directory_sync`, `delete_directory_recursive_sync`, `delete_file_sync`.
- Deterministic Rust assertions using `std::fs` to verify final filesystem state.
- Safety guard: tests must assert the target path starts with the test temp directory before invoking recursive deletion.
- Negative-path test must verify error branch behavior from `.op` source.

### Must NOT Have (guardrails, AI slop patterns, scope boundaries)
- Do NOT delete or test against `~`, `$HOME`, `/home`, `/Users`, `~/Downloads`, or a real downloads directory.
- Do NOT add Windows directory iteration shims, tilde expansion, async deletion, progress reporting, dry-run, trash/recycle integration, or parallel deletion.
- Do NOT modify `.github/workflows/ci.yml`.
- Do NOT refactor `runtime/opal_fs.c`, `src/type_system/module_resolver/standard_symbols_filesystem_operations.rs`, `src/codegen/functions_stdlib.rs`, `src/codegen/functions_call.rs`, or `src/codegen/error_abi.rs` unless Task 5 identifies a specific reproducer.
- Do NOT touch `is_directory_sync` implementation unless a newly discovered failing test proves an implementation defect.
- Do NOT rely on human inspection of filesystem state.

## Verification Strategy
> ZERO HUMAN INTERVENTION - all verification is agent-executed.
- Test decision: TDD first + Rust integration tests compiling/running `.op` source via existing `compile_program(...)` harness.
- QA policy: Every task has agent-executed scenarios.
- Evidence: `.sisyphus/evidence/task-{N}-{slug}.{ext}`

## Execution Strategy
### Parallel Execution Waves
> Target: 5-8 tasks per wave. <3 per wave (except final) = under-splitting.
> Extract shared dependencies as Wave-1 tasks for max parallelism.

Wave 1: Task 1 coverage inventory + Task 2 recursive-delete TDD + Task 3 full-workflow TDD + Task 4 negative-path TDD.
Wave 2: Task 5 diagnosis + Task 6 conditional implementation fixes/path API work.
Wave 3: Task 7 readiness report/checklist and final verification.

### Dependency Matrix (full, all tasks)
| Task | Blocks | Blocked By |
|---|---|---|
| 1 | 5, 7 | None |
| 2 | 5, 6, 7 | None |
| 3 | 5, 6, 7 | None |
| 4 | 5, 6, 7 | None |
| 5 | 6, 7 | 1, 2, 3, 4 |
| 6 | 7 | 5 |
| 7 | Final Verification | 1, 2, 3, 4, 5, 6 |

### Agent Dispatch Summary (wave → task count → categories)
| Wave | Task Count | Categories |
|---|---:|---|
| 1 | 4 | quick, unspecified-high |
| 2 | 2 | deep, unspecified-high |
| 3 | 1 | writing |
| Final | 4 | oracle, unspecified-high, deep |

## TODOs
> Implementation + Test = ONE task. Never separate.
> EVERY task MUST have: Agent Profile + Parallelization + QA Scenarios.

- [x] 1. Inventory existing filesystem `.op` coverage and register the new test module

  **What to do**: Confirm the executor's starting point before adding tests. Read `tests/integration_e2e/fs_predicates.rs`, `tests/integration_e2e/fs_helpers.rs`, `tests/integration_e2e/fs_state_guard.rs`, `tests/integration_e2e/tests.rs`, and the filesystem stdlib registrations. Add `mod fs_delete_directory_recursive;` to `tests/integration_e2e/tests.rs` immediately after `mod fs_directory_operations;` when creating the new test file in Task 2. Record in `.sisyphus/evidence/task-1-coverage-inventory.md` whether `is_directory_sync` is already covered by `fs_predicates_matrix` and whether any existing test invokes `delete_directory_recursive_sync` from `.op` source.
  **Must NOT do**: Do not rewrite existing passing predicate tests. Do not change CI. Do not modify runtime/compiler files in this task.

  **Recommended Agent Profile**:
  - Category: `quick` - Reason: focused test inventory and module registration.
  - Skills: [] - No specialized skill needed.
  - Omitted: [`git-master`] - No commit/history operation required.

  **Parallelization**: Can Parallel: YES | Wave 1 | Blocks: 5, 7 | Blocked By: None

  **References** (executor has NO interview context - be exhaustive):
  - Pattern: `tests/integration_e2e/fs_predicates.rs:21-52` - inline `.op` source builder and `compile_program(...)` runner pattern.
  - Pattern: `tests/integration_e2e/fs_predicates.rs:86-180` - existing `is_directory_sync` `.op` coverage for file, directory, and missing path.
  - Pattern: `tests/integration_e2e/tests.rs:5-41` - integration module registration location.
  - Pattern: `tests/integration_e2e/fs_helpers.rs:72-82` - `unique_probe_target_dir` helper for temp build directories.
  - Pattern: `tests/integration_e2e/fs_state_guard.rs:10-28` - `FsStateGuard` pattern for filesystem fixture cleanup.
  - API/Type: `src/type_system/module_resolver/standard_symbols_filesystem_operations.rs` - filesystem stdlib symbol registrations.
  - API/Type: `src/codegen/functions_stdlib.rs` - LLVM declarations for filesystem stdlib functions.

  **Acceptance Criteria** (agent-executable only):
  - [ ] `tests/integration_e2e/tests.rs` contains `mod fs_delete_directory_recursive;` after the new test file exists.
  - [ ] `.sisyphus/evidence/task-1-coverage-inventory.md` contains `is_directory_sync .op coverage: YES` with reference to `fs_predicates_matrix`.
  - [ ] `.sisyphus/evidence/task-1-coverage-inventory.md` contains `delete_directory_recursive_sync .op coverage before task: NO` unless the executor discovers an existing test, in which case it must cite the exact file and test name.

  **QA Scenarios** (MANDATORY - task incomplete without these):
  ```
  Scenario: Coverage inventory confirms known state
    Tool: Bash
    Steps: Run `cargo test --all-features --test integration_e2e fs_predicates_matrix -- --exact`.
    Expected: Command exits 0 and output includes `test result: ok. 1 passed`; evidence file states `is_directory_sync .op coverage: YES`.
    Evidence: .sisyphus/evidence/task-1-coverage-inventory.md

  Scenario: Module registration does not break test tree
    Tool: Bash
    Steps: Run `cargo test --all-features --test integration_e2e smoke_void_program_compiles_links_and_runs -- --exact` after registering the new module.
    Expected: Command exits 0 and output includes `test result: ok. 1 passed`.
    Evidence: .sisyphus/evidence/task-1-module-registration.txt
  ```

  **Commit**: NO | Message: N/A | Files: `tests/integration_e2e/tests.rs`, `.sisyphus/evidence/task-1-*`

- [x] 2. Add TDD `.op` integration test for `delete_directory_recursive_sync` on a nested tree

  **What to do**: Create `tests/integration_e2e/fs_delete_directory_recursive.rs` with `#![cfg(feature = "integration")]`, `use super::fs_helpers::unique_probe_target_dir;`, `use super::*;`, `use serial_test::serial;`, and a test named `fs_recursive_delete_from_op_source`. The Rust harness must create this exact fixture under `let fixture_root = temp_dir.join("recursive-delete-tree")`: `a.txt`, `sub/c.txt`, `sub/nested/d.txt`. It must assert `fixture_root.starts_with(std::env::temp_dir())` and `fixture_root.exists()` before compiling/running `.op`. The inline `.op` source must import `path_from`, `delete_directory_recursive_sync`, `path_exists_sync`, and `is_directory_sync`; call `delete_directory_recursive_sync(path_from('<escaped fixture_root>'))`; then print exactly `exists_after={exists_after}` and `dir_after={dir_after}` after checking the same target. Rust assertions must require successful process exit, stdout containing `exists_after=false` (or `exists_after=0`) and `dir_after=false` (or `dir_after=0`), and `!fixture_root.exists()`.
  **Must NOT do**: Do not create fixture paths outside `std::env::temp_dir()`. Do not call recursive delete before the sandbox prefix assertion. Do not test symlinks, permissions, or concurrent deletion in this task.

  **Recommended Agent Profile**:
  - Category: `unspecified-high` - Reason: requires careful test harness/source construction and safety assertions.
  - Skills: [] - No specialized skill needed.
  - Omitted: [`frontend-ui-ux`] - No UI work.

  **Parallelization**: Can Parallel: YES | Wave 1 | Blocks: 5, 6, 7 | Blocked By: None

  **References** (executor has NO interview context - be exhaustive):
  - Pattern: `tests/integration_e2e/fs_predicates.rs:21-27` - escape host path for inline `.op` string literal.
  - Pattern: `tests/integration_e2e/fs_predicates.rs:29-52` - compile and run inline source with `compile_program(...)`.
  - Pattern: `tests/integration_e2e/fs_predicates.rs:86-180` - serial filesystem test structure and cleanup.
  - API/Type: `runtime/opal_runtime.h` - C declarations for `delete_directory_recursive_sync`, `path_exists_sync`, and `is_directory_sync`.
  - API/Type: `runtime/opal_fs.c` - runtime implementations; only modify if Task 5 proves a defect.
  - API/Type: `src/type_system/module_resolver/standard_symbols_filesystem_operations.rs` - language signatures and declared error types.

  **Acceptance Criteria** (agent-executable only):
  - [ ] New test file `tests/integration_e2e/fs_delete_directory_recursive.rs` exists and is registered in `tests/integration_e2e/tests.rs`.
  - [ ] The test source contains `fn fs_recursive_delete_from_op_source()`.
  - [ ] The inline `.op` source includes `delete_directory_recursive_sync` and prints `exists_after=` and `dir_after=`.
  - [ ] `cargo test --all-features --test integration_e2e fs_recursive_delete_from_op_source -- --exact` exits 0 and reports `test result: ok. 1 passed` after implementation fixes are complete.

  **QA Scenarios** (MANDATORY - task incomplete without these):
  ```
  Scenario: Happy path recursive deletion from Opalescent source
    Tool: Bash
    Steps: Run `cargo test --all-features --test integration_e2e fs_recursive_delete_from_op_source -- --exact`.
    Expected: Command exits 0; output includes `test result: ok. 1 passed`; stdout captured by the test contains `exists_after=false` or `exists_after=0`, and `dir_after=false` or `dir_after=0`; Rust postcondition confirms `!fixture_root.exists()`.
    Evidence: .sisyphus/evidence/task-2-recursive-delete.txt

  Scenario: Sandbox guard prevents unsafe recursive delete
    Tool: Bash
    Steps: Inspect/run the same test and assert it contains `fixture_root.starts_with(std::env::temp_dir())`; run `cargo test --all-features --test integration_e2e fs_recursive_delete_from_op_source -- --exact`.
    Expected: The test fails before invoking `.op` if fixture root is outside temp dir; normal run exits 0 in temp dir.
    Evidence: .sisyphus/evidence/task-2-sandbox-guard.md
  ```

  **Commit**: NO | Message: N/A | Files: `tests/integration_e2e/fs_delete_directory_recursive.rs`, `tests/integration_e2e/tests.rs`, `.sisyphus/evidence/task-2-*`

- [x] 3. Add TDD `.op` integration test for the complete empty-directory workflow

  **What to do**: Extend `tests/integration_e2e/fs_delete_directory_recursive.rs` with `fs_empty_directory_workflow_from_op_source`. The Rust harness must create this exact fixture under `let target_dir = temp_dir.join("empty-workflow")`: `a.txt`, `b.txt`, `sub/c.txt`, `sub/nested/d.txt`. It must compile inline `.op` that imports `path_from`, `list_directory_sync`, `join_path_components`, `is_directory_sync`, `delete_directory_recursive_sync`, and `delete_file_sync`. The `.op` program must: `guard list_directory_sync(base) into entries else err => { print('LIST_ERR={err}'); return void }`; iterate `for entry in entries:`; build `child = join_path_components(base, [entry])` or the diagnosed replacement path API if Task 5 proves this expression invalid; guard `is_directory_sync(child) into child_is_dir else err => { print('STAT_ERR={err}'); return void }`; if `child_is_dir`, guard `delete_directory_recursive_sync(child) into _ else err => { print('RMDIR_ERR={err}'); return void }`; else guard `delete_file_sync(child) into _ else err => { print('DEL_ERR={err}'); return void }`; after the loop, guard `list_directory_sync(base) into remaining else err => { print('FINAL_LIST_ERR={err}'); return void }` and print exactly `remaining={remaining_len}`. Rust postcondition must assert `std::fs::read_dir(&target_dir).unwrap().count() == 0`.
  **Must NOT do**: Do not delete the root `target_dir`; only delete its top-level children. Do not hardcode expected directory listing order. Do not use shell `rm` for test setup or verification.

  **Recommended Agent Profile**:
  - Category: `unspecified-high` - Reason: covers multiple filesystem stdlib functions and may expose type/codegen defects.
  - Skills: [] - No specialized skill needed.
  - Omitted: [`playwright`] - No browser/UI work.

  **Parallelization**: Can Parallel: YES | Wave 1 | Blocks: 5, 6, 7 | Blocked By: None

  **References** (executor has NO interview context - be exhaustive):
  - Pattern: `tests/integration_e2e/fs_predicates.rs:21-52` - inline `.op` source compilation and execution.
  - Pattern: `tests/integration_e2e/fs_helpers.rs:72-82` - unique temp build target helper.
  - Pattern: `runtime/opal_fs.c` - runtime implementation of `list_directory_sync`, `join_path_components`, `is_directory_sync`, `delete_directory_recursive_sync`, and `delete_file_sync`.
  - API/Type: `src/codegen/functions_call.rs` - guard/propagate and aggregate return lowering; inspect only if test fails.
  - API/Type: `src/codegen/error_abi.rs` - error slot conventions; inspect only if guard behavior fails.

  **Acceptance Criteria** (agent-executable only):
  - [ ] The inline `.op` source exercises all six required functions: `path_from`, `list_directory_sync`, `join_path_components`, `is_directory_sync`, `delete_directory_recursive_sync`, `delete_file_sync`.
  - [ ] The test function name is exactly `fs_empty_directory_workflow_from_op_source`.
  - [ ] `cargo test --all-features --test integration_e2e fs_empty_directory_workflow_from_op_source -- --exact` exits 0 and reports `test result: ok. 1 passed` after implementation fixes are complete.
  - [ ] Rust assertion verifies `std::fs::read_dir(&target_dir).unwrap().count() == 0` and `target_dir.exists()` remains true.

  **QA Scenarios** (MANDATORY - task incomplete without these):
  ```
  Scenario: Empty-directory workflow deletes files and nested directories but preserves root
    Tool: Bash
    Steps: Run `cargo test --all-features --test integration_e2e fs_empty_directory_workflow_from_op_source -- --exact`.
    Expected: Command exits 0; output includes `test result: ok. 1 passed`; `.op` stdout includes `remaining=0`; Rust asserts `target_dir.exists()` and `read_dir(target_dir).count() == 0`.
    Evidence: .sisyphus/evidence/task-3-empty-workflow.txt

  Scenario: Workflow does not depend on directory listing order
    Tool: Bash
    Steps: Review the test and run `cargo test --all-features --test integration_e2e fs_empty_directory_workflow_from_op_source -- --exact`.
    Expected: Assertions check final filesystem state and `remaining=0`, not a specific ordered list of initial entries.
    Evidence: .sisyphus/evidence/task-3-order-independent.md
  ```

  **Commit**: NO | Message: N/A | Files: `tests/integration_e2e/fs_delete_directory_recursive.rs`, `.sisyphus/evidence/task-3-*`

- [x] 4. Add TDD negative-path `.op` test for recursive delete error handling

  **What to do**: Add `fs_recursive_delete_missing_path_error_from_op_source` to `tests/integration_e2e/fs_delete_directory_recursive.rs`. The Rust harness must choose `missing_dir = temp_dir.join("missing-recursive-delete-target")`, assert it does not exist, and compile inline `.op` that imports `path_from` and `delete_directory_recursive_sync`. The `.op` source must use `guard delete_directory_recursive_sync(target) into ignored else err => { print('ERR_PATH={err}'); return void }`; if the delete unexpectedly succeeds, print `UNEXPECTED_SUCCESS`. Rust must assert process exit success, stdout contains `ERR_PATH=`, stdout does not contain `UNEXPECTED_SUCCESS`, and `!missing_dir.exists()`.
  **Must NOT do**: Do not require a non-zero process exit for expected handled errors. Do not test permission-denied in v1; permissions vary across platforms and CI containers.

  **Recommended Agent Profile**:
  - Category: `quick` - Reason: focused negative-path test using existing guard pattern.
  - Skills: [] - No specialized skill needed.
  - Omitted: [`deep`] - No architectural change unless this exposes a defect in Task 5.

  **Parallelization**: Can Parallel: YES | Wave 1 | Blocks: 5, 6, 7 | Blocked By: None

  **References** (executor has NO interview context - be exhaustive):
  - Pattern: `tests/integration_e2e/fs_predicates.rs:21-52` - inline `.op` compile/run pattern.
  - API/Type: `src/type_system/checker/expressions_guard.rs` - guard type rules if the source does not typecheck.
  - API/Type: `src/codegen/functions_call.rs` - guard lowering if runtime behavior is wrong.
  - API/Type: `src/codegen/error_abi.rs` - error aggregate layout if the error branch is not taken correctly.

  **Acceptance Criteria** (agent-executable only):
  - [ ] Test function name is exactly `fs_recursive_delete_missing_path_error_from_op_source`.
  - [ ] Inline `.op` source contains `guard delete_directory_recursive_sync(target) into ignored else err` and prints `ERR_PATH=` in the else branch.
  - [ ] `cargo test --all-features --test integration_e2e fs_recursive_delete_missing_path_error_from_op_source -- --exact` exits 0 and reports `test result: ok. 1 passed` after implementation fixes are complete.

  **QA Scenarios** (MANDATORY - task incomplete without these):
  ```
  Scenario: Missing recursive-delete target uses handled error branch
    Tool: Bash
    Steps: Run `cargo test --all-features --test integration_e2e fs_recursive_delete_missing_path_error_from_op_source -- --exact`.
    Expected: Command exits 0; output includes `test result: ok. 1 passed`; captured `.op` stdout contains `ERR_PATH=` and does not contain `UNEXPECTED_SUCCESS`.
    Evidence: .sisyphus/evidence/task-4-missing-path-error.txt

  Scenario: Negative test leaves no filesystem residue
    Tool: Bash
    Steps: Run the same exact test command and assert the Rust postcondition checks `!missing_dir.exists()`.
    Expected: Command exits 0 and no directory is created at `missing-recursive-delete-target`.
    Evidence: .sisyphus/evidence/task-4-no-residue.md
  ```

  **Commit**: NO | Message: N/A | Files: `tests/integration_e2e/fs_delete_directory_recursive.rs`, `.sisyphus/evidence/task-4-*`

- [x] 5. Diagnose failing tests and classify the exact defect layer

  **What to do**: After Tasks 2-4 introduce failing/target tests, run each exact test command and classify failures into one or more buckets: test harness/source syntax, type resolver signature, parser/typechecker guard/for-loop issue, `FilesystemPath[]` iteration/path API ergonomics, codegen stdlib declaration, aggregate/error ABI lowering, or runtime implementation. Write `.sisyphus/evidence/task-5-diagnosis.md` with a table: test name, failing command, observed stderr/stdout, defect bucket, files to inspect/fix, and whether the conditional path API task is activated. If all tests pass without implementation changes, record `implementation fixes required: NO` and skip Task 6 except for formatting/lint checks.
  **Must NOT do**: Do not implement fixes in this task. Do not make speculative broad refactors. Do not activate path API work merely because a different style would be nicer; activate only if the workflow cannot be expressed with existing APIs or requires unsafe/opaque casts.

  **Recommended Agent Profile**:
  - Category: `deep` - Reason: requires tracing failures across language, compiler, codegen, and runtime boundaries.
  - Skills: [] - No specialized skill needed.
  - Omitted: [`frontend-ui-ux`] - No UI work.

  **Parallelization**: Can Parallel: NO | Wave 2 | Blocks: 6, 7 | Blocked By: 1, 2, 3, 4

  **References** (executor has NO interview context - be exhaustive):
  - API/Type: `src/type_system/module_resolver/standard_symbols_filesystem_operations.rs` - filesystem function signatures.
  - API/Type: `src/type_system/checker/expressions_guard.rs` - guard typing.
  - API/Type: `src/codegen/functions_stdlib.rs` - LLVM declarations and sret handling.
  - API/Type: `src/codegen/functions_call.rs` - call, guard, and propagate lowering.
  - API/Type: `src/codegen/error_abi.rs` - result aggregate/error field conventions.
  - API/Type: `runtime/opal_runtime.h` - C ABI declarations.
  - API/Type: `runtime/opal_fs.c` - runtime behavior.

  **Acceptance Criteria** (agent-executable only):
  - [ ] `.sisyphus/evidence/task-5-diagnosis.md` exists with a row for each of the three named tests from Tasks 2-4.
  - [ ] Diagnosis explicitly states `conditional path API task: ACTIVATED` or `conditional path API task: NOT ACTIVATED` with the exact reason.
  - [ ] Every proposed implementation file in Task 6 is justified by a failing command and observed failure.

  **QA Scenarios** (MANDATORY - task incomplete without these):
  ```
  Scenario: Failure classification is reproducible
    Tool: Bash
    Steps: Run the three exact commands for `fs_recursive_delete_from_op_source`, `fs_empty_directory_workflow_from_op_source`, and `fs_recursive_delete_missing_path_error_from_op_source`; save command outputs in the diagnosis evidence.
    Expected: Each failing or passing state is tied to a concrete command output and defect bucket.
    Evidence: .sisyphus/evidence/task-5-diagnosis.md

  Scenario: Path API activation gate is objective
    Tool: Bash
    Steps: In the workflow test, attempt the existing expression `join_path_components(base, [entry])`; record compiler/runtime outcome.
    Expected: If it compiles and passes, path API is NOT ACTIVATED; if it fails due to `FilesystemPath` composition/iteration limitations, path API is ACTIVATED with exact compiler error.
    Evidence: .sisyphus/evidence/task-5-path-api-gate.md
  ```

  **Commit**: NO | Message: N/A | Files: `.sisyphus/evidence/task-5-*`

- [x] 6. Implement only diagnosis-proven fixes, including conditional broader path API if activated

  **What to do**: Apply the minimal fix set justified by Task 5. If failures are in stdlib signature/type resolver, update `src/type_system/module_resolver/standard_symbols_filesystem_operations.rs` and corresponding tests. If failures are in LLVM declarations or call lowering, update `src/codegen/functions_stdlib.rs`, `src/codegen/functions_call.rs`, or `src/codegen/error_abi.rs` only where the diagnosis points. If runtime behavior is defective, update `runtime/opal_fs.c` and/or `runtime/opal_runtime.h` to keep C ABI and codegen consistent. If conditional path API is ACTIVATED, design a broader but bounded path API improvement that makes the workflow ergonomic from `.op` source; acceptable outcomes include direct `FilesystemPath` entry compatibility with `join_path_components` or a clearly named standard helper for parent+entry joining. The path API change must be covered by `fs_empty_directory_workflow_from_op_source`; add only additional targeted tests if the API has behavior not exercised by that workflow.
  **Must NOT do**: Do not touch unrelated filesystem APIs. Do not implement Windows support. Do not add shell calls to `rm`, `rmdir`, or `del`. Do not add a stdlib function unless Task 5 shows existing APIs cannot express the workflow ergonomically.

  **Recommended Agent Profile**:
  - Category: `unspecified-high` - Reason: implementation may span compiler/runtime/test layers depending on diagnosis.
  - Skills: [] - No specialized skill needed.
  - Omitted: [`visual-engineering`] - No visual/UI work.

  **Parallelization**: Can Parallel: NO | Wave 2 | Blocks: 7 | Blocked By: 5

  **References** (executor has NO interview context - be exhaustive):
  - Diagnosis: `.sisyphus/evidence/task-5-diagnosis.md` - authoritative list of files allowed to change.
  - Runtime: `runtime/opal_runtime.h` and `runtime/opal_fs.c` - keep signatures and result structs consistent.
  - Type system: `src/type_system/module_resolver/standard_symbols_filesystem_operations.rs` - language signatures.
  - Codegen: `src/codegen/functions_stdlib.rs`, `src/codegen/functions_call.rs`, `src/codegen/error_abi.rs` - stdlib declarations and error/aggregate lowering.
  - Tests: `tests/integration_e2e/fs_delete_directory_recursive.rs` - all new behavior must be proven here first.

  **Acceptance Criteria** (agent-executable only):
  - [ ] All files changed in this task are listed in `.sisyphus/evidence/task-5-diagnosis.md` or justified by a direct compiler error found while implementing the diagnosed fix.
  - [ ] `cargo test --all-features --test integration_e2e fs_recursive_delete_from_op_source -- --exact` exits 0 and reports `test result: ok. 1 passed`.
  - [ ] `cargo test --all-features --test integration_e2e fs_empty_directory_workflow_from_op_source -- --exact` exits 0 and reports `test result: ok. 1 passed`.
  - [ ] `cargo test --all-features --test integration_e2e fs_recursive_delete_missing_path_error_from_op_source -- --exact` exits 0 and reports `test result: ok. 1 passed`.

  **QA Scenarios** (MANDATORY - task incomplete without these):
  ```
  Scenario: Diagnosis-proven implementation fixes make TDD tests green
    Tool: Bash
    Steps: Run all three exact new test commands from the acceptance criteria.
    Expected: Each command exits 0 and includes `test result: ok. 1 passed`.
    Evidence: .sisyphus/evidence/task-6-targeted-tests.txt

  Scenario: No speculative path API work when gate is not activated
    Tool: Bash
    Steps: Compare changed files against `.sisyphus/evidence/task-5-diagnosis.md` and run `git diff -- src/type_system src/codegen runtime tests/integration_e2e`.
    Expected: If path API gate says NOT ACTIVATED, no new stdlib path API symbols or broad path API files are changed beyond required test/fix files.
    Evidence: .sisyphus/evidence/task-6-scope-control.md
  ```

  **Commit**: NO | Message: N/A | Files: Diagnosis-dependent; expected candidates include `tests/integration_e2e/fs_delete_directory_recursive.rs`, `tests/integration_e2e/tests.rs`, `src/type_system/module_resolver/standard_symbols_filesystem_operations.rs`, `src/codegen/functions_stdlib.rs`, `src/codegen/functions_call.rs`, `src/codegen/error_abi.rs`, `runtime/opal_runtime.h`, `runtime/opal_fs.c`

- [ ] 7. Produce objective readiness checklist and run CI-equivalent verification

  **What to do**: Create `.sisyphus/evidence/task-7-readiness-report.md` with the exact verdict format below. Verdict is `READY for Linux directory-emptying workflow` only if every command exits 0; otherwise verdict is `NOT READY` with the failing command and remaining defect bucket. Include the final answer to the user's question: Opalescent is ready for this class of Linux work if the checklist is green; otherwise name exactly what must still be added. Run full CI-equivalent commands.

  Required report format:
  ```markdown
  # Opalescent Directory Emptying Readiness

  Verdict: READY|NOT READY

  ## Required Functions Exercised From .op Source
  - path_from: YES/NO via fs_empty_directory_workflow_from_op_source
  - list_directory_sync: YES/NO via fs_empty_directory_workflow_from_op_source
  - join_path_components or approved path API replacement: YES/NO via fs_empty_directory_workflow_from_op_source
  - is_directory_sync: YES/NO via fs_predicates_matrix and fs_empty_directory_workflow_from_op_source
  - delete_directory_recursive_sync: YES/NO via fs_recursive_delete_from_op_source and fs_empty_directory_workflow_from_op_source
  - delete_file_sync: YES/NO via fs_empty_directory_workflow_from_op_source

  ## Commands
  - cargo test --all-features --test integration_e2e fs_predicates_matrix -- --exact: PASS/FAIL
  - cargo test --all-features --test integration_e2e fs_recursive_delete_from_op_source -- --exact: PASS/FAIL
  - cargo test --all-features --test integration_e2e fs_empty_directory_workflow_from_op_source -- --exact: PASS/FAIL
  - cargo test --all-features --test integration_e2e fs_recursive_delete_missing_path_error_from_op_source -- --exact: PASS/FAIL
  - cargo test --all-features: PASS/FAIL
  - cargo clippy --all-targets --all-features -- -D warnings: PASS/FAIL
  - cargo fmt --all -- --check: PASS/FAIL

  ## Scope Exclusions Confirmed
  - No real home/download paths used: YES/NO
  - No Windows support added: YES/NO
  - No tilde expansion added: YES/NO
  - No async/progress/dry-run/trash behavior added: YES/NO
  ```
  **Must NOT do**: Do not declare READY if any listed command fails. Do not write a subjective readiness verdict without command evidence.

  **Recommended Agent Profile**:
  - Category: `writing` - Reason: primarily evidence/reporting plus command verification.
  - Skills: [] - No specialized skill needed.
  - Omitted: [`deep`] - Deep debugging belongs to Task 5/6, not final reporting.

  **Parallelization**: Can Parallel: NO | Wave 3 | Blocks: Final Verification | Blocked By: 1, 2, 3, 4, 5, 6

  **References** (executor has NO interview context - be exhaustive):
  - Evidence: `.sisyphus/evidence/task-1-coverage-inventory.md`
  - Evidence: `.sisyphus/evidence/task-5-diagnosis.md`
  - Tests: `tests/integration_e2e/fs_predicates.rs`, `tests/integration_e2e/fs_delete_directory_recursive.rs`
  - CI: `.github/workflows/ci.yml` - confirms CI-equivalent command set.

  **Acceptance Criteria** (agent-executable only):
  - [ ] `.sisyphus/evidence/task-7-readiness-report.md` exists and follows the required format.
  - [ ] Report verdict is `READY` iff every listed command is PASS.
  - [ ] `cargo test --all-features` exits 0.
  - [ ] `cargo clippy --all-targets --all-features -- -D warnings` exits 0.
  - [ ] `cargo fmt --all -- --check` exits 0.

  **QA Scenarios** (MANDATORY - task incomplete without these):
  ```
  Scenario: CI-equivalent verification passes
    Tool: Bash
    Steps: Run `cargo test --all-features && cargo clippy --all-targets --all-features -- -D warnings && cargo fmt --all -- --check`.
    Expected: Entire command exits 0.
    Evidence: .sisyphus/evidence/task-7-ci-equivalent.txt

  Scenario: Readiness verdict is evidence-backed
    Tool: Bash
    Steps: Read `.sisyphus/evidence/task-7-readiness-report.md` and compare PASS/FAIL lines to captured command outputs.
    Expected: `Verdict: READY` appears only when every command line is PASS; otherwise `Verdict: NOT READY` names failing commands.
    Evidence: .sisyphus/evidence/task-7-readiness-report.md
  ```

  **Commit**: YES | Message: `test(fs): validate recursive directory emptying from op source` | Files: implementation/test files changed plus evidence files if repository convention requires evidence commits

## Final Verification Wave (MANDATORY — after ALL implementation tasks)
> 4 review agents run in PARALLEL. ALL must APPROVE. Present consolidated results to user and get explicit "okay" before completing.
> **Do NOT auto-proceed after verification. Wait for user's explicit approval before marking work complete.**
> **Never mark F1-F4 as checked before getting user's okay.** Rejection or user feedback -> fix -> re-run -> present again -> wait for okay.
- [x] F1. Plan Compliance Audit — oracle
- [x] F2. Code Quality Review — unspecified-high
- [x] F3. Real Manual QA — unspecified-high
- [x] F4. Scope Fidelity Check — deep

## Commit Strategy
- Commit after all tests and verification pass.
- Suggested commit message: `test(fs): validate recursive directory emptying from op source`
- Include only source/test files required by the validated implementation. Exclude `.sisyphus/evidence/` unless the repository convention requires evidence to be committed.

## Success Criteria
- The readiness answer is objective: Opalescent is ready for this Linux directory-emptying class of work iff all named tests in Definition of Done pass on a clean checkout.
- If conditional path API work is activated, its API is covered by the full-workflow `.op` integration test and documented in the readiness checklist.
- No final state depends on human inspection or manual cleanup.
