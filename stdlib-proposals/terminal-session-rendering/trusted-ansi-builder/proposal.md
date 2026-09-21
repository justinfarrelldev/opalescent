# Trusted ANSI Builder

## Overview
This proposal adds pure standard-library helpers that construct `TrustedTerminalOutput` values for reviewed ANSI terminal control sequences. Applications still write through `terminal_session_write_sync`, so the selected terminal-session output trust boundary remains intact.

The v1 goal is enough rendering support for a small full-screen editor: enter/leave alternate screen, clear screen, move cursor, hide/show cursor, choose cursor shape, append trusted fragments, and optionally emit a terminal bell. User text is never implicitly trusted by these helpers.

## Assumes
- The selected `terminal-session-input/typed-event-session` lifecycle and output trust model.
- `TrustedTerminalOutput` remains nominal and cannot be constructed from raw `string` except through explicit reviewed/declassification helpers.
- Fake terminal backends can inspect the trusted operations or the resulting trusted byte stream in tests.

## Syntax Design
No new language syntax is introduced. The proposal adds functions under `standard.terminal`.

```opal
import terminal_session_write_sync, terminal_session_flush_sync from standard
import trusted_terminal_output_clear_screen, trusted_terminal_output_move_cursor from standard

let write_clear_and_home = f(mutable ref session: TerminalSession): void errors TerminalSessionWriteError, TerminalSessionStateError, InvalidCursorPositionError =>
    let frame = trusted_terminal_output_clear_screen()
    propagate terminal_session_write_sync(mutable ref session, frame)
    let home = propagate trusted_terminal_output_move_cursor(1 as int32, 1 as int32)
    propagate terminal_session_write_sync(mutable ref session, home)
    propagate terminal_session_flush_sync(mutable ref session)
    return void
```

## Example Applications
A simple editor can build a full frame as trusted fragments and write them through the owned session.

```opal
import terminal_session_write_sync, terminal_session_flush_sync from standard
import trusted_terminal_output_clear_screen, trusted_terminal_output_move_cursor from standard
import trusted_terminal_output_from_application_text, trusted_terminal_output_join from standard

let render_editor_frame = f(mutable ref session: TerminalSession, rows: string[]): void errors TerminalSessionWriteError, TerminalSessionStateError, TerminalOutputTrustError, AllocationFailureError =>
    let mutable parts: TrustedTerminalOutput[] = []
    parts.push(trusted_terminal_output_clear_screen())
    parts.push(propagate trusted_terminal_output_move_cursor(1 as int32, 1 as int32))

    for row in rows:
        let trusted_row = propagate trusted_terminal_output_from_application_text(row)
        parts.push(trusted_row)

    let frame = propagate trusted_terminal_output_join(parts)
    propagate terminal_session_write_sync(mutable ref session, frame)
    propagate terminal_session_flush_sync(mutable ref session)
    return void
```

## Strengths
- Preserves the explicit trust boundary selected by terminal-session/input.
- Keeps rendering helpers pure except for allocation-fallible joining.
- Works naturally with fake backend assertions.
- Leaves room for future high-level frame/canvas APIs.

## Weaknesses
- Call sites are more verbose than direct session methods.
- ANSI support must be capability-gated or modeled for non-ANSI terminals.
- Incorrect trusted composition can still produce visually bad frames, even though untrusted strings are blocked.

## Impact on Existing Syntax
No parser or formatter changes are required. The module resolver, runtime readiness inventory, C ABI, Rust runtime model, and documentation need new standard-library symbols.

## Interactions with Other Concerns
- Depends on terminal session lifecycle and output trust.
- Pairs with `terminal-text-layout` for correct cell clipping.
- Pairs with `string-editing-primitives` and `collections-editing` for editor buffer rendering.
- Must not conflict with legacy stdout terminal helpers; those remain rejected while a session owns the coordinator.

## Implementation Difficulty
Low to medium. Pure trusted constructors are straightforward, but the runtime must keep capability checks, fake-backend observability, and C ABI lowering coherent.

## Must NOT Have
- No API that writes raw `string` to a terminal session.
- No `terminal_session_output_terminal` or acquired stdout handle.
- No implicit declassification of user input.
- No hidden global terminal owner.
