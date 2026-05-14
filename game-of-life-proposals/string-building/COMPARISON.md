# String Building Comparison

## Overview

Game of Life rendering repeatedly builds rows and full frames. Opalescent can already concatenate with interpolation assignment, as shown by `fs-markdown-roundtrip`, but a dedicated string-building surface would make repeated rendering clearer and less allocation-heavy.

## Summary Matrix

| Proposal | Ergonomics | Runtime Predictability | Implementation Effort | Game Fit | General Stdlib Fit |
| --- | --- | --- | --- | --- | --- |
| [Interpolation Accumulator](interpolation-accumulator/) | High | Medium | None to Low | Good | Medium |
| [String Builder Handle](string-builder-handle/) | High | High | Medium | Excellent | Excellent |
| [String Join Lines](string-join-lines/) | Very High | High | Low | Excellent | Excellent |

## Recommendation

Start with `string_join(values, separator)` because it is small, pure, and useful beyond games. Add `StringBuilder` once performance matters for large terminal frames. Keep interpolation accumulation as the current fallback and as a teaching pattern.

## Existing Syntax Anchor

Current Opalescent already supports this shape:

```opal
let mutable rendered = ''
let mutable index: int64 = 0
while index < lines.length:
    let line = lines[index]
    rendered = '{rendered}{line}\n'
    index = index + 1
```
