# Game of Life Support Proposals - Comparison & Recommendations

## Overview

These proposals cover the small standard-library and runtime additions that would make Conway's Game of Life pleasant to write in Opalescent. The current language already has enough core syntax for a terminal version: arrays, mutable locals, loops, guards, file reads, interpolation, and `print` are all exercised in `test-projects/` fixtures. The missing pieces are mostly ergonomic APIs around text building, output control, timing, terminal drawing, and pattern loading.

The proposals are intentionally standard-library shaped. They do not require new grammar. All examples use existing Opalescent syntax: imports at the top, colon blocks, `guard ... else err =>`, `propagate`, mutable locals, arrays, `.push`, `.length`, and explicit `return void`. Side-effecting host operations are fallible where the host can reject, interrupt, or fail them; the examples must not ignore those failures.

## Quick Comparison

| Area | Best Immediate Option | Richer Option | Main Risk | Priority |
| --- | --- | --- | --- | --- |
| [String Building](string-building/) | `string_join` and interpolation accumulator | Mutable `StringBuilder` handle | Hidden allocation behavior | High |
| [Print](print/) | `print_text` plus `flush_standard_output_sync` | Text writer sink | Cross-platform buffering details | High |
| [Sleep](sleep/) | Blocking `sleep_ms_sync` | Frame clock handle | Timer precision expectations | Medium |
| [Terminal](terminal/) | ANSI control functions | Screen buffer renderer | Windows console capabilities | High |
| [File Loading](file-loading/) | Whole-file plain text loader | Line or RLE loaders | Error taxonomy and format drift | Medium |

## Research Notes

The recommendations draw from three sources:

1. Existing Opalescent fixtures: `array-push`, `array-double`, `fs-markdown-roundtrip`, `_fs_read_text_lines`, `print-types`, and `string-interp-long` show the valid syntax and current runtime surface.
2. Established terminal practice: C stdio, POSIX and Windows terminals, ANSI escape control, ncurses-style screen buffering, and frame pacing in simple terminal games.
3. Game of Life formats: plain text grids are easiest for examples, line-based loaders give better diagnostics, and RLE is the common interchange format for Life patterns.

## Recommendation Tiers

### Tier 1: Minimal Useful Game of Life

- Add `print_text(value: string): void errors WriteFailureError, SinkClosedError` and `flush_standard_output_sync(): void errors FlushFailureError, SinkClosedError`.
- Add `sleep_ms_sync(milliseconds: int32): void errors InvalidDurationError`.
- Add `terminal_clear_screen_sync(): void errors UnsupportedTerminalError, OutputNotTerminalError, ControlWriteFailureError` and `terminal_move_cursor_sync(row: int32, column: int32): void errors UnsupportedTerminalError, OutputNotTerminalError, InvalidCursorPositionError, ControlWriteFailureError`.
- Add `string_join(lines: string[], separator: string): string`.

This gets a pleasant animated terminal Life with very little compiler work.

### Tier 2: Comfortable Text UI

- Add a `StringBuilder` handle or equivalent mutable text buffer.
- Add a `FrameClock` helper for stable animation cadence.
- Add a capability-aware terminal handle so Windows and redirected output are handled explicitly.
- Keep output, terminal, and timing failures typed so programs must handle or propagate them.

This makes larger boards and repeated rendering much cleaner.

### Tier 3: Pattern Ecosystem

- Add a plain text seed loader.
- Add a line-based loader with row/column diagnostics.
- Add RLE parsing for compatibility with common Life pattern libraries.

This matters once examples become reusable programs instead of demo fixtures.

## Directory Structure

```text
game-of-life-proposals/
|-- COMPARISON.md
|-- string-building/
|   |-- COMPARISON.md
|   |-- interpolation-accumulator/
|   |   `-- proposal.md
|   |-- string-builder-handle/
|   |   `-- proposal.md
|   `-- string-join-lines/
|       `-- proposal.md
|-- print/
|   |-- COMPARISON.md
|   |-- print-text-and-flush/
|   |   `-- proposal.md
|   |-- line-oriented-output/
|   |   `-- proposal.md
|   `-- text-writer-sink/
|       `-- proposal.md
|-- sleep/
|   |-- COMPARISON.md
|   |-- blocking-sleep-ms/
|   |   `-- proposal.md
|   |-- frame-clock/
|   |   `-- proposal.md
|   `-- deadline-timer/
|       `-- proposal.md
|-- terminal/
|   |-- COMPARISON.md
|   |-- ansi-control-functions/
|   |   `-- proposal.md
|   |-- capability-aware-console/
|   |   `-- proposal.md
|   `-- screen-buffer-renderer/
|       `-- proposal.md
`-- file-loading/
    |-- COMPARISON.md
    |-- whole-file-seed-loader/
    |   `-- proposal.md
    |-- line-based-seed-loader/
    |   `-- proposal.md
    `-- rle-pattern-loader/
        `-- proposal.md
```
