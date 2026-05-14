# Interpolation Accumulator

## Overview

This proposal keeps string building as a pattern rather than a new API. Programs use a mutable `string` local and append through interpolation assignment.

This is already valid Opalescent syntax and works today for small boards.

## Assumes

- String interpolation remains the primary string composition feature.
- Mutable locals are allowed for local rendering buffers.
- The compiler/runtime can tolerate repeated allocations for small examples.

## Syntax Design

No new functions are required.

```opal
let render_row = f(row: int32[]): string =>
    let mutable output = ''
    let mutable column: int64 = 0
    while column < row.length:
        if row[column] is (1 as int32):
            output = '{output}#'
        else:
            output = '{output}.'
        column = column + 1
    return output
```

## Example Application

```opal
let render_board = f(board: int32[][]): string =>
    let mutable frame = ''
    let mutable row_index: int64 = 0
    while row_index < board.length:
        let row_text = render_row(board[row_index])
        frame = '{frame}{row_text}\n'
        row_index = row_index + 1
    return frame
```

## Strengths

1. Zero new API surface.
2. Easy to teach from existing examples.
3. Keeps strings immutable at the value level.
4. Good enough for small Game of Life demos.

## Weaknesses

1. Repeated interpolation may allocate a fresh string every append.
2. Performance is hard to predict for larger boards.
3. The pattern hides intent compared with a builder or join API.
4. It encourages tight loops over string concatenation.

## Fit

- **Game fit**: Good for small boards.
- **Implementation effort**: None to Low.
- **Long-term stdlib fit**: Medium.

## Must NOT Have

- No new concatenation operator.
- No mutation of string contents by index.
- No compiler magic that silently changes string value semantics.
