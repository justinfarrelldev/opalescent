# Frame Clock

## Overview

This proposal adds a frame pacing handle. A `FrameClock` waits until the next frame boundary, accounting for time already spent rendering.

Prior art includes game-loop fixed timestep clocks, SDL frame delay helpers, and browser animation frame scheduling. For Opalescent, the surface should stay synchronous and explicit.

## Assumes

- `FrameClock` is an opaque standard-library handle.
- The clock stores target frame duration and next deadline.
- `frame_clock_wait_next_sync` blocks until the next frame slot.

## Proposed API

```opal
# frame_clock_new(frames_per_second: int32): FrameClock
# frame_clock_wait_next_sync(clock: FrameClock): void
```

## Syntax Design

```opal
import frame_clock_new, frame_clock_wait_next_sync from standard

let animate = f(board: int32[][]): void =>
    let clock = frame_clock_new(10 as int32)
    let mutable generation: int32 = 0
    while generation < (100 as int32):
        print('generation {generation}')
        frame_clock_wait_next_sync(clock)
        generation = generation + (1 as int32)
    return void
```

## Strengths

1. Better animation cadence than raw sleep.
2. Avoids each caller reimplementing elapsed-time math.
3. Still synchronous and easy to explain.
4. Useful for terminal games, simulations, and demos.

## Weaknesses

1. Requires monotonic clock support.
2. Needs clear behavior when rendering overruns the frame budget.
3. More API than `sleep_ms_sync`.

## Fit

- **Game fit**: Excellent.
- **Implementation effort**: Medium.
- **Long-term stdlib fit**: High.

## Must NOT Have

- No wall-clock time for frame pacing.
- No hidden global frame rate.
- No skipped update policy in the first version; it only waits.
