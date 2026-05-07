# Fix Windows File-System and Build Issues

## TL;DR
> **Summary**: Fix every unchecked item in `WINDOWS_ISSUES.md` with MSVC as the primary Windows target, Wine as the automated Windows runtime gate, and Linux behavior protected after every full unit of work. Work starts by making Windows builds reproducible, then centralizes Windows file/path behavior behind the runtime portability boundary, then fixes semantic/runtime/linker/hot-reload issues with TDD where infrastructure supports it.
> **Deliverables**:
> - Reproducible Linux→Windows MSVC compiler/runtime builds under xwin.
> - Wine-run Opalescent program that performs create/read/write/list/delete/rename file operations on Windows paths with spaces and Unicode.
> - Runtime C path/file APIs that support Windows separators, drive paths, UNC paths, UTF-8 paths via wide Win32 APIs, errno propagation, symlink metadata, and long paths.
> - Build-system fixes for LLVM dynamic feature, xwin pinning, bcrypt, missing `opal_rc.c`, structured xwin errors, and path quoting.
> - Hot-reload DLL copy-before-load behavior verified or completed.
> - Atomic commits per complete Windows issue unit.
> **Effort**: Large
> **Parallel**: YES - 5 waves
> **Critical Path**: Task 1 → Task 2 → Task 3 → Task 4 → Task 5 → Task 8 → Task 10 → Task 12 → Final Verification Wave

## Context

### Original Request
Fix all Windows-related issues specified in `WINDOWS_ISSUES.md` while not regressing Linux logic. The Opalescent compiler must verifiably produce Windows builds that can run successfully under Wine and do file operations. Use TDD with red-green-refactor where possible. Use atomic commits at full units of work.

### Interview Summary
- Scope: every unchecked issue listed in `WINDOWS_ISSUES.md` is in scope, including runtime C layer, build system, and hot reload.
- Windows target decision: **MSVC primary**. MinGW must not be intentionally regressed, but MinGW Wine-run acceptance is not required.
- Native Windows QA decision: **Wine only** for automated edge QA. Symlink and DLL-lock limitations under Wine must be documented as caveats; no manual native Windows gate is required.
- Test decision: **TDD red-green-refactor where possible** using existing Cargo unit tests, `integration`, and `windows-wine` feature harnesses.
- Commit decision: **atomic commits at full units of work**, grouped only when issues share an inseparable implementation boundary.

### Key Decisions and Defaults Applied
- **MSVC primary**: all Windows runnable acceptance criteria target `x86_64-pc-windows-msvc`.
- **Wine-only automated Windows runtime gate**: no native Windows manual or CI-only acceptance is required beyond existing `windows-latest` smoke where maintained.
- **MinGW non-regression definition**: compile/link smoke only where environment exists; no MinGW Wine run.
- **Unicode path acceptance**: include ASCII, spaces, and non-ASCII (`unicode-é-中`) file paths because issues 6 and 7 explicitly require Unicode-safe Windows FS I/O.
- **Long-path acceptance**: support paths longer than 260 bytes using dynamic allocation and Windows long-path handling where needed because issue 16 explicitly cites MAX_PATH truncation/overflow.
- **xwin pin default**: pin CI install to an explicit `xwin` version with `--locked`. Executor must first query the installed/available crate version; use the current latest compatible version if available, record it in `.github/workflows/ci.yml`, and stop if version discovery fails rather than leaving `cargo install xwin --locked` unpinned.
- **Wine default**: `scripts/verify-wine-prereqs.sh` must require `wine --version` parses to major version `>= 8`; lower versions skip/fail with an explicit diagnostic.
- **errno policy**: continue exposing stdlib-level existing error discriminants; map Win32 failures to POSIX `errno` via `opal_set_errno_from_win32` and existing C runtime error mapping, not by creating new Opalescent error types.

### Metis Review (gaps addressed)
- Added explicit defaults for xwin pin, Wine minimum, Unicode/long-path scope, errno mapping, and MinGW non-regression.
- Added guardrail to avoid a new broad path abstraction such as `opal_path_t`; use minimum viable helper boundary in `runtime/opal_portability.h` and `runtime/opal_fs.c`.
- Added mandatory immediate `GetLastError()` capture rule for every Win32 failure.
- Added explicit Wine caveat handling for symlink and DLL-lock tests.
- Added exact QA command patterns and evidence outputs to avoid human verification.

## Work Objectives

### Core Objective
Make Opalescent-produced Windows MSVC executables compile, link, run under Wine, and perform file operations correctly without regressing Linux behavior.

