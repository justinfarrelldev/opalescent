# Module-interface Layout Manifests

## Overview

This is the recommended v1 design. Every module interface exports authoritative, module-qualified ADT layout metadata for public user-defined types. Importing modules consume those manifests for both type checking and generated code, including transitive imports.

The key idea is that if the type checker accepts an attested refinement such as `status is EditorStatus.InvalidKey into invalid` in a downstream module, then codegen must have the same metadata needed to lower the discriminant check, bind the payload, and access `invalid.key`. Current `is ... into ...` syntax requires the left side to be a direct identifier, so code that starts from `state.status` should bind `let status = state.status` first. The metadata should not be rediscovered through fragile short-name maps; it should travel as part of the module interface.

## Assumes

- `.types.op` files are the authoritative home for type declarations.
- Module interfaces already exist for exported symbols, type declarations, return labels, borrow metadata, and standard-module imports.
- The compiler can add a structured ADT layout manifest to module interfaces.
- Public ADT layouts are part of the public module contract for generated code.
- Type identity is nominal and internally module-qualified.

## Syntax Design

No new surface syntax is required. The proposal makes the existing syntax reliable and gives it a stable compiler-internal representation.

### Ground-up file layout

A small editor project would use files like this:

```text
src/
  main.op
  editor.types.op
  state.op
  input.op
  commands.op
  render.op
  buffer.op
```

### Step 1: define all shared nominal state in a types file

```opal
# src/editor.types.op

##
  Description: Current editor input mode.
##
public type EditorMode:
    Normal
    Insert
    Command

##
  Description: Command parsed from command mode text.
##
public type EditorCommand:
    None
    Write
    Quit
    WriteQuit
    ForceQuit
    Invalid:
        text: string

##
  Description: User-visible editor status.
##
public type EditorStatus:
    Opened:
        path: string
    Written:
        path: string
    UnsavedChanges
    InvalidKey:
        key: string
    CommandError:
        text: string

##
  Description: Text cursor position.
##
public type CursorPosition:
    line: int64
    column: int64

##
  Description: Complete editor state.
##
public type EditorState:
    lines: string[]
    cursor: CursorPosition
    viewport_line: int64
    dirty: boolean
    mode: EditorMode
    command: EditorCommand
    status: EditorStatus
    running: boolean
```

The compiler records a manifest equivalent to:

```text
module: ./editor.types
public type EditorMode:
  kind: sum
  variants:
    Normal: discriminant 0, fields []
    Insert: discriminant 1, fields []
    Command: discriminant 2, fields []

public type EditorCommand:
  kind: sum
  variants:
    None: fields []
    Write: fields []
    Quit: fields []
    WriteQuit: fields []
    ForceQuit: fields []
    Invalid: fields [text: string]

public type CursorPosition:
  kind: product
  fields:
    line: int64
    column: int64

public type EditorState:
  kind: product
  fields:
    lines: string[]
    cursor: ./editor.types::CursorPosition
    viewport_line: int64
    dirty: boolean
    mode: ./editor.types::EditorMode
    command: ./editor.types::EditorCommand
    status: ./editor.types::EditorStatus
    running: boolean
```

The text format above is explanatory. The implementation can store Rust structs, serialized metadata, or both.

### Step 2: constructors work from any importing module

```opal
# src/state.op
import type EditorMode, EditorCommand, EditorStatus, CursorPosition, EditorState from ./editor.types

##
  Description: Create editor state after opening a file.
##
public let create_editor_state = f(path: string, lines: string[]): EditorState =>
    return new EditorState:
        lines: lines
        cursor: new CursorPosition:
            line: 0
            column: 0
        viewport_line: 0
        dirty: false
        mode: new EditorMode.Normal
        command: new EditorCommand.None
        status: new EditorStatus.Opened:
            path: path
        running: true
```

Codegen for `state.op` uses the manifest from `./editor.types` to lay out `EditorState`, `CursorPosition`, `EditorMode`, `EditorCommand`, and `EditorStatus`.

### Step 3: field access works from any importing module

```opal
# src/viewport.op
import type EditorState from ./editor.types

##
  Description: Keep the viewport aligned with the cursor.
##
public let clamp_viewport_to_cursor = f(state: EditorState): EditorState =>
    if state.cursor.line < state.viewport_line:
        return new EditorState:
            lines: state.lines
            cursor: state.cursor
            viewport_line: state.cursor.line
            dirty: state.dirty
            mode: state.mode
            command: state.command
            status: state.status
            running: state.running

    return state
```

Codegen does not guess `cursor` by name. It resolves `EditorState` to `./editor.types::EditorState`, reads that manifest, then finds the `cursor` field index and type.

### Step 4: payload checks and payload field access work from any importing module

