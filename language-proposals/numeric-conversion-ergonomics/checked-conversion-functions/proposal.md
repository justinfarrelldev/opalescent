# Checked Numeric Conversion Functions

## Overview

This alternative adds explicit value-preserving numeric conversion helpers for narrowing or otherwise fallible numeric conversions:

```opal
int64_to_int32(value: int64): int32 errors IntegerRangeError
```

The plain `A_to_B` name is intentionally the checked conversion. Opalescent should not expose an ordinary unchecked `int64_to_int32` that silently truncates, wraps, or clamps. The fallibility is already visible through the `errors` clause and at call sites through `propagate` or `guard`.

This proposal does not add `_checked`, `_unchecked`, `_saturating`, `_wrapping`, or `_or` conversion variants. If a future low-level domain needs wrapping or saturation, that behavior must be proposed separately with explicit names and module placement.

## Assumes

- Opalescent keeps explicit numeric conversions as a safety principle.
- Runtime-value narrowing cannot be implicit unless proven safe.
- Plain numeric conversion helper names mean value-preserving conversion or error.
- Widening conversions that are statically known to be safe may continue to use `as`; these helpers are for conversions that can fail at runtime.
- New numeric range errors can be declared in `standard.errors` or a numeric module.

## Syntax Design

No new syntax.

### Function families

Preferred module shape:

```opal
import int64_to_int32 from standard.numeric
```

Root `standard` may re-export the same helpers for compatibility with today's flat stdlib style, but `standard.numeric` is the domain home.

Signature:

```opal
int64_to_int32(value: int64): int32 errors IntegerRangeError
```

Additional helpers follow the same naming pattern when the conversion can fail:

```text
int32_to_int16
int64_to_uint32
uint64_to_int64
float64_to_int32
float64_to_float32
```

### Semantics

`A_to_B` succeeds only when the source value can be represented by `B` without changing the numeric value.

- Integer-to-integer conversions fail when the value is outside the destination range.
- Signed-to-unsigned conversions fail for negative values and for values above the destination maximum.
- Unsigned-to-signed conversions fail for values above the destination maximum.
- Float-to-integer conversions fail for NaN, infinity, fractional values, and values outside the destination range.
- Integer-to-float and float-to-float conversions are included only where they can be checked meaningfully; they fail when the destination cannot represent the same numeric value.

### Not provided in this milestone

```text
int64_to_int32_checked
int64_to_int32_unchecked
int64_to_int32_saturating
int64_to_int32_wrapping
int64_to_int32_or
```

The `_checked` suffix is redundant when checked conversion is the only ordinary conversion. The unchecked/saturating/wrapping/fallback variants are deliberately excluded to keep the first public surface small and aligned with Opalescent's explicit error philosophy.

## Example Applications

```opal
import int64_to_int32 from standard.numeric
import terminal_session_move_cursor_sync from standard.terminal
import type TerminalSession, TerminalSessionStateError, TerminalSessionWriteError from standard.terminal

##
  Description: Move to the status row by value-preserving conversion for an int32 terminal API.
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
    let row = propagate int64_to_int32(visible_rows + 1)
    let first_column = propagate int64_to_int32(1)
    propagate terminal_session_move_cursor_sync(mutable ref session, row, first_column)
    return void
```

## Strengths

- General solution for fallible numeric narrowing.
- Makes range checking explicit and searchable.
- Avoids raw `as` when runtime overflow is possible.
- Good fit for FFI/platform boundaries.
- Keeps unsafe/lossy policies out of the ordinary conversion surface.
- Easy to test exhaustively for boundary values.

## Weaknesses

- Still adds ceremony and `propagate`.
- Adds errors to signatures.
- Does not fix API mismatches by itself.
- Many numeric type combinations create a large helper surface if added indiscriminately.
- Exact float conversions may reject values that a caller would accept with an explicit future rounding policy.

## Impact on Existing Syntax

No syntax changes.

The standard library gains numeric conversion APIs and an error such as `IntegerRangeError`, plus a named set such as `NumericConversionErrors`. Documentation should state when `as` is allowed and when conversion helpers are preferred.

## Interactions with Other Concerns

- **Cast rules:** conversion helpers are the explicit APIs referenced by the current cast goals for conversions that cannot be proven safe.
- **Terminal APIs:** terminal wrappers or applications can use these before fixed-width public APIs or backend calls.
- **Error annotations:** helpers introduce numeric range errors; named error sets may be useful.
- **Strict floats:** float conversion semantics must align with strict float policy for NaN/Inf.
- **LSP:** diagnostics for unsafe casts can suggest the corresponding `A_to_B` helper.

## Implementation Difficulty

Low to medium.

Main tasks:

1. Choose module and naming convention.
2. Add numeric range error type(s).
3. Implement core conversion helpers.
4. Register signatures in the type system.
5. Add runtime/codegen lowering.
6. Add diagnostics suggesting conversion helpers for unsafe casts.
7. Document excluded lossy policy variants separately if they are ever proposed.

## Must NOT Have

- Must not silently wrap, clamp, saturate, or truncate in plain conversion helpers.
- Must not expose unchecked narrowing as ordinary stdlib API.
- Must not use a generic `convert` name that hides source, destination, or behavior.
- Must not make out-of-range constants runtime errors when the compiler can diagnose them statically.
- Must not add `_saturating`, `_wrapping`, or `_or` variants in this milestone.
