# Nullable Sentinel Types

## Overview
This proposal uses sentinel values within existing types to represent the absence of a value. Instead of introducing new wrapper types or error conditions, functions return a conventional "empty" or "invalid" value to signal that no data was found or produced.

Common sentinels include `-1` for indices, an empty string `""` for missing text, or `0` for certain numeric counts. This approach relies on documentation and developer convention rather than formal type system enforcement. It is the most performant and low-level way to handle optionality.

## Assumes
- The existing base types (int32, string, etc.) can represent sentinel values.
- A standard set of conventions for what constitutes a "missing" value for each type.
- Developers are disciplined in checking for sentinel values before using returned data.

## Syntax Design
No new syntax is required. This proposal utilizes existing primitive types and comparison operators.

```opal
let find_index = f(items: int32[], target: int32): int32 =>
    for i in 0..items.length:
        if items[i] is target:
            return i
    return -1
```

Callers check for the sentinel using `is`:

```opal
let index = find_index([1, 2, 3], 4)
if index is -1:
    print('Not found')
```

## Example Applications
A common application is finding a character in a string.

```opal
let find_char = f(text: string, char: string): int32 =>
    # Returns -1 if character is not found
    return -1

let start = find_char('hello', 'z')
if start is not -1:
    # Safe to use index
    return void
```

## Strengths
- **Maximum Performance**: No allocations, no tagged union overhead, and no error handling state.
- **Simplicity**: Extremely easy to understand and implement for developers coming from languages like C or Go.
- **Familiarity**: Matches patterns already common in many established libraries and languages.
- **Minimal Boilerplate**: Simple equality checks are often shorter than `guard` or pattern matching.

## Weaknesses
- **Lack of Type Safety**: The compiler cannot prevent a developer from forgetting to check for a sentinel value.
- **Sentinel Collision**: In some cases, every possible value of a type is valid, making it impossible to find a unique sentinel (e.g., any `int32` could be a valid result).
- **Inconsistency**: Different libraries might use different sentinels for the same type (e.g., `-1` vs `0`), leading to confusion.

## Impact on Existing Syntax
None. This proposal uses existing language features and is purely a matter of convention.

## Interactions with Other Concerns
- **LSP**: Language servers could potentially warn when a sentinel check is missing, but this requires more complex static analysis.
- **Standard Library**: Requires careful documentation of all functions that return sentinel values.
- **Memory Model**: No impact on Perceus+SCR, as no new memory is allocated.

## Implementation Difficulty
Zero. This proposal requires no changes to the compiler or runtime. It is a documentation and standard library design choice.

## Must NOT Have
- **Hidden Nulls**: Must not introduce a universal `null` keyword.
- **Magic Numbers**: Avoid using arbitrary sentinels; stick to established conventions like `-1` for indices or `""` for strings.
