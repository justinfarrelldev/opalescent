# Deadline Timer

## Overview

This proposal exposes one-shot deadlines. A caller creates a deadline some number of milliseconds in the future, does work, then waits for the remaining time if any.

This is lower-level than `FrameClock` but more flexible for simulations that separate update and draw phases.

## Assumes

- `Deadline` is an opaque standard-library handle or timestamp value.
- Deadlines use monotonic time.
- Waiting an already-expired deadline returns immediately.

## Proposed API

```opal
# deadline_after_ms(milliseconds: int32): Deadline
# deadline_wait(deadline: Deadline): void
```

## Syntax Design

```opal
import deadline_after_ms, deadline_wait from standard

let draw_with_budget = f(board: int32[][]): void =>
    let next_frame = deadline_after_ms(100 as int32)
    print(render_board(board))
    deadline_wait(next_frame)
    return void
```

## Strengths

1. Flexible primitive for frame loops.
2. Handles render-time compensation naturally.
3. Good bridge toward future time APIs.

## Weaknesses

1. Less beginner-friendly than `sleep_ms`.
2. Requires a monotonic timestamp representation.
3. More opportunities for misuse than a frame clock.

## Fit

- **Game fit**: Good.
- **Implementation effort**: Medium.
- **Long-term stdlib fit**: Medium to High.

## Must NOT Have

- No date/time calendar semantics.
- No wall-clock deadlines for animation.
- No automatic retry or callback behavior.
