# Propagation Only

## Overview

This proposal is the narrowest low-ceremony option: guard error bindings are handled only by sending the error upward, either unchanged or through a typed wrapper variant.

Recognized handlers:

- shorthand `propagate <call>()` outside any guard, for the common straight-line propagation case (including unique typed auto-lifts)
- typed wrapper constructor return with a `source: err` field, used inside a guard error clause that adds typed context
- terminal `propagate err` as the final top-level statement of an active guard error clause that performs handling first (logging, metrics, cleanup, etc.)

No `return err` in guard error clauses, no local `ignore`, no print-only handling, and no log-only handling. If the function wants to recover locally, it should use a different proposal or an explicit typed `match` rule.

This is closest in spirit to Rust's `?`, Zig's `try`, and Go's common `return err` style, but it stays aligned with Opalescent's `guard` syntax.

### Implemented caveat (guard error clauses)

The compiler enforces these rules for guard error clauses, which this proposal honors:

1. `propagate err` is only valid as the **final top-level statement** of an active guard error clause. Using `propagate err` anywhere else (including outside a guard error clause) is rejected.
2. A long-form guard error clause whose body contains **only** `propagate err` is rejected. Such a clause performs no handling, so the compiler asks the author to use the shorthand `propagate <call>()` form instead.
3. When no per-call handling is needed, use the shorthand `propagate <call>()` directly (no `guard`, no `else` clause).
4. Direct `return err` in a guard error clause is **invalid and not supported**. The compiler emits: "return err is not valid in a guard error clause; use propagate err to forward the guard error". To forward an unwrapped error, perform any handling work and then end the clause with `propagate err`. To return a typed wrapper, construct it explicitly via `return new <Wrapper>: source: err`.

## Assumes

- The current function declares every propagated error in its `errors` clause.
- Typed wrapper variants are normal Opalescent ADT constructors.
- `propagate` may auto-lift a callee error into a uniquely matching wrapper variant declared by the current function's `errors` clause.
- Ambiguous lifts require an explicit `guard` and wrapper constructor.
- Local recovery is intentionally out of scope.
- Direct `return err` in a guard error clause is rejected by the compiler; use shorthand `propagate <call>()` (no handling) or terminal `propagate err` after handling.

## Handler Set

### Direct propagation (no per-call handling)

```opal
let config_text = propagate read_text_sync(config_path)
```

When no logging, metrics, or cleanup is needed at the call site, use the shorthand `propagate <call>()` form. There is no `guard` and no `else` clause; this is the canonical "send the error up unchanged" form.

### Terminal `propagate err` after side effects

```opal
guard parse_config(config_text) into config else err =>
    log_parse_failure(err)
    propagate err
```

A guard error clause may run side effects (logging, metrics, cleanup) before forwarding the original error. The clause must end with `propagate err` as its final top-level statement. A guard clause whose body is only `propagate err` is rejected; replace it with the shorthand `propagate <call>()` form above.

### Typed contextual return

```opal
type ConfigLoadError:
    ParsingConfig:
        source: ParseError

guard parse_config(config_text) into config else err =>
    return new ConfigLoadError.ParsingConfig:
        source: err
```

This is the only form that uses `return` inside a guard error clause: it returns a **typed wrapper value**, not the bare `err` binding. Direct `return err` is rejected.

### Direct propagation outside guard

```opal
let config_text = propagate read_text_sync(config_path)
```

This proposal keeps the shorthand `propagate <call>()` form for the common straight-line case, and reserves `guard` for cases where the developer wants to run side effects before forwarding (`propagate err`) or to wrap the error in a typed variant (`return new Wrapper: source: err`).

### Auto-lifted propagation

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

If `read_text_sync` returns `FilesystemError` and `parse_config` returns `ParseError`, the compiler can lift each error into the uniquely matching `ConfigLoadError` variant. If two variants wrap the same source error type, the compiler must reject the bare `propagate` and require an explicit `guard`.

## Keywords

No new language keywords are required.

| Existing Keyword | Purpose |
|------------------|---------|
| `guard` | Names the error value for explicit handling, when side effects or typed wrapping are needed. |
| `return` | Returns a typed wrapper value built from the error binding (e.g. `return new Wrapper: source: err`). Bare `return err` in a guard error clause is rejected. |
| `propagate` | Shorthand `propagate <call>()` for direct forwarding, and terminal `propagate err` as the final top-level statement of a guard error clause. |
| `errors` | Declares the function's propagated error set. |

## Syntax Design

