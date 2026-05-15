# Text Writer Sink

## Overview

This proposal introduces explicit text writer handles. A terminal, file, in-memory buffer, or test capture sink could all implement the same output shape.

Prior art includes Rust `Write`, Go `io.Writer`, Java `Writer`, and dependency-injected output handlers used by the current Rust stdlib I/O layer.

## Assumes

- `TextWriter` is an opaque standard-library handle.
- The default stdout writer is available through `stdout_writer()`.
- Writer functions are ordinary imported functions, not methods.

## Proposed API

```opal
# stdout_writer(): TextWriter
# writer_write_sync(writer: TextWriter, value: string): void
# writer_flush_sync(writer: TextWriter): void
```

## Syntax Design

```opal
import stdout_writer, writer_write_sync, writer_flush_sync from standard

let draw_frame = f(board: int32[][]): void =>
    let writer = stdout_writer()
    let frame = render_board(board)
    writer_write_sync(writer, frame)
    writer_flush_sync(writer)
    return void
```

## Strengths

1. Clean dependency-injection story for tests.
2. Generalizes terminal, file, and memory output.
3. Can support stderr later without new grammar.
4. Fits larger applications better than global output functions.

## Weaknesses

1. More ceremony for small programs.
2. Requires handle lifetime and ownership policy.
3. Needs good naming if Opalescent avoids interface-style abstractions for now.

## Fit

- **Game fit**: Excellent for polished tools.
- **Implementation effort**: Medium.
- **Long-term stdlib fit**: Excellent.

## Must NOT Have

- No implicit global mutable writer hidden behind the handle.
- No object method syntax until method-style stdlib APIs are settled.
- No async writer surface in the first synchronous version.
