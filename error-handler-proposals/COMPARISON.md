# Error Handler Proposals - Comparison & Recommendations

## Overview

These proposals explore ways to make `guard` error bindings become must-handle values without turning Opalescent into a high-ceremony language. They assume the existing Opalescent error model remains intact, with one recommended extension: `propagate` may auto-lift a lower-level error into a uniquely matching typed wrapper variant declared by the current function's `errors` clause.

1. Fallible functions declare errors with `errors` clauses.
2. Call sites use `guard` for local handling or `propagate` for caller handling.
3. Statement guard shorthand such as `guard delete_file_sync(path) else err =>` remains valid for `void` success cases.
4. Error values are statically typed values, not exceptions.
5. Context is best represented by typed wrapper variants with fields such as `source`, not by string-only error annotations.
6. Windows behavior must stay explicit and portable: no POSIX-only path assumptions, no hidden shell behavior, and no platform-specific logging sink required by the language.

The core design question is: once a guard binds `err`, which operations count as actually handling that value?

---

## Quick Comparison

| Proposal | Handler Set | New Keywords | Ceremony | High-Stakes Safety | Windows Fit | Implementation Effort |
|----------|-------------|--------------|----------|-------------------|-------------|------------------------|
| [Core Local Handlers](core-local-handlers/) | `return err`, `print_error(err)`, `ignore err` | `ignore` | Low | Medium | High | Medium |
| [Observable Handlers](observable-handlers/) | `log_error(err)` followed by `return`, typed wrapper return, or typed fallback | None | Medium | Medium-High | High | Medium |
| [Propagation Only](propagation-only/) | `return err`, typed wrapper constructor return, auto-lifted `propagate` | None | Low-Medium | Very High | High | Medium-High |
| [Typed Match Recovery](typed-match-recovery/) | exhaustive match on `err`, `return err`, typed wrapper return, auto-lifted `propagate` | None | Medium-High | Very High | High | High |
| [Cleanup Probe Ignore](cleanup-probe-ignore/) | `ignore err` only in cleanup/probe contexts, plus `return err` elsewhere | `ignore` | Medium | High | High | Medium-High |

---

## Handler Categories

### Propagating Handlers

These preserve the failure path and send it upward.

```opal
guard read_text_sync(path) into contents else err =>
    return err
```

This is the closest Opalescent equivalent to Go's `return err`, Rust's `?`, and Zig's `try`, but it stays explicit at the guard site.

With typed auto-lifting, `propagate` can also add typed context when the current function declares a unique wrapper variant for the callee error:

```opal
type ConfigLoadError:
    ReadingConfig:
        source: FilesystemError
    ParsingConfig:
        source: ParseError

let load_config_sync = f(config_path: FilesystemPath): Config errors ConfigLoadError =>
    let config_text = propagate read_text_sync(config_path)
    let config = propagate parse_config(config_text)
    return config
```

The compiler can expand those propagations into `ConfigLoadError.ReadingConfig` and `ConfigLoadError.ParsingConfig` respectively when each lift is unambiguous.

### Observing Handlers

These record or display the error before either returning, recovering, or continuing.

```opal
guard read_text_sync(path) into contents else err =>
    log_error(err)
    return err
```

Observation alone is weak as a complete handler. The safer language design is to require observation plus a terminating or recovery action.

### Recovering Handlers

These convert an error into a valid success-path value or fallback action.

```opal
guard read_text_sync(path) into contents else err =>
    log_error(err)
    return ''
```

Recovery is useful, but dangerous if it silently masks failures. Typed match recovery gives the strongest version.

### Discarding Handlers

These intentionally drop the error.

```opal
guard delete_file_sync(temp_path) else cleanup_err =>
    ignore cleanup_err
```

This is the controversial category. If Opalescent wants to be safer than Go, discard should be opt-in, visible, and narrow by language design.

---

## Goal-by-Goal Analysis

### Safer than Go

Best fits: **Propagation Only with typed auto-lifting**, **Typed Match Recovery**, and a narrow **Cleanup Probe Ignore** design.

