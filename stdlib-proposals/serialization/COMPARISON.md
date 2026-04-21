# Serialization Alternatives Comparison

This document evaluates four proposed approaches for serialization in Opalescent, comparing them across six standardized axes.

## Summary Matrix

| Axis | JSON-Only Tree | JSON + TOML Uniform | Typed Derive Style | Streaming Sync |
|------|----------------|---------------------|-------------------|----------------|
| **Ergonomics** | ★★★★☆ | ★★★★☆ | ★★★★★ | ★★☆☆☆ |
| **Error-model fit** | ★★★★☆ | ★★★★☆ | ★★★★★ | ★★★★☆ |
| **Opalescent-idiom fit** | ★★★★☆ | ★★★★☆ | ★★★★★ | ★★★★☆ |
| **Implementation effort** | Low | Medium | High | High |
| **Extensibility** | ★★★☆☆ | ★★★★☆ | ★★★★★ | ★★★★★ |
| **Async readiness** | ★★★☆☆ | ★★★☆☆ | ★★★★☆ | ★★★★★ |

## Analysis

### JSON-Only Value Tree
- **Ergonomics**: Very high for simple scripts and quick data processing.
- **Opalescent-idiom fit**: Fits perfectly with the "explicit but concise" mantra.
- **Implementation effort**: Lowest effort; fastest to ship as an initial feature.

### JSON + TOML Uniform API
- **Ergonomics**: Excellent for CLI tools and services needing configuration flexibility.
- **Extensibility**: The most easily extended to other tree-like formats (e.g. YAML if ever needed).
- **Implementation effort**: Higher than JSON-only due to two separate parsers.

### Typed Derive Style
- **Ergonomics**: The gold standard for application development. Reduces boilerplate to near-zero.
- **Error-model fit**: Excellent; can produce highly specific error types for individual fields.
- **Async readiness**: Natural mapping to deferred data sources if built on a streaming core.

### Streaming Sync Readers and Writers
- **Ergonomics**: Low, but intended for a specific high-performance niche.
- **Implementation effort**: Very high; requires complex state machines in the parser and emitter.
- **Async readiness**: Perfectly aligned; synchronous `_sync` functions provide a direct template for future `deferred` counterparts.

## Final Recommendation
For the initial Opalescent standard library, **JSON-Only Value Tree** should be prioritized as the MVP, followed by **Streaming Sync** for performance-critical systems, then **Typed Derive Style** once the language's meta-programming story matures.
