# Maybe Tagged Union

## Overview
This proposal introduces a dedicated `Maybe T` tagged union to represent optional values. By standardizing on a single wrapper type with `Some` and `None` variants, Opalescent provides a clear, type-safe, and idiomatic way to express the possible absence of a value.

Unlike error-based absence, a `Maybe` type suggests that the missing case is an expected outcome of the logic rather than a failure condition. This is particularly useful for configuration, optional fields, and collection lookups where a value not being present is a common occurrence.

## Assumes
- Support for generic tagged unions as seen in the language specification.
- A compiler that can efficiently optimize small tagged unions (Perceus+SCR) to minimize allocation overhead.
- Pattern matching or `if ... is` syntax to destructure tagged unions.

## Syntax Design
A `Maybe T` type is defined in a standard library module.

```opal
##
Description: A generic tagged union for optional values.
##
```

Usage involves constructing `Some` or `None`:

```opal
let find_item = f(id: string): Maybe string =>
    if id is 'found':
        return new Some:
            value: 'Item'
    return new None
```

## Example Applications
Representing optional fields in a record:

```opal

let print_bio = f(user: User): void =>
    if user.bio is Some:
        print(user.bio.value)
    return void
```

## Strengths
- **Type-Level Clarity**: The return type `Maybe T` explicitly signals that a value may be absent, making it impossible to ignore at compile time.
- **Compositionality**: Allows for building generic combinators like `map`, `filter`, and `flatten` for optional values.
- **Semantic Separation**: Clearly distinguishes between "not found" (a valid state) and "error" (a failure state).
- **Reduced Verbosity**: Avoids listing specific `NotFound` errors in the `errors` clause for common operations.

## Weaknesses
- **Memory Overhead**: Each `Maybe` instance may require additional memory for the tag and potentially an allocation, though the compiler can optimize this.
- **Boilerplate**: Destructuring `Maybe` types can be more verbose than using `guard` for errors if the language lacks concise pattern-matching shortcuts.
- **Learning Curve**: Requires developers to understand and work with wrapper types.

## Impact on Existing Syntax
Minimal. This proposal utilizes existing type definition and construction syntax. It recommends adding `Maybe T` to the `standard` library.

## Interactions with Other Concerns
- **Generics**: Fully leverages Opalescent's generic type system.
- **Concurrency**: Immutable `Maybe` values are safe to share across threads.
- **FFI**: May require specialized handling when passing to or from languages with different nullability models.

## Implementation Difficulty
Low. This is a straightforward application of the existing type system. The primary work is in defining the type and potentially adding library functions for convenience.

## Must NOT Have
- **Null Reference**: Must not introduce a raw `null` pointer. Absence must always be wrapped in the `None` variant.
- **Implicit Coercion**: Must not allow implicit conversion between `T` and `Maybe T`. Construction must be explicit via `Some` and `None`.
