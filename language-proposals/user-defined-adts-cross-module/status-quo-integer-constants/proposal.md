# Status Quo: Integer Constants for Editor State

## Overview

This alternative keeps the current simple-editor workaround: represent modes, statuses, commands, and termination reasons with named integer constants instead of project-defined ADTs crossing module boundaries.

It does not attempt to fix generated-code support for user-defined ADTs. It is included because it is the baseline that already works and because it clarifies exactly what the language loses when nominal state types are unavailable.

## Assumes

- Primitive integers, booleans, arrays, strings, and labeled multiple returns remain reliable in generated code.
- Project modules can import ordinary functions and constants.
- The editor is small enough that manual state threading is tolerable.
- Safety is provided by naming discipline, not by nominal type checking.

## Syntax Design

No new syntax is introduced. State categories are represented by functions or public constants returning `int64`.

### Ground-up mode constants

```opal
# constants.op

##
  Description: Numeric value for normal editor mode.
##
public let editor_mode_normal = f(): int64 =>
    return 1

##
  Description: Numeric value for insert editor mode.
##
public let editor_mode_insert = f(): int64 =>
    return 2

##
  Description: Numeric value for command editor mode.
##
public let editor_mode_command = f(): int64 =>
    return 3
```

### Ground-up status constants

```opal
# constants.op

##
  Description: Numeric value for an opened-file status.
##
public let editor_status_opened = f(): int64 =>
    return 10

##
  Description: Numeric value for a written-file status.
##
public let editor_status_written = f(): int64 =>
    return 11

##
  Description: Numeric value for an invalid-key status.
##
public let editor_status_invalid_key = f(): int64 =>
    return 12
```

### Import and check constants

```opal
# render.op
import editor_mode_normal, editor_mode_insert, editor_mode_command from ./constants

##
  Description: Render a numeric mode value as display text.
##
public let editor_mode_name = f(mode: int64): string =>
    if mode is editor_mode_normal():
        return 'NORMAL'
    if mode is editor_mode_insert():
        return 'INSERT'
    if mode is editor_mode_command():
        return 'COMMAND'
    return 'UNKNOWN'
```

### Wide multi-return instead of `EditorState`

```opal
# input.op
import editor_mode_insert, editor_status_invalid_key from ./constants

##
  Description: Apply a text input event and return all updated editor fields.
##
public let apply_text_input = f(lines: string[], mode: int64, dirty: boolean, cursor_line: int64, cursor_column: int64, text: string): lines: string[], mode: int64, dirty: boolean, cursor_line: int64, cursor_column: int64, status: int64 =>
    if mode is editor_mode_insert():
        # Real editor code updates the selected line here.
        return lines: lines, mode: mode, dirty: true, cursor_line: cursor_line, cursor_column: cursor_column + text.length, status: 0

    return lines: lines, mode: mode, dirty: dirty, cursor_line: cursor_line, cursor_column: cursor_column, status: editor_status_invalid_key()
```

## Example Applications

A minimal event loop can stay entirely primitive:

```opal
import editor_mode_normal, editor_mode_insert from ./constants
import editor_mode_name from ./render
import apply_text_input from ./input

entry main = f(args: string[]): void =>
    let mutable lines: string[] = ['alpha', 'beta']
    let mutable mode = editor_mode_normal()
    let mutable dirty = false
    let mutable cursor_line: int64 = 0
    let mutable cursor_column: int64 = 0
    let mutable status: int64 = 0

    # Pretend the user pressed i.
    mode = editor_mode_insert()

    let lines: next_lines, mode: next_mode, dirty: next_dirty, cursor_line: next_line, cursor_column: next_column, status: next_status = apply_text_input(lines, mode, dirty, cursor_line, cursor_column, '!')

    lines = next_lines
    mode = next_mode
    dirty = next_dirty
    cursor_line = next_line
    cursor_column = next_column
    status = next_status

    print('mode={editor_mode_name(mode)} dirty={dirty}')
    return void
```

This is representative of the current compatibility style: the code can be made readable with labels, but the labels are compensating for the missing state record.

## Simple-neovim fit

### Works well

- It is proven by the existing simple editor fixture.
- It avoids fragile ADT field layout paths.
- It keeps generated code on primitive ABI paths.
- It makes deterministic terminal testing possible today.
- It is easy to debug because values are just numbers.

### Does not work well

- `mode`, `status`, `command`, and `termination_reason` all have the same type.
- The compiler cannot reject passing a status where a mode is expected.
- Payload data has to be carried separately. For example, `InvalidKey` cannot own its `key` field.
- Handler signatures grow as editor state grows.
- The main loop has repetitive code to copy every returned label back into mutable locals.
- Documentation and LSP completions cannot show the closed set of modes or commands.
- A reader cannot tell from a signature whether `int64` means a cursor coordinate, a mode, a status, or a command state.

## Strengths

- **No implementation cost:** This is already available.
- **Low codegen risk:** Primitives and arrays are the best-tested generated-code paths.
- **Good emergency fallback:** When richer ADTs break, integer constants let the fixture keep testing terminal/session behavior.
- **No syntax churn:** Users do not need to learn a new feature.

## Weaknesses

- **Weak type safety:** The important categories are not nominal.
- **Boilerplate explosion:** State updates become wide return signatures.
- **Poor extensibility:** Adding a new state field requires editing many signatures and assignments.
- **Stringly/numeric payloads:** Status messages and command payloads need side channels.
- **Not idiomatic long-term Opalescent:** The language has ADTs specifically to avoid this style.

## Impact on Existing Syntax

None. This alternative preserves current syntax and current generated-code assumptions.

## Interactions with Other Concerns

- **Multiple returns:** Heavy reliance on labeled multiple returns.
- **Explicit errors:** Error signatures still work normally, but state values are not self-describing.
- **Terminal generated testing:** Works well because the current editor already uses this shape.
- **LSP/docs:** Provides weak semantic information because constants are just functions returning numbers.
- **Future match/exhaustiveness:** Cannot benefit from exhaustive variant checks.

## Implementation Difficulty

None. The implementation already exists as application code.

## Must NOT Have

- Must not be documented as the preferred long-term Opalescent application architecture.
- Must not block work on nominal ADT support.
- Must not add more implicit conversions between integer state categories.
- Must not grow into a standard-library editor-state convention.
