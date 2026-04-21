# Testing Framework Alternatives Comparison

This document compares five testing-framework alternatives for Opalescent using the fixed six-axis schema.

## Summary Matrix

| Axis | Vitest-Style Describe/It | Flat `test()` Function | Spec Object Style | Property-Based Testing | Snapshot Testing |
|------|---------------------------|------------------------|-------------------|------------------------|------------------|
| **Ergonomics** | ★★★★★ | ★★★★☆ | ★★★☆☆ | ★★★☆☆ | ★★★★☆ |
| **Error-model fit** | ★★★★★ | ★★★★★ | ★★★★☆ | ★★★★☆ | ★★★★☆ |
| **Opalescent-idiom fit** | ★★★★☆ | ★★★★★ | ★★★★☆ | ★★★★☆ | ★★★☆☆ |
| **Implementation effort** | High | Low | Medium | High | Medium |
| **Extensibility** | ★★★★★ | ★★★★☆ | ★★★★★ | ★★★★★ | ★★★★☆ |
| **Async readiness** | ★★★★☆ | ★★★☆☆ | ★★★★☆ | ★★★★☆ | ★★★☆☆ |

## Analysis

### Vitest-Style Describe/It
- **Ergonomics**: Familiar to developers from JS ecosystems and very readable in nested suites.
- **Error-model fit**: Strong fit because assertion and hook failures can stay explicit through `errors` and `guard` patterns.
- **Implementation effort**: Highest due to mocks, stubs, spies, matcher chains, and lifecycle orchestration.

### Flat `test()` Function
- **Opalescent-idiom fit**: Closest to concise expression-oriented style with minimal ceremony.
- **Implementation effort**: Lowest path to a stable MVP for the standard library.
- **Extensibility**: Can layer richer features later, but less discoverable than suite-oriented APIs.

### Spec Object Style
- **Extensibility**: Strong for metadata-driven execution, filtering, retries, and toolchain hooks.
- **Error-model fit**: Good because each spec function can expose explicit error contracts.
- **Ergonomics**: More verbose than callback-driven styles for small modules.

### Property-Based Testing
- **Extensibility**: Excellent for evolving generator ecosystems and shrinking strategies.
- **Implementation effort**: High due to deterministic seeds, shrinking engine, and failure reporting.
- **Opalescent-idiom fit**: Good if APIs preserve explicit data flow and deterministic behavior.

### Snapshot Testing
- **Ergonomics**: Excellent for large structured outputs with low assertion noise.
- **Opalescent-idiom fit**: Moderate because snapshot approval workflows are more tool-driven than language-native.
- **Async readiness**: Adequate, but snapshot storage and update workflows may require additional runner protocol design.

## Final Recommendation
Prioritize **vitest-style-describe-it** as the primary design target because it delivers the most complete user-facing testing surface in one model, including mocking, stubbing, and spying. Keep **test-function-flat** as the fallback MVP path if implementation capacity is constrained, and treat property/snapshot capabilities as first-party extensions that plug into the same assertion and runner core.
