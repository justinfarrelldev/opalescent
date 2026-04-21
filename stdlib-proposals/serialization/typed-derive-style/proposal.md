# Typed Derive Style

## Overview
This proposal allows mapping JSON directly to user-defined types. This is the most ergonomic and type-safe approach for application-level data. The proposal highlights the eventual need for a `derive Serialize/Deserialize` attribute to automate this mapping. In the interim, developers provide manual transformation functions between user types and a low-level `JsonValue` tree.

The key feature is direct mapping of JSON structure to statically-typed records.

## Assumes
- Standard library JSON value tree (similar to `json-only-value-tree`).
- Prerequisites for future meta-programming or macro-based `derive` support.

## Syntax Design
While a future `derive` annotation is envisioned, the current design uses explicit mapping functions.

```opal
import Config, JsonValue from ./serialization_errors.types

let config_from_json_value = f(root: JsonValue): Config errors MissingRequiredFieldError, UnexpectedJsonShapeError =>
    # Hand-written transformation logic
    return new Config:
               version: 1
               status: "ok"
```

## Example Applications
Loading configuration directly into a record:
```opal
import Config, parse_json_string from ./serialization_errors.types
import config_from_json_value from ./serialization_errors.types

let get_config = f(json_text: string): Config errors MalformedJsonError, MissingRequiredFieldError, UnexpectedJsonShapeError =>
    guard parse_json_string(json_text) into root else err =>
        propagate err
        
    guard config_from_json_value(root) into config else err =>
        propagate err
        
    return config
```

## Strengths
- Highly ergonomic for the developer.
- Strong type safety for structured data.
- Minimizes manual tree traversal.

## Weaknesses
- Currently requires boilerplate mapping functions.
- Relies on future language features for full ergonomics.

## Impact on Existing Syntax
A `derive Serialize/Deserialize` macro extension is a prerequisite for full ergonomics and will affect the grammar of type definitions.

## Interactions with Other Concerns
Deeply integrated with the type system and error model.

## Implementation Difficulty
Medium to High. Requires either heavy boilerplate or a sophisticated macro system.

## Must NOT Have
- Dynamic, untyped mapping that bypasses the type system.
- Implicit mapping without explicit function calls or annotations.
