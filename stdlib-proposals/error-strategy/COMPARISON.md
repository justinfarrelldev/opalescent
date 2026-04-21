# Error Strategy Comparison

## Summary Matrix

| Axis | Open Error Set | Registered Hierarchy | Error Code Enum | Layered Wrapping |
|------|----------------|----------------------|-----------------|------------------|
| **Ergonomics** | ★★★★★ | ★★★★☆ | ★★★★☆ | ★★★☆☆ |
| **Error-model fit** | ★★★★★ | ★★★★☆ | ★★★★☆ | ★★★★★ |
| **Opalescent-idiom fit** | ★★★★★ | ★★★★☆ | ★★★★★ | ★★★★☆ |
| **Implementation effort** | Low (None) | Low (Architecture) | Low (Patterns) | Medium (Helpers) |
| **Extensibility** | ★★★★★ | ★★★☆☆ | ★★★★☆ | ★★★★★ |
| **Async readiness** | ★★★★☆ | ★★★★☆ | ★★★★☆ | ★★★★★ |

## Analysis

### Open Error Set
- **Ergonomics**: Highest due to no central registry; functions just declare what they signal.
- **Error-model fit**: Perfectly matches the explicit `errors` syntax.
- **Opalescent-idiom fit**: Matches the decentralized and explicit nature of the language.
- **Implementation effort**: Status quo, already part of the specification.
- **Extensibility**: Very high; new libraries can define new errors with zero coordination.
- **Async readiness**: Natural fit for any future async syntax.

### Registered Error Hierarchy
- **Ergonomics**: Slightly more friction due to the need for central registration.
- **Error-model fit**: Works well, but can lead to large, shared error sets.
- **Opalescent-idiom fit**: Fits the "explicit but concise" mantra by grouping common errors.
- **Implementation effort**: Low; mostly a structural decision for the standard library.
- **Extensibility**: Lower; the central module can become a bottleneck.
- **Async readiness**: No special blockers for async integration.

### Error Code Enum Module
- **Ergonomics**: Pleasant due to clear, module-scoped error enums.
- **Error-model fit**: Excellent fit for the `guard` and potential match expressions.
- **Opalescent-idiom fit**: Very high; mirrors the expression-oriented style.
- **Implementation effort**: Very low; just a pattern for library authors to follow.
- **Extensibility**: High; each module manages its own errors.
- **Async readiness**: Works well with future async/await models.

### Layered Error Wrapping
- **Ergonomics**: Slightly higher overhead due to explicit wrapping and unwrapping.
- **Error-model fit**: Perfect for complex systems where context is king.
- **Opalescent-idiom fit**: Good; the language's explicit nature makes the wrapping steps clear.
- **Implementation effort**: Medium; requires standard library helpers and potentially compiler support for "any error" types.
- **Extensibility**: Highest; any error can be extended with any context.
- **Async readiness**: Exceptional; tracing errors across async boundaries requires this kind of context.
