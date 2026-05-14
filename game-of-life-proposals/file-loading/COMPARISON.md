# File Loading Comparison

## Overview

Game of Life examples benefit from loading seed patterns instead of hard-coding boards. Opalescent already has path-safe filesystem functions, line reads, whole-file reads, and typed `guard` handling. The remaining design choice is the pattern format and parser surface.

## Summary Matrix

| Proposal | Ergonomics | Diagnostics | Implementation Effort | Pattern Ecosystem Fit |
| --- | --- | --- | --- | --- |
| [Whole File Seed Loader](whole-file-seed-loader/) | High | Medium | Low | Medium |
| [Line Based Seed Loader](line-based-seed-loader/) | High | High | Low-Medium | Medium |
| [RLE Pattern Loader](rle-pattern-loader/) | Medium | High | Medium | Excellent |

## Recommendation

Start with line-based plain text loading: each row is a line, `#` or `O` means live, `.` means dead. It maps directly to current `read_lines_sync` support and gives clean row/column errors. Add RLE after the board representation and parser error style are stable.

## Existing Syntax Anchor

Current file loading already uses this shape:

```opal
guard read_lines_sync(path_from('fixtures/sample.txt')) into lines else err =>
    print(err)
    propagate err
```
