# Single Timestamp Type

## Overview
This proposal unifies monotonic and wall-clock time into a single `Timestamp` type representing nanoseconds since the Unix epoch. This simplifies the API surface and makes it easier for developers to pass time values around without worrying about type mismatches.

## Assumes
- `int64` is sufficient for representing nanoseconds since epoch (covering ~292 years).
- The underlying OS provides a unified or convertible clock source.

## Syntax Design
```opal

let current_timestamp_sync = f(): Timestamp => ...
```

## Example Applications
```opal
let start = current_timestamp_sync()
let s = format_timestamp_as_iso8601(start)
let end = current_timestamp_sync()
let duration = duration_between(start, end)
```

## Strengths
- Simplicity: only one type to learn and use.
- Interoperability: easy to store and transmit as a single integer.
- Minimal boilerplate: fewer conversion functions required.

## Weaknesses
- Fragility: wall-clock adjustments (NTP) can affect interval measurements.
- Precision: doesn't explicitly handle leap seconds or different calendar systems at the core.

## Impact on Existing Syntax
None. New library proposal.

## Interactions with Other Concerns
Consistent with error handling via `guard`. The `_sync` suffix clearly marks side-effecting clock reads.

## Implementation Difficulty
Low. Wraps standard system clock functions.

## Must NOT Have
- Abbreviated names like `now`, `ts`, or `dt`.
- Separate types for monotonic and wall-clock time.
