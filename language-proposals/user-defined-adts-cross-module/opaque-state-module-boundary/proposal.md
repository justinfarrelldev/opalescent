# Opaque State Module Boundary

## Overview

This alternative avoids the cross-module ADT codegen problem by changing application architecture rather than the compiler. The editor keeps rich state and most state transitions inside one module, and other modules exchange primitive data, rendered rows, or high-level commands.

This is not a solution to user-defined ADTs across project modules. It is a workaround that may be useful while compiler support is incomplete. It is included because it can keep a project moving without integer constants everywhere, but it does not satisfy the long-term language goal from the post-mortem.

## Assumes

- ADTs are more reliable when their construction and field access are local to the same module or compilation context.
- Primitive values and arrays remain safe to pass across modules.
- The editor can tolerate a larger core module.
- Some module boundaries can be moved from typed state values to string rows, primitive summaries, or command functions.

## Syntax Design

No new syntax is introduced. The design changes where types are used.

### Ground-up state stays inside one core module

```opal
# editor_core.types.op

##
  Description: Internal editor mode.
##
public type EditorMode:
    Normal
    Insert
    Command

##
  Description: Internal editor state.
##
public type EditorState:
    lines: string[]
    cursor_line: int64
    cursor_column: int64
    viewport_line: int64
    dirty: boolean
    mode: EditorMode
    running: boolean
```

```opal
# editor_core.op
import type EditorMode, EditorState from ./editor_core.types

##
  Description: Create an internal editor state.
##
public let editor_create = f(lines: string[]): EditorState =>
    return new EditorState:
        lines: lines
        cursor_line: 0
        cursor_column: 0
        viewport_line: 0
        dirty: false
        mode: new EditorMode.Normal
        running: true

##
  Description: Apply an insert-mode transition internally.
##
public let editor_enter_insert = f(state: EditorState): EditorState =>
    return new EditorState:
        lines: state.lines
        cursor_line: state.cursor_line
        cursor_column: state.cursor_column
        viewport_line: state.viewport_line
        dirty: state.dirty
        mode: new EditorMode.Insert
        running: state.running

##
  Description: Render a primitive summary for modules that should not inspect state.
##
public let editor_summary = f(state: EditorState): string =>
    if state.mode is EditorMode.Insert:
        return 'mode=INSERT dirty={state.dirty}'
    return 'mode=OTHER dirty={state.dirty}'
```

This still passes `EditorState` to callers, so it only works if the compiler supports passing and returning the type. If field access is the failing operation, this can help by restricting field access to `editor_core.op`.

### Stronger workaround: keep cross-module boundaries primitive

If even passing `EditorState` is unreliable, the module can expose operations that return only primitive summaries.

```opal
# editor_core.op
import type EditorMode, EditorState from ./editor_core.types

let internal_state_after_demo = f(lines: string[]): EditorState =>
    let initial = new EditorState:
        lines: lines
        cursor_line: 0
        cursor_column: 0
        viewport_line: 0
        dirty: false
        mode: new EditorMode.Insert
        running: true
    return initial

##
  Description: Run one internal editor demo and return a primitive summary.
##
public let run_editor_demo_summary = f(lines: string[]): string =>
    let state = internal_state_after_demo(lines)
    if state.mode is EditorMode.Insert:
        return 'mode=INSERT line={state.cursor_line} column={state.cursor_column}'
    return 'mode=OTHER'
```

```opal
# main.op
import run_editor_demo_summary from ./editor_core

entry main = f(args: string[]): void =>
    let lines: string[] = ['alpha', 'beta']
    print(run_editor_demo_summary(lines))
    return void
```

This avoids exposing `EditorState` at the cost of making `editor_core.op` own too much behavior.

## Example Applications

A split can be drawn around rendering rows instead of state:

```opal
# editor_core.op
import type EditorMode, EditorState from ./editor_core.types

##
  Description: Render editor rows internally from state.
##
public let editor_render_rows = f(state: EditorState): string[] =>
    let mutable rows: string[] = []
    for line in state.lines:
        rows.push(line)
    if state.mode is EditorMode.Insert:
        rows.push('-- INSERT --')
    else:
        rows.push('-- NORMAL --')
    return rows
```

```opal
# terminal_render.op
import terminal_text_clip_to_cells from standard

##
  Description: Clip pre-rendered rows to a terminal width.
##
public let clip_rows = f(rows: string[], width: int64): string[] errors TerminalTextLayoutError, AllocationFailureError =>
    let mutable clipped_rows: string[] = []
    for row in rows:
        let clipped, used_cells = propagate terminal_text_clip_to_cells(row, width)
        clipped_rows.push(clipped)
    return clipped_rows
```

The renderer does not need `EditorState`; it only receives `string[]` rows. This reduces ADT pressure but also reduces type-rich composition.

## Simple-neovim fit

### Works well

- It can reduce the number of modules that need `EditorState` layout metadata.
- It can keep some nominal ADT benefits internally.
- It may avoid the exact codegen path that fails today.
- It encourages a small public API around editor actions and rendered rows.
- It is easier than a compiler overhaul.

### Does not work well

- It undermines the clean module split between input, commands, state, viewport, buffer, and rendering.
- `editor_core.op` can become large and hard to maintain.
- It does not let independent modules naturally accept and return `EditorState`.
- It does not solve `EditorMode`, `EditorStatus`, `EditorCommand`, and `EditorState` across module boundaries.
- It can force primitive summaries or row arrays through boundaries where nominal types would be clearer.

## Strengths

- **Low implementation cost:** Mostly application refactoring.
- **Pragmatic:** Helps projects proceed while compiler support is incomplete.
- **Encapsulation:** External modules cannot mutate or inspect state fields directly if boundaries are primitive.
- **Reduced codegen surface:** Fewer cross-module ADT operations need to work.

## Weaknesses

- **Not the language fix:** It avoids rather than solves the compiler issue.
- **Poor scalability:** One core module accumulates unrelated responsibilities.
- **Weaker type flow:** Primitive summaries replace domain-specific types at boundaries.
- **Testing awkwardness:** Fine-grained state transitions are harder to test from outside.
- **Limited reuse:** Buffer and viewport logic are less reusable if buried in editor core.

## Impact on Existing Syntax

None. This is an architecture pattern, not a syntax change.

## Interactions with Other Concerns

- **Project-level checking:** Less pressure because fewer modules import state types.
- **Terminal generated testing:** Works if the top-level editor behavior remains deterministic.
- **Match/exhaustiveness:** Internal code can benefit later, but external modules cannot.
- **LSP/docs:** Public API documentation becomes thinner and less state-specific.
- **Hot reload:** A larger core module may reduce fine-grained hot-swap opportunities.

## Implementation Difficulty

Low for an application workaround. No compiler implementation is required unless even local ADTs are unreliable.

Refactoring steps:

1. Move `EditorState` and transition helpers into `editor_core.op`.
2. Keep external module inputs and outputs primitive where possible.
3. Render rows inside the core before passing to terminal renderer helpers.
4. Expose only high-level functions such as `run_editor`, `editor_summary`, or `editor_render_rows`.
5. Add tests around public summaries and saved-file effects.

## Must NOT Have

- Must not be mistaken for robust cross-module ADT support.
- Must not require global mutable editor state as a hidden singleton.
- Must not make terminal/session ownership implicit.
- Must not permanently block a later migration to real `EditorState` values across modules.
- Must not hide important editor errors behind string summaries.
