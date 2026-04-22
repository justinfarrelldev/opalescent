# Windows Support for Opalescent Compiler (MSVC-primary, static LLVM, hot-reload day-one)

## TL;DR

> **Quick Summary**: Make `cargo build` produce a native `opalescent.exe` on Windows AND make the compiler emit Windows `.exe` artifacts (native and cross-compiled from Linux), with zero Linux regression. MSVC is the primary Windows toolchain; LLVM links statically on all platforms; hot-reload becomes a real production feature on both Linux (.so) and Windows (.dll) from day one.
>
> **Deliverables**:
> - `opalescent.exe` builds natively on `windows-latest` via `cargo build --release`
> - `opalescent` on Linux cross-compiles to `x86_64-pc-windows-msvc` via `--target` flag
> - Opalescent compiler emits working `.exe` binaries (tested via Wine on Linux CI, native on Windows CI)
> - Runtime C code portable across glibc + MSVC CRT (MinGW best-effort)
> - Real hot-reload for `.so` (Linux) and `.dll` (Windows) replacing current stub
> - Static LLVM linking on all platforms (removes `llvm14-0-prefer-dynamic`)
> - GitHub Actions CI with three jobs: `ubuntu-latest`, `windows-latest`, `ubuntu-latest + Wine`
> - Zero Linux regressions (full existing `cargo test` suite passes)
>
> **Estimated Effort**: XL (multi-week, ~32+ tasks across 6 waves)
> **Parallel Execution**: YES — 6 waves with strong parallelism in Waves 1, 2, 3
> **Critical Path**: Wave 0 (cargo build Windows green) → Wave 0.5 (expand TargetTriple model for 4-segment Rust triples) → Wave 1 (target abstraction) → Wave 2 (per-target codegen + linker) → Wave 3 (cross-compile + runtime portability + hot-reload) → Final Wave (F1-F4 parallel reviews + F5 Momus review after F1-F4 APPROVE + user okay)

---

## Context

### Original Request

> "Please fix the following issues to have the Opalescent compiler building as a Windows executable and also to have the Opalescent compiler outputting Windows executables after compilation. Linux builds should still work perfectly (as they do now). Use extensive testing to ensure that these work and have regression testing in-place to ensure they work. Use TDD with red-green-refactor where possible. This will likely be an extensive lift. Use Momus for a high-accuracy review - do not even bother asking me about whether we should, just proceed with the Momus review."

### Interview Summary

**Locked Decisions (user-confirmed)**:

- **Windows toolchain**: MSVC primary (link.exe + MSVC CRT), MinGW-w64 best-effort
- **Cross-compile Linux → Windows**: HARD requirement
- **Architecture**: x86_64-pc-windows-msvc ONLY (aarch64 out of scope)
- **Min Windows**: Windows 10 any + Server 2016+
- **Hot-reload on Windows**: SUPPORT DAY-ONE (.dll + LoadLibraryW) — elevates hot-reload from stub to real feature on both platforms
- **LLVM linking**: STATIC on ALL platforms (removes `llvm14-0-prefer-dynamic`)
- **Code signing**: Out of scope
- **`zig cc`**: Banned

### Research Findings (from 4 parallel exploration agents)

**Runtime C portability** (bg_7188ebe1):
- `opal_parse.c`: `_Thread_local`, `snprintf`, `strtoll`, `strtoull`, `strtof` — MSVC shims needed
- `opal_print.c`: `PRId64`/`PRIu64` → `"I64d"`/`"I64u"` fallback on MSVC
- `opal_rc.c` + `opal_rc.h`: hardcoded numeric offsets assume 8-byte pointers → replace with `offsetof()` + `_Static_assert`
- `opal_rng.c`: `/dev/urandom` POSIX-only → add `BCryptGenRandom` branch for Windows
- `opal_runtime.c`: textually `#include`s .c files (pulls POSIX into one TU) → restructure per-platform
- `opal_io.c` + `opal_string.c`: `strdup`, `getline`, `ssize_t` — MSVC reimplementation required
- `opal_bytes.c`, `opal_error.c`, `opal_runtime.h`: portable as-is

**Hardcoding inventory** (bg_f0c5a572):
- Production fixes: `src/compiler.rs:419-420, 618, 623, 307-342` (`.o` suffix, missing EXE_EXTENSION, inline linker Command::new)
- Canonical helpers already exist: `build_system::targets::dynamic_lib_extension`, `hot_reload::version::shared_library_extension`, `versioned_module_name`
- New helpers needed: `object_file_extension`, `executable_filename`, `detect_preferred_linker`
- **`libloading::Library` usage in src/: ZERO** — FsModuleLoader is stub; production hot-reload does not exist yet
- Test/bench `.so` literals are hygiene-only (not blocking)

**Build/CI inventory** (bg_023c2a03):
- `.github/workflows/` DOES NOT EXIST — CI built from scratch for BOTH Linux and Windows
- `Makefile.toml`: `build-all`, `build-cross-all`, `coverage-check`/`coverage-html` are UNGATED → need platform gating
- `Cargo.toml`: `inkwell 0.8.0` with `llvm14-0-prefer-dynamic` → will be removed
- No `build.rs`, no `rust-toolchain.toml`, no `.cargo/config.toml`
- `README.md:65-71`: no Windows instructions
- `scripts/check-line-count.sh`: Bash-only → needs PowerShell variant or CI gating

### Metis Review (bg_abed09dd) — Gaps Closed

**High-severity risks surfaced**:
1. ✅ Object extension must be target-driven (.o vs .obj) — addressed in Wave 1
2. ✅ `getline` needs full MSVC reimplementation (no `_GNU_SOURCE` escape) — Wave 3 runtime task
3. ✅ CRLF translation: Windows CRT translates `\n`→`\r\n` in text mode — test harness normalizes
4. ✅ Console codepage: `SetConsoleOutputCP(CP_UTF8)` at runtime init on Windows
5. ✅ Temp-file locking: close handle before linker opens — Wave 2 linker abstraction
6. ✅ Spaces in paths: always quote linker paths — Wave 2
7. ✅ inkwell prefer-dynamic: removed entirely (static on all platforms)
8. ✅ Wave 0 de-risking spike mandatory before Wave 1
9. ✅ CI matrix locked to 3 jobs (ubuntu-latest, windows-latest, ubuntu-wine)

**Hard directives applied**:
- All new code uses `&TargetTriple` / `TargetSpec`, never `&str`
- Negative test for invalid `--target` string
- Every acceptance criterion is an executable shell command
- TDD RGR per task, one commit per cycle
- Reuse `parse_target_triple` from `src/build_system/targets.rs` (extended in Task 0.5 to accept Rust-style 4-segment triples)
- Never `#[ignore]` a Linux test to unblock Windows work

---

## Work Objectives

### Core Objective

Make the Opalescent compiler build on and emit binaries for `x86_64-pc-windows-msvc` from both native Windows and cross-compiled Linux hosts, with the existing Linux build unaffected and full test coverage enforcing non-regression.

### Concrete Deliverables

- **D1**: `opalescent.exe` produced by `cargo build --release` on `windows-latest` CI runner
- **D2**: `opalescent` on Linux produces valid `.exe` via `opalescent run foo.op --target x86_64-pc-windows-msvc` (verified under Wine)
- **D3**: Native Windows `opalescent.exe` compiles `.op` source to working `.exe` on Windows
- **D4**: New `runtime/opal_portability.h` with MSVC-compatible shims
- **D5**: Real `hot_reload` module (not stub) working with `.so` on Linux AND `.dll` on Windows
- **D6**: `.github/workflows/ci.yml` with three jobs: ubuntu-latest, windows-latest, ubuntu-latest+wine
- **D7**: Linux full test suite (`cargo test --all-features`) PASS both before and after changes (regression gate)
- **D8**: `Cargo.toml` with `inkwell` feature `llvm14-0` only (no `-prefer-dynamic`); static LLVM on all platforms
- **D9**: `README.md` updated with Windows build instructions (MSVC and cross-compile from Linux)
- **D10**: `--target <triple>` CLI flag on `opal build` / `opal run` / `opal check` routed end-to-end

### Definition of Done

- [ ] `cargo test --all-features` → 0 failures on ubuntu-latest (regression gate)
- [ ] `cargo test --all-features` → 0 failures on windows-latest (new green)
- [ ] `cargo build --release --target x86_64-pc-windows-msvc` on ubuntu-latest → produces `opalescent.exe` runnable under Wine
- [ ] On ubuntu-latest+wine: `wine target/x86_64-pc-windows-msvc/release/opalescent.exe run test-projects/hello-world/src/main.op` → prints `Hello world`, exit 0
- [ ] On windows-latest: `target\release\opalescent.exe run test-projects\hello-world\src\main.op` → prints `Hello world`, exit 0
- [ ] Linux artifact `target/release/opalescent` on ubuntu-latest produces working Windows `.exe` via `--target x86_64-pc-windows-msvc` (validated via Wine job)
- [ ] `.github/workflows/ci.yml` passes on main with all 3 jobs green
- [ ] `runtime/opal_portability.h` exists; all `.c` files include it and compile cleanly under the MSVC-target cl driver (native `cl.exe` on Windows; `clang-cl` + xwin sysroot on Linux) AND glibc gcc
- [ ] Hot-reload: `test-projects/hot-reload-demo/` swaps modules live on BOTH Linux (.so) and Windows (.dll) — new integration test
- [ ] No `#[ignore]` directives added to existing Linux tests

### Must Have

- MSVC support as PRIMARY Windows toolchain
- Cross-compile Linux → Windows working in CI via `xwin`
- Static LLVM on all platforms (Linux change validated)
- Real hot-reload on both platforms (Linux .so, Windows .dll)
- `--target <triple>` CLI flag routed through compile pipeline
- Runtime C portability via `opal_portability.h`
- Target-driven object extension (`.o` MinGW, `.obj` MSVC)
- CI with three locked jobs
- `TargetTriple`/`TargetSpec` typed throughout new code (never `&str`)
- Every commit passes existing Linux tests (regression-first)

### Must NOT Have (Guardrails)

