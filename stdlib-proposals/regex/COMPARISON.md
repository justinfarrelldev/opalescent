# Regex API Surface Comparison

## Summary Matrix

| Axis | Compiled Regex Handle | Regex Module Functions | Pattern Type with Captures |
|------|------------------------|------------------------|----------------------------|
| **Ergonomics** | ★★★★☆ | ★★★★★ | ★★★☆☆ |
| **Error-model fit** | ★★★★★ | ★★★★☆ | ★★★★★ |
| **Opalescent-idiom fit** | ★★★★★ | ★★★★☆ | ★★★★☆ |
| **Implementation effort** | Medium (Parser + runtime cache) | Low (Thin stdlib wrappers) | High (Type flow + capture typing) |
| **Extensibility** | ★★★★★ | ★★★☆☆ | ★★★★★ |
| **Async readiness** | ★★★★☆ | ★★★☆☆ | ★★★★★ |

## Analysis

### Compiled Regex Handle
- **Ergonomics**: A one-time compile step adds slight setup, but repeated operations become concise and readable.
- **Error-model fit**: `compile_regex` naturally carries `InvalidPattern`, and later handle calls stay explicit with `errors` clauses.
- **Opalescent-idiom fit**: Explicit resource-like values and method calls match expression-oriented, explicit Opalescent style.
- **Implementation effort**: Requires a stable `Regex` runtime object and method dispatch, but no advanced type inference.
- **Extensibility**: Easy to add options, flags, streaming scanners, and future specialized matching APIs on the handle.
- **Async readiness**: CPU-bound today, yet handle ownership shape can map cleanly if deferred text streams arrive later.

### Regex Module Functions
- **Ergonomics**: Fastest for simple use cases because callers pass pattern and input directly in one call.
- **Error-model fit**: Works with explicit `errors`, but repeated calls repeat pattern validation and duplicate failure points.
- **Opalescent-idiom fit**: Functional style aligns with existing module-first APIs, though less explicit about reuse intent.
- **Implementation effort**: Smallest implementation: expose pure functions that compile and execute internally.
- **Extensibility**: Growth becomes awkward when adding precompiled options, per-pattern state, or advanced knobs.
- **Async readiness**: Flat functions can gain deferred siblings later, but no stable object to carry incremental state.

### Pattern Type with Captures
- **Ergonomics**: More verbose upfront, but best downstream clarity because captures are named and typed by design.
- **Error-model fit**: Strong fit since invalid pattern, missing captures, and conversion issues can be enumerated precisely.
- **Opalescent-idiom fit**: Typed result maps and explicit pattern construction fit static, explicit Opalescent semantics.
- **Implementation effort**: Highest cost due to capture metadata propagation through checker, runtime, and diagnostics.
- **Extensibility**: Most future-proof path for richer compile-time checking, parser integration, and typed regex transforms.
- **Async readiness**: Rich typed pattern object can later support deferred scanners while preserving typed capture contracts.
