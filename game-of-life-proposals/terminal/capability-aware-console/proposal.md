# Capability Aware Console

## Overview

This proposal introduces an explicit terminal handle with capability checks. Callers can decide whether to animate in place, print scrolling frames, or fail with a clear error.

Prior art includes terminfo/ncurses, Rust terminal crates, Node TTY checks, and Windows console mode detection.

## Assumes

- `Terminal` is an opaque handle.
- Capability checks are cheap and portable.
- Terminal operations can report typed errors in a later error taxonomy.

## Proposed API

```opal
# terminal_open_stdout(): Terminal
# terminal_supports_ansi(terminal: Terminal): boolean
# terminal_clear_sync(terminal: Terminal): void
# terminal_write_sync(terminal: Terminal, text: string): void
# terminal_flush_sync(terminal: Terminal): void
```

## Syntax Design

```opal
import terminal_open_stdout, terminal_supports_ansi, terminal_clear_sync, terminal_write_sync, terminal_flush_sync from standard

let draw_frame = f(frame: string): void =>
    let terminal = terminal_open_stdout()
    if terminal_supports_ansi(terminal):
        terminal_clear_sync(terminal)
    terminal_write_sync(terminal, frame)
    terminal_flush_sync(terminal)
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
