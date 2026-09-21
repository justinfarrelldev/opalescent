# Unicode Grapheme Cell Width

## Overview
This proposal adds terminal display-cell measurement and clipping based on Unicode grapheme clusters and wcwidth-like terminal width rules. It is the long-term layout surface needed for a Unicode-correct terminal editor.

## Assumes
- A documented Unicode version and grapheme segmentation implementation.
- A documented terminal width policy for ambiguous-width characters.
- Allocation-fallible clipping helpers.

## Syntax Design
No new syntax is introduced.

```opal
let inspect_width = f(line: string): string errors TerminalTextLayoutError, AllocationFailureError =>
    let cells = terminal_text_cell_width(line)
    let clipped, used_cells = propagate terminal_text_clip_to_cells(line, 80 as int64)
    return clipped
```

## Example Applications
```opal
import terminal_text_cell_width, terminal_text_clip_to_cells from standard

let fit_status = f(status: string, width: int64): string errors TerminalTextLayoutError, AllocationFailureError =>
    let clipped, used_cells = propagate terminal_text_clip_to_cells(status, width)
    return clipped
```

## Strengths
- Correct target for combining marks, wide CJK, and many emoji sequences.
- Gives editor cursor math a cell-based foundation.
- Makes clipping behavior explicit and testable.

## Weaknesses
- Unicode data/versioning is a public compatibility commitment.
- Terminal emulators differ on ambiguous and emoji widths.
- More expensive than scalar or ASCII-only logic.

## Impact on Existing Syntax
No syntax changes. Adds standard-library functions and Unicode data.

## Interactions with Other Concerns
Pairs with grapheme-aware string editing and terminal rendering. It should not replace scalar string indexing; it is a layout layer.

## Implementation Difficulty
High. Requires segmentation, width tables, tests, and clear compatibility documentation.

## Must NOT Have
- No byte-index clipping.
- No silent invalid UTF-8 handling; strings are already UTF-8.
- No hidden normalization.
