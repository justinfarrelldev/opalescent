# Whole-project Layout Merge

## Overview

This alternative fixes cross-module user ADTs by treating an `opal build` project as one generated-code unit for ADT layout purposes. Before lowering any module, the compiler walks the project dependency graph, collects every reachable `.types.op` ADT layout, merges those layouts into a single codegen map, and then codegen can construct, inspect, pass, and return those values from any project module.

This is close to the practical fix that a small application needs. It is especially attractive for the simple Neovim-like editor because that editor is one project, not a separately compiled package graph. The risk is that a plain global merge can become ad hoc unless it is module-qualified and aligned with the module interface model.

## Assumes

- `opal build` has a project graph rooted at `src/main.op`.
- All reachable project source files are available at build time.
- Cross-module ADT layout can be collected before final codegen.
- The generated binary is built as one coherent project image.
- Separate package ABI and hot reload compatibility are not solved in this first step.

## Syntax Design

No new syntax is introduced. The goal is to make the natural syntax work in a project build.

### Step 1: define editor types once

```opal
# src/editor.types.op

##
  Description: The editor's current input mode.
##
public type EditorMode:
    Normal
    Insert
    Command

##
  Description: Status shown in the editor status line.
##
public type EditorStatus:
    Opened:
        path: string
    Written:
        path: string
    UnsavedChanges
    InvalidKey:
        key: string

##
  Description: Cursor location in the text buffer.
##
public type CursorPosition:
    line: int64
    column: int64

##
  Description: Complete editor state shared by editor modules.
##
public type EditorState:
    lines: string[]
    cursor: CursorPosition
    viewport_line: int64
    dirty: boolean
    mode: EditorMode
    status: EditorStatus
    running: boolean
```

### Step 2: construct the product in one module

```opal
# src/state.op
import type EditorMode, EditorStatus, CursorPosition, EditorState from ./editor.types

##
  Description: Create the initial state after reading a file.
##
public let new_editor_state = f(path: string, lines: string[]): EditorState =>
    return new EditorState:
        lines: lines
        cursor: new CursorPosition:
            line: 0
            column: 0
        viewport_line: 0
        dirty: false
        mode: new EditorMode.Normal
        status: new EditorStatus.Opened:
            path: path
        running: true
```

### Step 3: update the product in another module

```opal
# src/modes.op
import type EditorMode, EditorStatus, EditorState from ./editor.types

##
  Description: Enter insert mode and update the status line.
##
public let enter_insert_mode = f(state: EditorState): EditorState =>
    return new EditorState:
        lines: state.lines
        cursor: state.cursor
        viewport_line: state.viewport_line
        dirty: state.dirty
        mode: new EditorMode.Insert
        status: new EditorStatus.UnsavedChanges
        running: state.running
```

### Step 4: inspect the product and sum payload in another module

```opal
# src/render.op
import type EditorMode, EditorStatus, EditorState from ./editor.types

##
  Description: Render one compact summary of editor state.
##
public let editor_summary = f(state: EditorState): string =>
    let mutable mode_text = 'UNKNOWN'
    if state.mode is EditorMode.Normal:
        mode_text = 'NORMAL'
    if state.mode is EditorMode.Insert:
        mode_text = 'INSERT'
    if state.mode is EditorMode.Command:
        mode_text = 'COMMAND'

    let status = state.status
    let mutable status_text = 'unknown'
    if status is EditorStatus.Opened into opened:
        status_text = 'opened {opened.path}'
    if status is EditorStatus.Written into written:
        status_text = 'written {written.path}'
    if status is EditorStatus.UnsavedChanges:
        status_text = 'unsaved changes'
    if status is EditorStatus.InvalidKey into invalid:
        status_text = 'invalid key {invalid.key}'

    return 'mode={mode_text} status={status_text} dirty={state.dirty}'
```

### Step 5: use all modules from main

```opal
# src/main.op
import new_editor_state from ./state
import enter_insert_mode from ./modes
import editor_summary from ./render

entry main = f(args: string[]): void =>
    let lines: string[] = ['alpha', 'beta']
    let initial = new_editor_state('sample.txt', lines)
    let next = enter_insert_mode(initial)
    print(editor_summary(next))
    return void
```

With whole-project layout merge, codegen sees all layouts before lowering `state.op`, `modes.op`, `render.op`, and `main.op`.

## Example Applications

The simple editor can migrate incrementally:

