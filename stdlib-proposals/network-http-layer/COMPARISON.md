# HTTP Client Alternative Comparison

This document compares three alternatives for the Opalescent synchronous HTTP client layer.

## Summary Matrix

| Axis | Minimal HTTP Client | Request Builder | Separate Client Type |
|------|---------------------|-----------------|----------------------|
| **Ergonomics** | ★★★★★ | ★★★★☆ | ★★★★☆ |
| **Error-model fit** | ★★★★★ | ★★★★★ | ★★★★★ |
| **Opalescent-idiom fit** | ★★★★★ | ★★★★☆ | ★★★★★ |
| **Implementation effort** | Low (1-2mo) | Medium (2-3mo) | Medium (2-3mo) |
| **Extensibility** | ★★★☆☆ | ★★★★★ | ★★★★★ |
| **Async readiness** | ★★★★☆ | ★★★★★ | ★★★★★ |

## Analysis

### Minimal HTTP Client (Sync)
- **Ergonomics**: Highest for simple scripts and one-off requests.
- **Error-model fit**: Excellent; direct function calls map perfectly to Opalescent's `errors` clause.
- **Implementation effort**: Simplest to implement as a wrapper around existing libraries.
- **Extensibility**: Lowest; adding options requires changing many function signatures.

### Request Builder (Sync)
- **Ergonomics**: Good for complex requests, but more verbose for simple ones.
- **Opalescent-idiom fit**: Fits well with value types, though functional chaining is more verbose without methods.
- **Extensibility**: Highest; new configuration fields can be added to the builder without breaking existing code.
- **Deferred readiness**: Very high; the builder structure maps cleanly to a later `send_deferred()` method.

### Separate Client Type (Sync)
- **Ergonomics**: Excellent for service-oriented applications where many requests share settings.
- **Opalescent-idiom fit**: Fits the language's focus on clear, structured data and resource grouping.
- **Extensibility**: High; client settings can evolve to include connection pooling and other advanced features.
- **Async readiness**: High; the client can easily support both sync and future deferred execution methods.
