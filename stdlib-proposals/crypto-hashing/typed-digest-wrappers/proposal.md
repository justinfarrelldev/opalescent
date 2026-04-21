# typed-digest-wrappers

## Overview

This alternative creates distinct digest wrapper types for each algorithm (`Sha256Digest`, `Sha512Digest`, `Blake3Digest`, `Md5Digest`) so digest values cannot be mixed accidentally. Instead of returning plain strings, hashing functions return typed wrappers and conversion functions handle explicit serialization.

The core goal is compile-time safety. Teams that persist or compare digests across multiple systems avoid subtle class-of-bug issues where values from different algorithms are accidentally interchanged.

## Assumes

- Opalescent type system is used to encode algorithm identity in value types
- Digest text serialization remains hex strings when crossing system boundaries
- File-backed digest generation uses `_sync` suffixes
- Error strategy supports explicit parse and length mismatch error types

## Syntax Design

```opal
let sha256_digest = f(input_bytes: uint8[]): Sha256Digest errors EmptyInputError =>
    return digest

let parse_sha256_digest = f(digest_hex: string): Sha256Digest errors InvalidHexDigestError, DigestLengthMismatchError =>
    return digest

let sha256_digest_file_sync = f(file_path: string): Sha256Digest errors FileOpenError, FileReadError, EmptyInputError =>
    return digest
```

Type wrappers enforce algorithm identity at compile time while still allowing explicit conversion to and from text.

## Example Applications

- `typed_digest_operations.op`: in-memory typed hashing, parsing, and conversion workflows
- `typed_digest_file_sync.op`: synchronous file-backed typed digest operations with explicit I/O errors
- `typed_digests.types.op`: digest wrapper, metadata, and error type declarations

## Strengths

- Strongest digest-safety guarantees through type distinctions
- Prevents accidental comparison of incompatible algorithm outputs
- Keeps migration paths explicit with parse and conversion functions
- Works well with structured metadata records in long-lived systems

## Weaknesses

- Most verbose API surface due to algorithm-specific wrappers
- Higher onboarding overhead for simple one-off scripts
- Adding a new algorithm introduces additional wrapper and helper symbols

## Impact on Existing Syntax

None. The proposal is additive and uses only existing Opalescent types, constructors, and function signatures.

## Interactions with Other Concerns

- **error-strategy/layered-error-wrapping**: parse failures can be wrapped with context when crossing module boundaries
- **byte-buffer-type/dedicated-bytes-type**: can later provide wrapper constructors from `Bytes` without API redesign
- **serialization/json-plus-toml-uniform-api**: typed digest wrappers serialize as explicit hex fields in config payloads

## Implementation Difficulty

Medium. The hashing core is straightforward, but wrapper construction, parsing, and conversion consistency across all algorithms increases the amount of surface to validate.

## Must NOT Have

- No generic untyped `Digest` replacement that collapses algorithm identity
- No exceptions or implicit conversion between algorithm wrappers
- No deferred keyword usage or unsuffixed file-backed digest helpers
- No shorthand type names that weaken clarity
