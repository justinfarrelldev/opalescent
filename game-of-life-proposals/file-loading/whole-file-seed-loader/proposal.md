# Whole File Seed Loader

## Overview

This proposal loads an entire pattern file into a string and parses it in one pass. It is the smallest API for demos and matches the existing `read_text_sync` runtime surface.

## Assumes

- Existing `read_text_sync` remains the filesystem primitive.
- `parse_life_text` is a pure parser added to the standard library or a sample library.
- Plain text rows use `#` for live and `.` for dead cells.

## Proposed API

```opal
# parse_life_text(text: string): int32[][] errors PatternParseError
```

## Syntax Design

```opal
import path_from, read_text_sync from standard
import parse_life_text from standard

##
  Description: Loads a Life seed file and prints the parsed board height.
##
entry main = f(args: string[]): void errors FileNotFoundError, PermissionDeniedError, ReadFailureError, IsADirectoryError, InvalidPathError, InvalidUtf8Error, PatternParseError =>
    let path = path_from('patterns/glider.life')

    guard read_text_sync(path) into text else err =>
        print(err)
        propagate err

    guard parse_life_text(text) into board else err =>
        print(err)
        propagate err

    print('height {board.length}')
    return void
```

## Strengths

1. Simple implementation path.
2. Uses existing whole-file read support.
3. Easy to document for tutorials.
4. Good for small Life patterns.

## Weaknesses

1. Error spans require parser-owned row/column tracking.
2. Reads the whole file even for large patterns.
3. Format detection is harder when everything is one string.

## Fit

- **Game fit**: Good.
- **Implementation effort**: Low.
- **Long-term stdlib fit**: Medium.

## Must NOT Have

- No implicit filesystem path strings; use `path_from`.
- No silent treatment of unknown characters.
- No platform-specific line-ending assumptions.
