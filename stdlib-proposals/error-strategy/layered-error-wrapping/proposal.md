# Layered Error Wrapping

## Overview
The Layered Error Wrapping strategy allows standard library functions to add contextual information to errors as they propagate. This is achieved through a `wrap_error_context(original, message)` helper that creates a new error containing both the original error and additional information.

This approach improves observability and debugging by allowing high-level functions to explain why a lower-level failure occurred in their specific context.

## Assumes
This proposal assumes that the type system can represent a wrapped error structure and that the `errors` keyword can handle these wrapped types.

## Syntax Design
A helper function `wrap_error_context` is provided:

```opal
type WrappedError:
    Context:
        original: AnyError
        message: string

let perform_operation_sync = f(): void errors MyError, WrappedError =>
    guard lower_level_sync() into val else err =>
        return wrap_error_context(err, "Failed during operation")
    return void
```

`AnyError` represents any type that can be used in an `errors` clause. The compiler ensures that all errors, including wrapped ones, are correctly declared in the signature.

## Example Applications
Adding context to a file read error:

```opal
let load_profile_sync = f(user_id: string): Profile errors IoError, WrappedError =>
    guard read_file_sync("/profiles/" + user_id) into data else err =>
        return wrap_error_context(err, "Failed to load profile for " + user_id)
    # ...
    return profile
```

Adding context to a database query error:

```opal
let update_user_sync = f(user: User): void errors DbError, WrappedError =>
    guard db_execute_sync("UPDATE users ...") into res else err =>
        return wrap_error_context(err, "Failed to update user record")
    return void
```

## Strengths
- **Observability**: Error logs can contain a rich history of where and why failures happened.
- **Debugging**: Developers get more context about the state of the application when an error occurs.
- **Flexibility**: Any error can be wrapped at any level without changing its original structure.
- **Composable**: Multiple layers of context can be added as the error propagates.

## Weaknesses
- **Complexity**: Inspecting the original error (unwrapping) requires extra code.
- **Performance**: Wrapping errors adds small overhead for memory allocation and copying.
- **Type Information**: If not careful, the specific type of the original error might be obscured behind a generic `WrappedError`.

## Impact on Existing Syntax
This strategy is compatible with existing syntax but introduces a new `WrappedError` type and a common pattern for error enrichment.

## Interactions with Other Concerns
- **Logging**: Highly beneficial for logging systems that can recursively unwrap and format these errors.
- **Concurrency**: Context is especially valuable for tracking errors across different threads or asynchronous operations.

## Implementation Difficulty
Medium. Requires the compiler to support a generic way to represent "any error" or for the standard library to define a shared base error type.

## Must NOT Have
- **Exceptions**: No implicit propagation; still uses `guard` and `propagate`.
- **Global Catch-all**: Errors must still be explicitly listed in signatures.
- **Implicit Wrapping**: Every wrap operation must be explicit in the code.