### Deliverables
- Fixed runtime C path helper behavior for `\`, drive-letter, UNC, Unicode, long path, and root cases.
- Fixed Windows file I/O wrappers to use UTF-8→UTF-16 wide Win32 APIs at the Windows edge.
- Fixed errno propagation and symlink metadata behavior in portability helpers.
- Fixed compiler/build pipeline issues for MSVC runtime sources, linker libraries, xwin diagnostics, Cargo LLVM features, CI pinning, and `Command::arg()` quoting misuse.
- Verified or completed hot-reload copy-before-load behavior for Windows DLLs.
- New tests and evidence artifacts for Linux regression and Wine MSVC file-operation success.

### Definition of Done (verifiable conditions with commands)
- `cargo test --all-features --workspace` exits 0 on Linux.
- `cargo clippy --all-targets --all-features -- -D warnings` exits 0.
- `cargo fmt --all -- --check` exits 0.
- `bash scripts/verify-wine-prereqs.sh` exits 0 in configured CI/Wine environment, or emits a structured `SKIP:` diagnostic outside that environment.
- `cargo test --features "integration windows-wine" --test integration_e2e -- --nocapture wine_msvc_file_ops` exits 0 in configured Wine/xwin environment.
- `cargo run --release -- test-projects/hello-world/src/main.op --target x86_64-pc-windows-msvc` produces a `.exe`, and `wine <exe>` exits 0.
- Final evidence bundle exists under `.sisyphus/evidence/windows-issues-final/` with stdout/stderr/exit files for Linux tests, Wine MSVC file ops, and linker checks.

### Must Have
- Red tests before implementation where current harness can express the failure.
- Every changed Windows API call captures `GetLastError()` immediately after failure.
- Windows runtime accepts UTF-8 internally and converts only at the Windows portability boundary.
- Linux code path remains behaviorally unchanged except where tests prove a shared bug fix is intentional.
- Atomic commits that correspond to issue checkboxes or explicitly justified inseparable groups.

### Must NOT Have
- No source changes outside the listed issue scope except tests/evidence needed to verify them.
- No new broad path abstraction (`opal_path_t`, global path object rewrite, or new stdlib FS API surface).
- No new Opalescent error type for Windows errno mapping.
- No MinGW Wine-run gate.
- No native-Windows-only acceptance requirement.
- No incidental refactors in unrelated compiler/linker/codegen paths.
- No use of literal quote bytes with `std::process::Command::arg()`.
- No static string literal assigned to any runtime `.error` field that consumers free.

## Verification Strategy
> ZERO HUMAN INTERVENTION - all verification is agent-executed.
- Test decision: TDD red-green-refactor where possible with Rust unit tests, `integration`, and `windows-wine` feature tests.
- QA policy: Every task has agent-executed scenarios.
- Evidence: `.sisyphus/evidence/task-{N}-{slug}.{ext}` and final bundle `.sisyphus/evidence/windows-issues-final/`.
- Baseline command before changes: `cargo test --all-features --workspace 2>&1 | tee .sisyphus/evidence/task-1-linux-baseline.txt`.
- Per-unit Linux regression command: `cargo test --all-features --workspace`.
- Wine prereq command: `bash scripts/verify-wine-prereqs.sh`.
- Wine MSVC command: `cargo test --features "integration windows-wine" --test integration_e2e -- --nocapture wine_msvc_file_ops` (test binary is `tests/integration_e2e.rs`, which is gated by `integration`; harness module is `tests/integration_e2e/windows_wine.rs`, which is gated by `windows-wine`).

## Execution Strategy

### Parallel Execution Waves
> Target: 5-8 tasks per wave. <3 per wave (except final) = under-splitting.
> Extract shared dependencies as Wave-1 tasks for max parallelism.

Wave 1: Task 1 baseline + Task 2 build/linker bootstrap + Task 3 Wine fixture/harness hardening.
Wave 2: Task 4 runtime portability boundary + Task 5 path helper semantics + Task 6 Unicode wide FS I/O.
Wave 3: Task 7 directory/errno/symlink semantics + Task 8 runtime init/source inclusion/RC + Task 9 long-path safety.
Wave 4: Task 10 CI/xwin/Cargo hardening + Task 11 hot-reload DLL copy-before-load verification/completion.
Wave 5: Task 12 checklist closure and final verification bundle.

### Dependency Matrix (full, all tasks)
- Task 1 blocks all tasks by establishing Linux baseline and issue checklist.
- Task 2 blocks Task 3 and Task 12 because Wine MSVC `.exe` generation depends on build/link fixes.
- Task 3 blocks Task 6, Task 7, Task 9, Task 12 by providing file-operation Wine acceptance fixture.
- Task 4 blocks Task 5, Task 6, Task 7, Task 9 by defining the Windows portability boundary.
- Task 5 blocks Task 6 and Task 9 because path parsing/root behavior feeds file APIs.
- Task 6 blocks Task 7 and Task 12 because directory enumeration/error tests rely on Unicode-safe APIs.
- Task 7 blocks Task 12.
- Task 8 blocks Task 12 and can run after Task 2.
- Task 9 blocks Task 12 and depends on Tasks 4-6.
- Task 10 can run after Task 2 but final CI checks depend on Tasks 3 and 12.
- Task 11 depends on Tasks 4-6 and blocks Task 12.
- Task 12 depends on Tasks 1-11.

### Agent Dispatch Summary (wave → task count → categories)
- Wave 1 → 3 tasks → unspecified-high, deep.
- Wave 2 → 3 tasks → deep, unspecified-high.
- Wave 3 → 3 tasks → unspecified-high, deep.
- Wave 4 → 2 tasks → quick, unspecified-high.
- Wave 5 → 1 task → deep.

## TODOs
> Implementation + Test = ONE task. Never separate.
> EVERY task MUST have: Agent Profile + Parallelization + QA Scenarios.

- [x] 1. Establish Baseline, Issue Checklist, and Blast-Radius Map

  **What to do**: Capture current Linux test baseline, record every unchecked `WINDOWS_ISSUES.md` item as an implementation checklist in the working notes, and map references before any code changes. Run `ast_grep_search` for Windows preprocessor checks and LSP/reference lookups for `compile_runtime_c_to_obj`, `LinkerCommand`, `lex_normalize_path`, `join_path_components`, `opal_opendir`, and `opal_stat`. Do not change implementation in this task except adding test scaffolding only if needed to make baseline commands reproducible.
  **Must NOT do**: Do not fix any issue in this task. Do not rewrite runtime or linker code. Do not alter CI.

  **Recommended Agent Profile**:
  - Category: `deep` - Reason: Cross-cutting baseline and blast-radius mapping affect every subsequent unit.
  - Skills: [] - No specialized skill required.
  - Omitted: [`git-master`] - Git operations are simple status/diff/commit only and can be executed directly by the worker if instructed.

  **Parallelization**: Can Parallel: NO | Wave 1 | Blocks: [2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12] | Blocked By: []

  **References** (executor has NO interview context - be exhaustive):
  - Issue List: `WINDOWS_ISSUES.md:7-64` and `WINDOWS_ISSUES.md:68-69` - every unchecked item is in scope.
  - Test Config: `Cargo.toml` - features include `integration` and `windows-wine`; current inkwell features include `llvm14-0-prefer-dynamic` to remove later.
  - Integration Harness: `tests/integration_e2e/windows_wine.rs` - Wine prereq/build/run/evidence helpers.
  - CI: `.github/workflows/ci.yml` - Linux, Windows, and cross-MSVC-from-Linux jobs.
  - Runtime: `runtime/opal_fs.c`, `runtime/opal_portability.h` - primary C runtime blast radius.
  - Linker: `src/build_system/linker.rs` - MSVC/MinGW/quote/xwin behavior.
  - Compiler: `src/compiler.rs` - runtime source bundling and object/link orchestration.

  **Acceptance Criteria** (agent-executable only):
  - [ ] `cargo test --all-features --workspace 2>&1 | tee .sisyphus/evidence/task-1-linux-baseline.txt` exits 0 or records current failures exactly in `.sisyphus/evidence/task-1-linux-baseline.txt` with no source changes.
  - [ ] `ast_grep_search` or equivalent records every `#ifdef _WIN32`, `_MSC_VER`, and `OPAL_HAS_DIRENT` occurrence in `.sisyphus/evidence/task-1-windows-blast-radius.txt`.
  - [ ] `git status --short` shows only evidence/checklist artifacts or no changes before Task 2 begins.

  **QA Scenarios** (MANDATORY - task incomplete without these):
  ```
  Scenario: Linux baseline captured
    Tool: Bash
    Steps: Run `cargo test --all-features --workspace 2>&1 | tee .sisyphus/evidence/task-1-linux-baseline.txt`.
    Expected: Command exits 0, or existing failures are captured with full output and no implementation files changed.
    Evidence: .sisyphus/evidence/task-1-linux-baseline.txt

  Scenario: Windows blast radius captured
    Tool: Bash / ast_grep_search
    Steps: Search runtime and src for `_WIN32`, `_MSC_VER`, `OPAL_HAS_DIRENT`, `FindFirstFile`, `MoveFileEx`, `fopen`, `_stat64`, and `XWIN_CACHE`; save paths and line summaries.
    Expected: Evidence file includes `runtime/opal_fs.c`, `runtime/opal_portability.h`, `src/build_system/linker.rs`, and `src/compiler.rs` entries.
    Evidence: .sisyphus/evidence/task-1-windows-blast-radius.txt
  ```

  **Commit**: NO | Message: `chore(windows): capture baseline and blast radius` | Files: [.sisyphus/evidence/task-1-*]

