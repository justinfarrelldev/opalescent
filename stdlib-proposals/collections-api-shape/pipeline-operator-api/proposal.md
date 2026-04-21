# Pipeline Operator API

## Overview
This proposal introduces a pipeline operator `|>` to facilitate collection transformations. The operator pipes the result of the left-hand expression as the first argument to the right-hand function call. This provides the readability of method chaining without requiring types to own their operations, preserving the flexibility of free functions.

## Assumes
This proposal assumes the `Absence via Errors` alternative from the `optional-representation` concern for handling cases where a collection search might return no result.

## Syntax Design
The pipeline operator `|>` is used between a value and a function call. If `|>` is not supported in the grammar, a fallback `pipe_into(collection, [op1, op2])` helper is proposed.

```opal
let numbers: int32[] = [1, 2, 3, 4]
let result = numbers 
    |> filter_array(f(n: int32): boolean => (n % 2) is 0)
    |> map_over_array(f(n: int32): int32 => n * n)
```

## Example Applications
Pipelines make complex data flows easy to follow.

```opal
let process_data = f(input: int32[]): int32[] =>
    return input 
        |> filter_array(f(n: int32): boolean => n > 10)
        |> map_over_array(f(n: int32): int32 => n * 2)
```

## Strengths
- **Readability**: Clear left-to-right data flow.
- **Decoupling**: Operations don't need to be defined on the type itself.
- **Flexibility**: Works with any function that takes the data as its first argument.

## Weaknesses
- **Grammar Change**: Requires introducing a new operator to the language.
- **Tooling**: Requires LSP support for correctly suggesting functions that fit the pipeline.
- **Error Handling**: Chaining fallible operations still requires explicit handling at each step or a specialized pipeline error handler.

## Impact on Existing Syntax
The `|>` operator is not currently in the Opalescent grammar. Adding it would be a minor addition to the expression syntax. If rejected, the `pipe_into` helper provides a more traditional but less ergonomic alternative.

## Interactions with Other Concerns
Interacts with the function call syntax and potentially with any future partial application features. Error handling remains explicit via `guard` or `propagate`.

## Implementation Difficulty
Low to Medium. Adding a new operator to the parser is straightforward; ensuring good type inference through the pipeline is the primary challenge.

## Must NOT Have
- **Implicit Argument Binding**: Should not magically bind to arguments other than the first.
- **Async Magic**: Should not automatically handle deferred/wait_for in the pipeline.
- **Mutation**: Should not encourage in-place mutation of the piped value.
