# Int64 Terminal Coordinate APIs

## Overview

This alternative changes public terminal row/column APIs to accept `int64` coordinates instead of `int32`. It directly addresses casts in editor code because editor positions, line counts, string lengths, and integer literals are already `int64`.

Instead of:

```opal
propagate terminal_session_move_cursor_sync(mutable ref session, (visible_rows + 1) as int32, 1 as int32)
```

users write:

```opal
propagate terminal_session_move_cursor_sync(mutable ref session, visible_rows + 1, 1)
```

The runtime still validates the values and can downcast internally after checks.

## Assumes

- Terminal source coordinates are conceptually small but represented as Opalescent `int64` at the source level.
- Runtime/backend APIs may still require `int32` or platform-specific widths internally.
- Terminal functions already validate invalid cursor positions and can extend validation to overlarge values.
- Existing `int32` arguments can be widened to `int64` explicitly or through literal inference/migration.

## Syntax Design

No new syntax.

Change selected public signatures from:

```opal
terminal_session_move_cursor_sync(session: mutable ref TerminalSession, row: int32, column: int32): void errors ...
```

to:

```opal
terminal_session_move_cursor_sync(session: mutable ref TerminalSession, row: int64, column: int64): void errors ...
```

Similarly consider high-level rendering helpers, size APIs, and any terminal cursor/viewport helper that currently asks application code for `int32` coordinates.

## Example Applications

```opal
import terminal_session_move_cursor_sync from standard.terminal
import type TerminalSession from standard.terminal

##
  Description: Move the terminal cursor to the editor status row using int64 coordinates.
##
public let move_to_status_row = f(
    session: mutable ref TerminalSession,
    visible_rows: int64
): void errors TerminalSessionWriteError, TerminalSessionStateError, InvalidCursorPositionError =>
    propagate terminal_session_move_cursor_sync(mutable ref session, visible_rows + 1, 1)
    return void
```

## Strengths

- Removes the most common terminal/editor cast noise.
- Aligns with Opalescent default integer literals and `.length` results.
- Keeps invalid positions explicit through existing terminal errors.
- Avoids introducing new conversion syntax.
- Keeps arithmetic in one integer width through editor code.
- Runtime can still enforce platform/backend limits.

## Weaknesses

- Changes public stdlib signatures.
- Existing code that stores terminal positions as `int32` may need widening casts or variable type changes.
- Runtime still needs internal downcasts for C/OS calls.
- Does not help other APIs that truly need fixed-width integers.
- If terminals logically use one-based positive positions, raw `int64` still does not encode that invariant.

## Impact on Existing Syntax

No syntax change, but stdlib API signatures change. Migration options:

1. Add `int64` overloads or new names first, deprecate `int32` versions later.
2. Change signatures directly before API stability.
3. Keep old runtime symbols but expose `int64` source-level wrappers.

The formatter and parser are unaffected.

## Interactions with Other Concerns

- **Checked conversion functions:** runtime wrappers can use checked downcasts internally.
- **Terminal coordinate types:** wrapper types can be added later if raw `int64` is not expressive enough.
- **Error annotations:** overlarge coordinates can reuse `InvalidCursorPositionError` or add a more specific range error.
- **Hot reload/ABI:** changing public terminal signatures changes interface hashes and should be versioned.
- **Numeric literal inference:** target-typed literals become less necessary for terminal calls once parameters are `int64`.

## Implementation Difficulty

Medium.

Main tasks:

1. Audit terminal APIs for `int32` coordinate parameters.
2. Decide migration strategy for existing names.
3. Update type-system standard symbol registrations.
4. Update codegen runtime declarations and wrappers.
5. Add runtime range checks before platform downcasts.
6. Update examples and fixtures to remove casts.
7. Document coordinate base and valid ranges explicitly.

## Must NOT Have

- Must not silently wrap or truncate `int64` values inside the runtime.
- Must not remove invalid-position errors.
- Must not widen every numeric API indiscriminately; this proposal targets source-level terminal coordinates.
- Must not make FFI fixed-width requirements invisible.
