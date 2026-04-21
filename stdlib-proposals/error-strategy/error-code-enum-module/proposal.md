# Error Code Enum Module

## Overview
The Error Code Enum Module strategy encourages each library or standard library module to export a single, comprehensive enum (or sum type) listing its errors. Callers interact with this enum using the standard `guard` pattern and can match on specific cases if needed.

This strategy balances centralization (within a module) and decentralization (across the entire ecosystem), making it easy to see all errors a specific module can produce.

## Assumes
This proposal assumes that the type system supports sum types or enums and that these can be used as error types in the `errors` clause.

## Syntax Design
Modules export their error enum:

```opal
type HttpError:
    BadRequest:
        status: int32
    Unauthorized
    InternalServerError

let request_sync = f(url: string): string errors HttpError =>
    # ...
    return response
```

Callers handle the module-specific enum:

```opal
guard request_sync(url) into data else err =>
    # err is of type HttpError
    return ""
```

## Example Applications
A JSON parser module with an error enum:

```opal
type JsonError:
    InvalidSyntax:
        line: int32
    UnexpectedEnd
    DepthExceeded

let parse_json = f(json: string): Value errors JsonError =>
    # ...
    return value
```

A standard library math module with an error enum:

```opal
type MathError:
    DivisionByZero
    DomainError
    Overflow

let divide_integers = f(a: int32, b: int32): int32 errors MathError =>
    # ...
    return result
```

## Strengths
- **Encapsulation**: Errors are scoped to the module that produces them.
- **Enumeration**: Callers can see all possible failure modes for a module in one place.
- **Interoperability**: Easy to match on specific variants within a module.
- **Predictability**: Each module follows the same pattern of exporting an error enum.

## Weaknesses
- **Coarseness**: Functions that only produce one error still use the entire enum type.
- **Composition**: Functions that call multiple modules must list multiple enums or wrap them.
- **Internal Changes**: Adding a new variant to a module's error enum is a breaking change for exhaustive matchers.

## Impact on Existing Syntax
This strategy is fully compatible with existing Opalescent syntax. It provides a consistent pattern for library development.

## Interactions with Other Concerns
- **Match Expression**: Works perfectly with future match or switch expressions for handling specific variants.
- **Serialization**: Enums are easy to serialize and transmit across module boundaries or networks.

## Implementation Difficulty
Low. This is a pattern-based approach leveraging the existing type system.

## Must NOT Have
- **Exceptions**: No implicit handling; still uses `guard` and `propagate`.
- **Dynamic Types**: Errors are statically typed via the module-exported enum.
- **Global Registry**: Errors are scoped to the module, not a global registry.