- [x] 2. Fix MSVC Build/Link Bootstrap Issues

  **What to do**: Add red unit tests and implementation for build-system issues 17, 18, 19, and 20. In `src/build_system/linker.rs`, add `/DEFAULTLIB:bcrypt` to MSVC shared args; replace missing `XWIN_CACHE`/`OPAL_XWIN_SYSROOT` panic with structured linker/build error propagated to the compiler; remove literal quoting from `Command::arg()` usage by making `quote_if_needed` either deleted for `Command::arg` paths or constrained to string-rendering tests only. In `src/compiler.rs`, include `runtime/opal_rc.c` in `RUNTIME_SOURCE`. Preserve MinGW `-lbcrypt` behavior.
  **Must NOT do**: Do not edit runtime filesystem behavior here. Do not change Linux `-no-pie`. Do not introduce manual shell command strings for linker invocation.

  **Recommended Agent Profile**:
  - Category: `deep` - Reason: Build/linker error propagation spans Rust command construction, compiler error handling, and tests.
  - Skills: [] - No special skill required.
  - Omitted: [`frontend-ui-ux`] - No UI work.

  **Parallelization**: Can Parallel: NO | Wave 1 | Blocks: [3, 8, 10, 12] | Blocked By: [1]

  **References**:
  - Issues: `WINDOWS_ISSUES.md:54-64` for bcrypt, `opal_rc.c`, xwin panic, and quote misuse.
  - Linker: `src/build_system/linker.rs:86` (`mingw_crt_libs` already has `-lbcrypt`), `src/build_system/linker.rs:217-254` (`build_msvc`, xwin env), `src/build_system/linker.rs:254-286` (`msvc_shared_args`), `src/build_system/linker.rs:286` (`quote_if_needed`).
  - Compiler Runtime: `src/compiler.rs:42-65` (`RUNTIME_SOURCE`, `OPAL_RC_H`, `OPAL_RUNTIME_H`) and `src/compiler.rs:330-444` (`compile_runtime_c_to_obj`, `link_object_files`).
  - Existing Tests: `src/build_system/linker.rs:408-656` contains linker/env unit-test patterns with `ENV_TEST_LOCK`.

  **Acceptance Criteria**:
  - [ ] Red tests fail before implementation: MSVC args lack bcrypt, missing xwin currently panics, `Command::arg()` inputs are incorrectly quoted for paths with spaces, and `RUNTIME_SOURCE` lacks `opal_rc.c` if not already fixed.
  - [ ] `cargo test --lib build_system::linker -- --nocapture` exits 0 after implementation.
  - [ ] `cargo test --lib compiler -- --nocapture` exits 0 after implementation.
  - [ ] `cargo test --all-features --workspace` exits 0.
  - [ ] `git diff` shows no changes to runtime FS semantics in `runtime/opal_fs.c` or `runtime/opal_portability.h`.

  **QA Scenarios**:
  ```
  Scenario: MSVC linker receives bcrypt and unquoted path args
    Tool: Bash
    Steps: Run `cargo test --lib build_system::linker -- --nocapture msvc` after adding tests covering output/input paths with spaces and `bcrypt.lib`.
    Expected: Tests pass; debug args include `/DEFAULTLIB:bcrypt` and no standalone literal quote characters around paths passed via `Command::arg()`.
    Evidence: .sisyphus/evidence/task-2-msvc-linker.txt

  Scenario: Missing xwin produces structured diagnostic
    Tool: Bash
    Steps: Run the new unit/integration test with `XWIN_CACHE` and `OPAL_XWIN_SYSROOT` unset under `ENV_TEST_LOCK`.
    Expected: Test asserts non-panic error text containing `xwin SDK not found` or `XWIN_CACHE` remediation.
    Evidence: .sisyphus/evidence/task-2-xwin-error.txt
  ```

  **Commit**: YES | Message: `fix(build): repair MSVC runtime link bootstrap` | Files: [`src/build_system/linker.rs`, `src/compiler.rs`, relevant tests]

- [x] 3. Add Wine MSVC File-Operation Acceptance Fixture

  **What to do**: Extend the existing Wine harness to add a `wine_msvc_file_ops` test fixture that compiles an Opalescent program to `x86_64-pc-windows-msvc`, runs it under Wine, and verifies file operations. The fixture must exercise directory creation, writing, reading, listing/opening a directory, rename/replace if stdlib supports it, delete, paths with spaces, and non-ASCII directory/file names. If current Opal language/runtime cannot express one operation yet, assert the closest existing stdlib function and document the gap in the test name. This task may start red if runtime issues are unfixed but must compile as a test.
  **Must NOT do**: Do not fix runtime C file APIs in this task. Do not require native Windows. Do not make MinGW a runnable gate.

  **Recommended Agent Profile**:
  - Category: `deep` - Reason: Requires understanding Opal fixture syntax, test harness, and Wine evidence collection.
  - Skills: [] - No browser or UI skills needed.
  - Omitted: [`playwright`] - No browser interaction.

  **Parallelization**: Can Parallel: YES | Wave 1 | Blocks: [6, 7, 9, 12] | Blocked By: [1, 2]

  **References**:
  - Harness: `tests/integration_e2e/windows_wine.rs` - `check_prereqs`, `build_opal_project`, `run_under_wine`, `capture_evidence`.
  - Test Projects: `test-projects/hello-world/` and README test-project conventions.
  - Stdlib FS Surface: `src/type_system/module_resolver/standard_symbols_filesystem_operations.rs` - use actual available FS function names and error variants.
  - README: Windows build and Wine commands in `README.md` Windows Build section.

  **Acceptance Criteria**:
  - [ ] New `windows-wine` test compiles and is skipped only by `scripts/verify-wine-prereqs.sh` when prerequisites are absent.
  - [ ] In a configured Wine/xwin environment, the test initially fails for at least one currently listed Windows issue before runtime fixes, satisfying red step.
  - [ ] Evidence files include stdout, stderr, exit code, and host-side file content verification.
  - [ ] `cargo test --features "integration windows-wine" --test integration_e2e -- --list` includes `wine_msvc_file_ops`.

  **QA Scenarios**:
  ```
  Scenario: Wine file operations fixture is discoverable
    Tool: Bash
    Steps: Run `cargo test --features "integration windows-wine" --test integration_e2e -- --list | tee .sisyphus/evidence/task-3-wine-list.txt`.
    Expected: Output contains `wine_msvc_file_ops` exactly once.
    Evidence: .sisyphus/evidence/task-3-wine-list.txt

  Scenario: Wine fixture records deterministic evidence
    Tool: Bash
    Steps: In a configured Wine/xwin environment, run `cargo test --features "integration windows-wine" --test integration_e2e -- --nocapture wine_msvc_file_ops 2>&1 | tee .sisyphus/evidence/task-3-wine-red.txt`.
    Expected: Test either fails on a listed Windows runtime issue or passes if prior implementation already fixed it; evidence files are written under `.sisyphus/evidence/` or `target/test-evidence/windows_wine/`.
    Evidence: .sisyphus/evidence/task-3-wine-red.txt
  ```

  **Commit**: YES | Message: `test(windows): add Wine MSVC file operations fixture` | Files: [`tests/integration_e2e/windows_wine.rs`, `test-projects/windows-file-ops/**` or equivalent fixture]

