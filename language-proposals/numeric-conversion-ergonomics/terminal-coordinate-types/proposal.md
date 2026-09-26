# Terminal Coordinate Types

## Overview

This alternative introduces domain-specific terminal coordinate types instead of passing raw integers to cursor APIs:

```opal
let row = propagate terminal_row_from_one_based(visible_rows + 1)
let column = terminal_column_first()
propagate terminal_session_move_cursor_sync(mutable ref session, row, column)
```

The wrapper types encode terminal coordinate invariants such as one-based positions and valid backend ranges. This reduces raw casts and makes terminal code more self-documenting, but it is a larger API change than simply using `int64` parameters.

## Assumes

- Terminal APIs have enough coordinate invariants to justify wrapper types.
- Wrapper construction can be fallible for invalid or out-of-range values.
- Wrapper values are small and cheap to pass.
- The standard library can expose these types from `standard.terminal`.

## Syntax Design

### Types

```opal
public type TerminalRow:
    value: int64

public type TerminalColumn:
    value: int64
```

The actual representation may be opaque. Users should construct values through functions rather than direct fields if invariants matter.

### Constructors

```opal
terminal_row_from_one_based(value: int64): TerminalRow errors InvalidCursorPositionError
terminal_column_from_one_based(value: int64): TerminalColumn errors InvalidCursorPositionError
terminal_column_first(): TerminalColumn
```

### Terminal API

```opal
terminal_session_move_cursor_sync(
    session: mutable ref TerminalSession,
    row: TerminalRow,
    column: TerminalColumn
): void errors TerminalSessionWriteError, TerminalSessionStateError
```

`InvalidCursorPositionError` moves from the write call into coordinate construction when coordinates are created from raw integers.

## Example Applications

```opal
import terminal_column_first, terminal_row_from_one_based from standard.terminal
import terminal_session_move_cursor_sync from standard.terminal
import type TerminalColumn, TerminalRow, TerminalSession from standard.terminal
import type TerminalSessionStateError, TerminalSessionWriteError from standard.terminal

##
  Description: Move to the editor status row using validated terminal coordinate values.
##
public let move_to_status_row = f(
    session: mutable ref TerminalSession,
    visible_rows: int64
): void errors InvalidCursorPositionError, TerminalSessionWriteError, TerminalSessionStateError =>
    let row: TerminalRow = propagate terminal_row_from_one_based(visible_rows + 1)
    let column: TerminalColumn = terminal_column_first()
    propagate terminal_session_move_cursor_sync(mutable ref session, row, column)
    return void
```

## Strengths

- Makes terminal coordinate domain explicit.
- Removes raw `int32` casts from application code.
- Prevents mixing row and column arguments accidentally.
- Can encode one-based vs zero-based coordinate conventions.
- Moves validation to construction boundaries.
- Useful for richer terminal layout APIs.

## Weaknesses

- More types and imports for simple terminal code.
- Existing APIs must change substantially.
- Users still need `propagate` when constructing from dynamic integers.
- May feel heavy for simple cursor movement.
- Requires clear policy for extracting raw values for FFI/runtime internals.

## Impact on Existing Syntax

No language syntax change, but significant stdlib API changes. Callers must construct `TerminalRow` and `TerminalColumn` values instead of passing integers directly.

A migration could keep raw-int APIs as convenience wrappers while introducing typed APIs for safety-critical code.

## Interactions with Other Concerns

- **Int64 terminal APIs:** raw `int64` APIs can be a simpler v1; coordinate types can be v2.
- **Checked conversions:** constructors perform checked conversion internally if backend limits require narrower values.
- **Error annotations:** invalid coordinate errors occur at constructor calls.
- **Terminal text layout:** row/column types can compose with viewport and size types later.
- **LSP/docs:** strong types improve hover/help text for terminal APIs.

## Implementation Difficulty

Medium to high.

Main tasks:

1. Design terminal coordinate types and visibility.
2. Add constructors and constants.
3. Update terminal function signatures or add typed overload names.
4. Implement runtime extraction and validation.
5. Update examples and fixtures.
6. Decide migration/deprecation policy for raw-int APIs.

## Must NOT Have

- Must not expose wrapper fields publicly if invariants can be bypassed.
- Must not require coordinate wrappers for every numeric terminal setting without clear benefit.
- Must not silently clamp invalid coordinates.
- Must not make simple terminal examples unreadable.
