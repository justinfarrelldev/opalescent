# Game of Life Full

This is a full Conway's Game of Life project written in Opalescent. It uses an
80 column by 40 row board, updates at 15 frames per second, and runs until the
process is stopped externally with Ctrl+C or a command such as `timeout`.

## Project Layout

- `src/main.op` owns the entry point, creates the configuration value, starts the
  frame clock, and runs the infinite animation loop.
- `src/life.types.op` defines `LifeConfig`, the project-level configuration type
  used by the entry module.
- `src/config.op` contains the fixed 80x40 board dimensions, the 15fps target,
  the flat-board cell count helper, and the `int8` cell constants.
- `src/board.op` contains row-major indexing and safe cell reads.
- `src/patterns.op` creates the deterministic seed board from recognizable
  Conway patterns.
- `src/rules.op` implements live-neighbor counting and next-generation rules.
- `src/render.op` clears ANSI terminals for each generation, streams each frame
  to stdout, and flushes once per frame.

## Flat Board Storage

The board is a single `int8[]`, not a nested array. Coordinates are converted to
an index with this formula:

```text
index = y * width + x
```

The value `0 as int8` means dead and `1 as int8` means alive. For an 80x40
board, each generation stores 3200 cells. The simulator keeps the current board
and builds the next board as a fresh flat array, so the live board storage is
small: roughly 6400 bytes of cell data plus Opalescent array and RC headers.

## Types File

`life.types.op` is a real `.types.op` file and can only contain type declarations
and imports. The entry module imports `LifeConfig` and builds the app's fixed
configuration value.

The helper modules use primitive `width`, `height`, and `frames_per_second`
parameters instead of accepting `LifeConfig` directly. This matches the compiler
behavior at the time this project was written: top-level `let ... = f(...)`
function signatures in value modules are checked through the built-in type mapper
before imported nominal types are available.

## Error Flow

Terminal and timing operations are fallible. This project follows Opalescent's
error style by propagating those errors to `entry main` instead of handling them
inside lower-level modules. The app itself does not use custom exit codes.

## Terminal Rendering

Opalescent can clear the terminal through `terminal_clear_screen_on_sync` when
stdout supports ANSI control sequences. The standard clear operation clears the
visible screen, clears terminal scrollback, and returns the cursor home. In an
interactive terminal this project clears before writing each generation, so the
board animates in place. When stdout is redirected, ANSI support is reported as
unavailable and frames are written as a readable transcript instead.

## Setup

You need the `opal` binary on your PATH. See [CONTRIBUTING.md](../../CONTRIBUTING.md) for full setup instructions.

## Run

From this project directory:

```bash
opal run
```

## Preview Safely

The program is intentionally infinite. Use an external timeout when previewing it
from scripts or automated checks:

```bash
opal build
timeout 3s ./target/program
```

For a captured transcript-style preview:

```bash
timeout 3s ./target/program > /tmp/game-of-life-full-preview.txt
```

The `timeout` wrapper normally exits with code `124` after stopping the process.
That is expected during previews and is not an application exit code.
