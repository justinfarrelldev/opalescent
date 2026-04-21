# Dedicated Bytes Type
<!-- Provide a clear, descriptive name for this language alternative or feature proposal. -->

## Overview
This proposal introduces a dedicated `Bytes` type to represent byte buffers. The `Bytes` type is a struct that wraps a raw `uint8[]` array, providing a clear semantic distinction between generic numeric arrays and binary data buffers.

By centralizing byte operations under this type, we improve code readability and enable more expressive APIs. The `Bytes` type serves as a foundation for all binary processing in Opalescent.

## Assumes
This proposal assumes the existence of structs and the `uint8` primitive type. It relies on the ability to define custom types in `.types.op` files and import them across the project.

## Syntax Design
A new `Bytes` type is defined in the `bytes` module. Operations are provided as functions that specifically accept and return `Bytes` instances.

```opal
type Bytes:
    data: uint8[]
    length: int32

# Usage
let buffer = new Bytes:
    data: [1, 2, 3]
    length: 3
```

## Example Applications
Processing binary data becomes more explicit with the `Bytes` type.

```opal
import Bytes from ./bytes.types
import read_file_sync from standard
import bytes_to_hex_string from standard

let hash_file = f(path: string): string errors FileReadError, FileNotFoundError =>
    let raw_data = propagate read_file_sync(path)
    let b = new Bytes:
        data: raw_data
        length: 100 # assuming fixed length for example
    return bytes_to_hex_string(b)
```

## Strengths
- **Semantic Clarity**: Distinguishes binary data from general `uint8` arrays.
- **Type Safety**: Prevents accidental mixing of byte buffers with other array types.
- **Extensibility**: The `Bytes` struct can be expanded with metadata (e.g., capacity, encoding hints) without breaking function signatures.
- **Organization**: Grouping operations like `concatenate_bytes` and `slice_bytes` creates a cohesive binary API.

## Weaknesses
- **Boilerplate**: Requires wrapping and unwrapping raw arrays.
- **Complexity**: Adds a new type to the standard library that developers must learn.
- **Overhead**: Minimal but non-zero overhead for the struct wrapper.

## Impact on Existing Syntax
This proposal requires no changes to existing syntax but introduces a new standard type.

## Interactions with Other Concerns
- **FFI**: The `Bytes` type provides a natural mapping for C-style buffers (pointer + length).
- **Concurrency**: The struct can be treated as an atomic unit for transfer between threads.
- **Error Handling**: Uses the standard `guard` and `propagate` mechanisms.

## Implementation Difficulty
Medium. Requires defining the new type and implementing a dedicated suite of operations in the standard library.

## Must NOT Have
- This proposal must NOT use `[uint8]` syntax.
- It must NOT use semicolons.
- It must NOT introduce `ByteString` or `Buffer` as alternative names.
