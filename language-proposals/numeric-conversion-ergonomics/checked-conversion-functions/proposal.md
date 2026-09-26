# Checked Numeric Conversion Functions

## Overview

This alternative adds explicit checked conversion helpers for narrowing or otherwise fallible numeric conversions:

```opal
int64_to_int32_checked(value: int64): int32 errors IntegerRangeError
```

It does not remove all ceremony, but it replaces raw casts at unsafe boundaries with named, checked operations. It is the general-purpose companion to API-specific fixes such as widening terminal coordinates.

## Assumes

- Opalescent keeps explicit numeric conversions as a safety principle.
- Runtime-value narrowing cannot be implicit unless proven safe.
- New numeric range errors can be declared in `standard.errors` or a numeric module.
- Wrapping and saturating conversions, if provided, have explicit names.

## Syntax Design

No new syntax.

### Function families

Possible module shape:

```opal
import int64_to_int32_checked from standard.numeric
```

or, if numeric helpers remain in `standard` initially:

```opal
import int64_to_int32_checked from standard
```

Signature:

```opal
int64_to_int32_checked(value: int64): int32 errors IntegerRangeError
```

Additional helpers follow the same naming pattern:

```text
int64_to_int16_checked
int64_to_uint32_checked
uint64_to_int64_checked
float64_to_int32_checked
```

### Explicit non-checked variants

If added, their names must reveal behavior:

```text
int64_to_int32_wrapping
int64_to_int32_saturating
```

They must not be called plain `int64_to_int32`.

## Example Applications

```opal
import int64_to_int32_checked from standard.numeric
import terminal_session_move_cursor_sync from standard.terminal
import type TerminalSession, TerminalSessionStateError, TerminalSessionWriteError from standard.terminal

##
  Description: Move to the status row by checked-converting editor coordinates for an int32 terminal API.
##
public let move_to_status_row = f(
    session: mutable ref TerminalSession,
    visible_rows: int64
): void errors
    IntegerRangeError,
    TerminalSessionWriteError,
    TerminalSessionStateError,
    InvalidCursorPositionError
=>
    let row = propagate int64_to_int32_checked(visible_rows + 1)
    propagate terminal_session_move_cursor_sync(mutable ref session, row, 1 as int32)
    return void
```

## Strengths

- General solution for fallible numeric narrowing.
- Makes range checking explicit and searchable.
- Avoids raw `as` when runtime overflow is possible.
- Good fit for FFI/platform boundaries.
- Can coexist with wrapping and saturating helpers.
- Easy to test exhaustively for boundary values.

## Weaknesses

- Still adds ceremony and `propagate`.
- Adds errors to signatures.
- Does not fix API mismatches by itself.
- Many numeric type combinations create a large helper surface.
- If overused, code can become conversion-heavy.

## Impact on Existing Syntax

No syntax changes.

The standard library gains numeric conversion APIs and an error such as `IntegerRangeError` or `NumericRangeError`. Documentation should state when `as` is allowed and when checked helpers are preferred.

## Interactions with Other Concerns

- **Cast rules:** checked helpers are the explicit APIs referenced by the current cast goals for conversions that cannot be proven safe.
- **Terminal int64 APIs:** terminal wrappers can use these internally.
- **Error annotations:** helpers introduce numeric range errors; named error sets may be useful.
- **Strict floats:** float-to-int checked conversions should align with strict float policy for NaN/Inf.
- **LSP:** diagnostics for unsafe casts can suggest checked helpers.

## Implementation Difficulty

Low to medium.

Main tasks:

1. Choose module and naming convention.
2. Add numeric range error type(s).
3. Implement core conversion helpers.
4. Register signatures in the type system.
5. Add runtime/codegen lowering.
6. Add diagnostics suggesting checked helpers for unsafe casts.
7. Document wrapping/saturating alternatives separately if added.

## Must NOT Have

- Must not silently wrap, clamp, or saturate in checked helpers.
- Must not encourage replacing safe widening casts with noisy checked calls.
- Must not use a generic `convert` name that hides behavior.
- Must not make out-of-range constants runtime errors; those should stay compile-time errors where possible.
