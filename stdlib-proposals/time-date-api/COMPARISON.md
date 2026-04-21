# Time and Date API Comparison

## Summary Matrix

| Axis | Monotonic and Wall Clock Split | Single Timestamp Type | Calendar-First |
|------|-------------------------------|-----------------------|----------------|
| **Ergonomics** | ★★★☆☆ | ★★★★☆ | ★★★★★ |
| **Error-model fit** | ★★★★★ | ★★★★★ | ★★★★★ |
| **Opalescent-idiom fit** | ★★★★★ | ★★★★☆ | ★★★★☆ |
| **Implementation effort** | Medium (4-6mo) | Low (2-3mo) | High (8-12mo) |
| **Extensibility** | ★★★★★ | ★★★☆☆ | ★★★★☆ |
| **Async readiness** | ★★★★★ | ★★★★★ | ★★★★★ |

## Analysis

### Monotonic and Wall Clock Split
- **Ergonomics**: Slightly lower due to the need to manage two distinct types, but prevents significant categories of bugs.
- **Opalescent-idiom fit**: Perfect fit. Explicitly separates concerns (interval measurement vs. calendar display) and uses clear, descriptive naming conventions.
- **Extensibility**: Highly extensible; the separate types allow for specialized methods without polluting a unified interface.

### Single Timestamp Type
- **Ergonomics**: High. A single type is easy to understand and pass around.
- **Implementation effort**: The simplest to implement as it leverages standard 64-bit integers and minimal platform-specific logic.
- **Extensibility**: Limited. A single integer-backed type can be difficult to extend with additional metadata (like timezones) later.

### Calendar-First
- **Ergonomics**: Highest for general application developers who primarily need to format and display dates.
- **Implementation effort**: Significant effort required to implement calendar arithmetic correctly and efficiently.
- **Async readiness**: All alternatives are equally ready for deferred via the `_sync` suffix convention.
