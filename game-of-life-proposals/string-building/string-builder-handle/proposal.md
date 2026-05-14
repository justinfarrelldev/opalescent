# String Builder Handle

## Overview

This proposal adds a mutable builder handle for repeated text assembly. The handle is explicit, local, and finished into an immutable `string`.

Prior art includes Java `StringBuilder`, C# `StringBuilder`, Rust `String`, Swift `String` append APIs, and Go `strings.Builder`. For Opalescent, the important part is that the handle makes allocation and mutation visible without adding new syntax.

## Assumes

- `StringBuilder` is an opaque standard-library type.
- Builder functions are language-facing builtins or normal standard functions.
- Finishing a builder returns a new immutable `string`.

## Proposed API

```opal
# string_builder_new(): StringBuilder
# string_builder_push(builder: StringBuilder, text: string): void
# string_builder_finish(builder: StringBuilder): string
```

## Syntax Design

```opal
import string_builder_new, string_builder_push, string_builder_finish from standard

let render_row = f(row: int32[]): string =>
    let builder = string_builder_new()
    let mutable column: int64 = 0
    while column < row.length:
        if row[column] is (1 as int32):
            string_builder_push(builder, '#')
        else:
            string_builder_push(builder, '.')
        column = column + 1
    return string_builder_finish(builder)
```

## Example Application

```opal
import string_builder_new, string_builder_push, string_builder_finish from standard

let render_board = f(board: int32[][]): string =>
    let builder = string_builder_new()
    let mutable row_index: int64 = 0
    while row_index < board.length:
        string_builder_push(builder, render_row(board[row_index]))
        string_builder_push(builder, '\n')
        row_index = row_index + 1
    return string_builder_finish(builder)
```

## Strengths

1. Best performance story for repeated frame rendering.
2. Makes mutation explicit and isolated.
3. Useful for logs, serializers, template output, and CLIs.
4. Avoids inventing a string concatenation operator.

## Weaknesses

1. Requires an opaque runtime object or handle design.
2. Needs ownership/lifetime policy for builder handles.
3. More ceremony than interpolation for tiny programs.
4. Error behavior for failed allocation must be specified.

## Fit

- **Game fit**: Excellent.
- **Implementation effort**: Medium.
- **Long-term stdlib fit**: Excellent.

## Must NOT Have

- No implicit global builder.
- No builder escaping rules that depend on hidden compiler state.
- No syntax like `builder += text`; use ordinary function calls.
