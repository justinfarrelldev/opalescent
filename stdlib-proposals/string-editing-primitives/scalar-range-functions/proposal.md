# Scalar Range Functions

## Overview
This proposal adds pure string editing helpers based on the same zero-based Unicode scalar indexes used by current string access and range helpers.

The core functions are `string_insert_at`, `string_delete_range`, and `string_replace_range`. They make common editor operations concise while preserving explicit range and allocation errors.

## Assumes
- Public strings are indexed by Unicode scalar position.
- `StringRangeError` covers range order and out-of-bounds failures.
- Allocation remains explicit through `AllocationFailureError`.

## Syntax Design
No new syntax is introduced.

```opal
let inserted = propagate string_insert_at('ac', 1 as int64, 'b')
let deleted = propagate string_delete_range('abcd', 1 as int64, 3 as int64)
let replaced = propagate string_replace_range('abcd', 1 as int64, 3 as int64, 'XY')
```

## Example Applications
```opal
import string_insert_at, string_delete_range from standard

let insert_text = f(line: string, column: int64, text: string): string errors StringRangeError, AllocationFailureError =>
    return propagate string_insert_at(line, column, text)

let backspace = f(line: string, column: int64): string errors StringRangeError, AllocationFailureError =>
    if column is 0:
        return line
    return propagate string_delete_range(line, column - 1, column)
```

## Strengths
- Minimal extension of existing scalar-indexed string APIs.
- Easy to explain and test.
- Useful outside editor code.
- Does not block future grapheme helpers.

## Weaknesses
- Not grapheme-cluster aware.
- Not terminal-cell aware.
- Repeated edits allocate new strings unless future builders or buffers optimize the path.

## Impact on Existing Syntax
No syntax changes. Adds standard-library symbols, documentation, runtime functions, and error family coverage.

## Interactions with Other Concerns
- Pairs with `terminal-text-layout` for rendering/cell clipping.
- Pairs with `collections-editing` for line insert/remove operations.
- Should use the same `StringRangeError` family as existing range helpers.

## Implementation Difficulty
Low. It can be implemented in terms of validated scalar slicing and concatenation, with care for allocation failure reporting.

## Must NOT Have
- No byte-index overloads.
- No silent clamping of invalid ranges.
- No claim of grapheme correctness.
