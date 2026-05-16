# Screen Buffer Renderer

## Overview

This proposal adds a row-oriented screen buffer renderer. Programs provide the full frame as `string[]`, and the terminal module handles clear, cursor movement, writes, and flush.

This is the most ergonomic option for grid simulations because the application thinks in rows, not escape sequences.

## Assumes

- Rows are ordinary `string[]` values.
- The renderer can choose direct ANSI output or fallback scrolling output.
- The first version redraws whole frames rather than computing diffs.
- Drawing failures are reported as typed terminal errors.

## Error Types

- `UnsupportedTerminalError`: emitted when the active terminal cannot support the requested draw operation.
- `OutputNotTerminalError`: emitted when output is redirected or otherwise not attached to a terminal.
- `InvalidCursorPositionError`: emitted when row or column values are outside the supported terminal coordinate range.
- `ControlWriteFailureError`: emitted when writing a terminal control sequence fails.
- `WriteFailureError`: existing standard-library write failure error, reused for row text writes.
- `FlushFailureError`: emitted when flushing the rendered frame fails.
- `SinkClosedError`: emitted when the terminal output sink is no longer writable.

## Proposed API

```opal
# terminal_draw_rows_sync(rows: string[]): void errors UnsupportedTerminalError, OutputNotTerminalError, ControlWriteFailureError, WriteFailureError, FlushFailureError, SinkClosedError
# terminal_draw_rows_at_sync(row: int32, column: int32, rows: string[]): void errors UnsupportedTerminalError, OutputNotTerminalError, InvalidCursorPositionError, ControlWriteFailureError, WriteFailureError, FlushFailureError, SinkClosedError
```

## Syntax Design

```opal
import terminal_draw_rows_sync from standard

let draw_board = f(board: int32[][]): void errors UnsupportedTerminalError, OutputNotTerminalError, ControlWriteFailureError, WriteFailureError, FlushFailureError, SinkClosedError =>
    let mutable rows: string[] = []
    let mutable row_index: int64 = 0
    while row_index < board.length:
        rows.push(render_row(board[row_index]))
        row_index = row_index + 1
    propagate terminal_draw_rows_sync(rows)
    return void
```

## Strengths

1. Best API shape for grid programs.
2. Keeps terminal control out of application logic.
3. Leaves room for diff-based rendering later.
4. Works naturally with `string_join` or row renderers.

## Weaknesses

1. More runtime behavior than direct ANSI helpers.
2. Needs a terminal fallback policy.
3. Less general than a writer sink or full terminal handle.

## Fit

- **Game fit**: Excellent.
- **Implementation effort**: Medium-High.
- **Long-term stdlib fit**: High.

## Must NOT Have

- No nested card/UI abstraction in the terminal layer.
- No hidden simulation state.
- No mandatory color or styling support in the first version.
- No ignored terminal draw failures.
