# JSON Plus TOML Uniform API

## Overview
This proposal provides a single, uniform API for both JSON and TOML serialization. Both formats are parsed into or serialized from a generic `StructuredValue` tree. This is highly beneficial for configuration systems that want to support multiple formats interchangeably.

The core functions are `parse_json_string`, `parse_toml_string`, `serialize_to_json_string`, and `serialize_to_toml_string`.

## Assumes
- Standard string and array handling.
- Tagged union support in the compiler for `StructuredValue`.
- Shared core tree representation.

## Syntax Design
No new syntax is required. The API uses standard functions and types, with both formats exposing identical shapes.

```opal
import StructuredValue from ./serialization_errors.types
import MalformedJsonError from ./serialization_errors.types
import MalformedTomlError from ./serialization_errors.types

let parse_json_string = f(input: string): StructuredValue errors MalformedJsonError =>
    # Implementation logic
    return StructuredValue.Null

let parse_toml_string = f(input: string): StructuredValue errors MalformedTomlError =>
    # Implementation logic
    return StructuredValue.Null
```

## Example Applications
A configuration loader that detects format:
```opal
import StructuredValue, parse_json_string, parse_toml_string from ./serialization_errors.types

let load_config = f(content: string, is_toml: bool): StructuredValue errors MalformedJsonError, MalformedTomlError =>
    if is_toml:
        guard parse_toml_string(content) into root else err =>
            propagate err
        return root
    else:
        guard parse_json_string(content) into root else err =>
            propagate err
        return root
```

## Strengths
- Unified API simplifies developer burden when supporting multiple formats.
- High flexibility for configuration use cases.
- Reuses common tree traversal logic.

## Weaknesses
- Adds TOML to the standard library's baseline requirements.
- Slightly more complex tree representation needed to support format-specific features if needed.

## Impact on Existing Syntax
None. Pure library addition.

## Interactions with Other Concerns
Consistent error handling across formats.

## Implementation Difficulty
Medium. Requires two distinct parsers (JSON and TOML) and two distinct emitters.

## Must NOT Have
- Formats other than JSON and TOML.
- Streaming support for TOML (typically a file-based format).
