# File I/O Standard Library (path-object-centric) + Windows Program Build Docs

## TL;DR

> **Quick Summary**: Implement 38 file-I/O stdlib builtins + 3 nominal types + 20 error types for Opalescent, exposed via `import X from standard`, using TDD red-green-refactor with per-category batching. Add a "Building Opalescent Programs for Windows" subsection to README.md documenting the end-user cross-compile workflow.
>
> **Deliverables**:
> - 38 new stdlib functions (1 path constructor + 6 pure path helpers + 4 readers + 7 writers + 7 file-mgmt + 9 dir-ops + 4 permissions — absolute_path_sync + 2 atomic writes + 1 text-append + 1 nofollow-metadata + 2 nofollow-inspection included)
> - 3 new nominal types: `FilesystemPath`, `FileMetadata`, `FilePermissions`
> - 20 new error types (19 domain errors + `InvalidUtf8Error`)
> - `runtime/opal_fs.c` + full compiler wiring across 7 touchpoints per builtin
> - 3 at-scale test projects under `test-projects/` using only project-local files with RAII state restoration
> - Matching Rust integration tests under `tests/integration_e2e/fs_*.rs`
> - New README subsection "Building Opalescent Programs for Windows"
>
> **Estimated Effort**: XL (stdlib surface) + Quick (README)
> **Parallel Execution**: YES — 6 waves
> **Critical Path**: Pre-Flight → Types+Errors → Path helpers (infra bootstrap) → everything else in parallel → Test projects → README → Final Verification

---

## Context

### Original Request
User requested two deliverables in one plan: (1) Document how to build Opalescent programs for Windows in README.md (if not already present), and (2) Implement the path-object-centric file-I/O stdlib proposal using `Bytes` (not `uint8[]`), with TDD red-green-refactor, multiple at-scale test projects that manipulate only project-local files and restore state, with self-review at the end.

### Interview Summary

**Key Discussions**:
- README Windows compiler-build section already exists; only the end-user program-build workflow needs adding.
- Exposure via existing `import X from standard` (precedent: `bytes-hex-roundtrip`).
- Proposal code in `stdlib-proposals/file-io-surface/path-object-centric/proposal.md` is not valid Opalescent — language-spec and existing stdlib code are the authoritative references.
- TDD granularity: per-category batches for infrastructure (touchpoints 1–6), per-function red-green for registration + tests (touchpoints 7–8).
- Self-review (Momus-style) inline at end, twice: once on plan, once on implementation.

**Research Findings**:
- Language syntax: `let name = f(params): ReturnType [errors E1, E2] => body`; types in `.types.op` only; single-quoted strings; `guard/propagate` for errors; imports via `import X from standard`.
- `_sync` suffix has NO prior usage — we introduce it per proposal style rule 14.
- 8-touchpoint wiring pattern established by `bytes_*` builtins: `opal_fs.c` (new), `opal_runtime.h`, `compiler.rs RUNTIME_SOURCE`, `functions_stdlib.rs` (STDLIB_NAMES + declare_stdlib_function), `statements.rs` (known_runtime_return_type + known_guard_success_type), `fs_builtins.rs` (new), `module_resolver.rs` (standard-module exports), tests.
- Fallible ABI: `{ void* value; const char* error_cstr }` struct; codegen lowers guard/propagate generically over NULL error-pointer check.
- Test harness: `cargo test --features integration <name>`; helpers `prepare_dir`/`cleanup_dir` in `tests/integration_print.rs`; `compile_program(...)` + `Command::new(binary).output()`.

### Metis Review
**Identified Gaps (resolved via user Q&A)**:
- Path construction: Both `path_from` builtin AND `new FilesystemPath:` record constructor.
- Text encoding: UTF-8 strict with `InvalidUtf8Error`; byte-oriented escape hatches (`read_contents_sync` etc.) retained.
- Line endings: Split on `\n`, strip single trailing `\r` (CRLF+LF both round-trip cleanly).
- Write atomicity: Both flavors — naive (default, `write_*_sync`) + atomic (`write_*_atomic_sync`).
- Symlinks: Content ops follow; metadata/inspection have `_nofollow_sync` variants; pure path manipulation is lexical.
- Permissions: Abstract triple `{readable, writable, executable}` portable across Unix/Windows.
- Platform scope: Linux-only CI; Windows parity deferred to `.sisyphus/followups.md`.
- Pre-Flight Validation mandatory first task: verify `FilesystemPath[]` / `string[]` return types and scalar fallible-ABI work.

**Directives incorporated**:
- Group 38 functions into category batches; infrastructure touchpoints wired per-batch, TDD per-function for registration + behavior tests.
- RAII cleanup guards (Rust `Drop`) — not trailing cleanup calls.
- Evidence capture: stdout + exit code + pre/post-state sha256 manifests + state-diff (must be empty) + cleanup-marker timestamp under `.sisyphus/evidence/fs-<project>/<scenario>/`.
- No string-based path APIs. `FilesystemPath` is the only path type.
- No async, no streaming, no memory-mapped I/O. All I/O is sync, whole-file.
- No refactoring of existing `bytes_*` stdlib code.

---

## Work Objectives

### Core Objective
Deliver a complete, importable file-I/O standard library surface for Opalescent using the `FilesystemPath` object as the universal path type, with full `guard`/`propagate` error handling, byte + UTF-8-strict text variants, atomic-write options, symlink-follow control, and abstract portable permissions — validated by 3 real test projects whose state is guaranteed restored after every run. Simultaneously, document the Windows program-build workflow in README.md.

### Concrete Deliverables

**Stdlib code**:
- `runtime/opal_fs.c` — C implementations for 38 builtins
- `runtime/opal_runtime.h` — extended with prototypes
- `src/compiler.rs` — RUNTIME_SOURCE includes `opal_fs.c`
- `src/codegen/functions_stdlib.rs` — 38 entries in STDLIB_NAMES + match arms in declare_stdlib_function
- `src/codegen/statements.rs` — 38 entries in known_runtime_return_type + fallible ones in known_guard_success_type
- `src/type_system/checker/fs_builtins.rs` (new) — registers 3 nominal types + 20 error nominals + 38 builtin signatures
- `src/type_system/checker.rs` — calls `register_fs_builtins(...)`
- `src/type_system/module_resolver.rs` — adds 38 names to standard-module exports
- `stdlib/prelude.op` — doc-only signatures for 38 fns + 3 types

**Test projects** (under `test-projects/`):
- `fs-path-manipulation/` — pure CPU path helpers; deterministic stdout; no fs mutation
- `fs-markdown-roundtrip/` — reads `fixture.md`, appends marker, reads back, then restores original bytes via RAII
- `fs-directory-operations/` — creates/lists/copies/moves/deletes inside local `./sandbox/`, fully cleaned start+end

**Rust integration tests** (under `tests/integration_e2e/`):
- `fs_path_manipulation.rs`
- `fs_markdown_roundtrip.rs`
- `fs_directory_operations.rs`
- Each uses `StateGuard` struct with `Drop` to snapshot-and-restore project-local files.

**Documentation**:
- `README.md` — new "## Building Opalescent Programs for Windows" subsection under the existing Windows Build section, covering: `opal.toml [build].targets`, CLI invocation, xwin/clang-cl prerequisites on Linux hosts, expected artifact path `target/<triple>/program.exe`, and a note on stdlib platform-specific behavior with links to a "Windows behavior notes" bullet list.

**Tracking**:
- `.sisyphus/followups.md` — Windows CI parity for stdlib tests deferred item.

### Definition of Done
- [ ] `cargo build --release` succeeds.
- [ ] `cargo test` (unit tests) → all pass.
- [ ] `cargo test --features integration fs_` → all 3 fs integration tests pass.
- [ ] `cargo test --features integration bytes_hex_roundtrip_compiles_links_and_runs` → still passes (no regression).
- [ ] All 3 test projects: `pre-state.txt` sha256 manifest == `post-state.txt` (diff empty).
- [ ] `file test-projects/fs-markdown-roundtrip/fixture.md` → matches byte-for-byte original (restored).
- [ ] Readme grep: `grep -q 'Building Opalescent Programs for Windows' README.md`.
- [ ] Every fallible builtin has ≥1 happy-path test and ≥1 failure-path test asserting the specific error type.
- [ ] `.sisyphus/evidence/fs-*/` populated for every QA scenario.

### Must Have
- 38 functions, 3 nominal types, 20 error types — exact names per inventory below.
- `FilesystemPath` as the ONLY path type in any public signature (no raw `string` paths).
- UTF-8 strict for `_text_` variants; byte escape hatches for binary data.
- Atomic write variants for contents + text.
- `_nofollow_sync` variants for metadata and is_file/is_directory inspection.
- RAII state-restoration in every test project's Rust harness.
- `import X from standard` as the exposure mechanism.
- Pre-Flight Validation gate before any TDD execution.

