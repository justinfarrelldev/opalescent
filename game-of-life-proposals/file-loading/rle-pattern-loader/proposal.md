# RLE Pattern Loader

## Overview

This proposal adds a Run Length Encoded Life pattern parser. RLE is the common interchange format for Conway's Game of Life patterns, so supporting it makes Opalescent examples interoperable with existing pattern collections.

## Assumes

- RLE parsing is synchronous and pure after file text is loaded.
- Metadata lines can be ignored initially or returned later through a richer type.
- The first version returns only `int32[][]`.

## Proposed API

```opal
# parse_life_rle(text: string): int32[][] errors RlePatternError
```

## Syntax Design

```opal
import path_from, read_text_sync from standard
import parse_life_rle from standard

##
  Description: Loads an RLE Life seed and prints the parsed board height.
##
entry main = f(args: string[]): void errors FileNotFoundError, PermissionDeniedError, ReadFailureError, IsADirectoryError, InvalidPathError, InvalidUtf8Error, RlePatternError =>
    guard read_text_sync(path_from('patterns/glider.rle')) into text else err =>
        print(err)
        propagate err

    guard parse_life_rle(text) into board else err =>
        print(err)
        propagate err

    print('height {board.length}')
    return void
```

## Example Application

```opal
import path_from, read_text_sync from standard
import parse_life_rle from standard

let print_glider_height = f(): void errors FileNotFoundError, PermissionDeniedError, ReadFailureError, IsADirectoryError, InvalidPathError, InvalidUtf8Error, RlePatternError =>
    guard read_text_sync(path_from('patterns/glider.rle')) into text else err =>
        print(err)
        propagate err

    guard parse_life_rle(text) into board else err =>
        print(err)
        propagate err

    print('height {board.length}')
    return void
```

## Strengths

1. Best compatibility with the Life ecosystem.
2. Compact files for large patterns.
3. Good demonstration of Opalescent typed parsing errors.
4. Can later expose metadata such as name, author, and rules.

## Weaknesses

1. More parser work than plain text rows.
2. Requires careful diagnostics for run counts and malformed headers.
3. RLE supports rule metadata that may outgrow `int32[][]` returns.

## Fit

- **Game fit**: Excellent once basics are stable.
- **Implementation effort**: Medium.
- **Long-term stdlib fit**: Medium, or high if a games/examples package exists.

## Must NOT Have

- No silent fallback from invalid RLE to plain text.
- No lossy metadata handling once metadata is exposed.
- No assumption that all RLE files use the default Life rule forever.
