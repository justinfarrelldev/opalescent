# Generated Product Update Helpers

## Overview

This alternative avoids new source syntax by generating update helper functions for product types. For a product `EditorState`, the compiler or a generated module would provide helpers such as:

```opal
editor_state_with_status(state, status)
editor_state_with_cursor_dirty_status(state, cursor, dirty, status)
```

Users call helpers instead of reconstructing every field. The helpers can also become an internal lowering target for future `with:` syntax.

This alternative is strongest as a compatibility and ABI strategy, but weaker as the primary user-facing syntax because helpers multiply quickly.

## Assumes

- The compiler has product field metadata for public product types.
- Generated helpers can be named without colliding with user symbols.
- Helpers either live in the defining module's generated interface or are compiler-internal lowering targets.
- RC ownership behavior for copied fields is centralized in helper codegen.

## Syntax Design

No new syntax is required.

### Single-field generated helper

For this type:

```opal
public type EditorState:
    cursor: CursorPosition
    dirty: boolean
    status: EditorStatus
```

Generate a conceptual helper:

```opal
public let editor_state_with_status = f(
    state: EditorState,
    status: EditorStatus
): EditorState =>
    return new EditorState:
        cursor: state.cursor
        dirty: state.dirty
        status: status
```

### Optional generated helper set

Possible helper families:

1. One helper per field: `editor_state_with_status`.
2. One generic generated helper with named fields, if the language later supports generated named parameters.
3. Common grouped helpers only when explicitly requested by an annotation.

V1 should prefer one helper per field if this alternative is chosen, because it has the smallest surface and avoids combinatorial explosion.

## Example Applications

```opal
import editor_state_with_status from ./editor.generated
import type EditorState, EditorStatus from ./editor.types

##
  Description: Return editor state with a replacement status using a generated helper.
##
public let with_status = f(state: EditorState, status: EditorStatus): EditorState =>
    return editor_state_with_status(state, status)
```

For multiple fields, helpers chain:

```opal
import editor_state_with_cursor, editor_state_with_dirty from ./editor.generated
import editor_state_with_status from ./editor.generated
import type CursorPosition, EditorState, EditorStatus from ./editor.types

##
  Description: Return editor state after a buffer edit using generated update helpers.
##
public let edit_state = f(
    state: EditorState,
    cursor: CursorPosition,
    status: EditorStatus
): EditorState =>
    let moved = editor_state_with_cursor(state, cursor)
    let dirtied = editor_state_with_dirty(moved, true)
    return editor_state_with_status(dirtied, status)
```

## Strengths

- No new source syntax required.
- Can be implemented as part of module-interface metadata and codegen.
- Excellent lowering target for future record-update syntax.
- Good for package and hot-reload ABI because helpers have stable signatures.
- Centralizes RC ownership details in generated code.
- LSP can offer helper completions if helpers are user-visible.

## Weaknesses

- Helper names are verbose and mechanical.
- Multi-field updates require chains or many generated combinations.
- Publicly exposing generated helpers may clutter module APIs.
- If helpers are hidden, users do not get an immediate ergonomic improvement without syntax.
- Combinatorial helper generation must be avoided.

## Impact on Existing Syntax

No syntax changes. Existing user-written helper functions remain valid.

If generated helpers are user-visible, import rules and documentation need a policy for generated symbols. If generated helpers are compiler-internal only, this proposal does not solve source ergonomics until paired with record-update syntax.

## Interactions with Other Concerns

- **Generated accessor ABI:** this is a subset of the broader generated-accessor strategy.
- **Record-update syntax:** block and inline `with` can lower to these helpers.
- **Hot reload:** helper signatures can participate in ABI compatibility hashes.
- **String ownership:** helper bodies must retain RC-managed fields safely.
- **Documentation:** generated helpers should be hidden by default unless users opt into generated API docs.

## Implementation Difficulty

Medium.

Main tasks:

1. Decide helper visibility and naming.
2. Generate one-field update helpers for public product types.
3. Serialize helper signatures in module metadata if public/importable.
4. Ensure helper bodies retain/drop RC-managed fields correctly.
5. Add import, docs, and LSP behavior for generated symbols if exposed.
6. Add tests for name collisions and stable ABI hashes.

## Must NOT Have

- Must not generate every possible field combination by default.
- Must not expose helper names that can collide with user-written names.
- Must not make generated helpers the only permanent ergonomic story if source syntax is desired.
- Must not hide ownership bugs inside helpers.
- Must not require users to import an entire generated module implicitly.
