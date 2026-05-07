# Typed Match Recovery

## Overview

This proposal treats a guard error as handled when the handler either returns it, converts it into a typed wrapper variant, or exhaustively matches on its typed variants. It is the strongest local recovery option because every recovery path is tied to a specific error shape.

Recognized handlers:

- `return err`
- typed wrapper constructor return with a `source: err` field
- auto-lifted `propagate` when a source error has one matching wrapper variant
- exhaustive match on `err` where every arm returns, propagates, uses a narrow ignore form, or produces a type-compatible fallback

This mirrors Rust `match`, TypeScript discriminated unions, C# typed `catch`, and Zig error-switch style. It extends Opalescent's existing match-expression direction into a statement-body form that is better suited to early returns from guard handlers.

## Assumes

- Standard library modules increasingly expose module-level error sum types such as `FilesystemError` or `ParseError`.
- `match` on ADT variants remains the normal Opalescent typed branching construct.
- The compiler can check exhaustiveness for error sum types.
- Functions still declare propagated error types through `errors` clauses.
- Typed wrapper variants are normal Opalescent ADT constructors.
- `propagate` may auto-lift a callee error into a uniquely matching wrapper variant declared by the current function's `errors` clause.
- Current Opalescent parses `match` as a brace-delimited expression. This proposal assumes a statement-body match form, or equivalent compiler support, for arms that return from the enclosing function.

## Handler Set

### Return unchanged

```opal
guard read_text_sync(path) into text else err =>
    return err
```

### Match and recover

```opal-proposed
guard read_text_sync(optional_path) into text else err =>
    match err:
        FilesystemError.FileNotFound => return ''
        FilesystemError.PermissionDenied => return err
        FilesystemError.InvalidPath => return err
        FilesystemError.ReadFailure => return err
        FilesystemError.InvalidUtf8 => return err
```

### Match and add context

```opal-proposed
type OrderLoadError:
    EmptyPayload:
        source: ParseError
    InvalidPayload:
        source: ParseError

guard parse_order(order_text) into order else err =>
    match err:
        ParseError.EmptyInput => return new OrderLoadError.EmptyPayload:
            source: err
        ParseError.InvalidField => return new OrderLoadError.InvalidPayload:
            source: err
```

## Keywords

No new language keywords are required if `match` remains part of the language.

| Existing Keyword | Purpose |
|------------------|---------|
| `match` | Typed and exhaustive error recovery. |
| `return` | Error propagation or fallback return. |
| `guard` | Error binding. |
| `propagate` | Straight-line propagation with unique typed auto-lifts. |
| `errors` | Function-level error declaration. |

## Syntax Design

The handler body must consume the guard error through an exhaustive match when it does not directly return the error:

```opal-proposed
guard string_to_int32(raw_value) into parsed else err =>
    match err:
        ParseError.InvalidDigit => return 0
        ParseError.Overflow => return err
```

The compiler should reject partial handling:

```opal-proposed
# Not accepted if ParseError also has Overflow
guard string_to_int32(raw_value) into parsed else err =>
    match err:
        ParseError.InvalidDigit => return 0
```

## Example Applications

### Optional file recovery

```opal-proposed
import path_from, read_text_sync from standard

type FilesystemError:
    FileNotFound
    PermissionDenied
    InvalidPath
    ReadFailure
    InvalidUtf8

let read_optional_notes_sync = f(notes_path_text: string): string errors FilesystemError =>
    let notes_path = path_from(notes_path_text)

    guard read_text_sync(notes_path) into notes else err =>
        match err:
            FilesystemError.FileNotFound => return ''
            FilesystemError.PermissionDenied => return err
            FilesystemError.InvalidPath => return err
            FilesystemError.ReadFailure => return err
            FilesystemError.InvalidUtf8 => return err

    return notes
```

### Backtester parse rules

```opal-proposed
type PriceParseError:
    EmptyInput
    InvalidTimestamp
    InvalidPrice
    Overflow

let parse_price_or_zero = f(row_text: string): int64 errors PriceParseError =>
    guard parse_price(row_text) into price else err =>
        match err:
            PriceParseError.EmptyInput => return 0
            PriceParseError.InvalidTimestamp => return err
            PriceParseError.InvalidPrice => return err
            PriceParseError.Overflow => return err

    return price
```

### Windows path rules

```opal-proposed
import path_from, join_path_components, read_text_sync from standard

let load_user_settings_sync = f(user_root_text: string): string errors FilesystemError =>
    let user_root = path_from(user_root_text)
    let settings_path = join_path_components(user_root, ['AppData', 'Local', 'Opalescent', 'settings.toml'])

    guard read_text_sync(settings_path) into settings_text else err =>
        match err:
            FilesystemError.FileNotFound => return ''
            FilesystemError.PermissionDenied => return err
            FilesystemError.InvalidPath => return err
            FilesystemError.ReadFailure => return err
            FilesystemError.InvalidUtf8 => return err

    return settings_text
```

## Strengths

1. Strongest local recovery story once statement-body match support exists.
2. Excellent fit for module-scoped error enums.
3. Makes partial handling visible at compile time.
4. Familiar to Rust, TypeScript, C#, and Zig users.
5. Allows safe fallback without treating all errors alike.
6. Works well with typed wrapper errors and high-stakes domain rules.

## Weaknesses

1. More verbose than direct propagation for local recovery paths.
2. Requires good sum-type ergonomics for standard library errors.
3. Adding new error variants can break exhaustive handlers.
4. Less pleasant for one-off scripts.
5. Requires stronger type checker support than simple handler recognition, especially for wrapper-lift analysis.
6. Requires parser/lowering work if Opalescent wants `match err:` arms that can `return` from the enclosing guard handler.

## Windows Support Requirements

- Filesystem error enums must represent Windows-specific failures without collapsing them into opaque strings.
- Exhaustive matching must remain stable across platforms; platform-specific payload fields can vary, but the variant set should be portable when possible.
- Path examples should use `path_from` and `join_path_components` rather than hard-coded separators.

## Fit

- **Ergonomics**: Medium.
- **Error-model fit**: Excellent.
- **High-stakes fit**: Excellent.
- **Implementation effort**: High because it depends on typed error variants, exhaustiveness checking, and a statement-body match form or equivalent lowering.

## Must NOT Have

- No catch-all `_` arm required by default.
- No collapsing typed errors into strings before matching.
- No string-only context helper as a substitute for wrapper variants.
- No exception-like catch blocks.
- No platform-only variants leaking into portable code without declaration.
