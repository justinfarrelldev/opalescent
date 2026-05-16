# Blocking Sleep Milliseconds

## Overview

This proposal adds a simple blocking `sleep_ms_sync` function. It pauses the current thread for at least the requested duration.

## Assumes

- Opalescent remains synchronous by default.
- Millisecond precision is enough for terminal demos.
- Negative durations are rejected at compile time when constant or reported as typed errors at runtime otherwise.

## Error Types

- `InvalidDurationError`: emitted when the duration is negative or too large for the host timer.

## Proposed API

```opal
# sleep_ms_sync(milliseconds: int32): void errors InvalidDurationError
```

## Syntax Design

```opal
import sleep_ms_sync from standard

let run_generations = f(count: int32): void errors InvalidDurationError =>
    let mutable generation: int32 = 0
    while generation < count:
        print('generation {generation}')
        propagate sleep_ms_sync(100 as int32)
        generation = generation + (1 as int32)
    return void
```

## Strengths

1. Tiny API surface.
2. Easy to map to POSIX and Windows.
3. Good enough for tutorials and demos.
4. No scheduler or async model required.

## Weaknesses

1. Blocking sleep is not ideal for concurrent programs.
2. Actual sleep duration depends on OS timer resolution.
3. It does not compensate for render time.

## Fit

- **Game fit**: Good.
- **Implementation effort**: Low.
- **Long-term stdlib fit**: High.

## Must NOT Have

- No async/deferred semantics in the first version.
- No busy-wait loop implementation.
- No sub-millisecond promise.
- No ignored invalid-duration failures.
