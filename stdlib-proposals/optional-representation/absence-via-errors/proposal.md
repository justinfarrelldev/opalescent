# Absence via Errors

## Overview
This proposal treats the absence of a value as a first-class error condition rather than a state of a data type. Instead of returning a wrapper type like `Maybe` or `Option`, functions that might fail to find or produce a value include an explicit error variant in their `errors` clause.

This approach leverages Opalescent's robust error-handling machinery, specifically the `guard` statement, to force callers to acknowledge and handle the "not found" case. It avoids the allocation and nesting overhead of wrapper types while maintaining strict type safety.

## Assumes
- Opalescent's existing error handling model using `errors` clauses and `guard` statements.
- The ability to define custom error types (tagged unions or records) as seen in the language specification.
- A compiler that enforces exhaustive handling of all declared error variants when using `guard`.

## Syntax Design
No new syntax is required. This proposal utilizes existing function signatures and error handling constructs.

```opal

let find_value = f(id: string): string errors NotFound =>
    if id is 'valid':
        return 'Found it'
    return new NotFound:
        Key:
            key: id
```

Callers handle the absence using `guard`:

```opal
guard find_value('test') into result else err =>
    return 'Default Value'
```

## Example Applications
A common use case is looking up an item in a collection.

```opal

let get_config = f(key: string): string errors LookupError =>
    # Implementation logic
    return 'value'

let start_app = f(): void =>
    guard get_config('port') into port else _ =>
        let default_port = '8080'
        return void
    return void
```

## Strengths
- **Zero Overhead**: No wrapper types are allocated on the heap; absence is signaled through the existing error channel.
- **Explicit Handling**: The `guard` syntax makes it impossible to forget to handle the missing case, as the compiler requires the `else` block.
- **Rich Context**: Errors can carry metadata (e.g., the key that was missing), providing better debugging information than a simple `None`.
- **Consistency**: Uses the same mental model for "missing value" as for "permission denied" or "disk full".

## Weaknesses
- **Verbosity**: Function signatures become longer as they must list `NotFound` in their `errors` clause.
- **Composition**: It is slightly more difficult to chain multiple optional operations compared to a monadic `map` on a `Maybe` type, though `propagate` helps.
- **Intent**: Some developers may find it semantically confusing to treat "not found" as an "error" rather than a valid state.

## Impact on Existing Syntax
None. This proposal uses existing language features and patterns. It simply establishes a convention for representing optionality.

## Interactions with Other Concerns
- **Error Handling**: Directly relies on and strengthens the error handling model.
- **LSP**: Language servers can provide excellent completion and refactoring for missing error cases.
- **Generics**: Works naturally with generic functions, as `NotFound` can be one of the error types.

## Implementation Difficulty
Very Low. This is a pattern-based proposal that requires no changes to the current compiler or runtime beyond what is already planned for error handling.

## Must NOT Have
- **Hidden Nulls**: Must never allow a value to be "secretly" missing without being declared in the `errors` clause.
- **Generic Errors**: Avoid using a single, global `Error` type; prefer specific, descriptive error variants for different types of absence.
