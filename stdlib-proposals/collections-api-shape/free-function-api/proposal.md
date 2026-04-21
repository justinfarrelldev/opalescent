# Free Function API

## Overview
This proposal defines a collection API based on standalone free functions. Functions like `map_over_array` and `filter_array` are defined globally or imported from the standard library. This approach follows the existing patterns seen in `language-spec/array_helpers.op`, favoring simplicity and explicit data flow over object-oriented or fluent styles.

## Assumes
This proposal assumes the `Absence via Errors` alternative from the `optional-representation` concern for handling cases where a collection search might return no result.

## Syntax Design
Functions are named verbosely to avoid collisions and clearly indicate the collection type they operate on.

```opal
import map_over_array, filter_array from standard

let numbers: int32[] = [1, 2, 3, 4]
let evens = filter_array<int32>(numbers, f(n: int32): boolean => (n % 2) is 0)
let squares = map_over_array<int32, int32>(evens, f(n: int32): int32 => n * n)
```

## Example Applications
The `free-function-api` excels in simple data transformations where the structure of the pipeline is flat.

```opal
let process_data = f(input: int32[]): int32[] =>
    let filtered = filter_array<int32>(input, f(n: int32): boolean => n > 10)
    return map_over_array<int32, int32>(filtered, f(n: int32): int32 => n * 2)
```

## Strengths
- **Simplicity**: No new grammar or dispatch mechanisms required.
- **Explicitness**: Every operation clearly states the type of collection it handles.
- **Consistency**: Matches the established style of existing language helpers.

## Weaknesses
- **Boilerplate**: Nested calls can become hard to read (the "LISP problem").
- **Verbosity**: Long function names like `find_first_matching_in_array` can be cumbersome.
- **Discoverability**: LSP completions are less focused than method-based discovery.

## Impact on Existing Syntax
This proposal has no impact on existing syntax as it uses standard function call semantics. It codifies the existing informal patterns in the language spec.

## Interactions with Other Concerns
Pairs naturally with the standard error handling model. Generic functions in Opalescent already support the required patterns.

## Implementation Difficulty
Very Low. Requires only the implementation of the standard library functions in pure Opalescent or as compiler built-ins.

## Must NOT Have
- **Method Syntax**: Should not allow `array.map()`.
- **Short Names**: Should avoid ambiguous names like `map` or `filter`.
- **Implicit Conversions**: No automatic coercion between collection types.
