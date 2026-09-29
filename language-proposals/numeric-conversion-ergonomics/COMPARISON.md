# Numeric Conversion Ergonomics — Comparison

## Status and scope

Accepted stdlib direction with alternatives recorded. This concern responds to frequent casts in terminal/editor code such as:

```opal
(visible_rows + 1) as int32
```

Opalescent should keep explicit numeric conversion as a safety principle. The goal is not to add implicit narrowing. The selected general-purpose solution is explicit checked conversion helpers; API widening or domain-specific coordinate types remain separate design choices for specific modules.

## Is this addressable?

Partly. Cast noise is addressable when it is caused by mismatched API choices, especially terminal functions accepting `int32` while normal counters and integer literals are `int64`. It is not addressable by making arbitrary runtime narrowing implicit without weakening Opalescent's safety goals.

## Reading order

1. This comparison.
2. [`checked-conversion-functions`](./checked-conversion-functions/proposal.md) — selected general-purpose explicit checked conversion helpers.
3. [`int64-terminal-apis`](./int64-terminal-apis/proposal.md) — alternative for terminal/editor coordinate noise: widen public terminal coordinate APIs to `int64`.
4. [`terminal-coordinate-types`](./terminal-coordinate-types/proposal.md) — domain-specific row/column wrapper types.
5. [`target-typed-integer-literals`](./target-typed-integer-literals/proposal.md) — reduce literal casts, not runtime narrowing.

## Problem shape

The editor naturally uses `int64` because:

- integer literals default to `int64`;
- array lengths are `int64`;
- string lengths are `int64`;
- cursor and viewport positions are scalar indexes and line indexes.

Terminal APIs currently accept `int32` rows and columns. That forces casts at call boundaries even when the values are already validated by editor logic.

## Comparison matrix

| Axis | Int64 terminal APIs | Checked conversion functions — selected | Terminal coordinate types | Target-typed integer literals |
|---|---:|---:|---:|---:|
| **Removes editor casts** | ★★★★★ | ★★☆☆☆ | ★★★★☆ | ★★☆☆☆ |
| **Preserves numeric safety** | ★★★★★ | ★★★★★ | ★★★★★ | ★★★★☆ |
| **General usefulness** | ★★☆☆☆ | ★★★★★ | ★★★☆☆ | ★★★☆☆ |
| **Implementation effort** | Medium | Low to medium | Medium to high | Medium |
| **API churn** | Medium | Low | High | Low |
| **Domain readability** | ★★★★☆ | ★★★☆☆ | ★★★★★ | ★★★☆☆ |
| **Risk of hiding bugs** | Low | Low | Low | Medium if overextended |

## Alternatives summary

### Int64 terminal APIs

Change public terminal coordinate APIs to accept `int64` rows/columns while preserving runtime validation. The runtime can downcast internally after checking host/backend limits.

```opal
propagate terminal_session_move_cursor_sync(mutable ref session, visible_rows + 1, 1)
```

This directly addresses the editor's actual cast noise because it removes the mismatch at the API boundary, but it is terminal-specific rather than a general numeric conversion mechanism.

### Checked conversion functions

Add explicit value-preserving helpers such as:

```opal
int64_to_int32(value: int64): int32 errors IntegerRangeError
```

The plain `A_to_B` helper is checked; there is no unchecked ordinary variant and no `_checked` suffix. This is the right general-purpose answer when narrowing is truly required. It does not remove ceremony, but it turns casts into checked, reusable operations.

### Terminal coordinate types

Introduce types such as `TerminalRow` and `TerminalColumn` with constructors from `int64`:

```opal
let row = propagate terminal_row_from_one_based(visible_rows + 1)
let column = terminal_column_first()
propagate terminal_session_move_cursor_sync(mutable ref session, row, column)
```

This makes domain invariants explicit but increases API surface and type verbosity.

### Target-typed integer literals

Allow integer literals to adopt an expected type more aggressively:

```opal
propagate sleep_ms_sync(50)
```

instead of:

```opal
propagate sleep_ms_sync(50 as int32)
```

This helps literal-only casts but cannot safely convert `(visible_rows + 1)` from `int64` to `int32` unless the expression is proven in range.

## Recommendation

Select **checked conversion functions** as the primary general-purpose numeric safety surface. Plain `A_to_B` helpers are checked and fallible through `IntegerRangeError`; arbitrary runtime narrowing remains explicit.

For terminal/editor APIs specifically, keep **int64 terminal APIs** and **terminal coordinate types** as separate API-design alternatives. They may still be appropriate if the terminal surface grows enough domain-specific invariants or if public coordinate types are revisited, but they are not the general numeric conversion mechanism.

Implement **target-typed integer literals** narrowly, if at all. It is useful for literal casts but should not become implicit runtime narrowing.

## Additional fixture ladder for terminal API alternatives

1. Call terminal cursor functions with `int64` literals.
2. Call terminal cursor functions with `int64` expressions derived from array/string lengths.
3. Verify negative row/column values still produce `InvalidCursorPositionError`.
4. Verify overlarge values produce a range/cursor error before runtime downcast.
5. Verify old `int32` callers still work through widening or migration compatibility.
6. Add checked conversion tests for in-range and out-of-range conversions.
7. Verify no implicit runtime narrowing is accepted outside selected APIs.
8. Remove noisy casts from terminal editor fixtures.

## Must not regress

- Arbitrary runtime narrowing must not become implicit.
- Constants that are out of range for their target type must remain compile-time errors.
- Terminal APIs must still reject invalid positions.
- FFI-facing functions that truly require fixed-width integers must say so.
- Checked conversion helpers must not silently wrap, saturate, clamp, or expose unchecked ordinary variants.
