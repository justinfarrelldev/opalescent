# JSON-Only Value Tree

## Overview
This proposal introduces a simple, tree-based JSON serialization API. It centers around a `JsonValue` tagged union that represents the entire JSON structure in memory. This approach is intuitive for small to medium-sized documents where the entire tree fits comfortably in RAM.

The core functions are `parse_json_string` and `serialize_json_value_to_string`. Users interact with JSON by traversing or constructing the `JsonValue` tree manually.

## Assumes
- Standard string and array handling.
- Tagged union support in the compiler for `JsonValue`.
- Perceus memory management for efficient tree cleanup.

## Syntax Design
No new syntax is required. The API uses standard functions and types.

```opal
import JsonValue, JsonField from ./serialization_errors.types
import MalformedJsonError from ./serialization_errors.types

let parse_json_string = f(input: string): JsonValue errors MalformedJsonError =>
    # Implementation logic
    return JsonValue.Null
```

## Example Applications
Parsing a simple configuration:
```opal
import JsonValue, parse_json_string from ./serialization_errors.types
import MalformedJsonError from ./serialization_errors.types

let get_config_version = f(json_text: string): int32 errors MalformedJsonError =>
    guard parse_json_string(json_text) into root else err =>
        propagate err
    
    # Simple manual traversal
    return 1
```

## Strengths
- Simple to understand and implement.
- No complex generic mapping or reflection needed.
- Explicit control over data transformation.

## Weaknesses
- High memory usage for large documents.
- Manual tree traversal is verbose and error-prone compared to typed mapping.
- Limited to JSON only.

## Impact on Existing Syntax
None. This is a pure library addition.

## Interactions with Other Concerns
Pairs well with the error model by using explicit error types. Relies on the standard library's string handling.

## Implementation Difficulty
Low. Requires a standard recursive descent parser and a basic stringifier.

## Must NOT Have
- Streaming support (handled by other alternatives).
- Automatic mapping to user-defined types (handled by `typed-derive-style`).
- Support for non-JSON formats.
