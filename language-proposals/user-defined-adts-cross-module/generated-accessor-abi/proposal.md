# Generated Accessor ABI

## Overview

This alternative makes cross-module user ADTs work by generating constructor, predicate, field accessor, payload accessor, and update-helper functions for each public ADT. Importing modules can keep direct source syntax such as `state.mode` and `new EditorState:`, but the compiler lowers that syntax to calls into the defining module's generated ABI instead of sharing raw layout metadata.

The defining module owns the representation. Consumers depend on stable helper signatures rather than concrete field offsets. This is the strongest option for separate compilation and hot reload, but it is heavier than the simple editor needs as a first fix.

## Assumes

- The compiler can generate hidden or public ABI helper functions for type declarations.
- Generated helpers can be imported or linked across project modules.
- The optimizer can inline helper calls in whole-project builds.
- ADT values can be passed through a stable representation, such as an opaque pointer or a layout-stable value handle.
- Public source syntax can remain direct even if the lowering is helper-based.

## Syntax Design

The preferred source syntax remains ordinary ADT syntax.

### Source type declarations

```opal
# editor.types.op

##
  Description: Current editor mode.
##
public type EditorMode:
    Normal
    Insert
    Command

##
  Description: Complete editor state.
##
public type EditorState:
    lines: string[]
    mode: EditorMode
    dirty: boolean
```

### User-written source

```opal
# modes.op
import type EditorMode, EditorState from ./editor.types

##
  Description: Enter insert mode.
##
public let enter_insert_mode = f(state: EditorState): EditorState =>
    return new EditorState:
        lines: state.lines
        mode: new EditorMode.Insert
        dirty: state.dirty
```

### Conceptual lowered ABI

The compiler could lower the previous source as if these generated helpers existed:

```opal
# generated concept, not user source
public let __editor_mode_insert = f(): EditorMode => ...
public let __editor_state_new = f(lines: string[], mode: EditorMode, dirty: boolean): EditorState => ...
public let __editor_state_lines = f(state: EditorState): string[] => ...
public let __editor_state_dirty = f(state: EditorState): boolean => ...
```

Then `enter_insert_mode` conceptually becomes:

```opal
public let enter_insert_mode = f(state: EditorState): EditorState =>
    return __editor_state_new(
        __editor_state_lines(state),
        __editor_mode_insert(),
        __editor_state_dirty(state),
    )
```

The user does not write this lowered code. It illustrates the ABI strategy.

## Generated helper families

For a product type:

```opal
public type CursorPosition:
    line: int64
    column: int64
```

Generate conceptual helpers:

```opal
__cursor_position_new(line: int64, column: int64): CursorPosition
__cursor_position_line(value: CursorPosition): int64
__cursor_position_column(value: CursorPosition): int64
__cursor_position_with_line(value: CursorPosition, line: int64): CursorPosition
__cursor_position_with_column(value: CursorPosition, column: int64): CursorPosition
```

For an enum-like sum:

```opal
public type EditorMode:
    Normal
    Insert
    Command
```

Generate conceptual helpers:

```opal
__editor_mode_normal(): EditorMode
__editor_mode_insert(): EditorMode
__editor_mode_command(): EditorMode
__editor_mode_is_normal(value: EditorMode): boolean
__editor_mode_is_insert(value: EditorMode): boolean
__editor_mode_is_command(value: EditorMode): boolean
```

For a payload sum:

```opal
public type EditorStatus:
    Opened:
        path: string
    InvalidKey:
        key: string
```

Generate conceptual helpers:

```opal
__editor_status_opened(path: string): EditorStatus
__editor_status_invalid_key(key: string): EditorStatus
__editor_status_is_opened(value: EditorStatus): boolean
__editor_status_is_invalid_key(value: EditorStatus): boolean
__editor_status_opened_payload(value: EditorStatus): EditorStatus.OpenedPayload
__editor_status_invalid_key_payload(value: EditorStatus): EditorStatus.InvalidKeyPayload
__editor_status_opened_payload_path(payload: EditorStatus.OpenedPayload): string
__editor_status_invalid_key_payload_key(payload: EditorStatus.InvalidKeyPayload): string
```