### Must NOT Have (Guardrails)
- No `file_` or similar prefix on any function.
- No `uint8[]` — use `Bytes` exclusively for byte data.
- No async / streaming / memory-mapped / locking primitives.
- No string-based path manipulation anywhere in the public API.
- No refactoring of existing stdlib code (`bytes_*`, `io_*` etc.) — scope is purely additive.
- No Windows CI for stdlib tests in this plan; tracked as follow-up.
- No 39th function or 21st error type — scope is frozen at 38 fns + 20 errors. Gaps → `.sisyphus/followups.md`.
- No test fixtures outside each test project's own directory.
- No absolute paths (anywhere a path is constructed, it must be derived from the test's project-local root or a `tempdir()`).
- Momus IS invoked as the final review gate (F5) per user directive: "you do need to invoke Momus at the end". Loop until `OKAY`.

---

## Verification Strategy (MANDATORY)

> **ZERO HUMAN INTERVENTION** — ALL verification is agent-executed.

### Test Decision
- **Infrastructure exists**: YES (Rust unit tests + cargo `integration` feature + test-projects harness).
- **Automated tests**: YES — TDD red-green-refactor per user directive.
- **Framework**: Rust `#[test]` + `cargo test --features integration` + Opalescent test-projects compiled and executed.
- **TDD cadence**:
  - Batch level: write all registration tests RED for a whole category (e.g., "File Reading"), then implement touchpoints 1–6 once for the category, then per-function GREEN + REFACTOR.
  - Per-function: one Rust unit test for type-checker registration + one integration test for compile+run behavior.

### QA Policy
Every function has:
- **Happy-path scenario**: compile an .op program that calls it with valid inputs, assert stdout/filesystem state matches expected.
- **Failure-path scenario**: compile an .op program that triggers the primary error type, assert the `guard` `else` branch fires and `error_cstr` matches the declared error name.
- Evidence under `.sisyphus/evidence/fs-<category>/<function>-<scenario>/`: `stdout.log`, `exit-code.txt`, `pre-state.txt`, `post-state.txt`, `state-diff.txt` (empty), `cleanup-marker.txt`.

### Per-Project State Restoration
Every Rust integration test wraps the test body in a `StateGuard` RAII struct:
```rust
struct StateGuard { project_dir: PathBuf, snapshot: HashMap<PathBuf, Vec<u8>>, created_paths: Vec<PathBuf> }
impl Drop for StateGuard {
    fn drop(&mut self) {
        // Restore every snapshotted file byte-for-byte.
        // Remove every path in created_paths (files then dirs in reverse order).
        // Write cleanup-marker.txt with ISO-8601 timestamp.
    }
}
```
Pre-state SHA-256 manifest is captured at StateGuard::new, post-state at test end; their diff must be empty.

### Tooling per QA Scenario
- **Opalescent program behavior**: Bash (compile via `opal run src/main.op`, assert stdout + exit code).
- **Filesystem state**: Bash (sha256sum manifest diff).
- **Type-checker negative tests**: Rust unit test (assert diagnostic error messages).

---

## Execution Strategy

### Parallel Execution Waves

```
Wave 0 (PRE-FLIGHT — blocks everything):
├── T0. Pre-Flight Validation [deep]

Wave 1 (FOUNDATION — after T0):
├── T1.  Register FilesystemPath / FileMetadata / FilePermissions nominal types [unspecified-high]
├── T2.  Register 20 error types in fs_builtins.rs [unspecified-high]
├── T3.  Create runtime/opal_fs.c skeleton + add to RUNTIME_SOURCE [quick]
├── T4.  Extend known_runtime_return_type / known_guard_success_type skeleton for FS result structs [unspecified-high]

Wave 2 (INFRASTRUCTURE BATCHES — after Wave 1, max parallel):
├── T5.  Infra batch: Path manipulation category (7 fns wired) [unspecified-high]
├── T6.  Infra batch: File Reading category (4 fns wired) [unspecified-high]
├── T7.  Infra batch: File Writing category (7 fns wired, incl. atomic) [deep]
├── T8.  Infra batch: File Management category (7 fns wired, incl. nofollow metadata) [deep]
├── T9.  Infra batch: Directory Operations category (9 fns wired, incl. 2 nofollow) [deep]
├── T10. Infra batch: Permissions category (4 fns wired) [unspecified-high]

Wave 3 (BEHAVIOR TDD — after Wave 2, max parallel):
├── T11. TDD: path helpers per-function red-green-refactor [unspecified-high]
├── T12. TDD: reading fns red-green-refactor [deep]
├── T13. TDD: writing fns red-green-refactor (incl. atomic temp+rename) [deep]
├── T14. TDD: file mgmt fns red-green-refactor [deep]
├── T15. TDD: dir ops fns red-green-refactor [deep]
├── T16. TDD: permissions fns red-green-refactor [unspecified-high]

Wave 4 (TEST PROJECTS + DOCS — after Wave 3, parallel):
├── T17. Test project: fs-path-manipulation + integration test [unspecified-high]
├── T18. Test project: fs-markdown-roundtrip + integration test [unspecified-high]
├── T19. Test project: fs-directory-operations + integration test [deep]
├── T20. stdlib/prelude.op doc additions [writing]
├── T21. README Windows-program-build subsection [writing]
├── T22. .sisyphus/followups.md entry for Windows parity [quick]

Wave FINAL (MANDATORY — after ALL):
├── F1. Plan compliance audit (oracle)
├── F2. Code quality review (unspecified-high)
├── F3. Real manual QA — run every QA scenario, verify every evidence file (unspecified-high)
├── F4. Scope fidelity check (deep)
├── F5. Inline Momus-style self-review by Prometheus
-> Present consolidated results -> wait for user's explicit "okay"

Critical Path: T0 → T1/T2/T3/T4 → T5-T10 → T11-T16 → T17-T22 → F1-F5 → user okay
Max Concurrent: 6 (Waves 2 & 3)
```

### Dependency Matrix (abbreviated)
- **T0**: blocks everything — validates array returns + scalar fallible ABI before investing in 38 fns.
- **T1–T4**: depend on T0; unblock all infra batches.
- **T5–T10**: depend on T1+T2+T3+T4; parallel amongst themselves.
- **T11–T16**: each depends on its own infra batch (T11→T5, T12→T6, …).
- **T17**: depends on T11 only (pure path helpers).
- **T18**: depends on T12+T13+T14 (reads, writes, file mgmt).
- **T19**: depends on T14+T15 (file mgmt + dir ops).
- **T20**: depends on all of T11–T16 (docs for completed surface).
- **T21–T22**: independent of stdlib implementation; can run any time after T0.
- **F1–F5**: depend on T17–T22.

### Agent Dispatch Summary
- Wave 0: 1 task → `deep`
- Wave 1: 4 tasks → 3×`unspecified-high` + 1×`quick`
- Wave 2: 6 tasks → 3×`unspecified-high` + 3×`deep`
- Wave 3: 6 tasks → 2×`unspecified-high` + 4×`deep`
- Wave 4: 6 tasks → 3×`unspecified-high` + 1×`deep` + 2×`writing` + 1×`quick`
- Wave FINAL: 4 tasks + self-review → `oracle`, `unspecified-high`×2, `deep`, Prometheus (self)

---

## TODOs

> Implementation + Test = ONE Task. Every task MUST have QA Scenarios.

- [x] 0. Pre-Flight Validation — Codegen Capability Check

  **What to do**:
  - Create a throwaway branch-local test program `/tmp/preflight_arrays.op` that declares a stdlib-style builtin signature `preflight_string_array(): string[]` and `preflight_path_array(): FilesystemPath[]` and a fallible scalar `preflight_fallible_bool(): boolean errors PermissionDeniedError`.
  - Wire each as a temporary builtin in `fs_builtins.rs` + `functions_stdlib.rs` + `statements.rs` + a minimal `opal_fs_preflight.c` that returns hardcoded values.
  - Compile with the full pipeline (`opal run`) and assert:
    - String array: program prints each element line by line.
    - FilesystemPath array: program iterates, calls `path_file_name` on each, prints.
    - Fallible boolean: `guard` success and error branches both reachable.
  - If ANY fail, STOP the plan and write findings to `.sisyphus/evidence/preflight/blockers.md`, then surface to user for replanning.
  - Once validated, REVERT all preflight scaffolding (do not leave preflight fns in the real code).

  **Must NOT do**:
  - Do NOT commit preflight scaffolding to main.
  - Do NOT implement any of the 38 real fns yet.

  **Recommended Agent Profile**:
  - **Category**: `deep` — speculative codegen validation needs careful reasoning about ABI.
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: NO — blocks all other tasks.
  - **Blocked By**: None.
  - **Blocks**: T1–T22.

  **References**:
  - `src/codegen/functions_stdlib.rs:STDLIB_NAMES` — existing builtin registration pattern
  - `src/codegen/statements.rs:known_runtime_return_type` — existing return-type wiring (Bytes returns)
  - `src/type_system/checker/bytes_builtins.rs` — nominal-type registration template
  - `runtime/opal_bytes.c` — C-side fallible return precedent (`bytes_result_type` struct shape)
  - **WHY**: The 8-touchpoint pattern is proven for single-value returns. Array returns and scalar fallible returns have NOT been empirically verified from a stdlib builtin. Failing now saves rewriting 38 functions later.

  **Acceptance Criteria**:
  - [ ] `/tmp/preflight_arrays.op` compiles via `opal run`.
  - [ ] stdout contains the expected string-array iteration.
  - [ ] stdout contains the expected FilesystemPath-array iteration (via `path_file_name`).
  - [ ] Both `guard` branches of `preflight_fallible_bool` exercise cleanly.
  - [ ] All preflight scaffolding reverted; `git status` shows only `.sisyphus/evidence/preflight/*`.

  **QA Scenarios**:

  ```
  Scenario: Array return types work from stdlib builtins
    Tool: Bash
    Preconditions: Preflight scaffolding wired, compiler built.
    Steps:
      1. cargo build --release
      2. target/release/opalescent run /tmp/preflight_arrays.op
      3. Capture stdout.
    Expected Result: stdout exactly:
      str-elem-0
      str-elem-1
      path-name: a.txt
      path-name: b.txt
      guard-ok: true
      guard-err: PermissionDeniedError
    Failure Indicators: Segfault, "unknown runtime function", type-check error on array iteration.
    Evidence: .sisyphus/evidence/preflight/stdout.log
             .sisyphus/evidence/preflight/exit-code.txt

  Scenario: Revert cleanly
    Tool: Bash
    Preconditions: Preflight validated.
    Steps:
      1. git status --porcelain -- src/ runtime/
      2. Assert empty output.
    Expected Result: No uncommitted preflight changes remain.
    Evidence: .sisyphus/evidence/preflight/git-status.txt
  ```

  **Commit**: NO

- [x] 1. Register FilesystemPath, FileMetadata, FilePermissions nominal types

  **What to do**:
  - Create `src/type_system/checker/fs_builtins.rs` following the `bytes_builtins.rs` pattern.
  - Inside, implement `register_fs_nominal_types(checker: &mut TypeChecker)` that registers three nominal product types:
    - `FilesystemPath` with a single field `raw: string` (enables `new FilesystemPath:` record constructor per user Q&A).
    - `FileMetadata` with fields `size_bytes: int64`, `is_directory: boolean`, `is_symlink: boolean`, `modified_unix_seconds: int64`.
    - `FilePermissions` with fields `readable: boolean`, `writable: boolean`, `executable: boolean`.
  - Ensure each type is importable via `import FilesystemPath from standard` etc.
  - Add the call `register_fs_nominal_types(...)` in `src/type_system/checker.rs::register_standard_builtins` (right after `register_bytes_builtins`).
  - Write one Rust unit test per type verifying: (a) type resolves when imported, (b) record literal `new FilesystemPath: raw = '/tmp'` typechecks, (c) field access typechecks.

  **Must NOT do**:
  - Do NOT add methods on these types (all ops are free fns per proposal).
  - Do NOT introduce any path-string coercion.

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high` — type-system wiring.
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES (with T2, T3, T4).
  - **Blocked By**: T0.
  - **Blocks**: T5–T10.

  **References**:
  - `src/type_system/checker/bytes_builtins.rs` — template for nominal type registration.
  - `src/type_system/checker.rs:~295` — `register_standard_builtins` call site.
  - **WHY**: Must match the exact registration pattern used by Bytes so `import` resolution finds them.

  **Acceptance Criteria**:
  - [ ] `cargo test type_system::fs::nominal_types` → PASS (3 tests).
  - [ ] Demo Opalescent program can `import FilesystemPath from standard` and construct a value via `new FilesystemPath: raw = '/a/b'`.
  - [ ] `rg 'FilesystemPath' src/type_system/` shows registration in both `fs_builtins.rs` and `checker.rs`.

  **QA Scenarios**:

  ```
  Scenario: Types resolve and construct
    Tool: Bash
    Preconditions: T0 passed, compiler rebuilt.
    Steps:
      1. cat > /tmp/fs_types_ok.op <<'EOF'
         import FilesystemPath from standard
         entry main = f(args: string[]): void =>
             let p = new FilesystemPath: raw = '/tmp/x'
             print(p.raw)
             return void
         EOF
      2. target/release/opalescent run /tmp/fs_types_ok.op
    Expected Result: stdout = "/tmp/x", exit 0.
    Failure Indicators: "unknown type FilesystemPath", record-construction error.
    Evidence: .sisyphus/evidence/fs-foundation/t1-types-ok/{stdout.log,exit-code.txt}

  Scenario: Wrong field type rejected
    Tool: Bash
    Preconditions: Same as above.
    Steps:
      1. cat > /tmp/fs_types_bad.op <<'EOF'
         import FilesystemPath from standard
         entry main = f(args: string[]): void =>
             let p = new FilesystemPath: raw = 42
             return void
         EOF
      2. target/release/opalescent check /tmp/fs_types_bad.op ; echo $?
    Expected Result: non-zero exit, diagnostic mentioning `raw` expected string.
    Failure Indicators: typechecks successfully (would mean unsafe coercion).
    Evidence: .sisyphus/evidence/fs-foundation/t1-types-bad/{stderr.log,exit-code.txt}
  ```

  **Commit**: YES (grouped with T2) — `feat(stdlib): register FilesystemPath types and filesystem error nominals`

- [x] 2. Register 20 filesystem error types

  **What to do**:
  - In `src/type_system/checker/fs_builtins.rs` add `register_fs_error_types(checker)` that registers all 20 error nominals as empty product types (matches existing error conventions):
    - FileNotFoundError, PermissionDeniedError, FileAlreadyExistsError, ReadFailureError, WriteFailureError, InvalidPathError, FilesystemFullError, IsADirectoryError, IsNotADirectoryError, DirectoryNotEmptyError, DirectoryNotFoundError, MetadataUnavailableError, OffsetOutOfRangeError, LineOutOfRangeError, CopyFailureError, MoveFailureError, DeleteFailureError, CreateFailureError, SetPermissionsError, InvalidUtf8Error.
  - Each must be importable individually via `import FileNotFoundError from standard`.
  - Add a single Rust unit test iterating all 20 names and verifying each resolves.

  **Must NOT do**:
  - Do NOT add fields to the error types (they are tag-only per existing Opalescent convention).
  - Do NOT namespace them under a module (all flat under `standard`).

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`.
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES (with T1, T3, T4).
  - **Blocked By**: T0.
  - **Blocks**: T5–T10.

  **References**:
  - **Canonical 20-error list is defined IN THIS PLAN ONLY** — see "Work Objectives → Concrete Deliverables" and T2 "What to do" below. The file `stdlib-proposals/file-io-surface/path-object-centric/filesystem_errors.types.op` is a PARTIAL EARLY DRAFT containing only 10 errors and must NOT be used as the source of truth; consult it only for naming style reference for the 10 it contains.
  - Any existing error-type registration (e.g., how Bytes errors are registered, if any) as pattern.

  **Acceptance Criteria**:
  - [ ] `cargo test type_system::fs::error_types` → PASS.
  - [ ] Demo program can `import FileNotFoundError from standard` and reference it in an `errors` clause.
  - [ ] All 20 names appear in the standard-module export set in `module_resolver.rs`.

  **QA Scenarios**:

  ```
  Scenario: All 20 errors importable
    Tool: Bash
    Preconditions: T1 committed, compiler rebuilt.
    Steps:
      1. Generate /tmp/fs_errors_all.op that imports all 20 error types on separate lines and references each in a dead `let` expression.
      2. target/release/opalescent check /tmp/fs_errors_all.op
    Expected Result: exit 0, no unknown-symbol diagnostics.
    Failure Indicators: Any "unknown type <ErrorName>" diagnostic.
    Evidence: .sisyphus/evidence/fs-foundation/t2-all-errors/{stdout.log,exit-code.txt}

  Scenario: Unknown error rejected
    Tool: Bash
    Preconditions: Same.
    Steps:
      1. echo "import NotAnError from standard" > /tmp/fs_errors_bad.op
      2. target/release/opalescent check /tmp/fs_errors_bad.op ; echo $?
    Expected Result: non-zero, diagnostic "unknown symbol NotAnError".
    Evidence: .sisyphus/evidence/fs-foundation/t2-unknown/{stderr.log,exit-code.txt}
  ```

  **Commit**: grouped with T1 — see above.

- [x] 3. Scaffold runtime/opal_fs.c and wire into RUNTIME_SOURCE

  **What to do**:
  - Create `runtime/opal_fs.c` with a leading comment block describing ownership contracts (caller-owns-error-string, caller-frees-returned-value, etc.) and an empty function table ready for T5–T10 to populate.
  - Declare the 10 `FsXxxResult` structs in `runtime/opal_runtime.h` (FsVoidResult, FsBytesResult, FsStringResult, FsBooleanResult, FsInt32Result, FsInt64Result, FsPathResult, FsPathArrayResult, FsStringArrayResult, FsMetadataResult, FsPermissionsResult) — each is `{ void* value; const char* error; }` with `value` typed appropriately.
  - Append `include_str!("../runtime/opal_fs.c")` to `RUNTIME_SOURCE` concat in `src/compiler.rs` (lines 41–56 range).
  - Ensure `cargo build --release` still compiles (no unresolved symbols yet since no builtins declared — the file is just a skeleton with structs).

  **Must NOT do**:
  - Do NOT add any fs function bodies yet (T5–T10 do that).
  - Do NOT modify existing `opal_bytes.c` / `opal_io.c`.

  **Recommended Agent Profile**:
  - **Category**: `quick`.
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES (with T1, T2, T4).
  - **Blocked By**: T0.
  - **Blocks**: T5–T10.

  **References**:
  - `runtime/opal_bytes.c` — ABI pattern for fallible returns.
  - `runtime/opal_runtime.h` — struct declaration site.
  - `src/compiler.rs:41-56` — RUNTIME_SOURCE concat.

  **Acceptance Criteria**:
  - [ ] `cargo build --release` → success.
  - [ ] `runtime/opal_fs.c` exists with header comment + no function bodies.
  - [ ] `runtime/opal_runtime.h` contains all 10 `Fs*Result` struct declarations.
  - [ ] `rg 'opal_fs.c' src/compiler.rs` returns a match.

  **QA Scenarios**:

  ```
  Scenario: Skeleton compiles
    Tool: Bash
    Preconditions: T0 passed.
    Steps:
      1. cargo build --release 2>&1 | tee .sisyphus/evidence/fs-foundation/t3-build.log
    Expected Result: "Compiling opalescent" + "Finished".
    Failure Indicators: compile error in opal_fs.c or opal_runtime.h.
    Evidence: .sisyphus/evidence/fs-foundation/t3-build.log

  Scenario: Structs defined
    Tool: Bash
    Steps:
      1. grep -c 'Result' runtime/opal_runtime.h
    Expected Result: count ≥ 11 (10 new + any existing).
    Evidence: .sisyphus/evidence/fs-foundation/t3-structs-count.txt
  ```

  **Commit**: YES (grouped with T4) — `feat(runtime): scaffold opal_fs.c and FS result struct ABI`

- [x] 4. Extend codegen known_runtime_return_type / known_guard_success_type for FS result structs

  **What to do**:
  - In `src/codegen/statements.rs`, extend `known_runtime_return_type` to handle each of the 10 new FS result structs, mapping them to the appropriate Opalescent return type lowering.
  - In `known_guard_success_type` (around lines 580–596), add match arms so that `guard`/`propagate` on any fs builtin produces the correct "unwrapped" success type (e.g., `FsBytesResult` → `Bytes`).
  - Add a Rust unit test `codegen::fs::result_structs_wire_through_guard` that calls each new mapping helper on a synthesized runtime-function descriptor and asserts the returned type matches expectation.

  **Must NOT do**:
  - Do NOT register the 38 function names yet (that's T5–T10's job).
  - Do NOT modify the Bytes result handling.

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`.
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES (with T1, T2, T3).
  - **Blocked By**: T0.
  - **Blocks**: T5–T10.

  **References**:
  - `src/codegen/statements.rs:518-596` — current known_runtime_return_type / known_guard_success_type.
  - `src/codegen/functions_stdlib.rs:198-205` — declare_stdlib_function precedent for Bytes ABI.

  **Acceptance Criteria**:
  - [ ] `cargo build --release` → success.
  - [ ] `cargo test codegen::fs::result_structs_wire_through_guard` → PASS.
  - [ ] Synthetic test: `guard` on a fake `FsBytesResult` builtin lowers to correct LLVM IR (check via snapshot test if feasible, else via type inspection).

  **QA Scenarios**:

  ```
  Scenario: Unit test passes
    Tool: Bash
    Steps:
      1. cargo test codegen::fs::result_structs_wire_through_guard
    Expected Result: "test result: ok. 1 passed".
    Evidence: .sisyphus/evidence/fs-foundation/t4-unit.log

  Scenario: No regressions in Bytes lowering
    Tool: Bash
    Steps:
      1. cargo test codegen::bytes
    Expected Result: all prior Bytes codegen tests still pass.
    Evidence: .sisyphus/evidence/fs-foundation/t4-bytes-regression.log
  ```

  **Commit**: grouped with T3.

- [ ] 5. Infra batch: Path Manipulation category (7 fns wired, no behavior yet)

  **What to do**:
  - Wire the 7 path-manipulation functions through touchpoints 3–7 (declarations only; empty C bodies returning hardcoded placeholder OK for RED phase).
  - Functions: `path_from`, `join_path_components`, `path_parent_directory`, `path_file_name`, `path_file_extension`, `normalize_path`, `absolute_path_sync`.
  - Signatures (register in `fs_builtins.rs`):
    - `path_from(raw_path: string): FilesystemPath` — infallible
    - `join_path_components(base: FilesystemPath, components: string[]): FilesystemPath` — infallible, pure lexical
    - `path_parent_directory(path: FilesystemPath): FilesystemPath` — infallible lexical (root returns itself)
    - `path_file_name(path: FilesystemPath): string` — infallible lexical
    - `path_file_extension(path: FilesystemPath): string` — infallible lexical (returns '' if none)
    - `normalize_path(path: FilesystemPath): FilesystemPath` — infallible lexical (collapses `..`, `.`, double separators)
    - `absolute_path_sync(path: FilesystemPath): FilesystemPath errors InvalidPathError, PermissionDeniedError` — fallible, TOUCHES FS (getcwd + join)
  - Add all 7 to `STDLIB_NAMES`, `declare_stdlib_function`, `known_runtime_return_type`, `known_guard_success_type` (only absolute_path_sync needs guard type), and the `standard`-module export list in `module_resolver.rs`.
  - Stub C bodies in `opal_fs.c` (return NULL-ptr FilesystemPath / empty string / whatever — behavior comes in T11).

  **Must NOT do**:
  - Do NOT implement behavior yet — just the wiring. T11 does TDD behavior.
  - Do NOT have `normalize_path` or any of the first 6 touch the filesystem.

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`.
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES (with T6–T10).
  - **Blocked By**: T1, T2, T3, T4.
  - **Blocks**: T11.

  **References**:
  - `stdlib-proposals/file-io-surface/path-object-centric/proposal.md` — signatures (may have syntax errors; authoritative type/semantics only).
  - Language spec — `f(params): Return errors E1, E2` syntax.
  - `src/codegen/functions_stdlib.rs:STDLIB_NAMES` — append here.

  **Acceptance Criteria**:
  - [ ] All 7 names present in `STDLIB_NAMES`.
  - [ ] `cargo build --release` succeeds.
  - [ ] A demo `.op` program can `import path_from from standard` and call it with a dummy arg; program compiles (may crash at runtime — that's fine here).
  - [ ] Type-check test: wrong arg type rejected for each fn.

  **QA Scenarios**:

  ```
  Scenario: All 7 fns compile
    Tool: Bash
    Steps:
      1. Generate /tmp/path_all.op importing and calling each of the 7 fns with type-correct dummy args.
      2. target/release/opalescent check /tmp/path_all.op
    Expected Result: exit 0.
    Evidence: .sisyphus/evidence/fs-infra/t5-all.log

  Scenario: Type errors caught
    Tool: Bash
    Steps:
      1. Generate /tmp/path_bad.op that calls path_from(42).
      2. target/release/opalescent check /tmp/path_bad.op ; echo $?
    Expected Result: non-zero, diagnostic on arg type.
    Evidence: .sisyphus/evidence/fs-infra/t5-bad.log
  ```

  **Commit**: YES — `feat(stdlib): wire path-manipulation infrastructure`

- [x] 6. Infra batch: File Reading category (4 fns wired)

  **What to do**:
  - Wire 4 reading functions: `read_contents_sync`, `read_text_sync`, `read_lines_sync`, `read_bytes_at_offset_sync`.
  - Signatures:
    - `read_contents_sync(path: FilesystemPath): Bytes errors FileNotFoundError, PermissionDeniedError, ReadFailureError, IsADirectoryError, InvalidPathError`
    - `read_text_sync(path: FilesystemPath): string errors FileNotFoundError, PermissionDeniedError, ReadFailureError, IsADirectoryError, InvalidPathError, InvalidUtf8Error`
    - `read_lines_sync(path: FilesystemPath): string[] errors FileNotFoundError, PermissionDeniedError, ReadFailureError, IsADirectoryError, InvalidPathError, InvalidUtf8Error`
    - `read_bytes_at_offset_sync(path: FilesystemPath, offset: int64, length: int64): Bytes errors FileNotFoundError, PermissionDeniedError, ReadFailureError, OffsetOutOfRangeError, InvalidPathError`
  - Wire through all 7 touchpoints with stub bodies.
  - Add to `module_resolver.rs` standard exports.

  **Must NOT do**:
  - Do NOT implement UTF-8 validation or line-splitting behavior yet.
  - Do NOT add streaming variants.

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`.
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES (with T5, T7–T10).
  - **Blocked By**: T1–T4.
  - **Blocks**: T12.

  **References**:
  - `runtime/opal_bytes.c` — byte ownership.
  - Proposal signatures for reading fns.

  **Acceptance Criteria**:
  - [ ] 4 fns in `STDLIB_NAMES`.
  - [ ] `cargo build --release` OK.
  - [ ] Each fn importable and callable in a compile-only demo.
  - [ ] `guard` on each unwraps to the declared success type.

  **QA Scenarios**:

  ```
  Scenario: Reading fns compile + type-check
    Tool: Bash
    Steps:
      1. Generate /tmp/read_all.op with guard over each of 4 fns and a propagate variant.
      2. target/release/opalescent check /tmp/read_all.op
    Expected Result: exit 0.
    Evidence: .sisyphus/evidence/fs-infra/t6-all.log

  Scenario: guard success type is correct
    Tool: Bash
    Steps:
      1. Generate .op where guard over read_text_sync binds the success into a `let x: string`.
      2. opal check
    Expected Result: exit 0; changing the binding type to int32 should fail.
    Evidence: .sisyphus/evidence/fs-infra/t6-guard-type.log
  ```

  **Commit**: YES — `feat(stdlib): wire file-reading infrastructure`

- [x] 7. Infra batch: File Writing category (7 fns wired, incl. atomic variants)

  **What to do**:
  - Wire 7 writing functions: `write_contents_sync`, `write_text_sync`, `write_contents_atomic_sync`, `write_text_atomic_sync`, `append_contents_sync`, `append_text_sync`, `write_bytes_at_offset_sync`.
  - Signatures:
    - `write_contents_sync(path: FilesystemPath, data: Bytes): void errors PermissionDeniedError, WriteFailureError, IsADirectoryError, InvalidPathError, FilesystemFullError`
    - `write_text_sync(path: FilesystemPath, text: string): void errors PermissionDeniedError, WriteFailureError, IsADirectoryError, InvalidPathError, FilesystemFullError`
    - `write_contents_atomic_sync(path: FilesystemPath, data: Bytes): void errors PermissionDeniedError, WriteFailureError, IsADirectoryError, InvalidPathError, FilesystemFullError`
    - `write_text_atomic_sync(path: FilesystemPath, text: string): void errors PermissionDeniedError, WriteFailureError, IsADirectoryError, InvalidPathError, FilesystemFullError`
    - `append_contents_sync(path: FilesystemPath, data: Bytes): void errors FileNotFoundError, PermissionDeniedError, WriteFailureError, IsADirectoryError, InvalidPathError, FilesystemFullError`
    - `append_text_sync(path: FilesystemPath, text: string): void errors FileNotFoundError, PermissionDeniedError, WriteFailureError, IsADirectoryError, InvalidPathError, FilesystemFullError`
    - `write_bytes_at_offset_sync(path: FilesystemPath, offset: int64, data: Bytes): void errors FileNotFoundError, PermissionDeniedError, WriteFailureError, OffsetOutOfRangeError, InvalidPathError, FilesystemFullError`
  - Wire stubs through all 7 touchpoints. Reserve `FsVoidResult { void* = NULL; const char* error; }` shape (value unused, only error signals).

  **Must NOT do**:
  - Do NOT implement atomic temp+rename yet (T13 does that).
  - Do NOT auto-create parent directories.

  **Recommended Agent Profile**:
  - **Category**: `deep` — atomic ABI has subtle ownership considerations.
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES (with T5, T6, T8–T10).
  - **Blocked By**: T1–T4.
  - **Blocks**: T13.

  **References**:
  - Proposal signatures.
  - POSIX rename(2) semantics for atomic guidance (but not implemented here).

  **Acceptance Criteria**:
  - [ ] 7 fns in `STDLIB_NAMES`.
  - [ ] `cargo build --release` OK.
  - [ ] `import write_text_atomic_sync from standard` succeeds.
  - [ ] Type errors on wrong arg types.

  **QA Scenarios**:

  ```
  Scenario: All 7 writers compile
    Tool: Bash
    Steps:
      1. Generate /tmp/write_all.op exercising each via propagate.
      2. target/release/opalescent check /tmp/write_all.op
    Expected Result: exit 0.
    Evidence: .sisyphus/evidence/fs-infra/t7-all.log

  Scenario: Atomic vs non-atomic distinct symbols
    Tool: Bash
    Steps:
      1. nm target/release/opalescent 2>/dev/null | grep -E 'write_(text|contents)(_atomic)?_sync' | sort -u | wc -l
    Expected Result: 4 distinct symbols.
    Evidence: .sisyphus/evidence/fs-infra/t7-symbols.log
  ```

  **Commit**: YES — `feat(stdlib): wire file-writing infrastructure with atomic variants`

- [ ] 8. Infra batch: File Management category (7 fns wired, incl. nofollow metadata)

  **What to do**:
  - Wire 7 fns: `create_file_sync`, `delete_file_sync`, `copy_file_sync`, `move_path_sync`, `path_exists_sync`, `read_metadata_sync`, `read_metadata_nofollow_sync`.
  - Signatures:
    - `create_file_sync(path: FilesystemPath): void errors FileAlreadyExistsError, PermissionDeniedError, CreateFailureError, InvalidPathError, FilesystemFullError`
    - `delete_file_sync(path: FilesystemPath): void errors FileNotFoundError, PermissionDeniedError, DeleteFailureError, IsADirectoryError, InvalidPathError`
    - `copy_file_sync(source: FilesystemPath, destination: FilesystemPath): void errors FileNotFoundError, PermissionDeniedError, CopyFailureError, IsADirectoryError, InvalidPathError, FilesystemFullError`
    - `move_path_sync(source: FilesystemPath, destination: FilesystemPath): void errors FileNotFoundError, PermissionDeniedError, MoveFailureError, FileAlreadyExistsError, InvalidPathError`
    - `path_exists_sync(path: FilesystemPath): boolean errors PermissionDeniedError, InvalidPathError`
    - `read_metadata_sync(path: FilesystemPath): FileMetadata errors FileNotFoundError, PermissionDeniedError, MetadataUnavailableError, InvalidPathError`
    - `read_metadata_nofollow_sync(path: FilesystemPath): FileMetadata errors FileNotFoundError, PermissionDeniedError, MetadataUnavailableError, InvalidPathError`
  - Wire all through 7 touchpoints. Add FsMetadataResult struct + lowering. Stub C bodies.

  **Must NOT do**:
  - Do NOT implement symlink-follow difference yet (T14 does that).
  - Do NOT auto-overwrite destination in copy/move (errors instead).

  **Recommended Agent Profile**:
  - **Category**: `deep` — FileMetadata return type involves struct-by-value lowering care.
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES (with T5–T7, T9, T10).
  - **Blocked By**: T1–T4.
  - **Blocks**: T14.

  **References**:
  - Proposal signatures.
  - `runtime/opal_bytes.c` — struct ownership pattern for nominal-type returns.

  **Acceptance Criteria**:
  - [ ] 7 fns in STDLIB_NAMES.
  - [ ] `cargo build --release` OK.
  - [ ] `guard read_metadata_sync(p) into m else { ... }` typechecks and `m.size_bytes` resolves.
  - [ ] `path_exists_sync` returns a boolean that `guard` unwraps.

  **QA Scenarios**:

  ```
  Scenario: File mgmt compile + type-check
    Tool: Bash
    Steps:
      1. Generate /tmp/fmgmt_all.op using guard for each of 7 fns.
      2. opal check
    Expected Result: exit 0.
    Evidence: .sisyphus/evidence/fs-infra/t8-all.log

  Scenario: FileMetadata field access typechecks
    Tool: Bash
    Steps:
      1. Generate .op using read_metadata_sync then printing m.size_bytes.
      2. opal check
    Expected Result: exit 0.
    Evidence: .sisyphus/evidence/fs-infra/t8-metadata-fields.log
  ```

  **Commit**: YES — `feat(stdlib): wire file-management infrastructure`

- [ ] 9. Infra batch: Directory Operations category (9 fns wired, incl. 2 nofollow inspections)

  **What to do**:
  - Wire 9 fns: `create_directory_sync`, `create_directory_recursive_sync`, `delete_directory_sync`, `delete_directory_recursive_sync`, `list_directory_sync`, `is_file_sync`, `is_file_nofollow_sync`, `is_directory_sync`, `is_directory_nofollow_sync`.
  - Signatures:
    - `create_directory_sync(path: FilesystemPath): void errors FileAlreadyExistsError, PermissionDeniedError, CreateFailureError, InvalidPathError, FilesystemFullError`
    - `create_directory_recursive_sync(path: FilesystemPath): void errors PermissionDeniedError, CreateFailureError, InvalidPathError, FilesystemFullError`
    - `delete_directory_sync(path: FilesystemPath): void errors DirectoryNotFoundError, PermissionDeniedError, DeleteFailureError, DirectoryNotEmptyError, IsNotADirectoryError, InvalidPathError`
    - `delete_directory_recursive_sync(path: FilesystemPath): void errors DirectoryNotFoundError, PermissionDeniedError, DeleteFailureError, IsNotADirectoryError, InvalidPathError`
    - `list_directory_sync(path: FilesystemPath): FilesystemPath[] errors DirectoryNotFoundError, PermissionDeniedError, ReadFailureError, IsNotADirectoryError, InvalidPathError`
    - `is_file_sync(path: FilesystemPath): boolean errors PermissionDeniedError, InvalidPathError`
    - `is_file_nofollow_sync(path: FilesystemPath): boolean errors PermissionDeniedError, InvalidPathError`
    - `is_directory_sync(path: FilesystemPath): boolean errors PermissionDeniedError, InvalidPathError`
    - `is_directory_nofollow_sync(path: FilesystemPath): boolean errors PermissionDeniedError, InvalidPathError`
  - Wire through 7 touchpoints; stub bodies. Ensure `FilesystemPath[]` return path is exercised (leans on T0 validation).

  **Must NOT do**:
  - Do NOT sort or filter `list_directory_sync` results in stub (T15 decides ordering guarantees).
  - Do NOT auto-create parents in `create_directory_sync` (that's what `_recursive` is for).

  **Recommended Agent Profile**:
  - **Category**: `deep` — `FilesystemPath[]` return is the most ABI-intricate in the plan.
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES (with T5–T8, T10).
  - **Blocked By**: T1–T4, T0 (validates array returns).
  - **Blocks**: T15.

  **References**:
  - Preflight validation T0 outcome — confirms array return is codegen-safe.
  - Proposal signatures.

  **Acceptance Criteria**:
  - [ ] 9 fns in STDLIB_NAMES.
  - [ ] `cargo build --release` OK.
  - [ ] Demo `.op` iterating `list_directory_sync(p)` with `for entry in entries` typechecks (entry is FilesystemPath).
  - [ ] Distinct symbols for the 4 `_nofollow_sync` vs follow variants.

  **QA Scenarios**:

  ```
  Scenario: Dir ops compile
    Tool: Bash
    Steps:
      1. Generate /tmp/dir_all.op exercising each of 9 fns.
      2. opal check
    Expected Result: exit 0.
    Evidence: .sisyphus/evidence/fs-infra/t9-all.log

  Scenario: FilesystemPath[] iteration typechecks
    Tool: Bash
    Steps:
      1. Generate .op: `for child in (propagate list_directory_sync(p)): print(path_file_name(child))`.
      2. opal check
    Expected Result: exit 0.
    Evidence: .sisyphus/evidence/fs-infra/t9-iter.log
  ```

  **Commit**: YES — `feat(stdlib): wire directory-operations infrastructure`

- [ ] 10. Infra batch: Permissions category (4 fns wired)

  **What to do**:
  - Wire 4 fns: `can_read_sync`, `can_write_sync`, `can_execute_sync`, `set_permissions_sync`.
  - Signatures:
    - `can_read_sync(path: FilesystemPath): boolean errors FileNotFoundError, PermissionDeniedError, InvalidPathError`
    - `can_write_sync(path: FilesystemPath): boolean errors FileNotFoundError, PermissionDeniedError, InvalidPathError`
    - `can_execute_sync(path: FilesystemPath): boolean errors FileNotFoundError, PermissionDeniedError, InvalidPathError`
    - `set_permissions_sync(path: FilesystemPath, permissions: FilePermissions): void errors FileNotFoundError, PermissionDeniedError, SetPermissionsError, InvalidPathError`
  - Wire through 7 touchpoints. FsPermissionsResult not strictly required (no fn returns FilePermissions in current inventory — `can_*` return booleans, `set_*` returns void).
  - Stub C bodies.

  **Must NOT do**:
  - Do NOT add an octal-mode API (proposal explicitly chose abstract triple).
  - Do NOT expose Unix-specific modes.

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`.
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES (with T5–T9).
  - **Blocked By**: T1–T4.
  - **Blocks**: T16.

  **References**:
  - Proposal signatures.
  - Windows parity notes → `.sisyphus/followups.md`.

  **Acceptance Criteria**:
  - [ ] 4 fns in STDLIB_NAMES.
  - [ ] `cargo build --release` OK.
  - [ ] `set_permissions_sync(p, new FilePermissions: readable = true, writable = true, executable = false)` typechecks.

  **QA Scenarios**:

  ```
  Scenario: Perms compile
    Tool: Bash
    Steps:
      1. Generate /tmp/perms_all.op exercising each of 4 fns.
      2. opal check
    Expected Result: exit 0.
    Evidence: .sisyphus/evidence/fs-infra/t10-all.log

  Scenario: FilePermissions record constructor
    Tool: Bash
    Steps:
      1. Generate .op constructing `new FilePermissions: readable = true, writable = false, executable = false` and passing to set_permissions_sync.
      2. opal check
    Expected Result: exit 0.
    Evidence: .sisyphus/evidence/fs-infra/t10-record.log
  ```

  **Commit**: YES — `feat(stdlib): wire permissions infrastructure`

- [ ] 11. TDD: Path Manipulation behavior (red-green-refactor per fn)

  **What to do**:
  - For each of the 7 path-manipulation fns, follow red-green-refactor:
    1. **RED**: Add a Rust integration test under `tests/integration_e2e/fs_path_manipulation.rs::fn_<name>_behavior` that compiles a minimal `.op` program calling the fn and asserting specific output. Test MUST fail against current stub.
    2. **GREEN**: Implement the C body in `opal_fs.c` with minimum code to pass.
    3. **REFACTOR**: Extract common path helpers if duplication appears (e.g., shared `op_path_normalize_in_place`).
  - Behavioral specs:
    - `path_from('/a/b/c.txt')` → FilesystemPath with raw='/a/b/c.txt'
    - `join_path_components(path_from('/a'), ['b', 'c.txt'])` → /a/b/c.txt (platform-appropriate separator — but raw stays POSIX-style on Linux)
    - `path_parent_directory(path_from('/a/b/c.txt'))` → /a/b ; `path_parent_directory(path_from('/'))` → /
    - `path_file_name(path_from('/a/b/c.txt'))` → 'c.txt' ; `path_file_name(path_from('/a/b/'))` → 'b'
    - `path_file_extension(path_from('/a/b.txt'))` → 'txt' ; `path_file_extension(path_from('/a/b'))` → ''
    - `normalize_path(path_from('/a/./b/../c'))` → /a/c ; LEXICAL ONLY, does not access filesystem
    - `absolute_path_sync(path_from('relative/x'))` → cwd-prefixed absolute path

  **Must NOT do**:
  - Do NOT make any of the first 6 functions touch the filesystem.
  - Do NOT introduce Unicode normalization (byte-preserving).

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`.
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES (with T12–T16).
  - **Blocked By**: T5.
  - **Blocks**: T17.

  **References**:
  - POSIX path rules for edge cases (root, trailing slash).
  - Proposal semantics.

  **Acceptance Criteria**:
  - [ ] `cargo test --features integration fs_path_manipulation::` → 7 tests, all PASS.
  - [ ] Each test asserts at least one non-trivial edge case (empty, trailing slash, dot, dot-dot).
  - [ ] REFACTOR pass completed (evidence: commit or inline code comment referencing extraction).

  **QA Scenarios**:

  ```
  Scenario: All 7 path-helper tests pass
    Tool: Bash
    Steps:
      1. cargo test --features integration fs_path_manipulation:: 2>&1 | tee .sisyphus/evidence/fs-tdd/t11-results.log
    Expected Result: "test result: ok. 7 passed".
    Evidence: .sisyphus/evidence/fs-tdd/t11-results.log

  Scenario: RED phase was genuinely red (history check)
    Tool: Bash
    Steps:
      1. git log --oneline -n 20 | grep -E 'TDD.*path' | wc -l
    Expected Result: ≥ 1 commit showing RED→GREEN progression.
    Evidence: .sisyphus/evidence/fs-tdd/t11-history.log
  ```

  **Commit**: YES — `feat(stdlib): implement path-manipulation builtins with TDD`

- [ ] 12. TDD: File Reading behavior (red-green-refactor per fn)

  **What to do**:
  - For each of 4 reading fns, red-green-refactor:
    - `read_contents_sync`: read whole file as Bytes; RED test reads a fixture with known SHA-256.
    - `read_text_sync`: UTF-8 strict; RED test has two cases — valid UTF-8 and invalid bytes → `InvalidUtf8Error`.
    - `read_lines_sync`: split on `\n`, strip single trailing `\r`; RED test has 4 cases — `\n`-only, `\r\n`, mixed, trailing newline handling.
    - `read_bytes_at_offset_sync`: seek+read; RED test covers happy path, offset-past-EOF → `OffsetOutOfRangeError`.
  - Fixtures live under `tests/fixtures/fs_reading/` (created here) and are READ-ONLY — never mutated. Use `StateGuard` to verify no mutation anyway.
  - Implement bodies in `opal_fs.c` using `fopen`/`fread`/`fstat`. Handle errno→error mapping (ENOENT → FileNotFoundError, EACCES → PermissionDeniedError, EISDIR → IsADirectoryError, etc.).

  **Must NOT do**:
  - Do NOT use BOM stripping.
  - Do NOT coerce invalid UTF-8 silently; ALWAYS return InvalidUtf8Error.

  **Recommended Agent Profile**:
  - **Category**: `deep` — UTF-8 strictness + line-ending rules need careful boundary handling.
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES (with T11, T13–T16).
  - **Blocked By**: T6.
  - **Blocks**: T18.

  **References**:
  - User Q&A: "UTF-8 strict" + "split on \n, strip trailing \r".
  - POSIX errno list.

  **Acceptance Criteria**:
  - [ ] `cargo test --features integration fs_reading::` → all PASS (≥ 4 happy + 4 error scenarios).
  - [ ] `read_lines_sync` CRLF+LF round-trip test passes (file saved with CRLF reads back same line-set as LF-only version).
  - [ ] `read_text_sync` on invalid UTF-8 returns InvalidUtf8Error specifically (asserted via `error_cstr` match).

  **QA Scenarios**:

  ```
  Scenario: Happy paths + error paths
    Tool: Bash
    Steps:
      1. cargo test --features integration fs_reading:: 2>&1 | tee .sisyphus/evidence/fs-tdd/t12-results.log
    Expected Result: all tests PASS.
    Evidence: .sisyphus/evidence/fs-tdd/t12-results.log

  Scenario: Read-only fixtures unchanged
    Tool: Bash
    Steps:
      1. sha256sum tests/fixtures/fs_reading/*.* > /tmp/fs_reading_before.txt
      2. cargo test --features integration fs_reading::
      3. sha256sum tests/fixtures/fs_reading/*.* > /tmp/fs_reading_after.txt
      4. diff /tmp/fs_reading_before.txt /tmp/fs_reading_after.txt
    Expected Result: empty diff.
    Evidence: .sisyphus/evidence/fs-tdd/t12-fixtures-unchanged.log
  ```

  **Commit**: YES — `feat(stdlib): implement file-reading builtins with TDD`

- [ ] 13. TDD: File Writing behavior (red-green-refactor, incl. atomic temp+rename)

  **What to do**:
  - For each of 7 writing fns, red-green-refactor. ALL tests use `tempfile::tempdir()` so writes never touch the source tree outside `StateGuard`-protected test project dirs.
  - Behavioral specs:
    - `write_contents_sync`: whole-file overwrite; RED test reads back and compares.
    - `write_text_sync`: UTF-8 text (no encoding transform — input is already `string`).
    - `write_contents_atomic_sync`: write to `<path>.tmp.<pid>`, fsync, rename. RED test kills process mid-write and asserts no partial file visible at target path (use a test-only synchronization hook or asserting the tmp path doesn't leak on success).
    - `write_text_atomic_sync`: same, for text.
    - `append_contents_sync`: append bytes; RED test verifies existing content preserved + new bytes at end.
    - `append_text_sync`: same, text.
    - `write_bytes_at_offset_sync`: seek+write at offset; RED test verifies surrounding bytes unchanged. Offset past EOF → OffsetOutOfRangeError (no auto-extend).
  - Implement in `opal_fs.c` using `fopen("wb"/"ab"/"rb+")`, `fwrite`, `fsync`, `rename(2)`.
  - Error mapping: ENOSPC → FilesystemFullError, EACCES → PermissionDeniedError, EISDIR → IsADirectoryError.

  **Must NOT do**:
  - Do NOT create parent dirs automatically.
  - Do NOT leave tmp files on rename failure (cleanup before returning error).
  - Do NOT use O_DIRECT or any platform-specific flags.

  **Recommended Agent Profile**:
  - **Category**: `deep` — atomic semantics require fsync discipline + cleanup on every error path.
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES (with T11, T12, T14–T16).
  - **Blocked By**: T7.
  - **Blocks**: T18.

  **References**:
  - POSIX `rename(2)`, `fsync(2)` semantics.
  - User Q&A: "both naive + atomic".

  **Acceptance Criteria**:
  - [ ] `cargo test --features integration fs_writing::` → all PASS (≥ 7 happy + 7 error scenarios).
  - [ ] Atomic variant: no tmp file visible at target path after successful call.
  - [ ] Atomic variant: on simulated rename failure (inject by pointing target at a read-only dir), tmp file cleaned up and correct error returned.
  - [ ] Append: prior bytes preserved byte-exact.

  **QA Scenarios**:

  ```
  Scenario: All 7 writers behavior
    Tool: Bash
    Steps:
      1. cargo test --features integration fs_writing:: 2>&1 | tee .sisyphus/evidence/fs-tdd/t13-results.log
    Expected Result: all PASS.
    Evidence: .sisyphus/evidence/fs-tdd/t13-results.log

  Scenario: Atomic cleanup on failure
    Tool: Bash
    Steps:
      1. Generate .op that calls `write_contents_atomic_sync` into a read-only dir.
      2. After run, check tempdir listing.
    Expected Result: no .tmp.* residue, error is PermissionDeniedError or WriteFailureError.
    Evidence: .sisyphus/evidence/fs-tdd/t13-atomic-cleanup.log
  ```

  **Commit**: YES — `feat(stdlib): implement file-writing builtins with TDD (incl. atomic)`

- [ ] 14. TDD: File Management behavior (red-green-refactor, incl. nofollow metadata)

  **What to do**:
  - For each of 7 file-mgmt fns, red-green-refactor:
    - `create_file_sync`: `open(O_CREAT | O_EXCL)`; exists → FileAlreadyExistsError.
    - `delete_file_sync`: `unlink(2)`; dir → IsADirectoryError.
    - `copy_file_sync`: read + write loop (no sendfile-specific paths). Destination exists → FileAlreadyExistsError (don't overwrite silently); destination dir → IsADirectoryError.
    - `move_path_sync`: `rename(2)`; cross-device → fallback to copy+delete with atomicity consideration.
    - `path_exists_sync`: `access(F_OK)` → boolean; EACCES → PermissionDeniedError (distinguish from false).
    - `read_metadata_sync`: `stat(2)` — follows symlinks; populate FileMetadata fields.
    - `read_metadata_nofollow_sync`: `lstat(2)` — does NOT follow. RED test: create symlink to non-existent target → `read_metadata_sync` errors FileNotFoundError, `read_metadata_nofollow_sync` succeeds with `is_symlink=true`.
  - All tests use `tempfile::tempdir()` — zero source-tree mutation.

  **Must NOT do**:
  - Do NOT silently overwrite destinations in copy/move.
  - Do NOT fall back to non-atomic move without clear documentation in the C comment.

  **Recommended Agent Profile**:
  - **Category**: `deep`.
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES (with T11–T13, T15, T16).
  - **Blocked By**: T8.
  - **Blocks**: T18, T19.

  **References**:
  - POSIX stat/lstat semantics.
  - User Q&A: "metadata and inspection have two variants (follow and don't follow)".

  **Acceptance Criteria**:
  - [ ] `cargo test --features integration fs_management::` → all PASS (≥ 7 happy + 7 error).
  - [ ] Symlink test case: follow vs nofollow behavior distinguishable.
  - [ ] Copy into existing file → FileAlreadyExistsError (asserted).

  **QA Scenarios**:

  ```
  Scenario: All 7 mgmt fns
    Tool: Bash
    Steps:
      1. cargo test --features integration fs_management:: 2>&1 | tee .sisyphus/evidence/fs-tdd/t14-results.log
    Expected Result: all PASS.
    Evidence: .sisyphus/evidence/fs-tdd/t14-results.log

  Scenario: Symlink follow vs nofollow
    Tool: Bash
    Steps:
      1. In tempdir: `ln -s /nonexistent /tmp/.../broken_link`
      2. Run .op calling read_metadata_sync(broken_link) → expect FileNotFoundError.
      3. Run .op calling read_metadata_nofollow_sync(broken_link) → expect success, is_symlink=true.
    Expected Result: both assertions pass.
    Evidence: .sisyphus/evidence/fs-tdd/t14-symlink.log
  ```

  **Commit**: YES — `feat(stdlib): implement file-management builtins with TDD`

- [ ] 15. TDD: Directory Operations behavior (red-green-refactor)

  **What to do**:
  - For each of 9 dir-ops fns, red-green-refactor in tempdirs:
    - `create_directory_sync`: `mkdir(2)`; exists → FileAlreadyExistsError; missing parent → CreateFailureError.
    - `create_directory_recursive_sync`: emulate `mkdir -p`; exists-as-dir is OK (idempotent); exists-as-file → CreateFailureError.
    - `delete_directory_sync`: `rmdir(2)`; non-empty → DirectoryNotEmptyError; not a dir → IsNotADirectoryError.
    - `delete_directory_recursive_sync`: walk + unlink/rmdir.
    - `list_directory_sync`: `opendir/readdir`; return FilesystemPath[] with full paths (path = parent joined with entry name); skip `.` and `..`; ordering: sorted lexicographically (document decision in `stdlib/prelude.op`).
    - `is_file_sync` / `is_file_nofollow_sync`: stat/lstat then S_ISREG.
    - `is_directory_sync` / `is_directory_nofollow_sync`: stat/lstat then S_ISDIR.
  - RED tests assert each specific error variant, empty dir listing, ordering guarantee, symlink-follow distinction.

  **Must NOT do**:
  - Do NOT return entries with just the basename (return full-path FilesystemPath so callers don't reconstruct).
  - Do NOT include `.` or `..` in listings.
  - Do NOT leave partially-deleted state on recursive-delete failure (document best-effort semantics if unavoidable).

  **Recommended Agent Profile**:
  - **Category**: `deep` — recursive delete + ordering guarantees require care.
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES (with T11–T14, T16).
  - **Blocked By**: T9.
  - **Blocks**: T19.

  **References**:
  - POSIX readdir, mkdir, rmdir, unlink, S_ISREG, S_ISDIR, S_ISLNK.

  **Acceptance Criteria**:
  - [ ] `cargo test --features integration fs_directory::` → all PASS (≥ 9 happy + relevant errors).
  - [ ] `list_directory_sync` returns sorted FilesystemPath[] with full paths and no `./..`.
  - [ ] `delete_directory_recursive_sync` on nested tree with 3 levels passes.
  - [ ] `is_file_sync` vs `is_file_nofollow_sync` on a symlink-to-file returns different values when target is also a file vs when target is a dir.

  **QA Scenarios**:

  ```
  Scenario: Dir ops all behavior
    Tool: Bash
    Steps:
      1. cargo test --features integration fs_directory:: 2>&1 | tee .sisyphus/evidence/fs-tdd/t15-results.log
    Expected Result: all PASS.
    Evidence: .sisyphus/evidence/fs-tdd/t15-results.log

  Scenario: list_directory_sync ordering
    Tool: Bash
    Steps:
      1. In tempdir create files b.txt, a.txt, c.txt.
      2. Run .op that prints each entry from list_directory_sync.
    Expected Result: printed order = a.txt, b.txt, c.txt.
    Evidence: .sisyphus/evidence/fs-tdd/t15-ordering.log
  ```

  **Commit**: YES — `feat(stdlib): implement directory-operations builtins with TDD`

- [ ] 16. TDD: Permissions behavior (red-green-refactor)

  **What to do**:
  - For each of 4 perms fns, red-green-refactor in tempdirs:
    - `can_read_sync`: use `access(R_OK)` → boolean. Dedicated "file exists but mode is 000" test → returns false.
    - `can_write_sync`: `access(W_OK)` → boolean. Note Linux-only; Windows parity is follow-up.
    - `can_execute_sync`: `access(X_OK)` → boolean. On a file chmod +x → true; without +x → false.
    - `set_permissions_sync`: translate abstract triple `{readable, writable, executable}` to a portable mode set: owner rwx bits, group preserved, others preserved (on Linux use chmod; mapping is read→0400, write→0200, execute→0100 for owner only). Document this mapping in `stdlib/prelude.op`.
  - Test on fixtures created per-test via `tempfile::tempdir()`; tear down via `StateGuard`.

  **Must NOT do**:
  - Do NOT expose octal modes in the public API.
  - Do NOT assume specific umask; be deterministic regardless.

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`.
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES (with T11–T15).
  - **Blocked By**: T10.
  - **Blocks**: T19.

  **References**:
  - POSIX access(2), chmod(2).
  - User Q&A: abstract triple.

  **Acceptance Criteria**:
  - [ ] `cargo test --features integration fs_permissions::` → all PASS (≥ 4 happy + error paths).
  - [ ] `set_permissions_sync` actually changes mode verified by post-call stat.
  - [ ] Re-reading via `can_*_sync` after `set_permissions_sync` reflects the new state.

  **QA Scenarios**:

  ```
  Scenario: Perms round-trip
    Tool: Bash
    Steps:
      1. cargo test --features integration fs_permissions::round_trip
    Expected Result: PASS.
    Evidence: .sisyphus/evidence/fs-tdd/t16-roundtrip.log

  Scenario: set_permissions persists via stat
    Tool: Bash
    Steps:
      1. .op sets `{readable=true, writable=false, executable=false}` on tmpfile.
      2. After: `stat -c '%a' tmpfile` → expect 400.
    Expected Result: matches.
    Evidence: .sisyphus/evidence/fs-tdd/t16-stat.log
  ```

  **Commit**: YES — `feat(stdlib): implement permissions builtins with TDD`

- [ ] 17. Test project: fs-path-manipulation + Rust integration test

  **What to do**:
  - Create `test-projects/fs-path-manipulation/` with `opal.toml`, `src/main.op`, `.gitignore`, `README.md`.
  - Program demonstrates ALL 7 path-manipulation fns end-to-end: constructs paths from strings, joins, normalizes, extracts extensions/parents/names, computes absolute paths — ALL LEXICAL (no fs reads/writes).
  - Write `tests/integration_e2e/fs_path_manipulation.rs` that:
    - Uses `StateGuard::new(project_dir)` snapshotting the project directory at start.
    - Runs `compile_program` + `Command::new(binary).output()`.
    - Asserts exact stdout (deterministic since pure).
    - Asserts post-state manifest matches pre-state (no drift).
    - On Drop, StateGuard restores any snapshotted files and removes any accidentally created paths.
  - Add to `tests/integration_e2e/tests.rs` module list.

  **Must NOT do**:
  - Do NOT perform any fs mutation in this project.
  - Do NOT depend on absolute CWD (compute cwd at runtime and include in expected stdout OR normalize before asserting).

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`.
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES (with T18–T22).
  - **Blocked By**: T11.
  - **Blocks**: F1–F5.

  **References**:
  - `test-projects/bytes-hex-roundtrip/` — closest template.
  - `tests/integration_e2e/bytes_stdlib.rs` — Rust harness pattern.
  - `tests/integration_print.rs:10-23` — prepare_dir/cleanup_dir helpers (still useful for base scaffolding).

  **Acceptance Criteria**:
  - [ ] `cargo test --features integration fs_path_manipulation_compiles_links_and_runs` → PASS.
  - [ ] Running the test 3 consecutive times leaves project dir sha256-identical.
  - [ ] `.sisyphus/evidence/fs-path-manipulation/happy/` has stdout, exit-code, pre-state, post-state, diff (empty), cleanup-marker.

  **QA Scenarios**:

  ```
  Scenario: Project runs deterministically
    Tool: Bash
    Steps:
      1. sha256sum test-projects/fs-path-manipulation/src/*.op > /tmp/before.txt
      2. for i in 1 2 3; do cargo test --features integration fs_path_manipulation_compiles_links_and_runs; done
      3. sha256sum test-projects/fs-path-manipulation/src/*.op > /tmp/after.txt
      4. diff /tmp/before.txt /tmp/after.txt
    Expected Result: empty diff (project dir unchanged).
    Evidence: .sisyphus/evidence/fs-path-manipulation/t17-3x/{before.txt,after.txt,diff.txt}

  Scenario: Stdout matches golden
    Tool: Bash
    Steps:
      1. cargo test --features integration fs_path_manipulation_compiles_links_and_runs -- --nocapture 2>&1 | tee /tmp/out.log
    Expected Result: contains expected path-helper output (exact list in test source).
    Evidence: .sisyphus/evidence/fs-path-manipulation/t17-stdout.log
  ```

  **Commit**: YES — `test(stdlib): fs-path-manipulation integration test`

- [ ] 18. Test project: fs-markdown-roundtrip + StateGuard RAII + Rust integration test

  **What to do**:
  - Create `test-projects/fs-markdown-roundtrip/` with:
    - `opal.toml`
    - `src/main.op` demonstrating: `read_text_sync` → transform (e.g., prepend a heading to each paragraph) → `write_text_atomic_sync` back → `read_lines_sync` → assert round-trip equality → restore original via `write_text_atomic_sync` → `read_metadata_sync` → `read_contents_sync`/`write_contents_sync` byte round-trip → UTF-8 invalid-byte detection via `read_text_sync` on fixture `docs/invalid-utf8.bin` (expect `InvalidUtf8Error`).
    - `docs/README.md`, `docs/notes.md`, `docs/invalid-utf8.bin` — committed fixtures, NEVER mutated long-term.
    - `.gitignore`, `README.md` explaining the workflow.
  - Create `tests/integration_e2e/fs_markdown_roundtrip.rs` with:
    - `struct StateGuard` capturing sha256 manifest of entire `test-projects/fs-markdown-roundtrip/docs/` at construction.
    - `impl Drop for StateGuard { fn drop(&mut self) { /* read current manifest; for each changed path, restore original bytes; remove any created paths; write cleanup-marker.txt */ } }`
    - Test body: `let _guard = StateGuard::new(...); compile_program; run binary; assert exit 0; assert stdout matches expected; after guard drops, assert manifest identical to pre-state`.
    - Use `tempfile::TempDir` ONLY for build artifacts; source-tree fixtures are restored in-place via StateGuard.
  - Add module to `tests/integration_e2e/tests.rs`.

  **Must NOT do**:
  - Do NOT commit any post-mutation state to git.
  - Do NOT use `naive` `write_text_sync` in the round-trip — atomic required so a killed test leaves no partial files.
  - Do NOT let the StateGuard's Drop panic on failure — log to stderr and continue cleanup for remaining files.

  **Recommended Agent Profile**:
  - **Category**: `deep` — RAII + sha256 manifest logic must be correct on error paths.
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES (with T17, T19–T22).
  - **Blocked By**: T12, T13, T14 (reads + writes + metadata must be implemented).
  - **Blocks**: F1–F5.

  **References**:
  - `test-projects/bytes-hex-roundtrip/` — closest template for fixtures-based round-trip.
  - `tests/integration_e2e/bytes_stdlib.rs` — compile_program + Command harness.
  - User directive: "state returned to the original file on-success... test suites should clean up the files each time afterwards".

  **Acceptance Criteria**:
  - [ ] `cargo test --features integration fs_markdown_roundtrip_compiles_links_and_runs` → PASS.
  - [ ] Running test 5 consecutive times: `docs/` sha256 manifest byte-identical before run 1 and after run 5.
  - [ ] Killing the test mid-run (simulated by `panic!` injection in a feature-gated branch) still leaves `docs/` restored on Drop.
  - [ ] `InvalidUtf8Error` asserted when program processes `docs/invalid-utf8.bin`.
  - [ ] Evidence artifacts present under `.sisyphus/evidence/fs-markdown-roundtrip/`.

  **QA Scenarios**:

  ```
  Scenario: 5x repeat leaves project byte-identical
    Tool: Bash
    Steps:
      1. sha256sum -c <(find test-projects/fs-markdown-roundtrip/docs -type f | xargs sha256sum) > /tmp/before.txt || true
      2. find test-projects/fs-markdown-roundtrip/docs -type f -exec sha256sum {} + > /tmp/before.txt
      3. for i in 1 2 3 4 5; do cargo test --features integration fs_markdown_roundtrip_compiles_links_and_runs; done
      4. find test-projects/fs-markdown-roundtrip/docs -type f -exec sha256sum {} + > /tmp/after.txt
      5. diff /tmp/before.txt /tmp/after.txt
    Expected Result: empty diff.
    Failure Indicators: non-empty diff indicates StateGuard failed to restore.
    Evidence: .sisyphus/evidence/fs-markdown-roundtrip/repeat-5x/{before.txt,after.txt,diff.txt}

  Scenario: StateGuard restores after simulated panic
    Tool: Bash
    Steps:
      1. Enable feature `panic_mid_run` in test.
      2. cargo test --features integration,panic_mid_run fs_markdown_roundtrip_panics_and_cleans || true
      3. find test-projects/fs-markdown-roundtrip/docs -type f -exec sha256sum {} + > /tmp/after-panic.txt
      4. diff /tmp/before.txt /tmp/after-panic.txt
    Expected Result: empty diff — panic triggered but StateGuard.drop() restored everything.
    Evidence: .sisyphus/evidence/fs-markdown-roundtrip/panic/{stderr.log,after-panic.txt,diff.txt}

  Scenario: Invalid UTF-8 surfaces correct error
    Tool: Bash
    Steps:
      1. cargo test --features integration fs_markdown_roundtrip_invalid_utf8 -- --nocapture 2>&1 | tee /tmp/utf8.log
    Expected Result: stdout or program output contains "InvalidUtf8Error".
    Evidence: .sisyphus/evidence/fs-markdown-roundtrip/invalid-utf8.log
  ```

  **Commit**: YES — `test(stdlib): fs-markdown-roundtrip project + StateGuard RAII harness`

- [ ] 19. Test project: fs-directory-operations + RAII + Rust integration test

  **What to do**:
  - Create `test-projects/fs-directory-operations/` with:
    - `opal.toml`
    - `src/main.op` demonstrating: `create_directory_recursive_sync('workspace/a/b/c')`, `create_file_sync`, `list_directory_sync` (assert sorted order), `is_file_sync` / `is_directory_sync`, `is_file_nofollow_sync` / `is_directory_nofollow_sync` on a symlink created at startup, `copy_file_sync`, `move_path_sync`, `set_permissions_sync` with abstract triple, `can_read_sync`/`can_write_sync`/`can_execute_sync`, `delete_file_sync`, `delete_directory_recursive_sync` — all within `workspace/` (a subdir created fresh per run).
    - `workspace/.gitkeep` (the only committed file under workspace; everything else is created and cleaned).
    - `.gitignore` excluding `workspace/*` except `.gitkeep`.
    - `README.md` explaining that `workspace/` starts empty every run.
  - Create `tests/integration_e2e/fs_directory_operations.rs`:
    - StateGuard captures the fact that `workspace/` contains only `.gitkeep` at start.
    - On Drop, recursively deletes anything in `workspace/` other than `.gitkeep` and writes `cleanup-marker.txt`.
    - Test body: construct guard, compile + run program, assert success + expected stdout, Drop verifies workspace is pristine.
  - Add to `tests/integration_e2e/tests.rs`.

  **Must NOT do**:
  - Do NOT manipulate files outside `test-projects/fs-directory-operations/workspace/`.
  - Do NOT rely on system temp dirs — the project's own `workspace/` subdir IS the sandbox per user directive.
  - Do NOT leave symlinks on disk between runs.

  **Recommended Agent Profile**:
  - **Category**: `deep` — symlink + permissions + recursive delete paths all interact.
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES (with T17, T18, T20–T22).
  - **Blocked By**: T14, T15, T16.
  - **Blocks**: F1–F5.

  **References**:
  - User directive: "manipulating markdown files in their directory" — this project's analog is `workspace/`.
  - `tests/integration_print.rs:10-23` — prepare_dir/cleanup_dir pattern.

  **Acceptance Criteria**:
  - [ ] `cargo test --features integration fs_directory_operations_compiles_links_and_runs` → PASS.
  - [ ] After 5 consecutive runs, `workspace/` contains only `.gitkeep` (sha256 identical).
  - [ ] `list_directory_sync` output in stdout is lexicographically sorted.
  - [ ] Symlink nofollow-vs-follow distinction exercised and asserted.
  - [ ] `can_execute_sync` returns true after `set_permissions_sync(…, executable=true)` on a regular file.

  **QA Scenarios**:

  ```
  Scenario: 5x repeat, workspace stays pristine
    Tool: Bash
    Steps:
      1. ls -la test-projects/fs-directory-operations/workspace/ > /tmp/ws-before.txt
      2. for i in 1 2 3 4 5; do cargo test --features integration fs_directory_operations_compiles_links_and_runs; done
      3. ls -la test-projects/fs-directory-operations/workspace/ > /tmp/ws-after.txt
      4. diff /tmp/ws-before.txt /tmp/ws-after.txt
    Expected Result: empty diff — only .gitkeep present.
    Evidence: .sisyphus/evidence/fs-directory-operations/repeat-5x/{before.txt,after.txt,diff.txt}

  Scenario: Symlink nofollow distinction
    Tool: Bash
    Steps:
      1. cargo test --features integration fs_directory_operations_symlink_variants -- --nocapture 2>&1 | tee /tmp/sl.log
    Expected Result: contains assertions showing is_file_sync=true AND is_file_nofollow_sync=false for a symlink pointing to a file.
    Evidence: .sisyphus/evidence/fs-directory-operations/symlink.log

  Scenario: Sorted listing
    Tool: Bash
    Steps:
      1. cargo test --features integration fs_directory_operations_list_sorted -- --nocapture 2>&1 | tee /tmp/sort.log
    Expected Result: program output shows entries in lexicographic order.
    Evidence: .sisyphus/evidence/fs-directory-operations/sort.log
  ```

  **Commit**: YES — `test(stdlib): fs-directory-operations project + workspace sandbox RAII`

- [ ] 20. Prelude docs: stdlib/prelude.op fs module

  **What to do**:
  - Update `stdlib/prelude.op` adding a new `fs` section documenting ALL 38 functions with:
    - Signature (exactly as implemented).
    - One-line purpose.
    - Error list.
    - Symlink semantics note for functions where it applies (follow vs nofollow).
    - UTF-8 / line-ending / atomicity note for text/content functions.
    - Permissions mapping note (`readable→0400, writable→0200, executable→0100` for owner; group/others preserved).
    - `list_directory_sync` ordering guarantee (lexicographic, `.`/`..` excluded, full-path entries).
  - Group by category with ASCII headers matching T5–T10 batches.

  **Must NOT do**:
  - Do NOT change any signature — this task DOCUMENTS only.
  - Do NOT re-export or rename; names are locked.

  **Recommended Agent Profile**:
  - **Category**: `writing`.
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES (with T17–T19, T21, T22).
  - **Blocked By**: T11–T16 (signatures confirmed by implementation).
  - **Blocks**: F1 (compliance audit reads prelude).

  **References**:
  - `stdlib/prelude.op` — existing format.
  - Proposal.

  **Acceptance Criteria**:
  - [ ] All 38 functions documented.
  - [ ] All 3 nominal types documented with fields.
  - [ ] All 20 error types listed with which fns raise them.
  - [ ] `opal check` on a .op file that `import`s everything from the fs module still typechecks.

  **QA Scenarios**:

  ```
  Scenario: Prelude coverage
    Tool: Bash
    Steps:
      1. grep -c '^let [a-z_]*_sync\|^let path_from\|^let join_path_components\|^let normalize_path\|^let absolute_path_sync' stdlib/prelude.op
    Expected Result: ≥ 38.
    Evidence: .sisyphus/evidence/prelude/t20-count.log

  Scenario: All errors referenced
    Tool: Bash
    Steps:
      1. for E in FileNotFoundError PermissionDeniedError FileAlreadyExistsError ReadFailureError WriteFailureError InvalidPathError FilesystemFullError IsADirectoryError IsNotADirectoryError DirectoryNotEmptyError DirectoryNotFoundError MetadataUnavailableError OffsetOutOfRangeError LineOutOfRangeError CopyFailureError MoveFailureError DeleteFailureError CreateFailureError SetPermissionsError InvalidUtf8Error; do grep -q $E stdlib/prelude.op || echo MISSING $E; done
    Expected Result: no "MISSING" lines.
    Evidence: .sisyphus/evidence/prelude/t20-errors.log
  ```

  **Commit**: YES — `docs(stdlib): document fs module in prelude`

- [ ] 21. README: "Building Opalescent Programs for Windows" subsection

  **What to do**:
  - Edit `README.md` adding new subsection under the existing Windows build section (which currently only covers building the compiler, not user programs for Windows).
  - Cover:
    - Prereqs: `rustup target add x86_64-pc-windows-msvc` + `cargo install cargo-xwin` (cross-compile from Linux).
    - Command: `opal build --target x86_64-pc-windows-msvc` (note: if flag not yet wired through CLI, document the env-var fallback `OPAL_TARGET=x86_64-pc-windows-msvc opal build`).
    - Expected output path.
    - Running on Windows host (artifact copy).
    - Known limitation: fs stdlib is Linux-only for now; see `.sisyphus/followups.md` for Windows parity tracking.
  - Keep existing compiler-for-Windows subsection intact.

  **Must NOT do**:
  - Do NOT duplicate the compiler-build instructions.
  - Do NOT promise MSI/installer support.
  - Do NOT claim Windows parity for fs stdlib before T22 followup is delivered.

  **Recommended Agent Profile**:
  - **Category**: `writing`.
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES (with T17–T20, T22).
  - **Blocked By**: None (docs only).
  - **Blocks**: F1.

  **References**:
  - `README.md` — existing "Native Windows Build (MSVC)" and "Cross-compilation from Linux (MSVC Target)" sections.

  **Acceptance Criteria**:
  - [ ] New subsection heading `## Building Opalescent Programs for Windows` (or analogous level) present.
  - [ ] Command example shown with exact flag.
  - [ ] Linux-only fs-stdlib caveat present with link to `.sisyphus/followups.md`.

  **QA Scenarios**:

  ```
  Scenario: README contains subsection
    Tool: Bash
    Steps:
      1. grep -n 'Building Opalescent Programs for Windows' README.md
    Expected Result: exactly one hit.
    Evidence: .sisyphus/evidence/readme/t21-heading.log

  Scenario: Caveat linked
    Tool: Bash
    Steps:
      1. grep -n 'followups.md' README.md
    Expected Result: ≥ 1 hit in the new subsection.
    Evidence: .sisyphus/evidence/readme/t21-followup-link.log
  ```

  **Commit**: YES — `docs(readme): add Building Opalescent Programs for Windows subsection`

- [ ] 22. Followups: .sisyphus/followups.md entry for Windows fs parity

  **What to do**:
  - Create (or append to) `.sisyphus/followups.md` with a section:
    - Title: "Windows fs stdlib parity".
    - Rationale: Linux-only per user directive in this planning cycle.
    - Scope of parity work: permissions mapping (ACL-based rather than mode bits), symlinks (require dev-mode or admin), atomic rename semantics (ReplaceFile), path separators (backslash normalization policy), `executable` determination by extension (`.exe`, `.bat`, `.cmd`, `.ps1`).
    - Test strategy: mirror T17–T19 but gated behind `#[cfg(windows)]` + CI matrix.
    - Dependencies to investigate: `windows-rs` vs direct Win32 calls in `runtime/opal_fs_windows.c`.

  **Must NOT do**:
  - Do NOT begin implementation here — it's a tracking entry only.

  **Recommended Agent Profile**:
  - **Category**: `writing`.
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES (with T17–T21).
  - **Blocked By**: None.
  - **Blocks**: F1.

  **References**:
  - User Q&A: "Linux-only tests now; Windows parity as documented follow-up".

  **Acceptance Criteria**:
  - [ ] `.sisyphus/followups.md` contains the "Windows fs stdlib parity" heading.
  - [ ] All 5 parity considerations enumerated.
  - [ ] README T21 caveat links here.

  **QA Scenarios**:

  ```
  Scenario: Followups entry present
    Tool: Bash
    Steps:
      1. grep -n 'Windows fs stdlib parity' .sisyphus/followups.md
    Expected Result: ≥ 1 hit.
    Evidence: .sisyphus/evidence/followups/t22-heading.log

  Scenario: Cross-link from README
    Tool: Bash
    Steps:
      1. grep -n 'Windows fs stdlib parity\|followups.md' README.md
    Expected Result: ≥ 1 hit.
    Evidence: .sisyphus/evidence/followups/t22-cross-link.log
  ```

  **Commit**: YES — `docs(followups): track Windows fs stdlib parity`

---

## Final Verification Wave

- [ ] F1. **Plan Compliance Audit** — `oracle`
  Read this plan end-to-end. For each "Must Have": verify implementation exists (grep source, compile, run). For each "Must NOT Have": search codebase for forbidden patterns — reject with file:line if found (e.g., `rg 'file_'` in new code, `rg 'uint8\[\]'` in fs_builtins, `rg 'async|stream|mmap' runtime/opal_fs.c`). Check evidence files exist in `.sisyphus/evidence/fs-*/`. Confirm 38 functions registered, 3 types, 20 errors.
  Output: `Must Have [N/N] | Must NOT Have [N/N] | Fns [38/38] | Types [3/3] | Errors [20/20] | VERDICT: APPROVE/REJECT`

- [ ] F2. **Code Quality Review** — `unspecified-high`
  Run `cargo build --release` + `cargo test` + `cargo test --features integration fs_` + `cargo clippy -- -D warnings` (if configured). Review `runtime/opal_fs.c` and `src/type_system/checker/fs_builtins.rs` for: `as any`, `unsafe` blocks without justification comment, unused imports, commented-out code, excessive comments, generic names (data/result/item/temp). Check for memory leaks in opal_fs.c (every malloc has a matching free or ownership transfer to caller).
  Output: `Build [PASS/FAIL] | Tests [N pass/N fail] | Files [N clean/N issues] | Memory [N leaks] | VERDICT`

- [ ] F3. **Real Manual QA** — `unspecified-high`
  Start from clean `git status` state. Execute every QA scenario from every task. For each test project: (a) capture pre-state sha256 manifest, (b) run `cargo test --features integration <name>` 3× in a row, (c) after each run verify post-state manifest matches pre-state (diff empty), (d) verify `cleanup-marker.txt` exists with fresh timestamp. Save all evidence to `.sisyphus/evidence/final-qa/`.
  Output: `Scenarios [N/N pass] | State-Restoration [N/N projects clean after 3 runs] | Cleanup-Markers [N/N fresh] | VERDICT`

- [ ] F4. **Scope Fidelity Check** — `deep`
  For each task T0–T22: read "What to do", read actual diff (`git diff` from plan start). Verify 1:1 — everything in spec built, nothing beyond spec built. Check "Must NOT do" compliance (no `file_` prefix, no `uint8[]`, no async/streaming, no refactor of existing stdlib). Detect cross-task contamination. Flag unaccounted changes.
  Output: `Tasks [23/23 compliant] | Contamination [CLEAN/N issues] | Unaccounted [CLEAN/N files] | VERDICT`

- [ ] F5. **Momus Plan-Critic Review (MANDATORY)**
  Invoke Momus agent with the plan file path as the prompt (`.sisyphus/plans/file-io-stdlib-path-object-centric.md`). Loop until Momus returns `OKAY`: on rejection, fix every issue Momus raises, regenerate/edit the plan, resubmit. No maximum retry limit. Per user: "you do need to invoke Momus at the end."
  Output: `Momus VERDICT: OKAY` (only acceptable terminal state)

---

## Commit Strategy

- **T0**: no commit (pre-flight proof only; evidence goes to `.sisyphus/evidence/preflight/`).
- **T1+T2**: `feat(stdlib): register FilesystemPath types and filesystem error nominals` — src/type_system/checker/fs_builtins.rs, src/type_system/checker.rs. Pre-commit: `cargo test type_system::`
- **T3+T4**: `feat(runtime): scaffold opal_fs.c and FS result struct ABI` — runtime/opal_fs.c, runtime/opal_runtime.h, src/compiler.rs, src/codegen/statements.rs. Pre-commit: `cargo build`
- **T5–T10**: one commit per infra batch, pattern `feat(stdlib): wire <category> infrastructure`. Pre-commit: `cargo build && cargo test codegen::`
- **T11–T16**: one commit per category TDD batch, pattern `feat(stdlib): implement <category> builtins with TDD`. Pre-commit: `cargo test --features integration fs_<category>`
- **T17–T19**: one commit per test project, pattern `test(stdlib): <project-name> integration test + Opalescent fixture`. Pre-commit: the respective `cargo test --features integration fs_<name>`
- **T20**: `docs(stdlib): prelude.op signatures for filesystem surface`
- **T21**: `docs(readme): add "Building Opalescent Programs for Windows" section`
- **T22**: `chore: track Windows CI parity for stdlib as followup`

---

## Success Criteria

### Verification Commands
```bash
# Build
cargo build --release  # Expected: success