```opal
# src/render_status.op
import type EditorStatus from ./editor.types

##
  Description: Render the editor status line text.
##
public let render_editor_status = f(status: EditorStatus): string =>
    if status is EditorStatus.Opened into opened:
        return 'opened {opened.path}'
    if status is EditorStatus.Written into written:
        return 'written {written.path}'
    if status is EditorStatus.UnsavedChanges:
        return 'unsaved changes'
    if status is EditorStatus.InvalidKey into invalid:
        return 'invalid key {invalid.key}'
    if status is EditorStatus.CommandError into command_error:
        return 'command error {command_error.text}'
    return 'unknown status'
```

The manifest supplies:

- the `EditorStatus` discriminant layout;
- the payload layout for `Opened`, `Written`, `InvalidKey`, and `CommandError`;
- the field index for `path`, `key`, and `text` inside each payload;
- ownership/drop behavior for the payload strings.

### Step 5: state transitions become small and typed

```opal
# src/commands.op
import type EditorCommand, EditorMode, EditorStatus, EditorState from ./editor.types

##
  Description: Parse command-mode text.
##
public let parse_editor_command = f(text: string): EditorCommand =>
    if text is 'w':
        return new EditorCommand.Write
    if text is 'q':
        return new EditorCommand.Quit
    if text is 'wq':
        return new EditorCommand.WriteQuit
    if text is 'q!':
        return new EditorCommand.ForceQuit
    return new EditorCommand.Invalid:
        text: text

##
  Description: Apply a parsed command to editor state.
##
public let apply_editor_command = f(state: EditorState, command: EditorCommand): EditorState =>
    if command is EditorCommand.Quit:
        if state.dirty:
            return new EditorState:
                lines: state.lines
                cursor: state.cursor
                viewport_line: state.viewport_line
                dirty: state.dirty
                mode: new EditorMode.Command
                command: command
                status: new EditorStatus.UnsavedChanges
                running: true
        return new EditorState:
            lines: state.lines
            cursor: state.cursor
            viewport_line: state.viewport_line
            dirty: state.dirty
            mode: state.mode
            command: command
            status: state.status
            running: false

    if command is EditorCommand.Invalid into invalid:
        return new EditorState:
            lines: state.lines
            cursor: state.cursor
            viewport_line: state.viewport_line
            dirty: state.dirty
            mode: new EditorMode.Command
            command: command
            status: new EditorStatus.CommandError:
                text: invalid.text
            running: true

    return state
```

### Step 6: transitive imports still carry layouts

```opal
# src/app.op
import create_editor_state from ./state
import apply_editor_command, parse_editor_command from ./commands
import render_editor_status from ./render_status

##
  Description: Run one deterministic command for a tiny editor example.
##
public let run_one_command = f(): string =>
    let lines: string[] = ['alpha', 'beta']
    let state = create_editor_state('sample.txt', lines)
    let command = parse_editor_command('q')
    let next = apply_editor_command(state, command)
    return render_editor_status(next.status)
```

```opal
# src/main.op
import run_one_command from ./app

entry main = f(args: string[]): void =>
    print(run_one_command())
    return void
```

`main.op` does not directly import `./editor.types`, but project codegen still needs those manifests because `app.op`, `state.op`, `commands.op`, and `render_status.op` use the types. This proposal requires transitive manifest discovery.

## Required manifest contents

For each public product type:

- canonical module ID;
- public type name;
- stable internal type ID;
- ordered field names;
- resolved field core types;
- field visibility if private fields are later introduced;
- ownership/drop metadata for each field;
- layout hash for ABI/hot reload checks.

For each public sum type:

- canonical module ID;
- public type name;
- stable internal type ID;
- ordered variants;
- discriminant values;
- payload field names and resolved types;
- payload ownership/drop metadata;
- propertyless variant representation;
- layout hash.

For aliases and opaque types:

- alias target or opaque identity;
- whether layout is public, private, or unavailable to consumers.

## Example Applications

### Small product-only example

```opal
# cursor.types.op
public type CursorPosition:
    line: int64
    column: int64
```

```opal
# cursor_math.op
import type CursorPosition from ./cursor.types

public let move_right = f(cursor: CursorPosition): CursorPosition =>
    return new CursorPosition:
        line: cursor.line
        column: cursor.column + 1
```

```opal
# main.op
import type CursorPosition from ./cursor.types
import move_right from ./cursor_math

entry main = f(args: string[]): void =>
    let cursor = new CursorPosition:
        line: 3
        column: 4
    let moved = move_right(cursor)
    print('{moved.line},{moved.column}')
    return void
```

### Small enum-only example

```opal
# mode.types.op
public type EditorMode:
    Normal
    Insert
    Command
```

