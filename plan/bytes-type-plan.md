# Plan: Dedicated `Bytes` Type (stdlib-proposals/byte-buffer-type/dedicated-bytes-type)

## Overview

Implement the **dedicated `Bytes` type** proposal from `stdlib-proposals/byte-buffer-type/dedicated-bytes-type/`. The proposal specifies a struct-backed byte buffer type wrapping a `uint8[]` array with a length field, plus a dedicated suite of binary-data operations (concatenation, slicing, hex encoding, hex decoding, and equality).

This is a **Rust-side standard library addition** living alongside the other `src/stdlib/*` submodules (e.g. `collections::array::OpalVec`, `strings`, `math`). It exposes `Bytes` as an Opalescent-language-level type implemented in Rust, not as new compiler syntax. Follows the existing pattern for `OpalVec<T>` etc. — a `pub struct` in its own module with a fail-fast, error-returning API.

The language goals this supports:

- **Maintainability at enterprise scale**: full doc comments on every item, exhaustive tests.
- **Safer than Go, easier than Rust**: `Bytes` provides semantic clarity over raw `Vec<u8>`.
- **Fail-fast on primitives**: all fallible operations return `Result<_, BytesError>`.
- **Purity codified in syntax**: methods are `&self` wherever possible; mutation is explicit.
- **Testing at the forefront**: every public method has at least 3 tests including edge cases.

## Specification (from proposal)

From `dedicated-bytes-type/bytes.types.op`:

```opal
type Bytes:
    data: uint8[]
    length: int32
```

From `manipulation.op` and `file_io.op`, the public operations:

1. `concatenate_bytes(first: Bytes, second: Bytes): Bytes` — combine two buffers.
2. `slice_bytes(buffer: Bytes, start: int32, end: int32): Bytes errors SliceRangeError` — subrange.
3. `bytes_to_hex_string(buffer: Bytes): string` — lowercase hex dump.

