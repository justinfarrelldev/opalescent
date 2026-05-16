# Line Oriented Output

## Overview

This proposal embraces line output and adds a clearly named `print_line` helper. For Game of Life, each row and status line is rendered to a string first, then printed as a complete line.

This is less flexible than no-newline output, but it matches the current testing style and keeps snapshots deterministic.

## Assumes

- Programs render rows as strings before printing.
- `print_line` is either an alias for current string `print` behavior or a documented string-only function.
- The current generic `print` remains available.
- `print_line` reports output failures as typed errors instead of silently dropping them.

## Error Types

- `WriteFailureError`: existing standard-library write failure error, reused for line output.
- `SinkClosedError`: emitted when the output sink is no longer writable.

## Proposed API

```opal
# print_line(value: string): void errors WriteFailureError, SinkClosedError
```

## Syntax Design

```opal
import print_line from standard

let print_board = f(board: int32[][]): void errors WriteFailureError, SinkClosedError =>
    let mutable row_index: int64 = 0
    while row_index < board.length:
        propagate print_line(render_row(board[row_index]))
        row_index = row_index + 1
    return void
```

## Example Application

```opal
import print_line from standard

let print_generation = f(generation: int64, board: int32[][]): void errors WriteFailureError, SinkClosedError =>
    propagate print_line('generation {generation}')
    propagate print_board(board)
    return void
```

## Strengths

1. Very easy to understand.
2. Works well with existing integration-test output style.
3. Encourages deterministic rendering.
4. Avoids flush and buffering concerns for simple apps.

## Weaknesses

1. Not enough for cursor-addressed animation by itself.
2. Requires rows to be prebuilt as strings.
3. Still scrolls the terminal unless paired with clear/move APIs.

## Fit

- **Game fit**: Medium.
- **Implementation effort**: Low.
- **Long-term stdlib fit**: High.

## Must NOT Have

- No replacement of current `print` in one step.
- No hidden formatting beyond existing string interpolation.
- No automatic terminal clearing.
- No ignored write failures.