Again, these names are illustrative. Real generated symbols should be compiler-internal, collision-resistant, and probably not user-importable by default.

## Example Applications

### Direct syntax preserved

```opal
# render_status.op
import type EditorStatus from ./editor.types

##
  Description: Render a status line.
##
public let status_text = f(status: EditorStatus): string =>
    if status is EditorStatus.Opened into opened:
        return 'opened {opened.path}'
    if status is EditorStatus.InvalidKey into invalid:
        return 'invalid key {invalid.key}'
    return 'ok'
```

The compiler lowers the variant checks and payload field access to generated predicates and accessors.

### Optional generated update syntax target

If generated `with_` helpers are available internally, the compiler can eventually support record-update syntax without exposing layout:

```opal
# Future syntax, not required for v1.
let next = state with:
    mode: new EditorMode.Insert
    dirty: true
```

Lowered conceptually to:

```opal
let next_dirty = __editor_state_with_dirty(state, true)
let next = __editor_state_with_mode(next_dirty, new EditorMode.Insert)
```

This proposal does not require record-update syntax, but generated helper ABI is compatible with it.

## Simple-neovim fit

### Works well

- The editor can use nominal `EditorState` across modules without exposing raw layout maps.
- Helpers can be stable across separately compiled modules.
- A future hot-reload host can compare helper signatures and layout hashes.
- Representation can change behind the helper ABI if the public source contract stays compatible.
- Direct syntax can remain pleasant if lowering is automatic.

### Does not work well

- The simple editor would generate many helper calls for common state transitions.
- Without optimizer inlining, record-heavy event handling may be slower and noisier in IR.
- The implementation is larger than necessary for a single-project fixture.
- Debugging generated helper names can be more difficult than debugging direct field offsets.
- If helpers are exposed publicly, the API surface becomes cluttered.

## Strengths

- **Excellent ABI boundary:** Consumers do not depend on concrete field offsets.
- **Hot reload friendly:** Function signatures and layout hashes form a clear compatibility contract.
- **Encapsulation:** Representation changes can be isolated to the defining module.
- **Future-proof:** Works well with packages, dynamic modules, and generated record-update helpers.
- **No source syntax burden:** Users can keep direct ADT syntax.

## Weaknesses

- **High implementation complexity:** Requires helper generation, linking, name mangling, and lowering rules.
- **Potential runtime overhead:** Many helper calls unless optimized.
- **Symbol explosion:** Large ADTs create many generated functions.
- **Tooling complexity:** Debuggers, docs, and diagnostics must hide or explain generated helpers carefully.
- **Ownership complexity:** Accessors returning RC-managed fields must have precise retain/drop semantics.

## Impact on Existing Syntax

No immediate syntax change is required if the compiler lowers existing syntax automatically. The generated ABI is an implementation strategy.

If generated helpers are exposed to user code, that would add a new discoverable API surface and should be a separate decision.

## Interactions with Other Concerns

- **Hot reload:** This is the strongest option for versioned dynamic modules.
- **Packages:** Helper signatures can be serialized in package metadata.
- **Memory model:** Accessors and constructors must define ownership transfer exactly.
- **LSP/docs:** Should show source-level fields and variants, not internal helpers.
- **Optimization:** Whole-project builds should inline helpers to avoid unnecessary overhead.
- **Record update:** Generated `with_` helpers provide a natural lowering target.

## Implementation Difficulty

High.

Main tasks:

1. Define stable generated symbol naming and visibility.
2. Generate constructors for products and variants.
3. Generate predicates for variants.
4. Generate payload extractor types and accessors.
5. Generate product field accessors and optional `with_` helpers.
6. Define ownership behavior for every helper.
7. Lower direct source syntax to helper calls.
8. Ensure helper calls inline in ordinary project builds.
9. Serialize helper ABI for packages/hot reload.
10. Hide generated internals from user-facing docs unless requested.

## Must NOT Have

- Must not force users to manually call ugly generated helper names for normal application code.
- Must not expose raw layout and generated helpers as competing public models without a clear rule.
- Must not return borrowed fields in ways that violate second-class reference rules.
- Must not create helper symbols that can collide with user identifiers.
- Must not make simple product field access unexpectedly fallible.
