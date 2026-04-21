# stream-compressor-object

## Overview
This alternative proposes a stateful streaming API centered on `Compressor` and `Decompressor` objects that process chunked data over multiple calls. Callers explicitly create a stream object, feed chunks through `*_write_chunk` methods, and finalize with `*_finish`.

The model is designed for large payloads that should not be loaded into memory as a single buffer. File-backed operations are still provided as convenience wrappers and are suffixed with `_sync` (`compress_file_sync`, `decompress_file_sync`) to preserve naming space for future deferred variants.

## Assumes
- The byte-buffer concern provides a `ByteBuffer` type used as the chunk transport unit.
- The error-strategy concern remains explicit and typed through `errors` clauses.
- Opalescent supports constructor syntax (`new TypeName:`) for stream object initialization.
- No deferred syntax is introduced, but API shape should remain compatible with future deferred wrappers.

## Syntax Design
```opal
let new_compressor = f(options: CompressionOptions): Compressor errors CompressionAlgorithmNotSupportedError =>
    return compressor

let compressor_write_chunk = f(compressor: Compressor, raw_chunk: ByteBuffer): ByteBuffer errors CompressionStreamStateError, CompressionEncodingError =>
    return compressed_chunk

let compressor_finish = f(compressor: Compressor): ByteBuffer errors CompressionStreamStateError, CompressionEncodingError =>
    return trailing_bytes

let new_decompressor = f(algorithm: CompressionAlgorithm): Decompressor errors CompressionAlgorithmNotSupportedError =>
    return decompressor

let decompressor_write_chunk = f(decompressor: Decompressor, compressed_chunk: ByteBuffer): ByteBuffer errors CompressionStreamStateError, CompressionDecodingError, CompressionDataCorruptionError =>
    return decoded_chunk

let decompressor_finish = f(decompressor: Decompressor): ByteBuffer errors CompressionStreamStateError, CompressionDecodingError, CompressionDataCorruptionError =>
    return trailing_bytes

let compress_file_sync = f(source_path: string, destination_path: string, options: CompressionOptions): void errors InvalidPathError, FileReadError, FileWriteError, CompressionAlgorithmNotSupportedError, CompressionStreamStateError, CompressionEncodingError =>
    return void

let decompress_file_sync = f(source_path: string, destination_path: string, algorithm: CompressionAlgorithm): void errors InvalidPathError, FileReadError, FileWriteError, CompressionAlgorithmNotSupportedError, CompressionStreamStateError, CompressionDecodingError, CompressionDataCorruptionError =>
    return void
```

## Example Applications
- `stream_compressor_api.op`: Defines the complete public stream object surface and all file-backed sync wrappers.
- `chunked_video_processing.op`: Demonstrates chunk-fed compression and decompression validation for large media workflows.
- `chunked_archive_jobs.op`: Demonstrates operational file-based archive compression and restore verification using `_sync` wrappers.

## Strengths
- Best fit for large payloads because it avoids forcing whole-data buffering.
- Encodes stream lifecycle explicitly, making chunk flow and finalization rules visible.
- Provides a natural long-term extension point for dictionary reuse and flush controls.
- Keeps future deferred adaptation straightforward by preserving chunk-oriented boundaries.

## Weaknesses
- More verbose than function-first APIs for simple one-shot compression scenarios.
- Introduces lifecycle-state errors that callers must understand and handle correctly.
- Slightly higher implementation complexity due to stream state management.

## Impact on Existing Syntax
No language-level syntax changes are required. This alternative only adds standard-library types and functions using existing declarations, constructors, and error-handling grammar.

## Interactions with Other Concerns
- **byte-buffer-type/dedicated-bytes-type**: Essential for clear chunk semantics and safer binary handling.
- **error-strategy/layered-error-wrapping**: Useful for attaching stream position context to chunk failures.
- **file-io-surface/handle-based**: Conceptually aligned because both models expose explicit operation lifecycles.
- **module-organization** concerns: Can live cleanly in `compression/streaming` while preserving a small top-level import.

## Implementation Difficulty
Medium. Stream state transitions, flush semantics, and chunk boundary guarantees require additional implementation and testing compared with one-shot helper functions.

## Must NOT Have
- Must NOT introduce exceptions, implicit retries, or hidden fallback algorithms.
- Must NOT expose unsuffixed file-backed operations that could collide with future deferred names.
- Must NOT allow incomplete stream usage examples that skip finish calls.
- Must NOT include any deferred keywords or non-Opalescent control constructs.
