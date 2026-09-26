# Inline `with` Record Update Expression

## Overview

This alternative adds a compact one-line record update form:

```opal
return state with status: status
```

For small updates, especially one-field replacements, this is the shortest readable spelling. It addresses boilerplate without requiring an indented block for every state transition.

This alternative is attractive as optional sugar after block `with:` exists, but it is not recommended as the first or only record-update design because multi-field updates become hard to format and parse.

## Assumes

- Product field layouts are available to the type checker and code generator.
- Record update semantics are the same as block `with:`: produce a new product, do not mutate the receiver.
- The parser can disambiguate inline update colons from type annotations and labeled returns.

## Syntax Design

### Single-field form

```opal
record_update_inline := expression "with" field_name ":" expression
```

Example:

```opal
let next = state with status: new EditorStatus.Ok
```

### Multi-field form

A possible extension is comma-separated fields:

```opal
let next = state with cursor: cursor, dirty: true, status: new EditorStatus.Moved
```

However, v1 should consider restricting inline syntax to exactly one field and relying on block `with:` for multiple fields. That avoids unclear wrapping behavior and parser ambiguity.

### Relationship to block form

The inline expression is exactly equivalent to:

```opal
let next = state with:
    status: new EditorStatus.Ok
```

The formatter may choose to expand inline updates to block form when the expression exceeds line width.

## Example Applications

### One-field status replacement

```opal
import type EditorState, EditorStatus from ./editor.types

##
  Description: Return editor state with a replacement status using inline update syntax.
##
public let with_status = f(state: EditorState, status: EditorStatus): EditorState =>
    return state with status: status
```

### Boolean flag replacement

```opal
import type EditorState from ./editor.types

##
  Description: Return editor state marked as saved.
##
public let mark_saved = f(state: EditorState): EditorState =>
    return state with saved: true
```

## Strengths

- Very concise for the common one-field case.
- Easy to read in simple helper functions.
- Keeps field names explicit.
- Can share type-checking and codegen machinery with block `with:`.
- Formatter can promote long inline updates to block updates.

## Weaknesses

- `field: value` visually resembles existing label and type syntaxes.
- Multi-field inline updates become dense quickly.
- Constructor-valued replacements may make lines too long.
- Parser ambiguity is higher than for `with:` blocks.
- If both inline and block syntax exist, users must learn two forms.

## Impact on Existing Syntax

This is additive if `with` is contextual. It should not reserve `with` as a global keyword.

The largest impact is syntactic ambiguity. The parser already uses colons for type annotations, constructor fields, labeled returns, guards, and control-flow blocks. The grammar must ensure that `state with status: status` is only parsed after an expression receiver and the contextual `with` keyword.

## Interactions with Other Concerns

- **Formatter:** should decide when inline updates remain inline and when they expand to block form.
- **Named-return destructuring:** inline `field: local` may look similar to `returned_label: local_name`; diagnostics and docs must be explicit.
- **Generated update helpers:** inline syntax can lower to the same helper as block syntax.
- **Record-update block syntax:** if both exist, the block form should be canonical for documentation and examples with more than one field.

## Implementation Difficulty

Medium if block update already exists; medium to high if implemented alone.

Main tasks:

1. Add parser support for contextual inline `with`.
2. Decide whether v1 supports one field or many comma-separated fields.
3. Reuse record-update type checking and codegen.
4. Add formatter line-width expansion rules.
5. Add diagnostics for ambiguous or malformed inline updates.

## Must NOT Have

- Must not allow multi-field updates that format unpredictably.
- Must not make `with` a reserved word outside update position.
- Must not permit field updates to be confused with type annotations in diagnostics.
- Must not mutate the receiver.
