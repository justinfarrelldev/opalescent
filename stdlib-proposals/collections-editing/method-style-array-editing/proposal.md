# Method-Style Array Editing

## Overview
This proposal exposes array editing as member-style operations: `values.insert(index, value)` and `values.remove_at(index)`. This matches the already documented `.push`, `.pop`, and `.at` style.

## Assumes
- Method calls on arrays are part of the standard-library lowering surface.
- Errors remain explicit and call sites use `propagate` or `guard`.
- Generic arrays preserve element type across insert/remove operations.

## Syntax Design
No new syntax beyond existing member-call style is introduced.

Selected generated-runtime v1 lowers `insert` and `remove_at` only when the receiver is an identifier-bound array value (for example, `lines.insert(...)`). Expression receivers such as `make_lines().insert(...)` remain future work so codegen can preserve ownership/cleanup semantics explicitly.

```opal
let insert_then_remove = f(lines: string[]): string[] errors IndexOutOfBoundsError, AllocationFailureError =>
    let with_line = propagate lines.insert(1 as int64, 'new')
    let updated, removed = propagate with_line.remove_at(0 as int64)
    return updated
```

## Example Applications
```opal
let delete_current_line = f(lines: string[], line_index: int64): string[] errors IndexOutOfBoundsError, AllocationFailureError =>
    let updated, removed = propagate lines.remove_at(line_index)
    return updated
```

## Strengths
- Best fit with existing array member operations.
- Discoverable through LSP member completion.
- Keeps editor buffer code readable.

## Weaknesses
- Requires method dispatch/lowering consistency.
- Public docs must explain which methods mutate and which return updated arrays.
- Might duplicate free-function names unless aliasing is deliberate.

## Impact on Existing Syntax
Uses existing member-call syntax. Requires standard-library method registration and docs.

## Interactions with Other Concerns
Can be an alias layer over free-function array editing. Should follow the broader `collections-api-shape/method-style-api` recommendation.

## Implementation Difficulty
Medium. The underlying runtime operations are simple, but method registration, generic inference, and diagnostics need care.

## Must NOT Have
- No implicit index clamping.
- No removal that discards the removed value without an explicit discard helper.
- No inconsistent behavior with `array_insert` / `array_remove_at`.
