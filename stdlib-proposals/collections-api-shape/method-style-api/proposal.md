# Method Style API

## Overview
This proposal introduces method-like syntax for collections, allowing operations like `map_over_array` and `filter_array` to be called directly on instances using the dot operator. This aligns with many modern languages and enhances discoverability and chainability while maintaining Opalescent's strict typing and error model.

## Assumes
This proposal assumes the `Absence via Errors` alternative from the `optional-representation` concern for handling cases where a collection search might return no result.

## Syntax Design
Operations are implemented as methods on the collection types. The syntax is `collection.method(args)`.

```opal
let numbers: int32[] = [1, 2, 3, 4]
let evens = numbers.filter_array(f(n: int32): boolean => (n % 2) is 0)
let squares = evens.map_over_array(f(n: int32): int32 => n * n)
```

## Example Applications
The `method-style-api` excels in fluent chaining of operations.

```opal
let process_data = f(input: int32[]): int32[] =>
    return input.filter_array(f(n: int32): boolean => n > 10)
                .map_over_array(f(n: int32): int32 => n * 2)
```

## Strengths
- **Ergonomics**: Natural "fluent" style that flows left-to-right.
- **Discoverability**: LSP can easily suggest methods for a given type.
- **Conciseness**: Reduces the need for deeply nested function calls.

## Weaknesses
- **Complexity**: Requires support for type-attached functions in the grammar and compiler.
- **Namespace Issues**: Methods could potentially collide with field names in some record types.
- **Opacity**: Can obscure the fact that new collections are being allocated.

## Impact on Existing Syntax
Method call syntax (`.method_name()`) is not yet fully defined as a first-class feature for all types in the current language spec. Supporting this would require updates to the parser and type checker to handle method dispatch on arrays and other collection types.

## Interactions with Other Concerns
Pairs naturally with the standard error handling model, although chaining multiple fallible methods requires careful error propagation using `propagate` at each step.

## Implementation Difficulty
Medium. Requires a new dispatch mechanism in the compiler to resolve method names to implementation functions based on the receiver's type.

## Must NOT Have
- **Mutability by Default**: Methods should remain pure and return new collections.
- **Implicit Dispatch**: No dynamic dispatch or inheritance-based polymorphism.
- **Inconsistent Naming**: Methods should still use verbose names like `map_over_array` to be explicit.
