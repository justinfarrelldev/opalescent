# Line Based Seed Loader

## Overview

This proposal loads a seed pattern as `string[]` with `read_lines_sync`, then parses rows. It is the best fit for Opalescent today because arrays, lengths, and line-based file reads are already exercised by fixtures.

## Assumes

- `read_lines_sync` returns lines without line endings.
- `parse_life_lines` validates rectangular shape unless a jagged mode is added later.
- Parser errors include row and column information.

## Proposed API

```opal
# parse_life_lines(lines: string[]): int32[][] errors PatternParseError
```

## Error Types

- `PatternParseError`: emitted when Life pattern rows are malformed; diagnostics should include row and column information when available.

## Syntax Design

```opal
import path_from, read_lines_sync from standard
import parse_life_lines from standard

##
  Description: Loads a line-based Life seed and prints the parsed board height.
##
entry main = f(args: string[]): void errors FileNotFoundError, PermissionDeniedError, ReadFailureError, IsADirectoryError, InvalidPathError, InvalidUtf8Error, PatternParseError =>
    guard read_lines_sync(path_from('patterns/glider.life')) into lines else err =>
        print(err)
        propagate err

    guard parse_life_lines(lines) into board else err =>
        print(err)
        propagate err

    print('height {board.length}')
    return void
```

## Example Application

```opal
import path_from, read_lines_sync from standard
import parse_life_lines from standard

let print_default_board_height = f(): void errors FileNotFoundError, PermissionDeniedError, ReadFailureError, IsADirectoryError, InvalidPathError, InvalidUtf8Error, PatternParseError =>
    guard read_lines_sync(path_from('patterns/glider.life')) into lines else err =>
        print(err)
        propagate err

    guard parse_life_lines(lines) into board else err =>
        print(err)
        propagate err

    print('height {board.length}')
    return void
```

## Strengths

1. Best diagnostics for row/column parse errors.
2. Reuses existing line read APIs.
3. Easy to test with fixtures.
4. Natural fit for rectangular board validation.

## Weaknesses

1. Slightly more API than whole-file parsing.
2. Still needs a pattern parser and error type.
3. Not compatible with common RLE files by itself.

## Fit

- **Game fit**: Excellent.
- **Implementation effort**: Low-Medium.
- **Long-term stdlib fit**: High.

## Must NOT Have

- No automatic padding of short rows in strict mode.
- No ignored trailing garbage.
- No reliance on absolute paths in examples.