Go makes it easy to write `_ = cleanup()` or to log and continue without a type-level distinction. Opalescent can do better by making every guard error binding a must-handle value and requiring one of a small set of recognized handling actions. Typed wrapper variants are stronger than string context because callers can still exhaustively match the resulting error.

### Less ceremony than Rust

Best fits: **Propagation Only with typed auto-lifting**, **Core Local Handlers**, and **Observable Handlers** for CLI/service boundaries.

These keep the common path short:

```opal
guard parse_config(text) into config else err =>
    return err
```

No lifetimes, no `Result<T, E>` pattern matching at every call site, and no borrow checker-shaped error model. Auto-lifted `propagate` can make the common path even shorter while preserving typed context at module boundaries.

### High-stakes fit

Best fits: **Typed Match Recovery**, **Propagation Only**, and a narrow **Cleanup Probe Ignore** design.

Backtesters, trading systems, migration tools, and infrastructure code benefit when silent error swallowing is impossible. A safer core language should avoid treating `print_error(err)` as complete handling unless it is followed by `return`, `propagate`, exhaustive `match`, or an explicit narrow discard form.

### Windows support

All proposals can fit Windows if the standard library owns platform behavior:

- `print_error(err)` must not assume ANSI-only terminals or POSIX stderr semantics.
- `log_error(err)` must work with Windows paths through `FilesystemPath`, `path_from`, and `join_path_components`.
- Typed wrapper error fields must preserve path values without lossy slash normalization.
- `ignore err` must not depend on OS behavior; it is purely a compile-time acknowledgement.

---

## Recommendation Tiers

### Tier 1: Best Overall Direction

**Propagation Only with typed auto-lifting** gives the best default balance for the core language. It keeps the common path short, uses the existing `propagate` keyword, and turns context into typed ADT variants instead of display strings.

**Typed Match Recovery** is the strongest pattern for local recovery, domain errors, and standard library module error enums.

### Tier 2: Strong Alternatives

**Cleanup Probe Ignore** is valuable because real software has best-effort cleanup, delete-if-exists, and fallback probes. If adopted, it should be narrow in the language itself.

**Observable Handlers** is useful at process boundaries, CLI surfaces, and service integration points, but it should no longer be the primary recommendation for adding context. Logging is observation, not a substitute for typed propagation.

### Tier 3: Friendly Default for Small Programs

**Core Local Handlers** is the most ergonomic and probably the easiest on-ramp, but allowing `print_error(err)` or general `ignore err` as complete handlers can be too weak for high-stakes defaults.

---

## Handler Set Design Options

### Permissive Core

Recognized handlers could include:

- `return err`
- `propagate fallible_call(...)`, including unique typed auto-lifts
- `print_error(err)` followed by `return`, fallback, or `ignore`
- `log_error(err)` followed by `return`, fallback, or `ignore`
- exhaustive match on `err`
- `ignore err`

### Balanced Core

Recognized handlers could include:

- `return err`
- `propagate fallible_call(...)`, including unique typed auto-lifts
- typed wrapper constructor return with a `source: err` field
- `log_error(err)` followed by `return`, exhaustive `match`, or typed fallback
- exhaustive match on `err`
- `ignore err` only in cleanup/probe contexts

### Minimal Core

Recognized handlers could include:

- `return err`
- auto-lifted `propagate fallible_call(...)` when the lift is unique
- typed wrapper constructor return with a `source: err` field
- exhaustive match on `err` where every branch returns, propagates, or produces a typed fallback

This design has no bare `print_error(err)` and no general `ignore err`.

---

## Directory Structure

```text
error-handler-proposals/
|-- COMPARISON.md
|-- core-local-handlers/
|   `-- proposal.md
|-- observable-handlers/
|   `-- proposal.md
|-- propagation-only/
|   `-- proposal.md
|-- typed-match-recovery/
|   `-- proposal.md
`-- cleanup-probe-ignore/
    `-- proposal.md
```
