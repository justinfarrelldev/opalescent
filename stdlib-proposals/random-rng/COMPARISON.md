# Comparison: Random Number Generation

This document evaluates two alternatives for the standard library's random number generation (RNG) interface.

## Summary Matrix

| Axis | Explicit RNG Handle | Thread-Local Default RNG |
|------|----------------------|---------------------------|
| **Ergonomics** | ★★★☆☆ | ★★★★★ |
| **Error-model fit** | ★★★★★ | ★★★★☆ |
| **Opalescent-idiom fit** | ★★★★★ | ★★★★☆ |
| **Implementation effort** | Low | Medium |
| **Extensibility** | ★★★★★ | ★★★★☆ |
| **Async readiness** | ★★★★★ | ★★★☆☆ |

## Analysis

### Explicit RNG Handle
- **Ergonomics**: Requires passing an `RandomNumberGenerator` instance to every function. While more verbose, it makes dependencies explicit.
- **Error-model fit**: Excellent. Seeding and range validation errors are handled through the standard `errors` clause and `guard`/`propagate` mechanisms.
- **Opalescent-idiom fit**: High. Aligns with the language's preference for explicitness and controlled side effects.
- **Implementation effort**: Low. Simple state management without global or thread-local storage.
- **Extensibility**: Easiest to swap implementations or mock for testing.

### Thread-Local Default RNG
- **Ergonomics**: Provides a global default RNG, significantly reducing boilerplate for common tasks.
- **Error-model fit**: Good, but implicit state can sometimes hide where errors (like initialization failure) originate.
- **Opalescent-idiom fit**: High, balancing safety with a smooth developer experience for non-critical tasks.
- **Implementation effort**: Medium. Requires robust thread-local storage management in the runtime.
- **Async readiness**: Moderate. Implicit thread-local state requires careful handling in green-thread or fiber-based deferred models.
