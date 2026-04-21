# Optional Representation Comparison

This document compares three alternative approaches for representing optional values (absence of data) in Opalescent.

### Summary Matrix

| Axis | Absence via Errors | Maybe Tagged Union | Nullable Sentinel Types |
|------|-------------------|-------------------|------------------------|
| **Ergonomics** | ★★★☆☆ | ★★★★☆ | ★★★★★ |
| **Error-model fit** | ★★★★★ | ★★★☆☆ | ★★☆☆☆ |
| **Opalescent-idiom fit** | ★★★★☆ | ★★★★★ | ★★★☆☆ |
| **Implementation effort** | Low | Medium | Very Low |
| **Extensibility** | ★★★★☆ | ★★★★★ | ★★☆☆☆ |
| **Async readiness** | ★★★★☆ | ★★★★☆ | ★★★☆☆ |

### Analysis

#### Absence via Errors
- **Ergonomics**: Decent, though requires explicit handling of each error case via `guard`.
- **Error-model fit**: Perfect. It leverages the existing error-handling machinery without adding new concepts.
- **Opalescent-idiom fit**: Strong. Opalescent favors explicit error handling, and this pattern is consistent with that.
- **Implementation effort**: Minimal. It uses the existing compiler infrastructure for errors.

#### Maybe Tagged Union
- **Ergonomics**: High. A single type `Maybe T` is very intuitive and allows for generic utility functions.
- **Error-model fit**: Moderate. It can sometimes conflict with error handling when a function needs to return both an error and a "not found" state.
- **Opalescent-idiom fit**: Perfect. It aligns with the "explicit and type-safe" philosophy and the use of tagged unions.
- **Implementation effort**: Moderate. Requires standardizing the type and potentially adding library support.

#### Nullable Sentinel Types
- **Ergonomics**: Excellent for simple cases, though it requires manual checking and documentation.
- **Error-model fit**: Poor. It bypasses the error model completely, leading to potential "silent failures."
- **Opalescent-idiom fit**: Moderate. It is a pragmatic choice but lacks the strictness of the other two options.
- **Implementation effort**: Very Low. It is a matter of convention rather than a compiler feature.
