# Monotonic and Wall Clock Split

## Overview
This proposal introduces two distinct types for handling time: `MonotonicInstant` for measuring intervals and `WallClockDateTime` for calendar-based time. By separating these concerns, we prevent common bugs where system clock adjustments interfere with duration measurements.

## Assumes
- Statically typed records.
- Standard error handling via `guard`.
- OS-level access to monotonic and UTC clocks.

## Syntax Design
```opal


let now_monotonic_sync = f(): MonotonicInstant => ...
let now_wall_clock_sync = f(): WallClockDateTime => ...
```

## Example Applications
```opal
let start = now_monotonic_sync()
sleep_sync(duration)
let end = now_monotonic_sync()
let diff = elapsed_time(start, end)
```

## Strengths
- Type safety: cannot accidentally use wall clock for intervals.
- Resilience: monotonic time is unaffected by NTP jumps or leap seconds.
- Clarity: explicit `_sync` suffix indicates OS interaction.

## Weaknesses
- Increased complexity: developers must choose between two types.
- Conversion overhead: converting between monotonic and wall clock is non-trivial and often discouraged.

## Impact on Existing Syntax
None. This is a new library proposal.

## Interactions with Other Concerns
Composes well with the error model by using `guard` for parsing. `_sync` suffix prepares for future deferred variants.

## Implementation Difficulty
Medium. Requires wrapping platform-specific monotonic and wall clock APIs.

## Must NOT Have
- Abbreviated names like `now`, `ts`, or `dt`.
- Implicit conversions between clock types.
