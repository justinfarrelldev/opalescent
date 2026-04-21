# Comparison: Logging

This document compares three standard-library logging alternatives for Opalescent.

## Summary Matrix

| Axis | Global Logger Module | Logger Handle | Structured Log Events |
|------|----------------------|---------------|-----------------------|
| **Ergonomics** | ★★★★★ | ★★★☆☆ | ★★★☆☆ |
| **Error-model fit** | ★★★★☆ | ★★★★★ | ★★★★★ |
| **Opalescent-idiom fit** | ★★★☆☆ | ★★★★★ | ★★★★☆ |
| **Implementation effort** | Low | Medium | Medium-High |
| **Extensibility** | ★★★☆☆ | ★★★★☆ | ★★★★★ |
| **Async readiness** | ★★★☆☆ | ★★★★☆ | ★★★★★ |

## Analysis

### Global Logger Module
- **Ergonomics**: Best day-to-day convenience, because callers avoid threading handles through signatures.
- **Error-model fit**: Good fit with explicit typed errors, though global setup and lifecycle errors are less local to call sites.
- **Opalescent-idiom fit**: Weaker than handle-based models due to hidden module state.
- **Implementation effort**: Lowest implementation cost with one shared buffer and one flush primitive.
- **Extensibility**: Can grow with more log levels and format options, but shared state can constrain evolution.
- **Async readiness**: Possible but requires careful global-state semantics across future schedulers.

### Logger Handle
- **Ergonomics**: More verbose from parameter threading, but dependencies remain explicit and understandable.
- **Error-model fit**: Excellent alignment because creation and flushing errors stay attached to caller-owned values.
- **Opalescent-idiom fit**: Strongest fit with explicit capability passing and reduced implicit state.
- **Implementation effort**: Moderate due to handle lifecycle and buffer ownership APIs.
- **Extensibility**: High because multiple logger implementations can share one handle contract.
- **Async readiness**: Strong since per-task handles map naturally to future deferred contexts.

### Structured Log Events
- **Ergonomics**: More upfront ceremony for event schemas, but clear payloads for long-lived systems.
- **Error-model fit**: Excellent typed composition, especially around sink-creation, event-write, and flush failures.
- **Opalescent-idiom fit**: Strong alignment with typed data-first programming and explicit effects.
- **Implementation effort**: Highest due to encoding strategy, event schema tooling, and sink management.
- **Extensibility**: Best long-term path for analytics, observability, and domain-specific event evolution.
- **Async readiness**: Best foundation because sink/event contracts can gain deferred mirrors without breaking type shape.