When no per-call handling is needed, use the shorthand `propagate <call>()` form:

```opal
let text = propagate read_text_sync(input_path)
```

A long-form guard handler must terminate through one of:

- terminal `propagate err` as the **final top-level statement**, after running handling work, or
- a typed wrapper `return new Wrapper: source: err`.

Direct `return err` in a guard error clause is rejected by the compiler.

```opal
guard read_text_sync(input_path) into text else err =>
    record_filesystem_failure(err)
    propagate err
```

A handler may add typed context:

```opal
type InputLoadError:
    ReadingInput:
        source: FilesystemError

guard read_text_sync(input_path) into text else err =>
    return new InputLoadError.ReadingInput:
        source: err
```

A handler may not recover locally:

```opal
# Not accepted by this proposal
guard read_text_sync(input_path) into text else err =>
    print_error(err)
    return ''
```

A handler containing only `propagate err` is rejected by the compiler:

```opal
# Rejected: replace with shorthand propagate read_text_sync(input_path)
guard read_text_sync(input_path) into text else err =>
    propagate err
```

Direct `return err` in a guard error clause is rejected by the compiler:

```opal
# Rejected: 'return err is not valid in a guard error clause;
#           use propagate err to forward the guard error'
guard read_text_sync(input_path) into text else err =>
    return err
```

## Example Applications

### Backtester input loading

```opal
import path_from, read_text_sync from standard

type BacktestInputError:
    ReadingPrices:
        source: FilesystemError

let load_prices_sync = f(input_path_text: string): string errors BacktestInputError =>
    let input_path = path_from(input_path_text)

    let price_text = propagate read_text_sync(input_path)

    return price_text
```

### Transaction step with no local recovery

```opal
let process_trade_batch_sync = f(batch_path_text: string): int32 errors FileNotFoundError, PermissionDeniedError, ReadFailureError, InvalidUtf8Error, ParseError =>
    let batch_text = propagate read_text_sync(path_from(batch_path_text))

    let trade_count = propagate parse_trade_count(batch_text)

    return trade_count
```

When no per-call handling is needed, the shorthand `propagate <call>()` form is preferred. A long-form guard clause is only used when the handler must run side effects (logging, metrics) before `propagate err`, or must wrap the error in a typed variant.

### Windows workspace input

```opal
import path_from, join_path_components, read_text_sync from standard

let load_workspace_manifest_sync = f(workspace_text: string): string errors FileNotFoundError, PermissionDeniedError, ReadFailureError, InvalidUtf8Error, InvalidPathError =>
    let workspace = path_from(workspace_text)
    let manifest_path = join_path_components(workspace, ['opalescent.toml'])

    let manifest_text = propagate read_text_sync(manifest_path)

    return manifest_text
```

## Strengths

1. Extremely easy to audit.
2. Very strong fit for safety-critical and financial code.
3. No new syntax or keywords.
4. Keeps failure semantics explicit and statically declared while avoiding string-only context.
5. Avoids log-and-continue bugs.
6. Maps cleanly to common Rust/Zig/Go propagation idioms.

## Weaknesses

1. Too narrow for CLI tools, cleanup, optional files, and retry/fallback workflows.
2. Can push error-list verbosity up the call stack.
3. Does not support graceful degradation without switching proposals.
4. Requires ADT wrapper design at module boundaries.
5. Ambiguous wrapper variants force explicit `guard` handling, which is correct but less terse.

## Windows Support Requirements

- No platform-specific behavior in the language core.
- Typed wrapper fields must preserve Windows path values exactly as values, not as normalized display-only strings.
- Propagated filesystem errors must retain enough platform detail for callers to distinguish permission, missing file, invalid path, and encoding failures.

## Fit

- **Ergonomics**: Good when auto-lifts are unambiguous; Medium when explicit wrappers are needed.
- **Error-model fit**: Excellent.
- **High-stakes fit**: Excellent.
- **Implementation effort**: Medium-High, mostly type-checker enforcement around `return err`, compatible error sets, and unique wrapper-lift resolution.

## Must NOT Have

- No `ignore err`.
- No print-only or log-only handling.
- No ambiguous implicit wrapping.
- No string-only context as a substitute for typed wrapper variants.
- No catch-all `AnyError` escape hatch unless explicitly declared by a separate error strategy.
- No bare `return err` inside a guard error clause; use terminal `propagate err` after handling, or `return new Wrapper: source: err` for typed wrapping.
- No long-form guard clause whose body is only `propagate err`; use the shorthand `propagate <call>()` form instead.
