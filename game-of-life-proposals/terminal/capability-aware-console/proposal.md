# Capability Aware Console

## Overview

This proposal introduces an explicit terminal handle with capability checks. Callers can decide whether to animate in place, print scrolling frames, or fail with a clear error.

Prior art includes terminfo/ncurses, Rust terminal crates, Node TTY checks, and Windows console mode detection.

## Assumes

- `Terminal` is an opaque handle.
- Capability checks are cheap and portable.
- Terminal operations report typed errors instead of silently dropping failures.

## Error Types

- `TerminalOpenFailureError`: emitted when opening the standard output terminal handle fails.
- `UnsupportedTerminalError`: emitted when the active terminal cannot support the requested control operation.
- `OutputNotTerminalError`: emitted when output is redirected or otherwise not attached to a terminal.
- `InvalidCursorPositionError`: emitted when row or column values are outside the supported terminal coordinate range.
- `ControlWriteFailureError`: emitted when writing a terminal control sequence fails.
- `WriteFailureError`: existing standard-library write failure error, reused for terminal text writes.
- `FlushFailureError`: emitted when flushing a terminal output sink fails.
- `SinkClosedError`: emitted when the terminal output sink is no longer writable.

## Proposed API

```opal
# terminal_open_stdout(): Terminal errors TerminalOpenFailureError
# terminal_supports_ansi(terminal: Terminal): boolean
# terminal_clear_sync(terminal: Terminal): void errors UnsupportedTerminalError, OutputNotTerminalError, ControlWriteFailureError
# terminal_write_sync(terminal: Terminal, text: string): void errors WriteFailureError, SinkClosedError
# terminal_flush_sync(terminal: Terminal): void errors FlushFailureError, SinkClosedError
```

## Syntax Design

```opal
import terminal_open_stdout, terminal_supports_ansi, terminal_clear_sync, terminal_write_sync, terminal_flush_sync from standard

let draw_frame = f(frame: string): void errors TerminalOpenFailureError, OutputNotTerminalError, UnsupportedTerminalError, ControlWriteFailureError, WriteFailureError, FlushFailureError, SinkClosedError =>
    let terminal = propagate terminal_open_stdout()
    if terminal_supports_ansi(terminal):
        propagate terminal_clear_sync(terminal)
    propagate terminal_write_sync(terminal, frame)
    propagate terminal_flush_sync(terminal)
    return void
```

## Strengths

1. Better Windows and redirected-output story.
2. Gives programs a graceful fallback path.
3. Scales to colors and cursor control later.
4. Testable through mock terminal handles.

## Weaknesses

1. More ceremony than direct functions.
2. Requires a handle type and runtime state.
3. Needs a stable capability vocabulary.

## Fit

- **Game fit**: Excellent.
- **Implementation effort**: Medium.
- **Long-term stdlib fit**: Excellent.

## Must NOT Have

- No assumption that stdout is always a TTY.
- No platform-specific branches in user code for basic behavior.
- No silent ANSI output when capability detection says unsupported.
- No ignored open, write, control, or flush failures.
