# Mutable Copy Product Update

## Overview

This alternative allows product fields to be assigned on a local mutable copy:

```opal
let mutable next = state
next.status = status
return next
```

The assignment updates `next`, not `state`. Semantically, `next` is a distinct product value with value/COW semantics. This gives users a familiar imperative spelling for a functional update.

This is not recommended as the primary design because it can blur Opalescent's value model, but it is a plausible secondary feature for code that performs several sequential updates.

## Assumes

- Product values can be bound to `let mutable` locals.
- Field assignment can be restricted to mutable local product bindings and mutable reference parameters.
- Assigning an RC-managed field retains the new value and releases the overwritten value safely.
- The compiler can reject assignment through immutable bindings and non-local temporaries.

## Syntax Design

### Local mutable copy

```opal
let mutable next: EditorState = state
next.status = status
return next
```

### Field assignment rule

A field assignment is valid only when the left side begins with a mutable binding or a `mutable ref` parameter:

```opal
next.status = status
```

Invalid:

```opal
state.status = status
```

when `state` is an ordinary immutable parameter.

### Nested field assignment

Nested field assignment should not be in v1:

```opal
next.cursor.column = column
```

This creates more complicated aliasing and partial-update semantics. Users can update the nested product first:

```opal
let mutable cursor = next.cursor
cursor.column = column
next.cursor = cursor
```

## Example Applications

```opal
import type CursorPosition, EditorState, EditorStatus from ./editor.types

##
  Description: Return editor state after a buffer edit using mutable-copy update syntax.
##
public let edit_state = f(
    state: EditorState,
    cursor: CursorPosition,
    status: EditorStatus
): EditorState =>
    let mutable next = state
    next.cursor = cursor
    next.dirty = true
    next.status = status
    return next
```

## Strengths

- Familiar to programmers from many languages.
- Handles several sequential updates without nested expressions.
- No new `with` expression syntax.
- Works naturally with conditional updates:

```opal
let mutable next = state
if should_bell:
    next.status = new EditorStatus.InvalidKey:
        key: key
return next
```

- Could be useful independently of record-update syntax.

## Weaknesses

- Can make product values look like reference objects.
- Harder to visually distinguish local mutation from external state mutation.
- Requires field-assignment codegen and ownership semantics.
- Nested assignments raise aliasing questions.
- Less declarative than `with:` blocks for simple updates.

## Impact on Existing Syntax

This extends assignment targets from variables and array indices to product fields rooted in mutable bindings.

The type checker must reject field assignment through immutable bindings, temporaries, function-call results, and non-mutable parameters.

## Interactions with Other Concerns

- **Second-class references:** field assignment through `mutable ref state` may be desirable but needs strict aliasing checks.
- **Arrays and strings:** assigning RC-managed fields must retain/release correctly.
- **Record-update syntax:** this can coexist with `with:`; `with:` remains preferred for direct functional updates.
- **Formatter:** nested member assignment formatting should be straightforward.
- **LSP:** diagnostics should explain when a field assignment requires `let mutable`.

## Implementation Difficulty

Medium to high.

Main tasks:

1. Extend assignment target parsing and AST representation.
2. Type-check product field assignment roots for mutability.
3. Resolve field offsets through layout manifests.
4. Emit code to update fields with RC retain/release.
5. Reject nested assignment for v1 or implement it deliberately.
6. Add diagnostics for immutable roots and unknown fields.
7. Add tests proving the original value is not accidentally mutated when copied.

## Must NOT Have

- Must not allow assignment through immutable product parameters.
- Must not imply reference semantics for ordinary product values.
- Must not permit assignment to unknown fields.
- Must not silently mutate aliased data in violation of value semantics.
- Must not implement nested field assignment accidentally without an explicit design.