- [x] 4. Centralize Windows UTF-8/Wide File API Boundary

  **What to do**: In `runtime/opal_portability.h` and supporting runtime C code, introduce minimal Windows-only helper wrappers for opening, stat-ing, deleting, renaming/replacing, mkdir/rmdir, realpath/absolute path, and directory enumeration that accept internal UTF-8 `char*` and call wide Win32 or wide CRT APIs (`_wfopen`, `_wstat64`/wide equivalent, `_wunlink`, `MoveFileExW`, `_wmkdir`, `_wrmdir`, `FindFirstFileW`). Ensure non-Windows remains direct POSIX/narrow behavior. Use existing `opal_utf8_to_wide` and `opal_wide_to_utf8`; extend only as needed. Replace narrow Windows calls that correspond to issue 6 and issue 7.
  **Must NOT do**: Do not create a new public Opalescent path abstraction. Do not change Linux wrappers. Do not leave Windows ANSI `A` APIs for filesystem paths.

  **Recommended Agent Profile**:
  - Category: `deep` - Reason: Runtime C portability boundary is the highest-risk cross-platform change.
  - Skills: [] - No extra skills required.
  - Omitted: [`frontend-ui-ux`] - No UI work.

  **Parallelization**: Can Parallel: NO | Wave 2 | Blocks: [5, 6, 7, 9, 11, 12] | Blocked By: [1]

  **References**:
  - Issues: `WINDOWS_ISSUES.md:22-27` for ANSI narrow APIs and `FindFirstFileA`.
  - Portability: `runtime/opal_portability.h:193-227` (`opal_utf8_to_wide`, `opal_wide_to_utf8`), `runtime/opal_portability.h:261-326` (`opal_opendir`, `FindFirstFileA`, `opal_closedir`), `runtime/opal_portability.h:497-526` (`opal_stat`, `opal_stat_nofollow`).
  - Runtime FS: `runtime/opal_fs.c:161`, `runtime/opal_fs.c:643`, `runtime/opal_fs.c:908`, `runtime/opal_fs.c:1020`, `runtime/opal_fs.c:1386`, `runtime/opal_fs.c:1475`, `runtime/opal_fs.c:1512` - representative `fopen` uses to route through wrapper.
  - Error Mapping: `runtime/opal_portability.h:154` (`opal_set_errno_from_win32`).

  **Acceptance Criteria**:
  - [ ] Unit/C tests or Wine fixture include non-ASCII path `unicode-é-中` and pass after implementation.
  - [ ] Search for `FindFirstFileA`, `MoveFileExA`, `_stat64`, `_unlink`, Windows `fopen` direct use shows none in Windows code paths except documented POSIX/non-Windows branches.
  - [ ] `cargo test --all-features --workspace` exits 0.
  - [ ] In configured environment, `cargo test --features "integration windows-wine" --test integration_e2e -- --nocapture wine_msvc_file_ops` reaches file create/write/read steps without ANSI path corruption.

  **QA Scenarios**:
  ```
  Scenario: Unicode path survives Windows file operations under Wine
    Tool: Bash
    Steps: Run `cargo test --features "integration windows-wine" --test integration_e2e -- --nocapture wine_msvc_file_ops 2>&1 | tee .sisyphus/evidence/task-4-wine-unicode.txt`.
    Expected: Stdout contains `unicode_path_ok=true`; host-side read verifies exact bytes `Hello, Opal!\n` from the Unicode file path.
    Evidence: .sisyphus/evidence/task-4-wine-unicode.txt

  Scenario: Linux runtime behavior preserved
    Tool: Bash
    Steps: Run `cargo test --all-features --workspace 2>&1 | tee .sisyphus/evidence/task-4-linux-regression.txt`.
    Expected: Exit code 0; no new Linux failures compared with Task 1 baseline.
    Evidence: .sisyphus/evidence/task-4-linux-regression.txt
  ```

  **Commit**: YES | Message: `fix(runtime): use wide Windows filesystem APIs` | Files: [`runtime/opal_portability.h`, `runtime/opal_fs.c`, relevant tests]

