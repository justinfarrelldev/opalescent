# High-Level Session Rendering Operations

## Overview
This proposal adds effectful rendering helpers that operate directly on `mutable ref TerminalSession`. They are convenience wrappers for common full-screen operations such as clear screen, cursor movement, drawing rows, flushing, cursor visibility, and chime feedback.

The design favors simple editor code over explicit trusted-output construction, while still preserving the selected terminal-session ownership model.

## Assumes
- `TerminalSession` exists and owns raw terminal output.
- Session writes continue to reject untrusted raw strings.
- Each helper maps to trusted runtime-controlled terminal output or capability-checked backend operations.

## Syntax Design
No new syntax is introduced. Functions live under `standard.terminal`.

```opal
let redraw_rows = f(mutable ref session: TerminalSession, rows: TrustedTerminalOutput[]): void errors TerminalSessionWriteError, TerminalSessionStateError, InvalidCursorPositionError =>
    propagate terminal_session_clear_screen_sync(mutable ref session)
    propagate terminal_session_move_cursor_sync(mutable ref session, 1 as int32, 1 as int32)
    propagate terminal_session_draw_rows_sync(mutable ref session, rows)
    return void
```

## Example Applications
```opal
import terminal_session_clear_screen_sync, terminal_session_move_cursor_sync from standard
import terminal_session_draw_rows_sync, terminal_session_flush_sync from standard

let redraw = f(mutable ref session: TerminalSession, rows: TrustedTerminalOutput[]): void errors TerminalSessionWriteError, TerminalSessionStateError, InvalidCursorPositionError =>
    propagate terminal_session_clear_screen_sync(mutable ref session)
    propagate terminal_session_draw_rows_sync(mutable ref session, rows)
    propagate terminal_session_move_cursor_sync(mutable ref session, 1 as int32, 1 as int32)
    propagate terminal_session_flush_sync(mutable ref session)
    return void
```

## Strengths
- Very readable call sites for beginner editor examples.
- Centralizes capability handling and fake-backend recording in session operations.
- Keeps legacy stdout terminal handles out of full-screen applications.

## Weaknesses
- Larger effectful API surface than pure builder helpers.
- Less explicit about trust construction at the application boundary.
- Can grow into many small terminal operations if not kept narrow.

## Impact on Existing Syntax
No syntax changes. Requires new standard-library declarations, runtime support, fake-backend assertions, and generated-code lowering.

## Interactions with Other Concerns
This can be layered over the trusted ANSI builder. It should not bypass `terminal-text-layout`; row drawing still needs trusted clipped text.

## Implementation Difficulty
Medium. Each helper needs runtime/fake backend behavior and C ABI coverage.

## Must NOT Have
- No raw-string session draw API.
- No output-terminal acquisition from a session.
- No implicit fallback to legacy stdout while the session is active.
