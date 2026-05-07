# Observable Handlers

## Overview

This proposal treats guard errors as handled only when they are observed through durable diagnostic helpers and then either returned, converted into a typed wrapper error, or recovered. It deliberately excludes `ignore err` from the core handler set.

Recognized handlers:

- `log_error(err)` followed by a terminating or recovery action
- `return err`
- typed wrapper constructor return with a `source: err` field
- typed fallback after logging

This mirrors common production service patterns in Go, TypeScript, and C#: log a durable diagnostic, then return or recover. It avoids making console printing or silent discard look like complete handling. After adopting typed auto-lifting, this proposal is best treated as a boundary-observability layer rather than the main way to add error context.

## Assumes

- `log_error` is a standard library helper, not a language keyword.
- `log_error(err)` records through a platform-neutral logging surface.
- Typed wrapper variants are normal Opalescent ADT constructors.
- `propagate` may auto-lift a callee error into a uniquely matching wrapper variant declared by the current function's `errors` clause.
- Local fallback is allowed when the function can prove or document that fallback is valid.

## Handler Set

### Log and return

```opal
guard read_text_sync(config_path) into config_text else err =>
    log_error(err)
    return err
```

### Typed context and return

```opal
type ConfigLoadError:
    ReadingConfig:
        source: FilesystemError

guard read_text_sync(config_path) into config_text else err =>
    return new ConfigLoadError.ReadingConfig:
        source: err
```

The context is the typed wrapper variant, not a string-only annotation. Callers can still match `ConfigLoadError.ReadingConfig` and inspect the original `source` error.

### Auto-lift and return

```opal
type ConfigLoadError:
    ReadingConfig:
        source: FilesystemError

let config_text = propagate read_text_sync(config_path)
```

When the current function declares `errors ConfigLoadError`, the compiler can lift `FilesystemError` into `ConfigLoadError.ReadingConfig` if that is the only matching wrapper variant.

### Log and fallback

```opal
guard read_text_sync(optional_notes_path) into notes else err =>
    log_error(err)
    return ''
```

This is useful for optional inputs. The fallback expression should be type-compatible and local, not a broad catch-all.

## Keywords

No new language keywords are required.

| Word | Kind | Purpose |
|------|------|---------|
| `log_error` | stdlib function | Durable error observation. |
| `propagate` | existing keyword | Forward errors, with a unique typed auto-lift when available. |
| `new Error.Context:` | existing constructor syntax | Preserve original error while adding typed context. |

## Syntax Design

The compiler recognizes handler shape rather than arbitrary variable reads. These count:

```opal
log_error(err)
return err
return new PortfolioLoadError.OpeningInput:
    source: err
```

These do not count:

```opal
let message = '{err}'
let _ignored = err
print('{err}')
```

`print('{err}')` can remain legal as ordinary output, but it should not satisfy must-handle semantics because it is not structured, durable, or tool-readable.

## Example Applications

### Config loader with context

```opal
import path_from, read_text_sync, log_error from standard

type ConfigLoadError:
    ReadingConfig:
        source: FilesystemError

let load_config_sync = f(config_path_text: string): string errors ConfigLoadError =>
    let config_path = path_from(config_path_text)

    guard read_text_sync(config_path) into config_text else err =>
        log_error(err)
        return new ConfigLoadError.ReadingConfig:
            source: err

    return config_text
```

### Optional metadata fallback

```opal
import path_from, read_text_sync, log_error from standard

let load_metadata_or_empty_sync = f(metadata_path_text: string): string =>
    let metadata_path = path_from(metadata_path_text)

    guard read_text_sync(metadata_path) into metadata_text else err =>
        log_error(err)
        return ''

    return metadata_text
```

### Windows-safe logging setup

```opal
import path_from, join_path_components, write_text_sync, log_error from standard

let write_report_sync = f(workspace_text: string, report_text: string): void errors WriteFailureError, InvalidPathError, PermissionDeniedError =>
    let workspace = path_from(workspace_text)
    let report_path = join_path_components(workspace, ['reports', 'summary.txt'])

    guard write_text_sync(report_path, report_text) else err =>
        log_error(err)
        return err

    return void
```

No example assumes `/tmp`, shell redirection, or POSIX path separators.

## Strengths

1. Strong high-stakes fit: every handled error is visible in logs or propagated.
2. No new parser keywords.
3. Aligns with Go, TypeScript, and C# service patterns.
4. Keeps durable observation separate from typed propagation.
5. Works well with structured logging proposals.
6. Easy to enforce with a compiler/linter pass over guard handler bodies.

## Weaknesses

1. Requires a logging standard library surface before the rule feels complete.
2. Logging can become noisy if used for expected control flow.
3. Developers need guidance on when fallback is acceptable.
4. Logging is not a type-level context mechanism and should not replace wrapper variants.
5. Slightly more ceremony than direct `return err` or auto-lifted `propagate`.

## Windows Support Requirements

- `log_error` must not assume syslog, ANSI color, or POSIX-only stderr behavior.
- File-backed loggers must use `FilesystemPath` and path helpers.
- Newline normalization should be deterministic when logs are written to files.
- Error wrapper fields must preserve Windows path values without lossy conversion.

## Fit

- **Ergonomics**: Good.
- **Error-model fit**: Good as a boundary-observability rule, weaker as the core context mechanism.
- **High-stakes fit**: Medium-High when paired with typed wrapper returns; Medium if teams log and recover too freely.
- **Implementation effort**: Medium if implemented as recognized stdlib calls plus type-checker validation for wrapper returns.

## Must NOT Have

- No `ignore err` in this proposal's core handler set.
- No treating plain `print(err)` as durable observation.
- No treating logging alone as typed context or complete handling.
- No hidden exception throwing after logging.
- No platform-specific logging requirement in the language core.