```opal
# mode_text.op
import type EditorMode from ./mode.types

public let mode_text = f(mode: EditorMode): string =>
    if mode is EditorMode.Normal:
        return 'normal'
    if mode is EditorMode.Insert:
        return 'insert'
    if mode is EditorMode.Command:
        return 'command'
    return 'unknown'
```

### Small payload example

```opal
# status.types.op
public type EditorStatus:
    Ok
    InvalidKey:
        key: string
```

```opal
# status_text.op
import type EditorStatus from ./status.types

public let status_text = f(status: EditorStatus): string =>
    if status is EditorStatus.Ok:
        return 'ok'
    if status is EditorStatus.InvalidKey into invalid:
        return 'invalid key {invalid.key}'
    return 'unknown'
```

## Simple-neovim fit

### Works well

- It enables the exact natural design requested by the post-mortem.
- `EditorMode`, `EditorCommand`, `EditorStatus`, and `EditorState` can be declared once and imported everywhere.
- Input, command, buffer, viewport, and render modules can all consume the same nominal `EditorState`.
- Handler signatures shrink from many labeled values to `f(state, event): EditorState errors ...`.
- Status payloads can carry real data such as invalid key names or command text.
- Renderer code can inspect payload statuses without extra side-channel strings.
- Future `match` exhaustiveness can build on the same manifest metadata.

### Does not work well

- Reconstructing `EditorState` without record-update syntax is still verbose.
- Large state records may produce repetitive code until update helpers or record update syntax exist.
- String ownership issues must still be fixed separately; a status payload containing a string must be owned safely.
- The implementation is more invasive than a small codegen patch.
- It requires careful collision and transitive import tests.

## Strengths

- **Correct abstraction boundary:** Layout metadata belongs to module interfaces, not incidental codegen globals.
- **Nominal identity:** Internal IDs can distinguish `./left.types::State` from `./right.types::State`.
- **Generated-code reliability:** Codegen receives the metadata that type checking already relied on.
- **Future package support:** Serialized manifests can become package interface metadata.
- **Hot reload readiness:** Layout hashes can feed ABI compatibility guards.
- **Better diagnostics:** Unknown fields and missing variants can be reported against source spans.
- **Editor-ready:** Fully supports the simple editor's desired state model.

## Weaknesses

- **Implementation effort:** Requires structured manifest design and plumbing.
- **ABI discipline:** Public field order and variant order become compatibility-sensitive.
- **Testing burden:** Needs fixtures for products, sums, nested ADTs, arrays, strings, transitive imports, aliases, and collisions.
- **Refactor cost:** Existing short-name layout maps may need module-qualified replacement.
- **Potential overexposure:** Public product field layout becomes part of the module contract unless opaque/private fields are designed later.

## Impact on Existing Syntax

No source syntax changes are required. The compiler internals change substantially:

- module interfaces gain ADT layout manifests;
- import resolution records canonical type IDs;
- codegen uses canonical type IDs for layouts and field indices;
- diagnostics report module-qualified names when needed;
- build/check commands can reason about project-level transitive layouts.

Existing working code should continue to compile.

## Interactions with Other Concerns

- **Project-level `opal check`:** Strongly complementary. Once manifests exist, project checking can use them to validate modules without running full codegen.
- **Hot reload:** Manifest layout hashes are the right input for ABI compatibility checks.
- **Memory model:** Ownership/drop metadata must be explicit for fields containing strings, arrays, and nested ADTs.
- **Match/exhaustiveness:** Variant manifests provide the variant list needed for exhaustiveness later.
- **LSP:** Completions for fields and variants can use the same metadata.
- **Docs:** Generated docs can show product fields and sum variants from authoritative manifests.
- **Packages:** Package artifacts can serialize manifests as their public type interface.

## Implementation Difficulty

Medium to high.

Recommended milestone order:

1. Add an internal `AdtTypeId` that includes canonical module path and type name.
2. Replace short-name layout keys in codegen with `AdtTypeId` where possible.
3. Extend `ModuleInterface` with a structured ADT manifest, not just ad hoc field maps.
4. Populate manifests when checking `.types.op` files.
5. Propagate manifests through direct and transitive imports.
6. Lower imported product construction and field access from manifests.
7. Lower imported enum construction and variant checks from manifests.
8. Lower imported payload variant construction, `into` binding, and payload field access from manifests.
9. Add layout-hash computation for public ADTs.
10. Migrate simple editor state from integer constants in a dedicated fixture.

## Must NOT Have

- Must not use short display names as global layout keys.
- Must not require source duplication of type declarations.
- Must not make codegen infer layout from the first field access it happens to see.
- Must not silently pick one layout when two imported types share a name.
- Must not expose private implementation types as public layout metadata.
- Must not claim record-update ergonomics are solved by this proposal.
- Must not hide string ownership bugs under ADT layout work.
