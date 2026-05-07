# Core Local Handlers

## Overview

This proposal defines a small, friendly handler set for `guard` error bindings:

- `return err`
- `print_error(err)`
- `ignore err`

It is intentionally close to common beginner-friendly patterns from Go, Zig, TypeScript, and C# while remaining explicit enough for Opalescent's safer-than-Go goal. The main idea is that an error binding is not considered handled merely because it was assigned to another variable or interpolated into an unrelated string. It must flow into one of the recognized handler forms.

## Assumes

- `guard` remains the local error-handling construct.
- `propagate` remains the explicit propagation construct for direct fallible calls.
- Error bindings are typed values, even if early standard library implementations render them as strings.
- `print_error` is a standard library helper designed for diagnostics, not a new language statement.
- `ignore` is a new language statement for visible, intentional discard.

## Handler Set

### `return err`

Propagates the current guard error from the current function.

```opal
guard read_text_sync(config_path) into config_text else err =>
    return err
```

This matches the spirit of Go's `if err != nil { return err }`, but the compiler knows `err` is a guard error binding and can require that the function declares a compatible `errors` clause.

### `print_error(err)`

Prints a user-facing diagnostic.

```opal
guard string_to_int32(input_text) into parsed else parse_err =>
    print_error(parse_err)
    return 0
```

`print_error(err)` plus a fallback can count as recovery in this proposal. `print_error(err)` by itself should not be enough because printing is not durable observability.

### `ignore err`

Intentionally discards an error.

```opal
guard delete_file_sync(temp_path) else cleanup_err =>
    ignore cleanup_err
```

This avoids the uglier pattern currently seen in cleanup-style code:

```opal
let _ignored_cleanup_err = cleanup_err
```

The statement is short, visible, and easy for code review tools to search.

## Keywords

| Keyword | Required? | Purpose |
|---------|-----------|---------|
| `ignore` | Yes | Marks an error value as intentionally discarded. |

No `because` keyword is included. Rationale can stay in normal comments where teams want it, but the language should not force a trailing string or comment-shaped reason into every cleanup path.

## Syntax Design

The `ignore` statement is expression-free and only accepts a bound name:

```opal
ignore cleanup_err
```

It should not accept arbitrary expressions:

```opal
# Not allowed by this proposal
ignore delete_file_sync(path)
```

The fallible operation must still be explicit through `guard`, so the compiler can know exactly which error binding is being discarded.

## Example Applications

### CLI input parsing

```opal
import string_to_int32, print_error from standard

entry main = f(args: string[]): void =>
    guard string_to_int32(args[0]) into count else parse_err =>
        print_error(parse_err)
        return void

    print('count={count}')
    return void
```

### Best-effort cleanup

```opal
import path_from, delete_file_sync, write_text_sync from standard

let rewrite_cache_sync = f(cache_path_text: string, payload: string): void errors WriteFailureError, InvalidPathError =>
    let cache_path = path_from(cache_path_text)
    let temp_path = path_from('{cache_path_text}.tmp')

    guard delete_file_sync(temp_path) else cleanup_err =>
        ignore cleanup_err

    guard write_text_sync(temp_path, payload) else write_err =>
        return write_err

    return void
```

### Windows path cleanup

```opal
import path_from, join_path_components, delete_file_sync from standard

let cleanup_windows_temp_sync = f(root_text: string): void =>
    let root = path_from(root_text)
    let temp_file = join_path_components(root, ['opalescent.tmp'])

    guard delete_file_sync(temp_file) else cleanup_err =>
        ignore cleanup_err

    return void
```

This keeps Windows support explicit by using path helpers instead of slash concatenation.

## Strengths

1. Very small mental model.
2. Easy migration from current `print(err)` and `_ignored_err` patterns.
3. `ignore err` is searchable and auditable.
4. No exception-like control flow.
5. Good fit for scripts, CLIs, tests, and examples.
6. Keeps statement guard shorthand pleasant for `void`-returning cleanup calls.

## Weaknesses

1. `ignore err` can become the new `_ = err` if used casually.
2. `print_error(err)` is not durable enough for production observability.
3. The proposal does not force typed recovery through `match`.
4. A permissive core handler set could still hide operational failures.
5. The compiler needs to distinguish true handling from ordinary reads.

## Windows Support Requirements

- `print_error` must render stable diagnostics on Windows terminals, including non-ANSI consoles.
- Error rendering must not depend on POSIX file descriptors.
- Path-bearing errors must preserve Windows paths and drive prefixes.
- `ignore` has no runtime behavior and therefore has no platform-specific path.

## Fit

- **Ergonomics**: Excellent.
- **Error-model fit**: Good.
- **High-stakes fit**: Medium unless the core language keeps `ignore` and bare printing narrow.
- **Implementation effort**: Medium because `ignore` requires parser, AST, type checker, formatter, and diagnostics work.

## Must NOT Have

- No `ignore err because 'text'` syntax.
- No implicit ignore for underscore-prefixed error names.
- No treating `let _x = err` as handling.
- No exceptions or panic-style hidden propagation.
