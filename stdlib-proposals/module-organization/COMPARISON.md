# Standard Library Module Organization Comparison

This document compares three alternatives for organizing the Opalescent standard library. Each model offers different trade-offs in terms of ergonomics, scalability, and implementation complexity.

## Summary Matrix

| Axis | Flat Bare Specifiers | Namespaced Stdlib | Tiered Stdlib |
|------|----------------------|-------------------|---------------|
| **Ergonomics** | ★★★★★ | ★★★★☆ | ★★★☆☆ |
| **Error-model fit** | ★★★★★ | ★★★★★ | ★★★★★ |
| **Opalescent-idiom fit** | ★★★★★ | ★★★★☆ | ★★★★☆ |
| **Implementation effort** | Low | Medium | High |
| **Extensibility** | ★★★☆☆ | ★★★★★ | ★★★★☆ |
| **Async readiness** | ★★★★☆ | ★★★★☆ | ★★★★☆ |

## Analysis

### Flat Bare Specifiers
The simplest model, where every stdlib area gets its own top-level name.
- **Ergonomics**: Highest. Shortest possible import paths.
- **Opalescent-idiom fit**: Matches the existing `import ... from math` pattern perfectly.
- **Extensibility**: Limited. As the stdlib grows, top-level namespace pollution becomes a concern.

### Namespaced Stdlib
All stdlib modules are grouped under a single `standard/` root.
- **Ergonomics**: Good. Slightly more verbose due to the prefix, but provides better organization.
- **Extensibility**: Very high. Allows for deep nesting and clear logical grouping without cluttering the global namespace.
- **Implementation effort**: Requires an update to the module resolver to handle hierarchical bare specifiers.

### Tiered Stdlib
Organizes the stdlib into `core`, `standard`, and `standard_extra` levels.
- **Ergonomics**: Moderate. Requires developers to remember which tier a module resides in.
- **Opalescent-idiom fit**: Fits well with a language that prioritizes control over binary size and runtime complexity.
- **Implementation effort**: Highest. Requires coordination between the compiler, the module resolver, and the build system to manage opt-in tiers.