- **No `zig cc`**: banned per user decision
- **No aarch64-pc-windows-\***: out of scope
- **No i686 Windows**: out of scope
- **No Windows ≤ 8**: min floor is Win10
- **No code signing**: no `signtool`, no `--sign` flag, no cert handling
- **No LLVM version change** (stays at 14)
- **No inkwell version change** (stays at 0.8.0)
- **No speculative target expansion** (no FreeBSD, musl, wasm additions)
- **No gratuitous trait extraction** for linker (concrete enum, <3 variants)
- **No over-abstracted ToolchainConfig** struct beyond immediate needs
- **No defensive error-handling explosion** (`?` propagation, don't over-catch)
- **No new tracing/logging calls** beyond what already exists
- **No doc-comment bloat** (JSDoc-style paragraphs in Rust)
- **No "rewrite runtime C in more portable style"** — shim only what's actually broken
- **No CI matrix expansion** beyond the 3 locked jobs
- **No `opalescent new` template changes**
- **No unrelated rustfmt churn** on touched files
- **No `#[ignore]` on Linux tests** to unblock Windows
- **No `as`-cast proliferation** in runtime C
- **No unused `cfg` branches** — every `#[cfg(target_os = "windows")]` must have a live call site
- **No generic error types with >3 variants** created for this effort
- **No PDB/debug info handling** for Windows hot-reload (defer; note in README)
- **No long-path (>260) opt-in support** (defer; document limitation)
- **No Unicode path tests** (defer)

---

## Verification Strategy (MANDATORY)

> **ZERO HUMAN INTERVENTION** — ALL verification is agent-executed. No exceptions.
> Acceptance criteria requiring "user manually tests/confirms" are FORBIDDEN.

### Test Decision

- **Infrastructure exists**: YES (`cargo test` + `src/tests.rs` + integration feature)
- **Automated tests**: **TDD RED-GREEN-REFACTOR** (user verbatim)
- **Framework**: `cargo test` (unit + `--features integration` for end-to-end)
- **New CI**: GitHub Actions — 3 jobs (ubuntu-latest, windows-latest, ubuntu-latest+wine)
- **If TDD**: Each task writes failing test first, implements minimum to pass, refactors, commits RGR separately

### QA Policy

Every task MUST include agent-executed QA scenarios. Evidence saved to `.sisyphus/evidence/task-{N}-{scenario-slug}.{ext}`.

- **Compilation / build**: Bash (`cargo build`, `cargo test`) → capture stdout/stderr/exit-code to evidence file
- **Cross-compile**: Bash (`cargo build --target x86_64-pc-windows-msvc`) on Linux, artifact path captured
- **Runtime execution (Linux)**: Bash running compiled binary, stdout compared against golden file
- **Runtime execution (Windows native)**: Bash on Windows runner running `.exe`, stdout normalized for CRLF
- **Runtime execution (cross, via Wine)**: Bash (`wine target/.../opalescent.exe ...`) on ubuntu-latest with Wine preinstalled
- **Runtime C portability**: Bash invoking `cl.exe` (Windows) / `gcc` (Linux) / `x86_64-w64-mingw32-gcc` (MinGW cross) on each `.c` file individually → capture warnings/errors
- **Hot-reload**: Bash orchestrating a host process, modifying a module, asserting swap via log output + function call result
- **CI**: Bash `gh run list --workflow=ci.yml --limit 1 --json conclusion` to assert green

### Regression Gate (MANDATORY on EVERY task touching shared code)

Before marking ANY task complete:

```bash
cargo test --all-features 2>&1 | tee .sisyphus/evidence/task-{N}-linux-regression.log
# Assert: "test result: ok. ... 0 failed"
```

If this fails, the task is incomplete. No exceptions.

---

## Execution Strategy

### Parallel Execution Waves

> Maximize throughput. Each wave completes before the next begins. Target 5-8 tasks per wave.

```
Wave 0 (De-risking spike — 1 task, BLOCKING):
└── Task 0: cargo build on windows-latest GREEN [deep]

Wave 0.5 (Target-triple model expansion — 1 task, BLOCKING Wave 1):
└── Task 0.5: Expand TargetTriple model for Rust 4-segment triples [unspecified-high]

Wave 1 (Foundation — target abstraction, static LLVM, CLI plumbing):
├── Task 1: Extract target types (TargetTriple everywhere, no &str) [unspecified-high]
├── Task 2: object_file_extension + executable_filename helpers [quick]
├── Task 3: detect_preferred_linker enum (MSVC/MinGW/Clang/Cc) [quick]
├── Task 4: --target CLI flag on build/run/check [unspecified-high]
├── Task 5: Remove llvm14-0-prefer-dynamic; switch to static LLVM [deep]
├── Task 6: Gate Makefile.toml Linux-only tasks [quick]
└── Task 7: GitHub Actions CI skeleton (3 jobs, Linux-only test matrix) [unspecified-high]

Wave 2 (Per-target codegen + linker — MAX PARALLEL):
├── Task 8: CodegenContext::for_triple (replace host-hardcoded) [deep]
├── Task 9: Target-driven emit_object_file [unspecified-high]
├── Task 10: LinkerCommand abstraction (replaces inline Command::new) [deep]
├── Task 11: MSVC linker path (link.exe discovery + args) [unspecified-high]
├── Task 12: MinGW linker path (x86_64-w64-mingw32-gcc) [unspecified-high]
├── Task 13: -no-pie host-gated correctly (Linux only) [quick]
└── Task 14: compile_program/compile_project thread target through [deep]

Wave 3 (Cross-compile + runtime portability + hot-reload):
├── Task 15: runtime/opal_portability.h (MSVC shims: thread_local, snprintf, strtoll/ull, strtof) [deep]
├── Task 16: opal_rc.c offsetof() + _Static_assert (replace hardcoded offsets) [unspecified-high]
├── Task 17: opal_rng.c BCryptGenRandom branch for Windows [unspecified-high]
├── Task 18: opal_runtime.c restructure (per-platform aggregator, no .c in .c) [deep]
├── Task 19: opal_io.c / opal_string.c MSVC getline/strdup reimpl [unspecified-high]
├── Task 20: opal_print.c PRId64/PRIu64 fallback [quick]
├── Task 21: xwin integration in Linux CI (cross-compile MSVC) [deep]
├── Task 22: Wine CI job executing cross-compiled .exe [unspecified-high]
├── Task 23: Real FsModuleLoader with libloading (Linux .so) [deep]
├── Task 24: Windows .dll LoadLibraryW hot-reload (copy-before-load) [deep]
└── Task 25: SetConsoleOutputCP(CP_UTF8) at runtime init on Windows [quick]

Wave 4 (Integration + docs):
├── Task 26: Windows native CI job enabled (test + build matrix) [unspecified-high]
├── Task 27: hot-reload-demo integration test (both platforms) [unspecified-high]
├── Task 28: README.md Windows build section (native + cross) [writing]
├── Task 29: scripts/check-line-count.ps1 (PowerShell variant) [quick]
└── Task 30: opal.toml x86_64-windows target docs update [quick]

Wave FINAL-A (4 parallel reviews — ALL must APPROVE):
├── Task F1: Plan compliance audit (oracle)
├── Task F2: Code quality review (unspecified-high)
├── Task F3: Real manual QA (unspecified-high)
└── Task F4: Scope fidelity check (deep)

Wave FINAL-B (after F1-F4 all APPROVE):
└── Task F5: Post-implementation Momus review (Momus - Plan Critic)
→ Present consolidated F1-F5 results → Get explicit user okay

Critical Path: 0 → 0.5 → 1 → 8 → 14 → 15 → 21 → 26 → F1-F4 → F5 → user okay
Parallel Speedup: ~65% faster than sequential
Max Concurrent: 7 (Waves 1 & 3)
```

### Dependency Matrix

- **0**: none → blocks 0.5, 5, 6, 7
- **0.5**: 0 → blocks 1, 2, 8 (target-model-consuming tasks)
- **1**: 0, 0.5 → blocks 4, 8, 9, 10, 11, 12, 14
- **2**: 0, 0.5 → blocks 9, 10, 11
- **3**: 1 → blocks 10, 11, 12
- **4**: 1 → blocks 14
- **5**: 0 → blocks 7, 26
- **6**: 0 → blocks 7
- **7**: 5, 6 → blocks 22, 26
- **8**: 1 → blocks 14
- **9**: 1, 2 → blocks 14
- **10**: 1, 2, 3 → blocks 11, 12, 14
- **11**: 3, 10 → blocks 14, 21
- **12**: 3, 10 → blocks 21
- **13**: 1 → blocks 14
- **14**: 4, 8, 9, 10, 11, 12, 13 → blocks 21, 22, 26, 27
- **15**: 0 → blocks 16, 17, 18, 19, 20, 21
- **16**: 15 → blocks 18
- **17**: 15 → blocks 18, 25
- **18**: 15, 16, 17 → blocks 21, 22
- **19**: 15 → blocks 18
- **20**: 15 → blocks 18
- **21**: 11, 14, 15-20 → blocks 22, 26
- **22**: 7, 14, 21 → blocks 26
- **23**: 14 → blocks 24, 27
- **24**: 23, 15-20 → blocks 27
- **25**: 17 → blocks 22, 26
- **26**: 5, 7, 14, 21, 22, 25 → blocks 27, F1-F4
- **27**: 14, 23, 24 → blocks F1-F4
- **28**: 26 → blocks F1-F4
- **29**: 6 → blocks none
- **30**: 14 → blocks F1-F4
- **F1-F4**: ALL implementation tasks → blocks F5
- **F5**: F1-F4 all APPROVE → blocks user okay

### Agent Dispatch Summary

- **Wave 0**: 1 task — T0 → `deep`
- **Wave 0.5**: 1 task — T0.5 → `unspecified-high`
- **Wave 1**: 7 tasks — T1 → `unspecified-high`, T2-T3 → `quick`, T4 → `unspecified-high`, T5 → `deep`, T6 → `quick`, T7 → `unspecified-high`
- **Wave 2**: 7 tasks — T8 → `deep`, T9 → `unspecified-high`, T10 → `deep`, T11-T12 → `unspecified-high`, T13 → `quick`, T14 → `deep`
- **Wave 3**: 11 tasks — T15 → `deep`, T16-T17 → `unspecified-high`, T18 → `deep`, T19 → `unspecified-high`, T20 → `quick`, T21 → `deep`, T22 → `unspecified-high`, T23-T24 → `deep`, T25 → `quick`
- **Wave 4**: 5 tasks — T26 → `unspecified-high`, T27 → `unspecified-high`, T28 → `writing`, T29-T30 → `quick`
- **Wave FINAL-A**: 4 tasks — F1 → `oracle`, F2 → `unspecified-high`, F3 → `unspecified-high`, F4 → `deep`
- **Wave FINAL-B**: 1 task — F5 → `Momus - Plan Critic` (sequential, after F1-F4 APPROVE)

---

## TODOs

- [ ] 0. **De-risking Spike: `cargo build` on windows-latest GREEN**

  **What to do**:
  - Create minimal `.github/workflows/ci.yml` with ONE job: `build-windows-spike` on `windows-latest`
  - Install LLVM 14 via `choco install llvm --version=14.0.6` OR use pre-built LLVM from `KyleMayes/install-llvm-action@v2` with `version: "14.0"`
  - Set `LLVM_SYS_140_PREFIX`, `LLVM_SYS_140_USE_DEBUG_MSVCRT` (per target profile)
  - Run `cargo build --release` — this WILL likely fail first time due to missing libs, LLVM discovery issues, or `llvm14-0-prefer-dynamic` incompatibility
  - Iterate until green: fix build only (no feature work yet)
  - Commit the working workflow file

  **Must NOT do**:
  - Add any application code changes
  - Add Linux job yet (that comes in Task 7)
  - Implement any Windows functionality (just build the binary)
  - Modify `Cargo.toml` except if absolutely required for LLVM to link (document why)

  **Recommended Agent Profile**:
  - **Category**: `deep` — Goal is open-ended (make cargo build green) with unknown failure modes requiring iteration.
  - **Skills**: [] (no skill covers Windows LLVM setup specifically)

  **Parallelization**:
  - **Can Run In Parallel**: NO
  - **Parallel Group**: Wave 0 solo (blocking spike)
  - **Blocks**: 1, 5, 7
  - **Blocked By**: None

  **References**:
  - `Cargo.toml:1-end` — current inkwell/llvm-sys config; understand what features are set
  - `README.md:65-71` — current Linux-only LLVM setup docs (pattern to mirror)
  - External: https://github.com/KyleMayes/install-llvm-action — CI action for LLVM install
  - External: https://crates.io/crates/llvm-sys — env vars (`LLVM_SYS_140_PREFIX`, `LLVM_SYS_140_USE_DEBUG_MSVCRT`, `LLVM_SYS_140_FFI_WORKAROUND`)
  - External: https://github.com/TheDan64/inkwell — Windows build notes in README / issues
  - Env var reference (from audit): `LLVM_SYS_140_PREFIX`, `LLVM_SYS_140_USE_DEBUG_MSVCRT`, `LLVM_SYS_140_FFI_WORKAROUND`

  **Acceptance Criteria**:
  - [ ] `.github/workflows/ci.yml` exists with `build-windows-spike` job
  - [ ] GitHub Actions run on push to spike branch: `gh run list --workflow=ci.yml --limit 1 --json conclusion --jq '.[0].conclusion'` → `"success"`
  - [ ] Artifact `target/release/opalescent.exe` present in spike run logs (captured via `actions/upload-artifact`)
  - [ ] Linux regression gate on local machine: `cargo test --all-features` → 0 failures
  - [ ] Evidence: `.sisyphus/evidence/task-0-windows-build.log` contains the successful CI run URL and truncated output

  **QA Scenarios**:

  ```
  Scenario: Happy path — Windows cargo build succeeds in CI
    Tool: Bash (gh CLI)
    Preconditions: .github/workflows/ci.yml committed; branch pushed
    Steps:
      1. gh workflow run ci.yml --ref <branch>
      2. gh run watch (wait until completion)
      3. gh run view --log-failed (should have no failures)
      4. gh run download <run-id> --name opalescent-exe --dir .sisyphus/evidence/task-0/
    Expected Result: conclusion="success"; opalescent.exe present in artifacts; file is PE32+ (verify: file opalescent.exe → "PE32+ executable")
    Failure Indicators: conclusion="failure"; any step non-zero exit
    Evidence: .sisyphus/evidence/task-0-windows-build.log, .sisyphus/evidence/task-0/opalescent.exe

  Scenario: Linux regression — local cargo test passes unchanged
    Tool: Bash
    Preconditions: Clean checkout of spike branch on Linux host
    Steps:
      1. cargo clean
      2. cargo test --all-features 2>&1 | tee .sisyphus/evidence/task-0-linux-regression.log
      3. grep -E "test result: ok. [0-9]+ passed; 0 failed" .sisyphus/evidence/task-0-linux-regression.log
    Expected Result: grep finds "0 failed" line; exit 0
    Failure Indicators: Any "FAILED" in output; non-zero exit
    Evidence: .sisyphus/evidence/task-0-linux-regression.log
  ```

  **Commit**: YES (groups with 0) — `chore(ci): de-risk cargo build on windows-latest`

- [x] 0.5. **Expand `TargetTriple` model to support Rust 4-segment triples**

  **What to do**:
  - Extend `src/build_system/targets.rs` to represent Rust-style triples (`x86_64-pc-windows-msvc`, `x86_64-pc-windows-gnu`, `x86_64-unknown-linux-gnu`, `x86_64-unknown-linux-musl`, `x86_64-apple-darwin`) IN ADDITION to the existing 2-segment form (`x86_64-linux`, `aarch64-darwin`, `x86_64-windows`) — backward compatibility is mandatory.
  - Add a new enum (exactly 3 variants, no more):
    ```rust
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum TripleEnv {
        Msvc,   // *-windows-msvc
        Gnu,    // *-windows-gnu, *-linux-gnu
        Musl,   // *-linux-musl
    }
    ```
  - Extend `TargetTriple` struct to add `pub env: Option<TripleEnv>` field. `None` = platform default (Linux=Gnu, Windows=Msvc, macOS=N/A). This is additive — existing `arch` + `platform` fields UNCHANGED.
  - Rewrite `parse_target_triple` to accept BOTH forms:
    - 2-segment (existing): `arch-platform` → `TargetTriple { arch, platform, env: None }` (preserves all existing call sites). When `platform == Windows && env == None`, downstream toolchain resolution (Tasks 3, 10, 11, 12) MUST treat the target AS-IF `env == Some(Msvc)` AND the parser MUST emit a ONE-TIME stderr deprecation warning: `warning: target "x86_64-windows" is deprecated; use "x86_64-pc-windows-msvc" or "x86_64-pc-windows-gnu" explicitly`. This is the ONE sanctioned use of bare `x86_64-windows` — all NEW code/docs/tests MUST use the 4-segment form (see Task 30 Must-Not).
    - 4-segment (new): `arch-vendor-os-env` where `vendor ∈ {pc, unknown, apple}`, `os ∈ {windows, linux, darwin}`, `env ∈ {msvc, gnu, musl}`
    - Explicitly allow `x86_64-pc-windows-msvc`, `x86_64-pc-windows-gnu`, `x86_64-unknown-linux-gnu`, `x86_64-unknown-linux-musl`, `x86_64-apple-darwin`
    - Explicitly REJECT unsupported combinations with existing `BuildError::InvalidTarget(input)` (e.g. `aarch64-pc-windows-msvc` — out of scope per locked decision)
  - Add `impl TargetTriple`:
    - `pub fn is_windows_msvc(&self) -> bool` → `platform == Windows && env == Some(Msvc)`
    - `pub fn is_windows_gnu(&self) -> bool` → `platform == Windows && env == Some(Gnu)`
    - `pub fn is_windows(&self) -> bool` → `platform == Windows` (env-agnostic)
    - `pub fn host() -> Self` — maps Rust `cfg!(target_os=...)` + `cfg!(target_env=...)` to a TargetTriple
    - `pub fn to_rust_triple(&self) -> String` — canonical 4-segment form for passing to `cargo build --target <X>` (e.g. Linux+Gnu → `"x86_64-unknown-linux-gnu"`)
  - RED: write failing tests BEFORE implementation:
    - `parse_rust_msvc_triple`: `parse_target_triple("x86_64-pc-windows-msvc")` → `Ok(triple)` with `env == Some(Msvc)`
    - `parse_rust_mingw_triple`: `"x86_64-pc-windows-gnu"` → `env == Some(Gnu)`
    - `parse_legacy_2_segment_still_works`: `"x86_64-windows"` → `env == None`, platform Windows; captured stderr contains the substring `"deprecated"`.
    - `parse_legacy_windows_resolves_as_msvc`: `parse_target_triple("x86_64-windows").unwrap().is_windows_msvc()` returns `true` via the documented fallback (either `is_windows_msvc` returns true when `env == None && platform == Windows`, OR a sibling method `effective_env()` returns `TripleEnv::Msvc`).
    - `parse_legacy_linux_still_works`: `"x86_64-linux"` → `env == None`, platform Linux
    - `reject_aarch64_windows_msvc`: `"aarch64-pc-windows-msvc"` → `Err(InvalidTarget)` (locked out-of-scope)
    - `reject_3_segment`: `"x86_64-unknown-linux"` → `Err` (ambiguous; require full 4-seg or legacy 2-seg)
    - `reject_unknown_env`: `"x86_64-pc-windows-clang"` → `Err`
    - `to_rust_triple_roundtrips`: parse then `to_rust_triple` returns canonical form
  - GREEN: implement parser branching on segment count (2 vs 4)
  - REFACTOR: extract `parse_env_segment` private helper; ensure all existing 2-segment tests still pass

  **Must NOT do**:
  - Remove the 2-segment form from `parse_target_triple` (breaking change — forbidden)
  - Add `aarch64-pc-windows-*` support (locked out-of-scope per user decision)
  - Add `i686-*` / `armv7-*` support (out-of-scope)
  - Add `TripleVendor` enum — we accept `pc`/`unknown`/`apple` as flavor-text only; correctness is determined by (arch, platform, env)
  - Add `FromStr for TargetTriple` (existing `parse_target_triple` is the canonical entry point)
  - Add `env` to `Display` impl without also updating any snapshot tests (audit with `rg "format!.*TargetTriple|\\{triple\\}" src/ tests/` first)
  - Introduce `AbI` as a separate concept from `env` — stick to Rust's `target_env` naming
  - Change `BuildError::InvalidTarget` variant signature

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high` — Data-model extension with strict backward-compat + exhaustive parser tests.
  - **Skills**: [`ai-slop-remover`] — Call during REFACTOR to ensure no defensive-check explosion in the parser.

  **Parallelization**:
  - **Can Run In Parallel**: NO — unblocks Task 1 and all downstream model usage.
  - **Parallel Group**: Wave 1 solo critical path (runs after Task 0 spike, before Task 1 refactor)
  - **Blocks**: 1, 2, 3, 4, 8
  - **Blocked By**: 0

  **References**:
  - `src/build_system/targets.rs:7-16` — existing `Platform` enum (DO NOT modify variants; only add methods)
  - `src/build_system/targets.rs:18-25` — existing `Architecture` enum (DO NOT modify variants)
  - `src/build_system/targets.rs:28-34` — existing `TargetTriple` struct (EXTEND with `env` field)
  - `src/build_system/targets.rs:49-82` — existing `parse_target_triple` (REWRITE to dispatch on segment count; keep 2-seg path bit-identical)
  - `src/build_system/targets.rs:84-91` — existing `dynamic_lib_extension` (keep unchanged — it's `env`-agnostic today)
  - `src/build_system.rs:13` (`pub mod targets;`) and `src/build_system.rs:19` (`pub use targets::{...};`) — re-export block; must export new `TripleEnv` too (NOTE: this repo uses `src/build_system.rs` as the module file, NOT `src/build_system/mod.rs`)
  - External: https://doc.rust-lang.org/rustc/platform-support.html — canonical list of Rust target triples we may need
  - External: https://doc.rust-lang.org/reference/conditional-compilation.html#target_env — `cfg!(target_env)` mapping used by `host()`
  - Metis directive (draft line 168): "Use `&TargetTriple` ... NOT `&str`" — depends on this model being expressive enough

  **Acceptance Criteria**:
  - [ ] `src/build_system/targets.rs` compiles; `pub enum TripleEnv { Msvc, Gnu, Musl }` exists
  - [ ] `TargetTriple` has new `env: Option<TripleEnv>` field (additive)
  - [ ] All 8 RED tests listed above exist AND pass after GREEN
  - [ ] `cargo test --all-features build_system::targets` shows ≥8 test names containing `parse_rust_*`, `parse_legacy_*`, `reject_*`, `to_rust_triple_*`, ALL "ok"
  - [ ] `cargo test --all-features` → 0 failures (ZERO regression — any existing test touching `TargetTriple` must still pass)
  - [ ] `cargo clippy --all-targets --all-features -- -D warnings` clean
  - [ ] `cargo build --release` succeeds on Linux (host-build regression gate)

  **QA Scenarios**:

  ```
  Scenario: Happy path — 4-segment Rust triples parse to correct env
    Tool: Bash
    Preconditions: Task 0.5 GREEN commit applied
    Steps:
      1. cargo test --all-features build_system::targets::tests 2>&1 | tee .sisyphus/evidence/task-0_5-unit.log
      2. grep -E "test .*parse_rust_msvc_triple.*ok" .sisyphus/evidence/task-0_5-unit.log
      3. grep -E "test .*parse_rust_mingw_triple.*ok" .sisyphus/evidence/task-0_5-unit.log
      4. grep -E "test .*parse_legacy_2_segment_still_works.*ok" .sisyphus/evidence/task-0_5-unit.log
      5. grep -E "test .*to_rust_triple_roundtrips.*ok" .sisyphus/evidence/task-0_5-unit.log
    Expected Result: All four greps succeed with exit 0
    Failure Indicators: Any grep returns no match; any "FAILED" line in log
    Evidence: .sisyphus/evidence/task-0_5-unit.log

  Scenario: Failure — locked out-of-scope triples rejected
    Tool: Bash
    Preconditions: Task 0.5 applied
    Steps:
      1. cargo test --all-features build_system::targets::tests::reject_aarch64_windows_msvc 2>&1 | tee .sisyphus/evidence/task-0_5-reject-aarch64.log
      2. grep -E "test .*reject_aarch64_windows_msvc.*ok" .sisyphus/evidence/task-0_5-reject-aarch64.log
      3. cargo test --all-features build_system::targets::tests::reject_unknown_env 2>&1 | tee .sisyphus/evidence/task-0_5-reject-env.log
      4. grep -E "test .*reject_unknown_env.*ok" .sisyphus/evidence/task-0_5-reject-env.log
    Expected Result: Both assertions pass — parser returns `Err(InvalidTarget)` for out-of-scope inputs
    Failure Indicators: Parser accepts `aarch64-pc-windows-msvc` (violates locked scope decision)
    Evidence: .sisyphus/evidence/task-0_5-reject-aarch64.log, .sisyphus/evidence/task-0_5-reject-env.log

  Scenario: Backward compatibility — existing 2-segment callers unchanged
    Tool: Bash
    Preconditions: Task 0.5 applied
    Steps:
      1. rg "parse_target_triple\\(" src/ tests/ --type rust | tee .sisyphus/evidence/task-0_5-callsites.log
      2. cargo test --all-features 2>&1 | tee .sisyphus/evidence/task-0_5-full-regression.log
      3. grep -E "test result: ok. [0-9]+ passed; 0 failed" .sisyphus/evidence/task-0_5-full-regression.log
    Expected Result: Every existing call site still compiles AND every existing test passes (ZERO regression)
    Failure Indicators: Any "FAILED" line; any compile error in step 2
    Evidence: .sisyphus/evidence/task-0_5-callsites.log, .sisyphus/evidence/task-0_5-full-regression.log
  ```

  **Commit**: YES (RGR cycle: 3 commits) — `feat(targets): support Rust 4-segment target triples with env (msvc/gnu/musl)`

- [x] 1. **Replace `&str` with `TargetTriple` in public API**

  **What to do**:
  - Audit `src/compiler.rs`, `src/build_system/*.rs`, `src/codegen/*.rs` for `target_os: &str` / `target: &str` params
  - Replace with `&TargetTriple` (from `src/build_system/targets.rs`, expanded in Task 0.5)
  - Update callers to construct `TargetTriple` via existing `parse_target_triple` (reuse, do not reinvent)
  - Add conversion `impl TargetTriple { pub fn host() -> Self }` if missing
  - RED: write failing test `tests_target_triple_typed_api` asserting signatures use `TargetTriple`
  - GREEN: change signatures
  - REFACTOR: remove now-unused `&str` helpers

  **Must NOT do**:
  - Create `TargetSpec` trait yet (only struct refactor)
  - Change `TargetTriple` struct fields in this task (Task 0.5 already added `env`; this task only threads the type through)
  - Touch CLI parsing in this task (Task 4 handles that)
  - Add `Into<TargetTriple> for &str` convenience impl (explicit construction only)

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high` — Mechanical refactor across multiple files, modest complexity.
  - **Skills**: [`ai-slop-remover`] — Will be called during REFACTOR step to clean up the diff.

  **Parallelization**:
  - **Can Run In Parallel**: NO within Wave 1 (blocks 4, 8-14)
  - **Parallel Group**: Wave 1 solo critical path
  - **Blocks**: 4, 8, 9, 10, 11, 12, 14
  - **Blocked By**: 0, 0.5

  **References**:
  - `src/build_system/targets.rs:86-91` — `dynamic_lib_extension()` signature pattern (actual file, 92 lines total)
  - `src/build_system/targets.rs:49-82` — `parse_target_triple` (extended in Task 0.5 to support 4-segment Rust triples; reuse that extended parser here)
  - `src/compiler.rs:307-342` — inline `Command::new` site currently accepts untyped target info
  - `src/build_system/targets.rs` — existing typed helpers pattern to match
  - Metis directive (from draft line 168): "Use `&TargetTriple` or `TargetSpec`, NOT `&str`, in new code"

  **Acceptance Criteria**:
  - [ ] `rg "target(_os)?: &str" src/ --type rust` returns 0 matches (excluding tests that specifically test parsing)
  - [ ] `cargo build --all-features` succeeds
  - [ ] `cargo test --all-features` → 0 failures (Linux regression gate)
  - [ ] New test `tests_target_triple_typed_api` exists and passes
  - [ ] `cargo clippy --all-targets --all-features -- -D warnings` clean

  **QA Scenarios**:

  ```
  Scenario: Typed API enforced
    Tool: Bash
    Preconditions: Branch with Task 1 changes
    Steps:
      1. rg "target(_os)?: &str" src/ --type rust | tee .sisyphus/evidence/task-1-grep.log
      2. Assert output excludes src/ (only acceptable: tests of target parsing)
      3. cargo test --all-features 2>&1 | tee .sisyphus/evidence/task-1-tests.log
      4. grep "0 failed" .sisyphus/evidence/task-1-tests.log
    Expected Result: grep (step 1) prints only whitelisted parsing test files; step 4 finds "0 failed"
    Evidence: .sisyphus/evidence/task-1-grep.log, .sisyphus/evidence/task-1-tests.log

  Scenario: Failure — attempting &str to compile_program rejected at compile time
    Tool: Bash
    Preconditions: Task 1 applied; repo is clean (no uncommitted changes)
    Steps:
      1. TMPDIR=$(mktemp -d) && trap "rm -rf $TMPDIR" EXIT
      2. Create standalone reproducer OUTSIDE the workspace: cat > $TMPDIR/str_target.rs <<'RS'
           // Standalone reproducer — compiled out-of-tree via `rustc --edition 2024 --extern opalescent=<path-to-rlib>`
           // DO NOT place in tests/ directory — would poison `cargo build --tests`
           fn main() { opalescent::compile_program("src", "out"); }
         RS
      3. First build the opalescent rlib: cargo build --lib --message-format=json 2>&1 | tee $TMPDIR/cargo.json
      4. Extract rlib path: RLIB=$(jq -r 'select(.reason=="compiler-artifact" and (.target.kind | index("lib"))) | .filenames[]' $TMPDIR/cargo.json | grep libopalescent | tail -1)
      5. Attempt to compile reproducer: rustc --edition 2024 --extern opalescent=$RLIB -L target/debug/deps $TMPDIR/str_target.rs -o $TMPDIR/out 2>&1 | tee .sisyphus/evidence/task-1-compile-fail.log ; echo "rustc_exit=$?" >> .sisyphus/evidence/task-1-compile-fail.log
      6. grep -E "(expected .*TargetTriple|mismatched types)" .sisyphus/evidence/task-1-compile-fail.log
      7. grep "rustc_exit=1" .sisyphus/evidence/task-1-compile-fail.log (confirms rustc failed as expected)
      8. Verify no stray files committed: git status --short | grep -v "^??" | grep -E "(str_target|compile_fail)" && exit 1 || true
    Expected Result: rustc exits non-zero with diagnostic mentioning `TargetTriple` / `mismatched types`; no files left in workspace after trap cleanup; `git status` shows no new tracked-path changes from the reproducer
    Failure Indicators: rustc compiles the reproducer (type-system regression); any file named `str_target.rs` exists inside `tests/` or `src/` after run
    Evidence: .sisyphus/evidence/task-1-compile-fail.log
    Cleanup: trap on EXIT removes $TMPDIR; reproducer NEVER enters the git working tree
  ```

  **Commit**: YES (RGR cycle: 3 commits) — `refactor(targets): replace &str with TargetTriple in public API`

- [x] 2. **`object_file_extension` + `executable_filename` helpers**

  **What to do**:
  - Add to `src/build_system/targets.rs`:
    - `pub fn object_file_extension(target: &TargetTriple) -> &'static str` (`.obj` for `*-windows-msvc`, `.o` otherwise including `*-windows-gnu`)
    - `pub fn executable_filename(stem: &str, target: &TargetTriple) -> String` (adds `.exe` for `*-windows-*`, nothing otherwise)
  - Prefer `std::env::consts::EXE_EXTENSION` when target == host; for cross, use target-driven logic
  - Add rustdoc `# Examples` to both helpers documenting the `env == None` legacy fallback behavior (legacy windows with `env == None` resolves as MSVC per Task 0.5)
  - RED: test `object_file_extension_windows_msvc` expects `.obj`
  - GREEN: implement
  - REFACTOR: extract internal match into private helper
  - Add unit test `object_file_extension_legacy_fallbacks` to `src/build_system/tests.rs` with these exact assertions (tests closed-variant fallback path; relies on Task 0.5 legacy resolution):
    - `object_file_extension(&parse_target_triple("x86_64-windows").unwrap())` → `".obj"` (legacy windows resolves as MSVC per Task 0.5)
    - `object_file_extension(&parse_target_triple("x86_64-linux").unwrap())` → `".o"`
    - `object_file_extension(&parse_target_triple("aarch64-darwin").unwrap())` → `".o"`
    - `executable_filename("prog", &parse_target_triple("x86_64-windows").unwrap())` → `"prog.exe"`
    - `executable_filename("prog", &parse_target_triple("x86_64-linux").unwrap())` → `"prog"`

  **Must NOT do**:
  - Create a `Platform` enum (use existing `TargetTriple`)
  - Hardcode strings in consumers (they must call these helpers in Task 9, 10, 14)
  - Support `.lib`, `.dylib`, `.a` (not needed for this task)

  **Recommended Agent Profile**:
  - **Category**: `quick` — Small pure helper functions, clear signatures.
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES with tasks 3, 4, 5, 6, 7 in Wave 1
  - **Parallel Group**: Wave 1 (with 3, 4, 5, 6, 7)
  - **Blocks**: 9, 10, 11
  - **Blocked By**: 0, 0.5

  **References**:
  - `src/build_system/targets.rs:86-91` — `dynamic_lib_extension()` as pattern to mirror (actual file, 92 lines total)
  - `src/hot_reload/version.rs:42-52` — `shared_library_extension()` alternate pattern (cfg-based; this task uses target-driven instead)
  - `std::env::consts::EXE_EXTENSION` — stdlib helper for host case
  - `src/compiler.rs:419-420` — current hardcoded `.o` and missing EXE_EXTENSION (Task 9 and 14 will fix to use these new helpers)

  **Acceptance Criteria**:
  - [ ] `src/build_system/targets.rs` contains both functions with documented behavior
  - [ ] Unit tests: `object_file_extension_windows_msvc` → `.obj`, `_windows_gnu` → `.o`, `_linux` → `.o`, `_darwin` → `.o`
  - [ ] Unit tests: `executable_filename("opalescent", windows_msvc)` → `"opalescent.exe"`, `(..., linux)` → `"opalescent"`
  - [ ] `cargo test --all-features` → 0 failures

  **QA Scenarios**:

  ```
  Scenario: Happy path — helpers return correct extensions per target
    Tool: Bash
    Preconditions: Task 2 applied
    Steps:
      1. cargo test --all-features build_system::targets::tests 2>&1 | tee .sisyphus/evidence/task-2-unit.log
      2. grep -E "test object_file_extension_windows_msvc.*ok" .sisyphus/evidence/task-2-unit.log
      3. grep -E "test executable_filename_windows_msvc.*ok" .sisyphus/evidence/task-2-unit.log
    Expected Result: Both greps find "ok"
    Evidence: .sisyphus/evidence/task-2-unit.log

  Scenario: Legacy 2-segment triples resolve to correct extensions (closed-variant fallback test)
    Tool: Bash
    Preconditions: Task 2 applied (helpers + `object_file_extension_legacy_fallbacks` test authored per "What to do"); Task 0.5 applied (legacy env=None resolves windows as MSVC).
    Steps:
      1. Verify the test exists in source: `grep -n "fn object_file_extension_legacy_fallbacks" src/build_system/tests.rs | tee .sisyphus/evidence/task-2-fallback-source.log` — assert exit 0 (test authored).
      2. Run the test: `cargo test --all-features build_system::tests::object_file_extension_legacy_fallbacks 2>&1 | tee .sisyphus/evidence/task-2-fallback.log`
      3. Assert pass: `grep -E "test object_file_extension_legacy_fallbacks.*ok" .sisyphus/evidence/task-2-fallback.log`
      4. Assert `# Examples` rustdoc exists on both helpers: `cargo doc --no-deps --document-private-items 2>&1 | tee .sisyphus/evidence/task-2-doc.log` then `grep -E "(object_file_extension|executable_filename)" target/doc/opalescent/build_system/targets/*.html | grep -q "Examples"` (non-zero exit fails the scenario).
    Expected Result: Test file contains the named fn (step 1); cargo test reports `ok` for that test (step 3); rustdoc-generated HTML contains `Examples` sections for both helpers (step 4).
    Failure Indicators: Test fn missing from source; any assertion returns wrong extension; test fails or panics; rustdoc missing `Examples` section.
    Evidence: .sisyphus/evidence/task-2-fallback-source.log, .sisyphus/evidence/task-2-fallback.log, .sisyphus/evidence/task-2-doc.log
  ```

  **Commit**: YES — `feat(build_system): add object_file_extension + executable_filename`

- [x] 3. **`detect_preferred_linker` enum**

  **What to do**:
  - Create `src/build_system/linker.rs` (new module)
  - Define `pub enum Linker { Msvc, MinGw, Clang, Cc }` (exactly 4 variants, no more)
  - `pub fn detect_preferred_linker(target: &TargetTriple) -> Linker`:
    - `*-windows-msvc` → `Msvc`
    - `*-windows-gnu` → `MinGw`
    - `*-linux-gnu` / `*-linux-musl` → `Cc` (prefer `cc`, fall back to `gcc`)
    - `*-darwin` → `Clang`
  - RED: test asserts Windows MSVC target → `Linker::Msvc`
  - GREEN: implement
  - REFACTOR: `impl Linker { pub fn binary_name(&self) -> &'static str }` for human-readable probe

  **Must NOT do**:
  - Add more than 4 variants (no `Lld`, `Gold`, `Mold` — those are stretch goals deferred)
  - Probe the filesystem in this task (Tasks 10-12 handle invocation)
  - Create a trait (concrete enum only per plan guardrail)
  - Add `Linker::from_str` (callers use TargetTriple, not strings)

  **Recommended Agent Profile**:
  - **Category**: `quick` — Small enum + single pure function.
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES with tasks 2, 4, 5, 6, 7 in Wave 1
  - **Parallel Group**: Wave 1
  - **Blocks**: 10, 11, 12
  - **Blocked By**: 1

  **References**:
  - `src/compiler.rs:307-342` — current inline `Command::new("link.exe"/"gcc"/"cc")` logic; captures decision matrix to encode
  - `src/compiler.rs:307-313` — current `link.exe /?` probe (do NOT replicate; detect via target, not runtime probe)
  - `src/build_system/targets.rs` — module where `TargetTriple` lives; new `linker.rs` sits alongside
  - Metis directive: "Never `#[ignore]` a Linux test to unblock Windows work" — Linux case must stay `Cc`

  **Acceptance Criteria**:
  - [ ] `src/build_system/linker.rs` exists with `Linker` enum + `detect_preferred_linker` function
  - [ ] `pub use` in `src/build_system.rs:19` re-export block exposes both `TargetTriple` and `TripleEnv`
  - [ ] Unit tests cover all 4 variants
  - [ ] `cargo test --all-features` → 0 failures
  - [ ] `cargo clippy --all-targets --all-features -- -D warnings` clean

  **QA Scenarios**:

  ```
  Scenario: Happy path — each target resolves to correct linker
    Tool: Bash
    Preconditions: Task 3 applied
    Steps:
      1. cargo test --all-features build_system::linker 2>&1 | tee .sisyphus/evidence/task-3-unit.log
      2. grep -cE "test .* ok" .sisyphus/evidence/task-3-unit.log
    Expected Result: At least 4 "ok" tests (one per Linker variant)
    Evidence: .sisyphus/evidence/task-3-unit.log

  Scenario: Failure — compilation rejects non-Linker values where Linker expected
    Tool: Bash
    Preconditions: Task 3 applied; workspace clean
    Steps:
      1. TMPDIR=$(mktemp -d) && trap "rm -rf $TMPDIR" EXIT
      2. Create out-of-tree reproducer (NEVER committed): cat > $TMPDIR/str_linker.rs <<'RS'
           // Out-of-tree negative test — asserts Linker enum cannot be satisfied by &str.
           // Placed in TMPDIR, not tests/, so `cargo build --tests` stays green.
           fn takes_linker(l: opalescent::build_system::linker::Linker) { let _ = l; }
           fn main() { takes_linker("msvc"); }
         RS
      3. cargo build --lib --message-format=json 2>&1 | tee $TMPDIR/cargo.json
      4. RLIB=$(jq -r 'select(.reason=="compiler-artifact" and (.target.kind | index("lib"))) | .filenames[]' $TMPDIR/cargo.json | grep libopalescent | tail -1)
      5. rustc --edition 2024 --extern opalescent=$RLIB -L target/debug/deps $TMPDIR/str_linker.rs -o $TMPDIR/out 2>&1 | tee .sisyphus/evidence/task-3-compile-fail.log ; echo "rustc_exit=$?" >> .sisyphus/evidence/task-3-compile-fail.log
      6. grep -E "(expected .*Linker|expected enum|mismatched types)" .sisyphus/evidence/task-3-compile-fail.log
      7. grep "rustc_exit=1" .sisyphus/evidence/task-3-compile-fail.log
      8. git status --short | grep -E "(str_linker|compile_fail)" && exit 1 || true
    Expected Result: rustc exits non-zero with `expected Linker` / `mismatched types` in diagnostic; no files left in workspace
    Failure Indicators: rustc accepts `"msvc"` as `Linker` (type-safety regression); any committed artifact from the reproducer
    Evidence: .sisyphus/evidence/task-3-compile-fail.log
    Cleanup: trap on EXIT removes $TMPDIR; reproducer file never enters repo
  ```

  **Commit**: YES — `feat(build_system): detect_preferred_linker enum`

- [x] 4. **`--target <triple>` CLI flag on `build`/`run`/`check`**

  **What to do**:
  - Add `--target <triple>` to CLI surface for `opal build`, `opal run`, `opal check`, `opal <file.op>` direct invocation
  - Parse via existing `parse_target_triple` (reuse)
  - Default to host triple when flag absent
  - On invalid triple: exit 1 with message `"unknown target triple: <input>. Supported: x86_64-linux, x86_64-pc-windows-msvc, ..."`
  - RED: test `cli_rejects_invalid_target` expects exit 1 with "unknown target triple"
  - GREEN: plumb through `src/app.rs` / `src/cli.rs`
  - REFACTOR: extract triple parsing into a single helper used by all 4 subcommands

  **Must NOT do**:
  - Add `--target` to `opal fmt`, `opal lsp`, `opal pkg`, `opal doc`, `opal bench` (not in scope)
  - Auto-correct typos (e.g. suggest "x86_64-windows-msvc" for "x86_windows")
  - Add target to `opal.toml` precedence resolution in this task (existing `[build].targets` array remains as-is)
  - Change the existing `opal test --target` flag (already exists; keep as-is)

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high` — CLI surface change touching multiple subcommands + error handling.
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES with 2, 3, 5, 6, 7
  - **Parallel Group**: Wave 1
  - **Blocks**: 14
  - **Blocked By**: 1

  **References**:
  - `src/app.rs` — current CLI dispatch (grep for subcommand handlers)
  - `src/cli.rs` (if exists) or inline in `src/app.rs` — argument parsing
  - README.md:140-150 — `opal test --target` exists; follow same convention
  - `src/build_system/targets.rs:49-82` — `parse_target_triple` (accepts 4-segment Rust triples after Task 0.5)
  - Metis directive: "Negative test for invalid `--target` string"

  **Acceptance Criteria**:
  - [ ] `opalescent run foo.op --target x86_64-pc-windows-msvc` accepts the flag (wired to compile pipeline via Task 14)
  - [ ] `opalescent run foo.op --target invalid` → exit 1, stderr contains `"unknown target triple"`
  - [ ] `opalescent build --target x86_64-pc-windows-msvc` accepted (no-op until Task 14 threads it through)
  - [ ] `opalescent check foo.op --target x86_64-pc-windows-msvc` accepted
  - [ ] `cargo test --all-features` → 0 failures
  - [ ] Help text for each subcommand includes `--target` line

  **QA Scenarios**:

  ```
  Scenario: Happy path — valid triple accepted across all 4 subcommands
    Tool: Bash
    Preconditions: Task 4 applied; cargo build --release
    Steps:
      1. ./target/release/opalescent run test-projects/hello-world/src/main.op --target x86_64-linux 2>&1 | tee .sisyphus/evidence/task-4-run.log
         (will fail at codegen until Task 14; acceptable here — we just validate parsing)
      2. ./target/release/opalescent build --target x86_64-pc-windows-msvc 2>&1 | tee .sisyphus/evidence/task-4-build.log
      3. ./target/release/opalescent check test-projects/hello-world/src/main.op --target x86_64-linux 2>&1 | tee .sisyphus/evidence/task-4-check.log
      4. Assert each log does NOT contain "unknown target triple"
    Expected Result: No parse failures; any errors come from downstream (acceptable pre-Task-14)
    Evidence: .sisyphus/evidence/task-4-{run,build,check}.log

  Scenario: Failure — invalid triple rejected with helpful message (MANDATORY negative test)
    Tool: Bash
    Preconditions: Task 4 applied
    Steps:
      1. ./target/release/opalescent run foo.op --target banana-pi-linux ; echo "exit=$?" | tee .sisyphus/evidence/task-4-negative.log
      2. ./target/release/opalescent run foo.op --target banana-pi-linux 2>&1 | grep -q "unknown target triple"
    Expected Result: step 1 exit=1; step 2 grep exit 0
    Evidence: .sisyphus/evidence/task-4-negative.log
  ```

  **Commit**: YES (RGR: 3 commits) — `feat(cli): --target flag on build/run/check`

- [x] 5. **Switch to static LLVM on ALL platforms (remove `llvm14-0-prefer-dynamic`)**

  **What to do**:
  - Edit `Cargo.toml`: change `inkwell` features from `["llvm14-0", "llvm14-0-prefer-dynamic"]` to `["llvm14-0"]`
  - Verify `LLVM_SYS_140_PREFIX` points to a location with static libs (`.a` on Linux, `.lib` on Windows)
  - Update `README.md:65-71` LLVM install instructions: note static libs required
  - RED: test not applicable (build-system change); use integration-level check
  - GREEN: change Cargo.toml, run `cargo build --release` on Linux, assert binary is statically linked against LLVM (verify: `ldd target/release/opalescent | grep -c libLLVM` → 0)
  - REFACTOR: none

  **Must NOT do**:
  - Upgrade LLVM version
  - Upgrade inkwell version
  - Add `build.rs` custom linker args (inkwell should handle it)
  - Add `[profile.release]` flags (no LTO/strip changes in this task)
  - Change MacOS behavior (not in scope — keep LLVM 14 static there too per user "all platforms")

  **Recommended Agent Profile**:
  - **Category**: `deep` — Build-system change with potential cascading failures (LLVM discovery, static lib availability, binary size impact). Requires iteration.
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES with 2, 3, 4, 6, 7
  - **Parallel Group**: Wave 1
  - **Blocks**: 7, 26
  - **Blocked By**: 0

  **References**:
  - `Cargo.toml` — current inkwell feature list (verify before editing)
  - `README.md:65-71` — current LLVM install docs (update to mention static libs)
  - External: https://crates.io/crates/llvm-sys — documents prefer-dynamic semantics
  - External: https://github.com/TheDan64/inkwell#llvm-compatibility — static vs dynamic tradeoffs
  - Metis risk #1: "inkwell llvm14-0-prefer-dynamic on Windows requires shipping LLVM DLLs" (resolved by this task)

  **Acceptance Criteria**:
  - [ ] `grep "llvm14-0-prefer-dynamic" Cargo.toml` → 0 matches
  - [ ] `cargo build --release` succeeds on Linux
  - [ ] Linux binary: `ldd target/release/opalescent | grep -c "libLLVM"` → 0 (statically linked)
  - [ ] `cargo test --all-features` → 0 failures (regression gate — tests still pass with static)
  - [ ] Binary size documented in evidence (before/after comparison)
  - [ ] README updated with static LLVM requirement

  **QA Scenarios**:

  ```
  Scenario: Happy path — Linux binary is statically linked against LLVM
    Tool: Bash
    Preconditions: Task 5 applied; `cargo clean && cargo build --release` completed
    Steps:
      1. ldd target/release/opalescent 2>&1 | tee .sisyphus/evidence/task-5-ldd.log
      2. grep -c "libLLVM" .sisyphus/evidence/task-5-ldd.log
      3. ls -lh target/release/opalescent | tee .sisyphus/evidence/task-5-size.log
    Expected Result: step 2 → "0"; binary exists; size logged (expect larger than pre-task baseline, document delta)
    Evidence: .sisyphus/evidence/task-5-ldd.log, .sisyphus/evidence/task-5-size.log

  Scenario: Happy path — Linux tests still pass (regression gate)
    Tool: Bash
    Preconditions: Task 5 applied
    Steps:
      1. cargo test --all-features 2>&1 | tee .sisyphus/evidence/task-5-regression.log
      2. grep -E "test result: ok.*0 failed" .sisyphus/evidence/task-5-regression.log
    Expected Result: grep finds match
    Evidence: .sisyphus/evidence/task-5-regression.log

  Scenario: Failure — missing static LLVM libs produces clear error
    Tool: Bash
    Preconditions: Task 5 applied; LLVM_SYS_140_PREFIX points to dyn-only install
    Steps:
      1. LLVM_SYS_140_PREFIX=/tmp/fake-llvm-no-static cargo build --release 2>&1 | tee .sisyphus/evidence/task-5-missing-static.log
    Expected Result: build fails with a message mentioning static libs OR linker error referencing libLLVM*.a / LLVM*.lib
    Evidence: .sisyphus/evidence/task-5-missing-static.log
  ```

  **Commit**: YES — `build(llvm): switch to static LLVM on all platforms`

- [x] 6. **Gate Linux-only tasks in `Makefile.toml`**

  **What to do**:
  - Audit `Makefile.toml` for tasks missing `[tasks.X.linux]` / `platforms = ["linux"]` gating
  - Specifically: `build-all` (L58-60), `build-cross-all` (L64-69), `coverage-check` (L89-92), `coverage-html` (L93-96)
  - For `build-all`: split into `build-all-linux` (already gated) and `build-all-windows` (already gated), make `build-all` a dispatch task that routes by host
  - For `build-cross-all`: gate to Linux (cross-compile from Linux is the use case; Windows host doesn't cross-compile to Linux in this effort)
  - For `coverage-*` tasks: gate `platforms = ["linux"]` (tarpaulin is Linux-only, ptrace-based)
  - RED: add integration test that parses `Makefile.toml` and asserts gating (using `cargo-make` dry-run or YAML parse)
  - GREEN: edit `Makefile.toml`
  - REFACTOR: none

  **Must NOT do**:
  - Add new tasks (just gate existing)
  - Change task bodies
  - Add Windows-equivalent tasks for coverage (tarpaulin alternatives not in scope)
  - Touch `dev`, `test`, `lint`, `setup` (already platform-agnostic)

  **Recommended Agent Profile**:
  - **Category**: `quick` — Pure config file edit with mechanical verification.
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES with 2, 3, 4, 5, 7
  - **Parallel Group**: Wave 1
  - **Blocks**: 7
  - **Blocked By**: 0

  **References**:
  - `Makefile.toml:58-60` — `build-all` ungated
  - `Makefile.toml:64-69` — `build-cross-all` ungated
  - `Makefile.toml:89-96` — `coverage-check`, `coverage-html` ungated
  - `Makefile.toml:26-34` — `build-x86-windows`, `build-x64-windows` already gated (pattern to mirror)
  - External: https://sagiegurari.github.io/cargo-make/#platform-override — `platforms` key syntax

  **Acceptance Criteria**:
  - [ ] `cargo make build-all` on Linux executes Linux build without attempting Windows tasks (unless host is Windows)
  - [ ] `cargo make coverage-check` on a non-Linux host prints "not supported on this platform" and exits 0
  - [ ] `cargo test --all-features` → 0 failures (unrelated, but regression gate)
  - [ ] All 4 identified tasks have `[tasks.X.linux]` override OR `condition = { platforms = ["linux"] }`

  **QA Scenarios**:

  ```
  Scenario: Happy path — Makefile.toml gating valid
    Tool: Bash
    Preconditions: Task 6 applied
    Steps:
      1. cargo make --list-all-steps 2>&1 | tee .sisyphus/evidence/task-6-list.log
      2. cargo make build-all --print-steps 2>&1 | tee .sisyphus/evidence/task-6-dry-run.log
    Expected Result: No "ungated" warnings; build-all resolves to Linux subtasks on Linux host
    Evidence: .sisyphus/evidence/task-6-list.log, .sisyphus/evidence/task-6-dry-run.log

  Scenario: Failure — coverage-check gracefully declines on non-Linux
    Tool: Bash
    Preconditions: Task 6 applied; simulate non-Linux via `CARGO_MAKE_PLATFORM_OVERRIDE=windows`
    Steps:
      1. CARGO_MAKE_PLATFORM_OVERRIDE=windows cargo make coverage-check 2>&1 | tee .sisyphus/evidence/task-6-skip.log
      2. grep -qE "skipped|not supported|platform" .sisyphus/evidence/task-6-skip.log
    Expected Result: grep finds "skipped" or equivalent; exit 0
    Evidence: .sisyphus/evidence/task-6-skip.log
  ```

  **Commit**: YES — `chore(cargo-make): gate Linux-only tasks`

- [x] 7. **GitHub Actions CI skeleton (3 jobs, Linux-only test matrix initially)**

  **What to do**:
  - Build out `.github/workflows/ci.yml` (started in Task 0) into full skeleton:
    - Job 1: `linux-tests` on `ubuntu-latest` — `cargo test --all-features`, `cargo clippy -- -D warnings`, `cargo fmt --check`
    - Job 2: `windows-build` on `windows-latest` — `cargo build --release` (no tests yet; Task 26 enables tests)
    - Job 3: `linux-cross-wine` on `ubuntu-latest` — placeholder that prints "pending Task 21/22" and succeeds; will be replaced in Task 22
  - Install LLVM 14 (static) on both Linux and Windows via consistent action
  - Cache cargo registry + target dir
  - RED: push to branch, observe CI fails if any job breaks
  - GREEN: iterate until all 3 jobs green
  - REFACTOR: extract reusable setup into composite action `.github/actions/setup-llvm/action.yml`

  **Must NOT do**:
  - Enable Windows tests yet (Task 26)
  - Enable cross-compile yet (Task 21)
  - Add macOS job (out of scope)
  - Add release/deploy jobs
  - Add job matrix beyond the 3 locked jobs (per Metis directive)
  - Add codecov/coverage upload (handled separately if at all)

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high` — CI config iteration with external integration (GitHub runners, LLVM actions).
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: Partially — depends on 5 (static LLVM) and 6 (gated makefile). Starts after 5 and 6 complete.
  - **Parallel Group**: Wave 1 tail (after 5, 6)
  - **Blocks**: 22, 26
  - **Blocked By**: 5, 6

  **References**:
  - `.github/workflows/ci.yml` (Task 0's spike version) — starting point
  - Metis directive: "CI matrix exactly: ubuntu-latest, windows-latest, ubuntu-latest+wine" — no more, no less
  - External: https://github.com/KyleMayes/install-llvm-action — LLVM setup action
  - External: https://docs.github.com/en/actions — workflow syntax
  - External: https://github.com/Swatinem/rust-cache — cargo caching

  **Acceptance Criteria**:
  - [ ] `.github/workflows/ci.yml` defines exactly 3 jobs: `linux-tests`, `windows-build`, `linux-cross-wine`
  - [ ] `gh run list --workflow=ci.yml --limit 1 --json conclusion --jq '.[0].conclusion'` → `"success"` on push to branch
  - [ ] Linux job runs: `cargo test --all-features`, `cargo clippy -- -D warnings`, `cargo fmt --all -- --check`
  - [ ] Windows job runs: `cargo build --release`
  - [ ] Cross-wine job: placeholder returning 0 with echo `"pending Task 21/22"`
  - [ ] Cargo cache hit rate logged (cache working)

  **QA Scenarios**:

  ```
  Scenario: Happy path — CI passes on clean branch
    Tool: Bash (gh CLI)
    Preconditions: .github/workflows/ci.yml committed to feature branch
    Steps:
      1. git push origin <branch>
      2. gh run watch --exit-status
      3. gh run view --json jobs --jq '.jobs[] | {name, conclusion}' | tee .sisyphus/evidence/task-7-jobs.json
    Expected Result: All 3 jobs conclusion="success"
    Evidence: .sisyphus/evidence/task-7-jobs.json

  Scenario: Failure — introduce deliberate clippy warning on disposable branch and observe rejection (no force-push)
    Tool: Bash (gh CLI)
    Preconditions: Task 7 applied; current branch is `<feature>`.
    Steps:
      1. Create a throwaway branch locally: `git checkout -b ci-negative-probe-$(date +%s)`
      2. Edit `src/main.rs` introducing `let _unused = 42;` (triggers clippy `unused_variables`).
      3. Commit NORMALLY (NO `--amend`): `git add src/main.rs && git commit -m "test(ci): deliberate clippy warning for CI negative probe"`
      4. Push NORMALLY (NO `-f`): `git push -u origin HEAD`
      5. `gh run watch --exit-status; echo "exit=$?" | tee .sisyphus/evidence/task-7-negative.log` — expect non-zero exit.
      6. `gh run view --json jobs --jq '.jobs[] | select(.name == "linux-tests") | .conclusion' | tee -a .sisyphus/evidence/task-7-negative.log` — expect `"failure"`.
      7. Clean up (no history rewrite): `git checkout <feature> && git branch -D ci-negative-probe-*; gh run list --branch ci-negative-probe-* --json databaseId --jq '.[].databaseId' | xargs -n1 gh run delete --confirm; git push origin --delete ci-negative-probe-* 2>/dev/null || true`
    Expected Result: CI run exits non-zero; `linux-tests` job conclusion is `"failure"`; throwaway branch deleted; NO commits rewritten, NO force-pushes, NO protected-branch modifications.
    Failure Indicators: CI passes (clippy config not enforcing warnings); force-push required (protocol violation — reject).
    Evidence: .sisyphus/evidence/task-7-negative.log
  ```

  **Commit**: YES — `ci: add GitHub Actions skeleton (3 jobs)`

- [x] 8. **`CodegenContext::for_triple` — replace host-hardcoded codegen**

  **What to do**:
  - Audit `src/codegen/*.rs` for references to `inkwell::targets::TargetMachine::get_default_triple()` or similar host-assumption calls
  - Add `impl CodegenContext { pub fn for_triple<'ctx>(context: &'ctx Context, module_name: &str, target: &TargetTriple) -> Self }`
  - Keep existing `new(&Context, &str)` as a thin wrapper calling `for_triple` with host target
  - Internally: use `Target::from_triple(&target.to_llvm_string())` to get `Target`, then `target.create_target_machine(...)` with appropriate CPU/features (`"x86-64"` CPU, `""` features for now)
  - For `x86_64-pc-windows-msvc`: RelocMode::PIC, CodeModel::Default
  - For `x86_64-unknown-linux-gnu`: RelocMode::PIC, CodeModel::Default (matches current)
  - RED: test `codegen_context_windows_msvc_triple` constructs context for Windows, asserts no panic and target data layout contains `"windows-msvc"` in the LLVM triple string
  - GREEN: implement
  - REFACTOR: consolidate CPU/features into `impl TargetTriple { fn llvm_cpu(&self) -> &'static str }`
  - Author unit test `for_triple_invalid_returns_err` in `src/codegen/context.rs` (or `src/codegen/mod.rs` `#[cfg(test)] mod tests`):
    - Call `CodegenContext::for_triple(&ctx, "probe", &parse_target_triple("aarch64-pc-windows-msvc").unwrap())` — aarch64-windows is explicitly OUT of scope per Task 0 and must return `Err` (not panic).
    - Assert `result.is_err()` via `assert!(result.is_err())`.
    - Assert error message contains the substring `"target"` or `"triple"` (case-insensitive) via `let msg = format!("{}", result.unwrap_err()); assert!(msg.to_lowercase().contains("target") || msg.to_lowercase().contains("triple"))`.
    - Rationale: aarch64-windows is a valid `TargetTriple` instance (parses successfully) but is unsupported by codegen in this effort — exactly the "valid but unsupported" error path we need to test without violating the closed-variant type.

  **Must NOT do**:
  - Change existing `CodegenContext::new` signature (wrap, don't replace)
  - Add features (AVX, SSE tuning) beyond `""` default
  - Handle aarch64 (out of scope)
  - Change `Context` lifetime scheme

  **Recommended Agent Profile**:
  - **Category**: `deep` — Non-trivial LLVM API interaction; risk of subtle layout/triple bugs that only surface at runtime.
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES with 9, 10, 11, 12, 13
  - **Parallel Group**: Wave 2
  - **Blocks**: 14
  - **Blocked By**: 1

  **References**:
  - `src/codegen/context.rs` or `src/codegen/mod.rs` — find `CodegenContext::new`; search for `TargetMachine`
  - External: https://thedan64.github.io/inkwell/inkwell/targets/index.html — `Target`, `TargetMachine`, `TargetData` API
  - External: https://llvm.org/docs/LangRef.html#data-layout — understand why triple matters (pointer size, endianness)
  - `src/build_system/targets.rs:28-34` — `TargetTriple` struct (post-Task-0.5: `arch` + `platform` + `env: Option<TripleEnv>`); add `to_llvm_string()` method on that struct
  - Metis directive: "Use `&TargetTriple` or `TargetSpec`, NOT `&str`"

  **Acceptance Criteria**:
  - [ ] `CodegenContext::for_triple` exists; `new` delegates to it with host
  - [ ] Unit test: constructing context for `x86_64-pc-windows-msvc` yields `TargetData` with LLVM triple containing `"windows-msvc"` and data layout string containing `"m:w"` (Windows mangling)
  - [ ] Unit test: constructing context for `x86_64-unknown-linux-gnu` yields triple containing `"linux-gnu"`
  - [ ] `cargo test --all-features` → 0 failures (regression)
  - [ ] Clippy clean

  **QA Scenarios**:

  ```
  Scenario: Happy path — Windows MSVC triple produces PE-compatible data layout
    Tool: Bash
    Preconditions: Task 8 applied
    Steps:
      1. cargo test --all-features codegen::context::tests::for_triple_windows_msvc 2>&1 | tee .sisyphus/evidence/task-8-unit.log
      2. grep -E "test .*for_triple_windows_msvc.*ok" .sisyphus/evidence/task-8-unit.log
    Expected Result: grep finds "ok"
    Evidence: .sisyphus/evidence/task-8-unit.log

  Scenario: Failure — invalid triple propagates error, not panic
    Tool: Bash
    Preconditions: Task 8 applied (helpers + `for_triple_invalid_returns_err` test authored per "What to do"); Task 0 applied (aarch64-windows rejected as out-of-scope at codegen layer).
    Steps:
      1. Verify test exists in source: `grep -rn "fn for_triple_invalid_returns_err" src/codegen/ 2>&1 | tee .sisyphus/evidence/task-8-invalid-source.log` — assert exit 0 (test authored).
      2. Run the test: `cargo test --all-features for_triple_invalid_returns_err 2>&1 | tee .sisyphus/evidence/task-8-invalid.log`
      3. Assert pass: `grep -E "test .*for_triple_invalid_returns_err.*ok" .sisyphus/evidence/task-8-invalid.log`
      4. Assert test asserted `is_err()` (no panic): `grep -E "(panicked|thread .* panicked)" .sisyphus/evidence/task-8-invalid.log && exit 1 || true` — step fails if ANY panic string appears in output.
      5. Assert full suite still passes: `cargo test --all-features 2>&1 | tee .sisyphus/evidence/task-8-regression.log` then `grep -E "test result: ok\. .* 0 failed" .sisyphus/evidence/task-8-regression.log`.
    Expected Result: Test file contains `for_triple_invalid_returns_err` fn (step 1); cargo test reports `ok` for that test (step 3); no panic strings in test output (step 4); full test suite reports 0 failed (step 5).
    Failure Indicators: Test fn missing from source; test fails; output contains "panicked" (implementation panicked instead of returning Err — type-system contract violated); regression in broader suite.
    Evidence: .sisyphus/evidence/task-8-invalid-source.log, .sisyphus/evidence/task-8-invalid.log, .sisyphus/evidence/task-8-regression.log
  ```

  **Commit**: YES (RGR: 3 commits) — `refactor(codegen): CodegenContext::for_triple`

- [x] 9. **Target-driven `emit_object_file`**

  **What to do**:
  - Locate current `emit_object_file(module, path)` (likely `src/codegen/emit.rs` or `src/compiler.rs`)
  - Change signature: `pub fn emit_object_file(module: &Module, path: &Path, target: &TargetTriple) -> Result<(), CompileError>`
  - Use `target.create_target_machine(...)` to get a `TargetMachine` matching the target (not host)
  - Use `object_file_extension(target)` from Task 2 when constructing the output path
  - Use `TargetMachine::write_to_file(module, FileType::Object, path)` with the target-matched machine
  - RED: test asserts that emitting for `x86_64-pc-windows-msvc` produces a file with header bytes starting with `'M' 'Z'` → `0x5A4D` (COFF/PE marker on Windows object files) OR `0x64 0x86` (COFF x86_64 machine type) — actual bytes vary; choose stable indicator
  - GREEN: implement
  - REFACTOR: extract file-opening logic (handle Windows temp-file lock per Metis risk #8)

  **Must NOT do**:
  - Skip the target parameter (defaulting to host defeats the task)
  - Change `FileType` (always `Object` for now)
  - Add assembly output
  - Stream to stdout

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high` — LLVM API + filesystem handling + target-driven logic; moderate complexity.
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES with 8, 10, 11, 12, 13
  - **Parallel Group**: Wave 2
  - **Blocks**: 14
  - **Blocked By**: 1, 2

  **References**:
  - `src/compiler.rs:618` — `format!("module_{index}.o")` currently hardcoded `.o`
  - `src/compiler.rs:419` — `output_dir.join("program.o")` hardcoded
  - `src/build_system/targets.rs` — `object_file_extension` (Task 2)
  - External: https://thedan64.github.io/inkwell/inkwell/targets/struct.TargetMachine.html#method.write_to_file
  - Metis risk #8: "Temp file locking on Windows: must close file handle before linker opens" — apply in refactor step

  **Acceptance Criteria**:
  - [ ] `emit_object_file` accepts `&TargetTriple` param
  - [ ] Linux target produces `.o` with ELF magic bytes `0x7F 'E' 'L' 'F'`
  - [ ] Windows MSVC target produces `.obj` with COFF machine type `0x8664` (x86_64) at offset 0
  - [ ] `cargo test --all-features` → 0 failures (regression)
  - [ ] Unit test covers both happy paths

  **QA Scenarios**:

  ```
  Scenario: Happy path — Linux .o has ELF magic
    Tool: Bash
    Preconditions: Task 9 applied
    Steps:
      1. cargo test --all-features --features integration codegen::emit::tests::emit_linux_object 2>&1 | tee .sisyphus/evidence/task-9-linux.log
      2. grep "ok" .sisyphus/evidence/task-9-linux.log
    Expected Result: test passes; test internally reads file and asserts ELF magic
    Evidence: .sisyphus/evidence/task-9-linux.log

  Scenario: Happy path — Windows MSVC .obj has COFF x86_64 machine type
    Tool: Bash
    Preconditions: Task 9 applied; LLVM 14 installed (for cross-target support)
    Steps:
      1. cargo test --all-features --features integration codegen::emit::tests::emit_windows_msvc_object 2>&1 | tee .sisyphus/evidence/task-9-windows.log
      2. grep "ok" .sisyphus/evidence/task-9-windows.log
    Expected Result: test passes; file has first 2 bytes 0x64 0x86 (COFF x86_64 machine) or equivalent verified header
    Evidence: .sisyphus/evidence/task-9-windows.log
  ```

  **Commit**: YES (RGR: 3 commits) — `feat(codegen): target-driven emit_object_file`

- [x] 10. **`LinkerCommand` abstraction — replace inline `Command::new`**

  **What to do**:
  - In `src/build_system/linker.rs` (from Task 3), add:
    - `pub struct LinkerCommand { linker: Linker, target: TargetTriple, inputs: Vec<PathBuf>, output: PathBuf, extra_args: Vec<String> }`
    - `impl LinkerCommand { pub fn new(target: &TargetTriple, output: PathBuf) -> Self; pub fn with_input(&mut self, p: PathBuf) -> &mut Self; pub fn build(self) -> std::process::Command; }`
  - `build()` dispatches on `self.linker` (set via `detect_preferred_linker(target)`):
    - `Msvc` → builds `link.exe` command (Task 11 fills args)
    - `MinGw` → builds `x86_64-w64-mingw32-gcc` command (Task 12 fills args)
    - `Clang` → builds `clang` command
    - `Cc` → builds `cc` command (current Linux behavior)
  - Replace `src/compiler.rs:307-342` inline Command::new block with `LinkerCommand::new(...).with_input(...).build()`
  - Quote all paths containing spaces (Metis risk #9)
  - RED: integration test builds a trivial C object and links it via `LinkerCommand` for host target; asserts exit 0 and output file exists
  - GREEN: implement
  - REFACTOR: extract path-quoting into private helper

  **Must NOT do**:
  - Add a `Linker` trait (concrete dispatch only)
  - Support `rustc`-style linker-flavor detection (out of scope)
  - Handle response files yet (defer until Task 11 if arg length exceeds limits)
  - Add `lld` variant (not in the 4 Linker variants)

  **Recommended Agent Profile**:
  - **Category**: `deep` — Refactor of critical linking step with cross-target correctness concerns.
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES with 8, 9, 13
  - **Parallel Group**: Wave 2
  - **Blocks**: 11, 12, 14
  - **Blocked By**: 1, 2, 3

  **References**:
  - `src/compiler.rs:307-342` — current inline Command::new block (replace)
  - `src/build_system/linker.rs` (Task 3) — `Linker` enum and `detect_preferred_linker`
  - `src/build_system/targets.rs` — `executable_filename` (Task 2) for output path
  - Metis risk #9: "Spaces in paths: must fix" — apply in REFACTOR
  - Metis risk #8: "Temp-file locking" — ensure inputs paths are closed/flushed before command runs

  **Acceptance Criteria**:
  - [ ] `src/compiler.rs:307-342` region contains zero `Command::new` calls for linking; all go through `LinkerCommand`
  - [ ] `LinkerCommand` unit tests cover all 4 Linker variants
  - [ ] Integration test: link a hello-world on Linux via `LinkerCommand` and run it; stdout matches expected
  - [ ] `cargo test --all-features` → 0 failures
  - [ ] Paths with spaces are double-quoted in emitted command (verified via test that constructs a path with spaces and inspects the `Command::get_args`)

  **QA Scenarios**:

  ```
  Scenario: Happy path — Linux link via LinkerCommand produces runnable binary
    Tool: Bash
    Preconditions: Task 10 applied
    Steps:
      1. cargo test --all-features --features integration build_system::linker::tests::link_hello_linux 2>&1 | tee .sisyphus/evidence/task-10-linux.log
      2. grep "ok" .sisyphus/evidence/task-10-linux.log
    Expected Result: test passes; internally the test compiles a C object, links via LinkerCommand, runs the binary, asserts stdout "Hello"
    Evidence: .sisyphus/evidence/task-10-linux.log

  Scenario: Failure — path with spaces does not break linker invocation
    Tool: Bash
    Preconditions: Task 10 applied
    Steps:
      1. cargo test --all-features build_system::linker::tests::path_with_spaces 2>&1 | tee .sisyphus/evidence/task-10-spaces.log
    Expected Result: test asserts generated Command args contain properly quoted paths; test passes
    Evidence: .sisyphus/evidence/task-10-spaces.log
  ```

  **Commit**: YES (RGR: 3 commits) — `refactor(linker): LinkerCommand abstraction`

- [x] 11. **MSVC-target linker path — native `link.exe` (Windows host) and `lld-link` (Linux host with xwin sysroot)**

  **What to do**:
  - **IMPORTANT TOOLCHAIN FACT** (ground-truth-verified): `xwin` downloads and splats MSVC **headers and import libs ONLY** — it does NOT provide `cl.exe` or `link.exe` binaries (those are Windows-native PE executables). Therefore the linker backend MUST differ by host:
    - **Windows host → MSVC target**: use real `link.exe` from Visual Studio Build Tools, discovered via `cc::windows_registry::find_tool(target, "link.exe")` (this API is compiled and callable only on Windows hosts; on Linux hosts the call is not available).
    - **Linux host → MSVC target (cross-compile)**: use `lld-link` from LLVM (it is drop-in argv-compatible with `link.exe`) together with xwin's extracted MSVC CRT/SDK as a `/winsysroot` (via clang-cl-style `-libpath:` args pointing into the xwin splat directory). Do NOT attempt to run `link.exe` on Linux. Do NOT use `xwin-run` (no such tool exists in the xwin project).
  - In `src/build_system/linker.rs`, fill in `Linker::Msvc` branch of `LinkerCommand::build()` with host-dispatched behavior (gate with `#[cfg(target_os = "windows")]` for the native discovery, provide a Linux-host fallback path that selects `lld-link` + xwin sysroot args):
    - Native Windows host: discover `link.exe` — prefer env var `OPAL_MSVC_LINKER` if set; else `cc::windows_registry::find_tool(target, "link.exe")` (Windows-only API — gated with `#[cfg(windows)]`).
    - Linux host (cross to MSVC target): discover `lld-link` — prefer env var `OPAL_MSVC_LINKER`; else look up `lld-link` or `lld-link-14` / `lld-link-15` in `$PATH`. Additionally require env `XWIN_CACHE` (or `OPAL_XWIN_SYSROOT`) pointing to the xwin splat directory; add `/libpath:$XWIN_CACHE/crt/lib/x86_64` and `/libpath:$XWIN_CACHE/sdk/lib/um/x86_64` and `/libpath:$XWIN_CACHE/sdk/lib/ucrt/x86_64`.
    - Shared args (both hosts): `/OUT:<output>`, `/SUBSYSTEM:CONSOLE`, `/DEFAULTLIB:libcmt` (static CRT), `/MACHINE:X64`, `<inputs>`.
    - DO NOT pass `-no-pie` (Linux-only, Task 13 handles).
    - DO NOT pass `/LTCG` or `/DEBUG` by default (not in scope).
    - When cross-compiling from Linux, if `lld-link` is missing OR `XWIN_CACHE` is unset, fail with a clear error naming both requirements.
  - RED: two unit tests —
    - `msvc_linker_on_windows_host_uses_link_exe` (gated `#[cfg(windows)]`): asserts program ends in `link.exe`.
    - `msvc_linker_on_linux_host_uses_lld_link` (gated `#[cfg(not(windows))]`): with `XWIN_CACHE=/tmp/fake-xwin` set, asserts program is `lld-link` and args contain three `/libpath:` entries rooted at `/tmp/fake-xwin`.
  - GREEN: implement.
  - REFACTOR: extract `fn msvc_args(output, inputs) -> Vec<String>` (shared) and `fn msvc_sysroot_args(xwin_cache: &Path) -> Vec<String>` (Linux-host only).

  **Must NOT do**:
  - Do NOT claim `xwin-run cl.exe` exists — no such tool; remove all references.
  - Do NOT call `cc::windows_registry::find_tool` on Linux hosts — that API is Windows-only and will not compile there; the Linux-host branch uses `lld-link`.
  - Do NOT invoke `cl.exe` on Linux — it's a PE binary and won't run.
  - Do NOT use MSVC C compiler (`cl.exe`) to link even on Windows — call `link.exe` (or `lld-link`) directly.
  - Do NOT add `/DYNAMICBASE`, `/NXCOMPAT` (defaults are fine).
  - Do NOT handle DEF files or resources.
  - Do NOT support MSVC <2019 specifics.
  - Do NOT ship `link.exe` or `lld-link` ourselves.

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high` — Platform-specific but mechanical; requires knowing link.exe/lld-link argv and xwin splat layout.
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES with 12 (different Linker variant)
  - **Parallel Group**: Wave 2
  - **Blocks**: 14, 21
  - **Blocked By**: 3, 10

  **References**:
  - External: https://learn.microsoft.com/en-us/cpp/build/reference/linker-options — `link.exe` (and argv-compatible `lld-link`) reference.
  - External: https://lld.llvm.org/windows_support.html — `lld-link` argv compatibility with MSVC `link.exe`.
  - External: https://docs.rs/cc/latest/cc/windows_registry/fn.find_tool.html — `cc::windows_registry::find_tool` (Windows-host-only API).
  - External: https://github.com/Jake-Shadle/xwin — xwin extracts MSVC CRT/SDK headers + libs (headers+libs ONLY, NOT compilers/linkers); splat layout under `<cache>/crt/`, `<cache>/sdk/lib/um`, `<cache>/sdk/lib/ucrt`.
  - `Cargo.toml` — `cc` crate (Windows-host-only usage for this task's discovery code).

  **Acceptance Criteria**:
  - [ ] On Windows host: `LinkerCommand` for `x86_64-pc-windows-msvc` returns a Command whose program resolves to `link.exe` (or full path ending in it); discovery uses `cc::windows_registry::find_tool` gated `#[cfg(windows)]`.
  - [ ] On Linux host: `LinkerCommand` for `x86_64-pc-windows-msvc` returns a Command whose program is `lld-link` (or `lld-link-14`/`-15`); args contain three `/libpath:` entries rooted at `$XWIN_CACHE`.
  - [ ] Shared args include: `/OUT:`, `/SUBSYSTEM:CONSOLE`, `/MACHINE:X64`, `/DEFAULTLIB:libcmt`.
  - [ ] Env var `OPAL_MSVC_LINKER` override works on both hosts (test sets env, asserts program equals override).
  - [ ] Linux-host missing `XWIN_CACHE` yields error message naming both `lld-link` and `XWIN_CACHE` as requirements.
  - [ ] `cargo test --all-features` → 0 failures on Linux host and Windows host.
  - [ ] On Windows runner: integration test links a hello-world .obj into a runnable .exe via `link.exe`.
  - [ ] On Linux runner (Task 21 CI): integration test links a hello-world .obj into a runnable .exe via `lld-link` + xwin, then runs it via wine.

  **QA Scenarios**:

  ```
  Scenario: Happy path — Windows native link.exe invocation produces runnable .exe
    Tool: Bash (on windows-latest CI runner)
    Preconditions: Task 11 applied; MSVC Build Tools installed; windows-build CI job.
    Steps:
      1. cargo test --all-features --features integration build_system::linker::tests::link_hello_windows_msvc 2>&1 | tee .sisyphus/evidence/task-11-windows.log
      2. grep "ok" .sisyphus/evidence/task-11-windows.log
    Expected Result: test passes; test produces hello.exe and runs it; stdout contains "Hello".
    Evidence: .sisyphus/evidence/task-11-windows.log (captured from CI run).

  Scenario: Happy path — Linux cross via lld-link + xwin produces wine-runnable .exe
    Tool: Bash (on ubuntu-latest CI runner)
    Preconditions: Task 21 CI job installs lld (`apt-get install -y lld`), runs `xwin splat` into $XWIN_CACHE, installs wine; Task 11 applied.
    Steps:
      1. which lld-link | tee .sisyphus/evidence/task-11-linux-cross-lld.log
      2. ls $XWIN_CACHE/crt/lib/x86_64 $XWIN_CACHE/sdk/lib/um/x86_64 $XWIN_CACHE/sdk/lib/ucrt/x86_64 | tee -a .sisyphus/evidence/task-11-linux-cross-lld.log
      3. cargo test --all-features --features integration build_system::linker::tests::link_hello_windows_msvc_cross 2>&1 | tee -a .sisyphus/evidence/task-11-linux-cross-lld.log
      4. wine target/test-hello.exe | grep "Hello"
    Expected Result: lld-link found; xwin splat directories exist and are non-empty; cross-link test passes; wine-run of produced .exe prints "Hello".
    Evidence: .sisyphus/evidence/task-11-linux-cross-lld.log.

  Scenario: Failure — missing link.exe on Windows produces clear error
    Tool: Bash
    Preconditions: Task 11 applied; env scrubbed of MSVC paths on Windows host.
    Steps:
      1. PATH=/tmp/empty cargo test --all-features build_system::linker::tests::msvc_linker_missing 2>&1 | tee .sisyphus/evidence/task-11-missing.log
    Expected Result: error message mentions "link.exe not found" or "MSVC linker" (not a cryptic OS error).
    Evidence: .sisyphus/evidence/task-11-missing.log.

  Scenario: Failure — missing lld-link / XWIN_CACHE on Linux produces clear error
    Tool: Bash
    Preconditions: Task 11 applied; Linux host; `lld-link` absent or `XWIN_CACHE` unset.
    Steps:
      1. unset XWIN_CACHE; cargo test --all-features build_system::linker::tests::linux_msvc_cross_missing 2>&1 | tee .sisyphus/evidence/task-11-linux-missing.log
    Expected Result: error message names BOTH `lld-link` (if missing) AND `XWIN_CACHE` (if unset); actionable install hint.
    Evidence: .sisyphus/evidence/task-11-linux-missing.log.
  ```

  **Commit**: YES (RGR: 3 commits) — `feat(linker): MSVC linker path — link.exe on Windows, lld-link on Linux`

- [x] 12. **MinGW-w64 linker path (`x86_64-w64-mingw32-gcc` args)**

  **What to do**:
  - Extend `LinkerCommand` (from Task 10) with a `Mingw` variant that uses `x86_64-w64-mingw32-gcc` as the driver.
  - Args: `-o <output.exe> <object.obj> -lbcrypt -luserenv -lws2_32 -ladvapi32 -lntdll` (Windows CRT libs linked via gcc driver; gcc handles CRT automatically).
  - Detect MinGW via `which("x86_64-w64-mingw32-gcc")` (or `where` on Windows); if `detect_preferred_linker` returned `Mingw`, use this path.
  - On Linux host + Windows target: MinGW is a valid cross-linker and requires NO sysroot tarball (unlike MSVC+xwin).
  - RGR: RED = integration test `mingw_target_produces_runnable_exe` cross-compiles on Linux with `--target x86_64-pc-windows-gnu`, runs under wine, expects "hello" output → FAIL (no MinGW variant). GREEN = implement `LinkerCommand::Mingw`. REFACTOR = share flag-building helpers with MSVC variant where sensible.

  **Must NOT do**:
  - Do NOT use `zig cc` (banned per user decision).
  - Do NOT statically link `libgcc`/`libstdc++` unless explicitly requested — default dynamic is fine for v1.
  - Do NOT attempt to probe MSVC env vars when MinGW is selected.

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
    - Reason: Straightforward linker invocation logic with integration test, similar complexity to Task 11.
  - **Skills**: none
    - Reason: No cross-cutting skills needed; scope is a single enum variant + Command builder.

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2 (with Tasks 8, 9, 10, 11, 13, 14)
  - **Blocks**: Task 21 (xwin CI), Task 22 (wine CI uses MinGW path for gnu triple)
  - **Blocked By**: Task 10 (LinkerCommand abstraction), Task 3 (linker detection enum)

  **References**:
  - Task 10 introduces `LinkerCommand` in `src/build_system/linker.rs` (NOT `src/compiler/linker.rs` — that file does not exist; the current inline linker logic lives in `src/compiler.rs:289-342` via `build_linker_command`). Task 12 extends Task 10's `LinkerCommand` enum — add `Mingw` variant alongside `Gcc`/`Msvc`.
  - Task 11 `LinkerCommand::Msvc` — mirror the structure (args builder, detect fn, integration test).
  - MinGW-w64 gcc driver docs: `https://www.mingw-w64.org/` — `-o`, `-l<lib>` standard GCC flags.
  - Windows CRT libs needed for Opalescent runtime: `bcrypt` (Task 17 BCryptGenRandom), `userenv`, `ws2_32`, `advapi32`, `ntdll` — required for default runtime link.
  - `test-projects/hello-world/src/main.op` — use as integration test fixture.

  **Acceptance Criteria**:
  - [ ] `src/build_system/linker.rs` has `LinkerCommand::Mingw { driver: PathBuf, args: Vec<String> }` (file created by Task 10; Task 12 extends).
  - [ ] `detect_mingw()` returns `Some(PathBuf)` when `x86_64-w64-mingw32-gcc` is on PATH.
  - [ ] Integration test `mingw_target_produces_runnable_exe` (gated `#[cfg(all(unix, feature = "wine-tests"))]`) compiles `hello-world` with `--target x86_64-pc-windows-gnu` and runs the `.exe` via `wine` — stdout contains `hello`.
  - [ ] `cargo test --features wine-tests mingw_target_produces_runnable_exe` → PASS on Linux with MinGW + wine installed.
  - [ ] Linux regression: `cargo test --features integration` (gnu-linux target, not cross) still PASS.

  **QA Scenarios**:
  ```
  Scenario: Linux cross-compile with MinGW produces runnable Windows .exe
    Tool: Bash (cargo + wine)
    Preconditions: Linux host with `x86_64-w64-mingw32-gcc` and `wine` installed; LLVM 14 configured.
    Steps:
      1. cargo build --release
      2. ./target/release/opalescent test-projects/hello-world/src/main.op --target x86_64-pc-windows-gnu
      3. file test-projects/hello-world/target/program.exe  # expect "PE32+ executable (console) x86-64, for MS Windows"
      4. wine test-projects/hello-world/target/program.exe  # expect stdout "hello"
    Expected Result: Exit 0; stdout contains "hello"; program.exe is a valid PE32+ binary.
    Failure Indicators: "linker not found", undefined references to Win32 functions, non-PE output.
    Evidence: .sisyphus/evidence/task-12-mingw-cross.log (cargo log + file + wine output)

  Scenario: Missing MinGW on PATH fails with actionable error
    Tool: Bash
    Preconditions: PATH scrubbed of `x86_64-w64-mingw32-gcc`.
    Steps:
      1. PATH=/usr/bin:/bin cargo run -- test-projects/hello-world/src/main.op --target x86_64-pc-windows-gnu
    Expected Result: Non-zero exit; stderr contains "x86_64-w64-mingw32-gcc not found" and install hint.
    Evidence: .sisyphus/evidence/task-12-mingw-missing.log
  ```

  **Commit**: YES (RGR: 3 commits) — `feat(linker): MinGW-w64 gcc path`

- [x] 13. **Host-gate `-no-pie` to Linux targets only**

  **What to do**:
  - In the linker args builder (Task 10 `LinkerCommand::Gcc` + Task 12 `Mingw`), conditionally add `-no-pie` **only** when `target.os == "linux"`.
  - Remove any unconditional `-no-pie` from previous inline `Command::new` paths (Task 10 should have already extracted this; verify nothing was left behind).
  - macOS / Windows targets MUST NOT receive `-no-pie` (flag is Linux-specific; `link.exe` rejects it outright, `ld64` warns).
  - RGR: RED = unit test `no_pie_flag_only_on_linux` builds `LinkerCommand` for each of `x86_64-unknown-linux-gnu`, `x86_64-pc-windows-msvc`, `x86_64-pc-windows-gnu`, `x86_64-apple-darwin`; asserts `-no-pie` present ONLY in the first. FAIL initially (unconditional flag). GREEN = add host gate. REFACTOR = extract `needs_no_pie(&TargetTriple) -> bool` helper.

  **Must NOT do**:
  - Do NOT remove `-no-pie` from Linux target — it is load-bearing (prevents `R_X86_64_32S` relocation failure per README escape hatch section).
  - Do NOT gate on **host** OS; gate on **target** OS. User on Linux cross-compiling to Windows must not get `-no-pie`.
  - Do NOT introduce a new escape-hatch flag; this is a bug fix, not a feature.

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Tiny, localized change with strong unit-test coverage. One conditional.
  - **Skills**: none.

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2 (with Tasks 8, 9, 10, 11, 12, 14)
  - **Blocks**: None (downstream waves consume `LinkerCommand` as a unit).
  - **Blocked By**: Task 10 (LinkerCommand abstraction).

  **References**:
  - Current hardcoded site: `src/compiler.rs:307-342` — search for `-no-pie` token; this is the removal target.
  - README.md "Escape Hatches" section documents WHY Linux needs `-no-pie`; preserve that rationale in code comment.
  - Task 1 `TargetTriple::os()` accessor — use this for the gate.

  **Acceptance Criteria**:
  - [ ] Unit test `no_pie_flag_only_on_linux` PASSES: 4 target triples verified, only linux-gnu gets `-no-pie`.
  - [ ] `grep -n "no-pie" src/` returns ONLY lines inside `LinkerCommand` gated by `target.os() == "linux"`.
  - [ ] Linux regression: `cargo test --features integration` all PASS (hello-world, fib-recursive, fib-iterative, simple-quiz).
  - [ ] Windows MSVC target: no `-no-pie` in `link.exe` argv (verify via Task 11 debug log).

  **QA Scenarios**:
  ```
  Scenario: Linux target still links with -no-pie
    Tool: Bash
    Preconditions: Linux host, gcc installed.
    Steps:
      1. RUST_LOG=opalescent::compiler::linker=debug cargo run -- test-projects/hello-world/src/main.op 2>&1 | tee /tmp/linker.log
      2. grep -q -- '-no-pie' /tmp/linker.log
    Expected Result: grep exits 0 (flag present).
    Evidence: .sisyphus/evidence/task-13-linux-no-pie-present.log

  Scenario: Windows target has no -no-pie
    Tool: Bash (cross-compile)
    Preconditions: Linux host + MinGW + wine.
    Steps:
      1. RUST_LOG=opalescent::compiler::linker=debug cargo run -- test-projects/hello-world/src/main.op --target x86_64-pc-windows-gnu 2>&1 | tee /tmp/linker-win.log
      2. grep -c -- '-no-pie' /tmp/linker-win.log  # expect 0
    Expected Result: Count is 0.
    Evidence: .sisyphus/evidence/task-13-windows-no-no-pie.log
  ```

  **Commit**: YES (RGR: 3 commits) — `fix(linker): host-gate -no-pie to Linux`

- [ ] 14. **Thread `target: &TargetTriple` through `compile_program` / `compile_project`**

  **What to do**:
  - Update public API signatures:
    - `compile_program(source: &str, output_dir: &Path) -> Result<PathBuf, CompileError>` → `compile_program(source: &str, output_dir: &Path, target: &TargetTriple) -> Result<PathBuf, CompileError>`.
    - Same for `compile_project(project_root, target: &TargetTriple)`.
  - Add backward-compat shim: `compile_program_host(source, output_dir)` = `compile_program(source, output_dir, &TargetTriple::host())` to minimize breakage in existing call sites.
  - Pass `target` down to: `CodegenContext::for_triple` (Task 8), `emit_object_file` (Task 9), `LinkerCommand::for_target` (Task 10).
  - `src/app.rs` CLI layer: parse `--target` (Task 4), resolve to `TargetTriple`, pass through.
  - Remove all internal callers that grab host triple from `std::env::consts::OS` — target is now explicit and propagates.
  - RGR: RED = unit test `compile_program_respects_target_override` calls `compile_program(..., &TargetTriple::x86_64_pc_windows_msvc())` on Linux host, asserts output path ends in `.exe`. FAIL (target param doesn't exist). GREEN = add param + plumbing. REFACTOR = collapse any now-redundant host-detection calls.

  **Must NOT do**:
  - Do NOT leave any `TargetTriple::host()` calls deep inside the compiler pipeline — host resolution MUST happen at the CLI boundary (`src/app.rs`) and propagate as an explicit parameter.
  - Do NOT default `target` to `Option<TargetTriple>` with a `None = host` inside compiler code — explicitness is the whole point of this task.
  - Do NOT change the `target` binding to `&mut` or `Arc` — `&TargetTriple` is sufficient (cheap Copy or Clone).

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: Signature change ripples through many call sites; requires careful audit to ensure no latent host-triple leak remains.
  - **Skills**: none.

  **Parallelization**:
  - **Can Run In Parallel**: YES (semantically — it coordinates with Tasks 8/9/10, but the function signature change is independent)
  - **Parallel Group**: Wave 2 (with Tasks 8, 9, 10, 11, 12, 13)
  - **Blocks**: Task 15 (opal_portability.h include path uses target), Task 21 (CI xwin uses `--target`), Task 22 (Wine CI uses `--target`), Task 23/24 (hot reload needs target-aware lib extension), Task 26 (Windows native tests).
  - **Blocked By**: Task 1 (TargetTriple type), Task 4 (`--target` CLI flag), Task 8 (CodegenContext::for_triple), Task 9 (emit_object_file target-driven), Task 10 (LinkerCommand abstraction).

  **References**:
  - `src/compiler.rs:compile_program` — public entry; change here first.
  - `src/compiler.rs:compile_project` — parallel entry for `opal build`.
  - `src/app.rs` — CLI layer; this is where host-default lives (single source of truth).
  - `src/hot_reload/version.rs:42-52` `shared_library_extension` — consumer of target (Task 23/24 will use this).
  - Rust stdlib `std::env::consts::{OS, ARCH}` — reference for what the host shim computes.

  **Acceptance Criteria**:
  - [ ] `compile_program` and `compile_project` signatures include `target: &TargetTriple` parameter.
  - [ ] `grep -rn "TargetTriple::host()" src/` returns results ONLY in `src/app.rs` and test helpers — zero results inside `src/compiler/`, `src/codegen/`, `src/hot_reload/`.
  - [ ] Unit test `compile_program_respects_target_override` PASSES.
  - [ ] All existing tests pass after signature migration: `cargo test --all` PASS.
  - [ ] Backward-compat shim `compile_program_host` exists and is documented as deprecated in favor of explicit target.

  **QA Scenarios**:
  ```
  Scenario: Explicit target override produces .exe on Linux host
    Tool: Bash
    Preconditions: Linux host, MinGW installed.
    Steps:
      1. cargo test --features integration compile_program_respects_target_override -- --nocapture
    Expected Result: Test passes; output path ends in `.exe`; file type is PE32+.
    Evidence: .sisyphus/evidence/task-14-target-override.log

  Scenario: No host-triple leaks remain in compiler pipeline
    Tool: Bash (grep audit)
    Preconditions: Working tree clean after Task 14 implementation.
    Steps:
      1. grep -rn "TargetTriple::host()\|std::env::consts::OS\|std::env::consts::ARCH" src/compiler src/codegen src/hot_reload 2>&1 | tee /tmp/audit.log
      2. wc -l /tmp/audit.log
    Expected Result: Line count is 0. (All host detection lives in `src/app.rs` only.)
    Evidence: .sisyphus/evidence/task-14-host-leak-audit.log
  ```

  **Commit**: YES (RGR: 3 commits) — `feat(compiler): thread target through compile_program/project`

- [ ] 15. **`runtime/opal_portability.h` — C portability header with MSVC shims**

  **What to do**:
  - Create `runtime/opal_portability.h` as the **single source of truth** for cross-platform C shims.
    - Platform detection (raw macros allowed ONLY inside this header): `#if defined(_WIN32)`, `#if defined(_MSC_VER)`, `#if defined(__MINGW32__)`.
    - Define project-level portability macros that runtime `.c` files MUST use instead of raw platform macros:
      ```c
      #if defined(_WIN32)
        #define OPAL_WINDOWS 1
      #else
        #define OPAL_WINDOWS 0
      #endif
      #if defined(_MSC_VER)
        #define OPAL_MSVC 1
      #else
        #define OPAL_MSVC 0
      #endif
      #if defined(__MINGW32__) || defined(__MINGW64__)
        #define OPAL_MINGW 1
      #else
        #define OPAL_MINGW 0
      #endif
      ```
  - Do NOT introduce per-file raw `#ifdef _WIN32` / `#if defined(_WIN32)` forests — runtime `.c` files MUST use the project-level `OPAL_WINDOWS` / `OPAL_MSVC` / `OPAL_MINGW` macros defined above. Raw platform macros are permitted ONLY inside `opal_portability.h` itself.
  - Do NOT depend on `<pthread.h>`, `<unistd.h>`, `<sys/types.h>` — these are not portable to MSVC.
  - Do NOT include C++-only features (`<type_traits>`, namespaces); this is a C89/C11-compat header.
  - Do NOT use `zig cc` or any other banned toolchain for verification.

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: C portability is subtle; getting the macros right across MSVC/MinGW/gcc/clang requires careful cross-compilation testing.
  - **Skills**: none.

  **Parallelization**:
  - **Can Run In Parallel**: NO (blocks most Wave 3 runtime tasks)
  - **Parallel Group**: Wave 3 start — sequential gate for Tasks 16-20.
  - **Blocks**: Tasks 16, 17, 18, 19, 20 (all runtime portability work depends on this header).
  - **Blocked By**: Task 14 (target threading — header needs to know target triple at compile time via CMake/build-script, though most macros are compile-time platform detection).

  **References**:
  - Research finding: `runtime/opal_parse.c:35` uses raw `thread_local` (C11) — breaks MSVC pre-VS2019.
  - Research finding: `runtime/opal_string.c:88` uses `ssize_t` — not defined in MSVC.
  - MSVC `__declspec(thread)`: `https://learn.microsoft.com/en-us/cpp/cpp/thread`.
  - C11 `_Thread_local`: https://en.cppreference.com/w/c/thread/thread_local.
  - Clang/GCC `__thread`: works on all Unix, avoid MSVC.
  - Existing pattern to match: no current portability header — this is greenfield; model after `https://github.com/libuv/libuv/blob/master/include/uv.h` for structure (but C-only, smaller scope).

  **Acceptance Criteria**:
  - [ ] `runtime/opal_portability.h` exists; header-guard `OPAL_PORTABILITY_H`.
  - [ ] Header compiles standalone with `gcc -std=c11 -Wall -Wextra -c -x c /dev/null -include runtime/opal_portability.h` → 0 warnings, 0 errors.
  - [ ] Header compiles with MinGW: `x86_64-w64-mingw32-gcc -std=c11 -Wall -Wextra -c -x c /dev/null -include runtime/opal_portability.h` → 0 warnings, 0 errors.
  - [ ] Header compiles with MSVC-target cl-driver (via `clang-cl` using xwin splat as sysroot on Linux, OR native `cl.exe` on Windows): `clang-cl /std:c11 /W3 /c /TC /FI opal_portability.h /imsvc $XWIN_CACHE/crt/include /imsvc $XWIN_CACHE/sdk/include/ucrt /imsvc $XWIN_CACHE/sdk/include/um /imsvc $XWIN_CACHE/sdk/include/shared empty.c /Fo:empty-msvc.obj` → 0 warnings (on Linux); or `cl.exe /std:c11 /W3 /c /TC /FI opal_portability.h empty.c /Fo:empty-msvc.obj` → 0 warnings (on Windows). `xwin-run` does NOT exist — do not use.
  - [ ] All seven runtime `.c`/`.h` files listed above include this header **first** (grep: `head -20 runtime/opal_*.c | grep -c opal_portability.h` ≥ 7).
  - [ ] No raw `#ifdef _WIN32` / `#if defined(_WIN32)` remains OUTSIDE `runtime/opal_portability.h` (audit: `grep -rn '#ifdef _WIN32\|#if defined(_WIN32)' runtime/ | grep -v 'opal_portability.h' | wc -l` → 0). Runtime `.c` files must use `OPAL_WINDOWS` / `OPAL_MSVC` / `OPAL_MINGW` instead.

  **QA Scenarios**:
  ```
  Scenario: Portability header compiles clean on all three toolchains
    Tool: Bash (multi-compiler matrix)
    Preconditions: Linux host with gcc, x86_64-w64-mingw32-gcc, clang-cl (from LLVM), and xwin splat directory populated at $XWIN_CACHE. Note: `xwin-run` does NOT exist; MSVC-target compilation on Linux uses `clang-cl` + xwin sysroot.
    Steps:
      1. echo 'int main(void){return 0;}' > /tmp/empty.c
      2. gcc -std=c11 -Wall -Wextra -Werror -include runtime/opal_portability.h -c /tmp/empty.c -o /tmp/empty-gcc.o
      3. x86_64-w64-mingw32-gcc -std=c11 -Wall -Wextra -Werror -include runtime/opal_portability.h -c /tmp/empty.c -o /tmp/empty-mingw.o
      4. clang-cl /std:c11 /W3 /WX /TC /FI opal_portability.h /imsvc $XWIN_CACHE/crt/include /imsvc $XWIN_CACHE/sdk/include/ucrt /imsvc $XWIN_CACHE/sdk/include/um /imsvc $XWIN_CACHE/sdk/include/shared /c /tmp/empty.c /Fo:/tmp/empty-msvc.obj
    Expected Result: All three commands exit 0; three object files exist.
    Evidence: .sisyphus/evidence/task-15-portability-matrix.log

  Scenario: No raw _WIN32 forests remain in runtime
    Tool: Bash (grep audit)
    Preconditions: Task 15 implementation complete.
    Steps:
      1. grep -rn '#ifdef _WIN32\|#if defined(_WIN32)' runtime/ | grep -v opal_portability.h | tee /tmp/win32-leak.log
      2. wc -l < /tmp/win32-leak.log
    Expected Result: Line count is 0 (all platform detection consolidated into opal_portability.h).
    Evidence: .sisyphus/evidence/task-15-no-win32-leak.log
  ```

  **Commit**: YES (RGR: 3 commits) — `feat(runtime): opal_portability.h with MSVC shims`

- [ ] 16. **`opal_rc` — replace hardcoded offsets with `offsetof()` + `_Static_assert`**

  **What to do**:
  - Current state (research finding): `runtime/opal_rc.h:58-62` and `runtime/opal_rc.c:31-34` use **hardcoded numeric offsets** `0`, `8`, `16`, `24` for RC header field positions. This is a latent ABI bomb — any struct-layout change silently corrupts reference counting.
  - Replace with `offsetof(struct OpalRcHeader, refcount)` etc., and add `OPAL_STATIC_ASSERT` checks ensuring offsets match what codegen emits.
  - Include `<stddef.h>` for `offsetof`; use `OPAL_STATIC_ASSERT` macro from Task 15.
  - Update any codegen site that also encodes the same offsets (search `src/codegen/` for `i64 0`, `i64 8`, `i64 16`, `i64 24` in GEP instructions near RC handling) to use a shared constant (e.g., `src/codegen/rc_layout.rs::RC_REFCOUNT_OFFSET`).
  - Add a build-time consistency test: a generated `rc_layout_test.c` that asserts `offsetof()` matches the Rust constants.
  - RGR: RED = unit test `rc_header_offsets_match_codegen` computes expected offsets from struct def, asserts equal to codegen constants — PASSES before change (trivial) but FAILS if struct gets a new field. Add runtime C _Static_assert that PANICS at compile time if offsets drift. GREEN = implement offsetof + static_assert. REFACTOR = extract `rc_layout.{h,rs}` as single source of truth.

  **Must NOT do**:
  - Do NOT silently change the RC header layout — this is purely a robustness change, not a layout change. Current offsets (0/8/16/24) are the ground truth.
  - Do NOT remove the numeric offsets from codegen without replacing with a named constant — leaving bare integers is the original bug.
  - Do NOT hide the `_Static_assert` behind `#ifdef NDEBUG` — it must fire in ALL builds.

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
    - Reason: Cross-cutting change touching both C runtime and Rust codegen; high care required but pattern is well-understood.
  - **Skills**: none.

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 3 (with Tasks 17, 18, 19, 20)
  - **Blocks**: None directly; Task 18 (runtime restructure) may touch adjacent files.
  - **Blocked By**: Task 15 (portability header for `OPAL_STATIC_ASSERT`).

  **References**:
  - `runtime/opal_rc.h:58-62` — current hardcoded offset macros `#define OPAL_RC_REFCOUNT_OFFSET 0` etc.
  - `runtime/opal_rc.c:31-34` — internal use of offsets in `opal_rc_retain`/`opal_rc_release`.
  - `src/codegen/` — search for `build_struct_gep` calls on RC headers; these emit the same offsets in LLVM IR and must be unified.
  - C stdlib `<stddef.h>` `offsetof` — the portable mechanism.
  - Cross-language constant pattern: Opalescent already shares constants elsewhere? (If not, this is the first; document the convention.)

  **Acceptance Criteria**:
  - [ ] `runtime/opal_rc.h` uses `offsetof(struct OpalRcHeader, <field>)` — zero hardcoded integer offsets.
  - [ ] `OPAL_STATIC_ASSERT` checks sizeof(header) and each field offset against expected values; compile fails on drift.
  - [ ] `src/codegen/rc_layout.rs` (new or existing) defines `RC_REFCOUNT_OFFSET: u64` etc.; codegen uses these constants, not literals.
  - [ ] Build-time consistency test `rc_layout_matches_rust_constants` PASSES on Linux, Windows MSVC, Windows MinGW.
  - [ ] Runtime regression: `cargo test --features integration` all 4 test projects PASS (hello-world, fib-recursive, fib-iterative, simple-quiz — exercise RC heavily).

  **QA Scenarios**:
  ```
  Scenario: offsetof matches codegen constants across platforms
    Tool: Bash (compile + run consistency probe)
    Preconditions: Task 15 complete; portability header available.
    Steps:
      1. cargo build --release
      2. cargo test --release rc_layout_matches_rust_constants -- --nocapture 2>&1 | tee /tmp/rc-layout.log
      3. grep -q "offsets match" /tmp/rc-layout.log
    Expected Result: Test passes; log confirms offsetof values equal Rust constants.
    Evidence: .sisyphus/evidence/task-16-offsetof-match.log

  Scenario: Intentionally drifted struct layout triggers static_assert
    Tool: Bash (negative test — temporary patch)
    Preconditions: Working tree clean after Task 16.
    Steps:
      1. # Apply a temporary patch adding a field to OpalRcHeader WITHOUT updating constants
      2. sed -i 's|int32_t type_tag;|int32_t type_tag; int32_t extra_field;|' runtime/opal_rc.h
      3. cargo build --release 2>&1 | tee /tmp/rc-drift.log
      4. grep -q "static_assert\|_Static_assert" /tmp/rc-drift.log
      5. git checkout runtime/opal_rc.h  # revert
    Expected Result: Build FAILS with static_assert message citing offset mismatch.
    Evidence: .sisyphus/evidence/task-16-static-assert-fires.log
  ```

  **Commit**: YES (RGR: 3 commits) — `fix(runtime): opal_rc offsetof() + _Static_assert`

- [ ] 17. **`opal_rng` — `BCryptGenRandom` on Windows; `/dev/urandom` remains on Unix**

  **What to do**:
  - Current state (research finding): `runtime/opal_rng.c:24-27` unconditionally opens `/dev/urandom`. This path does not exist on Windows → RNG init fails, program aborts.
  - Add Windows branch using `BCryptGenRandom` (CNG API). Use the project-level `OPAL_WINDOWS` macro from Task 15's `opal_portability.h` — do NOT use raw `_WIN32` (that is reserved for `opal_portability.h` itself):
    ```c
    #include "opal_portability.h"
    #if OPAL_WINDOWS
      #include <windows.h>
      #include <bcrypt.h>
      // BCryptGenRandom(NULL, buf, len, BCRYPT_USE_SYSTEM_PREFERRED_RNG)
    #else
      // existing /dev/urandom path
    #endif
    ```
  - Link `bcrypt.lib` on Windows (add to Task 11 MSVC args and Task 12 MinGW args if not already).
  - Verify entropy quality: `BCRYPT_USE_SYSTEM_PREFERRED_RNG` gives CSPRNG equivalent to `/dev/urandom`.
  - Handle NTSTATUS error codes (`BCRYPT_SUCCESS(status)` macro — need to define or include `<bcrypt.h>` properly).
  - RGR: RED = integration test that exercises RNG (e.g., `opal_rng_produces_random_bytes` — generate 1024 bytes, check distribution is non-zero and varies). Currently PASSES on Linux, FAILS on Windows cross-build. GREEN = add Windows branch. REFACTOR = extract `opal_rng_fill(void* buf, size_t len)` as the single public API.

  **Must NOT do**:
  - Do NOT use `rand()` / `srand()` — these are not CSPRNGs; insecure for any runtime that exposes RNG to user programs.
  - Do NOT use `CryptGenRandom` (deprecated CryptoAPI) — `BCryptGenRandom` (CNG) is the modern API.
  - Do NOT open `/dev/urandom` unconditionally — that is the bug.
  - Do NOT add a `rand_s`-based fallback; stick to BCryptGenRandom.

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
    - Reason: Platform-specific crypto API integration; moderate complexity, requires Win32 API familiarity.
  - **Skills**: none.

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 3 (with Tasks 16, 18, 19, 20)
  - **Blocks**: Task 21 (CI will test RNG as part of integration tests).
  - **Blocked By**: Task 15 (portability header for platform detection macros).

  **References**:
  - `runtime/opal_rng.c:24-27` — current /dev/urandom site.
  - Microsoft docs: `https://learn.microsoft.com/en-us/windows/win32/api/bcrypt/nf-bcrypt-bcryptgenrandom`.
  - `BCRYPT_USE_SYSTEM_PREFERRED_RNG` flag: produces CSPRNG-quality bytes.
  - Link lib: `bcrypt.lib` (MSVC) / `-lbcrypt` (MinGW).
  - Task 11 MSVC linker args (should already include `bcrypt.lib`); Task 12 MinGW args (should already include `-lbcrypt`) — verify, add if missing.
  - Similar pattern: `https://github.com/rust-lang/rust/blob/master/library/std/src/sys/pal/windows/rand.rs` — Rust stdlib uses same API.

  **Acceptance Criteria**:
  - [ ] `runtime/opal_rng.c` has `#if OPAL_WINDOWS` branch using `BCryptGenRandom` (uses Task 15's `opal_portability.h` macro; no raw `_WIN32`).
  - [ ] `bcrypt.lib` / `-lbcrypt` present in linker args for Windows targets.
  - [ ] Integration test `opal_rng_produces_random_bytes` PASSES on Linux (urandom), Windows MSVC (BCrypt), Windows MinGW (BCrypt).
  - [ ] Statistical sanity: 1024 bytes have entropy ≥ 7.0 bits/byte (crude check: distinct byte values ≥ 200).
  - [ ] No abort on startup on Windows: `opalescent hello_world.op --target x86_64-pc-windows-msvc && wine program.exe` runs to completion.

  **QA Scenarios**:
  ```
  Scenario: RNG fills buffer with varied bytes on Windows (via Wine)
    Tool: Bash (cross-compile + wine)
    Preconditions: Linux host + MinGW + wine + LLVM 14; Task 15 portability header committed; Task 17 Windows RNG branch committed.
    Steps:
      1. Create concrete test fixture at `test-projects/rng-probe/src/main.op` (committed, not /tmp):
         ```
         import "std/rng" as rng
         import "std/io" as io

         entry main = f(args: string[]): i32 => {
           let buf: u8[16] = rng.fill_bytes(16)
           // Print exactly 16 hex pairs, no separator, followed by newline
           for i in 0..16 {
             io.print_hex_byte(buf[i])
           }
           io.println("")
           return 0
         }
         ```
         (If `std/rng` API differs, use the canonical RNG import from `test-projects/hello-world/` as pattern and adapt; the test MUST print exactly 32 hex chars + "\n" — 33 bytes total output.)
      2. `cargo run --release -- test-projects/rng-probe/src/main.op --target x86_64-pc-windows-gnu 2>&1 | tee .sisyphus/evidence/task-17-rng-windows-build.log`
      3. `wine test-projects/rng-probe/target/main.exe > .sisyphus/evidence/task-17-rng-windows.out 2> .sisyphus/evidence/task-17-rng-windows.err; echo "exit=$?" | tee -a .sisyphus/evidence/task-17-rng-windows.log`
      4. Deterministic assertions against `.sisyphus/evidence/task-17-rng-windows.out`:
         - `[ "$(wc -c < .sisyphus/evidence/task-17-rng-windows.out)" = "33" ]` — exactly 32 hex chars + newline.
         - `grep -qE '^[0-9a-f]{32}$' .sisyphus/evidence/task-17-rng-windows.out` — 32 lowercase hex chars on one line.
         - `! grep -qE '^0{32}$' .sisyphus/evidence/task-17-rng-windows.out` — NOT all zeros (RNG returned real entropy).
         - Distinct-byte sanity: `HEX=$(cat .sisyphus/evidence/task-17-rng-windows.out); DISTINCT=$(echo "$HEX" | fold -w2 | sort -u | wc -l); [ "$DISTINCT" -ge 8 ]` — at least 8 distinct byte values out of 16 (probability of fewer with real CSPRNG ≈ 10⁻⁶).
         - Re-run step 3 to produce a second output; `! diff .sisyphus/evidence/task-17-rng-windows.out .sisyphus/evidence/task-17-rng-windows.out2` — two consecutive runs produce DIFFERENT bytes (RNG is not deterministic).
      5. Exit code assertion: `grep -q "exit=0" .sisyphus/evidence/task-17-rng-windows.log`.
    Expected Result: 33-byte output (32 hex + newline); matches `^[0-9a-f]{32}$`; not all zeros; ≥8 distinct byte values; two runs produce different outputs; program exits 0.
    Failure Indicators: Output length ≠ 33; any non-hex char; all-zero output (RNG broken); fewer than 8 distinct byte values (entropy catastrophically low); identical outputs across runs (RNG seeded deterministically — wrong); non-zero exit.
    Evidence: .sisyphus/evidence/task-17-rng-windows-build.log, .sisyphus/evidence/task-17-rng-windows.out, .sisyphus/evidence/task-17-rng-windows.out2, .sisyphus/evidence/task-17-rng-windows.err, .sisyphus/evidence/task-17-rng-windows.log

  Scenario: Linux /dev/urandom path unchanged
    Tool: Bash (strace)
    Preconditions: Linux host.
    Steps:
      1. cargo run --release -- test-projects/hello-world/src/main.op  # compile
      2. strace -e openat ./test-projects/hello-world/target/program 2>&1 | grep -c urandom
    Expected Result: Count ≥ 1 (process opens /dev/urandom at RNG init).
    Evidence: .sisyphus/evidence/task-17-rng-linux-urandom.log
  ```

  **Commit**: YES (RGR: 3 commits) — `feat(runtime): BCryptGenRandom on Windows`

- [ ] 18. **Restructure `opal_runtime.c` — per-platform aggregator instead of `#include`-of-`.c`-files**

  **What to do**:
  - Current state (research finding): `runtime/opal_runtime.c:1-6` textually `#include`s other `.c` files (`opal_rc.c`, `opal_parse.c`, etc.) — a unity-build trick that works with GCC but confuses MSVC toolchain parsing and breaks incremental builds.
  - Replace with a proper multi-TU build:
    - Each `runtime/opal_<module>.c` becomes an independent translation unit.
    - `runtime/opal_runtime.h` becomes the public umbrella header listing all public APIs.
    - Build script (Rust `build.rs` or the compiler's embedded build logic) compiles all runtime `.c` files separately and links them together (or archives into `libopal_runtime.a` / `opal_runtime.lib`).
  - Alternative if keeping unity build: rename `opal_runtime.c` to `opal_runtime_unity.c` and guard it with `#if !defined(OPAL_SEPARATE_TUS)`; provide a per-TU build path for MSVC.
  - Update `src/compiler.rs` to invoke the C compiler once per TU, collect all `.o`/`.obj` files, and pass them all to the linker.
  - RGR: RED = build fixture: run the full compile pipeline with MSVC → verify all runtime .c files compile to separate `.obj` files (check file listing). Currently FAILS (single unity .obj). GREEN = restructure. REFACTOR = extract build-script helper `compile_runtime_tus(target: &TargetTriple) -> Vec<PathBuf>`.

  **Must NOT do**:
  - Do NOT break the current Linux unity build without a replacement — if unity is kept, gate it behind `#if !defined(_MSC_VER)`.
  - Do NOT introduce `cmake` or `meson` as a build dependency — the compiler orchestrates its own C build via the `cc` crate or direct invocation.
  - Do NOT allow duplicate symbols across TUs — that is the current risk of naive unity-to-multi migration. Check with `nm -g` / `dumpbin /symbols`.
  - Do NOT depend on GCC-specific pragmas (`#pragma GCC optimize`) in per-TU code.

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: Build-system restructure with subtle symbol-visibility and link-order concerns across three toolchains.
  - **Skills**: none.

  **Parallelization**:
  - **Can Run In Parallel**: PARTIAL (coordinates with Tasks 16, 17, 19, 20 which modify runtime .c files — sequence edits carefully).
  - **Parallel Group**: Wave 3 (with Tasks 16, 17, 19, 20) — but other Wave-3 tasks should rebase on Task 18 if conflicts arise.
  - **Blocks**: Task 21 (xwin CI), Task 22 (Wine CI), Task 26 (Windows native tests).
  - **Blocked By**: Task 15 (portability header).

  **References**:
  - `runtime/opal_runtime.c:1-6` — current textual includes of .c files.
  - `runtime/opal_rc.c`, `opal_parse.c`, `opal_io.c`, `opal_string.c`, `opal_print.c`, `opal_rng.c` — all six become independent TUs.
  - Rust `cc` crate: `https://docs.rs/cc/latest/cc/` — handles MSVC/GCC/Clang abstraction if `build.rs` is chosen.
  - MSVC `/c` flag for separate compilation; `/Fo` for object output.
  - Existing `src/compiler.rs:307-342` C compile invocation — extend to loop over runtime TUs.

  **Acceptance Criteria**:
  - [ ] Each `runtime/opal_*.c` compiles independently with `gcc -c` (Linux host), `x86_64-w64-mingw32-gcc -c` (MinGW cross), `clang-cl /c /imsvc $XWIN_CACHE/...` (Linux host → MSVC target), and `cl.exe /c` (Windows host → MSVC target). Note: `cl.exe` is invoked ONLY on Windows hosts; Linux hosts use `clang-cl` with xwin sysroot — `xwin-run` and `wine cl.exe` are FORBIDDEN.
  - [ ] Zero duplicate symbols: `nm -g runtime/*.o | sort | uniq -d` is empty (Linux); `dumpbin /symbols runtime\*.obj` no duplicates (Windows).
  - [ ] `cargo test --features integration` all 4 test projects PASS on Linux (unchanged behavior).
  - [ ] Cross-compile test: `opalescent hello_world.op --target x86_64-pc-windows-msvc` produces `.exe`; wine executes; output "hello".
  - [ ] Build-time audit log shows N C invocations (one per TU) instead of 1 unity build.

  **QA Scenarios**:
  ```
  Scenario: Per-TU compilation produces distinct object files
    Tool: Bash (build audit)
    Preconditions: Linux host, gcc + MinGW installed.
    Steps:
      1. RUST_LOG=opalescent::compiler::runtime_build=debug cargo run --release -- test-projects/hello-world/src/main.op 2>&1 | tee /tmp/build.log
      2. grep -c 'compiling runtime TU:' /tmp/build.log  # expect ≥ 6
      3. ls test-projects/hello-world/target/runtime_objs/*.o | wc -l  # expect 6
    Expected Result: ≥ 6 TU compile lines; 6 object files on disk.
    Evidence: .sisyphus/evidence/task-18-per-tu-build.log

  Scenario: No duplicate symbols across runtime TUs
    Tool: Bash (nm)
    Preconditions: Per-TU build artifacts present.
    Steps:
      1. nm -g --defined-only test-projects/hello-world/target/runtime_objs/*.o | awk '{print $3}' | sort | uniq -d | tee /tmp/dup-syms.log
      2. wc -l < /tmp/dup-syms.log
    Expected Result: Line count is 0.
    Evidence: .sisyphus/evidence/task-18-no-dup-symbols.log
  ```

  **Commit**: YES (RGR: 3 commits) — `refactor(runtime): per-platform aggregator for opal_runtime.c`

- [ ] 19. **MSVC shims: `getline`, `strdup`, `ssize_t`**

  **What to do**:
  - Current state (research finding):
    - `runtime/opal_io.c:6,30` — `getline(&line, &n, stream)` is POSIX, not in MSVC.
    - `runtime/opal_string.c:88` — `strdup(s)` exists on MSVC as `_strdup` (leading underscore); `ssize_t` missing.
    - `runtime/opal_io.c` — uses `ssize_t` return from `getline`.
  - Implement MSVC-compatible shims in `opal_portability.h` (Task 15) or a new `runtime/opal_compat_msvc.c`:
    - `opal_getline(char** line, size_t* n, FILE* stream) -> ssize_t`: portable implementation using `fgetc` loop + dynamic realloc.
    - `#define strdup _strdup` on MSVC (or provide `opal_strdup` wrapper).
    - `typedef intptr_t ssize_t` for MSVC (already in Task 15 portability header — verify).
  - Replace all runtime uses of `getline` with `opal_getline`; all uses of `strdup` with `opal_strdup`.
  - Test the shim: feed various input sizes (0, 1, 1KB, 1MB) to ensure realloc growth works.
  - RGR: RED = unit test `opal_getline_matches_posix_getline` runs both side-by-side on identical input (Linux only, where POSIX getline exists), asserts identical behavior. Then cross-compile same test with MSVC and run via wine, asserting behavior identical to Linux baseline. GREEN = implement shim. REFACTOR = extract common fgetc-loop helper if there's duplication with a potential future `opal_getdelim`.

  **Must NOT do**:
  - Do NOT use `gets()` or `gets_s()` — gets is removed in C11; gets_s has awkward bounds semantics. Use `fgetc` loop.
  - Do NOT assume lines fit in a fixed buffer (e.g., `char buf[4096]`) — `getline` semantics are dynamically growing.
  - Do NOT forget to handle EOF, errno, and the "last line without newline" edge case.
  - Do NOT leak memory: initial buffer allocation must be freed on error.

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
    - Reason: Classic POSIX shim territory; moderate complexity, needs careful EOF and realloc handling.
  - **Skills**: none.

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 3 (with Tasks 16, 17, 18, 20)
  - **Blocks**: Task 21 (CI), Task 26 (native tests).
  - **Blocked By**: Task 15 (portability header for ssize_t typedef).

  **References**:
  - `runtime/opal_io.c:6` — `getline` declaration/prototype.
  - `runtime/opal_io.c:30` — `getline` call site.
  - `runtime/opal_string.c:88` — `strdup` + `ssize_t` usage.
  - POSIX getline spec: `https://pubs.opengroup.org/onlinepubs/9699919799/functions/getline.html`.
  - MSVC `_strdup`: `https://learn.microsoft.com/en-us/cpp/c-runtime-library/reference/strdup-wcsdup-mbsdup`.
  - Reference implementation (public domain, permissively licensed): `https://github.com/ivanrad/getline/blob/master/getline.c`.

  **Acceptance Criteria**:
  - [ ] `opal_getline` function defined; handles EOF, empty line, last-line-no-newline, 1MB line.
  - [ ] All `getline(` call sites in runtime replaced with `opal_getline(`; grep audit: `grep -rn '[^_]getline(' runtime/ | grep -v 'opal_getline\|fgetline'` returns 0 lines.
  - [ ] All `strdup(` replaced with `opal_strdup(`; same grep audit clean.
  - [ ] `ssize_t` used consistently via portability header; not redefined elsewhere.
  - [ ] Unit test `opal_getline_handles_edge_cases` PASSES on all 3 toolchains.
  - [ ] Linux regression: simple-quiz test project (uses stdin via getline) still PASSES.

  **QA Scenarios**:
  ```
  Scenario: opal_getline reads a 1MB line on Windows via Wine
    Tool: Bash (cross-compile + wine)
    Preconditions: Linux host + MinGW + wine.
    Steps:
      1. python3 -c "print('a'*1048576)" > /tmp/big-line.txt
      2. cargo run --release -- test-projects/simple-quiz/src/main.op --target x86_64-pc-windows-gnu
      3. wine test-projects/simple-quiz/target/program.exe < /tmp/big-line.txt > /tmp/out.txt
      4. wc -c < /tmp/out.txt  # sanity: output proportional to input
    Expected Result: Program reads 1MB line without crash; exits 0.
    Evidence: .sisyphus/evidence/task-19-getline-1mb.log

  Scenario: EOF without trailing newline handled
    Tool: Bash
    Preconditions: Linux host (baseline).
    Steps:
      1. printf 'noNewlineAtEnd' | ./test-projects/simple-quiz/target/program > /tmp/out.txt
      2. grep -q 'noNewlineAtEnd' /tmp/out.txt
    Expected Result: Last line read successfully despite missing newline.
    Evidence: .sisyphus/evidence/task-19-getline-eof.log
  ```

  **Commit**: YES (RGR: 3 commits) — `feat(runtime): MSVC getline/strdup reimpl`

- [ ] 20. **`PRId64` / `PRIu64` fallback for MSVC without `<inttypes.h>`**

  **What to do**:
  - Current state (research finding): `runtime/opal_print.c:3,18,34` use `PRId64`, `PRIu64` macros from `<inttypes.h>`. Some MSVC configurations (older SDKs, `/std:c11` strict) may not expand these correctly, or may use `"I64d"` / `"I64u"` variants.
  - Verify: modern MSVC (VS2019+) DOES ship `<inttypes.h>` with proper PRI macros — but guard with a fallback anyway for robustness.
  - In `opal_portability.h` (Task 15), add:
    ```c
    #include <inttypes.h>
    #ifndef PRId64
      #if defined(_MSC_VER)
        #define PRId64 "I64d"
        #define PRIu64 "I64u"
      #else
        #error "PRId64 not defined and compiler is not MSVC"
      #endif
    #endif
    ```
  - Alternative: always use `%lld` / `%llu` and `(long long)` casts — but this loses type safety for int64_t. Prefer PRI macros with fallback.
  - Test format-specifier correctness: print `INT64_MAX`, `INT64_MIN`, `UINT64_MAX` and verify stringified output matches expected decimal representation on all 3 toolchains.
  - RGR: RED = unit test `int64_print_matches_on_all_toolchains` compiles same printf on gcc/mingw/msvc, runs, compares outputs — FAILS if any toolchain differs. GREEN = add fallback. REFACTOR = none (this is already minimal).

  **Must NOT do**:
  - Do NOT use `%ld` or `%lld` without `PRId64` — width depends on platform (`long` is 32-bit on MSVC, 64-bit on Linux LP64).
  - Do NOT silently fall back to `%d` — will truncate on 64-bit values.
  - Do NOT duplicate the fallback in every `.c` file — it must live in portability header only.

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Mechanical macro addition with clear verification. Low complexity.
  - **Skills**: none.

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 3 (with Tasks 16, 17, 18, 19)
  - **Blocks**: None directly.
  - **Blocked By**: Task 15 (portability header).

  **References**:
  - `runtime/opal_print.c:3` — `PRId64` usage in format string.
  - `runtime/opal_print.c:18, 34` — additional PRI sites.
  - `<inttypes.h>` C99 standard.
  - MSVC legacy: `"I64d"` / `"I64u"` format specifiers predate C99 and always work.
  - MSVC modern: VS2015+ ships full C99 `<inttypes.h>` — fallback rarely triggers but must exist.

  **Acceptance Criteria**:
  - [ ] `opal_portability.h` has `#ifndef PRId64` fallback block for MSVC.
  - [ ] `printf("%" PRId64 "\n", INT64_MAX)` prints `9223372036854775807` on gcc, MinGW, MSVC.
  - [ ] `printf("%" PRIu64 "\n", UINT64_MAX)` prints `18446744073709551615` on all three.
  - [ ] `#error` branch never fires with supported toolchains (verify by grepping build log).
  - [ ] Linux regression: fib-recursive / fib-iterative (heavy int64 usage) PASS.

  **QA Scenarios**:
  ```
  Scenario: int64 boundary values print identically across toolchains
    Tool: Bash (3-way compile + run)
    Preconditions: Linux host; `gcc` installed; `x86_64-w64-mingw32-gcc` installed; `clang-cl` + `lld-link` available (LLVM 14+); `wine` installed; `$XWIN_CACHE` populated per Task 0 (splat layout: `$XWIN_CACHE/crt/{include,lib/x86_64}`, `$XWIN_CACHE/sdk/{include,lib}/{ucrt,um,shared}/x86_64`); Task 15 portability header at `runtime/opal_portability.h`; Task 20 PRI fallback block present.
    Steps:
      1. `cat > /tmp/pri_probe.c <<'EOF'` … EOF body (as currently written — C source that prints INT64_MAX/MIN/UINT64_MAX with PRI macros).
      2. Linux gcc: `gcc -std=c11 -I$(pwd)/runtime /tmp/pri_probe.c -o /tmp/pri-gcc 2>&1 | tee .sisyphus/evidence/task-20-gcc-build.log && /tmp/pri-gcc > .sisyphus/evidence/task-20-out-gcc.txt`
      3. MinGW cross: `x86_64-w64-mingw32-gcc -std=c11 -I$(pwd)/runtime /tmp/pri_probe.c -o /tmp/pri-mingw.exe 2>&1 | tee .sisyphus/evidence/task-20-mingw-build.log && wine /tmp/pri-mingw.exe > .sisyphus/evidence/task-20-out-mingw.txt 2> .sisyphus/evidence/task-20-mingw-wine.err`
      4. MSVC cross (clang-cl + lld-link + xwin sysroot):
         - Compile to object: `clang-cl --target=x86_64-pc-windows-msvc /std:c11 /I$(pwd)/runtime /imsvc $XWIN_CACHE/crt/include /imsvc $XWIN_CACHE/sdk/include/ucrt /imsvc $XWIN_CACHE/sdk/include/um /imsvc $XWIN_CACHE/sdk/include/shared /c /Fo/tmp/pri_probe.obj /tmp/pri_probe.c 2>&1 | tee .sisyphus/evidence/task-20-msvc-compile.log`
         - Link to exe: `lld-link /nologo /subsystem:console /libpath:$XWIN_CACHE/crt/lib/x86_64 /libpath:$XWIN_CACHE/sdk/lib/ucrt/x86_64 /libpath:$XWIN_CACHE/sdk/lib/um/x86_64 /out:/tmp/pri-msvc.exe /tmp/pri_probe.obj libcmt.lib libucrt.lib libvcruntime.lib kernel32.lib 2>&1 | tee .sisyphus/evidence/task-20-msvc-link.log`
         - Run under wine: `wine /tmp/pri-msvc.exe > .sisyphus/evidence/task-20-out-msvc.txt 2> .sisyphus/evidence/task-20-msvc-wine.err`
      5. Assert identical output across all three toolchains:
         - `diff .sisyphus/evidence/task-20-out-gcc.txt .sisyphus/evidence/task-20-out-mingw.txt | tee .sisyphus/evidence/task-20-diff-gcc-mingw.log` — must be empty.
         - `diff .sisyphus/evidence/task-20-out-gcc.txt .sisyphus/evidence/task-20-out-msvc.txt | tee .sisyphus/evidence/task-20-diff-gcc-msvc.log` — must be empty.
         - Assert exact bytes: `printf '9223372036854775807\n-9223372036854775808\n18446744073709551615\n' | diff - .sisyphus/evidence/task-20-out-gcc.txt` — must be empty (canonical reference).
    Expected Result: All three toolchains produce the exact byte sequence `9223372036854775807\n-9223372036854775808\n18446744073709551615\n`; both `diff` commands in step 5 exit 0 with empty output; canonical-reference diff exits 0.
    Failure Indicators: Any `diff` reports differences; any build command exits non-zero; any toolchain prints wrong decimal representation (truncation on MSVC would show `2147483647` instead of `9223372036854775807` — catastrophic format-specifier bug).
    Evidence: .sisyphus/evidence/task-20-gcc-build.log, task-20-mingw-build.log, task-20-msvc-compile.log, task-20-msvc-link.log, task-20-out-gcc.txt, task-20-out-mingw.txt, task-20-out-msvc.txt, task-20-diff-gcc-mingw.log, task-20-diff-gcc-msvc.log

  Scenario: Fallback never triggers with VS2019+ (defensive check)
    Tool: Bash
    Preconditions: Task 20 complete.
    Steps:
      1. grep -A2 'PRId64 not defined' runtime/opal_portability.h  # inspect fallback
      2. # Force fallback: define PRId64 undef before include, build, check no #error
    Expected Result: Fallback path exists but benign; no #error in normal builds.
    Evidence: .sisyphus/evidence/task-20-fallback-benign.log
  ```

  **Commit**: YES (RGR: 3 commits) — `fix(runtime): PRId64/PRIu64 MSVC fallback`

- [ ] 21. **CI: xwin integration for Linux → MSVC cross-compilation**

  **What to do**:
  - Add a GitHub Actions job `cross-msvc-from-linux` (ubuntu-latest) to `.github/workflows/ci.yml` (scaffold from Task 7).
  - Install LLVM toolchain (provides `clang-cl` and `lld-link`, which are what we actually use on Linux hosts — xwin does NOT provide `cl.exe`/`link.exe`):
    ```yaml
    - run: |
        sudo apt-get update
        sudo apt-get install -y lld clang
        # Verify lld-link is available (lld-link is part of the lld package)
        which lld-link || which lld-link-14 || (echo "lld-link missing" && exit 1)
    ```
  - Install [xwin](https://github.com/Jake-Shadle/xwin) to fetch the MSVC **headers + import libs + SDK** (xwin does NOT provide compiler or linker binaries — it is a sysroot provider only):
    ```yaml
    - run: |
        cargo install xwin --locked
        xwin --accept-license splat --output $HOME/.xwin
    ```
  - Configure environment so `clang-cl` can compile MSVC-target C code and `lld-link` can link it, using the xwin splat as a `/winsysroot`-equivalent on Linux:
    - `XWIN_CACHE=$HOME/.xwin` (env var consumed by Task 11's Linux-host MSVC linker path and Task 15's verification scenarios).
    - `CC_x86_64_pc_windows_msvc=clang-cl` with include flags wired via `cc` crate env: `CFLAGS_x86_64_pc_windows_msvc="/imsvc $HOME/.xwin/crt/include /imsvc $HOME/.xwin/sdk/include/ucrt /imsvc $HOME/.xwin/sdk/include/um /imsvc $HOME/.xwin/sdk/include/shared"`.
    - Linker: Task 11's `Linker::Msvc` branch on Linux will pick `lld-link` and synthesize `/libpath:` args for `$XWIN_CACHE/crt/lib/x86_64`, `$XWIN_CACHE/sdk/lib/ucrt/x86_64`, `$XWIN_CACHE/sdk/lib/um/x86_64`.
  - Run: `cargo build --release` + `cargo run --release -- test-projects/hello-world/src/main.op --target x86_64-pc-windows-msvc` to produce a `.exe`.
  - Artifact upload: cross-compiled `program.exe` → GitHub artifact for inspection.
  - RGR: RED = CI job fails because `lld-link` and/or `XWIN_CACHE` are missing. GREEN = install lld + xwin + wire env. REFACTOR = cache xwin splat directory via `actions/cache` keyed on xwin version.

  **Must NOT do**:
  - Do NOT claim xwin provides `cl.exe` or `link.exe` — it provides headers/libs/SDK ONLY. Compilation uses `clang-cl`; linking uses `lld-link` (both from LLVM).
  - Do NOT invoke `xwin-run` — no such tool exists.
  - Do NOT try to `wine cl.exe` or `wine link.exe` — we use native Linux `clang-cl` + `lld-link` (drop-in argv-compatible with MSVC) against the xwin sysroot.
  - Do NOT commit the xwin-fetched Microsoft headers/libs into the repo (license forbids redistribution; xwin splats them at CI time from Microsoft servers under EULA).
  - Do NOT use `zig cc` (banned).
  - Do NOT hardcode xwin version — use a pinned release tag in a repo-level variable for easy bumping.
  - Do NOT skip `--accept-license` — this is required for xwin to download MS SDK components.

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
    - Reason: CI YAML + toolchain orchestration. Requires understanding of GitHub Actions, xwin semantics, and MSVC toolchain env vars.
  - **Skills**: none.

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 4 (with Tasks 22, 23, 24, 25)
  - **Blocks**: Task 26 (Windows native matrix builds on this infrastructure).
  - **Blocked By**: Task 7 (CI skeleton), Task 11 (MSVC linker path), Task 14 (target threading), Task 15-20 (runtime portability).

  **References**:
  - xwin docs: `https://github.com/Jake-Shadle/xwin` — canonical tool.
  - GitHub Actions `Swatinem/rust-cache` — cache Rust build artifacts.
  - GitHub Actions `actions/cache` — cache xwin splat directory.
  - Task 7 `.github/workflows/ci.yml` — extend, don't rewrite.
  - Microsoft EULA for SDK: xwin handles acceptance flag; document in PR description.

  **Acceptance Criteria**:
  - [ ] `.github/workflows/ci.yml` has `cross-msvc-from-linux` job running on `ubuntu-latest`.
  - [ ] xwin splat directory cached across runs (cache hit rate ≥ 80% after first run).
  - [ ] Job produces `program.exe` artifact (PE32+, x86-64).
  - [ ] Job time budget: first run ≤ 15 min; cached run ≤ 5 min.
  - [ ] Job enforced as required status check on main branch.

  **QA Scenarios**:
  ```
  Scenario: CI cross-compile produces valid PE32+ artifact
    Tool: Bash (gh CLI)
    Preconditions: PR open with Task 21 changes.
    Steps:
      1. gh run watch --exit-status  # wait for CI
      2. gh run download <run-id> --name hello-world-msvc-exe -D /tmp/artifact
      3. file /tmp/artifact/program.exe
    Expected Result: file output: "PE32+ executable (console) x86-64, for MS Windows".
    Evidence: .sisyphus/evidence/task-21-xwin-artifact.log

  Scenario: xwin cache hit on second run
    Tool: Bash (gh CLI)
    Preconditions: At least one CI run completed.
    Steps:
      1. gh run rerun <run-id>
      2. gh run view <new-run-id> --log | grep -i 'cache hit'
    Expected Result: "Cache restored successfully" for xwin splat.
    Evidence: .sisyphus/evidence/task-21-xwin-cache-hit.log
  ```

  **Commit**: YES — `ci: xwin integration for cross-compile`

- [ ] 22. **CI: Wine job — execute cross-compiled .exe on Linux**

  **What to do**:
  - Extend `.github/workflows/ci.yml` with job `cross-wine-run` (ubuntu-latest) that consumes the `.exe` artifact from Task 21's xwin job and the Task 12 MinGW path.
  - Install wine: `sudo apt-get install -y wine64 winbind`.
  - Configure headless wine: `DISPLAY=:0 xvfb-run -a wine program.exe` (though CLI-only tests don't need Xvfb).
  - Run all 4 integration test projects: hello-world, fib-recursive, fib-iterative, simple-quiz via `wine`, collect stdout, compare to Linux-native expected output.
  - For simple-quiz (stdin required): pipe canned input via `printf '42\n' | wine program.exe`.
  - Matrix: run both `x86_64-pc-windows-msvc` (from Task 21 xwin) and `x86_64-pc-windows-gnu` (from Task 12 MinGW) binaries through wine.
  - RGR: RED = job fails because wine not installed. GREEN = install + run. REFACTOR = extract shared `run_with_wine.sh` helper script that takes a .exe path and expected output.

  **Must NOT do**:
  - Do NOT mark wine failures as non-blocking — if a cross-compiled .exe doesn't run under wine, Windows support is broken.
  - Do NOT use an old wine version from apt — install wine-stable ≥ 8.0 for modern Win64 support.
  - Do NOT run GUI tests in wine yet — CLI tests only for v1 (consistent with hello-world etc.).
  - Do NOT accept "wine reports warnings, but exit code 0" — capture wine's output fully; failure modes often manifest as warnings + wrong stdout.

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
    - Reason: CI integration with external emulator; moderate complexity but clear success criteria.
  - **Skills**: none.

  **Parallelization**:
  - **Can Run In Parallel**: YES (runs after Task 21 in CI DAG, but Task 22 IMPLEMENTATION can proceed in parallel with Task 21).
  - **Parallel Group**: Wave 4 (with Tasks 21, 23, 24, 25)
  - **Blocks**: Task 26 (native Windows tests use same expected-output fixtures).
  - **Blocked By**: Task 12 (MinGW linker), Task 15-20 (runtime), Task 21 (xwin job provides msvc artifacts).

  **References**:
  - Wine install on Ubuntu: `https://wiki.winehq.org/Ubuntu`.
  - Task 7 CI skeleton; Task 21 xwin job produces `.exe` to consume.
  - Test project expected outputs: `test-projects/<name>/README.md` — document expected stdout for each.
  - Headless wine: `https://askubuntu.com/questions/1013707/wine-in-ci-environments`.

  **Acceptance Criteria**:
  - [ ] `.github/workflows/ci.yml` has `cross-wine-run` job.
  - [ ] Wine version ≥ 8.0 installed (verify: `wine --version`).
  - [ ] Matrix runs 4 test projects × 2 targets (msvc, gnu) = 8 wine invocations, all PASS.
  - [ ] Each wine invocation captures stdout; comparison to expected output is exact (no "contains" — full match).
  - [ ] Job enforced as required status check.
  - [ ] Total job time ≤ 10 min (wine is slow but not that slow for these small programs).

  **QA Scenarios**:
  ```
  Scenario: All 4 test projects pass under wine (msvc build)
    Tool: Bash (CI log inspection)
    Preconditions: PR with Task 22 open.
    Steps:
      1. gh run watch --exit-status --job cross-wine-run
      2. gh run view <run-id> --log --job cross-wine-run | grep -E '(PASS|FAIL)'
    Expected Result: 8 PASS lines, 0 FAIL.
    Evidence: .sisyphus/evidence/task-22-wine-matrix.log

  Scenario: simple-quiz stdin handling works under wine
    Tool: Bash (local reproduction of CI step)
    Preconditions: Linux + MinGW + wine.
    Steps:
      1. ./target/release/opalescent test-projects/simple-quiz/src/main.op --target x86_64-pc-windows-gnu
      2. printf '42\n' | wine test-projects/simple-quiz/target/program.exe > /tmp/quiz-out.txt 2>&1
      3. grep -q 'correct' /tmp/quiz-out.txt  # or whatever expected output is
    Expected Result: Quiz output matches Linux baseline.
    Evidence: .sisyphus/evidence/task-22-wine-stdin.log
  ```

  **Commit**: YES — `ci: Wine job executing cross-compiled .exe`

- [ ] 23. **Real `FsModuleLoader` with `libloading` (Linux `.so` — upgrade from stub)**

  **What to do**:
  - Current state (research finding): `src/hot_reload/` has a `FsModuleLoader` stub that doesn't actually load `.so` files — hot reload is effectively a test-mock on Linux.
  - Replace stub with real implementation using the [`libloading`](https://crates.io/crates/libloading) crate:
    - `libloading::Library::new(&path)` — opens `.so`.
    - `lib.get::<Symbol>(b"func_name")` — resolves exported symbol.
    - Handle `Drop` to call `dlclose` implicitly.
  - Maintain the existing `ModuleLoader` trait boundary — this is swap-in behavior.
  - Target-aware extension helper (CRITICAL fix — current code is host-based, not target-based):
    - GROUND TRUTH (`src/hot_reload/version.rs:41-55`): current code is `const fn shared_library_extension() -> &'static str` with NO parameter, using host `#[cfg(target_os = "windows")]` / `#[cfg(target_os = "macos")]` / `#[cfg(not(any(...)))]`. This is HOST-driven and BREAKS cross-compile (Linux→Windows would return `".so"` when it must return `".dll"`).
    - CHANGE: replace the param-less `const fn shared_library_extension()` with a target-driven `pub fn shared_library_extension(target: &TargetTriple) -> &'static str` that dispatches on `target.platform` (via Task 0.5's expanded `TargetTriple`): `Windows → "dll"`, `MacOs → "dylib"`, `Linux → "so"`. Remove the `#[cfg(target_os)]` branches.
    - Update every call site of `shared_library_extension()` in `src/hot_reload/` (and anywhere else — run `lsp_find_references` on the symbol) to pass a `&TargetTriple`. Primary call site: `versioned_module_name(...)` — thread the target through.
    - RED test (separate from Task 23's main integration test): `shared_library_extension(&TargetTriple::windows_msvc_x86_64()) == "dll"` on a Linux host, and `shared_library_extension(&TargetTriple::linux_x86_64()) == "so"` on a Linux host. Both must hold regardless of host OS — proves target-driven, not host-driven.
  - RGR: RED = integration test `real_so_hot_reload_swaps_implementation` compiles two versions of a tiny `.op` module to `.so`, loads v1, calls exported fn, asserts v1 output, swaps to v2, asserts v2 output. Currently FAILS (stub returns mock). GREEN = implement libloading-backed loader. REFACTOR = extract `SymbolResolver` abstraction for unit testing.

  **Must NOT do**:
  - Do NOT use raw `libc::dlopen` / `dlsym` — libloading provides RAII and is cross-platform (bridges to `LoadLibraryW` on Windows, which Task 24 consumes).
  - Do NOT leak `Library` handles — `Drop` must close them.
  - Do NOT forget the COPY-BEFORE-LOAD pattern — Linux file locks are advisory but modifying an open .so is still UB risk. Copy to a temp path before loading; delete on unload.
  - Do NOT swap modules synchronously with in-flight calls — existing ABI-guard + atomic-swap protocol (from research) must remain intact.

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: Hot-reload correctness is subtle; libloading integration must preserve existing atomic-swap invariants.
  - **Skills**: none.

  **Parallelization**:
  - **Can Run In Parallel**: YES (Task 24 Windows .dll builds on top of Task 23's libloading integration).
  - **Parallel Group**: Wave 4 (with Tasks 21, 22, 24, 25)
  - **Blocks**: Task 24 (Windows `.dll` path uses same libloading crate), Task 27 (hot-reload integration test).
  - **Blocked By**: Task 14 (target threading — loader needs target-aware extension).

  **References**:
  - `libloading` crate: `https://docs.rs/libloading/latest/libloading/`.
  - `src/hot_reload/version.rs:41-55` — existing param-less host-cfg-based `shared_library_extension()`; this task REPLACES it with a target-driven `shared_library_extension(target: &TargetTriple)`.
  - `versioned_module_name` in the same file — primary call site that must be updated to accept/thread a `&TargetTriple`.
  - `src/hot_reload/loader.rs` (or equivalent; research identified `FsModuleLoader` as stub here).
  - Research finding from prior session: hot reload ABI-guard + atomic-swap mechanism exists and must be preserved.
  - COPY-BEFORE-LOAD pattern: standard practice; see `cargo-watch`, `dioxus` hot-reload for reference.

  **Acceptance Criteria**:
  - [ ] `libloading = "0.8"` added to `Cargo.toml`.
  - [ ] `FsModuleLoader` calls `libloading::Library::new` — no mocks in non-test code.
  - [ ] `src/hot_reload/version.rs` `shared_library_extension` signature is now `pub fn shared_library_extension(target: &TargetTriple) -> &'static str` (was: param-less `const fn` using host `#[cfg]`). Zero `#[cfg(target_os = ...)]` branches remain in this function.
  - [ ] `versioned_module_name` signature updated to accept/thread `&TargetTriple`; all call sites updated (`lsp_find_references` shows 0 stale callers).
  - [ ] Unit test asserts target-driven behavior: on a Linux host, `shared_library_extension(&TargetTriple::windows_msvc_x86_64()) == "dll"` AND `shared_library_extension(&TargetTriple::linux_x86_64()) == "so"` (proves target-driven, not host-driven).
  - [ ] Integration test `real_so_hot_reload_swaps_implementation` PASSES on Linux.
  - [ ] Copy-before-load: loader copies source `.so` to `<temp>/<uuid>.so` before opening; original file can be rewritten while old version still loaded.
  - [ ] No `dlopen` / `dlsym` raw libc calls anywhere (`grep -rn 'libc::dl' src/` returns 0).
  - [ ] `cargo test --features integration hot_reload` PASSES on Linux.

  **QA Scenarios**:
  ```
  Scenario: Hot-swap .so produces new behavior
    Tool: Bash (integration test)
    Preconditions: Linux, LLVM 14, two .op fixtures producing different outputs.
    Steps:
      1. cargo test --features integration real_so_hot_reload_swaps_implementation -- --nocapture 2>&1 | tee /tmp/swap.log
      2. grep -E 'v1 output|v2 output|SWAP OK' /tmp/swap.log
    Expected Result: Log shows v1 output before swap, v2 output after; SWAP OK marker present.
    Evidence: .sisyphus/evidence/task-23-so-swap.log

  Scenario: Rewriting source .so while loaded is safe
    Tool: Bash
    Preconditions: Hot-reload test scaffolding available.
    Steps:
      1. # Start test harness, load v1.so
      2. # Overwrite v1.so with garbage bytes while still loaded
      3. # Call exported fn — should still return v1 result (temp copy is load source)
      4. # Unload; temp copy cleaned up
    Expected Result: No segfault; v1 result returned; temp directory empty after test.
    Evidence: .sisyphus/evidence/task-23-copy-before-load.log
  ```

  **Commit**: YES (RGR: 3 commits) — `feat(hot_reload): real FsModuleLoader with libloading`

- [ ] 24. **Windows `.dll` hot reload — `LoadLibraryW` via libloading**

  **What to do**:
  - libloading (Task 23 dependency) already bridges to `LoadLibraryW`/`GetProcAddress`/`FreeLibrary` on Windows — no new crate needed.
  - Add Windows-specific logic in `FsModuleLoader`:
    - Produce `.dll` (not `.so`) when target is Windows — codegen needs to emit DLLs with proper exports (`dllexport` attribute on hot-reloadable functions).
    - Update `shared_library_extension(target)` to return `"dll"` on Windows targets.
    - Compiler must emit DLLs as dynamic libraries, not executables — add `--crate-type cdylib` equivalent for Opalescent modules flagged for hot reload.
  - COPY-BEFORE-LOAD is especially critical on Windows: Windows file-locking is mandatory — you cannot overwrite a loaded DLL at all. Copy to `%TEMP%\opal-hotreload\<uuid>.dll` before `LoadLibraryW`.
  - On unload (`FreeLibrary`), delete the temp copy.
  - Integration test mirrors Task 23 but for `.dll` under wine (CI) and native Windows (local verification).
  - RGR: RED = integration test `windows_dll_hot_reload_swaps` compiles two Opalescent modules to `.dll`, cross-compiles via MinGW, loads v1 under wine, swaps v2, asserts new output. FAILS initially (loader doesn't know how to emit `.dll`, libloading not wired for Windows path). GREEN = implement. REFACTOR = unify temp-copy logic with Task 23.

  **Must NOT do**:
  - Do NOT use raw `LoadLibraryW` via `winapi` / `windows-sys` — libloading provides the abstraction; don't bypass it.
  - Do NOT assume `/tmp` exists on Windows — use `std::env::temp_dir()` which returns `%TEMP%` on Windows.
  - Do NOT forget `dllexport` attributes — without them, symbols are private to the DLL and `GetProcAddress` returns NULL.
  - Do NOT skip the copy-before-load step on Windows (more critical than Linux due to mandatory locking).
  - Do NOT test hot-reload under wine and declare it done — schedule native Windows verification as part of Task 26.

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: Windows DLL semantics (locking, exports, dllexport) are distinct from `.so` and require careful handling.
  - **Skills**: none.

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 4 (with Tasks 21, 22, 23, 25)
  - **Blocks**: Task 27 (integration test for hot reload on both platforms).
  - **Blocked By**: Task 14 (target threading), Task 23 (libloading integration).

  **References**:
  - Windows `LoadLibraryW`: `https://learn.microsoft.com/en-us/windows/win32/api/libloaderapi/nf-libloaderapi-loadlibraryw`.
  - libloading Windows path: `https://docs.rs/libloading/latest/libloading/os/windows/index.html`.
  - `dllexport` in LLVM IR: codegen must emit `@symbol = dllexport ...` for Windows targets.
  - `std::env::temp_dir()`: `https://doc.rust-lang.org/std/env/fn.temp_dir.html`.
  - Existing `src/hot_reload/` structure from research.

  **Acceptance Criteria**:
  - [ ] `shared_library_extension(x86_64-pc-windows-*) == "dll"`.
  - [ ] Codegen emits `dllexport` attribute on hot-reloadable functions for Windows targets.
  - [ ] `FsModuleLoader` on Windows: copy `.dll` to temp, `libloading::Library::new(temp_path)`.
  - [ ] Integration test `windows_dll_hot_reload_swaps` PASSES under wine (CI) and on native Windows (Task 26).
  - [ ] Temp copies cleaned up: no leftover `.dll` files in `%TEMP%\opal-hotreload\` after test run.
  - [ ] Source `.dll` can be overwritten while v1 is loaded (thanks to copy-before-load).

  **QA Scenarios**:
  ```
  Scenario: Windows .dll hot-swap under wine
    Tool: Bash (cross-compile + wine)
    Preconditions: Linux + MinGW + wine.
    Steps:
      1. cargo test --features "integration wine-tests" windows_dll_hot_reload_swaps -- --nocapture 2>&1 | tee /tmp/dll-swap.log
      2. grep -E 'dllexport|LoadLibrary|SWAP OK' /tmp/dll-swap.log
    Expected Result: Test PASSES; log shows v1 → v2 transition.
    Evidence: .sisyphus/evidence/task-24-dll-swap.log

  Scenario: Source .dll can be overwritten while loaded (Windows locking bypass via copy-before-load)
    Tool: Bash (wine reproduction)
    Preconditions: Cross-compiled test harness.
    Steps:
      1. # Test scaffold loads v1.dll (copy-before-load means %TEMP%\uuid.dll is actual handle)
      2. # Overwrite source v1.dll bytes on disk while held
      3. # Expected: no ERROR_SHARING_VIOLATION; source file writable; loaded fn still returns v1 result
    Expected Result: No locking errors; test passes.
    Evidence: .sisyphus/evidence/task-24-dll-copy-before-load.log
  ```

  **Commit**: YES (RGR: 3 commits) — `feat(hot_reload): Windows .dll LoadLibraryW + copy-before-load`

- [ ] 25. **`SetConsoleOutputCP(CP_UTF8)` on Windows runtime init — fix mojibake in stdout**

  **What to do**:
  - Problem: Windows consoles default to OEM code page (e.g., CP437, CP1252) — any non-ASCII UTF-8 output from Opalescent programs prints as mojibake (`é` → `Ã©`).
  - Fix: call `SetConsoleOutputCP(CP_UTF8 /* 65001 */)` at runtime entry (early in the Opalescent runtime init path).
   - Location: in `runtime/opal_runtime.c` (or the per-TU equivalent after Task 18 restructure) — add an `opal_runtime_init()` function called before `main`. Use the `OPAL_WINDOWS` macro from Task 15's `opal_portability.h` (NOT raw `_WIN32`): `#include "opal_portability.h"` then `#if OPAL_WINDOWS`.
  - Also set input: `SetConsoleCP(CP_UTF8)` so stdin accepts UTF-8 from e.g. `chcp 65001` terminals.
  - Linker: link `kernel32.lib` (already present by default with MSVC/MinGW).
  - RGR: RED = integration test `windows_stdout_prints_utf8_correctly` — Opalescent program prints "héllo wörld" → wine run → captured stdout compared byte-for-byte against UTF-8 expected. FAILS before fix (OEM reinterpretation). GREEN = add SetConsoleOutputCP. REFACTOR = extract `opal_runtime_init_platform()` as a clean hook for future per-platform init.

  **Must NOT do**:
   - Do NOT call `SetConsoleOutputCP` on non-Windows — it's a Win32 API, gate with `#if OPAL_WINDOWS` (from `opal_portability.h`; do NOT use raw `_WIN32`).
  - Do NOT assume the console always exists — for processes with detached stdout (redirected to file/pipe), `GetConsoleMode` returns 0 and `SetConsoleOutputCP` is a no-op (safe to call anyway; but don't error on failure).
  - Do NOT use `SetConsoleOutputCP(CP_UTF8)` before runtime init is otherwise set up — must be early in `main` prologue, but after basic setup.
  - Do NOT convert strings to/from UTF-16 manually — just set code page, and MSVC `printf`/`fputs` handle UTF-8 bytes correctly once CP is 65001.

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Single Win32 API call with obvious integration point; minimal complexity.
  - **Skills**: none.

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 4 (with Tasks 21, 22, 23, 24)
  - **Blocks**: None directly; Task 22/26 tests will benefit.
  - **Blocked By**: Task 15 (portability header), Task 18 (runtime restructure — needs to know where init hook lives).

  **References**:
  - Win32 `SetConsoleOutputCP`: `https://learn.microsoft.com/en-us/windows/console/setconsoleoutputcp`.
  - `CP_UTF8 = 65001`.
  - `runtime/opal_runtime.c` — init path (post-Task 18 restructure).
  - Existing Linux init: whatever sets up RNG, signal handlers, etc. — model the Windows branch after it.

  **Acceptance Criteria**:
  - [ ] `runtime/opal_runtime.c` (or per-TU equivalent) has `SetConsoleOutputCP(CP_UTF8)` + `SetConsoleCP(CP_UTF8)` in init.
   - [ ] Gate: `#if OPAL_WINDOWS` (from `opal_portability.h`) — Linux unchanged; no raw `_WIN32` used.
  - [ ] Integration test `windows_stdout_prints_utf8_correctly` PASSES under wine.
  - [ ] Piped/redirected stdout still works (output to file captures correct UTF-8 bytes).
  - [ ] Linux regression: existing UTF-8 handling on Linux unchanged; tests pass.

  **QA Scenarios**:
  ```
  Scenario: UTF-8 prints correctly on Windows console via wine
    Tool: Bash (cross-compile + wine + UTF-8 byte check)
    Preconditions: Linux + MinGW + wine.
    Steps:
      1. cat > /tmp/utf8.op <<'EOF'
         entry main = f(args: string[]): void =>
             print('héllo wörld — 日本語')
             return void
         EOF
      2. cargo run --release -- /tmp/utf8.op --target x86_64-pc-windows-gnu
      3. wine /tmp/utf8.exe > /tmp/utf8-out.txt
      4. xxd /tmp/utf8-out.txt | head -3  # verify UTF-8 byte sequences (0xc3 0xa9 for é, 0xe6 0x97 0xa5 for 日)
    Expected Result: File contains correct UTF-8 bytes; no CP1252/OEM mojibake.
    Evidence: .sisyphus/evidence/task-25-utf8-stdout.log

  Scenario: Redirected stdout preserves UTF-8 (no console mode dependency)
    Tool: Bash
    Preconditions: Same as above.
    Steps:
      1. wine /tmp/utf8.exe 2>/dev/null | iconv -f utf-8 -t utf-8 > /tmp/roundtrip.txt
      2. diff <(echo 'héllo wörld — 日本語') /tmp/roundtrip.txt
    Expected Result: diff empty (bytes match).
    Evidence: .sisyphus/evidence/task-25-utf8-redirected.log
  ```

  **Commit**: YES (RGR: 3 commits) — `fix(runtime): SetConsoleOutputCP(CP_UTF8) on Windows init`

- [ ] 26. **Enable Windows native test matrix — `windows-latest` runs full suite**

  **What to do**:
  - Extend `.github/workflows/ci.yml` job `windows-native` (already scaffolded in Task 7 as a build-only job) to run the **full** integration test matrix.
  - Install on `windows-latest`:
    - Rust 2024 toolchain via `dtolnay/rust-toolchain@stable`.
    - LLVM 14 via `KyleMayes/install-llvm-action@v2` with `version: "14"`.
    - Set `LLVM_SYS_140_PREFIX` env var to the LLVM install dir.
  - Run: `cargo test --features integration` (exercises all 4 test projects compiled by `opalescent` itself natively on Windows).
  - Run: `cargo test --features integration hot_reload` (native .dll hot reload — Task 24 verification).
  - Artifacts: upload `target/debug/opalescent.exe` (the compiler itself as a Windows binary — Deliverable D1 evidence).
  - RGR: RED = job currently compiles but doesn't run tests (Task 7 scaffold). GREEN = enable full test run; fix any Windows-native-only failures. REFACTOR = factor out shared setup steps between windows-native and cross-msvc jobs.

  **Must NOT do**:
  - Do NOT mark tests as `#[ignore]` on Windows to make the job green — that defeats the purpose. If a test legitimately cannot run on Windows (none are expected), file a tracking issue and gate with `#[cfg(not(windows))]` with a code comment explaining why.
  - Do NOT skip hot-reload tests on native Windows — Task 24 requires native verification.
  - Do NOT use `windows-2019` or `windows-2022` pinned — use `windows-latest` to stay current with MSBuild / VS SDK updates.
  - Do NOT rely on Wine as a substitute — native execution is the point.

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
    - Reason: CI job activation + failure triage when native Windows behavior diverges from Linux.
  - **Skills**: none.

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 4 (with Tasks 21, 22, 23, 24, 25) — but often the last to stabilize because it integrates everything.
  - **Blocks**: None (terminal task in Wave 4 for runtime validation).
  - **Blocked By**: Tasks 11, 15-20, 24 (runtime + MSVC linker + hot reload all need to be ready for native run to succeed).

  **References**:
  - GitHub Actions `KyleMayes/install-llvm-action@v2`: `https://github.com/KyleMayes/install-llvm-action`.
  - Task 7 CI skeleton (Windows build job) — extend.
  - `Swatinem/rust-cache` — cache target dir.
  - LLVM 14 Windows binaries: distributed via the action; no manual download.

  **Acceptance Criteria**:
  - [ ] `.github/workflows/ci.yml` `windows-native` job runs `cargo test --features integration`.
  - [ ] All 4 test projects PASS on native Windows.
  - [ ] Hot-reload native test PASSES (Task 24 deliverable confirmed on real Windows).
  - [ ] `opalescent.exe` uploaded as artifact (Deliverable D1 evidence).
  - [ ] Job enforced as required status check.
  - [ ] Job time ≤ 20 min (first run with LLVM install); ≤ 10 min cached.

  **QA Scenarios**:
  ```
  Scenario: Native Windows test matrix fully green
    Tool: Bash (gh CLI)
    Preconditions: PR with Task 26 open.
    Steps:
      1. gh run watch --exit-status --job windows-native
      2. gh run view <run-id> --log --job windows-native | grep -E 'test result:' | tail -5
    Expected Result: All test result lines show "ok. N passed; 0 failed".
    Evidence: .sisyphus/evidence/task-26-windows-native-green.log

  Scenario: opalescent.exe artifact is a valid PE32+ binary
    Tool: Bash (gh CLI + file)
    Preconditions: Windows-native CI run complete.
    Steps:
      1. gh run download <run-id> --name opalescent-windows -D /tmp/art
      2. file /tmp/art/opalescent.exe
    Expected Result: "PE32+ executable (console) x86-64, for MS Windows".
    Evidence: .sisyphus/evidence/task-26-compiler-exe.log
  ```

  **Commit**: YES — `ci: enable Windows native test matrix`

- [ ] 27. **Hot-reload integration test — cross-platform `.so` / `.dll` coverage**

  **What to do**:
  - Add end-to-end integration test `hot_reload_end_to_end_cross_platform` that runs on **every** CI platform: Linux (Task 23 .so), Windows native (Task 24 .dll), Wine (Task 24 .dll under wine).
  - Scenario: compile a small `.op` module (function returning an int), load it via the hot-reload loader, call the function, assert v1 return value, modify source to return different value, recompile, trigger hot-swap, call again, assert v2 return value.
  - Include ABI-compatibility path AND ABI-break path:
    - ABI-compat: body change only → hot-swap succeeds.
    - ABI-break: signature change → hot-swap refused, full-restart signal raised.
  - Use `#[cfg]` gating so the same test source runs under all configurations:
    - Linux: uses `.so`, no wine.
    - Windows native: uses `.dll` directly.
    - Linux + Wine: uses `.dll` cross-compiled, runs inner harness under wine.
  - Evidence: capture v1 output, v2 output, ABI-break rejection message from each platform.
  - RGR: RED = test doesn't exist; writing it forces clean design of the cross-platform loader API (Task 23/24 consumers). GREEN = test exists and passes on Linux first. REFACTOR = add the Windows native + wine variants as conditional compilation, ensuring all three paths exercise the same loader trait.

  **Must NOT do**:
  - Do NOT duplicate the test source across platforms — use `#[cfg]` on a single test function.
  - Do NOT assume test ordering — tests must be independent (each creates its own temp dir, cleans up).
  - Do NOT rely on timing (sleeps) for swap synchronization — use the loader's explicit "swap complete" signal.

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
    - Reason: Test design that exercises multi-platform abstractions; moderate scope, high signal value.
  - **Skills**: none.

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 4 (with Tasks 21-26)
  - **Blocks**: None (terminal test).
  - **Blocked By**: Task 23 (real .so loader), Task 24 (Windows .dll loader).

  **References**:
  - Task 23 `FsModuleLoader` public API.
  - Task 24 Windows .dll path + `dllexport` codegen.
  - Existing ABI-guard design from hot_reload/ research — preserve it.
  - Rust `#[cfg]` target matching: `https://doc.rust-lang.org/reference/conditional-compilation.html`.

  **Acceptance Criteria**:
  - [ ] Single test source `src/hot_reload/tests/cross_platform.rs` (or equivalent) — gated with `#[cfg]` for 3 configurations.
  - [ ] Passes on: Linux native, Windows native (Task 26 job), Wine (Task 22 job).
  - [ ] Covers both ABI-compat path (swap OK) and ABI-break path (swap refused + restart signal).
  - [ ] Evidence files captured per-platform in `.sisyphus/evidence/task-27-<platform>.log`.
  - [ ] Zero flaky failures in 10 consecutive CI runs (verify by inspecting last 10 runs pre-merge).

  **QA Scenarios**:
  ```
  Scenario: End-to-end hot reload on Linux
    Tool: Bash
    Preconditions: Linux, LLVM 14, Task 23 complete.
    Steps:
      1. cargo test --features "integration" hot_reload_end_to_end_cross_platform -- --nocapture 2>&1 | tee /tmp/hr-linux.log
      2. grep -E 'v1 result|v2 result|ABI break detected' /tmp/hr-linux.log
    Expected Result: All three markers present in correct order; test PASSES.
    Evidence: .sisyphus/evidence/task-27-linux.log

  Scenario: End-to-end hot reload under Wine
    Tool: Bash (wine)
    Preconditions: Linux + MinGW + wine + Task 24 complete.
    Steps:
      1. cargo test --features "integration wine-tests" hot_reload_end_to_end_cross_platform -- --nocapture 2>&1 | tee /tmp/hr-wine.log
    Expected Result: Test PASSES under wine (harness orchestrates wine child process).
    Evidence: .sisyphus/evidence/task-27-wine.log

  Scenario: End-to-end hot reload on native Windows (CI only — captured in Task 26 run)
    Tool: CI log inspection
    Preconditions: windows-native CI job ran.
    Steps:
      1. gh run view <run-id> --job windows-native --log | grep hot_reload_end_to_end
    Expected Result: Test PASSES on Windows native.
    Evidence: .sisyphus/evidence/task-27-windows-native.log
  ```

  **Commit**: YES — `test(hot_reload): integration test for both platforms`

- [ ] 28. **README: Windows build instructions + cross-compile section**

  **What to do**:
  - Add a `## Windows` section to `README.md` (or a new `docs/windows.md` linked from README) covering:
    1. **Native Windows build** — Rust install via rustup-init.exe, LLVM 14 install (chocolatey `choco install llvm --version=14.0.6` OR direct MSI from LLVM releases), `LLVM_SYS_140_PREFIX` env var, `cargo build --release`.
    2. **Cross-compile Linux → Windows (MSVC)** — xwin install, env setup, `cargo run -- file.op --target x86_64-pc-windows-msvc`.
    3. **Cross-compile Linux → Windows (MinGW)** — `apt install gcc-mingw-w64-x86-64`, `cargo run -- file.op --target x86_64-pc-windows-gnu`.
    4. **Running Windows .exe on Linux (Wine)** — `apt install wine64`, `wine program.exe`.
    5. **Supported target triples** — list both `x86_64-pc-windows-msvc` and `x86_64-pc-windows-gnu`; note aarch64-windows is out of scope.
    6. **Hot reload on Windows** — note .dll extension, copy-before-load behavior, `dllexport` requirement.
  - Update README "Escape Hatches" table to note Windows `SetConsoleOutputCP(CP_UTF8)` as a Windows-specific runtime init (Task 25).
  - Update README "Build Targets" section to list new triples and their library extensions.
  - RGR: not strictly applicable (docs). Lightweight verification: run `opal run test-projects/hello-world/src/main.op --target x86_64-pc-windows-gnu` exactly as README instructs on a clean Linux VM; confirm success.

  **Must NOT do**:
  - Do NOT publish undocumented flags or features — every CLI flag introduced (esp. `--target`) must appear in README.
  - Do NOT write aspirational docs — if code signing is out of scope (per Task 0 decision), don't mention it.
  - Do NOT recommend `zig cc` (banned).
  - Do NOT include outdated paths — verify every command actually works in a test run.

  **Recommended Agent Profile**:
  - **Category**: `writing`
    - Reason: Pure documentation task; needs clear, tested, copyable command blocks.
  - **Skills**: none.

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 4 (with Tasks 21-27, 29, 30)
  - **Blocks**: None.
  - **Blocked By**: Tasks 4, 11, 12 (CLI `--target` flag must be final; linker paths documented accurately).

  **References**:
  - Existing `README.md` structure — mirror the `## Installation` / `## Build Targets` tone.
  - LLVM 14 Windows installer: `https://github.com/llvm/llvm-project/releases/tag/llvmorg-14.0.6`.
  - Chocolatey LLVM package: `https://community.chocolatey.org/packages/llvm`.
  - xwin README: `https://github.com/Jake-Shadle/xwin`.
  - MinGW-w64 on Ubuntu: `apt install gcc-mingw-w64-x86-64`.

  **Acceptance Criteria**:
  - [ ] `README.md` has a `## Windows` section (or equivalent) with 5+ subsections covering all cases listed above.
  - [ ] All commands in the section copy-paste work on a fresh Linux + fresh Windows VM (tested).
  - [ ] `opal help` output mentioning `--target` is consistent with README docs (Task 4 dependency).
  - [ ] Build Targets table updated with both Windows triples.
  - [ ] Escape Hatches table mentions Windows console CP init.
  - [ ] No references to banned tools (`zig cc`).

  **QA Scenarios**:
  ```
  Scenario: README commands work verbatim on clean Linux VM
    Tool: Bash (or docker)
    Preconditions: Docker or VM with fresh Ubuntu 22.04.
    Steps:
      1. docker run --rm -it ubuntu:22.04 bash
      2. # Inside: follow README Windows cross-compile (MinGW) section verbatim
      3. apt update && apt install -y curl build-essential gcc-mingw-w64-x86-64 wine64 llvm-14-dev
      4. # install rust, clone repo, set LLVM_SYS_140_PREFIX, cargo build
      5. ./target/release/opalescent test-projects/hello-world/src/main.op --target x86_64-pc-windows-gnu
      6. wine test-projects/hello-world/target/program.exe
    Expected Result: "hello" prints under wine; no deviation from README-described behavior.
    Evidence: .sisyphus/evidence/task-28-readme-mingw-path.log

  Scenario: `opal help` matches README docs for --target
    Tool: Bash
    Preconditions: Latest build.
    Steps:
      1. ./target/release/opalescent help | grep -A2 'target'
      2. diff <(./target/release/opalescent help | grep -A5 target) <(grep -A5 '\-\-target' README.md)
    Expected Result: Help text matches README claims about supported triples.
    Evidence: .sisyphus/evidence/task-28-help-matches-readme.log
  ```

  **Commit**: YES — `docs(readme): Windows build instructions`

- [ ] 29. **PowerShell `check-line-count.ps1` — parity with shell script**

  **What to do**:
  - Current state (verified ground truth): `scripts/check-line-count.sh` EXISTS as a file, but `Makefile.toml` does NOT currently define any `[tasks.check-line-count]` task and nothing in `Makefile.toml` invokes `scripts/check-line-count.sh`. Task 29 therefore (a) ADDS the missing cargo-make task, (b) creates a PowerShell equivalent for Windows CI, and (c) wires host-conditional dispatch. It does NOT modify an existing task (because none exists).
  - Create `scripts/check-line-count.ps1` — PowerShell equivalent with identical semantics to `scripts/check-line-count.sh`:
    - Walk `src/**/*.rs`, count lines, flag any file over the cap (extract threshold from the existing `.sh` to keep parity).
    - Exit code 0 on pass, 1 on fail; print offending files.
  - ADD a new `[tasks.check-line-count]` task to `Makefile.toml` that dispatches by host OS (using cargo-make's built-in `condition.platforms` or `script_runner` with `@duckscript` host-detection):
    - Linux/macOS: invoke `scripts/check-line-count.sh`.
    - Windows: invoke `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check-line-count.ps1` (use `pwsh` if available, fall back to `powershell`).
  - Verify byte-for-byte consistent output on the same repo snapshot (same file set → identical pass/fail verdict + identical offending-file list).
  - RGR: RED = `cargo make check-line-count` on windows-latest fails because the cargo-make task doesn't exist yet. GREEN = add the task + PowerShell script + host dispatch. REFACTOR = if the shell logic is non-trivial, consider a Rust binary `tools/check-line-count` usable from both; keep scripts if they stay simple.

  **Must NOT do**:
  - Do NOT require git-bash or WSL on Windows CI — the whole point is native PowerShell.
  - Do NOT let the two scripts diverge silently — add a unit/integration check that both produce identical output on the same corpus.
  - Do NOT count generated files, target/, or vendored code.

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Small PowerShell port; clear parity check.
  - **Skills**: none.

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 4 (with Tasks 21-28, 30)
  - **Blocks**: None.
  - **Blocked By**: Task 6 (Makefile.toml gating).

  **References**:
  - Existing bash script: `scripts/check-line-count.sh` (verified to exist). `Makefile.toml` does NOT currently invoke it — Task 29 both ADDS the cargo-make task and ports the script.
  - PowerShell core: `https://learn.microsoft.com/en-us/powershell/` — PS 7+ is cross-platform; target `pwsh` but also test on Windows PowerShell 5.1.
  - `Makefile.toml` cargo-make task syntax: `script_runner = "@shell"` for conditional script selection by host.

  **Acceptance Criteria**:
  - [ ] `scripts/check-line-count.ps1` exists; runs on `powershell` (PS 5.1) and `pwsh` (PS 7+).
  - [ ] On the same repo snapshot, bash and PS scripts produce **identical** output (file list + exit code).
  - [ ] `Makefile.toml` has a NEW `[tasks.check-line-count]` task (did not exist before this plan) with host-conditional dispatch.
  - [ ] Windows CI `cargo make check-line-count` PASSES (no bash dependency).
  - [ ] Linux CI `cargo make check-line-count` still PASSES (invokes the existing `scripts/check-line-count.sh` unchanged).

  **QA Scenarios**:
  ```
  Scenario: PS and bash scripts produce identical verdicts
    Tool: Bash (needs both bash and pwsh installed on test host)
    Preconditions: Linux with pwsh installed (`apt install powershell`).
    Steps:
      1. bash scripts/check-line-count.sh > /tmp/sh-out.txt 2>&1; echo "exit=$?" >> /tmp/sh-out.txt
      2. pwsh scripts/check-line-count.ps1 > /tmp/ps-out.txt 2>&1; echo "exit=$?" >> /tmp/ps-out.txt
      3. diff /tmp/sh-out.txt /tmp/ps-out.txt
    Expected Result: diff is empty; identical exit codes.
    Evidence: .sisyphus/evidence/task-29-script-parity.log

  Scenario: Windows CI runs PS script successfully
    Tool: gh CLI
    Preconditions: CI run on windows-latest.
    Steps:
      1. gh run view <run-id> --job windows-native --log | grep 'check-line-count'
    Expected Result: Task runs, exits 0.
    Evidence: .sisyphus/evidence/task-29-windows-ci-ps.log
  ```

  **Commit**: YES — `chore(scripts): PowerShell check-line-count variant`

- [ ] 30. **`opal.toml` docs — `x86_64-pc-windows-msvc` / `x86_64-pc-windows-gnu` in `[build].targets`**

  **What to do**:
  - Update README section "Project Configuration (`opal.toml`)" → "Supported target triples" table to include Windows entries with both toolchain suffixes:

    | Triple                       | Platform             | Linker          |
    |------------------------------|----------------------|-----------------|
    | `x86_64-pc-windows-msvc`     | Windows (MSVC CRT)   | `link.exe`      |
    | `x86_64-pc-windows-gnu`      | Windows (MinGW CRT)  | `x86_64-w64-mingw32-gcc` |

  - Update the example `[build]` block in README to show cross-target usage:
    ```toml
    [build]
    targets = ["x86_64-linux", "x86_64-pc-windows-msvc", "x86_64-pc-windows-gnu"]
    ```
  - If there's a parser in `src/compiler/config/opal_toml.rs` (or equivalent) that validates target strings, extend the allowlist to accept both Windows triples (and reject `aarch64-pc-windows-*` with a clear error per scope decision). Legacy `x86_64-windows` MUST continue to parse via Task 0.5's `parse_target_triple` legacy-alias branch — do NOT add it to any new allowlist, it must flow through the existing parser.
  - Add parser unit test: `opal_toml_accepts_windows_targets`, `opal_toml_accepts_legacy_x86_64_windows_with_warning` (asserts stderr contains "deprecated"), and `opal_toml_rejects_aarch64_windows`.
  - RGR: RED = unit test `opal_toml_accepts_windows_msvc` fails (parser allowlist is Linux/macOS only). GREEN = extend allowlist. REFACTOR = ensure the allowlist is the SAME set used by the `--target` CLI flag (Task 4) — single source of truth — AND that legacy `x86_64-windows` flows through `parse_target_triple` with deprecation warning, not through a second allowlist.

  **Must NOT do**:
  - Do NOT REMOVE support for bare `x86_64-windows` — it is a REQUIRED deprecated legacy alias that Task 0.5 mandates for backward compatibility with the existing `parse_legacy_2_segment_still_works` test and all current `opal.toml` files in the wild. It MUST continue to parse successfully, defaulting to MSVC (`TripleEnv::Msvc`), while emitting a ONE-TIME deprecation warning to stderr: `warning: target "x86_64-windows" is deprecated; use "x86_64-pc-windows-msvc" or "x86_64-pc-windows-gnu" explicitly`.
  - Do NOT allow `aarch64-pc-windows-msvc` or `aarch64-pc-windows-gnu` — out of scope per interview decision (x86_64 only).
  - Do NOT accept target triples that no supported linker can produce (consistency with Task 3 linker detection).
  - Do NOT document target triples inconsistently — if README says `x86_64-pc-windows-msvc`, don't have code messages saying `x86_64-windows-msvc`.
  - Do NOT leave bare `x86_64-windows` in NEW code, NEW docs, or NEW test cases beyond the explicit legacy-compat tests in Task 0.5 — all new references MUST use the 4-segment Rust form.

  **Recommended Agent Profile**:
  - **Category**: `writing`
    - Reason: Mostly doc + small parser allowlist update with unit test.
  - **Skills**: none.

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 4 (with Tasks 21-29)
  - **Blocks**: None.
  - **Blocked By**: Task 1 (TargetTriple type), Task 4 (`--target` CLI), Task 28 (README Windows section coordinates doc style).

  **References**:
  - `README.md` → "Supported target triples" table (currently lists Linux/macOS/windows-generic; must be updated to be specific).
  - `src/compiler/config/` — `opal.toml` parser (or equivalent location).
  - Task 1 `TargetTriple` FromStr impl — single-source parser.
  - Task 4 `--target` flag — uses same parser; docs must agree.

  **Acceptance Criteria**:
  - [ ] README target-triples table lists both `x86_64-pc-windows-msvc` and `x86_64-pc-windows-gnu` with correct linker note, AND documents `x86_64-windows` as a deprecated legacy alias (defaults to MSVC, emits warning).
  - [ ] `opal.toml` parser accepts both 4-segment triples AND the legacy 2-segment `x86_64-windows` (defaulting to MSVC with stderr deprecation warning).
  - [ ] `opal.toml` parser rejects `aarch64-pc-windows-*` with error message directing to x86_64 variant.
  - [ ] Unit tests `opal_toml_accepts_windows_msvc`, `opal_toml_accepts_windows_gnu`, `opal_toml_accepts_legacy_x86_64_windows_with_warning`, `opal_toml_rejects_aarch64_windows` all PASS.
  - [ ] `grep -rn 'x86_64-windows' src/` returns matches ONLY inside (a) `parse_target_triple` legacy-alias branch, (b) the deprecation warning string literal, and (c) the legacy-compat unit test in `src/build_system/tests.rs`. All other source references MUST use the 4-segment form.

  **QA Scenarios**:
  ```
  Scenario: opal.toml with Windows targets builds successfully
    Tool: Bash
    Preconditions: Tasks 1-27 complete.
    Steps:
      1. cat > /tmp/test-proj/opal.toml <<'EOF'
         name = "test"
         version = "0.1.0"
         [build]
         targets = ["x86_64-linux", "x86_64-pc-windows-gnu"]
         EOF
      2. mkdir -p /tmp/test-proj/src
      3. cp test-projects/hello-world/src/main.op /tmp/test-proj/src/
      4. cd /tmp/test-proj && opal build
      5. ls target/
    Expected Result: Two artifacts — `program` (Linux ELF) and `program.exe` (Windows PE32+).
    Evidence: .sisyphus/evidence/task-30-multi-target-build.log

  Scenario: aarch64-pc-windows-msvc rejected with clear error
    Tool: Bash
    Preconditions: Task 30 complete.
    Steps:
      1. cat > /tmp/bad-proj/opal.toml <<'EOF'
         name = "bad"
         version = "0.1.0"
         [build]
         targets = ["aarch64-pc-windows-msvc"]
         EOF
      2. cd /tmp/bad-proj && opal build 2>&1 | tee /tmp/bad-out.log
      3. grep -qE 'aarch64.*out of scope|not supported' /tmp/bad-out.log
    Expected Result: Non-zero exit; clear error citing scope decision; suggestion to use `x86_64-pc-windows-msvc`.
    Evidence: .sisyphus/evidence/task-30-aarch64-rejected.log
  ```

  **Commit**: YES — `docs(targets): opal.toml x86_64-windows documentation`

---

## Final Verification Wave (MANDATORY — after ALL implementation tasks)

> 5 review agents run in PARALLEL. ALL must APPROVE. Present consolidated results to user and get explicit "okay" before completing. Do NOT auto-proceed. Never mark F1-F5 checked before user's okay.

- [ ] F1. **Plan Compliance Audit** — `oracle`

  Read `.sisyphus/plans/windows-support.md` end-to-end. For each "Must Have" (D1-D10): verify implementation exists (read file, run cargo command, wine invocation, cl.exe invocation). For each "Must NOT Have": search codebase — reject with file:line if found (e.g. grep for `zig cc`, `signtool`, `aarch64-pc-windows`, `#[ignore]` added in this diff, `as &str` instead of `&TargetTriple`). Check `.sisyphus/evidence/` for all task evidence files. Compare D1-D10 against plan.

  **Output**: `Must Have [N/10] | Must NOT Have [N/22 clean] | Tasks [N/30] | Evidence [N files] | VERDICT: APPROVE/REJECT`

  **QA Scenarios**:
  ```
  Scenario: Must-Have D1-D10 all implemented
    Tool: Bash
    Preconditions: Tasks 0-30 complete on a disposable branch.
    Steps:
      1. For each of D1..D10 from plan "Must Have" list, run the associated verification command (grep/cargo/wine/file) as specified in each task's Acceptance Criteria.
      2. Record PASS/FAIL per D-item to `.sisyphus/evidence/final-qa/F1-must-have.log`.
      3. Assert: `grep -c "PASS" .sisyphus/evidence/final-qa/F1-must-have.log` equals 10.
    Expected Result: 10 PASS, 0 FAIL.
    Failure Indicators: Any D-item FAIL; missing evidence reference.
    Evidence: .sisyphus/evidence/final-qa/F1-must-have.log

  Scenario: Must-NOT-Have sweep finds zero forbidden patterns
    Tool: Bash
    Preconditions: Tasks 0-30 complete.
    Steps:
      1. Run the forbidden-pattern sweep: `bash scripts/f1-must-not-sweep.sh 2>&1 | tee .sisyphus/evidence/final-qa/F1-must-not.log` (script created by F1 agent; contains the 22 greps below).
      2. The script asserts (each must print 0):
         - `grep -rn "zig cc" src/ runtime/ scripts/ Cargo.toml Makefile.toml .github/workflows/`
         - `grep -rn "signtool" src/ runtime/ scripts/ .github/workflows/`
         - `grep -rn "aarch64-pc-windows" src/`
         - `grep -rn "xwin-run" src/ scripts/ .github/workflows/`
         - `grep -rn "wine cl.exe" src/ scripts/ .github/workflows/`
         - Plus the remaining Must-NOT patterns from the plan (full list at .sisyphus/plans/windows-support.md "Must NOT Have" section).
      3. Assert sweep script exit code is 0.
    Expected Result: All 22 greps return 0 matches; script exits 0.
    Failure Indicators: Any grep returns ≥1; print the offending `file:line` to the log.
    Evidence: .sisyphus/evidence/final-qa/F1-must-not.log

  Scenario: Evidence index completeness
    Tool: Bash
    Preconditions: Tasks 0-30 executed; `.sisyphus/evidence/` populated.
    Steps:
      1. `find .sisyphus/evidence -type f -name "task-*" | sort > .sisyphus/evidence/final-qa/F1-evidence-index.txt`
      2. For each Task N in plan, assert at least one file matches pattern `task-N-*` in the index.
      3. Count missing tasks: `missing=$(bash scripts/f1-evidence-check.sh); echo "missing=$missing"; [ "$missing" -eq 0 ]`
    Expected Result: Zero tasks missing evidence.
    Evidence: .sisyphus/evidence/final-qa/F1-evidence-index.txt
  ```

- [ ] F2. **Code Quality Review** — `unspecified-high`

  Run `cargo +stable fmt --all -- --check`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test --all-features` on Linux. Run `cargo clippy` with `--target x86_64-pc-windows-msvc`. Review all changed files for: `as any`/`.unwrap()` in non-test code, empty `catch`/`_ =>`, `println!` in prod paths, commented-out code, unused imports, dead `cfg` branches. Check AI slop: excessive comments, over-abstraction, generic names (data/result/item/temp), premature trait extraction.

  **Output**: `Fmt [PASS/FAIL] | Clippy Linux [PASS/FAIL] | Clippy Windows [PASS/FAIL] | Tests [N pass/N fail] | Files [N clean/N issues] | VERDICT`

  **QA Scenarios**:
  ```
  Scenario: Formatting + Linux clippy + tests all green
    Tool: Bash
    Preconditions: Tasks 0-30 complete; Linux host with full toolchain.
    Steps:
      1. `cargo +stable fmt --all -- --check 2>&1 | tee .sisyphus/evidence/final-qa/F2-fmt.log`
      2. `cargo clippy --all-targets --all-features -- -D warnings 2>&1 | tee .sisyphus/evidence/final-qa/F2-clippy-linux.log`
      3. `cargo test --all-features 2>&1 | tee .sisyphus/evidence/final-qa/F2-tests.log`
      4. Assert: fmt exit 0, clippy exit 0, tests show `test result: ok. N passed; 0 failed`.
    Expected Result: All three commands exit 0; tests show 0 failures.
    Failure Indicators: Any non-zero exit; any `warning:` line in clippy log; any `FAILED` in tests log.
    Evidence: .sisyphus/evidence/final-qa/F2-fmt.log, F2-clippy-linux.log, F2-tests.log

  Scenario: Windows-target clippy passes from Linux host
    Tool: Bash
    Preconditions: `rustup target add x86_64-pc-windows-msvc` complete; `XWIN_CACHE` populated.
    Steps:
      1. `cargo clippy --all-targets --all-features --target x86_64-pc-windows-msvc -- -D warnings 2>&1 | tee .sisyphus/evidence/final-qa/F2-clippy-windows.log`
      2. Assert exit 0; assert log has no `warning:` lines.
    Expected Result: Exit 0, zero warnings.
    Evidence: .sisyphus/evidence/final-qa/F2-clippy-windows.log

  Scenario: AI-slop and anti-pattern scan on changed files
    Tool: Bash
    Preconditions: Tasks 0-30 complete on disposable branch.
    Steps:
      1. `git diff --name-only $(git merge-base HEAD main)..HEAD -- '*.rs' > .sisyphus/evidence/final-qa/F2-changed-rs.txt`
      2. For each file in that list, run `grep -nE "(^[[:space:]]*//[[:space:]]*TODO|\\.unwrap\\(\\)|as any|@ts-ignore|console\\.log|println!\\(\"DEBUG|let (data|result|item|temp|foo|bar)[[:space:]]*[:=])" "$f"` and aggregate into `F2-slop.log`.
      3. Exclude lines in `#[cfg(test)]` or `mod tests` blocks (use `ast-grep` or comment anchors).
      4. Assert aggregate line count is 0 (or all matches are human-reviewed and whitelisted in `F2-slop-whitelist.txt`).
    Expected Result: Zero non-whitelisted matches.
    Evidence: .sisyphus/evidence/final-qa/F2-slop.log, F2-changed-rs.txt
  ```

- [ ] F3. **Real Manual QA** — `unspecified-high`

  Start from clean state (`cargo clean`). Execute EVERY QA scenario from EVERY task using the specified tool — follow exact steps, capture evidence. Specifically verify:
  1. Linux native: `cargo build --release && ./target/release/opalescent run test-projects/hello-world/src/main.op`
  2. Linux → Windows cross: `cargo build --release --target x86_64-pc-windows-msvc && wine ./target/x86_64-pc-windows-msvc/release/opalescent.exe run test-projects/hello-world/src/main.op`
  3. Hot-reload: run `test-projects/hot-reload-demo/` end-to-end on Linux (must observe live module swap)
  4. Negative: `opalescent run foo.op --target invalid-triple` → exit 1 with clear error
  5. Regression: full `cargo test --all-features` on Linux

  Save to `.sisyphus/evidence/final-qa/`.

  **Output**: `Scenarios [N/N pass] | Integration [N/N] | Edge Cases [N tested] | Regression [PASS/FAIL] | VERDICT`

  **QA Scenarios**:
  ```
  Scenario: Linux native build + hello-world happy path
    Tool: Bash
    Preconditions: Clean state (`cargo clean`); Linux host.
    Steps:
      1. `cargo build --release 2>&1 | tee .sisyphus/evidence/final-qa/F3-linux-build.log`
      2. `./target/release/opalescent run test-projects/hello-world/src/main.op 2>&1 | tee .sisyphus/evidence/final-qa/F3-linux-hello.log`
      3. Assert log contains exactly `Hello world` (or the project's documented output).
    Expected Result: Build exit 0; runtime prints expected output.
    Evidence: .sisyphus/evidence/final-qa/F3-linux-build.log, F3-linux-hello.log

  Scenario: Linux → Windows cross-compile + wine execution
    Tool: Bash
    Preconditions: `XWIN_CACHE` set; `wine` installed; `rustup target add x86_64-pc-windows-msvc` complete.
    Steps:
      1. `cargo build --release --target x86_64-pc-windows-msvc 2>&1 | tee .sisyphus/evidence/final-qa/F3-cross-build.log`
      2. Assert `file target/x86_64-pc-windows-msvc/release/opalescent.exe` output contains `PE32+`.
      3. `wine ./target/x86_64-pc-windows-msvc/release/opalescent.exe run test-projects/hello-world/src/main.op 2>&1 | tee .sisyphus/evidence/final-qa/F3-cross-hello.log`
      4. Assert log contains expected output.
    Expected Result: PE32+ binary produced; wine execution prints expected output.
    Failure Indicators: Build fails; `file` does not report PE32+; wine returns non-zero or garbled output.
    Evidence: .sisyphus/evidence/final-qa/F3-cross-build.log, F3-cross-hello.log

  Scenario: Hot-reload module swap live (fully scripted — zero manual interaction)
    Tool: Bash
    Preconditions: Task 27's hot-reload-demo project built on Linux; `run.sh` writes stdout to a log the scenario can tail; demo source file `src/greeting.op` starts with `"hello"` literal.
    Steps:
      1. Setup: `cd test-projects/hot-reload-demo && cp src/greeting.op .sisyphus/evidence/final-qa/F3-hotreload-greeting.orig` (backup for restore).
      2. Start host in background redirecting stdout+stderr: `./run.sh > .sisyphus/evidence/final-qa/F3-hotreload.log 2>&1 & echo $! > .sisyphus/evidence/final-qa/F3-hotreload.pid`
      3. Capture initial PID: `HOST_PID=$(cat .sisyphus/evidence/final-qa/F3-hotreload.pid)` and verify alive: `kill -0 $HOST_PID`.
      4. Poll for v1 output (max 10s): `for i in $(seq 1 20); do grep -q "v1: hello" .sisyphus/evidence/final-qa/F3-hotreload.log && break; sleep 0.5; done` then assert `grep -q "v1: hello" .sisyphus/evidence/final-qa/F3-hotreload.log`.
      5. Programmatic file overwrite via heredoc (no editor, no tmux): `cat > src/greeting.op <<'OP'` … `OP` — write the complete new source with `"hola"` replacing `"hello"` (exact contents specified in Task 27 acceptance criteria).
      6. Trigger filesystem-notify settle: `sync && sleep 0.2`.
      7. Poll for v2 output (max 5s): `for i in $(seq 1 10); do grep -q "v2: hola" .sisyphus/evidence/final-qa/F3-hotreload.log && break; sleep 0.5; done` then assert `grep -q "v2: hola" .sisyphus/evidence/final-qa/F3-hotreload.log`.
      8. Verify PID unchanged (hot-reload, not restart): `STILL_PID=$(pgrep -f hot-reload-demo | head -1); [ "$STILL_PID" = "$HOST_PID" ] || { echo "PID changed: was $HOST_PID now $STILL_PID" >&2; exit 1; }` — capture to `.sisyphus/evidence/final-qa/F3-hotreload-pid.log`.
      9. Cleanup: `kill $HOST_PID || true; wait $HOST_PID 2>/dev/null || true; cp .sisyphus/evidence/final-qa/F3-hotreload-greeting.orig src/greeting.op` (restore original source so repeated runs are idempotent).
    Expected Result: Log contains both `v1: hello` (before swap) and `v2: hola` (after swap within 5s); PID before and after swap are identical (proving hot-reload, not process restart); source file restored to original state.
    Failure Indicators: `v1: hello` never appears (host failed to start); `v2: hola` never appears within 5s after file overwrite (hot-reload broken); PID differs between step 3 and step 8 (process restarted, not hot-reloaded); source file not restored after run.
    Evidence: .sisyphus/evidence/final-qa/F3-hotreload.log, .sisyphus/evidence/final-qa/F3-hotreload.pid, .sisyphus/evidence/final-qa/F3-hotreload-pid.log

  Scenario: Negative — invalid target triple rejected
    Tool: Bash
    Preconditions: Task 4 complete.
    Steps:
      1. `./target/release/opalescent run test-projects/hello-world/src/main.op --target aarch64-pc-windows-msvc; echo "exit=$?"` | tee .sisyphus/evidence/final-qa/F3-negative.log
      2. Assert exit code non-zero AND log contains substring `out of scope` OR `InvalidTarget`.
    Expected Result: Non-zero exit with clear diagnostic.
    Evidence: .sisyphus/evidence/final-qa/F3-negative.log

  Scenario: Regression — full Linux test suite green
    Tool: Bash
    Preconditions: All implementation tasks complete.
    Steps:
      1. `cargo test --all-features 2>&1 | tee .sisyphus/evidence/final-qa/F3-regression.log`
      2. Assert final line matches `test result: ok\\. [0-9]+ passed; 0 failed`.
    Expected Result: 0 failures.
    Evidence: .sisyphus/evidence/final-qa/F3-regression.log
  ```

- [ ] F4. **Scope Fidelity Check** — `deep`

  For each task 0-30: read "What to do", read actual `git log --follow` + `git diff` on relevant paths. Verify 1:1 — everything in spec was built (no missing), nothing beyond spec was built (no creep). Check "Must NOT do" compliance per task and plan-wide. Detect cross-task contamination: Task N touching Task M's files. Flag unaccounted changes. Special checks: zero new files outside `src/`, `runtime/`, `tests/`, `test-projects/hot-reload-demo/`, `.github/workflows/`, `scripts/`; no `Cargo.toml` dependencies added beyond portability needs.

  **Output**: `Tasks [N/31 compliant] | Contamination [CLEAN/N issues] | Unaccounted [CLEAN/N files] | Deps added [N, listed] | VERDICT`

  **QA Scenarios**:
  ```
  Scenario: Changed-file set is bounded by whitelisted directories
    Tool: Bash
    Preconditions: All tasks merged on disposable branch; `main` as base.
    Steps:
      1. `git diff --name-only $(git merge-base HEAD main)..HEAD > .sisyphus/evidence/final-qa/F4-changed.txt`
      2. `grep -vE '^(src/|runtime/|tests/|test-projects/hot-reload-demo/|\\.github/workflows/|scripts/|Cargo\\.toml$|Cargo\\.lock$|Makefile\\.toml$|README\\.md$|\\.sisyphus/)' .sisyphus/evidence/final-qa/F4-changed.txt > .sisyphus/evidence/final-qa/F4-unaccounted.txt || true`
      3. Assert `.sisyphus/evidence/final-qa/F4-unaccounted.txt` is empty (`[ ! -s file ]`).
    Expected Result: Zero unaccounted-for changed paths.
    Failure Indicators: Any path outside whitelist → REJECT with file list.
    Evidence: .sisyphus/evidence/final-qa/F4-changed.txt, F4-unaccounted.txt

  Scenario: Cargo dependency additions are audited and justified
    Tool: Bash
    Preconditions: All tasks merged.
    Steps:
      1. `git diff $(git merge-base HEAD main)..HEAD -- Cargo.toml | grep -E '^\\+[^+]' | grep -E '^\\+[a-z_-]+\\s*=' > .sisyphus/evidence/final-qa/F4-deps-added.txt || true`
      2. For each added dep, verify it appears in at least one task's "References" or "What to do" section of the plan (`grep -l "$dep" .sisyphus/plans/windows-support.md`).
      3. Assert every added dep has a plan citation.
    Expected Result: Every added dep is plan-justified.
    Evidence: .sisyphus/evidence/final-qa/F4-deps-added.txt

  Scenario: Per-task 1:1 scope adherence (spot-audit 5 random tasks)
    Tool: Bash
    Preconditions: Tasks 0-30 complete.
    Steps:
      1. Pick 5 tasks at random (e.g., `shuf -i 0-30 -n 5` → e.g. 3,8,14,21,27).
      2. For each, capture the per-task diff: `git log --all --oneline --grep "Task $N" | head -5 > F4-task-$N.log; git diff <commit>^..<commit> > F4-task-$N.diff`.
      3. Compare diff against plan's "What to do" bullets: assert each bullet has a corresponding diff hunk; assert no hunks touch unrelated files (cross-task contamination).
      4. Record PASS/FAIL per sampled task to `F4-scope.log`.
    Expected Result: 5/5 sampled tasks PASS; zero contamination.
    Evidence: .sisyphus/evidence/final-qa/F4-task-*.log, F4-task-*.diff, F4-scope.log
  ```

- [ ] F5. **Post-Implementation Momus Review** — `Momus - Plan Critic`

  Resubmit the now-IMPLEMENTED plan to Momus for a final post-implementation audit. Unlike the pre-execution review (which verified the plan was executable), this verifies the EXECUTED WORK matches the approved plan with zero drift.

  **Tool**: `task(subagent_type="Momus - Plan Critic", load_skills=[], prompt=".sisyphus/plans/windows-support.md", run_in_background=false)` — fresh session, NO session_id (session resume replays cached output).

  **Preconditions**: F1-F4 have all produced APPROVE verdicts AND their evidence files exist in `.sisyphus/evidence/`.

  **Steps**:
  1. Confirm F1-F4 APPROVE verdicts present in `.sisyphus/evidence/final-qa/` (if any is REJECT, do NOT run F5 yet — fix issues, re-run F1-F4 first).
  2. Append a "Post-Implementation Evidence Index" section to `.sisyphus/plans/windows-support.md` listing every evidence file under `.sisyphus/evidence/` with its generating task number (do NOT modify any other plan content).
  3. Invoke Momus with path-only prompt (above). Capture verdict to `.sisyphus/evidence/final-qa/F5-momus-verdict.md`.
  4. If Momus returns `OKAY`: verdict APPROVE.
  5. If Momus returns `REJECT`: list every issue in the verdict file; DO NOT auto-fix — escalate to user for triage (could be plan drift, scope gap, or legitimate new finding).

  **Expected Result**: Momus `OKAY` verdict captured in `.sisyphus/evidence/final-qa/F5-momus-verdict.md`.

  **Failure Indicators**: Momus `REJECT`; cached-replay suspected (verdict text identical to any prior round's `.sisyphus/evidence/` momus log — if so, re-invoke WITHOUT session_id).

  **Output**: `Momus Verdict [OKAY/REJECT] | Evidence Index [N files] | Drift Issues [N listed] | VERDICT`

  **QA Scenarios**:
  ```
  Scenario: F1-F4 precondition gate
    Tool: Bash
    Preconditions: F1-F4 task outputs saved to `.sisyphus/evidence/final-qa/`.
    Steps:
      1. For each of F1..F4, assert a verdict file exists: `for f in F1 F2 F3 F4; do test -f .sisyphus/evidence/final-qa/$f-verdict.md || { echo "MISSING $f"; exit 1; }; done`
      2. For each verdict file, assert content contains the literal string `VERDICT: APPROVE` (case-sensitive): `grep -L "VERDICT: APPROVE" .sisyphus/evidence/final-qa/F[1-4]-verdict.md` returns empty.
    Expected Result: All 4 verdict files exist and contain APPROVE.
    Failure Indicators: Any missing file or any verdict lacking APPROVE → F5 does NOT run, return REJECT with "preconditions-not-met".
    Evidence: stdout capture in .sisyphus/evidence/final-qa/F5-preconditions.log

  Scenario: Momus fresh-session verdict capture
    Tool: Bash (wrapping task() invocation via orchestrator)
    Preconditions: F5-preconditions passed; evidence index section appended to the plan file.
    Steps:
      1. Orchestrator invokes `task(subagent_type="Momus - Plan Critic", load_skills=[], prompt=".sisyphus/plans/windows-support.md", run_in_background=false)` — session_id MUST be omitted.
      2. Capture full Momus output to `.sisyphus/evidence/final-qa/F5-momus-verdict.md` (include the session_id returned in the task_metadata block).
      3. Assert captured text starts with `**[OKAY]**` OR `**[REJECT]**` (Momus's verdict convention).
      4. Cache-replay guard: diff the captured verdict against any prior `.sisyphus/evidence/*momus*.md` file. If byte-for-byte identical, REJECT this scenario and re-invoke fresh.
    Expected Result: Verdict file exists; first line is `**[OKAY]**`; no cache-replay detected.
    Failure Indicators: Verdict is `**[REJECT]**` (escalate to user per step 5 above); cache-replay diff matches (retry fresh).
    Evidence: .sisyphus/evidence/final-qa/F5-momus-verdict.md
  ```

  **Must NOT do**:
  - Do NOT resume a prior Momus session (confirmed cached-replay behavior — always fresh).
  - Do NOT wrap the prompt in explanations — path string ONLY.
  - Do NOT auto-fix REJECT findings without user triage.

---

## Commit Strategy

> One commit per RGR cycle (RED, GREEN, REFACTOR separately). Task boundaries align with commit groups. Use conventional commits.

- **0**: `chore(ci): de-risk cargo build on windows-latest` — .github/workflows/ci.yml (spike job), Cargo.toml (if LLVM config change needed)
- **1**: `refactor(targets): replace &str with TargetTriple in public API` — src/build_system/targets.rs, src/build_system/*, src/compiler.rs
- **2**: `feat(build_system): add object_file_extension + executable_filename` — src/build_system/targets.rs
- **3**: `feat(build_system): detect_preferred_linker enum` — src/build_system/linker.rs (new)
- **4**: `feat(cli): --target flag on build/run/check` — src/app.rs, src/cli.rs
- **5**: `build(llvm): switch to static LLVM on all platforms` — Cargo.toml
- **6**: `chore(cargo-make): gate Linux-only tasks` — Makefile.toml
- **7**: `ci: add GitHub Actions skeleton (3 jobs)` — .github/workflows/ci.yml
- **8**: `refactor(codegen): CodegenContext::for_triple` — src/codegen/*
- **9**: `feat(codegen): target-driven emit_object_file` — src/codegen/emit.rs
- **10**: `refactor(linker): LinkerCommand abstraction` — src/build_system/linker.rs
- **11**: `feat(linker): MSVC link.exe path` — src/build_system/linker.rs
- **12**: `feat(linker): MinGW gcc path` — src/build_system/linker.rs
- **13**: `fix(linker): host-gate -no-pie to Linux` — src/build_system/linker.rs
- **14**: `feat(compiler): thread target through compile_program/project` — src/compiler.rs
- **15**: `feat(runtime): opal_portability.h with MSVC shims` — runtime/opal_portability.h (new), all .c files include
- **16**: `fix(runtime): opal_rc offsetof() + _Static_assert` — runtime/opal_rc.c, runtime/opal_rc.h
- **17**: `feat(runtime): BCryptGenRandom on Windows` — runtime/opal_rng.c
- **18**: `refactor(runtime): per-platform aggregator for opal_runtime.c` — runtime/opal_runtime.c
- **19**: `feat(runtime): MSVC getline/strdup reimpl` — runtime/opal_io.c, runtime/opal_string.c
- **20**: `fix(runtime): PRId64/PRIu64 MSVC fallback` — runtime/opal_print.c
- **21**: `ci: xwin integration for cross-compile` — .github/workflows/ci.yml
- **22**: `ci: Wine job executing cross-compiled .exe` — .github/workflows/ci.yml
- **23**: `feat(hot_reload): real FsModuleLoader with libloading (Linux .so)` — src/hot_reload/loader.rs
- **24**: `feat(hot_reload): Windows .dll LoadLibraryW + copy-before-load` — src/hot_reload/loader.rs
- **25**: `fix(runtime): SetConsoleOutputCP(CP_UTF8) on Windows init` — runtime/opal_runtime.c
- **26**: `ci: enable Windows native test matrix` — .github/workflows/ci.yml
- **27**: `test(hot_reload): integration test for both platforms` — tests/hot_reload_integration.rs, test-projects/hot-reload-demo/
- **28**: `docs(readme): Windows build instructions` — README.md
- **29**: `chore(scripts): PowerShell check-line-count variant` — scripts/check-line-count.ps1
- **30**: `docs(targets): opal.toml x86_64-windows documentation` — README.md (targets table)

Pre-commit hook on every task: `cargo test --all-features` (Linux regression gate).

---

## Success Criteria

### Verification Commands

```bash
# Linux regression (MUST PASS)
cargo test --all-features
# Expected: "test result: ok. ... 0 failed"

# Linux native build
cargo build --release
# Expected: target/release/opalescent exists, is ELF 64-bit

# Linux → Windows cross
cargo build --release --target x86_64-pc-windows-msvc
# Expected: target/x86_64-pc-windows-msvc/release/opalescent.exe exists, is PE32+

# Linux → Windows cross runtime (via Wine)
wine target/x86_64-pc-windows-msvc/release/opalescent.exe run test-projects/hello-world/src/main.op
# Expected: "Hello world" on stdout, exit 0

# Windows native build (on windows-latest CI)
cargo build --release
# Expected: target\release\opalescent.exe exists

# Windows native runtime (on windows-latest CI)
target\release\opalescent.exe run test-projects\hello-world\src\main.op
# Expected: "Hello world" on stdout, exit 0

# CI green
gh run list --workflow=ci.yml --limit 1 --json conclusion --jq '.[0].conclusion'
# Expected: "success"

# Hot-reload integration (Linux)
cargo test --features integration hot_reload_demo_swaps_live_module
# Expected: test passes

# Runtime C portability (all three toolchains)
gcc -c runtime/opal_runtime.c -o /tmp/test.o                     # glibc host
x86_64-w64-mingw32-gcc -c runtime/opal_runtime.c -o /tmp/test.o  # MinGW cross
cl.exe /c runtime\opal_runtime.c                                  # MSVC on windows-latest
# Expected: all produce object files with 0 errors, 0 warnings

# Negative: invalid target
opalescent run foo.op --target invalid-triple
# Expected: exit 1, stderr contains "unknown target triple"
```

### Final Checklist

- [ ] All 10 deliverables (D1-D10) verified by Final Wave agents
- [ ] All 22 "Must NOT Have" guardrails absent (grep-verified)
- [ ] All 30 implementation tasks complete with evidence files
- [ ] CI green on main
- [ ] Linux tests 0 failed (regression gate)
- [ ] Windows tests 0 failed (new)
- [ ] Wine job 0 failed
- [ ] Momus approval: OKAY verdict received
- [ ] User explicit okay received after Final Wave presentation
