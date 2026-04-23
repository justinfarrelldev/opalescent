# Learnings — file-io-stdlib-path-object-centric

## [2026-04-22] Session start

### Codebase patterns
- 8-touchpoint wiring: opal_fs.c → opal_runtime.h → compiler.rs RUNTIME_SOURCE → functions_stdlib.rs (STDLIB_NAMES + declare_stdlib_function) → statements.rs (known_runtime_return_type + known_guard_success_type) → fs_builtins.rs (new) → module_resolver.rs → tests
- Fallible ABI: `{ void* value; const char* error_cstr }` struct shape
- Template for nominal types: `src/type_system/checker/bytes_builtins.rs`
- Template for test harness: `tests/integration_e2e/bytes_stdlib.rs`
- Template for test project: `test-projects/bytes-hex-roundtrip/`
- RUNTIME_SOURCE concat in `src/compiler.rs` lines 41-56
- STDLIB_NAMES in `src/codegen/functions_stdlib.rs` ~224-276
- known_runtime_return_type in `src/codegen/statements.rs` ~518-575
- known_guard_success_type in `src/codegen/statements.rs` ~580-596
- register_standard_builtins in `src/type_system/checker.rs` ~295
- standard-module exports in `src/type_system/module_resolver.rs` ~576-672

## [2026-04-22] T0 preflight completion notes
- Array type annotations on local `let` bindings are unreliable in this codegen path; type inference for array-returning stdlib calls avoids that failure mode for preflight.
- `for` loop codegen currently requires explicit materialization of iteration variable into `env.variables`; missing binding manifests as `unknown variable '<iter-var>'` during codegen.
- `FilesystemPath` nominal values can be iterated in arrays and field-accessed in preflight (`p.raw`) once loop variable binding is materialized.
- Preflight runtime validation achieved expected output lines for both `string[]` and `FilesystemPath[]` iteration plus guard success/error branches.
- Guard boolean success value prints as textual `true/false` when explicitly normalized in source (`if ok is true`), not by raw numeric formatting.

## [2026-04-22] For-loop iterator variable codegen fix
- `Stmt::For` must explicitly materialize and bind the iteration variable each iteration (alloca + store + `env.variables.insert`) before lowering loop body, otherwise identifier loads fail with `unknown variable '<iter-var>'`.
- Array iteration in codegen should use the existing array ABI conventions: array pointer from iterable binding alloca plus length from either `binding.length` (literal-backed arrays) or companion `{name}_len` binding (imported/param arrays).
- Loop-scoped variable shadowing should preserve previous bindings with insert/restore semantics around body emission to avoid scope leakage outside loop iterations.

## [2026-04-22] T0 preflight validation execution learnings
- Temporary array-returning stdlib builtins can be lowered by extracting struct return `{ptr, len}` in `codegen_let_statement` for `let arr = stdlib_call()` when inferred/declared as `CoreType::Array(_)`, and then binding companion `{name}_len` for downstream `for`/index lowering.
- `FilesystemPath.raw` member access works in codegen by handling `CoreType::Generic { name: "FilesystemPath" }` in intrinsic member-access path and loading field 0 directly.
- `opal_runtime.h` must NOT be included from concatenated runtime fragments that also define shared typedefs (`ParseResult*`, `BytesResult`) unless those fragments are refactored; self-contained temporary runtime C avoided redefinition conflicts.
- Compiler CLI `opal <file> --run` prints an additional `target/program` line before program output; exact expected stdout lines were captured from running `./target/program` after successful compile step.

## [2026-04-22] T6 infra wiring completion
- T6 File Reading infra follows existing fallible ABI pattern: LLVM declarations use  for bytes/string and  for line arrays.
- Standard module exports wired with exact FilesystemPath parameters and declared error sets, enabling type-checking/guard flows without runtime behavior changes.
- Runtime C stubs return placeholder  errors with NULL values (and count=0 for line arrays) to keep linker/typechecker paths green before implementation batches.

## [2026-04-22] T6 infra wiring completion
- T6 File Reading infra follows existing fallible ABI pattern: LLVM declarations use {i8* value, i8* error} for bytes/string and {i8** value, i64 count, i8* error} for line arrays.
- Standard module exports wired with exact FilesystemPath parameters and declared error sets, enabling type-checking/guard flows without runtime behavior changes.
- Runtime C stubs return placeholder error strings with NULL values (and count=0 for line arrays) to keep linker/typechecker paths green before implementation batches.

## [2026-04-22T22:00:39-04:00] T7 infra wiring completion
- File-writing stdlib infra mirrors T6 read batch conventions: all seven fallible writers are declared as `{i8* value, i8* error}` (`FsVoidResult`) in `declare_stdlib_function`.
- ABI mapping for the write batch is consistent across layers: `FilesystemPath`/`Bytes`/`string` lower to `i8*`, and `offset` lowers to `i64` for `write_bytes_at_offset_sync`.
- Standard module export ordering remains function exports before nominal type exports; this keeps resolver wiring aligned with prior FS batches and avoids checker churn.

## [2026-04-22T22:07:07-04:00] T8 infra wiring completion
- File-management batch ABI wiring follows established FS conventions: create/delete/copy/move use `FsVoidResult`, existence uses `FsBooleanResult`, and both metadata reads use `FsMetadataResult`.
- `declare_stdlib_function` now reuses shared local result-struct LLVM types for the new batch, keeping value/error layouts aligned with guard/propagate lowering expectations.
- Standard module exports include exact requested error sets for each of the seven functions, with `FilesystemPath` parameter types and `FileMetadata` return type for metadata readers.

## [2026-04-22T22:14:04-04:00] T8 verification fix: FileMetadata member access
- Root cause: `ModuleResolver::new()` preloaded `standard` ADT field schemas (including `FileMetadata.size_bytes`), but `TypeChecker` did not mirror those schemas into its local `adt_fields` map used by member-access resolution.
- Minimal fix: in both `TypeChecker::new` and `TypeChecker::with_environment`, register the `standard` module interface from `module_resolver` into the checker (`register_module_interface`), which copies ADT field metadata and makes `m1.size_bytes`/`m2.is_directory` resolve.

## [2026-04-22T22:26:00-04:00] T9 infra wiring completion
- Directory-operations ABI wiring uses existing FS result structs consistently: create/delete variants return `FsVoidResult`, `list_directory_sync` returns `FsPathArrayResult`, and file/dir inspection variants return `FsBooleanResult`.
- `declare_stdlib_function` now includes a reusable LLVM `FsPathArrayResult` struct (`{i8** value, i64 count, i8* error}`), matching the preflight-proven array return convention used by guard/propagate lowering.
- Standard module exports include exact T9 signatures and error sets for all nine directory functions, including distinct nofollow inspection variants.
