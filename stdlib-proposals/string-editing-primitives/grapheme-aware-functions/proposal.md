# Grapheme-Aware Functions

## Overview
This proposal adds Unicode grapheme-cluster editing helpers for applications that need user-perceived character operations rather than scalar operations.

The helpers mirror the scalar editing surface but count and edit by grapheme cluster index. This is the necessary direction before a terminal editor can claim general Unicode cursor/backspace correctness.

## Assumes
- A documented Unicode version is bundled with each Opalescent release.
- Grapheme segmentation rules are stable for the release and named in docs.
- Scalar helpers exist or can share validation/allocation infrastructure.

## Syntax Design
No new syntax is introduced.

```opal
let count = string_grapheme_length(text)
let edited = propagate string_grapheme_insert_at(text, 1 as int64, 'x')
```

## Example Applications
```opal
import string_grapheme_delete_range from standard

let delete_previous_grapheme = f(line: string, grapheme_column: int64): string errors StringRangeError, AllocationFailureError =>
    if grapheme_column is 0:
        return line
    return propagate string_grapheme_delete_range(line, grapheme_column - 1, grapheme_column)
```

## Strengths
- Matches user-visible editing behavior better than scalar indexes.
- Prevents backspace from splitting combining sequences in supported cases.
- Creates a foundation for Unicode-correct editor examples.

## Weaknesses
- Larger Unicode dependency and compatibility surface.
- Grapheme indexes are still not terminal display cells.
- More expensive than scalar-only operations.

## Impact on Existing Syntax
No syntax changes. Adds library/runtime support and documentation for Unicode versioning.

## Interactions with Other Concerns
Works with `terminal-text-layout/unicode-grapheme-cell-width` for cursor cell movement and clipping. Does not replace scalar APIs.

## Implementation Difficulty
High. Requires Unicode segmentation data, tests for combining marks/emoji, and release-version documentation.

## Must NOT Have
- No hidden normalization of strings.
- No byte-index overloads.
- No claim that grapheme count equals display width.
