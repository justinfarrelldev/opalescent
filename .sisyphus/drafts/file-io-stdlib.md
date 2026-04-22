# Draft: File I/O Standard Library (path-object-centric)

## Original Request
User wants two things in ONE plan:
1. Document how to build the Opalescent compiler for Windows AND build programs for Windows using it — in the README, IF not already present.
2. Implement the file-io-surface/path-object-centric stdlib proposal, leveraging existing stdlib (e.g., `Bytes` not `uint8[]`).

## Requirements (confirmed)
- README Windows section: **Already present** (verified at `README.md` lines starting at "## Windows Build"). Still need to verify it covers BOTH (a) building the Opalescent compiler for Windows AND (b) building Opalescent programs for Windows — user said "if not already present" so if existing content is incomplete, extend it; else leave it alone.
- File I/O proposal: path-object-centric (proposal at `stdlib-proposals/file-io-surface/path-object-centric/proposal.md`)
- Use `Bytes` type (not `uint8[]`) — leverage existing stdlib
- Proposal code is NOT valid Opalescent — MUST understand language-spec first
- TDD with red-green-refactor
- Multiple test projects demonstrating real-at-scale usage
- Test projects manipulate ONLY files in their own project dir (markdown etc)
- State restored to original on success; test suites clean up after each run
- Ultrawork
- NO Momus review request from user — user explicitly says "do one" (I'll self-review)
- Research via subagents — DONE (4 explore agents dispatched)

## Functions Required (user-specified list)
### File Reading
- Read entire file contents into memory (as bytes or string)
- Read file line by line (or iterate lines)
- Read a fixed number of bytes from an offset (random access)

### File Writing
- Write/overwrite entire file contents
- Append data to an existing file
- Write at a specific offset

### File Management
- Create a new file
- Delete a file
- Copy a file
- Move/rename a file
- Check if a file exists
- Get file metadata (size, timestamps, permissions)

### Directory Operations
- Create a directory (including recursive/nested)
- Delete a directory (empty, and recursive)
- List directory contents
- Check if a path is a file vs. directory

### Path Manipulation (CPU-only, no I/O — no `_sync` suffix)
- Join path components
- Get parent directory
- Get file name
- Get file extension
- Get absolute path from relative
- Normalize a path (resolve `.` and `..`)

### Permissions / Metadata
- Check read/write/execute permissions
- Set permissions

## Naming Constraint
- Do NOT prefix with `file_` or similar (per user)
- Proposal uses names like `read_contents_sync`, `write_contents_sync`, `join_path_components`, `path_parent_directory`, `path_file_name`, `path_file_extension` — these are acceptable.
- `_sync` suffix: applies to I/O (filesystem-touching) operations per style rule 14. CPU-only path helpers unsuffixed.

## Technical Decisions (to confirm after research)
- Bytes type: reuse built-in `Bytes`, not `uint8[]` (per user + byte-buffer-type proposal recommendation)
- Error handling: `errors` clause on signatures, `guard`/`propagate` in callers — no Result<T,E>
- FilesystemPath: new product type in a `.types.op` file within stdlib-proposals destination? Needs decision on where stdlib code physically lives.
- Exposure mechanism: TBD from research (built-in globals vs import-based stdlib module)
- Error types: FileNotFoundError, PermissionDeniedError, ReadFailureError, IsADirectoryError, InvalidPathError, WriteFailureError, FilesystemFullError, FileAlreadyExistsError, IsNotADirectoryError (from `filesystem_errors.types.op`) — expand with: LineOutOfRangeError, OffsetOutOfRangeError, DirectoryNotEmptyError, DirectoryNotFoundError, MetadataUnavailableError.

## Scope Boundaries
- INCLUDE: Every function in user's list, FilesystemPath type + constructors, errors, test projects, integration tests, README Windows verification.
- EXCLUDE: Async/streaming handles (the handle-based proposal is a separate concern), non-Linux platform-specific code beyond what the runtime already does, network filesystems, symlinks beyond basic metadata, file locking.

## Research Findings (ALL agents returned)

### Language Syntax (bg_7356880a)
- `let name = f(params): ReturnType [errors E1, E2] => <body>` — non-entry functions
- `entry main = f(args: string[]): void => <body>` — exactly one per program; implicitly public/impure/untested
- Types in `.types.op` only. Product: `type P: \n    field: T`. Sum: nested indented variants. Generics: `type Result<T,E>:`.
- Constructor: `new Person:\n    name: 'Alice'\n    age: 30`
- `guard EXPR into name else err =>\n    <block>` — binds success; else-branch handles error
- `propagate EXPR` — bubbles error to caller (signature must list it)
- Imports: `import X, Y from standard`, `import T from ./models.types`, `import foo as bar from module`, `import math as m`
- Strings: single quotes only, `'Hello {world}'` interpolation
- Arrays: `T[]` type, `[a, b, c]` literals, `xs[i]` indexing, `xs.push(v)` for mutable
- Numerics: int8/int16/int32/int64, uint8/uint16/uint32/uint64, float32/float64. Int literals default to int64.
- Comments: `#` line, `## ... ##` doc block (required on public fns, ≥30 chars)
- Ops: `is`, `is not`, `and/or/xor/not`, `band/bor/bxor/bnot/bshl/bshr/bushr`, `+-*/`, `^` exponent
- Returns must be explicit: `return <expr>` or `return void`
- `_sync` suffix: NO EXISTING USAGE in repo; treat as our new convention per proposal style rule 14 (I/O fns get `_sync`, CPU-only path fns don't)
- Mutability: `let mutable x: T = ...` opts in

### Stdlib Wiring Recipe (bg_dd3e0c31) — EXACT file list for each new fn
Per new builtin, touch these files:
1. **`runtime/opal_fs.c`** (new file) — C impl returning `FsResult` struct `{value_ptr, error_cstr}` on fallible or direct type on infallible.
2. **`runtime/opal_runtime.h`** — add C prototype (near bytes prototypes, ~line 74)
3. **`src/compiler.rs`** — add `include_str!("../runtime/opal_fs.c")` to RUNTIME_SOURCE concat (lines 41-56)
4. **`src/codegen/functions_stdlib.rs`** — add match arm in `declare_stdlib_function` + entry in `STDLIB_NAMES` array
5. **`src/codegen/statements.rs`** — add entry in `known_runtime_return_type` and `known_guard_success_type` (if fallible)
6. **`src/type_system/checker/fs_builtins.rs`** (new) — register FilesystemPath type + error nominals + function signatures. Call from `register_standard_builtins()` in `src/type_system/checker.rs` near where `register_bytes_builtins()` is called (~line 295).
7. **`src/type_system/module_resolver.rs`** — add name to standard-module exported symbols list (near where `bytes_*` are listed) so `import X from standard` resolves.
8. **`stdlib/prelude.op`** — doc signature (docs only)
9. **Tests**: `src/codegen/tests.rs` (declaration shape), `src/type_system/tests.rs` (bare-call-fails-without-guard test), integration test-project.

### Fallible Return Shape
- Struct `{ void* value; const char* error; }` (2 pointers).
- codegen generically lowers guard/propagate: checks error field for NULL.
- Use existing `bytes_result_type` for functions returning Bytes; create similar result types for other return types (string, int32, FilesystemPath, void, boolean, FileMetadata).
- Void-returning fallible functions: struct `{ int8_t _dummy; const char* error }` or reuse existing void_result shape (check source).

### Test Harness (bg_27244c43)
- Location: `tests/integration_e2e/*.rs`
- Topline: `#![cfg(feature = "integration")]`
- Helpers in `tests/integration_print.rs`: `prepare_dir(path)` wipes+creates; `cleanup_dir(path)` wipes
- `compile_program(source_path: &Path, source: &str, output_dir: &Path, target: &TargetTriple) -> Result<PathBuf, CompileError>` (from `opalescent::compiler`)
- Test template: prepare_dir → read source → compile_program(.., TargetTriple::host()) → Command::new(binary).output() → assert status + stdout contains → cleanup_dir → final assert message.empty.
- Run: `cargo test --features integration <test_name>`
- Interactive: spawn with piped stdio, write to child.stdin.
- No macro registration; tests reference literal paths `test-projects/<name>/src/main.op`.

### Exposure Mechanism (bg_d0a9c5fe)
- Use EXISTING `import X from standard` mechanism. Parser supports it, module_loader maps "standard"→`__stdlib__/standard`, ModuleResolver::register_standard_modules preloads interfaces.
- Do NOT make globals (reserved for tiny primitives like `print`).
- Real precedent: `bytes-hex-roundtrip` uses `import bytes_from_hex, bytes_to_hex, bytes_concatenate, bytes_slice from standard`.

## Technical Decisions (CONFIRMED)
- **Type**: `FilesystemPath` nominal type with one field `raw_path: string`. Registered in typechecker as a nominal generic-less type like `Bytes`. Construction via `new FilesystemPath:\n    raw_path: '/etc/hosts'`.
- **Bytes**: reuse existing `Bytes` (not `uint8[]`) — user-mandated.
- **Error types to register**: FileNotFoundError, PermissionDeniedError, FileAlreadyExistsError, ReadFailureError, WriteFailureError, InvalidPathError, FilesystemFullError, IsADirectoryError, IsNotADirectoryError, DirectoryNotEmptyError, DirectoryNotFoundError, MetadataUnavailableError, OffsetOutOfRangeError, LineOutOfRangeError, CopyFailureError, MoveFailureError, DeleteFailureError, CreateFailureError, SetPermissionsError.
- **Exposure**: `import X from standard` (not globals).
- **Naming**: no `file_` prefix. I/O functions end in `_sync`. CPU-only path helpers don't.
- **FileMetadata + FilePermissions types**: new nominal types in `src/type_system/checker/fs_builtins.rs`.

## Function Inventory (final signature list)
### Path manipulation (CPU-only, no `_sync`, no errors)
- `make_filesystem_path(raw_path: string): FilesystemPath` — constructor helper (or users use `new FilesystemPath:`)
- `join_path_components(base: FilesystemPath, component: string): FilesystemPath`
- `path_parent_directory(path: FilesystemPath): FilesystemPath`
- `path_file_name(path: FilesystemPath): string`
- `path_file_extension(path: FilesystemPath): string`
- `path_to_absolute(path: FilesystemPath): FilesystemPath errors InvalidPathError` — NOTE: reads CWD via syscall so IS I/O-touching; gets `_sync`? Edge case → call it `absolute_path_sync` OR keep as pure using only string manipulation plus no-cwd (then it requires base). DECISION: `absolute_path_sync(path: FilesystemPath): FilesystemPath errors InvalidPathError` — touches CWD.
- `normalize_path(path: FilesystemPath): FilesystemPath` — pure resolution of `.`/`..` only; does NOT touch fs.

### File Reading
- `read_contents_sync(path: FilesystemPath): Bytes errors FileNotFoundError, PermissionDeniedError, ReadFailureError, IsADirectoryError, InvalidPathError`
- `read_text_sync(path: FilesystemPath): string errors FileNotFoundError, PermissionDeniedError, ReadFailureError, IsADirectoryError, InvalidPathError`
- `read_lines_sync(path: FilesystemPath): string[] errors FileNotFoundError, PermissionDeniedError, ReadFailureError, IsADirectoryError, InvalidPathError`
- `read_bytes_at_offset_sync(path: FilesystemPath, offset: int64, length: int32): Bytes errors FileNotFoundError, PermissionDeniedError, ReadFailureError, IsADirectoryError, InvalidPathError, OffsetOutOfRangeError`

### File Writing
- `write_contents_sync(path: FilesystemPath, data: Bytes): void errors WriteFailureError, PermissionDeniedError, FilesystemFullError, InvalidPathError, IsADirectoryError`
- `write_text_sync(path: FilesystemPath, text: string): void errors WriteFailureError, PermissionDeniedError, FilesystemFullError, InvalidPathError, IsADirectoryError`
- `append_contents_sync(path: FilesystemPath, data: Bytes): void errors FileNotFoundError, WriteFailureError, PermissionDeniedError, FilesystemFullError, InvalidPathError, IsADirectoryError`
- `write_bytes_at_offset_sync(path: FilesystemPath, offset: int64, data: Bytes): void errors FileNotFoundError, WriteFailureError, PermissionDeniedError, FilesystemFullError, InvalidPathError, IsADirectoryError, OffsetOutOfRangeError`

### File Management
- `create_file_sync(path: FilesystemPath): void errors FileAlreadyExistsError, PermissionDeniedError, InvalidPathError, CreateFailureError`
- `delete_file_sync(path: FilesystemPath): void errors FileNotFoundError, PermissionDeniedError, InvalidPathError, IsADirectoryError, DeleteFailureError`
- `copy_file_sync(source: FilesystemPath, destination: FilesystemPath): void errors FileNotFoundError, PermissionDeniedError, InvalidPathError, FilesystemFullError, CopyFailureError, IsADirectoryError`
- `move_path_sync(source: FilesystemPath, destination: FilesystemPath): void errors FileNotFoundError, PermissionDeniedError, InvalidPathError, FilesystemFullError, MoveFailureError`
- `path_exists_sync(path: FilesystemPath): boolean errors PermissionDeniedError, InvalidPathError`
- `read_metadata_sync(path: FilesystemPath): FileMetadata errors FileNotFoundError, PermissionDeniedError, InvalidPathError, MetadataUnavailableError`

### Directory Operations
- `create_directory_sync(path: FilesystemPath): void errors FileAlreadyExistsError, PermissionDeniedError, InvalidPathError, CreateFailureError`
- `create_directory_recursive_sync(path: FilesystemPath): void errors PermissionDeniedError, InvalidPathError, CreateFailureError`
- `delete_directory_sync(path: FilesystemPath): void errors DirectoryNotFoundError, DirectoryNotEmptyError, PermissionDeniedError, InvalidPathError, IsNotADirectoryError, DeleteFailureError`
- `delete_directory_recursive_sync(path: FilesystemPath): void errors DirectoryNotFoundError, PermissionDeniedError, InvalidPathError, IsNotADirectoryError, DeleteFailureError`
- `list_directory_sync(path: FilesystemPath): FilesystemPath[] errors DirectoryNotFoundError, PermissionDeniedError, InvalidPathError, IsNotADirectoryError, ReadFailureError`
- `is_file_sync(path: FilesystemPath): boolean errors PermissionDeniedError, InvalidPathError, FileNotFoundError`
- `is_directory_sync(path: FilesystemPath): boolean errors PermissionDeniedError, InvalidPathError, FileNotFoundError`

### Permissions / Metadata
- `can_read_sync(path: FilesystemPath): boolean errors FileNotFoundError, InvalidPathError`
- `can_write_sync(path: FilesystemPath): boolean errors FileNotFoundError, InvalidPathError`
- `can_execute_sync(path: FilesystemPath): boolean errors FileNotFoundError, InvalidPathError`
- `set_permissions_sync(path: FilesystemPath, permissions: FilePermissions): void errors FileNotFoundError, PermissionDeniedError, InvalidPathError, SetPermissionsError`

Total function count: **28 functions** (6 path manip + 4 reading + 4 writing + 6 file mgmt + 7 dir ops + 4 permissions). Plus 3 new nominal types (FilesystemPath, FileMetadata, FilePermissions) plus 19 error types.

## Test Project Plan
Three test projects (all markdown-only; restore state after):
1. **`fs-path-manipulation`** — exercises ONLY CPU-only path helpers (no I/O). Prints results. Deterministic, no cleanup needed beyond target dir.
2. **`fs-markdown-roundtrip`** — read a README.md, append a marker line, read back, verify, then restore original bytes. Integration test snapshots stdout + verifies file bytes equal original at end.
3. **`fs-directory-operations`** — create subdir, create files, list, read metadata, copy/move, delete recursive. All under project's `./sandbox/` which is cleared at start AND end.

Each test-project gets:
- `opal.toml` (name + version)
- `src/main.op` (entry main)
- `.gitignore` (`/target/`, `*.o`, `/sandbox/`)
- `README.md`
- Test-fixture markdown file (e.g., `fixture.md`) with known content. Integration test snapshot bytes before, runs binary, asserts stdout, asserts fixture bytes still equal snapshot.
- Rust integration test in `tests/integration_e2e/fs_*.rs`.

## README Windows Section Status
**VERIFIED PRESENT** at `README.md` lines from "## Windows Build" covering:
- Native Windows MSVC build of the compiler (rustup + LLVM 14 + LLVM_SYS_140_PREFIX + cargo build --release)
- Cross-compilation from Linux via xwin + clang-cl/lld-link
- Wine testing
- Target triples `x86_64-pc-windows-msvc` / `x86_64-pc-windows-gnu` already documented in Build Targets section.

**Gap assessment**: The current section documents how to build the COMPILER for Windows. It does NOT explicitly document how end-users build their OPALESCENT PROGRAMS for Windows (i.e., cross-compiling a .op file to a Windows .exe). The `opal.toml` `[build].targets = ["x86_64-pc-windows-msvc"]` is mentioned but the user-facing workflow (e.g., `opal build --target x86_64-pc-windows-msvc` or configuring opal.toml) and the prerequisites on the host side (LLVM, Windows SDK via xwin if cross-compiling) should be explicitly called out.

**Action**: Add a "Building Opalescent Programs for Windows" subsection to the Windows Build section that:
- Explains target triples for Opalescent programs (not the compiler)
- Shows how to configure opal.toml `[build].targets`
- Shows the CLI invocation (if cross-compiling)
- Notes prerequisites (xwin sysroot, clang-cl) reuse from the compiler cross-compile section.


## Test Strategy Decision
- **Infrastructure exists**: YES (cargo test + `integration` feature per README)
- **Automated tests**: YES (TDD RED-GREEN-REFACTOR — user explicitly requested)
- **Framework**: Rust integration tests + Opalescent test-projects + runtime C unit tests if needed
- **Agent-Executed QA**: MANDATORY per system rules. For each task, agent must compile+run the test project and capture evidence (stdout, exit code, file-state verification).

## Plan Name
`.sisyphus/plans/file-io-stdlib-path-object-centric.md`

## Metis-Surfaced Design Decisions (LOCKED by user Q&A)
1. **Path construction**: Both — `path_from(raw_path: string): FilesystemPath` builtin AND `new FilesystemPath:` record constructor. (+1 fn)
2. **Text encoding**: UTF-8 strict with `InvalidUtf8Error`. Byte-oriented escape hatches retain same naming scheme — `read_contents_sync`/`write_contents_sync`/`append_contents_sync` are the byte variants; `read_text_sync`/`write_text_sync`/`read_lines_sync`/`append_text_sync` are the strict UTF-8 variants.
3. **Line endings**: Split on `\n`, strip single trailing `\r` per line (CRLF+LF both round-trip cleanly).
4. **Write atomicity**: Provide both flavors — naive (`write_contents_sync`, `write_text_sync`) AND atomic (`write_contents_atomic_sync`, `write_text_atomic_sync`). (+2 fns)
5. **Symlinks**:
   - Content ops (`read_*`, `write_*`, `append_*`): follow symlink to target (POSIX stat-like).
   - Metadata & inspection: two variants — follow (default, no suffix) and no-follow (`_nofollow_sync`). Affects `read_metadata_sync`, `is_file_sync`, `is_directory_sync`. (+3 fns)
   - Pure path manipulation: purely lexical, never touches filesystem.
6. **Permissions**: Abstract triple `{readable: boolean, writable: boolean, executable: boolean}` — portable across Unix/Windows. On Windows, `executable` derived from extension.
7. **Platform scope**: Linux-only CI for stdlib tests; Windows parity → `.sisyphus/followups.md`.
8. **Pre-Flight Validation**: MANDATORY first task — verify `FilesystemPath[]`, `string[]` return types work from stdlib builtins AND scalar fallible-ABI supports `boolean` / `int32` / `int64` returns.

## Updated Function Inventory (34 functions total)
### Path manipulation (6 fns, no `_sync` except absolute_path)
- `path_from(raw_path: string): FilesystemPath`  [constructor builtin — new]
- `join_path_components(base: FilesystemPath, component: string): FilesystemPath`
- `path_parent_directory(path: FilesystemPath): FilesystemPath`
- `path_file_name(path: FilesystemPath): string`
- `path_file_extension(path: FilesystemPath): string`
- `normalize_path(path: FilesystemPath): FilesystemPath`
- `absolute_path_sync(path: FilesystemPath): FilesystemPath errors InvalidPathError`  [touches CWD → `_sync`]

### File Reading (bytes + text) (6 fns)
- `read_contents_sync(path: FilesystemPath): Bytes errors FileNotFoundError, PermissionDeniedError, ReadFailureError, IsADirectoryError, InvalidPathError`
- `read_text_sync(path: FilesystemPath): string errors FileNotFoundError, PermissionDeniedError, ReadFailureError, IsADirectoryError, InvalidPathError, InvalidUtf8Error`
- `read_lines_sync(path: FilesystemPath): string[] errors FileNotFoundError, PermissionDeniedError, ReadFailureError, IsADirectoryError, InvalidPathError, InvalidUtf8Error`
- `read_bytes_at_offset_sync(path: FilesystemPath, offset: int64, length: int32): Bytes errors FileNotFoundError, PermissionDeniedError, ReadFailureError, IsADirectoryError, InvalidPathError, OffsetOutOfRangeError`

### File Writing (bytes + text + atomic variants) (8 fns)
- `write_contents_sync(path: FilesystemPath, data: Bytes): void errors WriteFailureError, PermissionDeniedError, FilesystemFullError, InvalidPathError, IsADirectoryError`
- `write_text_sync(path: FilesystemPath, text: string): void errors WriteFailureError, PermissionDeniedError, FilesystemFullError, InvalidPathError, IsADirectoryError`
- `write_contents_atomic_sync(path: FilesystemPath, data: Bytes): void errors WriteFailureError, PermissionDeniedError, FilesystemFullError, InvalidPathError, IsADirectoryError`  [NEW — temp+rename]
- `write_text_atomic_sync(path: FilesystemPath, text: string): void errors WriteFailureError, PermissionDeniedError, FilesystemFullError, InvalidPathError, IsADirectoryError`  [NEW — temp+rename]
- `append_contents_sync(path: FilesystemPath, data: Bytes): void errors FileNotFoundError, WriteFailureError, PermissionDeniedError, FilesystemFullError, InvalidPathError, IsADirectoryError`
- `append_text_sync(path: FilesystemPath, text: string): void errors FileNotFoundError, WriteFailureError, PermissionDeniedError, FilesystemFullError, InvalidPathError, IsADirectoryError`  [NEW text variant]
- `write_bytes_at_offset_sync(path: FilesystemPath, offset: int64, data: Bytes): void errors FileNotFoundError, WriteFailureError, PermissionDeniedError, FilesystemFullError, InvalidPathError, IsADirectoryError, OffsetOutOfRangeError`

### File Management (6 fns)
- `create_file_sync(path: FilesystemPath): void errors FileAlreadyExistsError, PermissionDeniedError, InvalidPathError, CreateFailureError`
- `delete_file_sync(path: FilesystemPath): void errors FileNotFoundError, PermissionDeniedError, InvalidPathError, IsADirectoryError, DeleteFailureError`
- `copy_file_sync(source: FilesystemPath, destination: FilesystemPath): void errors FileNotFoundError, PermissionDeniedError, InvalidPathError, FilesystemFullError, CopyFailureError, IsADirectoryError`
- `move_path_sync(source: FilesystemPath, destination: FilesystemPath): void errors FileNotFoundError, PermissionDeniedError, InvalidPathError, FilesystemFullError, MoveFailureError`
- `path_exists_sync(path: FilesystemPath): boolean errors PermissionDeniedError, InvalidPathError`
- `read_metadata_sync(path: FilesystemPath): FileMetadata errors FileNotFoundError, PermissionDeniedError, InvalidPathError, MetadataUnavailableError`
- `read_metadata_nofollow_sync(path: FilesystemPath): FileMetadata errors FileNotFoundError, PermissionDeniedError, InvalidPathError, MetadataUnavailableError`  [NEW]

### Directory Operations (7 fns)
- `create_directory_sync(path: FilesystemPath): void errors FileAlreadyExistsError, PermissionDeniedError, InvalidPathError, CreateFailureError`
- `create_directory_recursive_sync(path: FilesystemPath): void errors PermissionDeniedError, InvalidPathError, CreateFailureError`
- `delete_directory_sync(path: FilesystemPath): void errors DirectoryNotFoundError, DirectoryNotEmptyError, PermissionDeniedError, InvalidPathError, IsNotADirectoryError, DeleteFailureError`
- `delete_directory_recursive_sync(path: FilesystemPath): void errors DirectoryNotFoundError, PermissionDeniedError, InvalidPathError, IsNotADirectoryError, DeleteFailureError`
- `list_directory_sync(path: FilesystemPath): FilesystemPath[] errors DirectoryNotFoundError, PermissionDeniedError, InvalidPathError, IsNotADirectoryError, ReadFailureError`
- `is_file_sync(path: FilesystemPath): boolean errors PermissionDeniedError, InvalidPathError, FileNotFoundError`
- `is_file_nofollow_sync(path: FilesystemPath): boolean errors PermissionDeniedError, InvalidPathError, FileNotFoundError`  [NEW]
- `is_directory_sync(path: FilesystemPath): boolean errors PermissionDeniedError, InvalidPathError, FileNotFoundError`
- `is_directory_nofollow_sync(path: FilesystemPath): boolean errors PermissionDeniedError, InvalidPathError, FileNotFoundError`  [NEW]

### Permissions (4 fns)
- `can_read_sync(path: FilesystemPath): boolean errors FileNotFoundError, InvalidPathError`
- `can_write_sync(path: FilesystemPath): boolean errors FileNotFoundError, InvalidPathError`
- `can_execute_sync(path: FilesystemPath): boolean errors FileNotFoundError, InvalidPathError`
- `set_permissions_sync(path: FilesystemPath, permissions: FilePermissions): void errors FileNotFoundError, PermissionDeniedError, InvalidPathError, SetPermissionsError`

Count: 7 path + 4 reading + 7 writing + 7 file-mgmt + 9 dir-ops (with nofollow) + 4 permissions = **38 functions** (recount: path_from is 1; path helpers 6; absolute_path_sync 1; read 4; write 7; file mgmt 6 + nofollow metadata 1 = 7; dir ops 5 + 2 nofollow = 7; permissions 4 → 1+6+1+4+7+7+7+4 = **37**). Will finalize exact count in plan.

## Errors (20 total)
Existing 19 + `InvalidUtf8Error` (new, for UTF-8 strict variants).

## RAII Cleanup Guard Pattern
Every Rust integration test uses a `struct StateGuard { project_dir: PathBuf, snapshot: HashMap<PathBuf, [u8;32]> }` that in `Drop` restores all snapshotted files to their original bytes and removes any new files. Guarantees cleanup even on panic/assert failure.

## Evidence Capture Protocol
Per QA scenario under `.sisyphus/evidence/fs-<project>/<scenario>/`:
- `stdout.log`, `exit-code.txt`, `pre-state.txt` (sha256 manifest), `post-state.txt`, `state-diff.txt` (must be empty), `cleanup-marker.txt` (Drop-written timestamp).
