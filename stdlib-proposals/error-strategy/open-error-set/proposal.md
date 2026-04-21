# Open Error Set

## Overview
The Open Error Set strategy allows any function to declare its own unique error types directly in its signature using the `errors` keyword. There is no central registry or hierarchy. This approach favors decentralization and local reasoning, enabling libraries to define specific errors without upstream coordination.

It fits the Opalescent vision by being explicit and statically verifiable while maintaining a low barrier to entry for library authors.

## Assumes
This proposal assumes the existing `errors` keyword and `guard`/`propagate` syntax are available in the language. It also assumes that error types can be defined locally or imported from other modules.

## Syntax Design
No new keywords are required. Errors are declared in the function signature:

```opal
let process_data = f(input: string): int32 errors InvalidFormat, DatabaseTimeout =>
    # Implementation
    return 42
```

Each error name refers to a type defined in the current scope or imported. The compiler ensures that all errors returned or propagated by the function are listed in the `errors` clause.

## Example Applications
A parser library can define its own error types:

```opal

let parse_config = f(text: string): Config errors ParseError =>
    # ...
    return config
```

A network client can define its own errors:

```opal

let fetch_data_sync = f(url: string): string errors NetworkError =>
    # ...
    return response
```

## Strengths
- **Decentralization**: Libraries are independent and don't need a central error registry.
- **Precision**: Function signatures explicitly list every possible failure mode.
- **Low Boilerplate**: No need to wrap errors in a global hierarchy or enum unless desired.
- **Type Safety**: The compiler tracks and enforces error exhaustive handling.

## Weaknesses
- **Signature Bloat**: Functions that call many library components may have long `errors` clauses.
- **Fragmentation**: Similar errors (e.g., `Timeout`) might be defined multiple times across different libraries with slightly different structures.
- **Refactoring**: Changing an internal error type propagates changes through all calling signatures unless abstracted.

## Impact on Existing Syntax
This is the status quo for Opalescent and has no breaking impact. It validates the existing `errors`, `guard`, and `propagate` design.

## Interactions with Other Concerns
- **LSP**: Can easily provide completions for missing error types in the `errors` clause based on the function body.
- **Generics**: Functions could potentially be generic over error sets, though that is out of scope for this specific strategy.

## Implementation Difficulty
Low. The core mechanics (tracking error types through the call graph) are already required by the language spec.

## Must NOT Have
- **Exceptions**: No implicit unwinding or catch-all blocks.
- **Implicit Propagation**: Errors must be explicitly handled via `guard` or passed via `propagate`.
- **Placeholder Errors**: No `AnyError` or `Exception` type that bypasses explicit enumeration.
