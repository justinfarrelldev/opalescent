# Calendar-First

## Overview
This proposal centers `CalendarDateTime` as the primary type for time. Instead of working with raw timestamps or monotonic counters, developers interact directly with human-readable date and time components. This is ideal for applications where calendar logic is frequent and precision measurement is secondary.

## Assumes
- The system clock is synchronized via NTP for accuracy.
- `CalendarDateTime` is an immutable record.

## Syntax Design
```opal

let current_calendar_time_sync = f(): CalendarDateTime => ...
```

## Example Applications
```opal
let start = current_calendar_time_sync()
let s = format_iso8601(start)
let end = current_calendar_time_sync()
let diff = duration_difference(start, end)
```

## Strengths
- Highly ergonomic for common application logic (logging, UI, scheduling).
- Intuitive: mirrors how humans think about time.
- Direct access to fields without expensive conversion functions.

## Weaknesses
- Performance: calculating durations between calendar dates is more complex than simple integer subtraction.
- Correctness: vulnerable to system clock jumps and leap seconds when used for interval measurement.

## Impact on Existing Syntax
None. New library proposal.

## Interactions with Other Concerns
Consistent with error handling via `guard`. The `_sync` suffix correctly identifies blocking OS-clock reads.

## Implementation Difficulty
High. Requires robust calendar arithmetic (handling leap years, days in month, etc.) within the standard library.

## Must NOT Have
- Abbreviated names like `now`, `ts`, or `dt`.
- Opaque timestamp types as the primary interface.
