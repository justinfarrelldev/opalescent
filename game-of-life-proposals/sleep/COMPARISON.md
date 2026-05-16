# Sleep and Timing Comparison

## Overview

Animation needs a timing primitive. The simplest option is a blocking sleep function. More polished games need frame pacing that accounts for render time and timer precision. Invalid durations, invalid frame rates, and host timer failures should be flat typed errors.

## Summary Matrix

| Proposal | Ergonomics | Timing Quality | Implementation Effort | Game Fit |
| --- | --- | --- | --- | --- |
| [Blocking Sleep Milliseconds](blocking-sleep-ms/) | Very High | Medium | Low | Good |
| [Frame Clock](frame-clock/) | High | High | Medium | Excellent |
| [Deadline Timer](deadline-timer/) | Medium | High | Medium | Good |

## Recommendation

Start with `sleep_ms_sync(milliseconds: int32): void errors InvalidDurationError`. Add `FrameClock` later for smooth animation and deterministic examples; its constructor should reject invalid frame rates and its wait operation should report `TimerUnavailableError` or `TimerWaitFailureError`.

## Research Notes

Most beginner game loops use blocking sleep first: C `Sleep`/`usleep`, Rust `thread::sleep`, Go `time.Sleep`, Python `time.sleep`. Frame clocks are common in game engines because render time must be subtracted from the next delay.
