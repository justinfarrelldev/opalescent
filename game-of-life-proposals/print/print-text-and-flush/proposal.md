# Print Text and Flush

## Overview

This proposal adds two small output functions: one writes text without appending a newline, and one flushes the output stream. This is the smallest change that makes terminal animation practical.

Prior art includes C `fputs` and `fflush`, Rust `write!` plus `flush`, Python `print(..., end='')`, and Node `process.stdout.write`.

## Assumes

- Existing `print` remains newline-oriented.
- `print_text` accepts only `string`; other values can use interpolation.
- `flush_output` flushes stdout or the active process output sink.

## Proposed API

```opal
# print_text(value: string): void
# flush_output(): void
```

## Syntax Design

```opal
import print_text, flush_output from standard

let print_row = f(row: int32[]): void =>
    let mutable column: int64 = 0
    while column < row.length:
        if row[column] is (1 as int32):
            print_text('#')
        else:
            print_text('.')
        column = column + 1
    print_text('\n')
    flush_output()
    return void
```

## Strengths

1. Minimal API surface.
2. Excellent fit for terminal games.
3. Keeps formatted values explicit through interpolation.
4. Maps cleanly to C stdio and Windows console output.

## Weaknesses

1. Still low-level for full frame rendering.
2. Flush behavior can be platform dependent unless specified carefully.
3. Does not handle stderr or custom output sinks.

## Fit

- **Game fit**: Excellent.
- **Implementation effort**: Low.
- **Long-term stdlib fit**: Excellent.

## Must NOT Have

- No change to existing `print` behavior.
- No implicit flush after every `print_text` call.
- No overloaded formatting mini-language in the first version.