# All unit + integration tests
cargo test  # Expected: 0 failures
cargo test --features integration fs_  # Expected: 3 fs integration tests pass
cargo test --features integration bytes_hex_roundtrip_compiles_links_and_runs  # Expected: no regression

# State-restoration (run test project 3x; fixture unchanged)
sha256sum test-projects/fs-markdown-roundtrip/fixture.md > /tmp/before.txt
for i in 1 2 3; do cargo test --features integration fs_markdown_roundtrip; done
sha256sum test-projects/fs-markdown-roundtrip/fixture.md > /tmp/after.txt
diff /tmp/before.txt /tmp/after.txt  # Expected: empty

# Forbidden patterns
rg 'fn file_' runtime/ src/  # Expected: empty
rg 'uint8\[\]' src/type_system/checker/fs_builtins.rs  # Expected: empty

# README section
grep -q '## Building Opalescent Programs for Windows' README.md  # Expected: exit 0

# Evidence
ls .sisyphus/evidence/fs-path-manipulation/ .sisyphus/evidence/fs-markdown-roundtrip/ .sisyphus/evidence/fs-directory-operations/  # Expected: populated
```

### Final Checklist
- [ ] All 38 functions in `STDLIB_NAMES` and registered in `fs_builtins.rs`.
- [ ] All 3 nominal types + 20 error types registered.
- [ ] `import` of every stdlib fs function from `standard` succeeds in a test program.
- [ ] 3 test projects compile, run, and pass 3 consecutive runs without drift.
- [ ] README has the new subsection with working command snippets.
- [ ] All 5 final-verification tasks return APPROVE/OKAY.
- [ ] User has issued explicit "okay" after reviewing F1–F5 output.
