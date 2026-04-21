# Collections API Shape Comparison

This document compares four alternative approaches for the shape and organization of the collections API in Opalescent.

### Summary Matrix

| Axis | Free Function | Method Style | Pipeline Operator | Module per Collection |
|------|---------------|--------------|-------------------|-----------------------|
| **Ergonomics** | ★★☆☆☆ | ★★★★★ | ★★★★☆ | ★★★☆☆ |
| **Error-model fit** | ★★★★★ | ★★★☆☆ | ★★★☆☆ | ★★★★★ |
| **Opalescent-idiom fit** | ★★★★★ | ★★★☆☆ | ★★★★☆ | ★★★★☆ |
| **Implementation effort** | Very Low | Medium | Medium | Low |
| **Extensibility** | ★★★★☆ | ★★★★☆ | ★★★★★ | ★★★★★ |
| **Async readiness** | ★★★★★ | ★★★★☆ | ★★★★★ | ★★★★★ |

### Analysis

#### Free Function API
- **Ergonomics**: Lowest. Deep nesting and long function names are required to avoid collisions.
- **Error-model fit**: Perfect. Standard function calls integrate naturally with `guard` and `propagate`.
- **Implementation effort**: Very Low. No new grammar or dispatch mechanisms required.
- **Opalescent-idiom fit**: Matches the existing patterns in the language spec helpers.

#### Method Style API
- **Ergonomics**: Highest. Intuitive left-to-right chaining and great LSP discoverability.
- **Error-model fit**: Moderate. Chaining fallible methods can be tricky without specialized error handling.
- **Implementation effort**: Medium. Requires a dispatch mechanism based on the receiver type.

#### Pipeline Operator API
- **Ergonomics**: High. Provides chaining without needing methods on types.
- **Opalescent-idiom fit**: Strong. Complements the functional flavor of the language.
- **Extensibility**: Excellent. Can be used with any function from any module.

#### Module per Collection API
- **Ergonomics**: Good. Allows for short names within a clear module context.
- **Implementation effort**: Low. Primarily a standard library organization effort.
- **Extensibility**: Very high. Easy to add new collection modules without naming conflicts.
- **Opalescent-idiom fit**: Consistent with the language's strong module system.
