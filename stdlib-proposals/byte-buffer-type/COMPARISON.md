# Comparison: Byte Buffer Representation

This document compares two approaches for representing byte buffers in Opalescent: using raw `uint8[]` arrays directly versus introducing a dedicated `Bytes` struct.

### Summary Matrix

| Axis | raw-uint8-array | dedicated-bytes-type |
|------|---------------|---------------|
| **Ergonomics** | ★★★☆☆ | ★★★★☆ |
| **Error-model fit** | ★★★★★ | ★★★★★ |
| **Opalescent-idiom fit** | ★★★★★ | ★★★★☆ |
| **Implementation effort** | Low (1mo) | Medium (3mo) |
| **Extensibility** | ★★☆☆☆ | ★★★★★ |
| **Async readiness** | ★★★★☆ | ★★★★☆ |

### Analysis

#### raw-uint8-array: The Minimalist Approach
- **Ergonomics**: Functional but slightly clunky. Developers must call free functions like `concatenate_byte_arrays(a, b)` rather than more intuitive methods. However, it requires no new concepts.
- **Opalescent-idiom fit**: Matches the language's preference for using primitive types and simple arrays wherever possible.
- **Implementation effort**: Extremely low. It only requires adding utility functions to the standard library.
- **Extensibility**: Limited. It's difficult to add metadata or change the underlying representation without breaking every function signature that takes `uint8[]`.

#### dedicated-bytes-type: The Structured Approach
- **Ergonomics**: Provides better semantic clarity and allows for more expressive APIs centered around the `Bytes` type.
- **Opalescent-idiom fit**: Aligns well with the use of records for structured data, though it adds a layer of wrapping that the minimalist approach avoids.
- **Implementation effort**: Requires defining the type, constructor logic, and a suite of operations, posing a moderate maintenance burden.
- **Extensibility**: Excellent. The `Bytes` struct can easily be updated to include capacity, encoding hints, or reference-counting metadata without affecting public function signatures.
- **Async readiness**: Both approaches are equally ready for future async expansion, as they both rely on standard data passing patterns that can be wrapped in async-aware structures later.
