# Streaming Sync Readers and Writers

## Overview
This proposal introduces a low-level, memory-bounded API for JSON serialization using a streaming model. It's designed for processing large JSON documents that cannot fit in memory. The API provides a `JsonStreamReader` to pull tokens one-by-one and a `JsonStreamWriter` to push values sequentially.

The streaming functions are explicitly marked with a `_sync` suffix to indicate their blocking nature and to distinguish them from future deferred alternatives.

## Assumes
- Standard synchronous I/O or byte buffer operations.
- Opacity for `JsonStreamReader` and `JsonStreamWriter` handles.

## Syntax Design
No new syntax is required. The API uses standard functions and types, following a pull-based model for reading and a push-based model for writing.

```opal
import JsonStreamReader, JsonToken from ./serialization_errors.types
import MalformedJsonError from ./serialization_errors.types

let read_next_token_sync = f(reader: JsonStreamReader): JsonToken errors MalformedJsonError =>
    # Implementation logic
    return JsonToken.StartObject
```

## Example Applications
Processing a large array of objects without loading the whole array into memory:
```opal
import JsonStreamReader, read_next_token_sync from ./serialization_errors.types

let process_large_json = f(reader: JsonStreamReader): void errors MalformedJsonError =>
    # Read tokens in a loop
    return void
```

## Strengths
- Constant memory overhead regardless of document size.
- High performance for data ingestion and export.
- Explicit and predictable.

## Weaknesses
- Much higher developer complexity to use correctly compared to tree-based APIs.
- Prone to logic errors when tracking nested structures manually.

## Impact on Existing Syntax
None. Pure library addition.

## Interactions with Other Concerns
Consistent with the error model and future deferred I/O potential.

## Implementation Difficulty
High. Requires robust state-machine-based parsers and emitters that can suspend/resume.

## Must NOT Have
- Higher-level mapping built into the stream.
- Automatic document buffering.