Additional operations needed for a complete, ergonomic API (derivable from the proposal's goals without expanding scope):

- Constructor / factory: `Bytes::new()` (empty), `Bytes::from_slice(&[u8])`, `Bytes::from_vec(Vec<u8>)`.
- Accessors: `length()`, `get(index)`, `as_slice()`, `data()` (read-only view).
- `from_hex_string(hex: &str)` for round-trip symmetry with `to_hex_string`.
- `equals(other)` — value equality (already covered by `PartialEq` derive, but exposed semantically).

## Module Location

`src/stdlib/bytes.rs` (flat file). Register it in `src/stdlib.rs` alongside `strings`, `math`, etc.

This is a single-file module (well under 500 lines). If it grows past 500 lines we split into `src/stdlib/bytes/{mod.rs, hex.rs, ops.rs}`.

## Error Model

A `BytesError` enum with these variants (mirroring `VecError`):

- `IndexOutOfBounds { index: usize, length: usize }`
- `InvalidRange { start: usize, end: usize, length: usize }`
- `InvalidHexLength { length: usize }` — odd-length hex string
- `InvalidHexCharacter { character: char, position: usize }` — non-hex char in input

## TDD Checklist

### Red Phase — Write tests first (all failing)

- [x] Create `src/stdlib/bytes/tests.rs` (inline module) with the test plan below.
- [x] Register `pub mod bytes` in `src/stdlib.rs` with a placeholder empty `bytes.rs` so tests compile-fail on missing symbols only (proper red).
- [x] Confirm `cargo test` fails with missing-symbol errors (red phase verified).

### Required test cases (≥3 per public method)

**Construction & basic accessors**

- [x] `test_bytes_new_is_empty` — `Bytes::new().length() == 0`.
- [x] `test_bytes_from_slice_preserves_data` — round-trip a `&[u8]`.
- [x] `test_bytes_from_vec_preserves_data` — round-trip a `Vec<u8>`.
- [x] `test_bytes_length_matches_data` — length equals data length after construction.
- [x] `test_bytes_get_in_bounds` — `get(0)` returns first byte.
- [x] `test_bytes_get_out_of_bounds` — `get(len)` returns `None`.
- [x] `test_bytes_as_slice_matches_construction` — slice view returns the original bytes.

**Concatenation (`concatenate`)**

- [x] `test_concatenate_two_nonempty` — `[1,2] + [3,4] == [1,2,3,4]`.
- [x] `test_concatenate_empty_left` — `[] + [1,2] == [1,2]`.
- [x] `test_concatenate_empty_right` — `[1,2] + [] == [1,2]`.
- [x] `test_concatenate_both_empty` — `[] + [] == []` with length 0.
- [x] `test_concatenate_length_is_sum` — resulting length == sum of lengths.

**Slicing (`slice`)**

- [x] `test_slice_full_range_returns_copy_equal` — `slice(0, len)` equals original.
- [x] `test_slice_empty_range_returns_empty` — `slice(n, n)` returns empty.
- [x] `test_slice_middle_range` — `[0,1,2,3,4].slice(1,4) == [1,2,3]`.
- [x] `test_slice_start_greater_than_end_errors` — returns `InvalidRange`.
- [x] `test_slice_end_out_of_bounds_errors` — returns `InvalidRange`.

**Hex encoding (`to_hex_string`)**

- [x] `test_to_hex_empty_returns_empty_string`.
- [x] `test_to_hex_single_byte_zero` — `[0x00]` → `"00"`.
- [x] `test_to_hex_single_byte_ff` — `[0xFF]` → `"ff"` (lowercase).
- [x] `test_to_hex_multi_byte` — `[0xDE, 0xAD, 0xBE, 0xEF]` → `"deadbeef"`.
- [x] `test_to_hex_length_is_double_byte_count` — output length == 2 × byte count.

**Hex decoding (`from_hex_string`)**

- [x] `test_from_hex_empty_returns_empty_bytes`.
- [x] `test_from_hex_roundtrip` — `from_hex_string(to_hex_string(b)) == b`.
- [x] `test_from_hex_uppercase_accepted` — `"DEADBEEF"` decodes same as `"deadbeef"`.
- [x] `test_from_hex_odd_length_errors` — `"abc"` returns `InvalidHexLength`.
- [x] `test_from_hex_invalid_char_errors` — `"zz"` returns `InvalidHexCharacter`.

**Equality**

- [x] `test_equals_same_content` — two `Bytes` with same data compare equal.
- [x] `test_equals_different_content` — differing data compares unequal.
- [x] `test_equals_different_length` — different length compares unequal.

### Green Phase — Minimal implementation

- [x] Implement `BytesError` enum with all four variants.
- [x] Implement `Bytes` struct wrapping `Vec<u8>`.
- [x] Implement `Bytes::new`, `from_slice`, `from_vec`.
- [x] Implement `length`, `get`, `as_slice`.
- [x] Implement `concatenate`.
- [x] Implement `slice`.
- [x] Implement `to_hex_string`.
- [x] Implement `from_hex_string`.
- [x] Derive `PartialEq`, `Eq`, `Clone`, `Debug`.
- [x] All tests pass.

### Refactor Phase — Quality pass

- [x] Extract hex nibble helper (`nibble_to_hex_char`, `hex_char_to_nibble`) for readability.
- [x] Ensure `#[must_use]` on every constructor and query method.
- [x] Add `# Errors` doc sections to all `Result`-returning functions.
- [x] Add module-level documentation describing rationale and `no_std` compatibility.
- [x] Add doc comments on every variant of `BytesError`.
- [x] Cross-reference proposal location in module docs.
- [x] Run `cargo make lint` and fix all warnings.
- [x] Confirm file is `< 500` lines (split if necessary).
- [x] Confirm `no_std` compliance (only `alloc` + `core`, no `std`).

### Verification

- [x] `cargo make test` passes (all existing + new tests).
- [x] `cargo make lint` passes with no warnings.
- [x] `scripts/check-line-count.sh` reports all files < 1000 lines.
- [x] `cargo make build-all` (or current-platform subset) succeeds.
- [x] Update `PLAN.md` with a checked entry under a new "Standard Library Extensions" section.

## Out of Scope

- No new Opalescent-language syntax (no new keywords, no new AST nodes).
- No file-I/O integration (`read_file_sync` from the proposal is already covered by `stdlib::fs`).
- No FFI / C-ABI exposure (future work when the runtime links against it).
- No mutability operations like `push`/`insert` — the proposal treats `Bytes` as immutable-by-default. Mutation requires new `Bytes` via `concatenate` / `slice`. This matches Opalescent's "immutable by default" principle.
