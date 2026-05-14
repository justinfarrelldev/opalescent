# ANSI Control Functions

## Overview

This proposal adds small terminal helpers that map to ANSI escape sequences where supported. They provide the fastest path to an animated terminal Game of Life.

## Assumes

- Functions write to stdout or the active terminal output stream.
- Unsupported terminals either return a typed error or degrade safely.
- Coordinates are 1-based, matching common ANSI cursor addressing.

## Proposed API

```opal
# terminal_clear_screen(): void
# terminal_move_cursor(row: int32, column: int32): void
# terminal_hide_cursor(): void
# terminal_show_cursor(): void
```

## Syntax Design

```opal
import terminal_clear_screen, terminal_move_cursor, terminal_hide_cursor, terminal_show_cursor from standard

let draw_at_top = f(frame: string): void =>
    terminal_hide_cursor()
    terminal_move_cursor(1 as int32, 1 as int32)
    terminal_clear_screen()
    print(frame)
    terminal_show_cursor()
    return void
```

## Strengths

1. Small and easy to teach.
2. Excellent for simple terminal animation.
3. No new language syntax.
4. Maps directly to ANSI-capable terminals.

## Weaknesses

1. Raw ANSI assumptions can be wrong on some Windows or redirected outputs.
2. No capability discovery.
3. No flicker minimization beyond caller discipline.

## Fit

- **Game fit**: Excellent for demos.
- **Implementation effort**: Low.
- **Long-term stdlib fit**: Medium.

## Must NOT Have

- No hard-coded POSIX-only behavior.
- No failure to restore the cursor after runtime errors when avoidable.
- No color API until basic control is stable.