- [x] 5. Fix Windows Path Helper Semantics and `strdup` Usage

  **What to do**: Fix issues 1, 2, 3, and 4 in `runtime/opal_fs.c`. Update `path_parent_directory`, `path_file_name`, and `path_file_extension` to recognize both `/` and `\` separators. Replace bare `strdup` calls in `safe_strdup` and path helpers with `opal_strdup` from `opal_portability.h`. Update `lex_normalize_path` and `join_path_components` to recognize Windows drive-letter absolute paths (`C:\...`), UNC roots (`\\server\share`), mixed separators, and platform roots without hardcoding `/` on Windows. Preserve POSIX normalization outputs on Linux.
  **Must NOT do**: Do not alter Opalescent stdlib API signatures. Do not canonicalize paths by touching the filesystem; this is lexical normalization only.

  **Recommended Agent Profile**:
  - Category: `deep` - Reason: Path normalization has many edge cases and Linux regression risk.
  - Skills: [] - No extra skills required.
  - Omitted: [`playwright`] - No browser/UI.

  **Parallelization**: Can Parallel: NO | Wave 2 | Blocks: [6, 9, 12] | Blocked By: [1, 4]

  **References**:
  - Issues: `WINDOWS_ISSUES.md:7-18`.
  - Functions: `runtime/opal_fs.c:86` (`safe_strdup`), `runtime/opal_fs.c:367` (`lex_normalize_path`), `runtime/opal_fs.c:496` (`join_path_components`), `runtime/opal_fs.c:562-580` (`path_parent_directory`, `path_file_name`, `path_file_extension`).
  - Portability: `runtime/opal_portability.h` provides `opal_strdup` and `opal_path_separator`.

  **Acceptance Criteria**:
  - [ ] Tests cover parent/name/extension for `C:\Users\foo\bar.txt`, `C:/Users/foo/bar.txt`, `\\server\share\dir\file.ext`, `/tmp/file.ext`, relative paths, and extensionless files.
  - [ ] Tests cover `join_path_components("base", ["C:\\abs"] )` replacing accumulator on Windows instead of appending.
  - [ ] Tests cover `lex_normalize_path` preserving correct Windows roots and Linux `/` behavior.
  - [ ] `grep` or equivalent finds no bare `strdup(` in `runtime/opal_fs.c` except inside `opal_strdup` implementation if any.
  - [ ] `cargo test --all-features --workspace` exits 0.

  **QA Scenarios**:
  ```
  Scenario: Windows lexical path cases pass
    Tool: Bash
    Steps: Run the added path-helper unit/C tests and save output to `.sisyphus/evidence/task-5-path-tests.txt`.
    Expected: Cases for drive-letter, UNC, mixed separators, root collapse, parent directory, file name, and extension all pass exactly.
    Evidence: .sisyphus/evidence/task-5-path-tests.txt

  Scenario: POSIX path cases unchanged
    Tool: Bash
    Steps: Run `cargo test --all-features --workspace 2>&1 | tee .sisyphus/evidence/task-5-linux-regression.txt`.
    Expected: Exit code 0; existing Linux path/module tests still pass.
    Evidence: .sisyphus/evidence/task-5-linux-regression.txt
  ```

  **Commit**: YES | Message: `fix(runtime): normalize Windows path helpers` | Files: [`runtime/opal_fs.c`, `runtime/opal_portability.h`, relevant tests]

- [x] 6. Fix Runtime File I/O Error Allocation and Absolute Path Behavior

  **What to do**: Fix issue 5 and complete issue 3 interactions in `absolute_path_sync` and adjacent runtime FS functions. Ensure every non-NULL `.error` field is heap-allocated via `safe_strdup`/`opal_strdup`, never a string literal. Ensure absolute path detection uses the shared Windows absolute-path helper and root collapse returns the correct platform root. Add tests that free returned errors to catch literal-free crashes under MSVC/Wine.
  **Must NOT do**: Do not create new error variants. Do not change the existing consumer contract that errors are freed by callers.

  **Recommended Agent Profile**:
  - Category: `unspecified-high` - Reason: Focused runtime correctness with crash-risk tests.
  - Skills: [] - No extra skills required.
  - Omitted: [`frontend-ui-ux`] - No UI work.

  **Parallelization**: Can Parallel: YES | Wave 2 | Blocks: [7, 12] | Blocked By: [4, 5]

  **References**:
  - Issue: `WINDOWS_ISSUES.md:19-20` and root behavior in `WINDOWS_ISSUES.md:13-14`.
  - Function: `runtime/opal_fs.c:597` (`absolute_path_sync`).
  - Error Contract: `runtime/opal_fs_errors.h` forbids static literals because consumers free errors.
  - File I/O: `runtime/opal_fs.c:624-680`, `runtime/opal_fs.c:889-945`, `runtime/opal_fs.c:1463-1512` representative error assignment patterns.

  **Acceptance Criteria**:
  - [ ] Test triggers both `absolute_path_sync` error paths cited by issue 5 and frees `.error` without crash under Wine/MSVC.
  - [ ] Static analysis/search shows no direct assignment of string literals to `.error` fields in `runtime/opal_fs.c`.
  - [ ] `absolute_path_sync("C:\\Users\\foo")` and UNC input are treated as absolute on Windows.
  - [ ] Linux `absolute_path_sync("/tmp")` behavior remains unchanged.
  - [ ] `cargo test --all-features --workspace` exits 0.

  **QA Scenarios**:
  ```
  Scenario: Error fields are heap allocated
    Tool: Bash
    Steps: Run the new runtime/Wine test that calls `absolute_path_sync` error paths and frees `.error`.
    Expected: Exit code 0; no invalid free, crash, or MSVC CRT heap error.
    Evidence: .sisyphus/evidence/task-6-error-allocation.txt

  Scenario: Windows absolute paths are not mangled
    Tool: Bash
    Steps: Run `cargo test --features "integration windows-wine" --test integration_e2e -- --nocapture wine_msvc_file_ops` after fixture includes absolute path case.
    Expected: Stdout contains `absolute_path_ok=true` and no `InvalidPathError` for drive/UNC absolute inputs.
    Evidence: .sisyphus/evidence/task-6-wine-absolute.txt
  ```

  **Commit**: YES | Message: `fix(runtime): allocate filesystem errors safely` | Files: [`runtime/opal_fs.c`, `runtime/opal_fs_errors.h` if tests require, relevant tests]

- [x] 7. Fix Directory Enumeration Errno and Windows Symlink Metadata

  **What to do**: Fix issues 8, 9, 10, and 11. In `runtime/opal_portability.h`, make `opal_opendir` capture `GetLastError()` immediately on `FindFirstFileW` failure and map it with `opal_set_errno_from_win32`. Make `opal_closedir` set `errno` when `FindClose` fails. Remove the conflicting non-static extern forward declarations in `runtime/opal_fs.c` under `#if !OPAL_HAS_DIRENT` and rely on the portability header definitions. Update `opal_stat` on Windows to report `is_symlink` consistently for reparse points while preserving follow/nofollow semantics documented by existing code. Add tests for nonexistent dir, file-as-dir, and symlink/reparse metadata; Wine symlink limitations must use explicit ignore-with-reason only if the environment cannot reliably create symlinks.
  **Must NOT do**: Do not add new stdlib error variants. Do not silently skip errno assertions. Do not make native Windows a required gate.

  **Recommended Agent Profile**:
  - Category: `deep` - Reason: Error mapping and symlink semantics cross C portability and Opal stdlib metadata.
  - Skills: [] - No extra skills required.
  - Omitted: [`playwright`] - No browser/UI.

  **Parallelization**: Can Parallel: YES | Wave 3 | Blocks: [12] | Blocked By: [4, 6]

  **References**:
  - Issues: `WINDOWS_ISSUES.md:28-38`.
  - Directory Helpers: `runtime/opal_portability.h:261-326` (`opal_opendir`, `opal_readdir`, `opal_closedir`).
  - Forward Declarations: `runtime/opal_fs.c` `#if !OPAL_HAS_DIRENT` block described in issue 10.
  - Stat Helpers: `runtime/opal_portability.h:497-526` (`opal_stat`, `opal_stat_nofollow`).
  - FS Stdlib Surface: `src/type_system/module_resolver/standard_symbols_filesystem_operations.rs` for `read_metadata_sync` contract.

  **Acceptance Criteria**:
  - [ ] `opal_opendir` nonexistent path maps to `ENOENT` and produces existing `FileNotFoundError`/equivalent stdlib discriminant.
  - [ ] File-as-directory maps to `ENOTDIR` or existing closest discriminant documented in test.
  - [ ] `opal_closedir` failure path sets errno in a deterministic unit test or mockable wrapper test.
  - [ ] No conflicting extern declarations remain for `opal_opendir`, `opal_readdir`, or `opal_closedir` in `runtime/opal_fs.c`.
  - [ ] Windows symlink/reparse metadata sets `is_symlink` correctly where Wine supports it; otherwise test is ignored with explicit Wine limitation reason and native gate is not required.
  - [ ] `cargo test --all-features --workspace` exits 0.

  **QA Scenarios**:
  ```
  Scenario: Directory errno mapping is deterministic
    Tool: Bash
    Steps: Run the new directory errno tests and save output to `.sisyphus/evidence/task-7-dir-errno.txt`.
    Expected: Nonexistent directory produces `ENOENT`; file-as-directory produces `ENOTDIR` or documented existing discriminant; no stale errno.
    Evidence: .sisyphus/evidence/task-7-dir-errno.txt

  Scenario: Symlink metadata behavior is explicit
    Tool: Bash
    Steps: Run the symlink metadata test under Linux and configured Wine if available.
    Expected: Linux passes; Wine either passes or is marked ignored with reason `Wine limitation: symlink/reparse behavior differs from native Windows`.
    Evidence: .sisyphus/evidence/task-7-symlink-metadata.txt
  ```

  **Commit**: YES | Message: `fix(runtime): propagate Windows directory errno` | Files: [`runtime/opal_portability.h`, `runtime/opal_fs.c`, relevant tests]

- [x] 8. Add Runtime Initialization Call and Verify Reference Counting Source Inclusion

  **What to do**: Fix issue 15 and add regression coverage for issue 18 after Task 2. Ensure `runtime/opal_runtime.c` is included in `RUNTIME_SOURCE` and written to the runtime temp source. Verify `runtime/opal_rc.c` inclusion from Task 2 with a link test that exercises RC symbols. Update generated C main wrapper in `src/codegen/functions_call/tail.rs` so generated programs call `opal_runtime_init()` before user entrypoint code, exactly once, with no effect on Linux other than benign initialization. Add a generated-program test that verifies `opal_runtime_init` is referenced/called for Windows output.
  **Must NOT do**: Do not change user-level Opal entrypoint semantics. Do not call runtime init multiple times per process.

  **Recommended Agent Profile**:
  - Category: `unspecified-high` - Reason: Touches compiler runtime source embedding and generated C/LLVM wrapper behavior.
  - Skills: [] - No extra skills required.
  - Omitted: [`frontend-ui-ux`] - No UI work.

  **Parallelization**: Can Parallel: YES | Wave 3 | Blocks: [12] | Blocked By: [2]

  **References**:
  - Issues: `WINDOWS_ISSUES.md:40-41` and regression coverage for `WINDOWS_ISSUES.md:57-58`.
  - Runtime Source: `src/compiler.rs:42-65` (`RUNTIME_SOURCE`, included runtime headers).
  - Runtime Init: `runtime/opal_runtime.c`, `runtime/opal_runtime.h` - `opal_runtime_init()` sets Windows console output codepage.
  - RC Runtime: `runtime/opal_rc.c`, `runtime/opal_rc.h` - symbols declared by `src/codegen/rc_emitter.rs`.
  - Main Wrapper: `src/codegen/functions_call/tail.rs` - `emit_c_main_wrapper` named in issue.

  **Acceptance Criteria**:
  - [ ] `RUNTIME_SOURCE` includes `opal_runtime.c` exactly once and still includes `opal_rc.c` from Task 2 exactly once.
  - [ ] Generated main wrapper calls `opal_runtime_init()` before invoking Opal `main`/entrypoint.
  - [ ] Link test using RC allocation symbols no longer fails unresolved externals.
  - [ ] Wine run of Unicode-output fixture prints expected UTF-8 bytes if fixture exists; otherwise test asserts emitted wrapper contains init call/reference.
  - [ ] `cargo test --all-features --workspace` exits 0.

  **QA Scenarios**:
  ```
  Scenario: RC symbols link into generated program
    Tool: Bash
    Steps: Run the new/updated compile-link test for an Opal program that exercises reference-counted allocation.
    Expected: Link exits 0; no unresolved externals for `opal_rc_alloc`, `opal_rc_inc`, `opal_rc_dec`, `opal_rc_drop_iterative`, `opal_weak_alloc`, `opal_weak_upgrade`, or `opal_weak_dec`.
    Evidence: .sisyphus/evidence/task-8-rc-link.txt

  Scenario: Runtime init is called before entrypoint
    Tool: Bash
    Steps: Run generated wrapper/codegen test and save emitted IR/C-wrapper evidence if available.
    Expected: Evidence shows `opal_runtime_init` before user entrypoint invocation; Linux tests remain green.
    Evidence: .sisyphus/evidence/task-8-runtime-init.txt
  ```

  **Commit**: YES | Message: `fix(runtime): include init and rc sources in generated programs` | Files: [`src/compiler.rs`, `src/codegen/functions_call/tail.rs`, `runtime/opal_runtime.c`, `runtime/opal_rc.c`, relevant tests]

- [x] 9. Remove MAX_PATH-Capped Runtime Buffers

  **What to do**: Fix issue 16. Replace Windows `OPAL_PATH_BUFFER_CAP` dependence for runtime filesystem operations with dynamic allocation sized from input length and Win32 API needs. Where Win32 long-path prefixes are required, add a minimal helper that converts UTF-8 to a wide path and applies `\\?\` or UNC long-path handling without affecting relative-path semantics. Keep Linux buffer behavior safe and unchanged unless a shared helper clearly benefits both platforms.
  **Must NOT do**: Do not blindly increase `OPAL_PATH_BUFFER_CAP` and leave truncation risk. Do not add global path object abstraction. Do not make all paths absolute if the existing function expects lexical relative behavior.

  **Recommended Agent Profile**:
  - Category: `deep` - Reason: Long path handling interacts with wide API conversion, normalization, and stack buffer safety.
  - Skills: [] - No extra skills required.
  - Omitted: [`frontend-ui-ux`] - No UI work.

  **Parallelization**: Can Parallel: YES | Wave 3 | Blocks: [12] | Blocked By: [4, 5, 6]

  **References**:
  - Issue: `WINDOWS_ISSUES.md:43-44`.
  - Buffer Constant: `runtime/opal_portability.h:48-50` (`OPAL_PATH_BUFFER_CAP`).
  - Directory Pattern Buffer: `runtime/opal_portability.h:267`.
  - Absolute/CWD Buffers: `runtime/opal_portability.h:408`, `runtime/opal_portability.h:419`.
  - Runtime path creation: `runtime/opal_fs.c` functions that report `path too long` or use temp buffers.

  **Acceptance Criteria**:
  - [ ] Windows path operations with a path length > 260 bytes do not truncate and do not overflow.
  - [ ] Test writes and reads exact bytes through a nested long path under Wine/MSVC.
  - [ ] Static search confirms no Windows filesystem operation relies on fixed 260-byte path buffer for user-provided paths.
  - [ ] Linux path tests pass unchanged.
  - [ ] `cargo test --all-features --workspace` exits 0.

  **QA Scenarios**:
  ```
  Scenario: Long Windows path write/read succeeds
    Tool: Bash
    Steps: Run the Wine MSVC file ops fixture with a nested path longer than 260 bytes.
    Expected: Stdout contains `long_path_ok=true`; host-side file content equals `Hello, Opal!\n`.
    Evidence: .sisyphus/evidence/task-9-long-path-wine.txt

  Scenario: No fixed Windows MAX_PATH dependency remains
    Tool: Bash
    Steps: Search `runtime/` for `OPAL_PATH_BUFFER_CAP`, `MAX_PATH`, and `char .*\[260\]`; record remaining uses.
    Expected: Remaining uses are non-user-path constants or POSIX-only and documented; no Windows user path truncation risk.
    Evidence: .sisyphus/evidence/task-9-maxpath-search.txt
  ```

  **Commit**: YES | Message: `fix(runtime): remove Windows MAX_PATH filesystem cap` | Files: [`runtime/opal_portability.h`, `runtime/opal_fs.c`, relevant tests]

- [x] 10. Harden Cargo and CI Windows Toolchain Reproducibility

  **What to do**: Fix issues 12 and 13. Remove `llvm14-0-prefer-dynamic` from `Cargo.toml` so native Windows builds do not require an LLVM DLL on PATH and CI no longer patches the manifest with `sed`. Pin `xwin` installation in `.github/workflows/ci.yml` to an explicit version with `--locked` after verifying the version exists, update `scripts/verify-wine-prereqs.sh` to report the tested/pinned xwin expectation and Wine minimum, and remove any fragile CI workaround that edits `Cargo.toml` at runtime. Keep LLVM 14 feature `llvm14-0`.
  **Must NOT do**: Do not upgrade LLVM major version. Do not remove existing Linux/macOS build support. Do not make local Wine prerequisites mandatory for developers without the `windows-wine` feature.

  **Recommended Agent Profile**:
  - Category: `quick` - Reason: Focused manifest/CI/script changes with straightforward tests.
  - Skills: [] - No extra skills required.
  - Omitted: [`frontend-ui-ux`] - No UI work.

  **Parallelization**: Can Parallel: YES | Wave 4 | Blocks: [12] | Blocked By: [2]

  **References**:
  - Issues: `WINDOWS_ISSUES.md:48-53`.
  - Manifest: `Cargo.toml` inkwell features currently include `llvm14-0-prefer-dynamic`.
  - CI: `.github/workflows/ci.yml` cross-MSVC job installs `xwin`; Windows job has `sed` workaround per issue.
  - Make task: `Makefile.toml` `wine-tests` currently must be checked and updated to use `--features "integration windows-wine"` if it invokes `integration_e2e` Wine tests.
  - Prereq Script: `scripts/verify-wine-prereqs.sh` checks Wine, clang-cl, xwin cache, LLVM env.

  **Acceptance Criteria**:
  - [ ] `Cargo.toml` no longer contains `llvm14-0-prefer-dynamic`.
  - [ ] `.github/workflows/ci.yml` no longer contains a `sed` step modifying `Cargo.toml`.
  - [ ] `.github/workflows/ci.yml` installs `xwin` with explicit `--version <verified-version> --locked` or equivalent pinned command.
  - [ ] `Makefile.toml` Wine test task uses `cargo test --features "integration windows-wine"` for `integration_e2e` Wine tests.
  - [ ] `scripts/verify-wine-prereqs.sh` emits deterministic diagnostics for missing/old Wine and xwin cache.
  - [ ] `cargo check --workspace --all-features` exits 0.

  **QA Scenarios**:
  ```
  Scenario: Manifest no longer requests dynamic LLVM preference
    Tool: Bash
    Steps: Search `Cargo.toml` and `.github/workflows/ci.yml` for `llvm14-0-prefer-dynamic` and `sed -i` manifest patching.
    Expected: No matches for `llvm14-0-prefer-dynamic`; no CI step patches Cargo features at runtime.
    Evidence: .sisyphus/evidence/task-10-cargo-ci-search.txt

  Scenario: Wine prerequisites report reproducibly
    Tool: Bash
    Steps: Run `bash scripts/verify-wine-prereqs.sh 2>&1 | tee .sisyphus/evidence/task-10-wine-prereqs.txt`.
    Expected: In configured env, output starts with `OK:` and includes Wine major >= 8 and xwin cache path; otherwise output starts with `SKIP:` and names the missing prerequisite.
    Evidence: .sisyphus/evidence/task-10-wine-prereqs.txt
  ```

  **Commit**: YES | Message: `chore(ci): pin Windows toolchain prerequisites` | Files: [`Cargo.toml`, `.github/workflows/ci.yml`, `scripts/verify-wine-prereqs.sh`]

- [x] 11. Verify or Complete Windows DLL Hot-Reload Copy-Before-Load

  **What to do**: Resolve issue 14 in `src/hot_reload/loader.rs`. Exploration found copy-before-load logic already present (`temp_copy_path_for`, `load_library` copies to a temp file before `Library::new`). The executor must first add/inspect tests proving Windows copy-before-load semantics, unique temp names, cleanup on unload, and reload while original DLL path remains replaceable. If tests prove the implementation already satisfies issue 14, mark it closed with regression tests only. If gaps exist, implement the smallest fix.
  **Must NOT do**: Do not redesign hot reload architecture. Do not require native Windows. Do not make Wine DLL-lock behavior a perfect proxy for native Windows; document caveat in test comments.

  **Recommended Agent Profile**:
  - Category: `unspecified-high` - Reason: May be verification-only or a focused implementation depending on current test gap.
  - Skills: [] - No extra skills required.
  - Omitted: [`playwright`] - No browser/UI.

  **Parallelization**: Can Parallel: YES | Wave 4 | Blocks: [12] | Blocked By: [4, 6]

  **References**:
  - Issue: `WINDOWS_ISSUES.md:68-69`.
  - Loader: `src/hot_reload/loader.rs:101-171` - `FilesystemModuleLoader`, `temp_copy_path_for`, `load_library`, `fs::copy`, `Library::new(&copy_path)`.
  - Loader cleanup: `src/hot_reload/loader.rs:176-203` - loaded path maps and unload.
  - Hot swap: `src/hot_reload/loader.rs:245-268` - `hot_swap_module` orchestration.

  **Acceptance Criteria**:
  - [ ] Test asserts `FilesystemModuleLoader::load_library` loads from a unique temp copy, not original DLL path.
  - [ ] Test asserts repeated loads use distinct temp names.
  - [ ] Test asserts unload removes the temp copy where platform permits.
  - [ ] Test asserts original module path can be overwritten/replaced after load because loader holds the temp copy, or records an explicit Wine limitation if environment cannot prove native DLL-lock behavior.
  - [ ] If implementation was already correct, commit is still a test/regression commit that references issue 14.
  - [ ] `cargo test --all-features --workspace` exits 0.

  **QA Scenarios**:
  ```
  Scenario: Hot reload loads copied DLL path
    Tool: Bash
    Steps: Run hot_reload loader tests with `--nocapture` and save output.
    Expected: Test observes loaded path is in temp directory and differs from original module path; repeated reloads create unique paths.
    Evidence: .sisyphus/evidence/task-11-hot-reload-copy.txt

  Scenario: Wine DLL-lock caveat documented
    Tool: Bash
    Steps: Run `cargo test --all-features --workspace hot_reload 2>&1 | tee .sisyphus/evidence/task-11-hot-reload-tests.txt`.
    Expected: Tests pass; any Wine-specific ignore includes reason `Wine limitation: DLL lock behavior differs from native Windows`.
    Evidence: .sisyphus/evidence/task-11-hot-reload-tests.txt
  ```

  **Commit**: YES | Message: `test(hot_reload): lock Windows DLL copy-before-load behavior` | Files: [`src/hot_reload/loader.rs`, relevant tests]

- [x] 12. Close `WINDOWS_ISSUES.md` and Produce Final Evidence Bundle

  **What to do**: After Tasks 1-11 are complete, update `WINDOWS_ISSUES.md` checkboxes for every resolved issue and add concise resolution notes if the file style permits. Run the full verification matrix, collect evidence under `.sisyphus/evidence/windows-issues-final/`, and ensure commit history is atomic by reviewing `git log --oneline` for unit-of-work boundaries. Include Wine caveats for symlink and DLL-lock behavior in test comments/evidence, not as blocking manual steps.
  **Must NOT do**: Do not mark issues complete without a test or evidence note. Do not add new scope. Do not skip final Linux regression because Wine passed.

  **Recommended Agent Profile**:
  - Category: `deep` - Reason: Final integration gate, checklist closure, and evidence consolidation.
  - Skills: [] - No extra skills required.
  - Omitted: [`frontend-ui-ux`] - No UI work.

  **Parallelization**: Can Parallel: NO | Wave 5 | Blocks: [F1, F2, F3, F4] | Blocked By: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11]

  **References**:
  - Checklist: `WINDOWS_ISSUES.md` all unchecked items.
  - Test Commands: README Testing section, `.github/workflows/ci.yml`, `Makefile.toml` `wine-tests` task, `scripts/verify-wine-prereqs.sh`.
  - Evidence Standard: `.sisyphus/evidence/` existing project evidence files.

  **Acceptance Criteria**:
  - [ ] Every issue in `WINDOWS_ISSUES.md` listed in this plan is checked or has a resolution note explaining already-implemented behavior plus regression test.
  - [ ] `cargo test --all-features --workspace` exits 0 and output saved to `.sisyphus/evidence/windows-issues-final/linux-tests.txt`.
  - [ ] `cargo clippy --all-targets --all-features -- -D warnings` exits 0 and output saved.
  - [ ] `cargo fmt --all -- --check` exits 0 and output saved.
  - [ ] `bash scripts/verify-wine-prereqs.sh` output saved.
  - [ ] In configured environment, `cargo test --features "integration windows-wine" --test integration_e2e -- --nocapture wine_msvc_file_ops` exits 0 and output saved.
  - [ ] `cargo run --release -- test-projects/hello-world/src/main.op --target x86_64-pc-windows-msvc` produces `.exe`; `wine <exe>` exits 0 and output is saved.
  - [ ] MinGW non-regression compile/link smoke is attempted if toolchain exists; if absent, evidence records structured skip from prereq detection.

  **QA Scenarios**:
  ```
  Scenario: Full Linux regression and quality gate
    Tool: Bash
    Steps: Run `cargo test --all-features --workspace`, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo fmt --all -- --check`; save outputs under `.sisyphus/evidence/windows-issues-final/`.
    Expected: All exit 0.
    Evidence: .sisyphus/evidence/windows-issues-final/linux-tests.txt

  Scenario: Final Wine MSVC executable performs file operations
    Tool: Bash
    Steps: Run `bash scripts/verify-wine-prereqs.sh`, then `cargo test --features "integration windows-wine" --test integration_e2e -- --nocapture wine_msvc_file_ops`; save stdout/stderr/exit artifacts.
    Expected: Exit code 0; stdout contains exact lines `readback_ok=true`, `unicode_path_ok=true`, `long_path_ok=true`, and `opendir_missing_errno=ENOENT`; stderr contains no `panic` or `error:`.
    Evidence: .sisyphus/evidence/windows-issues-final/wine-msvc-file-ops.txt
  ```

  **Commit**: YES | Message: `chore(windows): close filesystem issue checklist` | Files: [`WINDOWS_ISSUES.md`, `.sisyphus/evidence/windows-issues-final/**` if evidence is tracked by project convention]

## Final Verification Wave (MANDATORY — after ALL implementation tasks)
> 4 review agents run in PARALLEL. ALL must APPROVE. Present consolidated results to user and get explicit "okay" before completing.
> **Do NOT auto-proceed after verification. Wait for user's explicit approval before marking work complete.**
> **Never mark F1-F4 as checked before getting user's okay.** Rejection or user feedback -> fix -> re-run -> present again -> wait for okay.
- [x] F1. Plan Compliance Audit — oracle
- [x] F2. Code Quality Review — unspecified-high
- [x] F3. Real Manual QA — unspecified-high (+ playwright if UI)
- [x] F4. Scope Fidelity Check — deep

## Commit Strategy
- Use atomic commits at full units of work.
- Prefer one commit per `WINDOWS_ISSUES.md` checkbox.
- Group checkboxes only when a single function rewrite makes separation artificial; mention every issue number in the commit body.
- Do not commit red-only tests that leave main broken unless the repository workflow explicitly permits a temporary red commit. Default: each commit must include red test plus green implementation and pass its acceptance commands.
- Commit messages use conventional style and issue reference, for example `fix(runtime): handle Windows path separators in fs helpers`.

## Success Criteria
- All unchecked items in `WINDOWS_ISSUES.md` are either implemented and tested or documented as already implemented with a regression test.
- Linux regression suite remains green.
- Wine MSVC file-operation fixture passes with exact stdout and host-side file checks.
- CI no longer relies on mutable `xwin` latest installs or `sed` patching of `Cargo.toml` for LLVM dynamic feature removal.
- Final verification agents approve and user gives explicit okay.
