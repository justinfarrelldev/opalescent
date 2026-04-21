# File I/O Surface Alternatives Comparison

This document compares three alternative designs for the Opalescent standard library file I/O surface: **Whole-File Operations**, **Handle-Based**, and **Path-Object-Centric**.

## Summary Matrix

| Axis | Whole-File Operations | Handle-Based | Path-Object-Centric |
|------|-----------------------|--------------|---------------------|
| **Ergonomics** | ★★★★★ | ★★★☆☆ | ★★★★☆ |
| **Error-model fit** | ★★★★★ | ★★★★☆ | ★★★★★ |
| **Opalescent-idiom fit** | ★★★★★ | ★★★☆☆ | ★★★★☆ |
| **Implementation effort** | Low (1-2mo) | Medium (3-4mo) | Low (2mo) |
| **Extensibility** | ★★★☆☆ | ★★★★★ | ★★★★☆ |
| **Async readiness** | ★★★☆☆ | ★★★★★ | ★★★★☆ |

## Analysis

### Whole-File Operations
- **Ergonomics**: Highest ergonomics for common app-level tasks. Reading or writing a file is a single atomic-feeling call.
- **Error-model fit**: Perfect fit. Since operations are atomic, the error set is predictable and easier to handle in a single `guard` block.
- **Opalescent-idiom fit**: Fits the "simple and explicit" mantra well for high-level logic.
- **Async readiness**: Poor. While it can be wrapped in an deferred task, it doesn't provide a foundation for streaming.

### Handle-Based
- **Ergonomics**: More boilerplate required for simple tasks (open -> read -> close).
- **Opalescent-idiom fit**: Slightly less idiomatic due to the manual resource management (closing handles), which contrasts with the language's focus on automated safety.
- **Extensibility**: Best for performance-critical or specialized I/O (random access, sparse files).
- **Async readiness**: Excellent. Handles are the natural foundation for non-blocking I/O and event loops.

### Path-Object-Centric
- **Ergonomics**: Good balance. Separating path logic from I/O logic reduces string-processing bugs.
- **Error-model fit**: Strong. By validating paths as objects, some errors (like `InvalidPathError`) can be caught earlier in the CPU-only phase.
- **Implementation effort**: Low. Primarily involves building a robust path manipulation library.
- **Extensibility**: Good. The `FilesystemPath` object can be extended with metadata or platform-specific properties without changing I/O signatures.
