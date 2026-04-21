# Plan: Opalescent-Language Bytes Stdlib Integration

## Overview

Promote the existing `src/stdlib/bytes.rs` Rust library into first-class
Opalescent-language stdlib built-ins that can be imported and called from `.op`
programs and that compile through the full pipeline (type checker → LLVM IR →
C runtime → native binary).

## Surface Exposed to `.op` Code

A user writes:

```opal
import bytes_new, bytes_to_hex, bytes_from_hex, bytes_concatenate, bytes_slice from standard

let size: int32 = buffer.length
```

and gets the following signatures:

| Opalescent signature                                                           | LLVM symbol            |
| ------------------------------------------------------------------------------ | ---------------------- |
| `bytes_new(): Bytes`                                                           | `i8* @bytes_new()`     |
| `b.length: int32` (lowered to runtime helper)                                  | `i32 @bytes_length(i8*)` |
| `bytes_to_hex(b: Bytes): string`                                               | `i8* @bytes_to_hex(i8*)` |
| `bytes_from_hex(s: string): Bytes errors HexDecodeError`                       | `{i8*, i8*} @bytes_from_hex(i8*)` |
| `bytes_concatenate(a: Bytes, b: Bytes): Bytes`                                 | `i8* @bytes_concatenate(i8*, i8*)` |
| `bytes_slice(b: Bytes, start: int32, end: int32): Bytes errors SliceRangeError`| `{i8*, i8*} @bytes_slice(i8*, i32, i32)` |

`Bytes` is registered as a nominal type (`CoreType::Generic { name: "Bytes", type_args: [] }`). At the LLVM layer it lowers to `i8*` (opaque owned pointer to the heap `OpalBytes` struct), reusing the existing `CoreType::Generic → i8*` mapping in `src/codegen/types.rs`.

The fallible builtins use the `{value_ptr, error_cstr}` struct-return convention already used by `string_to_int32`, so they integrate with `guard ... into ... else err =>` for free.

## Layers Touched

1. **C runtime** — `runtime/opal_bytes.c` + entries in `runtime/opal_runtime.h`.
2. **Compiler wiring** — `src/compiler.rs` `RUNTIME_SOURCE` include.
3. **Type system** — `src/type_system/checker.rs::register_standard_builtins` adds the six signatures and the `Bytes` nominal type.
4. **Codegen declarations** — `src/codegen/functions_stdlib.rs` declares the six functions and lists them in `STDLIB_NAMES`.
5. **Codegen return-type inference** — `src/codegen/statements.rs::known_runtime_return_type` for `bytes_from_hex` and `bytes_slice` so guard statements can bind `Bytes`.
6. **Prelude documentation** — `stdlib/prelude.op`.
7. **Sample projects** — `test-projects/bytes-*` demonstrating each operation.

## Error Model

The two fallible functions return a `{i8*, i8*}` pair where the second pointer is either `NULL` (success) or a C string. This exactly matches the convention already used for `string_to_intN` so no new codegen infrastructure is required. At the language level these errors are surfaced as:

- `bytes_from_hex`: `errors HexDecodeError`
- `bytes_slice`: `errors SliceRangeError`

Error type names are registered as generic nominal types so `guard ... else err =>` can bind `err: string` (consistent with other stdlib error surfaces).

## TDD Checklist

### Red — Tests first

- [x] **Type-system test** (`src/type_system/tests.rs`): a fixture program that imports all six symbols and uses them in a function body must type-check without errors. Also: `bytes_slice` used inside a `guard` must bind a `Bytes` value in the success branch.
- [x] **Codegen declaration test** (`src/codegen/tests.rs`): a fixture program importing all six symbols must emit the LLVM declarations listed in the table above.
- [x] **Codegen guard test** (`src/codegen/tests.rs`): a guarded `bytes_from_hex` call must emit a `{i8*, i8*}` struct return declaration.
- [x] Run `cargo test --lib stdlib::bytes` and the new tests — new tests fail to compile or fail at runtime (red).

### Green — Implementation

- [x] Write `runtime/opal_bytes.c` (copying logic from `src/stdlib/bytes.rs` into C).
- [x] Extend `runtime/opal_runtime.h` with the six C prototypes.
- [x] Extend `RUNTIME_SOURCE` in `src/compiler.rs` with `include_str!("../runtime/opal_bytes.c")`.
- [x] In `src/codegen/functions_stdlib.rs`:
  - [x] Add struct type `bytes_result_type = { i8*, i8* }`.
  - [x] Match arms for the six `bytes_*` names producing the correct `FunctionType`.
  - [x] Append the six names to `STDLIB_NAMES`.
- [x] In `src/codegen/statements.rs::known_runtime_return_type`: add arms for `bytes_from_hex` and `bytes_slice` returning `CoreType::Generic { name: "Bytes", type_args: [] }` (success-branch type for guard). Arms for `bytes_new`, `bytes_concatenate` returning `Bytes`; `bytes_length` returning `Int32`; `bytes_to_hex` returning `String`.
- [x] In `src/type_system/checker.rs::register_standard_builtins`: register bytes builtin signatures plus `Bytes.length` member typing and the `Bytes` nominal type (and `HexDecodeError`, `SliceRangeError` error type names). Implemented in new `src/type_system/checker/bytes_builtins.rs` to respect the 500-line soft limit.
- [x] All new tests now pass (green).

### Refactor

- [x] Extract a small helper in `functions_stdlib.rs` for the repeated `{i8*, i8*}` return type if the proliferation warrants it.
- [x] Ensure every public function in C has a doc comment-style block at the top of the `.c` file.
- [x] Update module docs at the top of `src/stdlib/bytes.rs` noting that this is also exposed to Opalescent programs through the `standard` module.
- [x] `cargo make lint` must pass.
- [x] `scripts/check-line-count.sh` must pass.

### Integration validation

- [x] Create `test-projects/bytes-hex-roundtrip` — consolidated fixture exercising `bytes_from_hex`, `Bytes.length`, `bytes_to_hex`, `bytes_concatenate`, and `bytes_slice` (including a `guard` branch). The single consolidated project subsumes the originally-planned `bytes-concat` and `bytes-slice` projects; it asserts every surface without triplicating `opal.toml` scaffolding.
- [x] Project has an `opal.toml` and an `src/main.op`.
- [x] Compiled, linked, executed, and stdout-asserted inside `tests/integration_e2e/bytes_stdlib.rs`, gated by `#[cfg(feature = "integration")]` so it runs under `cargo test --features integration`.
- [x] Update `PLAN.md` checklist entry under "Standard Library Extensions → Dedicated `Bytes` Type".

## Out of Scope

- `bytes_get`, `bytes_as_slice`, `bytes_from_vec`, `bytes_from_slice` — these are Rust-only helpers for now; not exposed to `.op`. Can be added later without breaking the initial surface.
- FFI exposure of `Bytes` to external C code beyond the runtime.
- Reference counting of `Bytes` via the Perceus/SCR infrastructure — for the first cut, `OpalBytes*` is heap-allocated and leaked (consistent with other stdlib returns like `take_input`'s `char*` buffer which is also leaked today).
