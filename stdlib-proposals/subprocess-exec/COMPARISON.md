# Subprocess Execution Comparison

## Summary Matrix

| Axis | run-command-function | command-builder | process-handle |
|------|----------------------|-----------------|----------------|
| **Ergonomics** | ★★★★★ | ★★★★☆ | ★★★☆☆ |
| **Error-model fit** | ★★★★☆ | ★★★★★ | ★★★★★ |
| **Opalescent-idiom fit** | ★★★★★ | ★★★★☆ | ★★★★☆ |
| **Implementation effort** | Low (small API) | Medium (builder surface) | Medium-High (stateful handle) |
| **Extensibility** | ★★☆☆☆ | ★★★★☆ | ★★★★★ |
| **Async readiness** | ★★★☆☆ | ★★★★☆ | ★★★★★ |

## Analysis

### run-command-function
- **Ergonomics**: Fastest path for common one-shot commands.
- **Error-model fit**: Strong fit, but all behavior funnels through one entry point.
- **Implementation effort**: Lowest effort and easiest to teach.
- **Extensibility**: Limited when callers need incremental process interaction.

### command-builder
- **Ergonomics**: Clear for commands with many args and environment values.
- **Error-model fit**: Works naturally with `guard` and `propagate` around `run_sync`.
- **Implementation effort**: Moderate due to builder state and validation rules.
- **Extensibility**: Good base for adding cwd, stdin piping, and policy flags later.

### process-handle
- **Ergonomics**: More ceremony, but best for long-running child processes.
- **Error-model fit**: Excellent because each lifecycle stage has explicit error types.
- **Implementation effort**: Highest complexity due to handle state transitions.
- **Extensibility**: Best long-term shape for future streaming and deferred-compatible APIs.
