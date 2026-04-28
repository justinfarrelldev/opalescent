# fs Test Projects on Linux (MSVC-Portable)

> **SSOT**: This plan supersedes `.sisyphus/plans/file-io-stdlib-path-object-centric.md`
> for all fs work going forward. **DO NOT EDIT THE OLD PLAN**. The old plan
> is retained for historical reference only. All fs runtime, codegen audit,
> test-project, and MSVC-verification work must proceed from THIS plan.
>
> **Old plan banner action (human-only)**: Insert the following at the top of
> `.sisyphus/plans/file-io-stdlib-path-object-centric.md` manually:
> `> SUPERSEDED by .sisyphus/plans/fs-test-projects-linux.md — do not edit.`
> Sisyphus must not modify the old plan.

## TL;DR

> **Quick Summary**: Make all 34 existing fs builtins functional on Linux end-to-end + add 1 new builtin (`read_first_line_sync`) via
> 20 realistic test projects (3 high-level + 17 granular `_fs_*`), validated
> by Rust integration tests with `FsStateGuard` RAII cleanup, with strict MSVC
> portability (clang-cl + xwin compile-AND-link gate per task). Per-scenario
> red-green-refactor mandatory.
>
> **Deliverables**:
> - `runtime/opal_fs.c`: replace 27 stubs with real POSIX impls; fix 3 silent-bug path helpers
> - `runtime/opal_fs_errors.h` (NEW): stable error-discriminant SSOT
> - `runtime/opal_portability.h`: extend with Windows shims (dirent, realpath, stat, mkdir, etc.)
> - `runtime/opal_msvc_link_probe.c` (NEW): MSVC link-gate harness
> - `tests/integration_e2e/fs_state_guard.rs` (NEW): `FsStateGuard` RAII implementation
> - `tests/integration_e2e/fs_*.rs`: 20 integration tests (one per project)
> - `test-projects/fs-directory-operations/`, `test-projects/fs-path-manipulation/`, `test-projects/fs-markdown-roundtrip/`: populated
> - `test-projects/_fs_*` (17 projects): populated with realistic mini-apps
> - `stdlib/prelude.op`: add fs section (doc-only, ≤ 50 lines)
> - `.gitattributes` (root + per-project): CRLF/LF preservation for fixtures
> - `Cargo.toml`: add `serial_test = "3"` dev-dependency
>
> **Estimated Effort**: XL (40+ tasks, multi-week scope)
> **Parallel Execution**: YES — 8 waves
> **Critical Path**: T1 (codegen audit) → T2 (error contract) → T3 (portability extend) → T4 (FsStateGuard) → T5 (smoke project `_fs_path_from`) → Wave 3 (path projects) → Wave 4-7 (runtime families + tests) → Wave 8 (high-level) → Final review

---

## Context

### Original Request
> "Your goal is to make sure fs operations work in Opalescent properly on Linux.
> Read the Sisyphus plan for Windows support for architectural decisions to use going
> forward — create these implementations with Windows support via MSVC in mind.
> First step: design all of the test projects for fs — start with creating
> fs-directory-operations and compile that for Linux using `opal run` to make sure
> it runs and works on Linux. Make sure all of these test projects are re-runnable
> with the test suite, and make sure they all clean up after themselves to a
> completely-clean blank slate. Once fs-directory-operations runs, move on to
> fs-path-manipulation and then all of the _fs ones. Use red-green-refactor —
> do not skip any steps from it."
>
> Subsequent clarifications:
> - Cover ALL fs-* and `_fs_*` in ONE plan.
> - "Research stdlib status harder" — it does exist.
> - Per-function/per-scenario fine-grained RGR.
> - Explicit MSVC verification tasks.
> - Old file-io plan is "likely out of date — do a research wave, do not edit
>   it, new plan is SSOT."
> - Rust `StateGuard` RAII cleanup.
> - Granular `_fs_*` = realistic mini-apps with multi-file src/ and subfolders,
>   but ONE major thing each.

### Interview Summary

**Key Decisions** (from user clarifications):
- **Scope coverage**: ALL fs-* and `_fs_*` in ONE plan (no splits).
- **Old plan handling**: Do not edit; new plan is SSOT.
- **TDD cadence**: Per-scenario RGR; no skipped steps.
- **Cleanup**: Rust RAII guard with snapshot+restore; full re-runnability.
- **Project realism**: Multi-file `src/`, subfolders allowed; 1 major thing each.
- **MSVC**: Verification only (clang-cl + xwin); no Wine execution required.

### Research Findings (from explore agents + Metis review)

**Wiring is COMPLETE for all 34 existing fs builtins; this plan adds 1 new builtin (`read_first_line_sync`) end-to-end** (corrected understanding):
- All 7 compiler touchpoints wired: `STDLIB_NAMES`, `declare_stdlib_function`,
  `statements.rs` known_runtime/guard types, `module_resolver.rs` exports,
  `compiler.rs` RUNTIME_SOURCE, `type_system/checker/fs_builtins.rs`
  (registers types AND signatures), `runtime/opal_fs.c` C entry points.
- Implication: **NO stdlib gap-fill needed**. Focus shifts to runtime impl.

**`runtime/opal_fs.c` is mostly STUBS or SILENT BUGS** (Metis correction):
- ✅ `absolute_path_sync` — REAL (uses `realpath`).
- ⚠️ `path_from` — IDENTITY (returns `strdup(path)`); silent bug. Runtime symbol: `char* path_from(const char* raw)`.
- ⚠️ `normalize_path` — IDENTITY; silent bug. Runtime symbol: `char* normalize_path(const char* path)`.
- ⚠️ `join_path_components` — does not normalize separators or handle
  absolute components correctly; silent bug.
- ✅ `fs_path_file_extension`, `fs_path_file_name`, `fs_path_parent_directory`
  — REAL (string manipulation).
- ❌ ~27 I/O functions return `"not implemented"` stubs.
- ❌ Permission builtins do NOT exist (out of scope; deferred).

**`runtime/opal_portability.h` ALREADY EXISTS** (Metis correction):
- Provides `OPAL_WINDOWS`, `OPAL_MSVC`, `OPAL_POSIX`, `OPAL_HAS_DIRENT` macros.
- Forward-declares `opal_opendir/readdir/closedir` (Windows impls NOT YET present).
- Plan must EXTEND it — not create.

**Test infrastructure**:
- `tests/integration_e2e.rs:15-28`: `prepare_dir` / `cleanup_dir` helpers exist.
- NO `StateGuard` exists — must be implemented.
- `[features] integration = []` already in `Cargo.toml`.
- Compile path: `compile_program(source, &target_dir)` (in-process, fast).
- Pattern reference: `tests/integration_e2e.rs`, `tests/integration_print.rs`.

**MSVC architecture** (from `.sisyphus/plans/windows-support.md`):
- D4: `runtime/opal_portability.h` is SSOT for cross-platform shims;
  raw `_WIN32`/`_MSC_VER` allowed only there.
- Static LLVM linking; `XWIN_CACHE` env var (default `~/.xwin`).
- Regression Gate per task: `cargo test --all-features` Linux green.
- Long-path (>260 chars): explicitly DEFERRED.
- CRLF normalization in test harness (mandatory for read_lines fixtures).
- Cross-compile: `clang-cl` + `lld-link` via xwin sysroot.

### Metis Review

**Critical findings addressed**:
- Wave reorder: validate harness on cheap path-only surface BEFORE I/O stubs.
- Error string contract: SSOT in new `runtime/opal_fs_errors.h`.
- MSVC verification: must include link probe, not just compile.
- `.gitattributes` strategy mandatory for CRLF fixture stability.
- `serial_test` crate for fs test isolation (not global `--test-threads=1`).
- `FsStateGuard` Drop must check `std::thread::panicking()` to avoid double-panic abort.
- Decide & document `read_lines_sync` trailing-newline policy before Wave 4.
- `list_directory_sync` ordering policy: sort at runtime + document.
- Codegen audit task (T1) must freeze success-sentinel contract before stub replacement.
- Pre-existing path bugs (`normalize_path`, `path_from`, `join_path_components`)
  fixed in Wave 3, not silently kept.

---

## Work Objectives

### Core Objective
Replace 27 fs runtime stubs and 3 silent-bug helpers with real POSIX implementations,
fronted by 20 realistic test projects exercising every fs builtin via per-scenario RGR,
with mandatory MSVC compile-AND-link verification per task.

### Concrete Deliverables
- `runtime/opal_fs.c`: 27 stubs → real impls; 3 silent bugs fixed.
- `runtime/opal_fs_errors.h`: stable error-discriminant macros (`OPAL_FS_ERR_NOT_FOUND`, etc.).
- `runtime/opal_portability.h`: Windows shims for dirent/realpath/stat/mkdir/etc.
- `runtime/opal_msvc_link_probe.c`: 3-line MSVC link-gate harness.
- `tests/integration_e2e/fs_state_guard.rs`: `FsStateGuard` RAII type.
- `tests/integration_e2e/fs_*.rs`: 20 integration tests (one per project), `#[serial(fs)]`.
- 20 test-project directories populated (3 fs-* + 17 _fs_*).
- `stdlib/prelude.op`: fs documentation section.
- `.gitattributes` (root + per-project) for CRLF fixture preservation.
- `Cargo.toml`: `serial_test = "3"` dev-dep.

### Definition of Done
- [ ] `cargo test --features integration fs_` → 20+ tests pass, 0 fail.
- [ ] `cargo test --all-features` Linux → green (Regression Gate).
- [ ] `clang-cl /c runtime/opal_fs.c` (xwin sysroot) → exit 0.
- [ ] `lld-link runtime/opal_fs.obj runtime/opal_msvc_link_probe.obj …` → exit 0.
- [ ] `git check-attr text -- test-projects/_fs_read_lines_crlf/tests/fixtures/crlf.txt` → `text: unset`.
- [ ] All 20 projects re-runnable: 2 consecutive `cargo test` invocations both pass with no manual reset.
- [ ] All 20 projects have `opal.toml`, `.gitignore`, `README.md` (≥30 chars), `src/main.op` with `entry main`.
- [ ] `ast_grep_search` for `r.error = "not implemented"` in `runtime/opal_fs.c` → 0 matches.
- [ ] `FsStateGuard` Drop verified: sha256 manifest unchanged after every test.

### Must Have
- 20 test projects (3 fs-* + 17 _fs_*) — all populated, all passing.
- Per-scenario RGR commits (red/green/refactor prefix in git log).
- Per-task MSVC compile-AND-link verification.
- `FsStateGuard` RAII with sha256 snapshot+restore + panicking guard.
- `runtime/opal_fs_errors.h` SSOT for error discriminants.
- `.gitattributes` `-text` for all fixture files.
- `serial_test` crate gating fs tests under `#[serial(fs)]`.
- All consuming runtime tasks blocked on `T-portability-extend`.
- Codegen lowering audit (T1) BEFORE any stub replacement.

### Must NOT Have (Guardrails)
- **NO** edits to `.sisyphus/plans/file-io-stdlib-path-object-centric.md`.
- **NO** new fs builtins — only fill bodies of existing 27 stubs + fix 3 silent bugs.
- **NO** permissions scenarios (`read_permissions`, `set_*`) — DEFERRED.
- **NO** long-path (>260 char) scenarios — DEFERRED.
- **NO** hot-reload, network, or async fs work.
- **NO** `tempfile::TempDir` for fs project workspaces (defeats FsStateGuard semantics).
- **NO** `opal_io`/`opal_parse`/RNG/time builtins in `_fs_*` projects (non-determinism).
- **NO** scope creep into other stdlibs.
- **NO** projects exceeding LoC caps (`_fs_*` ≤ 150 LoC `.op`; `fs-*` ≤ 400 LoC).
- **NO** placeholders like `[expected output]` in test assertions — concrete values only.
- **NO** criteria requiring "user verifies" / "manually inspect".
- **NO** raw `_WIN32`/`_MSC_VER` outside `runtime/opal_portability.h`.
- **NO** global `--test-threads=1` (use `#[serial(fs)]`).

---

## Verification Strategy

> **ZERO HUMAN INTERVENTION** — ALL verification is agent-executed.

### Test Decision
- **Infrastructure exists**: YES (cargo test + `integration` feature).
- **Automated tests**: YES — TDD red-green-refactor per scenario.
- **Framework**: Rust `#[test]` + `cargo test --features integration`, gated `#[serial(fs)]`.
- **Each task follows**: RED (failing scenario) → GREEN (minimum impl) → REFACTOR.

### Per-Task Regression Gate (MANDATORY, every task)

**Phased gate**: MSVC compile gate (step 3) becomes runnable from **T3 onwards** (T3 atomically lands `opal_realpath` in `runtime/opal_portability.h` AND swaps the single `realpath()` call site in `opal_fs.c:114`). MSVC link gate (step 4) becomes runnable from **T5 onwards** (T5 creates the `runtime/opal_msvc_link_probe.c` harness + `scripts/msvc_link_probe.sh` script). Phasing per task:

- **T1, T2**: steps 1-2 ONLY (MSVC gates impossible — `opal_fs.c:114` still calls raw `realpath()`).
- **T3**: steps 1-2 + 3 (T3 itself unblocks step 3 via its 2-line `opal_fs.c` swap; T3 QA verifies clang-cl of both `opal_portability.h` probe AND `opal_fs.c` directly).
- **T4**: steps 1-2 + 3 (link probe artifact not yet created).
- **T5**: steps 1-2 + 3 + 4 (T5 creates link probe; from here on link gate is mandatory).
- **T6, T7**: steps 1-2 + 3 + 4 (no project files touched yet, so step 5 N/A).
- **T8+**: ALL FIVE steps (FsStateGuard introduced in T4 + project files start in T8).

```bash
# 1. Linux full regression — ALL TASKS
cargo test --all-features 2>&1 | tail -5 | grep "test result: ok"

# 2. Family-scoped fs tests — ALL TASKS
cargo test --features integration fs_ 2>&1 | grep -E "test result: ok\. [0-9]+ passed"

# 3. MSVC compile gate (xwin sysroot) — T3+ MANDATORY
# (Pre-T3 the gate is impossible: opal_fs.c:114 uses raw realpath() not available under MSVC.
#  T3 atomically introduces opal_realpath AND swaps the call site. From T3 onward this gate is mandatory.)
clang-cl /nologo /c \
  /imsvc $XWIN_CACHE/crt/include \
  /imsvc $XWIN_CACHE/sdk/include/ucrt \
  /imsvc $XWIN_CACHE/sdk/include/um \
  /imsvc $XWIN_CACHE/sdk/include/shared \
  runtime/opal_fs.c -Foruntime/opal_fs.obj
# Expected: exit 0

# 4. MSVC link gate — T5+ MANDATORY (T5 creates the link-probe artifact; T3-T4 only run step 3)
lld-link /nologo runtime/opal_fs.obj runtime/opal_msvc_link_probe.obj \
  /OUT:/tmp/probe.exe /SUBSYSTEM:CONSOLE \
  /DEFAULTLIB:msvcrt /DEFAULTLIB:kernel32 \
  /ENTRY:opal_msvc_link_probe
# Expected: exit 0

# 5. FsStateGuard manifest verification (post-test) — ALL TASKS THAT TOUCH PROJECT FILES (T8+)
sha256sum -c .sisyphus/evidence/task-{N}-state-manifest.sha256
# Expected: exit 0, all "OK"
```

**MSVC gate phasing rationale**: `runtime/opal_fs.c:114` (pre-T3) calls POSIX `realpath(path, NULL)` which doesn't exist under MSVC/clang-cl. Forcing the MSVC gate on T1/T2 is impossible. T3 atomically (a) introduces `opal_realpath` in `runtime/opal_portability.h` with a unified POSIX/Windows behavior (POSIX `ENOENT` → lexical fallback to match `_fullpath` semantics) AND (b) swaps the single call site in `opal_fs.c:114` to `opal_realpath` — a strict 2-line edit. From T3 onward, `clang-cl /c runtime/opal_fs.c` is achievable and is mandatory on every subsequent task. T5 creates `runtime/opal_msvc_link_probe.c` + `scripts/msvc_link_probe.sh`, unlocking the link gate from T5 onwards. Linux behavior is preserved: existing-path semantics unchanged; non-existent-path semantics shift from "InvalidPathError" to "lexical absolute path success" (this divergence is documented in T14 as the unified contract). T34 is the final full-runtime MSVC verification gate.

### QA Policy
Every task includes agent-executed QA scenarios. Evidence saved to
`.sisyphus/evidence/task-{N}-{scenario-slug}.{ext}`.

- **Library/Module (Rust tests)**: `bash` running `cargo test`.
- **CLI (`opal run`)**: `bash` running compiled binary, capturing stdout/exit code.
- **Runtime C (MSVC compile/link)**: `bash` running clang-cl + lld-link.
- **State integrity (FsStateGuard)**: `bash` running `sha256sum -c manifest`.

---

## Execution Strategy

### Parallel Execution Waves

```
Wave 1 (Foundations — sequential within wave):
├── T1: Codegen lowering audit + success-sentinel contract freeze [deep]
├── T2: Error-discriminant SSOT (runtime/opal_fs_errors.h) [quick]
├── T3: Portability extend (runtime/opal_portability.h Windows shims) [unspecified-high]
├── T4: FsStateGuard RAII implementation [deep]
├── T5: MSVC link-probe harness (runtime/opal_msvc_link_probe.c + scripts/msvc_link_probe.sh) [quick]
├── T6: .gitattributes + fs_helpers module wiring [quick]
└── T7: serial_test dev-dep + prelude ## fs section [quick]

Wave 2 (Smoke Test — single project, validates whole pipeline):
└── T8: _fs_path_from smoke project (RED-GREEN-REFACTOR) [unspecified-high]

Wave 3 (Path-only projects — PARALLEL, fix silent bugs in this wave):
├── T9:  _fs_normalize_path (FIX silent bug) [unspecified-high]
├── T10: _fs_join_path_components (FIX silent bug) [unspecified-high]
├── T11: _fs_path_file_extension [quick]
├── T12: _fs_path_file_name [quick]
├── T13: _fs_path_parent_directory [quick]
└── T14: _absolute_path_sync [quick]

Wave 4 (Read family runtime + projects):
├── T15: Read family runtime impl (read_text, read_lines, read_contents, read_bytes_at_offset) [deep]
├── T16: _fs_read_text_happy [unspecified-high]
├── T17: _fs_read_text_invalid_utf8 [unspecified-high]
├── T18: _fs_read_contents_happy [unspecified-high]
├── T19: _fs_read_contents_is_dir [unspecified-high]
├── T20: _fs_read_contents_not_found [unspecified-high]
├── T21: _fs_read_lines_lf [unspecified-high]
├── T22: _fs_read_lines_crlf [unspecified-high]
├── T23: _fs_read_lines_mixed [unspecified-high]
├── T24: _fs_read_offset_happy [unspecified-high]
└── T25: _fs_read_offset_oob [unspecified-high]

Wave 5 (Write/append family runtime — no _fs_* projects but wired into high-level):
└── T26: Write family runtime impl (write_*, append_*, write_atomic_*, write_bytes_at_offset) [deep]

Wave 6 (File ops + Directory + Metadata families runtime — PARALLEL):
├── T27: File ops family (create_file, delete_file, copy_file, move_path) [deep]
├── T28: Directory family (create_dir, create_dir_recursive, delete_dir, delete_dir_recursive, list_directory) [deep]
└── T29: Metadata family (exists, is_file, is_dir, metadata) [unspecified-high]

Wave 7 (High-level fs-* composition projects — PARALLEL):
├── T30: fs-directory-operations [unspecified-high]
├── T31: fs-path-manipulation [unspecified-high]
└── T32: fs-markdown-roundtrip [unspecified-high]

Wave 8 (Re-runnability + MSVC consolidation):
├── T33: Re-runnability gauntlet (run `cargo test --features integration fs_` twice; assert green both) [quick]
└── T34: MSVC full-suite verification (clang-cl + lld-link all runtime/*.c) [unspecified-high]

Wave FINAL (Review — 4 parallel, then user okay):
├── F1: Plan compliance audit (oracle)
├── F2: Code quality review (unspecified-high)
├── F3: Real manual QA (unspecified-high + playwright N/A — bash-only)
└── F4: Scope fidelity check (deep)
→ Present results → Get explicit user okay

Critical Path: T1 → T2 → T3 → T4 → T5 → T6 → T8 → T15 → T26 → T27 → T28 → T29 → T30 → T33 → F1-F4 → user okay
Parallel Speedup: ~60% faster than sequential
Max Concurrent: 10 (Wave 4 read-family projects)
```

### Dependency Matrix

| Task | Depends On | Blocks | Wave |
|------|------------|--------|------|
| T1   | none       | T2, T15, T26-T29 | 1 |
| T2   | T1         | T15, T26-T29 | 1 |
| T3   | none (parallel with T1, T2) | T8-T34 | 1 |
| T4   | T3         | T8-T32 | 1 |
| T5   | T2, T3     | T8-T32 | 1 |
| T6   | T3         | T8 (link gate) | 1 |
| T7   | T1         | none (doc-only) | 1 |
| T8   | T1-T6      | T9-T14 (validates pipeline) | 2 |
| T9-T14 | T8       | T30, T31 | 3 |
| T15  | T1, T2, T3 | T16-T25 | 4 |
| T16-T25 | T15     | T30, T32 | 4 |
| T26  | T1, T2, T3 | T27, T30-T32 | 5 |
| T27  | T26        | T30 | 6 |
| T28  | T26        | T30 | 6 |
| T29  | T26        | T30-T32 | 6 |
| T30  | T9-T14, T26-T29 | T33 | 7 |
| T31  | T9-T14     | T33 | 7 |
| T32  | T15-T25, T26 | T33 | 7 |
| T33  | T30-T32    | F1-F4 | 8 |
| T34  | all        | F1-F4 | 8 |
| F1-F4 | T33, T34  | user okay | FINAL |

