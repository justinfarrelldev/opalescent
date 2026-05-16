# ANSI Control Functions

## Overview

This proposal adds small terminal helpers that map to ANSI escape sequences where supported. They provide the fastest path to an animated terminal Game of Life.

## Assumes

- Functions write to stdout or the active terminal output stream.
- Unsupported terminals return a typed error; callers that want fallback behavior should use capability-aware terminal APIs.
- Coordinates are 1-based, matching common ANSI cursor addressing.
- Terminal control failures are reported as typed errors.

## Error Types

- `UnsupportedTerminalError`: emitted when the active terminal cannot support the requested control operation.
- `OutputNotTerminalError`: emitted when output is redirected or otherwise not attached to a terminal.
- `InvalidCursorPositionError`: emitted when row or column values are outside the supported terminal coordinate range.
- `ControlWriteFailureError`: emitted when writing a terminal control sequence fails.

Examples that also write frame text use `WriteFailureError`, `FlushFailureError`, and `SinkClosedError` from the output proposal.

## Proposed API

```opal
# terminal_clear_screen_sync(): void errors UnsupportedTerminalError, OutputNotTerminalError, ControlWriteFailureError
# terminal_move_cursor_sync(row: int32, column: int32): void errors UnsupportedTerminalError, OutputNotTerminalError, InvalidCursorPositionError, ControlWriteFailureError
# terminal_hide_cursor_sync(): void errors UnsupportedTerminalError, OutputNotTerminalError, ControlWriteFailureError
# terminal_show_cursor_sync(): void errors UnsupportedTerminalError, OutputNotTerminalError, ControlWriteFailureError
```

## Syntax Design

```opal
import print_text, flush_standard_output_sync, terminal_clear_screen_sync, terminal_move_cursor_sync from standard

let draw_at_top = f(frame: string): void errors UnsupportedTerminalError, OutputNotTerminalError, InvalidCursorPositionError, ControlWriteFailureError, WriteFailureError, FlushFailureError, SinkClosedError =>
    propagate terminal_move_cursor_sync(1 as int32, 1 as int32)
    propagate terminal_clear_screen_sync()
    propagate print_text(frame)
    propagate flush_standard_output_sync()
    return void
```

## Strengths

1. Small and easy to teach.
2. Excellent for simple terminal animation.
3. No new language syntax.
4. Maps directly to ANSI-capable terminals.

## Weaknesses

1. Raw ANSI assumptions can be wrong on some Windows or redirected outputs.
2. No capability discovery.
3. No flicker minimization beyond caller discipline.

## Fit

- **Game fit**: Excellent for demos.
- **Implementation effort**: Low.
- **Long-term stdlib fit**: Medium.

## Must NOT Have

- No hard-coded POSIX-only behavior.
- No failure to restore the cursor after runtime errors when avoidable.
- No color API until basic control is stable.
- No ignored terminal capability or control-write failures.
