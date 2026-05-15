# Print and Output Comparison

## Overview

Current `print` is line-oriented and works well for diagnostics, tests, and simple programs. A polished Game of Life wants more control: write a cell without a newline, flush after a frame, and optionally route output through a writer handle.

## Summary Matrix

| Proposal | Ergonomics | Implementation Effort | Terminal Animation Fit | General CLI Fit |
| --- | --- | --- | --- | --- |
| [Print Text and Flush](print-text-and-flush/) | High | Low | Excellent | Excellent |
| [Line Oriented Output](line-oriented-output/) | Very High | Low | Medium | Excellent |
| [Text Writer Sink](text-writer-sink/) | Medium | Medium | Excellent | Excellent |

## Recommendation

Add `print_text` and `flush_standard_output_sync` first. Keep current `print` as newline-oriented because it is heavily used by fixtures. Add writer sinks after terminal capabilities and file output share a common design.

## Existing Syntax Anchor

Current printing is simple and well tested:

```opal
print(42)
print(true)
print(false)
print('hello')
```
