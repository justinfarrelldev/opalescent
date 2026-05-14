# String Join Lines

## Overview

This proposal adds a pure `string_join` helper that joins an array of strings with a separator. Game of Life can render each row independently, push it into `string[]`, and join rows with `\n`.

Prior art includes Python `separator.join(values)`, JavaScript `array.join(separator)`, Rust slice `join`, and many standard library collection APIs.

## Assumes

- Arrays of strings are already supported.
- `string_join` allocates once when possible.
- The function is pure and returns a new string.

## Proposed API

```opal
# string_join(values: string[], separator: string): string
```

## Syntax Design

```opal
import string_join from standard

let render_board = f(board: int32[][]): string =>
    let mutable rows: string[] = []
    let mutable row_index: int64 = 0
    while row_index < board.length:
        rows.push(render_row(board[row_index]))
        row_index = row_index + 1
    return string_join(rows, '\n')
```

## Example Application

```opal
import string_join from standard

let render_status = f(generation: int64, live_cells: int64): string =>
    let mutable parts: string[] = []
    parts.push('generation={generation}')
    parts.push('live={live_cells}')
    return string_join(parts, ' ')
```

## Strengths

1. Very small API with broad usefulness.
2. Encourages row-at-a-time rendering.
3. Keeps string values immutable.
4. Easier to optimize than arbitrary interpolation loops.

## Weaknesses

1. Requires building an intermediate `string[]`.
2. Less flexible than a builder for mixed incremental output.
3. Does not solve non-string formatting by itself.

## Fit

- **Game fit**: Excellent.
- **Implementation effort**: Low.
- **Long-term stdlib fit**: Excellent.

## Must NOT Have

- No method-only API until module organization settles.
- No special treatment of `\n`; it is just a separator string.
- No automatic trimming of empty rows.
