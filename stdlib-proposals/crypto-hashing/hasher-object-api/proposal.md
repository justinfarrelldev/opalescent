# hasher-object-api

## Overview

This alternative models hashing as an explicit stateful `Hasher` value. Callers create a hasher with `new_hasher`, feed chunks through `update`, and obtain the digest with `finalize`. File-backed incremental hashing is exposed as `hash_stream_file_sync`.

The shape is optimized for large payloads and streaming workflows while keeping Opalescent error handling explicit. Lifecycle misuse (such as updating a finalized hasher) is surfaced as typed errors instead of implicit behavior.

## Assumes

- Algebraic types from Opalescent are used to model hasher state
- `uint8[]` is the active byte transport representation
- Synchronous file reads exist for chunk-based updates
- Error handling follows `guard` and `propagate` only

## Syntax Design

```opal
let new_hasher = f(algorithm: HashAlgorithm): Hasher =>
    return hasher

let update = f(hasher: Hasher, next_chunk_bytes: uint8[]): Hasher errors EmptyInputChunkError, HasherFinalizedError =>
    return updated_hasher

let finalize = f(hasher: Hasher): string errors HasherFinalizedError =>
    return digest_hex

let hash_stream_file_sync = f(file_path: string, algorithm: HashAlgorithm): string errors InvalidFilePathError, FileOpenError, FileReadError, EmptyInputChunkError, HasherFinalizedError =>
    return digest_hex
```

Pure object lifecycle calls remain unsuffixed. Only the file-backed stream path carries `_sync`.

## Example Applications

- `incremental_hashing.op`: in-memory chunk updates and finalization flow for telemetry batches
- `file_stream_hashing_sync.op`: blocking stream hashing from file path with path and I/O validation
- `full_workflow_sync.op`: release verification flow that composes stream hashing with summary generation
- `hasher_object.types.op`: hasher, algorithm, and error types for lifecycle-safe usage

## Strengths

- Natural support for incremental and streaming workloads
- Strong lifecycle safety via explicit `HasherFinalizedError`
- Easy bridge from one-shot hashing to chunk-based workflows
- Clean placement for future performance extensions like reusable internal buffers

## Weaknesses

- More ceremony than direct one-shot function calls
- Requires clear guidance for when to use `reset`
- Slightly larger mental model due to explicit state transitions

## Impact on Existing Syntax

None. The proposal is library-level and uses existing constructors, signatures, and error clauses.

## Interactions with Other Concerns

- **file-io-surface/handle-based**: aligns with chunked reading and explicit handle usage
- **error-strategy/registered-error-hierarchy**: lifecycle and I/O errors can be centrally grouped
- **byte-buffer-type/raw-uint8-array**: straightforward with chunk arrays and no wrapper conversions

## Implementation Difficulty

Medium. Requires careful state transitions and consistent behavior across update/finalize/reset operations in addition to synchronous chunked file reading.

## Must NOT Have

- No hidden mutable globals storing hasher state
- No exceptions or non-typed error channels
- No deferred API surface in this concern phase
- No unsuffixed file-backed stream hashing function names
