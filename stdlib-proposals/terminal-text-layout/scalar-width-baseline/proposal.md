# ASCII Scalar Width Baseline

## Overview
This proposal defines a deliberately small terminal layout baseline where printable ASCII scalars occupy one terminal cell and non-ASCII text is rejected or escaped before rendering.

It is not a Unicode layout solution. It is a documented v1 limitation that lets a simple editor fixture be built without silently corrupting cursor math for unsupported text.

## Assumes
- Current strings expose Unicode scalar counts.
- The first editor can be scoped to ASCII/single-cell text.
- Non-ASCII behavior must be explicit and test-covered.

## Syntax Design
No new syntax is introduced.

```opal
let clipped = propagate terminal_ascii_clip_to_cells(line, 80 as int64)
```

## Example Applications
```opal
import terminal_ascii_clip_to_cells, terminal_ascii_is_single_cell_text from standard

let render_ascii_line = f(line: string, max_cells: int64): string errors TerminalTextLayoutError, AllocationFailureError =>
    if not terminal_ascii_is_single_cell_text(line):
        return propagate terminal_ascii_clip_to_cells('[unsupported unicode]', max_cells)
    return propagate terminal_ascii_clip_to_cells(line, max_cells)
```

## Strengths
- Simple and honest.
- Prevents accidental claims of Unicode correctness.
- Easy to test with editor fixtures.

## Weaknesses
- Rejects common valid UTF-8 text.
- Not suitable for internationalized editor behavior.
- Still requires careful escaping of control characters.

## Impact on Existing Syntax
No syntax changes. Adds a small terminal layout module.

## Interactions with Other Concerns
Pairs with terminal rendering and string editing. It is superseded by the grapheme cell-width proposal for general Unicode behavior.

## Implementation Difficulty
Low. ASCII detection and cell clipping are straightforward.

## Must NOT Have
- No claim that scalar count equals display width generally.
- No silent acceptance of unsupported wide/combining text in editor mode.
- No raw control character pass-through.
