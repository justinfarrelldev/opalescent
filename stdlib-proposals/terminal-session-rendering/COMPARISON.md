# Terminal Session Rendering Comparison

## Status and scope

This concern covers session-owned terminal rendering helpers for full-screen programs that already use the selected `terminal-session-input` API. It does not revive legacy `StdoutTerminal` handles while a `TerminalSession` owns the coordinator, and it does not permit raw untrusted `string` writes to the session output channel.

The target user is a small terminal editor that needs clear/home, cursor positioning, row drawing, flushing, and optional chime feedback without bypassing `TrustedTerminalOutput` / `SafeTerminalDiagnosticOutput`.

## Comparison matrix

| Axis | High-level session rendering operations | Trusted ANSI builder — recommended v1 | Frame/canvas object |
|---|---:|---:|---:|
| **Ergonomics** | ★★★★★ | ★★★★☆ | ★★★★☆ |
| **Error-model fit** | ★★★★☆ | ★★★★★ | ★★★★☆ |
| **Opalescent-idiom fit** | ★★★★☆ | ★★★★★ | ★★★★☆ |
| **Implementation effort** | Medium (2-3mo) | Low (1-2mo) | High (4-6mo) |
| **Extensibility** | ★★★☆☆ | ★★★★☆ | ★★★★★ |
| **Async readiness** | ★★★★☆ | ★★★★☆ | ★★★★★ |

## Analysis

### High-level session rendering operations
- **Ergonomics**: Best immediate call-site shape for clear, cursor, draw rows, and chime operations.
- **Error-model fit**: Composes with session write/flush errors, but hides some trust construction behind effectful helpers.
- **Implementation effort**: Requires runtime and fake-backend paths for each operation.

### Trusted ANSI builder — recommended v1
- **Ergonomics**: Slightly more explicit, because callers build reviewed output and then call `terminal_session_write_sync`.
- **Error-model fit**: Preserves the selected output trust boundary: raw application text must be declassified before it can be written.
- **Implementation effort**: Smallest safe addition; most helpers are pure constructors for nominal trusted output.
- **Extensibility**: Future high-level helpers can be layered over the builder without breaking existing programs.

### Frame/canvas object
- **Ergonomics**: Good for complete redraws and future diffing.
- **Implementation effort**: Largest v1 surface because it introduces a retained frame type, clipping policy, cursor policy, and commit semantics.
- **Async readiness**: Strong long-term shape for deferred commits and batched rendering, but unnecessary for a first editor.

## Selection

Select **trusted ANSI builder plus minimal session convenience helpers** for v1. The accepted surface should include pure constructors for clear/home, cursor movement, alternate screen, cursor visibility, cursor shape, and bell/chime if audible feedback remains in scope. A later high-level frame object can consume the same trusted output values.

## Required red fixtures

- `terminal-session-render-clear-home`
- `terminal-session-render-cursor-position`
- `terminal-session-render-bell-or-status-chime`
- compile-fail/security: direct raw `string` to session render remains rejected
