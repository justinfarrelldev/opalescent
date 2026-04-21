# Comparison: Crypto Hashing API Shape

This document compares three alternatives for the `crypto` hashing concern in Opalescent. All three alternatives keep pure in-memory hashing unsuffixed and reserve `_sync` only for file-backed hashing operations.

## Summary Matrix

| Axis | hash-function-module | hasher-object-api | typed-digest-wrappers |
|------|----------------------|-------------------|-----------------------|
| **Ergonomics** | ★★★★★ | ★★★★☆ | ★★★☆☆ |
| **Error-model fit** | ★★★★☆ | ★★★★★ | ★★★★☆ |
| **Opalescent-idiom fit** | ★★★★☆ | ★★★★☆ | ★★★★★ |
| **Implementation effort** | Low (1-2 weeks) | Medium (2-3 weeks) | Medium (2-4 weeks) |
| **Extensibility** | ★★★☆☆ | ★★★★★ | ★★★★☆ |
| **Async readiness** | ★★★★☆ | ★★★★★ | ★★★★☆ |

## Analysis

### hash-function-module
- **Ergonomics**: Best for one-shot hashing, very little setup, clear call sites.
- **Error-model fit**: Strong fit. Only stream/file paths carry explicit `errors` lists.
- **Opalescent-idiom fit**: Good explicit function surface, but weaker state modeling.
- **Implementation effort**: Smallest implementation surface and easiest migration path.
- **Extensibility**: Adding new algorithms grows one module but can become crowded.
- **Async readiness**: File-backed API already isolated as `hash_file_sync`.

### hasher-object-api
- **Ergonomics**: Slightly more ceremony, but excellent for incremental data pipelines.
- **Error-model fit**: Excellent because lifecycle mistakes are explicit typed errors.
- **Opalescent-idiom fit**: Good fit using `new TypeName:` constructors and clear state values.
- **Implementation effort**: Requires state transitions and finalize/reset semantics.
- **Extensibility**: Best long-term shape for chunked streams and advanced modes.
- **Async readiness**: Incremental API can map cleanly to future non-blocking readers.

### typed-digest-wrappers
- **Ergonomics**: Most verbose due to algorithm-specific function names.
- **Error-model fit**: Strong and explicit, especially for parse/length mismatch failures.
- **Opalescent-idiom fit**: Best compile-time safety by making digest confusion impossible.
- **Implementation effort**: Moderate, because every algorithm gets dedicated wrappers.
- **Extensibility**: Good, though adding algorithms adds many symbols.
- **Async readiness**: File-backed methods already isolated with `_sync` suffixes.

## Recommendation

`hasher-object-api` is the strongest default for stdlib evolution because it handles both one-shot and incremental workflows while keeping error paths explicit and future stream integration straightforward.

`hash-function-module` remains the easiest onboarding shape, and `typed-digest-wrappers` is ideal when compile-time digest safety matters more than API brevity.
