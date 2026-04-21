# compress-decompress-functions

## Overview
This alternative proposes a flat, function-first compression module that emphasizes direct operations over object lifecycle management. In-memory compression uses unsuffixed functions (`compress`, `decompress`) because those operations are pure CPU transformations on `ByteBuffer` values.

File-backed operations use explicit `_sync` suffixes (`compress_file_sync`, `decompress_file_sync`) to reserve unsuffixed names for a future deferred decision. The model is optimized for straightforward application code such as startup cache generation, backup rotation, and one-shot archive transformations.

## Assumes
- The byte-buffer concern provides a `ByteBuffer` type suitable for binary payload transport.
- The error-strategy concern continues using explicit `errors` clauses with `guard` and `propagate`.
- Compression algorithms are represented as a closed enum (`CompressionAlgorithm`) instead of stringly-typed inputs.
- No deferred surface is introduced in this proposal.

## Syntax Design
```opal
let compress = f(data: ByteBuffer, algorithm: CompressionAlgorithm): ByteBuffer errors CompressionAlgorithmNotSupportedError, CompressionEncodingError =>
    return compressed_bytes

let decompress = f(compressed_data: ByteBuffer, algorithm: CompressionAlgorithm): ByteBuffer errors CompressionAlgorithmNotSupportedError, CompressionDecodingError, CompressionDataCorruptionError =>
    return raw_bytes

let compress_file_sync = f(source_path: string, destination_path: string, algorithm: CompressionAlgorithm): void errors InvalidPathError, FileReadError, FileWriteError, CompressionAlgorithmNotSupportedError, CompressionEncodingError =>
    return void

let decompress_file_sync = f(source_path: string, destination_path: string, algorithm: CompressionAlgorithm): void errors InvalidPathError, FileReadError, FileWriteError, CompressionAlgorithmNotSupportedError, CompressionDecodingError, CompressionDataCorruptionError =>
    return void
```

## Example Applications
- `compression_functions.op`: API surface sketch for the four public operations and explicit error contracts.
- `startup_asset_cache.op`: In-memory round-trip validation path that calls `compress` and `decompress` with `guard` handling.
- `nightly_backup_rotation.op`: File-backed maintenance workflow calling `compress_file_sync` and `decompress_file_sync` with explicit failure propagation.

## Strengths
- Very low ceremony for typical callers that only need one-shot compression.
- Matches existing Opalescent function-oriented style and explicit error signatures.
- Easy adoption path for teams migrating from utility-function compression APIs.
- Keeps the object model out of small programs that do not process streams.

## Weaknesses
- Not ideal for very large payload streams where chunk-by-chunk processing is required.
- Stateful tuning options (window reuse, partial flush strategies) are awkward to add.
- Repeated operation pipelines may allocate more temporary buffers than an object model.

## Impact on Existing Syntax
No parser or core language changes are required. This is a pure standard-library surface proposal using existing function signatures, enums, type imports, and error-handling constructs.

## Interactions with Other Concerns
- **byte-buffer-type/dedicated-bytes-type**: Strong fit because `ByteBuffer` centralizes binary semantics.
- **error-strategy/error-code-enum-module**: Fully compatible through explicit typed error sets and `guard` branches.
- **file-io-surface/whole-file-operations**: Naturally aligned for one-shot file compression tasks.
- **module-organization** alternatives: Works as either a single `compression` module or a `compression/functions` submodule.

## Implementation Difficulty
Low. The API maps directly to common one-shot codec primitives and requires modest glue code for file reads/writes plus typed error mapping.

## Must NOT Have
- Must NOT introduce exceptions, try/catch, or `Result<T, E>` wrappers.
- Must NOT include deferred keywords or unsuffixed file-backed operations.
- Must NOT replace enum algorithms with unvalidated string algorithm names.
- Must NOT allow fallible example calls without `guard` or `propagate` handling.
