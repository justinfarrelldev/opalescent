# Line Buffer Object

## Overview
This proposal introduces a `TextBuffer` abstraction for line-oriented editing. Instead of manipulating `string[]` directly, applications call buffer operations such as insert text, split line, join line, insert line, and remove line.

This is attractive for editor code, but it is larger than the primitive helpers needed for the first generated editor fixture.

## Assumes
- Product/sum type declarations in `.types.op` files.
- Explicit errors for line/column range failures and allocation.
- Existing or proposed string and array editing primitives.

## Syntax Design
No new syntax is introduced.

```opal
let buffer = propagate text_buffer_from_lines(lines)
let updated = propagate text_buffer_insert_text(buffer, 1 as int64, 0 as int64, '!')
```

## Example Applications
```opal
import text_buffer_from_lines, text_buffer_insert_text, text_buffer_to_lines from standard

let add_bang = f(lines: string[]): string[] errors TextBufferError, AllocationFailureError =>
    let buffer = propagate text_buffer_from_lines(lines)
    let edited = propagate text_buffer_insert_text(buffer, 1 as int64, 4 as int64, '!')
    return propagate text_buffer_to_lines(edited)
```

## Strengths
- Best editor ergonomics.
- Centralizes line/range validation.
- Allows future optimized storage without changing editor source.

## Weaknesses
- Premature abstraction for v1.
- Requires careful ownership/copy semantics.
- Might hide allocation costs.

## Impact on Existing Syntax
No syntax changes. Adds a new standard-library type and functions.

## Interactions with Other Concerns
Builds on `string-editing-primitives` and `collections-editing`. Could later use ropes or piece tables internally without changing the public type.

## Implementation Difficulty
Medium to high depending on whether the initial buffer is immutable, affine, or internally optimized.

## Must NOT Have
- No implicit file I/O.
- No hidden terminal display-width policy.
- No mutable global editor state.
