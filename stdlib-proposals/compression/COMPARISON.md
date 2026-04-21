# Compression Surface Alternatives Comparison

This document compares two API shapes for compression in the Opalescent standard library: a flat function-first module and a stateful stream object model.

## Summary Matrix

| Axis | compress-decompress-functions | stream-compressor-object |
|------|-------------------------------|--------------------------|
| **Ergonomics** | ★★★★★ | ★★★☆☆ |
| **Error-model fit** | ★★★★★ | ★★★★☆ |
| **Opalescent-idiom fit** | ★★★★★ | ★★★★☆ |
| **Implementation effort** | Low (1-2mo) | Medium (2-4mo) |
| **Extensibility** | ★★★☆☆ | ★★★★★ |
| **Async readiness** | ★★★☆☆ | ★★★★★ |

## Analysis

### compress-decompress-functions
- **Ergonomics**: Best for common whole-buffer and whole-file operations. Callers use direct `compress` and `decompress` functions with minimal setup.
- **Error-model fit**: Very strong because each operation is atomic and integrates naturally with `guard` and `propagate`.
- **Implementation effort**: Lower because it maps directly onto existing one-shot compression libraries.
- **Extensibility**: Moderate, but advanced streaming controls would eventually require adding another API shape.

### stream-compressor-object
- **Ergonomics**: More setup and lifecycle steps, but predictable for large-data workflows.
- **Opalescent-idiom fit**: Still explicit and type-safe, though it introduces more state transitions than function-first modules.
- **Extensibility**: Excellent for chunked processing, future dictionary APIs, and backpressure-aware wrappers.
- **Async readiness**: Strongest option because chunk boundaries map cleanly to future deferred transport surfaces without renaming core operations.