1. Convert `mode` from `int64` to `EditorMode`.
2. Convert `status` from `int64` plus message side channel to `EditorStatus`.
3. Convert `command_state` and parsed command values to `EditorCommand`.
4. Replace repeated handler return labels with one `EditorState` return.

A handler can become:

```opal
import string_delete_range from standard
import type EditorMode, EditorStatus, CursorPosition, EditorState from ./editor.types

##
  Description: Apply backspace to an editor state.
##
public let apply_backspace = f(state: EditorState): EditorState errors StringRangeError, AllocationFailureError, IndexOutOfBoundsError =>
    if state.mode is not EditorMode.Insert:
        return new EditorState:
            lines: state.lines
            cursor: state.cursor
            viewport_line: state.viewport_line
            dirty: state.dirty
            mode: state.mode
            status: new EditorStatus.InvalidKey:
                key: 'Backspace'
            running: state.running

    if state.cursor.column is 0:
        return state

    let line = propagate state.lines.at(state.cursor.line)
    let updated_line = propagate string_delete_range(line, state.cursor.column - 1, state.cursor.column)

    let mutable updated_lines: string[] = state.lines
    updated_lines[state.cursor.line] = updated_line

    return new EditorState:
        lines: updated_lines
        cursor: new CursorPosition:
            line: state.cursor.line
            column: state.cursor.column - 1
        viewport_line: state.viewport_line
        dirty: true
        mode: state.mode
        status: state.status
        running: state.running
```

## Simple-neovim fit

### Works well

- It is the most direct path to the desired editor source shape.
- The editor is a single project, so full-project layout collection is naturally available.
- It removes the integer constant workaround for modes/statuses/commands.
- It lets handlers return `EditorState` instead of wide multiple returns.
- It allows renderer, input, buffer, and command modules to share one nominal state type.

### Does not work well

- It can hide missing interface modeling by relying on all source being present.
- If layouts are keyed only by short names, two modules defining `State` can collide.
- It is less convincing for future packages, cached builds, and hot reload.
- It may not produce clean diagnostics if codegen discovers missing metadata late.
- It can become a one-off project-build hack if not refactored into a manifest model.

## Strengths

- **No surface syntax changes:** Users write the ADT syntax they already expect.
- **Strong practical payoff:** It fixes the most painful editor architecture problem.
- **Good for fixture-driven development:** Small cross-module projects can prove each ADT operation.
- **Implementation bridge:** It can share collection logic with the eventual manifest approach.
- **Immediate generated-code coverage:** Exercises construction, field access, variant checks, payload extraction, pass, and return.

## Weaknesses

- **Architecture risk:** A global layout map can become another implicit side channel.
- **Name collision risk:** Short-name keys are not robust.
- **Weak separate-compilation story:** Downstream code cannot consume layouts without source or serialized interface metadata.
- **Hot reload weakness:** ABI hashes need stable public layout manifests, not incidental project maps.
- **Incrementality weakness:** Any ADT layout change can invalidate broad codegen if dependencies are not tracked precisely.

## Impact on Existing Syntax

No syntax changes. Existing valid code remains valid. The implementation changes project-build analysis and codegen metadata flow.

## Interactions with Other Concerns

- **Module resolver:** Must expose enough project graph information for layout collection.
- **Type checker:** Must keep accepted ADT operations aligned with codegen layout availability.
- **Codegen:** Needs product layouts and sum-variant payload layouts before lowering all modules.
- **RC/memory model:** Field ownership/drop behavior must be known for ADTs containing strings, arrays, and nested ADTs.
- **Project-level `opal check`:** This option benefits from project-aware checking, but can be implemented first for `opal build`.
- **Hot reload:** Should eventually feed ABI signature generation, but a naive merge is insufficient.

## Implementation Difficulty

Medium.

Main tasks:

1. Collect all reachable project modules before codegen.
2. Extract ADT product layouts and sum variant layouts from `.types.op` declarations.
3. Convert AST field types to codegen core types after imports are resolved.
4. Store layouts under module-qualified keys.
5. Teach field access and variant lowering to use module-qualified owner identity.
6. Add diagnostics for duplicate ambiguous short names.
7. Add red fixtures for product field access, payload variants, and transitive imports.

## Must NOT Have

- Must not key layouts only by display name.
- Must not require duplicating type definitions in `.op` files.
- Must not allow private types to leak as public ABI by accident.
- Must not turn codegen metadata absence into a runtime crash.
- Must not be described as package ABI support unless serialized interfaces are implemented.
