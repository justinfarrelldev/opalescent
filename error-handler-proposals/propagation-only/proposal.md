# Propagation Only

## Overview

This proposal is the narrowest low-ceremony option: guard error bindings are handled only by sending the error upward, either unchanged or through a typed wrapper variant.

Recognized handlers:

- `return err`
- typed wrapper constructor return with a `source: err` field
- `propagate` for direct fallible calls outside the guard handler, including unique typed auto-lifts

No local `ignore`, no print-only handling, and no log-only handling. If the function wants to recover locally, it should use a different proposal or an explicit typed `match` rule.

This is closest in spirit to Rust's `?`, Zig's `try`, and Go's common `return err` style, but it stays aligned with Opalescent's `guard` syntax.

## Assumes

- The current function declares every propagated error in its `errors` clause.
- `return err` is an error return, not a success-value return.
- Typed wrapper variants are normal Opalescent ADT constructors.
- `propagate` may auto-lift a callee error into a uniquely matching wrapper variant declared by the current function's `errors` clause.
- Ambiguous lifts require an explicit `guard` and wrapper constructor.
- Local recovery is intentionally out of scope.

## Handler Set

### Direct return

```opal
guard parse_config(config_text) into config else err =>
    return err
```

### Typed contextual return

```opal
type ConfigLoadError:
    ParsingConfig:
        source: ParseError

guard parse_config(config_text) into config else err =>
    return new ConfigLoadError.ParsingConfig:
        source: err
```

### Direct propagation outside guard

```opal
let config_text = propagate read_text_sync(config_path)
```

This proposal keeps `propagate` for the common straight-line case and `guard` for cases where the developer wants to name the error or disambiguate the wrapper.

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
| `guard` | Names the error value for explicit handling. |
| `return` | Returns the error value to the caller. |
| `propagate` | Directly forwards a fallible call's error. |
| `errors` | Declares the function's propagated error set. |

## Syntax Design

A guard handler must terminate through an error return:

```opal
guard read_text_sync(input_path) into text else err =>
    return err
```

A handler may add context:

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

    guard parse_trade_count(batch_text) into trade_count else err =>
        return err

    return trade_count
```

### Windows workspace input

```opal
import path_from, join_path_components, read_text_sync from standard

let load_workspace_manifest_sync = f(workspace_text: string): string errors FileNotFoundError, PermissionDeniedError, ReadFailureError, InvalidUtf8Error, InvalidPathError =>
    let workspace = path_from(workspace_text)
    let manifest_path = join_path_components(workspace, ['opalescent.toml'])

    guard read_text_sync(manifest_path) into manifest_text else err =>
        return err

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
