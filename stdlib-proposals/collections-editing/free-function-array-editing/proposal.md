# Free-Function Array Editing

## Overview
This proposal adds generic free functions for inserting and removing array elements by index. It gives editor line buffers stable operations without requiring new method dispatch work.

## Assumes
- Arrays use `T[]` syntax and explicit allocation failures.
- Index failures use `IndexOutOfBoundsError`.
- Returning updated arrays fits the current immutable-by-default style, even when implementations optimize in place.

## Syntax Design
No new syntax is introduced.

```opal
let insert_then_remove = f(lines: string[]): string[] errors IndexOutOfBoundsError, AllocationFailureError =>
    let updated = propagate array_insert(lines, 1 as int64, 'new line')
    let after_remove, removed = propagate array_remove_at(updated, 0 as int64)
    return after_remove
```

## Example Applications
```opal
import array_insert, array_remove_at from standard

let split_line = f(lines: string[], line_index: int64, before: string, after: string): string[] errors IndexOutOfBoundsError, AllocationFailureError =>
    let replaced, removed = propagate array_remove_at(lines, line_index)
    let with_before = propagate array_insert(replaced, line_index, before)
    return propagate array_insert(with_before, line_index + 1, after)
```

## Strengths
- Smallest public surface for insertion/removal.
- Straightforward generic signatures.
- Avoids exposing undocumented runtime vector internals.

## Weaknesses
- Less ergonomic than `values.insert(...)`.
- Repeated immutable-style calls can appear allocation-heavy.
- Multiple-return removal needs clear documentation.

## Impact on Existing Syntax
No syntax changes. Adds standard-library functions and generated-code support.

## Interactions with Other Concerns
Pairs with string editing for line split/join. Can later receive method aliases from `method-style-array-editing`.

## Implementation Difficulty
Low to medium. Runtime support exists conceptually through vector insert/remove behavior, but it needs public signatures, lowering, error mapping, and tests.

## Must NOT Have
- No silent negative-index wrapping.
- No mutation hidden behind a borrowed immutable array parameter.
- No undocumented tuple return; removal uses labeled multiple returns.