### Agent Dispatch Summary

| Wave | Tasks | Agents |
|------|-------|--------|
| 1 | 7  | T1→`deep`, T2→`quick`, T3→`unspecified-high`, T4→`deep`, T5→`quick`, T6→`quick`, T7→`writing` |
| 2 | 1  | T8→`unspecified-high` |
| 3 | 6  | T9-T10→`unspecified-high`, T11-T14→`quick` |
| 4 | 11 | T15→`deep`, T16-T25→`unspecified-high` |
| 5 | 1  | T26→`deep` |
| 6 | 3  | T27-T28→`deep`, T29→`unspecified-high` |
| 7 | 3  | T30-T32→`unspecified-high` |
| 8 | 2  | T33→`quick`, T34→`unspecified-high` |
| FINAL | 4 | F1→`oracle`, F2→`unspecified-high`, F3→`unspecified-high`, F4→`deep` |

---

## TODOs

> Implementation + Test = ONE Task. Never separate.
> EVERY task MUST have: Recommended Agent Profile + Parallelization info + QA Scenarios.

> ### GLOBAL WIRING CONTRACT (applies to ALL tasks)
>
> **Rust integration test module declarations live in `tests/integration_e2e/tests.rs`, NOT in `tests/integration_e2e.rs`.**
>
> The actual repo layout is:
> - `tests/integration_e2e.rs:31-32` contains: `#[path = "integration_e2e/tests.rs"] mod tests;` — this is the ONLY `mod` line in that file.
> - `tests/integration_e2e/tests.rs:5-19` is the module-tree root that declares siblings: `mod bytes_stdlib;`, `mod compile_failures;`, `mod interactive_io;`, `mod project_execution;`, etc. It also has commented-out stubs: `// mod fs_path_manipulation;`, `// mod fs_markdown_roundtrip;`, `// mod fs_management;`, `// mod fs_reading;`, `// mod fs_writing;`, `// mod fs_directory;`, `// mod fs_permissions;`, `// mod fs_directory_operations;`.
>
> **Rule for every task that creates `tests/integration_e2e/<name>.rs`:**
> 1. Add `mod <name>;` (or uncomment the existing stub) inside `tests/integration_e2e/tests.rs` — NEVER touch `tests/integration_e2e.rs`.
> 2. For shared helper modules (`fs_helpers`, `fs_state_guard`), use `pub(crate) mod <name>;` so test modules can `use super::<name>::*;`.
> 3. The `super::*;` import at the top of `tests/integration_e2e/tests.rs` already brings `prepare_dir`, `cleanup_dir`, etc. into scope for sibling modules — no need to re-export.
>
> **Anti-pattern (will cause compile errors):** Adding `pub mod fs_xyz;` inside `tests/integration_e2e.rs` — this creates a duplicate module path because `tests.rs` will also try to declare it, and modules under `#[path]`-mounted parents resolve via the parent's tree, not the file's directory.
>
> If a task description says "Wire `pub mod ...;` into `tests/integration_e2e.rs`" treat that as shorthand for "add `mod ...;` to `tests/integration_e2e/tests.rs`" per this contract.

