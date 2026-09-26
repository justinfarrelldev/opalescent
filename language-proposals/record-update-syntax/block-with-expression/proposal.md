# Block `with:` Record Update Expression

## Overview

This alternative adds an indentation-sensitive record update expression for product types:

```opal
let next = state with:
    status: new EditorStatus.UnsavedChanges
```

It creates a new product value of the same type as `state`, preserving every field not listed in the update block and replacing the listed fields with new values. It directly addresses the editor-state boilerplate that currently requires helper functions or full product reconstruction.

This is the recommended v1 because it matches Opalescent's existing block style, avoids braces, and stays readable when updating several fields.

## Assumes

- Product types have known field layouts through local type declarations or module-interface layout manifests.
- Product values have value semantics: the update produces a new value and does not mutate the original receiver.
- RC-managed unchanged fields can be retained or moved according to existing RC analysis.
- `with` becomes a contextual keyword in expression position after a receiver expression.

## Syntax Design

### Source form

```opal
record_update_expression := expression "with" ":" newline indent record_update_field+ dedent
record_update_field := field_name ":" expression newline
```

Example:

```opal
let next = state with:
    cursor: cursor
    dirty: true
    status: new EditorStatus.InsertedText
```

The receiver expression must type-check as a product type. Each update field must name a field on that product. Each update expression must type-check against the declared field type.

### Duplicate fields

Duplicate updates are rejected:

```opal
let next = state with:
    dirty: true
    dirty: false
```

The diagnostic should point at the second `dirty` and explain that each field may appear at most once.

### Nested updates

V1 does not need special dot-path syntax. Nesting composes naturally:

```opal
let next_cursor = state.cursor with:
    column: column

return state with:
    cursor: next_cursor
```

A future extension may allow `cursor.column: column`, but v1 should avoid that additional grammar and ownership complexity.

### Fallible update expressions

A field value expression may contain `propagate` if the enclosing function declares the errors:

```opal
let next = state with:
    status: new EditorStatus.InvalidKey:
        key: propagate string_copy(key_text)
```

The update expression itself is not fallible unless one of its field expressions is fallible.

## Example Applications

### Replace status only

```opal
import type EditorState, EditorStatus from ./editor.types

##
  Description: Return editor state with a replacement status.
##
public let with_status = f(state: EditorState, status: EditorStatus): EditorState =>
    return state with:
        status: status
```

### Replace cursor, dirty flag, and status after an edit

```opal
import type CursorPosition, EditorState, EditorStatus from ./editor.types

##
  Description: Return editor state after a text edit.
##
public let edit_transition_state = f(
    state: EditorState,
    cursor: CursorPosition,
    status: EditorStatus
): EditorState =>
    return state with:
        cursor: cursor
        dirty: true
        status: status
```

### Stop the editor loop

```opal
import type EditorState, EditorStatus, EditorTermination from ./editor.types

##
  Description: Return editor state after a stop condition.
##
public let stopped_state = f(
    state: EditorState,
    status: EditorStatus,
    termination: EditorTermination
): EditorState =>
    return state with:
        status: status
        running: false
        termination: termination
```

## Strengths

- Makes the changed fields visible and hides mechanical unchanged fields.
- Uses colon and indentation, matching Opalescent's existing visual style.
- Scales better than inline syntax for multi-field updates.
- Preserves field-name checking and explicit values.
- Naturally composes with existing product constructors.
- Can lower to full reconstruction, generated update helpers, or optimized field-copy code.
- Good LSP target: completion can list fields of the receiver type.

## Weaknesses

- Adds new parser and formatter work.
- `with` must become a contextual keyword without breaking existing identifiers.
- The compiler must define exact ownership behavior for unchanged RC fields.
- Nested updates require temporary locals unless a future path-update extension is added.
- Very small one-field updates are still two lines.

## Impact on Existing Syntax

This is additive. Existing constructors and helper functions keep working.

Parser impact:

- Recognize `expression with:` as a record update expression.
- Disambiguate from a variable named `with`; the keyword should be contextual only after a completed expression in update position.

Formatter impact:

- Emit one update field per line.
- Indent field expressions consistently with constructor fields.
- Preserve or normalize blank lines according to existing block rules.

Type-checker impact:

- Resolve the receiver product type.
- Validate field existence, duplicates, and field expression types.
- Preserve nominal product identity.

Codegen impact:

- Build a new product with all original fields plus replacements.
- Retain RC-managed original fields that are copied into the new value.
- Apply normal ownership rules for replacement expressions.

## Interactions with Other Concerns

- **ADT layout manifests:** imported product field layouts are required for cross-module updates.
- **String ownership:** unchanged and replacement string fields must be retained/moved safely.
- **Arrays:** copied array fields should preserve value semantics and COW/RC behavior.
- **Hot reload:** source-level updates should lower through stable product layout metadata or generated helpers.
- **Generated accessor ABI:** this syntax can lower to generated `with_` helpers if direct layout access is not available.
- **Formatter:** this feature should be added only with formatter support because indentation is semantic.

## Implementation Difficulty

Medium.

Main tasks:

1. Add AST node for product update expressions.
2. Extend the parser with contextual `with:` recognition.
3. Extend type checking with product-field validation.
4. Extend codegen to reconstruct product values safely.
5. Add RC retain/drop tests for unchanged fields.
6. Add diagnostics for unknown, duplicate, and mistyped fields.
7. Add formatter round-trip coverage.
8. Add LSP completion/hover support for update fields.

## Must NOT Have

- Must not mutate the receiver.
- Must not permit unknown fields to be ignored.
- Must not allow duplicate fields with last-write-wins behavior.
- Must not require braces around the update body.
- Must not silently drop RC-managed fields copied from the receiver.
- Must not introduce structural typing for nominal product values.
