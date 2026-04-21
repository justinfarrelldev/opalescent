# Numeric Math Surface Comparison

This document compares three alternatives for presenting the numeric and bitwise math operations in the Opalescent standard library.

## Summary Matrix

| Axis | Expand Math Module | Split Math Modules | Typed Math Traits |
|------|---------------------|--------------------|-------------------|
| **Ergonomics** | ★★★★★ | ★★★★☆ | ★★★★☆ |
| **Error-model fit** | ★★★★★ | ★★★★★ | ★★★★★ |
| **Opalescent-idiom fit** | ★★★★★ | ★★★★☆ | ★★★★☆ |
| **Implementation effort** | Low | Medium | High |
| **Extensibility** | ★★★☆☆ | ★★★★★ | ★★★★★ |
| **Async readiness** | ★★★★☆ | ★★★★☆ | ★★★★☆ |

## Analysis

### Expand Math Module
Keep the single `math` bare specifier and add all listed functions to it.
- **Ergonomics**: Highest. Short, flat imports with no extra module separators needed.
- **Opalescent-idiom fit**: Fits perfectly with the current `import ... from math` pattern.
- **Implementation effort**: Minimal. No changes to the module resolver or type system.

### Split Math Modules
Divide math into `math/integer`, `math/floating_point`, and `math/bitwise`.
- **Ergonomics**: Good. More verbose imports, but logically grouped.
- **Extensibility**: Very high. Allows for easy addition of new math domains (complex, matrix, etc.) without cluttering a single module.
- **Implementation effort**: Moderate. Requires support for hierarchical bare specifiers.

### Typed Math Traits
Operations are associated with types (e.g., `int64.greatest_common_divisor`).
- **Ergonomics**: Great for discoverability via LSP, but slightly more verbose for call sites.
- **Opalescent-idiom fit**: Matches a more structured, type-centric language flavor.
- **Implementation effort**: Highest. Requires significant work in the compiler to support associated functions and trait-based dispatch.
- **Extensibility**: Excellent. New operations can be added to existing types without needing to import new modules.
