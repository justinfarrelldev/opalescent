# Target-Typed Integer Literals

## Overview

This alternative lets integer literals adopt an expected integer type from context when the literal value fits. It reduces casts such as `1 as int32` and `50 as int32` without allowing implicit narrowing of runtime expressions.

Example:

```opal
propagate sleep_ms_sync(50)
```

where `sleep_ms_sync` expects `int32`.

This helps numeric noise, but it only partially addresses terminal editor casts because `(visible_rows + 1)` is an `int64` runtime expression unless `visible_rows` itself is inferred as `int32`.

## Assumes

- Integer literals still default to `int64` when there is no expected type.
- Expected-type inference can flow into function arguments, array literals, constructors, and annotated bindings.
- Out-of-range literals remain compile-time errors.
- Runtime expressions are not implicitly narrowed.

## Syntax Design

No new syntax.

### Function arguments

```opal
propagate terminal_session_move_cursor_sync(mutable ref session, row, 1)
```

If the third parameter expects `int32`, literal `1` is typed as `int32`.

### Annotated bindings

Already-valid style remains:

```opal
let count: int32 = 10
```

This proposal extends the same expected-type behavior to more expression contexts.

### Not allowed

```opal
let row: int64 = visible_rows + 1
propagate terminal_session_move_cursor_sync(mutable ref session, row, 1)
```

If the terminal API expects `int32`, `row` is not implicitly narrowed. The caller must use a checked conversion or the API must accept `int64`.

## Example Applications

```opal
import sleep_ms_sync from standard
import terminal_session_move_cursor_sync from standard.terminal
import type TerminalSession, TerminalSessionStateError, TerminalSessionWriteError from standard.terminal

##
  Description: Use target-typed integer literals for int32 terminal and time arguments.
##
public let draw_at_fixed_position = f(
    session: mutable ref TerminalSession
): void errors
    InvalidDurationError,
    TerminalSessionWriteError,
    TerminalSessionStateError,
    InvalidCursorPositionError
=>
    propagate sleep_ms_sync(50)
    propagate terminal_session_move_cursor_sync(mutable ref session, 2, 3)
    return void
```

## Strengths

- Removes many low-value literal casts.
- Preserves explicit conversion for runtime values.
- Matches user expectations from typed languages with contextual literal typing.
- Improves examples and docs.
- No new keywords or operators.

## Weaknesses

- Does not solve dynamic `int64` to `int32` expressions.
- Adds inference complexity.
- Can make the type of a literal less obvious at a glance.
- Needs careful diagnostics for ambiguous generic contexts.
- If overextended beyond literals, it would violate numeric safety goals.

## Impact on Existing Syntax

No syntax change. Some previously required literal casts become optional.

The type checker must propagate expected integer types into literal expressions in more contexts. The formatter may remove redundant literal casts only if an explicit style decision is made; automatic removal is not required.

## Interactions with Other Concerns

- **Int64 terminal APIs:** if terminal APIs widen to `int64`, this proposal is less important for terminal code but still useful elsewhere.
- **Checked conversion functions:** runtime narrowing still uses checked helpers.
- **Generic functions:** ambiguous literals in generic contexts need clear inference rules.
- **Diagnostics:** out-of-range target-typed literals should point at the literal and expected type.
- **Cast rules:** explicit `as` remains available for clarity.

## Implementation Difficulty

Medium.

Main tasks:

1. Audit current literal inference rules.
2. Add expected-type propagation for function arguments and constructor fields.
3. Ensure out-of-range literals produce compile-time diagnostics.
4. Avoid implicit narrowing for non-literal expressions.
5. Add tests for generic ambiguity and overload-like standard calls.
6. Update docs to explain default vs contextual literal typing.

## Must NOT Have

- Must not implicitly narrow variables or runtime expressions.
- Must not allow out-of-range literals to compile.
- Must not change uncontextualized integer literals away from `int64`.
- Must not infer signed/unsigned conversions that hide intent in ambiguous contexts.