- [x] 1. **T1: Codegen Lowering Audit + Success-Sentinel Contract**

  **What to do**:
  - Read `src/codegen/functions_stdlib.rs` and `src/codegen/statements.rs` to document the lowering of all 8 fs result struct types (`FsPathResult`, `FsBytesResult`, `FsStringResult`, `FsStringArrayResult`, `FsVoidResult`, `FsBooleanResult`, `FsMetadataResult`, `FsPathArrayResult`).
  - For each struct, document the EXACT success sentinel: is success determined by `error == NULL`, by `value != NULL`, or both? Specifically clarify `FsVoidResult` (where `value` is unused).
  - Freeze the contract by adding a doc-comment block at the top of `runtime/opal_runtime.h` (above the FsResult typedefs) stating the success/failure sentinel rules.
  - Verify codegen for `propagate read_text(path)` and `guard read_text(path) into x else {…}` lowers consistently with the documented sentinel.
  - No behavioral changes — documentation + verification only.

  **Must NOT do**:
  - Modify any FsResult struct layouts.
  - Change codegen lowering.
  - Touch other stdlib types.

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: Requires reading codegen + statements.rs + runtime headers and synthesizing a contract. Wrong contract = downstream rework risk for 27 stub replacements.
  - **Skills**: `[]`
    - No skill overlap; pure code investigation + documentation.

  **Parallelization**:
  - **Can Run In Parallel**: NO (foundation task, blocks T2/T15/T26-T29)
  - **Parallel Group**: Wave 1 sequential
  - **Blocks**: T2, T15, T26-T29
  - **Blocked By**: None

  **References**:
  - `src/codegen/functions_stdlib.rs` (full file) — FFI struct definitions and declare_stdlib_function arms.
  - `src/codegen/statements.rs:known_runtime_return_type, known_guard_success_type` — guard/propagate lowering.
  - `runtime/opal_runtime.h:84-94` — FsResult struct typedefs (success-sentinel target location; covers FsVoidResult through FsPermissionsResult).
  - `runtime/opal_fs.c:1-50` — examples of how stubs return `{value: NULL, error: "not implemented"}`.

  **WHY**:
  - Without a frozen sentinel contract, every stub replacement will guess and produce subtly different success returns, causing flaky `propagate`/`guard` behavior.

  **Acceptance Criteria**:
  - [ ] Doc block added to `runtime/opal_runtime.h` describing success-sentinel rule per struct (≥ 200 chars).
  - [ ] `cargo test --all-features` → green (Regression Gate steps 1-2).
  - [ ] MSVC compile/link gate DEFERRED to T3 (see "Per-Task Regression Gate" phasing note: `opal_fs.c:114` uses raw `realpath()` which can't compile under MSVC until T3 atomically introduces `opal_realpath` AND swaps the call site in a single 2-line edit).

  **QA Scenarios**:
  ```
  Scenario: Sentinel contract documented and accurate
    Tool: Bash
    Preconditions: Plan T1 complete
    Steps:
      1. Run: grep -A 30 "Success sentinel" runtime/opal_runtime.h
      2. Assert: output mentions FsVoidResult, FsBytesResult, FsStringResult, FsStringArrayResult, FsPathResult, FsBooleanResult, FsMetadataResult, FsPathArrayResult.
      3. Assert: output explicitly states "error == NULL means success" (or equivalent).
    Expected Result: All 8 struct types documented with sentinel rule.
    Failure Indicators: missing struct, ambiguous wording.
    Evidence: .sisyphus/evidence/task-1-sentinel-doc.txt

  Scenario: Regression gate green
    Tool: Bash
    Preconditions: Plan T1 complete, no source changes outside runtime/opal_runtime.h
    Steps:
      1. Run: cargo test --all-features 2>&1 | tee /tmp/t1-tests.log
      2. Assert: exit 0
      3. Assert: log contains "test result: ok"
    Expected Result: All existing tests pass.
    Evidence: .sisyphus/evidence/task-1-regression.log
  ```

  **Evidence to Capture**:
  - [ ] `.sisyphus/evidence/task-1-sentinel-doc.txt`
  - [ ] `.sisyphus/evidence/task-1-regression.log`
  - [ ] (MSVC compile/link evidence DEFERRED to T3 — see Per-Task Regression Gate phasing note)

  **Commit**: YES
  - Message: `chore(runtime): document fs result success sentinel`
  - Files: `runtime/opal_runtime.h`
  - Pre-commit: `cargo test --all-features` green

- [x] 2. **T2: Error-Discriminant SSOT (`runtime/opal_fs_errors.h`)**

  **What to do**:
  - Create new file `runtime/opal_fs_errors.h` defining stable error-discriminant macros:
> **CRITICAL CONTRACT**: Discriminant string values MUST EXACTLY MATCH the nominal error type names registered in `src/type_system/checker/fs_builtins.rs:29-48`. The runtime emits errors as `"<NominalName>: <message>"` and the language matches errors by nominal-name prefix. Any deviation breaks `match err { FileNotFoundError => ... }` semantics. Verified via `grep '"[A-Z][a-zA-Z]*Error"' src/type_system/checker/fs_builtins.rs`.

- `OPAL_FS_ERR_NOT_FOUND "FileNotFoundError"`
- `OPAL_FS_ERR_PERMISSION_DENIED "PermissionDeniedError"`
- `OPAL_FS_ERR_IS_DIRECTORY "IsADirectoryError"`
- `OPAL_FS_ERR_NOT_A_DIRECTORY "IsNotADirectoryError"`
- `OPAL_FS_ERR_INVALID_UTF8 "InvalidUtf8Error"`
- `OPAL_FS_ERR_ALREADY_EXISTS "FileAlreadyExistsError"`
- `OPAL_FS_ERR_INVALID_PATH "InvalidPathError"`
- `OPAL_FS_ERR_OUT_OF_BOUNDS "OffsetOutOfRangeError"` (or `"LineOutOfRangeError"` per call site — pick at emission)
- `OPAL_FS_ERR_IO "ReadFailureError"` (or `"WriteFailureError"`/`"CopyFailureError"`/`"MoveFailureError"`/`"DeleteFailureError"`/`"CreateFailureError"` — pick at emission per operation)
- `OPAL_FS_ERR_FILESYSTEM_FULL "FilesystemFullError"`
- `OPAL_FS_ERR_DIRECTORY_NOT_EMPTY "DirectoryNotEmptyError"`
- `OPAL_FS_ERR_DIRECTORY_NOT_FOUND "DirectoryNotFoundError"`
- `OPAL_FS_ERR_METADATA_UNAVAILABLE "MetadataUnavailableError"`
- `OPAL_FS_ERR_SET_PERMISSIONS "SetPermissionsError"`
  - Add helper function `char* opal_fs_format_err(const char* prefix, const char* detail)` (NOT a macro — must allocate) that returns a heap-allocated string `"<prefix>: <detail>"` via `malloc`+`snprintf`. Caller becomes owner; freed by Opal runtime's error consumer (string-result deallocator path, same as `value` strings).
  - **Allocation contract** (LOCKED): Error strings in `Fs*Result.error` are EITHER (a) NULL on success, OR (b) a heap-allocated `char*` produced by `opal_fs_format_err` or `strdup`. Static string literals (current stub pattern at `runtime/opal_fs.c:111,117,129,…`) are FORBIDDEN in T15+ replacements — they break the uniform-free contract on the consumer side. T15 REFACTOR step MUST audit and convert any static-literal error strings touched.
  - Include header from `runtime/opal_fs.c` (after `opal_portability.h`, before std headers).
  - Document the rule in a top-of-file comment: "All fs error strings emitted by runtime MUST start with one of these discriminants, followed by ': ' and a detail string. Opalescent code matches on the prefix."

  **Must NOT do**:
  - Replace any stub error strings yet (T15+ does that).
  - Add error types not already declared in `src/type_system/checker/fs_builtins.rs`.

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Simple file creation with a defined contract; no investigation needed beyond reading the existing 20 error types.
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: NO (sequential within Wave 1; T1 must complete first to ensure doc-block style consistency)
  - **Blocks**: T15, T26-T29
  - **Blocked By**: T1

  **References**:
  - `src/type_system/checker/fs_builtins.rs:1-92` — 20 fs error types currently registered.
  - `runtime/opal_fs.c` (header section, lines 1-30) — include order convention.
  - `runtime/opal_portability.h` — must be included BEFORE this new header.

  **WHY**:
  - Without a SSOT for error discriminants, every replace-stubs task will invent strings and break `match` on errors in Opalescent code.

  **Acceptance Criteria**:
  - [ ] `runtime/opal_fs_errors.h` exists with all 14 macros listed above (lines 466-479 of this plan). NOTE: the 14 macros map to 20 nominal error type names (registered in `src/type_system/checker/fs_builtins.rs:29-48`) because two macros use call-site discriminant selection: `OPAL_FS_ERR_OUT_OF_BOUNDS` → 2 names (line 473), `OPAL_FS_ERR_IO` → 6 names (line 474). Macro count: 14. Nominal-name count: 20.
  - [ ] Header is `#include`-guarded.
  - [ ] `runtime/opal_fs.c` includes the new header (after `opal_portability.h`).
  - [ ] `cargo test --all-features` → green.
  - [ ] MSVC compile/link gate DEFERRED to T3 (see "Per-Task Regression Gate" phasing note: `opal_fs.c:114` uses raw `realpath()` which can't compile under MSVC until T3 atomically introduces `opal_realpath` AND swaps the call site in a single 2-line edit). Note: `opal_fs_errors.h` itself is plain macros + one `static inline` helper; no platform-specific dependencies, so it parses cleanly under both POSIX and MSVC preprocessors. Header-only verification at this stage is achieved transitively via T3's QA scenario (which probe-compiles `opal_portability.h` under clang-cl, and the chain `opal_portability.h → opal_fs_errors.h` is exercised once T2 lands and T3 runs).

  **QA Scenarios**:
  ```
  Scenario: Header exposes all 14 discriminants
    Tool: Bash
    Steps:
      1. Run: grep -cE "^#define OPAL_FS_ERR_[A-Z_]+ " runtime/opal_fs_errors.h
      2. Assert: count == 14 (exact — see plan lines 466-479 for the 14-macro list)
      3. Run: grep -oE '"[A-Z][a-zA-Z]+Error"' runtime/opal_fs_errors.h | sort -u | wc -l
      4. Assert: count == 20 (exact — must match the 20 nominal types in `src/type_system/checker/fs_builtins.rs:29-48`)
      5. Run: comm -23 <(grep -oE '"[A-Z][a-zA-Z]+Error"' runtime/opal_fs_errors.h | sort -u) <(grep -oE '"[A-Z][a-zA-Z]+Error"' src/type_system/checker/fs_builtins.rs | sort -u)
      6. Assert: empty output (every nominal name in the runtime header is present in the type-system registry)
    Expected Result: 14 macros defined; 20 nominal error type names registered; SSOT alignment verified.
    Evidence: .sisyphus/evidence/task-2-headers.txt

  Scenario: opal_fs.c picks up new header
    Tool: Bash
    Steps:
      1. Run: grep "opal_fs_errors.h" runtime/opal_fs.c
      2. Assert: include line present
    Evidence: .sisyphus/evidence/task-2-include.txt
  ```

  **Commit**: YES
  - Message: `chore(runtime): add fs error-discriminant SSOT header`
  - Files: `runtime/opal_fs_errors.h`, `runtime/opal_fs.c`
  - Pre-commit: `cargo test --all-features` green + MSVC gates

- [x] 3. **T3: Portability Extend — Windows Shims in `runtime/opal_portability.h`**

  **What to do**:
  - Extend `runtime/opal_portability.h` with Windows implementations of:
    - `opal_opendir`, `opal_readdir`, `opal_closedir` (FindFirstFileA-based).
    - `opal_realpath(const char* path, char* resolved_buf, size_t buf_size)` — `_fullpath` on MSVC, `realpath` on POSIX. **Behavior unification**: on POSIX, if `realpath()` returns NULL with `errno == ENOENT`, fall back to lexical resolution (concatenate cwd + path, collapse `.`/`..` segments, normalize separators) so non-existent paths return a valid absolute path on both platforms — matches Windows `_fullpath` semantics. Document in a header comment block above the function.
    - `opal_stat(const char* path, struct opal_stat_result*)` — wraps `_stat64` on MSVC, `stat` on POSIX.
    - `opal_mkdir(const char* path)` — `_mkdir` on MSVC, `mkdir` on POSIX (use 0755 mode on POSIX).
    - `opal_rmdir(const char* path)` — `_rmdir` on MSVC, `rmdir` on POSIX.
    - `opal_unlink(const char* path)` — `_unlink` on MSVC, `unlink` on POSIX.
    - `opal_path_separator()` — returns `'\\'` on Windows, `'/'` on POSIX.
  - Define `OPAL_API` macro: `__declspec(dllexport)` on Windows + DLL build, empty otherwise.
  - All raw `_WIN32`/`_MSC_VER` macros confined to this header.
  - Document include order in a top-of-file comment.
  - **Targeted `opal_fs.c` call-site swap (the ONLY edit allowed in `opal_fs.c` from this task)**: change `runtime/opal_fs.c:114` from `char* resolved = realpath(path, NULL);` to use `opal_realpath` via a small adapter. Two-line change: declare a fixed-size buffer (`char buf[OPAL_PATH_MAX]; char* resolved = opal_realpath(path, buf, sizeof(buf)) ? strdup(buf) : NULL;`). `OPAL_PATH_MAX` is defined in `opal_portability.h` (`4096` on POSIX, `_MAX_PATH`/`260` on Windows). This swap UNBLOCKS the MSVC compile gate from T3 (this task) onwards. NO other lines in `opal_fs.c` are touched. The semantic change is zero on Linux when paths exist; for non-existent paths on Linux, behavior shifts from "return NULL → InvalidPathError" to "return lexical absolute path → success" — this is the unified behavior and matches T14's documented contract.

  **Must NOT do**:
  - Implement long-path (`\\\\?\\`) prefix handling — DEFERRED.
  - Add permission-related shims (chmod, etc.) — DEFERRED.
  - Modify `runtime/opal_fs.c` BEYOND the single 2-line `realpath`→`opal_realpath` swap at line 114 (T15+ does the broader audit/refactor of all error strings and stub replacements).

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
    - Reason: Cross-platform C with Windows API knowledge; non-trivial but well-scoped.
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES with T1, T2 (independent files)
  - **Blocks**: T8 onwards (all consuming tasks).
  - **Blocked By**: None

  **References**:
  - `runtime/opal_portability.h` — current state (extend, do not rewrite).
  - `.sisyphus/plans/windows-support.md` D4 directive — SSOT rule.
  - Microsoft docs: `FindFirstFileA`, `_fullpath`, `_stat64`, `_mkdir`, `_rmdir`, `_unlink`.
  - POSIX: `dirent.h`, `realpath(3)`, `stat(2)`, `mkdir(2)`, `rmdir(2)`, `unlink(2)`.

  **WHY**:
  - All consuming runtime tasks (T15-T29) need these shims; centralizing them once prevents drift and stomping.

  **Acceptance Criteria**:
  - [ ] All 7 shim functions implemented for both Windows and POSIX branches.
  - [ ] `OPAL_API` macro defined.
  - [ ] `OPAL_PATH_MAX` macro defined (`4096` POSIX / `_MAX_PATH` Windows).
  - [ ] `opal_realpath` lexical-fallback documented and implemented on POSIX (ENOENT → lexical resolve).
  - [ ] `runtime/opal_fs.c:114` swapped from raw `realpath()` to `opal_realpath()`-via-adapter (2-line change only).
  - [ ] `clang-cl /c runtime/opal_portability.h` (via a stub .c that includes it) → exit 0.
  - [ ] `clang-cl /c runtime/opal_fs.c` (xwin flags) → exit 0 (FIRST task at which this is achievable).
  - [ ] POSIX `gcc -c` of a stub including the header → exit 0.
  - [ ] `cargo test --all-features` → green.
  - [ ] `cargo test --features integration absolute_path` → green (verifies semantic change for non-existent paths is acceptable on Linux).
  - [ ] `ast-grep --lang c --pattern '#ifdef _WIN32'` outside `opal_portability.h` → 0 matches in `runtime/`.
  - [ ] `grep -n 'realpath(' runtime/opal_fs.c` → 0 matches (only `opal_realpath` should remain).

   **QA Scenarios**:
   ```
   Scenario: Header compiles under MSVC and POSIX
     Tool: Bash
     Steps:
       1. printf '#include "runtime/opal_portability.h"\nint main(void){return 0;}\n' > /tmp/probe.c
      2. clang-cl /nologo /c /imsvc $XWIN_CACHE/crt/include /imsvc $XWIN_CACHE/sdk/include/ucrt /imsvc $XWIN_CACHE/sdk/include/um /imsvc $XWIN_CACHE/sdk/include/shared /tmp/probe.c -Fo/tmp/probe.obj
      3. gcc -c /tmp/probe.c -o /tmp/probe-posix.o
    Expected Result: both compiles exit 0.
    Evidence: .sisyphus/evidence/task-3-msvc-probe.log, .sisyphus/evidence/task-3-posix-probe.log

  Scenario: opal_fs.c compiles under MSVC after call-site swap
    Tool: Bash
    Steps:
      1. clang-cl /nologo /c /imsvc $XWIN_CACHE/crt/include /imsvc $XWIN_CACHE/sdk/include/ucrt /imsvc $XWIN_CACHE/sdk/include/um /imsvc $XWIN_CACHE/sdk/include/shared runtime/opal_fs.c -Foruntime/opal_fs.obj
      2. Assert: exit 0
      3. Run: grep -nE '\brealpath\s*\(' runtime/opal_fs.c
      4. Assert: empty output (only `opal_realpath` should remain).
    Expected Result: clang-cl exits 0; no raw realpath() call sites left.
    Evidence: .sisyphus/evidence/task-3-opal-fs-msvc.log

  Scenario: Linux behavior preserved for existing paths
    Tool: Bash
    Steps:
      1. cargo test --features integration absolute_path 2>&1 | tee .sisyphus/evidence/task-3-linux-abspath.log
      2. Assert: "test result: ok" in output
    Expected Result: existing absolute_path tests still pass on Linux.
    Evidence: .sisyphus/evidence/task-3-linux-abspath.log

  Scenario: No raw _WIN32 outside opal_portability.h
    Tool: Bash
    Steps:
      1. Run: grep -RE '\b_WIN32\b|\b_MSC_VER\b' runtime/ | grep -v opal_portability.h
      2. Assert: empty output
    Expected Result: 0 matches.
    Evidence: .sisyphus/evidence/task-3-grep-portability.txt
  ```

  **Commit**: YES
  - Message: `feat(runtime): extend opal_portability.h with Windows fs shims + swap opal_fs.c realpath call site`
  - Files: `runtime/opal_portability.h`, `runtime/opal_fs.c` (single 2-line swap at line 114)
  - Pre-commit: regression gate + MSVC compile (opal_portability.h probe + opal_fs.c) + POSIX compile + Linux absolute_path tests green

- [x] 4. **T4: `FsStateGuard` RAII Implementation**

   **What to do**:
   - Create `tests/integration_e2e/fs_state_guard.rs` defining `pub struct FsStateGuard { project_path: PathBuf, manifest: Vec<(PathBuf, [u8; 32])> }`.
   - `impl FsStateGuard`:
     - `pub fn new(project_path: impl AsRef<Path>) -> io::Result<Self>` — hashes `src/**`, `tests/fixtures/**` (if exists), and top-level files (`opal.toml`, `opal.pkg.toml`, `.gitignore`, `.gitattributes`, `README.md`); wipes and recreates `target/` and `workspace/`.
     - `impl Drop` — wipes `target/` and `workspace/`; re-hashes manifest set; if mismatched AND `!std::thread::panicking()`, panics with diff list; if already panicking, prints `eprintln!` and returns silently.
     - Hash algorithm: stable sort relative paths, then `sha256(path_bytes || ":" || sha256(file_bytes))` per file, then `sha256` of concatenation. Use `sha2 = "0.10"` crate (add to dev-dependencies).
   - Add `tempfile = "3"` to `Cargo.toml [dev-dependencies]` if not already present (run `grep tempfile Cargo.toml` to check).
   - Smoke unit tests inside `fs_state_guard.rs` MUST use `tempfile::TempDir` for self-contained testing — do NOT depend on any committed `test-projects/` directory (those are created in T8+).

  **Must NOT do**:
  - Use `tempfile::TempDir` for ANY actual fs project workspace (defeats FsStateGuard semantics — see global rule at line 180). Note: T4's smoke unit tests INSIDE `fs_state_guard.rs` ARE permitted to use `tempfile::TempDir` for self-contained guard-mechanism testing only — that exception is scoped strictly to T4's own unit tests, never to a real fs project.
  - Hash `target/` or `workspace/` (these are wipe-only).
  - Hash `.git/`.

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: Drop-on-panic semantics are subtle; sha256 manifest must be deterministic across platforms (path sorting, byte-mode reads).
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: NO (must complete before T8 smoke project)
  - **Blocks**: T8-T32
  - **Blocked By**: T3 (uses portability conventions in tests)

  **References**:
  - `tests/integration_e2e.rs:15-28` — existing `prepare_dir`/`cleanup_dir` (FsStateGuard wraps these conceptually).
  - `tests/integration_print.rs` — harness reference for `#[cfg(feature = "integration")]` test pattern.
  - Rust std: `std::thread::panicking()`, `std::fs::read`, `std::fs::remove_dir_all`.

  **WHY**:
  - Without this RAII, every fs test risks leaving stale state and causing the next run to fail. User explicitly required full re-runnability.

  **Acceptance Criteria**:
  - [ ] `tests/integration_e2e/fs_state_guard.rs` exists with `FsStateGuard` type.
  - [ ] `Cargo.toml` has `sha2 = "0.10"` in `[dev-dependencies]`.
  - [ ] Smoke test `fs_state_guard_smoke` passes.
  - [ ] Smoke test verifies: pre-run hash == post-run hash for `src/`; `target/` and `workspace/` empty after Drop.
  - [ ] `cargo test --all-features` → green.
  - [ ] MSVC compile gate (`clang-cl /c runtime/opal_fs.c`) → exit 0 (T3 swap already in tree; verify regression).
  - [ ] MSVC link gate DEFERRED to T5 (link-probe artifact not yet created — T5 introduces `runtime/opal_msvc_link_probe.c` + `scripts/msvc_link_probe.sh`).

   **QA Scenarios**:
   ```
   Scenario: FsStateGuard smoke test (self-contained, uses tempdir)
     Tool: Bash
     Preconditions: T4 deliverables in tests/integration_e2e/fs_state_guard.rs; tempfile dev-dep available
     Steps:
       1. Run: cargo test --features integration --test integration_e2e fs_state_guard::smoke 2>&1 | tee .sisyphus/evidence/task-4-smoke.log
       2. Assert: exit 0 AND output contains "1 passed"
       3. Run: cargo test --features integration --test integration_e2e fs_state_guard::manifest_diff 2>&1 | tee .sisyphus/evidence/task-4-manifest.log
       4. Assert: exit 0 AND output contains "1 passed"
     Expected Result: FsStateGuard's RAII Drop and manifest comparison verified using tempdir-only fixtures (no committed _fs_* projects required).
     Evidence: .sisyphus/evidence/task-4-smoke.log + task-4-manifest.log
   ```

  **Evidence to Capture**:
  - [ ] `.sisyphus/evidence/task-4-smoke.log`
  - [ ] `.sisyphus/evidence/task-4-mutation-detect.log`
  - [ ] `.sisyphus/evidence/task-4-msvc.log`

  **Commit**: YES
  - Message: `feat(test): add FsStateGuard RAII for fs project re-runnability`
  - Files: `tests/integration_e2e/fs_state_guard.rs`, `Cargo.toml`, `tests/integration_e2e/tests.rs` (mod declaration)
  - Pre-commit: regression gate green

- [x] 5. **T5: MSVC Link-Probe Harness (`runtime/opal_msvc_link_probe.c` + script)**

  **What to do**:
  - Create `runtime/opal_msvc_link_probe.c`: a minimal `int main(void) { return 0; }` that:
    - `#include`s **HEADERS ONLY**: `opal_runtime.h`, `opal_fs_errors.h`, `opal_portability.h`. **CRITICAL**: do NOT `#include "opal_fs.c"` — including a .c file would cause its definitions to be compiled into the probe object AND `opal_fs.obj` is also linked, causing `lld-link` LNK2005 multiply-defined symbol errors.
    - References (calls or takes address of) at least one symbol from each translation unit via `extern` declarations or via the public API exposed in headers:
      - From `opal_portability.h`: call `opal_path_separator()`.
      - From `opal_fs_errors.h`: use `OPAL_FS_ERR_NOT_FOUND` macro in a static const string.
      - From `opal_fs.c` (via runtime header): take address of `read_text_sync` (declared `OPAL_API` in `runtime/opal_runtime.h:97`) — DO NOT CALL it; just `void* p = (void*)&read_text_sync;` to force the linker to resolve the symbol. Verified to exist via `grep "read_text_sync" runtime/opal_runtime.h`.
    - Goal: the probe FORCES the linker to resolve runtime symbols WITHOUT pulling their definitions through `#include`.
  - Create `scripts/msvc_link_probe.sh` (POSIX shell) that:
    1. Verifies `XWIN_CACHE` is set (else fail with clear message).
    2. Runs `clang-cl /nologo /c $XWIN_FLAGS runtime/opal_fs.c -Fo:runtime/opal_fs.obj` to compile the runtime ONCE.
    3. Runs `clang-cl /nologo /c $XWIN_FLAGS runtime/opal_msvc_link_probe.c -Fo:runtime/opal_msvc_link_probe.obj` to compile the probe (which uses `extern` decls only).
    4. Runs `lld-link /subsystem:console /entry:main /out:/tmp/opal_probe.exe runtime/opal_msvc_link_probe.obj runtime/opal_fs.obj kernel32.lib libucrt.lib libcmt.lib /libpath:$XWIN_CACHE/crt/lib/x86_64 /libpath:$XWIN_CACHE/sdk/lib/um/x86_64 /libpath:$XWIN_CACHE/sdk/lib/ucrt/x86_64`.
    5. Echoes `MSVC LINK PROBE: PASS` on success or `FAIL: <reason>` and exit 1 on failure.
  - **Verification of single-definition discipline**: script asserts `nm` (or `llvm-nm`) on the probe object shows runtime symbols as UNDEFINED (`U`), and on the runtime object shows them as DEFINED (`T`). If runtime symbols are defined in BOTH objects → fail with explicit "duplicate definition" error.
  - Make script executable (`chmod +x`).
  - Document in script header: "Run this after every runtime/ change touched by tasks T15+. Required for Regression Gate compliance."

  **Must NOT do**:
  - Run the linked .exe (Wine optional, not required for link-gate).
  - Add to CI (defer to user; Sisyphus runs it locally per task).

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
    - Reason: MSVC linker flag knowledge required; libpath ordering matters; cross-compilation specifics.
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES with T4 (independent file); REQUIRES T2 + T3 already complete (uses `opal_fs_format_err` exports from T2 + compile-gate unblocked at T3)
  - **Blocks**: All runtime tasks T15-T29 (used as gate)
  - **Blocked By**: T2 (opal_fs_errors.h + opal_fs_format_err), T3 (compile gate must already be unblocked via opal_realpath swap)

  **References**:
  - `.sisyphus/plans/windows-support.md` D4/Regression Gate sections — MSVC verification protocol.
  - README "Cross-compilation from Linux (MSVC Target)" section — `XWIN_CACHE`, `CFLAGS_x86_64_pc_windows_msvc` env.
  - Microsoft `lld-link` docs — `/subsystem`, `/entry`, `/libpath`, `kernel32.lib`/`libucrt.lib`/`libcmt.lib`.

  **WHY**:
  - `clang-cl /c` only compiles, doesn't catch unresolved externals (e.g., wrong `_stat64` linkage). Without a link probe, MSVC breakage hides until a real Windows build. Per `windows-support.md` D4.

  **Acceptance Criteria**:
  - [ ] `runtime/opal_msvc_link_probe.c` exists and `#include`s all 4 runtime headers.
  - [ ] `scripts/msvc_link_probe.sh` exists, executable, fails clearly when `XWIN_CACHE` unset.
  - [ ] Running `bash scripts/msvc_link_probe.sh` → exit 0 + `MSVC LINK PROBE: PASS` on stdout.
  - [ ] `cargo test --all-features` → green.

  **QA Scenarios**:
  ```
  Scenario: Link probe passes after T1-T4
    Tool: Bash
    Preconditions: XWIN_CACHE set; T1-T4 complete; runtime is in pre-stub-replace state
    Steps:
      1. Run: bash scripts/msvc_link_probe.sh 2>&1 | tee .sisyphus/evidence/task-5-probe.log
      2. Assert: exit 0
      3. Assert: stdout contains "MSVC LINK PROBE: PASS"
    Expected Result: link probe passes; produces /tmp/opal_probe.exe.
    Failure Indicators: unresolved external, "cannot open input file" → libpath wrong.
    Evidence: .sisyphus/evidence/task-5-probe.log

  Scenario: Probe fails clearly when XWIN_CACHE unset
    Tool: Bash
    Steps:
      1. Run: env -u XWIN_CACHE bash scripts/msvc_link_probe.sh; echo "exit=$?"
      2. Assert: exit code != 0
      3. Assert: stderr contains "XWIN_CACHE" and "set"
    Evidence: .sisyphus/evidence/task-5-probe-no-env.log
  ```

  **Commit**: YES
  - Message: `feat(runtime): add MSVC clang-cl + lld-link probe harness`
  - Files: `runtime/opal_msvc_link_probe.c`, `scripts/msvc_link_probe.sh`
  - Pre-commit: link probe passes + regression gate green

- [x] 6. **T6: Test Fixture Conventions — `.gitattributes` + Helper Module**

  **What to do**:
  - Create `.gitattributes` at repo root with:
    ```
    test-projects/**/fixtures/**     -text
    test-projects/**/*.crlf.txt      -text
    test-projects/**/*.bin           -text binary
    ```
  - For each of the 20 fs test projects (created in later waves), the agent must add a per-project `.gitattributes` with the same rules (T8+ tasks include this in their checklist).
  - Add `tests/integration_e2e/fs_helpers.rs` exposing:
    - `pub fn fs_project_root(name: &str) -> PathBuf` — returns `<repo>/test-projects/<name>`.
    - `pub fn read_evidence(name: &str, scenario: &str) -> String` — convenience for asserting evidence.
    - `pub fn assert_workspace_empty(project: &str)` — asserts `target/` and `workspace/` are empty/missing.
    - `pub use crate::integration_e2e::fs_state_guard::FsStateGuard;` — re-export.
    - `pub fn strip_crlf(s: &str) -> String` — **NEW helper, implemented fresh in this task** (does NOT currently exist anywhere in `tests/` — verified via `grep -r "strip_crlf" tests/` returns zero matches). Replaces all `\r\n` with `\n` and strips trailing `\r` characters. Used to normalize line endings before assertion comparisons across platforms.
  - Wire `mod fs_helpers;` and `mod fs_state_guard;` into `tests/integration_e2e/tests.rs` (NOT into `tests/integration_e2e.rs` — that file uses `#[path = "integration_e2e/tests.rs"] mod tests;` and `tests.rs` is the module-tree root for sibling files). Verified via reading `tests/integration_e2e.rs:31-32` + `tests/integration_e2e/tests.rs:5-19`. Add as `pub(crate) mod fs_helpers;` + `pub(crate) mod fs_state_guard;` so other test modules can import via `use super::fs_helpers::*;`.

  **Must NOT do**:
  - Replace `prepare_dir`/`cleanup_dir` (those are still used by non-fs tests).
  - Move `compile_program` callsites.

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Mechanical wiring + small helper module.
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES with T5 (different files)
  - **Blocks**: T8-T32
  - **Blocked By**: T4 (FsStateGuard re-export)

  **References**:
  - `tests/integration_e2e.rs:15-28` — `prepare_dir` / `cleanup_dir` helpers (note: `strip_crlf` does NOT exist here — implemented fresh in T6).
  - Git docs: `.gitattributes` `-text` semantics (no normalization on checkout).

  **WHY**:
  - Fixtures with deliberate CRLF endings get clobbered by `core.autocrlf=true` on Windows checkouts; `-text` is the only reliable preservation method.

  **Acceptance Criteria**:
  - [ ] Repo-root `.gitattributes` exists with the 3 rules above.
  - [ ] `tests/integration_e2e/fs_helpers.rs` exists with all 5 exports listed.
  - [ ] `tests/integration_e2e/tests.rs` declares `pub(crate) mod fs_helpers;` + `pub(crate) mod fs_state_guard;` (NOT in `tests/integration_e2e.rs`).
  - [ ] `cargo test --all-features` → green.
  - [ ] `git check-attr text -- 'test-projects/probe-project/fixtures/sample.txt'` → reports `unset` (works without file existing — `check-attr` evaluates patterns against pathspecs).
  - [ ] `git check-attr text -- 'test-projects/probe-project/foo.crlf.txt'` → reports `unset`.
  - [ ] `git check-attr binary -- 'test-projects/probe-project/foo.bin'` → reports `set`.

  **QA Scenarios**:
  ```
  Scenario: gitattributes rules registered
    Tool: Bash
    Preconditions: NONE — `git check-attr` evaluates pattern rules against a pathspec; the file does NOT need to exist on disk. Probe paths chosen below match the 3 rules added to repo-root `.gitattributes`.
    Steps:
      1. Run: git check-attr text -- 'test-projects/probe-project/fixtures/sample.txt' 2>&1
      2. Assert: output ends with ": text: unset"
      3. Run: git check-attr text -- 'test-projects/probe-project/example.crlf.txt' 2>&1
      4. Assert: output ends with ": text: unset"
      5. Run: git check-attr binary -- 'test-projects/probe-project/blob.bin' 2>&1
      6. Assert: output ends with ": binary: set"
    Expected Result: all 3 attribute rules resolve as expected → no LF↔CRLF normalization for fixtures, binary marker for `.bin`.
    Evidence: .sisyphus/evidence/task-6-gitattr.log

  Scenario: helpers compile and re-export FsStateGuard
    Tool: Bash
    Steps:
      1. Run: cargo build --tests --features integration 2>&1 | tee .sisyphus/evidence/task-6-build.log
      2. Assert: exit 0
      3. Run: grep "pub use" tests/integration_e2e/fs_helpers.rs
      4. Assert: line includes "FsStateGuard"
    Evidence: .sisyphus/evidence/task-6-build.log
  ```

  **Commit**: YES
  - Message: `chore(test): add .gitattributes + fs_helpers module for fs test infra`
  - Files: `.gitattributes`, `tests/integration_e2e/fs_helpers.rs`, `tests/integration_e2e/tests.rs` (mod declaration — see T4 wiring contract)
  - Pre-commit: regression gate green + MSVC gates

- [x] 7. **T7: `serial_test` Dev-Dep + Prelude `fs` Doc Section**

  **What to do**:
  - Add to `Cargo.toml` `[dev-dependencies]`: `serial_test = "3"`.
  - Add to `stdlib/prelude.op` (≤ 50 new lines, append at end before any existing closing) a doc-only `## fs` section listing the **34 existing fs_-prefixed builtins** grouped by family (Path Manipulation 9, Read 5, Write 4, Directory 5, File 4, Metadata 4, High-level 3) — names + signatures only, no impls. T18 will later append `read_first_line_sync` (without `fs_` prefix — intentional per user decision) to this same `## fs` section as the 35th entry.
  - Format: each builtin on one line: `# fs_read_file_to_string(path: string): FsStringResult — read whole file as UTF-8 string`. The T18 entry will be `# read_first_line_sync(path: string): FsStringResult — read first line of UTF-8 file (FsReadError on failure)`.

  **Must NOT do**:
  - Add Opalescent function bodies (these are codegen-resolved builtins, not stdlib defs).
  - Modify any other prelude content.
  - Change formatting of existing prelude lines.

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Mechanical doc edit + Cargo.toml line.
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES with T5/T6 (different files)
  - **Blocks**: None directly (documentation gate)
  - **Blocked By**: T1 (uses sentinel doc style)

  **References**:
  - `stdlib/prelude.op` (28 lines) — current contents, formatting style.
  - `src/type_system/checker/fs_builtins.rs:1-92` — authoritative list of fs error nominal types (20 `…Error` discriminants); function builtins are registered separately in `src/type_system/module_resolver/standard_symbols_core_io_and_bytes.rs`, `standard_symbols_filesystem_operations.rs`, and `standard_symbols_filesystem_types_and_errors.rs`.
  - https://docs.rs/serial_test/3.0.0/serial_test/ — `#[serial(key)]` semantics.

  **WHY**:
  - `serial_test` enables grouped serialization (`#[serial(fs)]`) without forcing `--test-threads=1` globally; preserves test parallelism for non-fs tests.
  - Prelude doc section gives users in-tree discovery of fs API without reading checker source.

  **Acceptance Criteria**:
  - [ ] `Cargo.toml` `[dev-dependencies]` contains `serial_test = "3"`.
  - [ ] `cargo build --tests --features integration` → green (resolves serial_test).
  - [ ] `stdlib/prelude.op` has a `## fs` section listing the 34 existing `fs_`-prefixed builtins (T18 later appends `read_first_line_sync` as the 35th).
  - [ ] `grep -c '^# fs_' stdlib/prelude.op` → exactly 34.
  - [ ] `grep -c '^## fs' stdlib/prelude.op` → exactly 1 (single section header).
  - [ ] `cargo test --all-features` → green.
  - [ ] MSVC link probe → PASS (no behavior change).

  **QA Scenarios**:
  ```
  Scenario: serial_test resolves and 34 fs_-prefixed builtins documented (read_first_line_sync added in T18)
    Tool: Bash
    Preconditions: T18 has NOT yet run at this point in the plan; prelude doc-section initially lists only the 34 fs_-prefixed builtins. T18 will append `# read_first_line_sync(...)` (no `fs_` prefix — see T18 acceptance criteria).
    Steps:
      1. Run: grep 'serial_test' Cargo.toml
      2. Assert: line contains "= \"3\""
      3. Run: grep -c '^# fs_' stdlib/prelude.op
      4. Assert: count == 34 (all `fs_`-prefixed builtins)
      5. Run: grep -c '^## fs' stdlib/prelude.op
      6. Assert: count == 1 (the `## fs` section header exists exactly once)
      7. Run: cargo build --tests --features integration 2>&1 | tail -5
      8. Assert: exit 0
    Expected Result: dep resolves; all 34 `fs_`-prefixed builtins enumerated under a single `## fs` section. `read_first_line_sync` (the 35th) is NOT yet present — added by T18 as a non-`fs_`-prefixed entry.
    Evidence: .sisyphus/evidence/task-7-prelude.log
  ```

  **Commit**: YES
  - Message: `chore(stdlib): document fs builtins in prelude; add serial_test dev-dep`
  - Files: `Cargo.toml`, `stdlib/prelude.op`
  - Pre-commit: regression gate green

---

## Wave 2 — Smoke (Validate Pipeline Before Stub Work)

- [x] 8. **T8: `_fs_path_from` Smoke Project — End-to-End Pipeline Validation**

  **What to do**:
  - Create `test-projects/_fs_path_from/` with:
    - `opal.toml`: `name = "_fs_path_from"`, `version = "0.1.0"`, `[build] targets = ["x86_64-linux"]`.
    - `.gitignore`: `target/`, `workspace/`.
    - `.gitattributes`: same 3 rules from T6.
    - `README.md` (≤ 30 lines): "Smoke project for `path_from` builtin. Validates that fs builtin compile + link + run pipeline works end-to-end on Linux BEFORE replacing 27 stubs."
    - `src/main.op` (≤ 60 lines): calls `path_from("hello/world")` (which is currently a SILENT BUG — returns identity); prints the result; entry signature `entry main = f(args: string[]): void =>`.
    - `src/paths.op`: small helper `let print_path = f(p: Path): void => print('path={p.value}')` (validates multi-file project layout).
  - Create `tests/integration_e2e/fs_path_from.rs`:
    - `#[test] #[cfg(feature = "integration")] #[serial(fs)] fn fs_path_from_smoke()`
    - Body: instantiate `FsStateGuard::new("test-projects/_fs_path_from")?`, run `compile_program(&main_source, &target_dir)`, exec the binary via `Command`, capture stdout, assert it contains `path=hello/world` (current silent-bug behavior — documents existing state).
    - On Drop, FsStateGuard wipes target/workspace and verifies src manifest unchanged.
  - DO NOT fix `path_from` here — Wave 3 (T9) does that. This task documents current behavior as a baseline.
  - Wire `mod fs_path_from;` into `tests/integration_e2e/tests.rs` (the module-tree root — see T4/T6 wiring contract). Several stub `mod fs_*;` lines are already commented-out in `tests/integration_e2e/tests.rs:10-17` — uncomment the matching one or add new line if not present.

  **Must NOT do**:
  - Fix `path_from` silent bug (deferred to T9).
  - Replace any I/O stubs (deferred to Wave 4+).
  - Add additional builtin calls beyond `path_from`.

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: First end-to-end fs project; FsStateGuard integration; baseline-bug documentation; sets template for 16 other `_fs_*` projects.
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: NO (smoke gate)
  - **Blocks**: T9-T34 (template for all other _fs_* projects)
  - **Blocked By**: T1, T2, T3, T4, T5, T6, T7

  **References**:
  - `test-projects/bytes-hex-roundtrip/` — closest existing reference for opal.toml + multi-file src/ layout.
  - `tests/integration_e2e.rs:15-28` — prepare_dir/cleanup_dir (replaced here by FsStateGuard).
  - `tests/integration_print.rs` — closest harness reference for `compile_program` + `Command::new`.
  - `src/codegen/functions_stdlib.rs` (path_from arm) — confirms Path-returning signature.
  - `src/type_system/checker/fs_builtins.rs` — Path nominal type.

  **WHY**:
  - This is the smoke gate for EVERY downstream fs task. If T8 doesn't compile/link/run on Linux, no later task can. Failing fast here saves weeks of mis-scoped work.

  **Acceptance Criteria**:
  - [ ] `test-projects/_fs_path_from/` has all 5 files (opal.toml, .gitignore, .gitattributes, README.md, src/main.op, src/paths.op).
  - [ ] `target/release/opalescent run test-projects/_fs_path_from/src/main.op` → exit 0; stdout contains `path=hello/world`.
  - [ ] `cargo test --features integration fs_path_from_smoke` → exit 0.
  - [ ] After test: `test-projects/_fs_path_from/target/` and `workspace/` empty/missing.
  - [ ] After test: `sha256sum test-projects/_fs_path_from/src/main.op` unchanged from before-test snapshot.
  - [ ] Re-run `cargo test --features integration fs_path_from_smoke` → still passes (re-runnability).
  - [ ] `cargo test --all-features` → green.
  - [ ] MSVC link probe → PASS.

  **QA Scenarios**:
  ```
  Scenario: Happy path — opal run produces expected stdout
    Tool: Bash
    Preconditions: cargo build --release done; alias opal=$(pwd)/target/release/opalescent
    Steps:
      1. Run: opal run test-projects/_fs_path_from/src/main.op
      2. Wait for exit; capture stdout
      3. Assert: stdout contains "path=hello/world"
      4. Assert: exit code == 0
    Expected Result: program runs and prints documented baseline.
    Evidence: .sisyphus/evidence/task-8-opal-run.log

  Scenario: Re-runnability — second run identical to first
    Tool: Bash
    Preconditions: T8 commit landed
    Steps:
      1. Run: cargo test --features integration fs_path_from_smoke 2>&1 | tee /tmp/run1.log
      2. Run: sha256sum test-projects/_fs_path_from/src/*.op > /tmp/hash1.txt
      3. Assert: ls test-projects/_fs_path_from/target/ → empty or "No such file"
      4. Run: cargo test --features integration fs_path_from_smoke 2>&1 | tee /tmp/run2.log
      5. Run: sha256sum test-projects/_fs_path_from/src/*.op > /tmp/hash2.txt
      6. Assert: diff /tmp/hash1.txt /tmp/hash2.txt → empty
      7. Assert: both runs ended with "test result: ok. 1 passed"
    Expected Result: deterministic re-run; src/ untouched; target/workspace clean both times.
    Evidence: .sisyphus/evidence/task-8-rerun.log

  Scenario: FsStateGuard restores after deliberate workspace pollution
    Tool: Bash
    Steps:
      1. Run: mkdir -p test-projects/_fs_path_from/workspace && echo "stale" > test-projects/_fs_path_from/workspace/junk.txt
      2. Run: cargo test --features integration fs_path_from_smoke
      3. Assert: ls test-projects/_fs_path_from/workspace/ → empty or missing
    Expected Result: guard wipes pre-existing workspace pollution.
    Evidence: .sisyphus/evidence/task-8-pollution-cleanup.log
  ```

  **Evidence to Capture**:
  - [ ] `.sisyphus/evidence/task-8-opal-run.log`
  - [ ] `.sisyphus/evidence/task-8-rerun.log`
  - [ ] `.sisyphus/evidence/task-8-pollution-cleanup.log`
  - [ ] `.sisyphus/evidence/task-8-msvc-probe.log`
  - [ ] `.sisyphus/evidence/task-8-src-manifest.txt` (sha256 snapshot)

  **Commit**: YES
  - Message: `feat(test-projects): _fs_path_from smoke project (baseline; pre-fix)`
  - Files: `test-projects/_fs_path_from/**`, `tests/integration_e2e/fs_path_from.rs`, `tests/integration_e2e/tests.rs` (mod declaration — see T4 wiring contract)
  - Pre-commit: regression gate green + MSVC link probe PASS + smoke test PASS

## Wave 3 — Path Manipulation (Fix Silent Bugs + 6 Granular Projects)

> Wave 3 fixes 3 silent-bug helpers in `runtime/opal_fs.c` AND populates 6
> realistic `_fs_*` projects covering the 9 path-manipulation builtins.
> Per-builtin RGR enforced.

- [x] 9. **T9: Fix `path_from`, `normalize_path`, `join_path_components` (Silent Bugs — Infallible Contract)**

  **What to do**:
  - **CRITICAL CONTRACT (LOCKED)**: All three builtins are registered as **INFALLIBLE** in `src/type_system/module_resolver/standard_symbols_core_io_and_bytes.rs:389-477` (`error_types: Vec::new()`). Their runtime ABI is plain `char*` (NOT `FsPathResult`), per `runtime/opal_fs.c:55,60,102`:
    - `char* path_from(const char* raw)` — `(string) -> FilesystemPath`
    - `char* normalize_path(const char* path)` — `(FilesystemPath) -> FilesystemPath`
    - `char* join_path_components(const char* base, const char** components, int64_t count)` — `(FilesystemPath, string[]) -> FilesystemPath`
  - This task MUST NOT change the type system signature, the runtime ABI return type, or add `error_types` — coordinated symbol-registration changes are explicitly OUT OF SCOPE here. Validation and error-producing behavior live in `absolute_path_sync` (which IS fallible per `standard_symbols_core_io_and_bytes.rs:480-503` with `InvalidPathError` + `PermissionDeniedError`).
  - In `runtime/opal_fs.c`, replace the current identity/no-op implementations of the 3 path helpers (keeping the existing `char*` signatures intact):
    - `path_from(const char* raw)`: if `raw == NULL` or `raw[0] == '\0'`, return `strdup("")` (sentinel-empty Path; downstream normalize/absolute_path_sync detect+report). Otherwise return `strdup(raw)`. NO error string is returned (function is infallible at ABI level).
    - `normalize_path(const char* path)`: collapse runs of `/`, resolve `.` and `..` lexically (no FS calls), preserve leading `/` for absolute paths, preserve trailing slash semantics per locked policy. On `..` escaping root in absolute path, return `strdup("")` (empty Path, NOT an error string — caller path validation lives in `absolute_path_sync`). Empty input → `strdup("")`.
    - `join_path_components(const char* base, const char** components, int64_t count)`: start from `base` (use `""` if NULL); for each of `count` components, if component is absolute (starts with `/`), reset accumulator to that component; else append with single `/` separator (use `opal_path_separator()` from portability.h). Always run result through internal `lex_normalize` static helper. NULL `components` array with `count > 0` → return `strdup(base ? base : "")`.
  - All 3 helpers return heap-allocated `char*` via `strdup` (caller owns; Opal runtime's string-result deallocator frees).
  - All 3 are **lexical** — no `realpath` / `stat` / FS access (that's `absolute_path_sync`'s job).
  - Update `runtime/opal_runtime.h` doc block (T1) with finalized semantics: separator handling, `.` / `..` rules, trailing slash behavior, **and the empty-string sentinel** for invalid input.
  - **Per-scenario RGR**: For each of the 3 functions, write the failing Rust integration test FIRST (red), implement (green), then refactor with shared `lex_normalize` static helper once green.

  **Must NOT do**:
  - Touch any I/O stub (deferred to Wave 4+).
  - Use `realpath(3)` (that's `absolute_path_sync`'s job).
  - Add Windows backslash conversion logic — defer to per-platform `opal_path_separator()` and a future task; on Linux build, separator is `/`.
  - **Change the runtime ABI return type from `char*` to `FsPathResult`** — that requires coordinated changes to `runtime/opal_runtime.h`, `src/codegen/statements.rs:known_runtime_return_type`/`known_guard_success_type`, AND `standard_symbols_core_io_and_bytes.rs:389-477` (adding `error_types`). Out of scope for this plan; deferred.
  - **Add `error_types` to `path_from` / `normalize_path` / `join_path_components` symbol registrations** — they remain infallible.
  - Return error discriminants from these 3 helpers (no `OPAL_FS_ERR_INVALID_PATH:` strings emitted by them; `absolute_path_sync` is the entry point for path validation errors).

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: Lexical path normalization has subtle edge cases (trailing slash, `..` past root, empty components, Windows drive letters in paths). Each must be correct or downstream projects fail.
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: NO (foundation for Wave 3 projects)
  - **Blocks**: T10-T14
  - **Blocked By**: T1, T2, T3, T8

  **References**:
  - `runtime/opal_fs.c` (current 3 implementations) — replace, do not append.
  - Rust `std::path::Path::components` semantics — model behavior.
  - Python `os.path.normpath` and `os.path.join` — reference for `..` handling and absolute-component reset.
  - `runtime/opal_fs_errors.h` (T2) — discriminants.

  **Acceptance Criteria** (per-scenario RGR; each in a separate commit):
  - [ ] **path_from** (infallible — `(string) -> FilesystemPath`):
    - [ ] RED: test `path_from_handles_empty_via_sentinel` fails before fix (current identity returns "" but stdout assertion is missing).
    - [ ] GREEN: `""` → `Path("")` (empty sentinel, no error); `"hello"` → `Path("hello")`; `"hello/world"` → `Path("hello/world")` (no normalization at construction time — that's `normalize_path`'s job).
    - [ ] REFACTOR: extract `safe_strdup(const char*)` helper that handles NULL → `strdup("")`.
  - [ ] **normalize_path** (infallible — `(FilesystemPath) -> FilesystemPath`):
    - [ ] RED: test `normalize_collapses_double_slash` fails before fix.
    - [ ] GREEN: `"a//b"` → `"a/b"`; `"a/./b"` → `"a/b"`; `"a/b/.."` → `"a"`; `"a/b/../../c"` → `"c"`; `"/a/b/../../.."` → `""` (empty sentinel — root-escape case); `"./a"` → `"a"`; `""` → `""`.
    - [ ] REFACTOR: extract `lex_normalize_components` static helper (consumed by `join_path_components` too).
  - [ ] **join_path_components** (infallible — `(FilesystemPath, string[]) -> FilesystemPath`):
    - [ ] RED: test `join_handles_absolute_reset` fails before fix.
    - [ ] GREEN: `("a", ["b","c"])` → `"a/b/c"`; `("a", ["/b","c"])` → `"/b/c"` (absolute reset); `("a/", ["b"])` → `"a/b"` (no double sep); `("a", [])` → `"a"`; `("", ["x"])` → `"x"`.
    - [ ] REFACTOR: dedupe with `lex_normalize_components`.
  - [ ] `cargo test --all-features` → green.
  - [ ] MSVC clang-cl + lld-link probe → PASS.
  - [ ] All 3 helpers covered by ≥ 4 test cases each, distributed across `tests/integration_e2e/fs_path_from.rs`, `fs_normalize_path.rs`, `fs_join_path_components.rs` (separate files per builtin).
  - [ ] `runtime/opal_runtime.h` doc block (T1) updated with: empty-sentinel semantics, separator rules, `.`/`..` resolution, trailing-slash policy.
  - [ ] No new `error_types` added to `standard_symbols_core_io_and_bytes.rs` for these 3 builtins (verify via `git diff` — only `runtime/opal_fs.c` + `runtime/opal_runtime.h` doc block changes in commits).

  **QA Scenarios**:
  ```
  Scenario: path_from handles empty input via sentinel (no error emitted)
    Tool: Bash
    Steps:
      1. Run: cargo test --features integration path_from_handles_empty_via_sentinel -- --nocapture
      2. Assert: exit 0
      3. Assert: stdout shows the test program printed an empty path sentinel (e.g. "Path()" or "<empty>")
      4. Assert: stdout does NOT contain "InvalidPathError" (confirms function is infallible — no error union)
    Expected Result: empty input round-trips as empty Path; no error discriminant emitted (validation deferred to absolute_path_sync).
    Evidence: .sisyphus/evidence/task-9-empty-sentinel.log

  Scenario: normalize handles all canonical cases (happy path matrix)
    Tool: Bash
    Steps:
      1. Run: cargo test --features integration normalize_canonical_matrix -- --nocapture
      2. Assert: exit 0
      3. Assert: log contains all 7 input→output pairs from acceptance criteria above (including the empty-sentinel root-escape case)
    Expected Result: matrix passes.
    Evidence: .sisyphus/evidence/task-9-normalize-matrix.log

  Scenario: normalize root-escape returns empty sentinel (NOT an error)
    Tool: Bash
    Steps:
      1. Run: cargo test --features integration normalize_root_escape_returns_empty -- --nocapture
      2. Assert: exit 0
      3. Assert: stdout shows empty result for input "/a/b/../../.."
      4. Assert: stdout does NOT contain "InvalidPathError" or "cannot escape root" as an error discriminant (confirms infallible contract)
    Evidence: .sisyphus/evidence/task-9-root-escape.log

  Scenario: join handles absolute-component reset
    Tool: Bash
    Steps:
      1. Run: cargo test --features integration join_handles_absolute_reset -- --nocapture
      2. Assert: result == "/b/c" for input base="a", components=["/b", "c"]
    Evidence: .sisyphus/evidence/task-9-join-reset.log

  Scenario: Type-system contract preserved (no error_types added)
    Tool: Bash
    Steps:
      1. Run: git diff src/type_system/module_resolver/standard_symbols_core_io_and_bytes.rs | grep -E '(path_from|normalize_path|join_path_components)' | tee /tmp/symdiff.log
      2. Assert: file /tmp/symdiff.log is empty (no symbol changes for these 3)
      3. Run: cargo build --release 2>&1 | tail -5
      4. Assert: exit 0 (full type-check still passes)
    Expected Result: 3 helpers still infallible at type-system level; no coordinated breakage.
    Evidence: .sisyphus/evidence/task-9-symdiff.log

  Scenario: MSVC link probe still PASSes after fixes
    Tool: Bash
    Steps:
      1. Run: bash scripts/msvc_link_probe.sh
      2. Assert: exit 0; "MSVC LINK PROBE: PASS"
    Evidence: .sisyphus/evidence/task-9-msvc.log
  ```

  **Evidence to Capture**:
  - [ ] `.sisyphus/evidence/task-9-empty-reject.log`
  - [ ] `.sisyphus/evidence/task-9-normalize-matrix.log`
  - [ ] `.sisyphus/evidence/task-9-root-escape.log`
  - [ ] `.sisyphus/evidence/task-9-join-reset.log`
  - [ ] `.sisyphus/evidence/task-9-msvc.log`

  **Commit**: YES (3 commits, one per builtin, RGR-tagged)
  - Messages: `fix(runtime): path_from handles empty via sentinel`, `fix(runtime): normalize_path performs lexical normalization`, `fix(runtime): join_path_components handles absolute reset`
  - Files: `runtime/opal_fs.c`, `runtime/opal_runtime.h`, `tests/integration_e2e/fs_path_helpers.rs`, `tests/integration_e2e/tests.rs` (mod declaration — see T4 wiring contract)
  - Pre-commit: per-commit regression gate + MSVC link probe

- [x] 10. **T10: `_fs_path_from` Project Upgrade (Post-Fix Validation)**

  **What to do**:
  - Update `test-projects/_fs_path_from/src/main.op` (created in T8) to validate the FIXED behavior:
    - Drive 4 cases: empty (expect error via `guard ... else { print('error: invalid'); return void }`), simple path (`"hello"`), nested path (`"hello/world"`), trailing slash (`"hello/"`).
    - Print results.
  - Update `tests/integration_e2e/fs_path_from.rs` to assert post-fix behavior: error path printed for empty, paths printed for valid inputs.
  - Re-run smoke; FsStateGuard re-validates re-runnability.

  **Must NOT do**:
  - Add new helpers in src/.
  - Test other builtins (one major thing per project).
  - Exceed 150 LoC for src/.

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Mechanical update of existing project + test post-T9 fix.
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES with T11-T14 (independent project)
  - **Blocks**: None
  - **Blocked By**: T9

  **References**:
  - `test-projects/_fs_path_from/` (post-T8 state) — extend, do not rewrite.
  - `src/codegen/statements.rs` `guard ... else` lowering — for error-path test.

  **Acceptance Criteria**:
  - [ ] `cargo test --features integration fs_path_from_smoke` → exit 0.
  - [ ] Re-run → still passes; `target/`, `workspace/` clean.
  - [ ] `opal run test-projects/_fs_path_from/src/main.op` → exit 0; stdout contains all 4 case outputs.
  - [ ] `cargo test --all-features` → green.
  - [ ] MSVC link probe → PASS.

  **QA Scenarios**:
  ```
  Scenario: All 4 path_from cases produce expected output
    Tool: Bash
    Steps:
      1. Run: opal run test-projects/_fs_path_from/src/main.op
      2. Capture stdout
      3. Assert: contains "error: invalid" (empty case)
      4. Assert: contains "path=hello"
      5. Assert: contains "path=hello/world"
      6. Assert: contains "path=hello/" (trailing slash preserved per T1 doc)
    Evidence: .sisyphus/evidence/task-10-cases.log

  Scenario: Re-runnability after upgrade
    Tool: Bash
    Steps:
      1. Run: cargo test --features integration fs_path_from_smoke 2>&1 | tee /tmp/r1.log
      2. Run: cargo test --features integration fs_path_from_smoke 2>&1 | tee /tmp/r2.log
      3. Assert: both pass; ls workspace/ empty
    Evidence: .sisyphus/evidence/task-10-rerun.log
  ```

  **Commit**: YES
  - Message: `test(test-projects): _fs_path_from validates fixed behavior (4 cases)`
  - Files: `test-projects/_fs_path_from/src/main.op`, `tests/integration_e2e/fs_path_from.rs`
  - Pre-commit: regression gate + MSVC

- [x] 11. **T11: `_fs_normalize_path` Project (Lexical Normalization Showcase)**

  **What to do**:
  - Create `test-projects/_fs_normalize_path/`:
    - `opal.toml`, `.gitignore`, `.gitattributes`, `README.md` (T8 template).
    - `src/main.op` (≤ 80 LoC): "path-canonicalizer mini-app" — reads a list of 6 paths from a hardcoded array (`['a//b', 'a/./b', 'a/b/..', 'a/b/../../c', './a', '/a/b/../../..']`), calls `normalize_path` on each (infallible — returns `FilesystemPath`), prints `<input> -> <output>` per line. The last one demonstrates the **empty-sentinel branch**: `if normalized.is_empty() { print('{p}: <empty-sentinel root-escape>') } else { print('{p} -> {normalized}') }` (NO `guard`/`propagate` — `normalize_path` does NOT return an error union per type-system contract).
    - `src/cases.op`: small helper `let format_case = f(input: string, output: string): string => return '{input} -> {output}'`.
  - Create `tests/integration_e2e/fs_normalize_path.rs`: `#[serial(fs)]` test asserts all 6 expected output lines + the error line.
  - Wire `mod fs_normalize_path;` into `tests/integration_e2e/tests.rs` (per T4/T6 wiring contract).

  **Must NOT do**:
  - Test other builtins.
  - Exceed 150 LoC.
  - Use I/O (no fs_read_*, fs_write_*).

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
    - Reason: Multi-file Opalescent project + integration test scaffold.
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES with T10/T12/T13/T14
  - **Blocks**: None
  - **Blocked By**: T9

  **References**:
  - `test-projects/_fs_path_from/` (post-T10) — closest project template.
  - `src/codegen/statements.rs` propagate/guard lowering.

  **Acceptance Criteria**:
  - [ ] All 7 files exist (opal.toml, .gitignore, .gitattributes, README.md, src/main.op, src/cases.op, plus integration test).
  - [ ] `opal run test-projects/_fs_normalize_path/src/main.op` → exit 0; stdout contains 6 `<input> -> <output>` lines + 1 error line.
  - [ ] `cargo test --features integration fs_normalize_path` → exit 0; FsStateGuard restores cleanly.
  - [ ] Re-run → identical result; src/ unchanged.
  - [ ] `cargo test --all-features` → green.
  - [ ] MSVC link probe → PASS.

  **QA Scenarios**:
  ```
  Scenario: All 6 happy-path cases + 1 error case produce documented output
    Tool: Bash
    Steps:
      1. Run: opal run test-projects/_fs_normalize_path/src/main.op
      2. Capture stdout
      3. Assert: contains "a//b -> a/b"
      4. Assert: contains "a/./b -> a/b"
      5. Assert: contains "a/b/.. -> a"
      6. Assert: contains "a/b/../../c -> c"
      7. Assert: contains "./a -> a"
      8. Assert: contains "/a/b/../../.." (and " error" or similar marker)
    Evidence: .sisyphus/evidence/task-11-cases.log

  Scenario: Re-runnability + workspace clean
    Tool: Bash
    Steps:
      1. Run: cargo test --features integration fs_normalize_path 2>&1 | tee /tmp/n1.log
      2. Run: cargo test --features integration fs_normalize_path 2>&1 | tee /tmp/n2.log
      3. Assert: both pass; ls workspace/ empty
    Evidence: .sisyphus/evidence/task-11-rerun.log
  ```

  **Commit**: YES
  - Message: `test(test-projects): add _fs_normalize_path lexical canonicalizer`
  - Files: `test-projects/_fs_normalize_path/**`, `tests/integration_e2e/fs_normalize_path.rs`, `tests/integration_e2e/tests.rs` (mod declaration — see T4 wiring contract)
  - Pre-commit: regression gate + MSVC

- [x] 12. **T12: `_fs_join_path_components` Project (Path Builder Mini-App)**

  **What to do**:
  - Create `test-projects/_fs_join_path_components/`:
    - Standard 4 root files (T8 template).
    - `src/main.op`: build paths from base + arrays. 5 cases (per locked T9 contract — `join_path_components(base, parts[])`, infallible): `("home", ["user", "docs"])` → `"home/user/docs"`; `("a/", ["b"])` → `"a/b"` (no double sep); `("a", ["/b", "c"])` → `"/b/c"` (absolute reset); `("a", [])` → `"a"` (empty parts → base unchanged); `("", ["x"])` → `"x"` (empty base).
    - `src/builder.op`: helper `let build_doc_path = f(base: string, parts: string[]): string => return join_path_components(base, parts)` (NO `errors`, NO `propagate` — function is infallible per `standard_symbols_core_io_and_bytes.rs:403`).
  - Integration test `tests/integration_e2e/fs_join_path_components.rs` mirrors T11 structure.
  - Wire mod into `tests/integration_e2e/tests.rs` (per T4/T6 wiring contract — NOT `tests/integration_e2e.rs`).

  **Must NOT do**:
  - Test other builtins.
  - Exceed 150 LoC.

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES with T10/T11/T13/T14
  - **Blocked By**: T9

  **References**:
  - `test-projects/_fs_normalize_path/` (T11) — same template.
  - `src/type_system/checker/fs_builtins.rs` — `InvalidPathError` error type.

  **Acceptance Criteria**:
  - [ ] Project files exist.
  - [ ] `opal run` produces all 4 case outputs.
  - [ ] `cargo test --features integration fs_join_path_components` → green twice in a row.
  - [ ] `cargo test --all-features` → green.
  - [ ] MSVC link probe → PASS.

  **QA Scenarios**:
  ```
  Scenario: Build doc path + absolute reset case
    Tool: Bash
    Steps:
      1. Run: opal run test-projects/_fs_join_path_components/src/main.op 2>&1 | tee /tmp/qa.log; echo "EXIT=${PIPESTATUS[0]}"
      2. Assert: /tmp/qa.log contains "home/user/docs"
      3. Assert: stdout contains "/b/c"
      4. Assert: stdout contains "a/b" (trailing-slash dedupe)
      5. Assert: stdout contains "empty" or error marker for [] case
    Evidence: .sisyphus/evidence/task-12-cases.log

  Scenario: Edge case — empty parts array returns base unchanged (infallible)
    Tool: Bash
    Preconditions: T9 locks `join_path_components` infallible per `standard_symbols_core_io_and_bytes.rs:403`; no error union in signature
    Steps:
      1. Run: cargo test --features integration fs_join_path_components_empty 2>&1 | tee .sisyphus/evidence/task-12-empty.log
      2. Assert: exit 0 AND output contains "1 passed"
      3. Run: cargo test --features integration fs_join_path_components_empty_base 2>&1 | tee .sisyphus/evidence/task-12-empty-base.log
      4. Assert: exit 0 AND output contains "1 passed"
    Expected Result: Two infallible empty-input scenarios both succeed: (a) `("a", [])` → `"a"` (base unchanged), (b) `("", ["x"])` → `"x"` (empty base + non-empty parts). No error path exists — function signature has no `errors` clause.
    Evidence: .sisyphus/evidence/task-12-empty.log + task-12-empty-base.log
  ```

  **Commit**: YES
  - Message: `test(test-projects): add _fs_join_path_components builder app`
  - Files: `test-projects/_fs_join_path_components/**`, integration test, mod export
  - Pre-commit: regression gate + MSVC

- [x] 13. **T13: `_fs_path_helpers_query` Project (Covers `file_extension`/`file_name`/`parent_directory`)**

  **What to do**:
  - Create `test-projects/_fs_path_helpers_query/` covering the 3 query helpers (already real per research):
    - `src/main.op`: small "file inspector" mini-app. Given a hardcoded array of 5 paths (`['/home/user/doc.pdf', '/home/user/', 'noext', 'a/b/c.tar.gz', '/']`), for each path: extract extension via `fs_path_file_extension`, name via `fs_path_file_name`, parent via `fs_path_parent_directory`. Print as `<path>: ext=<>, name=<>, parent=<>`.
    - `src/inspect.op`: helper `let inspect = f(p: Path): string => ...` formatting the line.
  - Integration test asserts 5 expected lines covering: typical path, trailing slash, no extension, multi-dot extension (returns last `.ext`), root.
  - Wire mod.

  **Must NOT do**:
  - Modify the 3 helpers (they work; just exercise them).
  - Touch `absolute_path_sync` (separate task T14).
  - Exceed 150 LoC.

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES with T10/T11/T12/T14
  - **Blocked By**: T9 (uses fixed `path_from` in input construction)

  **References**:
  - `runtime/opal_fs.c` (the 3 working impls) — confirm behavior with multi-dot extension and trailing slash.
  - `src/type_system/checker/fs_builtins.rs` — return types.

  **Acceptance Criteria**:
  - [ ] All 5 input cases covered with expected outputs.
  - [ ] Test re-run → green; src/ unchanged.
  - [ ] `cargo test --all-features` → green.
  - [ ] MSVC link probe → PASS.

  **QA Scenarios**:
  ```
  Scenario: file inspector matrix (5 cases)
    Tool: Bash
    Steps:
      1. Run: opal run test-projects/_fs_path_helpers_query/src/main.op
      2. Assert: line "/home/user/doc.pdf: ext=pdf, name=doc.pdf, parent=/home/user"
      3. Assert: line for "/home/user/" (trailing slash) — name behavior per runtime impl
      4. Assert: line for "noext" — ext empty
      5. Assert: line for "a/b/c.tar.gz" — ext=gz (NOT tar.gz)
      6. Assert: line for "/" — parent="/" or empty per documented behavior
    Evidence: .sisyphus/evidence/task-13-matrix.log

  Scenario: Re-runnability
    Tool: Bash
    Steps:
      1. Run twice; assert both green; workspace clean
    Evidence: .sisyphus/evidence/task-13-rerun.log
  ```

  **Commit**: YES
  - Message: `test(test-projects): add _fs_path_helpers_query inspector`
  - Files: project + test + mod
  - Pre-commit: regression gate + MSVC

- [x] 14. **T14: `_absolute_path_sync` Project + Resolve Edge Cases**

  **What to do**:
  - Create `test-projects/_absolute_path_sync/`:
    - `src/main.op`: "path resolver" mini-app. Resolves 4 inputs: a relative path that EXISTS (`./README.md` relative to project), a relative path that does NOT exist (must still resolve lexically to absolute or return error per current impl — document behavior), a path with `..` (`./src/../README.md`), an already-absolute path. Prints `<input> -> <absolute>` or `<input>: error`.
    - `src/resolver.op`: helper.
  - **Document** in README.md the exact semantics (does it require existence? — confirm via reading current `runtime/opal_fs.c::absolute_path_sync`).
  - Integration test: pre-creates a fixture file in `workspace/` (preserved by FsStateGuard since workspace is wipe-only — actually NO: FsStateGuard wipes workspace. So integration test creates the fixture INSIDE the test setup AFTER guard creation, in a way that's fine because guard restores the manifest, not workspace contents).
  - Wait — adjust: Integration test uses `tempfile::tempdir()` for the fixture (since workspace is wiped), OR test against `src/main.op` itself (already exists, manifest-tracked).
  - Resolve approach: test resolves `./src/main.op` (manifest-tracked → guaranteed present) and `./does_not_exist.txt` (assert error or absolute-path-without-existence-check per current impl).

  **Must NOT do**:
  - Touch `realpath` semantics in runtime (use existing impl as-is).
  - Use `tempfile`.
  - Exceed 150 LoC.

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: Behavior depends on whether current `absolute_path_sync` requires existence; must read runtime impl first and document accurately. Edge case-heavy.
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES with T10/T11/T12/T13
  - **Blocked By**: T9

  **References**:
  - `runtime/opal_fs.c::absolute_path_sync` — current impl (uses `realpath`).
  - POSIX `realpath(3)` — fails on non-existent paths.
  - `runtime/opal_portability.h::opal_realpath` (T3) — Windows `_fullpath` does NOT require existence.

  **WHY**:
  - This task surfaces a portability divergence: POSIX `realpath` requires existence, Windows `_fullpath` does not. Either decide here to normalize behavior in `opal_realpath` shim (T3 update), or document and accept the divergence. Likely outcome: Update T3's `opal_realpath` to lexically resolve when path doesn't exist (avoid the POSIX/Windows divergence).

  **Acceptance Criteria**:
  - [ ] Project files exist.
  - [ ] `opal run` produces 4 expected outputs (per documented behavior).
  - [ ] If POSIX/Windows divergence found: T3 updated with lexical-fallback in `opal_realpath` shim, AND a comment documents the unified behavior, AND link probe re-passes.
  - [ ] Test re-runs cleanly.
  - [ ] `cargo test --all-features` → green.
  - [ ] MSVC link probe → PASS.

  **QA Scenarios**:
  ```
  Scenario: Resolve manifest-tracked file (happy path)
    Tool: Bash
    Preconditions: test-projects/_absolute_path_sync/src/main.op exists
    Steps:
      1. Run: cd test-projects/_absolute_path_sync && opal run src/main.op
      2. Capture stdout
      3. Assert: contains absolute path ending in "src/main.op"
    Evidence: .sisyphus/evidence/task-14-resolve.log

  Scenario: Non-existent path behavior documented and consistent
    Tool: Bash
    Steps:
      1. Run: cargo test --features integration absolute_path_sync_nonexistent -- --nocapture
      2. Assert: behavior matches README documentation (either error OR lexical absolute)
    Evidence: .sisyphus/evidence/task-14-nonexist.log

  Scenario: MSVC link probe + unified shim
    Tool: Bash
    Steps:
      1. Run: bash scripts/msvc_link_probe.sh
      2. Assert: PASS
      3. Run: grep "_fullpath\|realpath" runtime/opal_portability.h
      4. Assert: a comment exists explaining unified behavior or divergence
    Evidence: .sisyphus/evidence/task-14-msvc.log
  ```

  **Commit**: YES (possibly 2 commits: project + portability fix if divergence found)
  - Messages: `test(test-projects): add _absolute_path_sync resolver` (+ optional `fix(runtime): unify opal_realpath behavior across POSIX/Windows`)
  - Files: project + test + possibly runtime/opal_portability.h
  - Pre-commit: regression gate + MSVC link probe

## Wave 4 — Read Family (5 builtins, ~11 tasks)

> Per Metis: lock policy decisions BEFORE Wave 4 starts.
> **Policy locks** (executor must follow these — non-negotiable):
> - **`read_lines_sync` trailing newline**: Lines are split on `\n`. A trailing newline does NOT produce an extra empty element. Empty file → `[]`. CRLF line endings normalized to LF before splitting (matches `strip_crlf` helper).
> - **`read_first_line_sync` empty file**: Returns error `OPAL_FS_ERR_NOT_FOUND: file is empty` (or alternative discriminant `OPAL_FS_ERR_OUT_OF_BOUNDS: file is empty` — pick one in T15 contract update and document in opal_runtime.h).
> - **`read_file_to_string_sync` non-UTF-8 input**: Returns error `OPAL_FS_ERR_INVALID_UTF8: <byte offset>`.
> - **All read funcs**: FileNotFoundError vs PermissionDeniedError distinguished via `errno` (ENOENT/EACCES); IsADirectoryError via `EISDIR` or post-stat check.

- [x] 15. **T15: Replace `read_text_sync` Stub (Real POSIX Impl + UTF-8 Validation)**

  **What to do**:
  - In `runtime/opal_fs.c`, replace `read_text_sync` stub:
    - `fopen(path, "rb")`; on NULL: branch on `errno` → `ENOENT` → `FileNotFoundError`, `EACCES` → `PermissionDeniedError`, `EISDIR` → `IsADirectoryError`, else → `Io: <strerror(errno)>`.
    - `fseek(SEEK_END)` + `ftell` for size; `rewind`; `malloc(size + 1)`; `fread`; null-terminate.
    - Validate UTF-8: walk bytes, return error `OPAL_FS_ERR_INVALID_UTF8: <byte offset>` on first malformed sequence (use a small inline validator — RFC 3629 — or extract to static helper).
    - On success: return `FsStringResult { value = buf, error = NULL }`.
    - Free `buf` on error path.
  - Add static helper `static int validate_utf8(const char* buf, size_t len, size_t* err_offset)`.
  - Per-RGR commits per error case (FileNotFoundError, PermissionDeniedError, IsADirectoryError, InvalidUtf8Error, success) — 5 commits.

  **Must NOT do**:
  - Use `mmap` (deferred; portable pure-stdio for first impl).
  - Cap file size (defer to a later policy task; document this in T1 doc block).
  - Touch other read funcs (T16-T18).

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: First real I/O impl; sets the error-translation pattern for all other Read-family stubs. UTF-8 validation has subtle edge cases (overlong encodings, surrogate halves).
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: NO (sets pattern for T16-T18; do sequentially)
  - **Blocks**: T16, T17, T18, T19 (showcase project)
  - **Blocked By**: T1, T2, T3, T9 (path normalization), T8 (smoke validates pipeline)

  **References**:
  - `runtime/opal_fs.c::read_text_sync` (current stub) — replace.
  - `runtime/opal_fs_errors.h` (T2) — discriminants.
  - RFC 3629 — UTF-8 byte sequence rules.
  - POSIX `errno.h` — error codes.
  - `runtime/opal_portability.h::opal_stat` (T3) — for IsADirectoryError pre-check.

  **Acceptance Criteria** (per-scenario RGR; 5 commits):
  - [ ] **FileNotFoundError** RED-GREEN: test reads `nonexistent.txt` → asserts `FileNotFoundError` error.
  - [ ] **PermissionDeniedError** RED-GREEN: test reads `/etc/shadow` (or chmod 000 fixture) → asserts `PermissionDeniedError`.
  - [ ] **IsADirectoryError** RED-GREEN: test reads `./src` → asserts `IsADirectoryError`.
  - [ ] **InvalidUtf8Error** RED-GREEN: test reads a fixture with byte 0xFF → asserts `InvalidUtf8Error: 0`.
  - [ ] **Success** RED-GREEN: test reads `README.md` of self-test fixture → returns expected string content (sha256 match).
  - [ ] REFACTOR: extract `errno_to_fs_error(int err, const char* op_prefix) -> char*` helper (heap-allocated via `opal_fs_format_err` from T2) for reuse in T16-T18+; audit existing static-literal error strings touched and convert to heap-allocated to honor uniform-free contract.
  - [ ] `cargo test --all-features` → green.
  - [ ] MSVC link probe → PASS.

  **QA Scenarios**:
  ```
  Scenario: FileNotFoundError (failure path)
    Tool: Bash
    Steps:
      1. Run: cargo test --features integration read_file_to_string_not_found -- --nocapture
      2. Assert: error string starts with "FileNotFoundError:"
    Evidence: .sisyphus/evidence/task-15-notfound.log

  Scenario: PermissionDeniedError (failure path)
    Tool: Bash
    Preconditions: a chmod-000 fixture under tests/fixtures/perm_denied.txt (created in test setup, restored by guard)
    Steps:
      1. Run: cargo test --features integration read_file_to_string_perm_denied
      2. Assert: error string starts with "PermissionDeniedError:"
    Evidence: .sisyphus/evidence/task-15-perm.log

  Scenario: IsADirectoryError (failure path)
    Tool: Bash
    Steps:
      1. Run: cargo test --features integration read_file_to_string_is_directory
      2. Assert: error string starts with "IsADirectoryError:"
    Evidence: .sisyphus/evidence/task-15-isdir.log

  Scenario: InvalidUtf8Error (failure path)
    Tool: Bash
    Preconditions: tests/fixtures/invalid_utf8.bin contains byte 0xFF (committed with .gitattributes -text)
    Steps:
      1. Run: cargo test --features integration read_file_to_string_invalid_utf8
      2. Assert: error string starts with "InvalidUtf8Error:"
    Evidence: .sisyphus/evidence/task-15-utf8.log

  Scenario: Success — sha256 match
    Tool: Bash
    Steps:
      1. Run: cargo test --features integration read_file_to_string_success -- --nocapture
      2. Assert: stdout sha256 of returned string matches sha256 of fixture
    Evidence: .sisyphus/evidence/task-15-success.log
  ```

  **Evidence to Capture**: 5 logs above + `.sisyphus/evidence/task-15-msvc.log`

  **Commit**: YES (5 commits, RGR-tagged)
  - Messages: `feat(runtime): read_file_to_string handles FileNotFoundError`, `... PermissionDeniedError`, `... IsADirectoryError`, `... InvalidUtf8Error`, `... success path`
  - Files: `runtime/opal_fs.c`, `runtime/opal_runtime.h`, `tests/fixtures/**` (UTF-8 fixtures), `tests/integration_e2e/fs_read_file.rs`
  - Pre-commit: per-commit regression gate + MSVC

- [x] 16. **T16: Replace `read_contents_sync` Stub (Binary Read)**

  **What to do**:
  - Replace stub: same skeleton as T15 minus UTF-8 validation; returns `FsBytesResult { value, count, error }`.
  - Reuse `errno_to_fs_error` helper from T15.
  - Per-RGR: FileNotFoundError, PermissionDeniedError, IsADirectoryError, Success (4 commits).

  **Must NOT do**:
  - Validate UTF-8 (binary read).
  - Add new error helpers (reuse T15's).
  - Touch other stubs.

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
    - Reason: Mirrors T15 with simpler return type; reuse pattern.
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: NO (sequential after T15 to lock pattern)
  - **Blocks**: T19, T26 (round-trip uses bytes)
  - **Blocked By**: T15

  **References**:
  - `runtime/opal_fs.c::read_contents_sync` — replace.
  - T15's `errno_to_fs_error` helper.
  - `FsBytesResult` struct — value pointer + count field.

  **Acceptance Criteria**:
  - [ ] 4 error/success cases pass (FileNotFoundError, Permission, IsADirectoryError, Success).
  - [ ] Reads binary fixture (e.g., 0x00..0xFF byte sequence) and returns exactly 256 bytes (sha256 match).
  - [ ] Empty file → returns `value = malloc(0)` (or NULL with count=0; document and pick one) with `error = NULL`.
  - [ ] `cargo test --all-features` → green.
  - [ ] MSVC link probe → PASS.

  **QA Scenarios**:
  ```
  Scenario: Read 256-byte fixture (success)
    Tool: Bash
    Preconditions: tests/fixtures/256bytes.bin = 0x00..0xFF (256 bytes, .gitattributes -text binary)
    Steps:
      1. Run: cargo test --features integration read_file_to_bytes_256_success -- --nocapture
      2. Assert: count == 256
      3. Assert: sha256 matches expected
    Evidence: .sisyphus/evidence/task-16-256.log

  Scenario: Empty file edge case
    Tool: Bash
    Steps:
      1. Run: cargo test --features integration read_file_to_bytes_empty
      2. Assert: count == 0; no error
    Evidence: .sisyphus/evidence/task-16-empty.log

   Scenario: FileNotFoundError + PermissionDeniedError + IsADirectoryError (matrix)
     Tool: Bash
     Steps:
       1. Run: cargo test --features integration read_file_to_bytes_not_found
       2. Assert: exit 0 AND output contains "1 passed"
       3. Run: cargo test --features integration read_file_to_bytes_perm
       4. Assert: exit 0 AND output contains "1 passed"
       5. Run: cargo test --features integration read_file_to_bytes_isdir
       6. Assert: exit 0 AND output contains "1 passed"
     Evidence: .sisyphus/evidence/task-16-errors.log
  ```

  **Commit**: YES (4 commits)
  - Messages: `feat(runtime): read_file_to_bytes handles FileNotFoundError`, `... PermissionDeniedError`, `... IsADirectoryError`, `... success path`
  - Files: `runtime/opal_fs.c`, `tests/fixtures/256bytes.bin`, `tests/integration_e2e/fs_read_bytes.rs`
  - Pre-commit: regression gate + MSVC

- [x] 17. **T17: Replace `read_lines_sync` Stub (Line Splitting + Policy)**

  **What to do**:
  - Replace stub. Internally read whole file (reuse `read_file_to_string_sync` machinery — extract a static `read_to_buf` helper), validate UTF-8, then split:
    - Normalize CRLF to LF (`strip_crlf` semantics in C: skip `\r` if followed by `\n`).
    - Split on `\n`.
    - Trailing `\n` does NOT produce extra empty element (per locked policy).
    - Empty file → `count = 0`, `value = NULL`.
  - Returns `FsStringArrayResult { char** value; int64_t count; char* error }` (per `runtime/opal_runtime.h:92`; distinct from `FsPathArrayResult` used by `list_directory_sync` at `:91`).
  - Allocation pattern: single contiguous buffer for line data + separate `char**` index array (document in opal_runtime.h doc block).

  **Must NOT do**:
  - Treat `\r` alone as a line separator (only `\n` and `\r\n`).
  - Limit line count or line length.

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: String array allocation/freeing is error-prone; CRLF normalization spec must match `strip_crlf`.
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: NO (uses extracted helper from T15)
  - **Blocks**: T18 (first_line uses lines), T19
  - **Blocked By**: T15

  **References**:
  - `tests/integration_e2e/fs_helpers.rs::strip_crlf` (created in T6) — line-ending semantics.
  - T15's `read_to_buf` (extract).
  - `FsStringArrayResult` struct (per `runtime/opal_runtime.h:92`).

  **Acceptance Criteria** (per-scenario RGR; 5 commits):
  - [ ] **Empty file** → count == 0, no error.
  - [ ] **LF only** → `"a\nb\nc\n"` → `["a","b","c"]` (trailing \n consumed).
  - [ ] **No trailing LF** → `"a\nb\nc"` → `["a","b","c"]`.
  - [ ] **CRLF** → `"a\r\nb\r\n"` → `["a","b"]`.
  - [ ] **InvalidUtf8Error** → propagated from validator with offset.
  - [ ] `cargo test --all-features` → green.
  - [ ] MSVC link probe → PASS.

  **QA Scenarios**:
  ```
  Scenario: All 5 line-split cases pass
    Tool: Bash
    Steps:
      1. Run: cargo test --features integration read_lines_matrix -- --nocapture
      2. Assert: 5 sub-tests pass; line counts match expected
    Evidence: .sisyphus/evidence/task-17-matrix.log

  Scenario: CRLF fixture preservation (verifies .gitattributes works)
    Tool: Bash
    Preconditions: tests/fixtures/crlf_lines.crlf.txt (committed with -text attr; raw bytes contain \r\n)
    Steps:
      1. Run: xxd tests/fixtures/crlf_lines.crlf.txt | head -1
      2. Assert: output contains "0d0a" (CR-LF)
      3. Run: cargo test --features integration read_lines_crlf
      4. Assert: pass; lines split correctly with no \r in elements
    Evidence: .sisyphus/evidence/task-17-crlf.log
  ```

  **Commit**: YES (5 commits)
  - Files: `runtime/opal_fs.c`, `runtime/opal_runtime.h`, `tests/fixtures/crlf_lines.crlf.txt`, `tests/integration_e2e/fs_read_lines.rs`
  - Pre-commit: regression gate + MSVC

- [x] 18. **T18: ADD `read_first_line_sync` as NEW Builtin (End-to-End: header + impl + codegen + symbol registry + prelude)**

  > **SCOPE NOTE**: Unlike other Wave 4 tasks, T18 is NOT a stub replacement — `read_first_line_sync` does NOT currently exist anywhere in the codebase (no entry in `runtime/opal_runtime.h`, no stub in `runtime/opal_fs.c`, no registration in any `standard_symbols_*.rs`, no codegen wiring). T18 must wire this builtin end-to-end across all 5 layers. Scope explicitly approved by user as a new builtin to bring the set to "34 existing + 1 new = 35".

  **What to do**:
  1. **Runtime header** (`runtime/opal_runtime.h`): Add export declaration `FsStringResult read_first_line_sync(const char* path);` near other read_* exports (~line 99); follow existing comment/grouping conventions from T1.
  2. **Runtime impl** (`runtime/opal_fs.c`): Add NEW function (not stub-replace — there is no stub):
     - Streaming impl: `fopen(path, "rb")`; loop `fgetc` collecting bytes until `\n` or EOF; if `\r\n` sequence detected, drop the `\r`.
     - Empty file → return error per locked policy (`OPAL_FS_ERR_OUT_OF_BOUNDS: file is empty`); document in `runtime/opal_fs_errors.h`.
     - File with only `"\n"` → returns `""` (empty first line) with no error.
     - Validate UTF-8 of the returned line only (NOT the rest of the file).
     - Reuse `errno_to_fs_error` helper from T15.
  3. **Symbol registry** (`src/type_system/module_resolver/standard_symbols_filesystem_operations.rs`): Register `String::from("read_first_line_sync")` builtin entry with signature `(FilesystemPath) -> string` and error union per locked discriminant policy (`FileNotFoundError | PermissionDeniedError | IsADirectoryError | InvalidUtf8Error | OffsetOutOfRangeError | ReadFailureError`); follow existing entry conventions in the file.
  4. **Codegen wiring** (`src/codegen/functions_stdlib.rs` + `src/codegen/statements.rs`): Add `read_first_line_sync` arm matching the pattern used for sibling `read_text_sync` / `read_lines_sync`; LLVM extern decl + return-shape unpacking via `FsStringResult` discipline.
  5. **Prelude** (`stdlib/prelude.op`): Add `read_first_line_sync` entry under the `## fs` section added by T7.

  **Must NOT do**:
  - Read the whole file (defeats the point of `first_line`).
  - Allocate huge buffer upfront — grow dynamically (start 256B, double on overflow, cap at 16MB to avoid runaway).
  - Touch unrelated read_* functions.
  - Skip ANY of the 5 wiring layers (compile will fail or builtin will be unresolvable).
  - Use `fs_` prefix on the symbol — canonical naming per `standard_symbols_*.rs` convention is no-prefix `read_first_line_sync`.

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: NO (sequential within Wave 4 to share helpers)
  - **Blocks**: T19
  - **Blocked By**: T15, T17

  **References**:
  - T15's helper functions (`errno_to_fs_error`, `read_to_buf`).
  - `runtime/opal_runtime.h:96-99` — sibling read_* declarations to follow for naming + struct return type (`FsStringResult`).
  - `runtime/opal_fs.c` — sibling read_* impls (e.g. `read_text_sync` after T15) for fopen/error/UTF-8 validation patterns.
  - `src/type_system/module_resolver/standard_symbols_filesystem_operations.rs` — sibling `read_*_sync` registrations to follow.
  - `src/codegen/functions_stdlib.rs:389-398` (extern decl arm) and `:502` (registry array entry) — pattern for adding new fs builtin (study `is_directory_sync` as canonical example).
  - `src/type_system/checker/fs_builtins.rs:29-48` — error nominal type names to use in the error union.

  **Acceptance Criteria** (per-scenario RGR; 4 commits):
  - [ ] **Empty file** → error per policy.
  - [ ] **Single-line file no LF** → returns content as-is.
  - [ ] **Multi-line LF** → returns first line only.
  - [ ] **Multi-line CRLF** → returns first line without `\r`.
  - [ ] **FileNotFoundError + PermissionDeniedError + IsADirectoryError** → propagate from helper.
  - [ ] `cargo test --all-features` → green.
  - [ ] MSVC link probe → PASS.

  **QA Scenarios**:
  ```
  Scenario: Empty file → policy-locked error
    Tool: Bash
    Steps:
      1. Run: cargo test --features integration read_first_line_empty
      2. Assert: error contains "file is empty" or matches OPAL_FS_ERR_OUT_OF_BOUNDS prefix
    Evidence: .sisyphus/evidence/task-18-empty.log

  Scenario: CRLF first line stripped correctly
    Tool: Bash
    Preconditions: tests/fixtures/crlf_lines.crlf.txt from T17
    Steps:
      1. Run: cargo test --features integration read_first_line_crlf
      2. Assert: returned string has no trailing \r
    Evidence: .sisyphus/evidence/task-18-crlf.log

   Scenario: Streaming — first-line read completes in bounded time on large file
     Tool: Bash
     Preconditions: tempfile fixture (NOT committed) generated by Rust test setup; not in test-projects/
     Steps:
       1. Run: cargo test --features integration read_first_line_streaming_bounded -- --nocapture 2>&1 | tee .sisyphus/evidence/task-18-streaming.log
       2. Assert: exit 0 AND output contains "1 passed"
     Expected Result: Rust test internally writes a 10MB tempfile with `\n` at byte 100, calls `read_first_line_sync`, asserts wall-clock duration < 50ms (panics if exceeded).
     Evidence: .sisyphus/evidence/task-18-streaming.log
  ```

  **Commit**: YES (4 commits per RGR)
  - Files: `runtime/opal_fs.c`, `tests/integration_e2e/fs_read_first_line.rs`, fixtures
  - Pre-commit: regression gate + MSVC

- [x] 19. **T19: `_fs_read_text_lines` Project (Read-Family Showcase Mini-App)**

  **What to do**:
  - Create `test-projects/_fs_read_text_lines/`:
    - Standard 4 root files.
    - `fixtures/sample.txt` (committed with `.gitattributes -text`): 4 lines, mixed LF/CRLF.
    - `src/main.op` (≤ 120 LoC): "log-summary" mini-app. Reads `fixtures/sample.txt` via `read_lines_sync`, prints line count, reads first line via `read_first_line_sync`, reads whole file via `read_text_sync` and asserts via runtime `==` that joining lines + `\n` reproduces the content (modulo trailing-newline policy).
    - `src/summary.op`: helper functions.
    - On any error (FileNotFoundError etc.), use `guard ... else` and print error discriminant.
  - Integration test: assert stdout contains "lines=4", "first=<expected>", "match=true".

  **Must NOT do**:
  - Test write/append (separate wave).
  - Test other read funcs (e.g., bytes — separate project).
  - Exceed 150 LoC.

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: NO (depends on T15+T17+T18)
  - **Blocks**: None
  - **Blocked By**: T15, T17, T18

  **References**:
  - `test-projects/_fs_path_from/` template (T8, T10).
  - `tests/fixtures/crlf_lines.crlf.txt` from T17 (model for fixtures).

  **Acceptance Criteria**:
  - [ ] All files exist; fixture committed with `-text` attr (verify via `git check-attr`).
  - [ ] `opal run` produces "lines=4", first-line, and "match=true".
  - [ ] Re-run → identical output; workspace clean.
  - [ ] `cargo test --features integration fs_read_text_lines` → green twice.
  - [ ] `cargo test --all-features` → green.
  - [ ] MSVC link probe → PASS.

  **QA Scenarios**:
  ```
  Scenario: Read-family integration (happy path)
    Tool: Bash
    Steps:
      1. Run: opal run test-projects/_fs_read_text_lines/src/main.op 2>&1 | tee /tmp/qa.log; echo "EXIT=${PIPESTATUS[0]}"
      2. Assert: /tmp/qa.log contains "lines=4"
      3. Assert: stdout contains "match=true"
    Evidence: .sisyphus/evidence/task-19-readfam.log

  Scenario: Re-runnability with mutated workspace
    Tool: Bash
    Steps:
      1. mkdir -p test-projects/_fs_read_text_lines/workspace && touch test-projects/_fs_read_text_lines/workspace/junk
      2. cargo test --features integration fs_read_text_lines
      3. Assert: pass; workspace empty after
    Evidence: .sisyphus/evidence/task-19-rerun.log

  Scenario: Fixture preserves CRLF on Linux checkout
    Tool: Bash
    Steps:
      1. Run: git check-attr text -- test-projects/_fs_read_text_lines/fixtures/sample.txt
      2. Assert: output contains "unset"
      3. Run: file test-projects/_fs_read_text_lines/fixtures/sample.txt
      4. Assert: output mentions "with CRLF" or similar (xxd check fallback)
    Evidence: .sisyphus/evidence/task-19-crlf-attr.log
  ```

  **Commit**: YES
  - Message: `test(test-projects): add _fs_read_text_lines log-summary mini-app`
  - Files: project + integration test + mod
  - Pre-commit: regression gate + MSVC

## Wave 5 — Write/Append Family (4 builtins, ~5 tasks)

> Per Metis: write semantics must be locked BEFORE Wave 5.
> **Policy locks** (executor must follow):
> - **`write_file_string_sync` / `write_file_bytes_sync`**: Truncate-and-replace. Creates file with mode 0644 if missing. Parent directory MUST exist (returns `FileNotFoundError` if not). Atomicity: NOT guaranteed in v1 (document for users; T23 showcases atomic-write pattern via temp+rename).
> - **`append_file_string_sync`**: Opens with `"ab"`, writes, closes. Creates file with 0644 if missing. Parent must exist.
> - **UTF-8**: `write_file_string_sync` and `append_file_string_sync` accept any byte sequence the runtime passes (the type system already enforces String at the language level — runtime treats it as raw bytes).
> - **Disk-full / quota**: Surface as `Io: <strerror>` (e.g., ENOSPC, EDQUOT). Document.
> - **Symlink target**: Follow symlinks (default `fopen` behavior). No special handling.

- [x] 20. **T20: Replace `write_text_sync` Stub (Truncate + Replace)**

  **What to do**:
  - In `runtime/opal_fs.c`, replace `write_text_sync` stub:
    - `fopen(path, "wb")`; on NULL: `errno_to_fs_error` (FileNotFoundError for missing parent, PermissionDeniedError, IsADirectoryError).
    - `fwrite(content, 1, len, f)`; on short write: error `Io: short write (<wrote>/<expected>)`.
    - `fclose`; on error: `Io: close failed: <strerror>`.
    - On success: return `FsResult { value = NULL, error = NULL }`.
  - Per-RGR: FileNotFoundError (missing parent), PermissionDeniedError, IsADirectoryError, DiskFull (simulated via /dev/full on Linux), Success — 5 commits.

  **Must NOT do**:
  - Implement atomic write (temp+rename) here — that's T23's showcase.
  - Add fsync (defer to a later policy task; document).
  - Return partial success on short write.

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: Sets write-family error pattern; disk-full edge case is subtle.
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: NO (sequential; sets pattern for T21, T22)
  - **Blocks**: T21, T22, T23, T24, T26 (round-trip)
  - **Blocked By**: T15 (errno_to_fs_error helper)

  **References**:
  - `runtime/opal_fs.c::write_text_sync` — replace.
  - T15's `errno_to_fs_error` helper.
  - `/dev/full` Linux special device for ENOSPC simulation.
  - POSIX `errno.h`, `fwrite(3)`.

  **Acceptance Criteria** (per-scenario RGR; 5 commits):
  - [ ] **FileNotFoundError (missing parent)**: `write("/nonexistent/dir/file.txt", "x")` → `FileNotFoundError` error.
  - [ ] **PermissionDeniedError**: `write("/root/file.txt", "x")` (or chmod-555 dir fixture) → `PermissionDeniedError`.
  - [ ] **IsADirectoryError**: `write("./tests", "x")` → `IsADirectoryError`.
  - [ ] **DiskFull**: `write("/dev/full", "x")` → `Io: ` error containing "No space left" or ENOSPC translation.
  - [ ] **Success**: `write(tmp, "hello\n")` → file contents exactly `"hello\n"` (sha256 match).
  - [ ] REFACTOR: extract `fwrite_all(f, buf, len)` helper for reuse in T21/T22.
  - [ ] `cargo test --all-features` → green.
  - [ ] MSVC link probe → PASS.

  **QA Scenarios**:
  ```
  Scenario: Success — round-trip via shell
    Tool: Bash
    Steps:
      1. Run: cargo test --features integration write_file_string_success -- --nocapture
      2. Assert: test passes; fixture file's sha256 matches expected
    Evidence: .sisyphus/evidence/task-20-success.log

   Scenario: FileNotFoundError + PermissionDeniedError + IsADirectoryError matrix
     Tool: Bash
     Preconditions: tests/fixtures/perm_denied_dir/ chmod 000 (created in setup, restored in guard Drop)
     Steps:
       1. Run: cargo test --features integration write_file_string_not_found
       2. Assert: exit 0 AND output contains "1 passed"
       3. Run: cargo test --features integration write_file_string_perm
       4. Assert: exit 0 AND output contains "1 passed"
       5. Run: cargo test --features integration write_file_string_isdir
       6. Assert: exit 0 AND output contains "1 passed"
     Evidence: .sisyphus/evidence/task-20-errmatrix.log

  Scenario: DiskFull via /dev/full (Linux-only)
    Tool: Bash
    Preconditions: Linux host (skip on non-Linux via #[cfg(target_os = "linux")])
    Steps:
      1. Run: cargo test --features integration write_file_string_disk_full
      2. Assert: error contains "No space left" or matches OPAL_FS_ERR_IO with ENOSPC text
    Evidence: .sisyphus/evidence/task-20-diskfull.log
  ```

  **Commit**: YES (5 commits, RGR-tagged)
  - Messages: `feat(runtime): write_file_string handles FileNotFoundError`, `... PermissionDeniedError`, `... IsADirectoryError`, `... DiskFull`, `... success path`
  - Files: `runtime/opal_fs.c`, `tests/integration_e2e/fs_write_file_string.rs`, fixtures
  - Pre-commit: regression gate + MSVC

- [x] 21. **T21: Replace `write_contents_sync` Stub (Binary Write)**

  **What to do**:
  - Mirror T20 minus string semantics. Accept `const uint8_t* bytes` + `int64_t count` per signature in `fs_builtins.rs`.
  - Reuse `errno_to_fs_error` and `fwrite_all` helpers.
  - Per-RGR: FileNotFoundError, PermissionDeniedError, IsADirectoryError, Success (4 commits).

  **Must NOT do**:
  - Re-test DiskFull (covered by T20; test pattern is the same).
  - Add new helpers.

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES (with T22 — different functions, shared helper from T20)
  - **Parallel Group**: Wave 5 sub-wave (T21 ∥ T22)
  - **Blocks**: T23, T24, T26
  - **Blocked By**: T20

  **References**:
  - T20's `fwrite_all` helper.
  - `FsResult` (void* value + error).

  **Acceptance Criteria**:
  - [ ] 4 cases pass.
  - [ ] Writing 256-byte fixture (0x00..0xFF) → file sha256 matches.
  - [ ] Writing 0 bytes → empty file created (verify size == 0).
  - [ ] `cargo test --all-features` → green.
  - [ ] MSVC link probe → PASS.

  **QA Scenarios**:
  ```
  Scenario: Round-trip 256-byte fixture
    Tool: Bash
    Steps:
      1. Run: cargo test --features integration write_file_bytes_256_roundtrip
      2. Assert: written file's sha256 matches fixture's sha256
    Evidence: .sisyphus/evidence/task-21-256.log

  Scenario: Empty write (edge case)
    Tool: Bash
    Steps:
      1. Run: cargo test --features integration write_file_bytes_empty
      2. Assert: file exists; size == 0
    Evidence: .sisyphus/evidence/task-21-empty.log

   Scenario: Error matrix (FileNotFoundError + PermissionDeniedError + IsADirectoryError)
     Tool: Bash
     Steps:
       1. Run: cargo test --features integration write_file_bytes_not_found
       2. Assert: exit 0 AND output contains "1 passed"
       3. Run: cargo test --features integration write_file_bytes_perm
       4. Assert: exit 0 AND output contains "1 passed"
       5. Run: cargo test --features integration write_file_bytes_isdir
       6. Assert: exit 0 AND output contains "1 passed"
     Evidence: .sisyphus/evidence/task-21-errmatrix.log
  ```

  **Commit**: YES (4 commits)
  - Files: `runtime/opal_fs.c`, `tests/integration_e2e/fs_write_file_bytes.rs`, `tests/fixtures/256bytes.bin` (already from T16)
  - Pre-commit: regression gate + MSVC

- [x] 22. **T22: Replace `append_text_sync` Stub (Append Mode)**

  **What to do**:
  - Replace stub: `fopen(path, "ab")`; reuse `fwrite_all`; close.
  - On missing file: create with 0644 (default behavior of `"ab"`).
  - Per-RGR: FileNotFoundError (missing parent), PermissionDeniedError, IsADirectoryError, NewFileCreated, AppendExisting (5 commits).

  **Must NOT do**:
  - Add a `append_contents_sync` (not in 34-builtin set).
  - Implement atomic append (lockfiles, etc.) — defer.

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES (with T21)
  - **Parallel Group**: Wave 5 sub-wave (T21 ∥ T22)
  - **Blocks**: T24
  - **Blocked By**: T20

  **References**:
  - T20's helpers.
  - `fopen(3)` mode "ab" semantics.

  **Acceptance Criteria** (5 commits):
  - [ ] **FileNotFoundError (missing parent)**: error.
  - [ ] **PermissionDeniedError**: error.
  - [ ] **IsADirectoryError**: error.
  - [ ] **NewFileCreated**: append to nonexistent file → file created with appended content.
  - [ ] **AppendExisting**: append "B" to file containing "A" → file contains "AB".
  - [ ] `cargo test --all-features` → green.
  - [ ] MSVC link probe → PASS.

  **QA Scenarios**:
  ```
  Scenario: Append-to-existing
    Tool: Bash
    Steps:
      1. Run: cargo test --features integration append_existing
      2. Assert: file contains "AB" (verified via Rust read_to_string in test)
    Evidence: .sisyphus/evidence/task-22-existing.log

  Scenario: Append creates new file
    Tool: Bash
    Steps:
      1. Run: cargo test --features integration append_creates_new
      2. Assert: file exists with appended content; perms == 0644
    Evidence: .sisyphus/evidence/task-22-new.log

   Scenario: Error matrix
     Tool: Bash
     Steps:
       1. Run: cargo test --features integration append_not_found
       2. Assert: exit 0 AND output contains "1 passed"
       3. Run: cargo test --features integration append_perm
       4. Assert: exit 0 AND output contains "1 passed"
       5. Run: cargo test --features integration append_isdir
       6. Assert: exit 0 AND output contains "1 passed"
     Evidence: .sisyphus/evidence/task-22-errmatrix.log
  ```

  **Commit**: YES (5 commits)
  - Files: `runtime/opal_fs.c`, `tests/integration_e2e/fs_append_file_string.rs`
  - Pre-commit: regression gate + MSVC

- [x] 23. **T23: `_fs_write_text_atomic` Project (Atomic Write Showcase)**

  **What to do**:
  - Create `test-projects/_fs_write_text_atomic/`:
    - Standard 4 root files; `.gitattributes` with `* -text`.
    - `src/main.op` (≤ 130 LoC): "config-saver" mini-app. Demonstrates atomic-write pattern in Opalescent code:
      1. Build target path via `join_path_components(work_dir, ["config.json"])`.
      2. Build temp path via `join_path_components(work_dir, ["config.json.tmp.<pid>"])` (use stub pid string).
      3. `write_text_sync(temp_path, content)`.
      4. `move_path_sync(temp_path, target_path)` (relies on T28).
      5. On any error → cleanup temp, surface error.
    - `src/atomic.op`: helper `atomic_write(target, content)` returning `Result<Void, FsError>`.
    - Print "wrote atomically: <bytes>" on success.
  - Integration test: assert success path; assert no `.tmp.` file remains after run.

  **Must NOT do**:
  - Use OS-specific atomicity primitives (rename across filesystems).
  - Exceed 150 LoC.
  - Test rename itself (that's T28's job).

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: NO (depends on T20 + T28)
  - **Blocks**: None
  - **Blocked By**: T20, T28

  **References**:
  - `test-projects/_fs_path_from/` template.
  - T20 contract; T28 rename contract.
  - LWN: atomic-rename pattern.

  **Acceptance Criteria**:
  - [ ] Project files exist; `.gitattributes` configured.
  - [ ] `opal run` produces "wrote atomically: <N>" with N == byte count.
  - [ ] After run: target file exists, no `.tmp.*` files in workspace.
  - [ ] Re-run → identical output; FsStateGuard restores clean.
  - [ ] `cargo test --features integration fs_write_text_atomic` → green twice.
  - [ ] `cargo test --all-features` → green.
  - [ ] MSVC link probe → PASS.

  **QA Scenarios**:
  ```
  Scenario: Atomic write (happy path)
    Tool: Bash
    Steps:
      1. Run: opal run test-projects/_fs_write_text_atomic/src/main.op 2>&1 | tee /tmp/qa.log; echo "EXIT=${PIPESTATUS[0]}"
      2. Assert: /tmp/qa.log contains "wrote atomically:"
      3. Run: ls test-projects/_fs_write_text_atomic/workspace/
      4. Assert: only "config.json" present (no .tmp.* files)
    Evidence: .sisyphus/evidence/task-23-atomic.log

  Scenario: Re-runnability
    Tool: Bash
    Steps:
      1. Run twice: cargo test --features integration fs_write_text_atomic
      2. Assert: both runs pass; workspace empty after
    Evidence: .sisyphus/evidence/task-23-rerun.log
  ```

  **Commit**: YES
  - Message: `test(test-projects): add _fs_write_text_atomic config-saver mini-app`
  - Files: project + integration test + mod
  - Pre-commit: regression gate + MSVC

- [x] 24. **T24: `_fs_append_log` Project (Append Showcase)**

  **What to do**:
  - Create `test-projects/_fs_append_log/`:
    - Standard 4 root files; `.gitattributes`.
    - `src/main.op` (≤ 100 LoC): "log-writer" mini-app. Calls `append_text_sync` 5 times with 5 different lines, then reads back via `read_lines_sync` and asserts (via runtime equality) line count == 5.
    - `src/logger.op`: helper `append_line(path, msg) -> Result<Void, FsError>`.
    - Print "appended 5 lines; readback confirmed".
  - Integration test: assert stdout match; assert log file size grows monotonically across calls (assert via Rust `metadata().len()` between subprocess calls — split into 5 sub-runs).

  **Must NOT do**:
  - Test concurrent appends (single-threaded showcase only).
  - Exceed 120 LoC.

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES (with T23 — independent projects)
  - **Parallel Group**: Wave 5 final sub-wave (T23 ∥ T24)
  - **Blocks**: None
  - **Blocked By**: T17 (read_lines), T22 (append)

  **References**:
  - T22 contract; T17 read_lines.
  - `test-projects/_fs_path_from/` template.

  **Acceptance Criteria**:
  - [ ] `opal run` succeeds with expected output.
  - [ ] Re-run → identical; clean workspace after.
  - [ ] `cargo test --features integration fs_append_log` → green twice.
  - [ ] `cargo test --all-features` → green.
  - [ ] MSVC link probe → PASS.

  **QA Scenarios**:
  ```
  Scenario: Append 5 lines + readback
    Tool: Bash
    Steps:
      1. Run: opal run test-projects/_fs_append_log/src/main.op 2>&1 | tee /tmp/qa.log; echo "EXIT=${PIPESTATUS[0]}"
      2. Assert: /tmp/qa.log contains "appended 5 lines; readback confirmed"
    Evidence: .sisyphus/evidence/task-24-append5.log

  Scenario: Monotonic file growth
    Tool: Bash
    Steps:
      1. Run: cargo test --features integration fs_append_log_monotonic
      2. Assert: 5 sub-assertions confirm size grows after each append
    Evidence: .sisyphus/evidence/task-24-monotonic.log
  ```

  **Commit**: YES
  - Message: `test(test-projects): add _fs_append_log log-writer mini-app`
  - Files: project + integration test + mod
  - Pre-commit: regression gate + MSVC

---

## Wave 6 — File / Directory / Metadata Family (~9 tasks)

> Per Metis: lock policy decisions BEFORE Wave 6.
> **Policy locks**:
> - **`list_directory_sync`**: Returns lexicographically sorted entries (use `qsort` with `strcmp`). Excludes `.` and `..`. Does NOT recurse. Returns `FsPathArrayResult` (per `runtime/opal_runtime.h:91`; distinct from `FsStringArrayResult` used by `read_lines_sync`).
> - **`create_directory_sync`**: Single-level only; if parent missing → `FileNotFoundError`. Mode 0755. (Recursive create is `create_directory_recursive_sync` per `runtime/opal_runtime.h:114` — not in scope of this plan; deferred.)
> - **`remove_directory_sync`**: Empty directory only; if non-empty → `Io: directory not empty` (ENOTEMPTY). For recursive removal use a separate `remove_directory_all` (not in 34-set; document).
> - **`copy_file_sync`**: Truncates dest. Preserves contents only (NOT mode/timestamps in v1). Same-file copy → success no-op (or skip via stat compare). Cross-device → use streaming copy (no `link()`).
> - **`rename_path_sync`**: POSIX `rename(2)` semantics. Atomic within same filesystem; cross-device → returns `Io: EXDEV` (caller must fall back to copy+remove).
> - **`remove_file_sync`**: `unlink(2)`; on directory → `IsADirectoryError`; on missing → `FileNotFoundError`.
> - **`path_exists_sync`** / **`path_is_file_sync`** / **`path_is_directory_sync`**: Use `stat`; broken symlinks → `path_exists_sync` returns false (follow symlinks). Document.
> - **`metadata_sync`**: Returns `FsMetadataResult` with size + mtime + is_dir + is_file (extend struct if needed; verify in T1 doc).

- [x] 25. **T25: Replace File-Removal + Metadata Stubs (`remove_file`, `path_exists`, `path_is_file`, `path_is_directory`, `metadata`)**

  **What to do**:
  - Replace 5 stubs in `runtime/opal_fs.c`:
    - `delete_file_sync`: `unlink(path)`; errno → discriminant; on EISDIR → `IsADirectoryError`.
    - `path_exists_sync`: `stat`; on ENOENT → `value = false, error = NULL`; on success → `value = true`; other errors → propagate.
    - `is_file_sync`: `stat`; check `S_ISREG`; ENOENT → `false, NULL`.
    - `is_directory_sync`: `stat`; check `S_ISDIR`; ENOENT → `false, NULL`.
    - `read_metadata_sync`: `stat`; populate `FsMetadataResult { size, mtime_secs, is_dir, is_file, error }`. Verify struct fields exist in T1 doc; if not, EXTEND `runtime/opal_runtime.h` + codegen.
  - Per-RGR: 5 functions × ~3 cases each = ~15 commits (group by function: 5 commits, each squashing its own RGR cycles).

  **Must NOT do**:
  - Implement `metadata_async` / atime / ctime — defer.
  - Resolve symlinks differently from POSIX default.

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: 5 stubs touching `stat` + struct layout; FsMetadataResult may need extension affecting codegen.
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: NO (touches struct layout + 5 stubs together)
  - **Blocks**: T26, T27, T29, T30 (high-level dir ops)
  - **Blocked By**: T15 (errno helper), T1 (sentinel doc — verify FsMetadataResult struct)

  **References**:
  - `runtime/opal_fs.c` — 5 stubs.
  - `runtime/opal_runtime.h` — `FsMetadataResult` struct (verify shape).
  - `src/codegen/functions_stdlib.rs` — metadata return-type lowering.
  - `src/type_system/checker/fs_builtins.rs` — `metadata` signature.
  - POSIX `stat(2)`, `unlink(2)`.

  **Acceptance Criteria**:
  - [ ] `remove_file`: FileNotFoundError, IsADirectoryError, PermissionDeniedError, Success.
  - [ ] `path_exists`: file exists → true; dir exists → true; missing → false; broken symlink → false.
  - [ ] `path_is_file`: regular file → true; dir → false; missing → false.
  - [ ] `path_is_directory`: dir → true; file → false; missing → false.
  - [ ] `metadata`: returns size matching `wc -c`; mtime within 5s of `time(NULL)`; is_dir/is_file flags correct; missing → `FileNotFoundError`.
  - [ ] If `FsMetadataResult` struct extended → codegen + checker updated; full `cargo test --all-features` green.
  - [ ] MSVC link probe → PASS.

  **QA Scenarios**:
  ```
  Scenario: Predicate matrix (5 fixtures × 3 predicates)
    Tool: Bash
    Steps:
      1. Run: cargo test --features integration "fs_predicates_matrix"
      2. Assert: 15 sub-assertions pass (file/dir/missing/broken-symlink/regular-symlink × exists/is_file/is_directory)
    Evidence: .sisyphus/evidence/task-25-predicates.log

  Scenario: Metadata size + mtime
    Tool: Bash
    Steps:
      1. Run: cargo test --features integration metadata_size_mtime
      2. Assert: size matches expected; mtime within 5s of test start
    Evidence: .sisyphus/evidence/task-25-meta.log

   Scenario: remove_file error matrix
     Tool: Bash
     Steps:
       1. Run: cargo test --features integration remove_file_not_found
       2. Assert: exit 0 AND output contains "1 passed"
       3. Run: cargo test --features integration remove_file_isdir
       4. Assert: exit 0 AND output contains "1 passed"
       5. Run: cargo test --features integration remove_file_perm
       6. Assert: exit 0 AND output contains "1 passed"
       7. Run: cargo test --features integration remove_file_success
       8. Assert: exit 0 AND output contains "1 passed"
     Evidence: .sisyphus/evidence/task-25-remove.log
  ```

  **Commit**: YES (5 commits, one per function)
  - Messages: `feat(runtime): remove_file real impl`, `... path_exists`, `... path_is_file`, `... path_is_directory`, `... metadata`
  - Files: `runtime/opal_fs.c`, `runtime/opal_runtime.h` (if extended), `src/codegen/**` (if extended), `tests/integration_e2e/fs_predicates.rs`, `tests/integration_e2e/fs_metadata.rs`
  - Pre-commit: regression gate + MSVC

- [x] 26. **T26: Replace `copy_file_sync` Stub (Streaming Copy)**

  **What to do**:
  - Replace stub: open src `"rb"`, dest `"wb"`, loop `fread`/`fwrite` with 64KB buffer, close both.
  - Reuse `errno_to_fs_error`.
  - Same-file detection: stat both; if same inode → success no-op (or document as undefined and skip).
  - Per-RGR: FileNotFoundError (src), FileNotFoundError (dest parent), PermissionDeniedError, IsADirectoryError (src), IsADirectoryError (dest), Success (6 commits).

  **Must NOT do**:
  - Preserve mode/timestamps (defer).
  - Use `sendfile(2)` or `copy_file_range(2)` — pure stdio for portability.
  - Implement directory copy (not in 34-set).

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES (with T27 — independent stubs)
  - **Parallel Group**: Wave 6 sub-wave (T26 ∥ T27)
  - **Blocks**: T29 (showcase)
  - **Blocked By**: T15, T20, T25

  **References**:
  - T15/T20 helpers (errno_to_fs_error, fwrite_all).
  - POSIX `stat(2)` for same-file detection.

  **Acceptance Criteria**:
  - [ ] 6 cases pass.
  - [ ] Round-trip 256-byte fixture (T16) → dest sha256 matches src.
  - [ ] Large file (10MB generated in test setup) → copies correctly; no truncation.
  - [ ] `cargo test --all-features` → green.
  - [ ] MSVC link probe → PASS.

  **QA Scenarios**:
  ```
  Scenario: 10MB streaming copy
    Tool: Bash
    Preconditions: tests/fixtures/large_10mb.bin generated in test setup (NOT committed)
    Steps:
      1. Run: cargo test --features integration copy_file_10mb
      2. Assert: dest sha256 matches src sha256
      3. Assert: copy completes in < 5s
    Evidence: .sisyphus/evidence/task-26-10mb.log

   Scenario: Error matrix (5 cases)
     Tool: Bash
     Steps:
       1. Run: cargo test --features integration copy_file_src_missing
       2. Assert: exit 0 AND output contains "1 passed"
       3. Run: cargo test --features integration copy_file_dest_parent_missing
       4. Assert: exit 0 AND output contains "1 passed"
       5. Run: cargo test --features integration copy_file_perm
       6. Assert: exit 0 AND output contains "1 passed"
       7. Run: cargo test --features integration copy_file_src_isdir
       8. Assert: exit 0 AND output contains "1 passed"
       9. Run: cargo test --features integration copy_file_dest_isdir
       10. Assert: exit 0 AND output contains "1 passed"
     Evidence: .sisyphus/evidence/task-26-errmatrix.log
  ```

  **Commit**: YES (6 commits)
  - Files: `runtime/opal_fs.c`, `tests/integration_e2e/fs_copy_file.rs`
  - Pre-commit: regression gate + MSVC

- [x] 27. **T27: Replace `move_path_sync` Stub (POSIX rename)**

  **What to do**:
  - Replace stub: `rename(src, dest)`; on EXDEV → return `Io: EXDEV: cross-device rename not supported (caller should copy+delete)`.
  - Per-RGR: FileNotFoundError (src), PermissionDeniedError, ExistsOverwrite (POSIX rename overwrites — document), CrossDevice, Success (5 commits).

  **Must NOT do**:
  - Implement copy+delete fallback (caller's responsibility — document).
  - Preserve special handling for directories (POSIX `rename` handles dirs naturally if same fs).

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES (with T26)
  - **Parallel Group**: Wave 6 sub-wave (T26 ∥ T27)
  - **Blocks**: T23, T29
  - **Blocked By**: T15, T25

  **References**:
  - POSIX `rename(2)`, errno EXDEV.
  - T15 errno helper.

  **Acceptance Criteria**:
  - [ ] 5 cases pass.
  - [ ] Rename within /tmp → succeeds; src gone, dest present with original content.
  - [ ] Rename overwriting existing dest → succeeds (POSIX semantics); document in error contract.
  - [ ] Cross-device test: SKIP if not feasible in CI (gate on `cfg!(target_os = "linux")` + presence of `/dev/shm` as different fs); document the skip.
  - [ ] `cargo test --all-features` → green.
  - [ ] MSVC link probe → PASS.

  **QA Scenarios**:
  ```
  Scenario: Rename within fs (success)
    Tool: Bash
    Steps:
      1. Run: cargo test --features integration rename_within_fs
      2. Assert: src gone, dest present with content matching
    Evidence: .sisyphus/evidence/task-27-within.log

  Scenario: Cross-device EXDEV (best-effort)
    Tool: Bash
    Steps:
      1. Run: cargo test --features integration rename_cross_device -- --nocapture
      2. Assert: either error contains "EXDEV" OR test self-skips with logged reason
    Evidence: .sisyphus/evidence/task-27-exdev.log

   Scenario: Error matrix
     Tool: Bash
     Steps:
       1. Run: cargo test --features integration rename_not_found
       2. Assert: exit 0 AND output contains "1 passed"
       3. Run: cargo test --features integration rename_perm
       4. Assert: exit 0 AND output contains "1 passed"
       5. Run: cargo test --features integration rename_overwrite
       6. Assert: exit 0 AND output contains "1 passed"
     Evidence: .sisyphus/evidence/task-27-errmatrix.log
  ```

  **Commit**: YES (5 commits)
  - Files: `runtime/opal_fs.c`, `tests/integration_e2e/fs_rename_path.rs`
  - Pre-commit: regression gate + MSVC

- [x] 28. **T28: Replace Directory Stubs (`create_directory`, `remove_directory`, `list_directory`)**

  **What to do**:
  - 3 stubs in `runtime/opal_fs.c`:
    - `create_directory_sync`: `mkdir(path, 0755)` (use `opal_mkdir` shim from T3); errno → discriminant.
    - `delete_directory_sync`: `rmdir`; on ENOTEMPTY → `Io: directory not empty`.
    - `list_directory_sync`: `opendir` + `readdir` loop; skip `.`/`..`; collect into `char**`; sort with `qsort`(strcmp); return `FsPathArrayResult` (per `runtime/opal_runtime.h:91`).
  - Per-RGR per function (4+3+5 commits ~ 12 commits total).

  **Must NOT do**:
  - Recursive create/remove (not in 34-set).
  - Filtering (e.g., hidden files) — return everything except `.`/`..`.

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: 3 stubs + sort policy + portability shim use; readdir loop has subtle freeing.
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: NO (uses portability shims; sequential to lock pattern)
  - **Blocks**: T29, T30 (high-level dir ops)
  - **Blocked By**: T3 (shims), T15 (errno helper)

  **References**:
  - `runtime/opal_portability.h` — `opal_opendir/readdir/closedir/mkdir/rmdir`.
  - POSIX `mkdir(2)`, `rmdir(2)`, `readdir(3)`.
  - `qsort(3)`.

  **Acceptance Criteria**:
  - [ ] `create_directory`: FileNotFoundError (parent), PermissionDeniedError, FileAlreadyExistsError (EEXIST), Success.
  - [ ] `remove_directory`: FileNotFoundError, PermissionDeniedError, NotEmpty, Success.
  - [ ] `list_directory`: FileNotFoundError, PermissionDeniedError, IsNotADirectoryError, Success (empty dir → count=0), Success (3 entries → sorted).
  - [ ] `cargo test --all-features` → green.
  - [ ] MSVC link probe → PASS.

  **QA Scenarios**:
  ```
  Scenario: list_directory sort order
    Tool: Bash
    Preconditions: setup creates dir with files "c.txt", "a.txt", "b.txt"
    Steps:
      1. Run: cargo test --features integration list_directory_sorted
      2. Assert: returned array == ["a.txt", "b.txt", "c.txt"]
    Evidence: .sisyphus/evidence/task-28-sort.log

  Scenario: create + remove round-trip
    Tool: Bash
    Steps:
      1. Run: cargo test --features integration mkdir_rmdir_roundtrip
      2. Assert: dir created, then removed; final state matches initial
    Evidence: .sisyphus/evidence/task-28-roundtrip.log

  Scenario: NotEmpty error
    Tool: Bash
    Steps:
      1. Run: cargo test --features integration rmdir_not_empty
      2. Assert: error contains "not empty"
    Evidence: .sisyphus/evidence/task-28-notempty.log
  ```

  **Commit**: YES (12 commits, RGR-tagged per scenario)
  - Files: `runtime/opal_fs.c`, `tests/integration_e2e/fs_directories.rs`
  - Pre-commit: regression gate + MSVC

- [x] 29. **T29: `_fs_dir_inventory` Project (Directory Family Showcase)**

  **What to do**:
  - Create `test-projects/_fs_dir_inventory/`:
    - Standard 4 root files; `.gitattributes`.
    - `src/main.op` (≤ 140 LoC): "directory-inventory" mini-app:
      1. Create `workspace/inventory/` via `create_directory_sync`.
      2. Write 3 files (`a.txt`, `b.txt`, `c.txt`) via `write_text_sync`.
      3. List via `list_directory_sync` → assert sorted, count == 3.
      4. Read each via `read_text_sync` → assert content matches.
      5. Remove each via `delete_file_sync`.
      6. Remove dir via `delete_directory_sync`.
      7. Print "inventory: 3 files; cleanup ok".
    - `src/inventory.op`: helpers.
  - Integration test: assert stdout match; FsStateGuard verifies workspace empty after.

  **Must NOT do**:
  - Test rename/copy (those have own showcases — T23 atomic, no rename showcase needed since T23 covers it).
  - Exceed 150 LoC.

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: NO (depends on T20, T15, T25, T28)
  - **Blocks**: None
  - **Blocked By**: T20, T15, T25, T28

  **References**:
  - All Wave 4-6 contracts.
  - `test-projects/_fs_path_from/` template.

  **Acceptance Criteria**:
  - [ ] `opal run` succeeds with expected stdout.
  - [ ] Re-run twice → identical; clean workspace after.
  - [ ] `cargo test --features integration fs_dir_inventory` → green twice.
  - [ ] `cargo test --all-features` → green.
  - [ ] MSVC link probe → PASS.

  **QA Scenarios**:
  ```
  Scenario: Full directory lifecycle
    Tool: Bash
    Steps:
      1. Run: opal run test-projects/_fs_dir_inventory/src/main.op 2>&1 | tee /tmp/qa.log; echo "EXIT=${PIPESTATUS[0]}"
      2. Assert: /tmp/qa.log contains "inventory: 3 files; cleanup ok"
      3. Run: ls test-projects/_fs_dir_inventory/workspace/
      4. Assert: directory is empty
    Evidence: .sisyphus/evidence/task-29-lifecycle.log

  Scenario: Re-runnability
    Tool: Bash
    Steps:
      1. Run twice: cargo test --features integration fs_dir_inventory
      2. Assert: both pass; workspace clean
    Evidence: .sisyphus/evidence/task-29-rerun.log
  ```

  **Commit**: YES
  - Message: `test(test-projects): add _fs_dir_inventory directory-lifecycle mini-app`
  - Files: project + integration test + mod
  - Pre-commit: regression gate + MSVC

## Wave 7 — High-Level Showcase Projects (3 fs-* projects)

> The 3 fs-* projects are LARGE realistic mini-apps (≤ 400 LoC) that combine MULTIPLE families. They are the user's stated entry points ("start with fs-directory-operations").

- [x] 30. **T30: `fs-directory-operations` Project — End-to-End Directory CRUD App**

  **What to do**:
  - Create `test-projects/fs-directory-operations/` (existing empty dir):
    - Standard 4 root files; `.gitattributes`.
    - Multi-file `src/`:
      - `src/main.op` (≤ 100 LoC): orchestrator.
      - `src/operations/create.op`: create-directory + nested mkdir-style helper (loop over components and call `create_directory_sync`).
      - `src/operations/list.op`: recursive listing (depth-1 wrapper using `list_directory_sync` + `is_directory_sync`).
      - `src/operations/remove.op`: recursive remove (depth-first walk + `delete_file_sync` + `delete_directory_sync`).
      - `src/types/inventory_entry.op`: nominal type for directory entries.
    - `fixtures/seed.json` (committed): describes a directory tree to materialize.
    - `src/main.op` flow:
      1. Read seed via `read_text_sync` (parse manually — no JSON parser yet; treat as line-list spec like `dir/sub/file.txt|content`).
      2. Materialize tree: create dirs + write files.
      3. List recursively: print sorted entries.
      4. Tear down: remove all files + dirs.
      5. Print "fs-directory-operations: created N entries, listed N, removed N — all match".
  - LoC cap: 400 across all .op files.

  **Must NOT do**:
  - Implement a JSON parser (use line-format).
  - Test path manipulation extensively (covered by `_fs_*` projects).
  - Exceed 400 LoC total.

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: Multi-file project with recursive walk logic; orchestrates 6+ builtins.
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: NO (depends on most of Waves 4-6)
  - **Blocks**: None
  - **Blocked By**: T15, T17, T20, T25, T28

  **References**:
  - `test-projects/_fs_dir_inventory/` (T29) — simpler version, copy structure.
  - All Wave 4-6 contracts.

  **Acceptance Criteria**:
  - [ ] All files exist; project compiles via `opal build`.
  - [ ] `opal run` succeeds with expected output ("created N entries...").
  - [ ] Re-run twice → identical; FsStateGuard reports clean.
  - [ ] `cargo test --features integration fs_directory_operations` → green twice.
  - [ ] `cargo test --all-features` → green.
  - [ ] MSVC link probe → PASS.

  **QA Scenarios**:
  ```
  Scenario: End-to-end directory CRUD
    Tool: Bash
    Steps:
      1. Run: opal run test-projects/fs-directory-operations/src/main.op 2>&1 | tee /tmp/qa.log; echo "EXIT=${PIPESTATUS[0]}"
      2. Assert: /tmp/qa.log contains "fs-directory-operations: created"
      3. Assert: stdout contains "all match"
    Evidence: .sisyphus/evidence/task-30-e2e.log

  Scenario: Re-runnability with workspace mutation
    Tool: Bash
    Steps:
      1. Manually mkdir test-projects/fs-directory-operations/workspace/junk-dir
      2. cargo test --features integration fs_directory_operations
      3. Assert: pass; junk-dir gone after FsStateGuard restore
    Evidence: .sisyphus/evidence/task-30-rerun.log

  Scenario: Project compiles cleanly with opal build
    Tool: Bash
    Steps:
      1. Run: opal build test-projects/fs-directory-operations
      2. Assert: exit 0; no warnings
    Evidence: .sisyphus/evidence/task-30-build.log
  ```

  **Commit**: YES (1-3 commits if logical groups)
  - Message: `test(test-projects): add fs-directory-operations end-to-end CRUD app`
  - Files: project + integration test + mod
  - Pre-commit: regression gate + MSVC

- [x] 31. **T31: `fs-path-manipulation` Project — Path Algebra Showcase**

   **What to do**:
   - Create `test-projects/fs-path-manipulation/`:
     - Standard 4 root files.
     - `src/main.op` (≤ 80 LoC): orchestrator.
     - `src/path_ops/normalize.op`: wraps `normalize_path` (infallible) with a small DSL of inputs (15+ test inputs).
     - `src/path_ops/join.op`: 10+ join scenarios.
     - `src/path_ops/query.op`: 10+ extension/name/parent queries.
     - `src/path_ops/absolute.op`: 5+ absolute-path resolutions.
     - `fixtures/path_cases.txt` (committed with `-text`): line-list of `input -> expected_output` pairs (40+ cases).
     - Flow: parse fixture lines, run each operation, assert via runtime equality, print pass count.
   - Commit a `sample.bad.txt` fixture alongside the canonical `sample.txt` for the sanity-test scenario. The bad fixture differs by exactly one byte and triggers deterministic assertion failure. Add `.gitattributes -text` rule to preserve byte-exactness.
   - LoC cap: 400.

  **Must NOT do**:
  - Test I/O ops (this project is purely path manipulation).
  - Exceed 400 LoC.

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: Comprehensive path-algebra coverage requires careful case selection.
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES (with T30, T32 — independent projects)
  - **Parallel Group**: Wave 7 (T30 ∥ T31 ∥ T32)
  - **Blocks**: None
  - **Blocked By**: T9, T13, T14 (path family fixes)

  **References**:
  - All `_fs_path_*` projects (T10–T14).

  **Acceptance Criteria**:
  - [ ] 40+ path cases pass via runtime equality.
  - [ ] `opal run` prints "passed 40/40 cases" or similar.
  - [ ] Re-runnability via FsStateGuard.
  - [ ] `cargo test --features integration fs_path_manipulation` → green twice.
  - [ ] MSVC link probe → PASS.

  **QA Scenarios**:
  ```
  Scenario: All 40+ path cases
    Tool: Bash
    Steps:
      1. Run: opal run test-projects/fs-path-manipulation/src/main.op 2>&1 | tee /tmp/qa.log; echo "EXIT=${PIPESTATUS[0]}"
      2. Assert: /tmp/qa.log contains "passed 40/40" (or current case count)
    Evidence: .sisyphus/evidence/task-31-cases.log

   Scenario: Sanity check — known-bad fixture causes deterministic failure
     Tool: Bash
     Preconditions: Pre-committed bad fixture at `test-projects/<chosen-project>/fixtures/sample.bad.txt` (committed alongside canonical sample.txt; differs by exactly one byte to trigger known assertion failure). Use `.gitattributes -text` so git does not normalize line endings.
     Steps:
       1. Run: cargo test --features integration sanity_known_bad_fixture 2>&1 | tee .sisyphus/evidence/task-31-sanity-fail.log
       2. Assert: exit non-zero (test deliberately FAILS — that IS the sanity check)
       3. Assert: stderr contains "assertion failed" OR "sanity_known_bad_fixture"
       4. Run: cargo test --features integration --no-fail-fast 2>&1 | tee .sisyphus/evidence/task-31-others-pass.log
       5. Assert: output contains "test result: ok" at least once (other tests still pass — only sanity deliberately fails)
     Expected Result: Sanity test deterministically fails with bad fixture; bad fixture is COMMITTED, never mutated at runtime.
     Evidence: .sisyphus/evidence/task-31-sanity-fail.log + task-31-others-pass.log
  ```

  **Commit**: YES
  - Message: `test(test-projects): add fs-path-manipulation path-algebra showcase`
  - Files: project + integration test + mod
  - Pre-commit: regression gate + MSVC

- [x] 32. **T32: `fs-markdown-roundtrip` Project — Read/Process/Write Pipeline**

  **What to do**:
  - Create `test-projects/fs-markdown-roundtrip/`:
    - Standard 4 root files; `.gitattributes` includes `fixtures/*.md -text` for deterministic line endings.
    - `src/main.op` (≤ 100 LoC): orchestrator.
    - `src/processing/parse.op`: split markdown into blocks (heading vs paragraph) using `read_lines_sync`.
    - `src/processing/transform.op`: prepend `> ` to every paragraph line (mock blockquote transform).
    - `src/processing/serialize.op`: rejoin and write via `write_text_sync`.
    - `src/types/block.op`: nominal `Block` type with discriminator.
    - `fixtures/input.md` (committed `-text`): 30-line markdown file with mixed headings/paragraphs.
    - `fixtures/expected_output.md` (committed `-text`): expected transformed result.
    - Flow: read input → parse → transform → serialize to `workspace/output.md` → read back → compare to `expected_output.md` byte-for-byte (sha256 match via runtime helper).
    - Print "roundtrip: ok (N bytes match)".
  - LoC cap: 400.

  **Must NOT do**:
  - Implement a real markdown parser (line-based heuristic only).
  - Use external libraries.
  - Exceed 400 LoC.

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: Multi-file pipeline orchestrating read + transform + write; needs careful line-handling.
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES (with T30, T31)
  - **Parallel Group**: Wave 7
  - **Blocks**: None
  - **Blocked By**: T15, T17, T20

  **References**:
  - `_fs_read_text_lines/` (T19) and `_fs_write_text_atomic/` (T23).

  **Acceptance Criteria**:
  - [ ] `opal run` succeeds with "roundtrip: ok".
  - [ ] Output bytes match `fixtures/expected_output.md` exactly.
  - [ ] Re-runnability; `cargo test` green twice.
  - [ ] MSVC link probe → PASS.

  **QA Scenarios**:
  ```
  Scenario: End-to-end markdown roundtrip
    Tool: Bash
    Steps:
      1. Run: opal run test-projects/fs-markdown-roundtrip/src/main.op 2>&1 | tee /tmp/qa.log; echo "EXIT=${PIPESTATUS[0]}"
      2. Assert: /tmp/qa.log contains "roundtrip: ok"
      3. Run: sha256sum test-projects/fs-markdown-roundtrip/workspace/output.md test-projects/fs-markdown-roundtrip/fixtures/expected_output.md
      4. Assert: hashes match
    Evidence: .sisyphus/evidence/task-32-roundtrip.log

  Scenario: Re-runnability with mutated workspace
    Tool: Bash
    Steps:
      1. Inject extra files in workspace/
      2. cargo test --features integration fs_markdown_roundtrip
      3. Assert: pass; workspace empty after
    Evidence: .sisyphus/evidence/task-32-rerun.log
  ```

  **Commit**: YES
  - Message: `test(test-projects): add fs-markdown-roundtrip read/transform/write pipeline`
  - Files: project + integration test + mod
  - Pre-commit: regression gate + MSVC

---

## Wave 8 — Re-runnability + MSVC Verification (final tasks)

- [x] 33. **T33: Full Re-runnability Sweep — `fs_*` Suite Verification**

  **What to do**:
  - Add a top-level integration test `tests/integration_e2e/fs_rerunnability.rs`:
    - For each fs_* test in the suite, assert FsStateGuard manifest pre-test == manifest post-test.
    - Implement as a single `#[test]` that:
      1. Snapshots workspace state for ALL test-project directories under `test-projects/_fs_*` and `test-projects/fs-*`.
      2. Runs `cargo test --features integration fs_ -- --test-threads=1` as a subprocess (or uses `serial_test` ordering directly via cfg).
      3. Snapshots state again.
      4. Asserts both snapshots identical (sha256 manifest equality).
  - Enforce that all 20 projects have a `.gitignore` excluding `workspace/` and `target/`.
  - Add documentation `.sisyphus/evidence/T33-rerunnability-policy.md` describing what constitutes "clean state".

  **Must NOT do**:
  - Modify existing tests' setup logic (they should already use FsStateGuard from T4).
  - Add CI-only logic — must work locally.

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: NO (final integration test)
  - **Blocks**: T34, F1–F4
  - **Blocked By**: ALL implementation tasks (T1–T32)

  **References**:
  - `tests/integration_e2e/fs_state_guard.rs` (T4).
  - All test projects under `test-projects/`.

  **Acceptance Criteria**:
  - [ ] `cargo test --features integration fs_rerunnability` → green.
  - [ ] Run the full fs suite TWICE in a single `cargo test` invocation (e.g., via test repetition via env var or duplicate test fn) → both runs green.
  - [ ] After `cargo test`, `git status test-projects/` shows clean (no modified/untracked files).
  - [ ] Policy doc committed.

  **QA Scenarios**:
  ```
  Scenario: Full suite double-run
    Tool: Bash
    Steps:
      1. Run: cargo test --features integration fs_ 2>&1 | tee .sisyphus/evidence/task-33-run1.log
      2. Run: cargo test --features integration fs_ 2>&1 | tee .sisyphus/evidence/task-33-run2.log
      3. Assert: both runs exit 0 with same passing count
      4. Run: git status --porcelain test-projects/
      5. Assert: empty output
    Evidence: .sisyphus/evidence/task-33-doublerun.log

  Scenario: Manifest pre/post equality
    Tool: Bash
    Steps:
      1. Run: cargo test --features integration fs_rerunnability_manifest
      2. Assert: pass
    Evidence: .sisyphus/evidence/task-33-manifest.log
  ```

  **Commit**: YES
  - Message: `test(integration): full fs_* re-runnability sweep + state-restore manifest verification`
  - Files: `tests/integration_e2e/fs_rerunnability.rs`, `tests/integration_e2e/tests.rs` (mod declaration), `.sisyphus/evidence/T33-rerunnability-policy.md`
  - Pre-commit: regression gate + MSVC

- [x] 34. **T34: MSVC Compile + Link Final Verification (Full Runtime)**

  **What to do**:
  - Extend `scripts/msvc_link_probe.sh` (from T5) to:
    1. Compile ALL runtime .c files (`runtime/opal_*.c`) with `clang-cl --target=x86_64-pc-windows-msvc /winsysroot=$XWIN_SYSROOT /Fo:target/msvc-probe/`.
    2. Link with `lld-link /out:target/msvc-probe/runtime.dll /dll target/msvc-probe/*.obj /libpath:$XWIN_SYSROOT/.../lib msvcrt.lib kernel32.lib`.
    3. Assert exit 0.
    4. Run a smoke harness: link `opal_msvc_link_probe.c` against the runtime, check undefined-symbol report is empty.
  - Document the script invocation in `.sisyphus/evidence/T34-msvc-verification.md`.
  - Add CI-equivalent shell command snippet for future Windows-support work (per `windows-support.md` D4).

  **Must NOT do**:
  - Run on actual Windows host (script must be Linux-only with xwin sysroot).
  - Resolve runtime semantics differences between Linux and Windows here (T3/T9/T14 already handled).

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: Final portability gate; xwin/clang-cl/lld-link orchestration is fiddly.
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: NO (final gate)
  - **Blocks**: F1–F4
  - **Blocked By**: T5, T33, ALL runtime-modifying tasks

  **References**:
  - `.sisyphus/plans/windows-support.md` D4 (xwin sysroot setup).
  - T5 script.
  - clang-cl docs, lld-link docs.

  **Acceptance Criteria**:
  - [ ] `bash scripts/msvc_link_probe.sh --full` → exit 0.
  - [ ] All runtime .c files compile with clang-cl --target=x86_64-pc-windows-msvc.
  - [ ] Linking produces `runtime.dll` (or `.lib`) with no undefined symbols.
  - [ ] Documentation committed at `.sisyphus/evidence/T34-msvc-verification.md`.

  **QA Scenarios**:
  ```
  Scenario: Full MSVC compile + link
    Tool: Bash
    Steps:
      1. Run: bash scripts/msvc_link_probe.sh --full 2>&1 | tee .sisyphus/evidence/task-34-msvc.log
      2. Assert: exit 0
      3. Assert: log contains "all object files linked"
      4. Assert: target/msvc-probe/runtime.dll (or .lib) exists
    Evidence: .sisyphus/evidence/task-34-msvc.log

  Scenario: Undefined-symbol report is empty
    Tool: Bash
    Steps:
      1. Run: bash scripts/msvc_link_probe.sh --report-undefined
      2. Assert: stdout shows "0 undefined symbols"
    Evidence: .sisyphus/evidence/task-34-undef.log
  ```

  **Commit**: YES
  - Message: `build(runtime): full MSVC clang-cl + lld-link verification gate`
  - Files: `scripts/msvc_link_probe.sh`, `.sisyphus/evidence/T34-msvc-verification.md`
  - Pre-commit: regression gate

---

## Final Verification Wave (MANDATORY — after ALL implementation tasks)

> 4 review agents run in PARALLEL. ALL must APPROVE. Present consolidated
> results to user and get explicit "okay" before completing.
>
> **Do NOT auto-proceed after verification. Wait for user's explicit approval.**

- [ ] F1. **Plan Compliance Audit** — `oracle`
  Read this plan end-to-end. For each "Must Have": verify implementation exists
  (read file, run command). For each "Must NOT Have": search codebase for forbidden
  patterns — reject with file:line if found. Check evidence files exist in
  `.sisyphus/evidence/`. Compare deliverables against plan. Specifically verify:
  (a) old plan UNTOUCHED (`git log .sisyphus/plans/file-io-stdlib-path-object-centric.md` shows no edits during this work),
  (b) `ast_grep_search` for `not implemented` in `runtime/opal_fs.c` returns 0,
  (c) all 20 project dirs have `opal.toml` + `src/main.op` + `.gitignore` + `README.md`,
  (d) no permissions or long-path code present.
  Output: `Must Have [N/N] | Must NOT Have [N/N] | Tasks [N/N] | VERDICT: APPROVE/REJECT`

- [x] F2. **Code Quality Review** — `unspecified-high`
  Run `cargo test --all-features`, `cargo clippy --all-targets --all-features -- -D warnings`,
  `cargo fmt --check`. Review all changed files for: `as any`/`unsafe` blocks
  without justification comments, empty catches, `println!`/`eprintln!` in non-test
  code, commented-out code, unused imports, unused warnings. Check AI slop:
  excessive comments, over-abstraction, generic names (`data`/`result`/`item`/`temp`).
  Verify `runtime/opal_fs.c` uses only macros from `opal_portability.h` (no raw `_WIN32`).
  Output: `Build [PASS/FAIL] | Clippy [PASS/FAIL] | Fmt [PASS/FAIL] | Tests [N pass/N fail] | Files [N clean/N issues] | VERDICT`

- [x] F3. **Real Manual QA (Bash)** — `unspecified-high`
  Start from clean state (`cargo clean`). Execute EVERY QA scenario from EVERY task —
  follow exact steps, capture evidence to `.sisyphus/evidence/final-qa/`. Run
  `cargo test --features integration fs_` TWICE in succession (re-runnability gauntlet);
  both must pass. Verify `git status` clean after both runs (no leftover files).
  Run MSVC compile + link gate on full runtime/. Test edge cases: empty fixtures,
  permission-denied paths (e.g. /root), nonexistent dirs.
  Output: `Scenarios [N/N pass] | Re-runs [2/2 green] | Git clean [YES/NO] | MSVC [PASS/FAIL] | VERDICT`

- [ ] F4. **Scope Fidelity Check** — `deep`
  For each task: read "What to do", read actual diff (`git log --oneline` + `git diff`).
  Verify 1:1 — everything in spec was built (no missing), nothing beyond spec was built
  (no creep). Check "Must NOT do" compliance per task. Detect cross-task contamination:
  Task N touching Task M's files. Flag unaccounted changes. Specifically verify:
  (a) NO permissions code, (b) NO long-path code, (c) NO new fs builtins (only stub
  replacement), (d) NO edits to old plan, (e) RGR commits present per scenario
  (`git log --grep='^\\(red\\|green\\|refactor\\):'` shows ≥3 per project).
  Output: `Tasks [N/N compliant] | Contamination [CLEAN/N issues] | RGR [N/N projects with red+green+refactor] | Old plan untouched [YES/NO] | VERDICT`

---

## Commit Strategy

Each task uses RGR-prefixed commits:
- `red(<scope>): <desc>` — failing scenario added (test compiles but fails or asserts wrong)
- `green(<scope>): <desc>` — minimum impl makes test pass
- `refactor(<scope>): <desc>` — cleanup, no behavior change
- `chore(<scope>): <desc>` — non-RGR scaffolding (e.g. T1-T7 infra)

Pre-commit per task: `cargo test --all-features` (Regression Gate) + MSVC compile/link gate.

---

## Success Criteria

### Verification Commands
```bash
# 1. All fs tests pass on Linux
cargo test --features integration fs_  # Expected: 20+ tests, all green

# 2. Regression gate
cargo test --all-features  # Expected: all green

# 3. Re-runnability gauntlet (CRITICAL)
cargo test --features integration fs_ && cargo test --features integration fs_
# Expected: both invocations green, no setup between them

# 4. MSVC compile + link
clang-cl /c runtime/opal_fs.c (with xwin flags)  # Expected: exit 0
lld-link runtime/opal_fs.obj runtime/opal_msvc_link_probe.obj ...  # Expected: exit 0

# 5. Stub replacement complete
ast-grep --lang c --pattern 'r.error = "not implemented";' runtime/opal_fs.c
# Expected: 0 matches

# 6. Git cleanliness post-test
git status --porcelain  # Expected: empty (no leftover files)

# 7. Old plan untouched
git log --oneline -- .sisyphus/plans/file-io-stdlib-path-object-centric.md
# Expected: most recent commit predates this plan

# 8. .gitattributes correct
git check-attr text -- test-projects/_fs_read_lines_crlf/tests/fixtures/crlf.txt
# Expected: text: unset

# 9. RGR evidence
git log --oneline --grep='^\(red\|green\|refactor\):' | wc -l
# Expected: ≥ 60 (3 per project × 20 projects, minimum)
```

### Final Checklist
- [ ] All "Must Have" present
- [ ] All "Must NOT Have" absent
- [ ] All tests pass (Linux + MSVC compile/link)
- [ ] Re-runnability verified (2 consecutive `cargo test` runs green)
- [ ] FsStateGuard manifest unchanged across runs
- [ ] Old plan untouched
- [ ] User explicit "okay" given after F1-F4 approval
