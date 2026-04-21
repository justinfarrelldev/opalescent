# hash-function-module

## Overview

This alternative exposes a flat `crypto` module with named hash functions for one-shot in-memory digest computation: `sha256`, `sha512`, `blake3`, and `md5`. The same module also provides file-backed hashing through `hash_file_sync` so blocking operations are clearly marked with `_sync`.

The design favors minimal ceremony and direct readability. Developers pick an algorithm function and call it, using standard Opalescent `guard` or `propagate` patterns for every fallible operation.

## Assumes

- `uint8[]` is the canonical byte representation from `byte-buffer-type`
- `error-strategy/error-code-enum-module` or equivalent typed error surfaces exist
- Local helper types live in `crypto_hashing.types.op`
- No deferred runtime is present in this phase, so file hashing remains synchronous

## Syntax Design

```opal
let sha256 = f(input_bytes: uint8[]): string errors EmptyInputError =>
    # ...
    return digest_hex

let hash_file_sync = f(file_path: string, algorithm: HashAlgorithm): string errors FileOpenError, FileReadError, EmptyInputError =>
    # ...
    return digest_hex
```

The one-shot functions are unsuffixed because they are pure in-memory operations. File-backed hashing is explicitly synchronous (`hash_file_sync`) to reserve unsuffixed naming for future deferred decisions.

## Example Applications

- `one_shot_hashing.op`: startup payload hashing across all named algorithms
- `file_hashing_sync.op`: artifact verification from disk with algorithm selection and explicit error branches
- `crypto_hashing.types.op`: error and supporting type declarations used by both scenarios

## Strengths

- Very approachable API with almost no onboarding overhead
- Strong compatibility with scripts and CLI-like usage patterns
- Keeps pure and file-backed flows visibly distinct via `_sync`
- Easy to document and teach because algorithm names are first-class entry points

## Weaknesses

- Flat surfaces can become crowded as more algorithms and options are added
- Incremental hashing requires extra helper functions rather than a dedicated state model
- Digest values are plain strings, so algorithm confusion is possible at call sites

## Impact on Existing Syntax

None. This is a standard library shape proposal using existing function signatures, `errors` clauses, and `guard` handling.

## Interactions with Other Concerns

- **byte-buffer-type/dedicated-bytes-type**: can later swap `uint8[]` parameters to `Bytes` wrappers if adopted
- **error-strategy/error-code-enum-module**: maps naturally to explicit hashing/file error enums
- **file-io-surface/whole-file-operations**: aligns with whole-file read model feeding one-shot digesting

## Implementation Difficulty

Low. The API surface is straightforward and mostly wraps existing digest primitives plus a synchronous file-reading path.

## Must NOT Have

- No exceptions, `try/catch`, or hidden failure channels
- No deferred variants in this concern phase
- No semicolons or non-Opalescent signature forms
- No unsuffixed file-backed hashing function names
