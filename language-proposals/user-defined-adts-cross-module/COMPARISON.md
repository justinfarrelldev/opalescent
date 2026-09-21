# User-defined ADTs Across Project Modules — Comparison

## Status and scope

This concern is a language/compiler proposal package, not a standard-library API. It responds to the `SIMPLE_NEOVIM_POST_MORTEM.md` finding that the simple modal editor should have been able to model its domain with nominal ADTs such as `EditorMode`, `EditorStatus`, `EditorCommand`, and `EditorState` across project module boundaries.

The current editor worked by using named `int64` constants and wide labeled multiple returns. That was a practical compatibility layer, but it is not the intended long-term Opalescent style. The purpose of this package is to compare ways to make user-defined ADTs dependable in generated project code.

## Reading order

1. This comparison.
2. [`status-quo-integer-constants`](./status-quo-integer-constants/proposal.md) — keep the current workaround.
3. [`enum-first-nominal-states`](./enum-first-nominal-states/proposal.md) — make propertyless nominal states work first.
4. [`whole-project-layout-merge`](./whole-project-layout-merge/proposal.md) — compile the full project with one merged ADT layout map.
5. [`module-interface-layout-manifests`](./module-interface-layout-manifests/proposal.md) — recommended v1: module-qualified layout manifests exported through interfaces.
6. [`generated-accessor-abi`](./generated-accessor-abi/proposal.md) — hide layouts behind generated constructors/accessors.
7. [`opaque-state-module-boundary`](./opaque-state-module-boundary/proposal.md) — avoid cross-module ADT values by architecture.

## Ground-up syntax target

The syntax shown below is the user experience this concern is trying to make reliable in generated code. Some of it is already used by standard terminal types; the gap is dependable project-defined ADTs across ordinary project modules.

### 1. Type declarations live in `.types.op`

```opal
# editor.types.op

##
  Description: The editor's current input mode.
##
public type EditorMode:
    Normal
    Insert
    Command

##
  Description: Parsed command-line command in command mode.
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
  Description: User-visible status after the latest editor action.
##
public type EditorStatus:
    Opened:
        path: string
    Written:
        path: string
    UnsavedChanges
    InvalidKey:
        key: string
    Message:
        text: string

##
  Description: Cursor position in the editable text buffer.
##
public type CursorPosition:
    line: int64
    column: int64

##
  Description: Complete state for a small modal editor.
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

### 2. Ordinary modules import the types explicitly

```opal
# state.op
import type EditorMode, EditorCommand, EditorStatus, CursorPosition, EditorState from ./editor.types

##
  Description: Build the initial editor state for an opened file.
##
public let editor_state_opened = f(path: string, lines: string[]): EditorState =>
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

### 3. Product fields should be readable in any importing module

```opal
# viewport.op
import type EditorState from ./editor.types

##
  Description: Returns true when the cursor is above the visible viewport.
##
public let cursor_is_above_viewport = f(state: EditorState): boolean =>
    return state.cursor.line < state.viewport_line
```

### 4. Rebuilding a product should be valid across modules

Opalescent does not need record-update syntax for v1. A reliable reconstruct-with-changed-field pattern is enough.

```opal
# modes.op
import type EditorMode, EditorState from ./editor.types

##
  Description: Enter insert mode without changing the buffer.
##
public let enter_insert_mode = f(state: EditorState): EditorState =>
    return new EditorState:
        lines: state.lines
        cursor: state.cursor
        viewport_line: state.viewport_line
        dirty: state.dirty
        mode: new EditorMode.Insert
        command: state.command
        status: state.status
        running: state.running
```

### 5. Enum-style variants should be checkable by name

```opal
# render_status.op
import type EditorMode, EditorState from ./editor.types

##
  Description: Render the current mode name for the status line.
##
public let editor_mode_name = f(state: EditorState): string =>
    if state.mode is EditorMode.Normal:
        return 'NORMAL'
    if state.mode is EditorMode.Insert:
        return 'INSERT'
    if state.mode is EditorMode.Command:
        return 'COMMAND'
    return 'UNKNOWN'
```

### 6. Payload variants should support `into` bindings and field access

Current attested `is ... into ...` syntax requires the left side to be a direct identifier. If the payload value starts as a member expression such as `state.status`, bind it to a local first.

```opal
# render_status.op
import type EditorStatus, EditorState from ./editor.types

##
  Description: Render a status value as user-visible text.
##
public let editor_status_text = f(status: EditorStatus): string =>
    if status is EditorStatus.Opened into opened:
        return 'opened {opened.path}'
    if status is EditorStatus.Written into written:
        return 'written {written.path}'
    if status is EditorStatus.UnsavedChanges:
        return 'unsaved changes'
    if status is EditorStatus.InvalidKey into invalid:
        return 'invalid key {invalid.key}'
    if status is EditorStatus.Message into message:
        return message.text
    return 'unknown status'

##
  Description: Render the status field from a complete editor state.
##
public let editor_state_status_text = f(state: EditorState): string =>
    let status = state.status
    return editor_status_text(status)
```

### 7. Handlers should pass one `EditorState`, not ten primitive results

```opal
# input_handlers.op
import string_insert_at from standard
import type EditorMode, EditorStatus, CursorPosition, EditorState from ./editor.types

##
  Description: Apply text input to the editor state.
##
public let apply_text_input = f(state: EditorState, text: string): EditorState errors StringRangeError, AllocationFailureError, IndexOutOfBoundsError =>
    if state.mode is EditorMode.Insert:
        let line = propagate state.lines.at(state.cursor.line)
        let updated_line = propagate string_insert_at(line, state.cursor.column, text)

        let mutable updated_lines: string[] = state.lines
        updated_lines[state.cursor.line] = updated_line

        return new EditorState:
            lines: updated_lines
            cursor: new CursorPosition:
                line: state.cursor.line
                column: state.cursor.column + text.length
            viewport_line: state.viewport_line
            dirty: true
            mode: state.mode
            command: state.command
            status: new EditorStatus.Message:
                text: 'inserted text'
            running: state.running

    return new EditorState:
        lines: state.lines
        cursor: state.cursor
        viewport_line: state.viewport_line
        dirty: state.dirty
        mode: state.mode
        command: state.command
        status: new EditorStatus.InvalidKey:
            key: text
        running: state.running
```

The exact editor implementation may choose different fields, but this is the shape Opalescent should support: small nominal state types, explicit error handling, ordinary imports, and generated code that knows the same layout information the type checker accepted.

## Comparison matrix

| Axis | Status quo integer constants | Enum-first nominal states | Whole-project layout merge | Module-interface layout manifests — recommended | Generated accessor ABI | Opaque state module boundary |
|---|---:|---:|---:|---:|---:|---:|
| **Simple-neovim fit** | ★★☆☆☆ | ★★★☆☆ | ★★★★☆ | ★★★★★ | ★★★★☆ | ★★★☆☆ |
| **Ergonomics** | ★★☆☆☆ | ★★★☆☆ | ★★★★☆ | ★★★★★ | ★★★☆☆ | ★★☆☆☆ |
| **Type safety** | ★☆☆☆☆ | ★★★☆☆ | ★★★★☆ | ★★★★★ | ★★★★★ | ★★★☆☆ |
| **Opalescent-idiom fit** | ★★☆☆☆ | ★★★☆☆ | ★★★★☆ | ★★★★★ | ★★★★☆ | ★★★☆☆ |
| **Implementation effort** | None | Low to medium | Medium | Medium to high | High | Low |
| **Compiler/codegen risk** | Low | Medium | Medium | Medium to high | High | Low |
| **Hot-reload/ABI readiness** | ★★☆☆☆ | ★★☆☆☆ | ★★☆☆☆ | ★★★★☆ | ★★★★★ | ★★★☆☆ |
| **Package/separate compilation readiness** | ★☆☆☆☆ | ★★☆☆☆ | ★★☆☆☆ | ★★★★★ | ★★★★★ | ★★☆☆☆ |
| **Extensibility** | ★☆☆☆☆ | ★★☆☆☆ | ★★★☆☆ | ★★★★★ | ★★★★☆ | ★★☆☆☆ |

## Alternatives summary

### Status quo integer constants

Keep using `int64` constants such as `mode_normal()`, `mode_insert()`, and `status_saved()`. This is what the simple editor does today.

**What works well for simple-neovim:**

- It already compiles and runs.
- Generated code stays on well-tested primitive paths.
- It avoids cross-module ADT layout bugs entirely.

**What does not work well:**

- `mode: int64` is not self-documenting.
- The compiler cannot prevent mixing a mode with a status or termination reason.
- Event handlers return many labeled primitive values instead of one `EditorState`.
- Adding new editor state means touching a large amount of repetitive plumbing.

This is acceptable as a fixture survival strategy, not as the long-term language answer.

### Enum-first nominal states

Make simple propertyless user enums reliable across module boundaries before solving products and payload variants completely.

**What works well for simple-neovim:**

- `EditorMode`, simple `EditorCommand`, and termination reasons become nominal quickly.
- The no-payload variant path is smaller than full product/sum layout support.
- It provides visible improvement without requiring record updates or complex drop logic first.

**What does not work well:**

- `EditorState` remains primitive plumbing or local-only.
- `EditorStatus.InvalidKey(key)` and other payload statuses remain unsupported or awkward.
- It risks becoming another partial plateau where the editor is only half nominal.

This is a reasonable stepping stone only if the full ADT plan remains active.

### Whole-project layout merge

During `opal build`, gather ADT layouts from every project module and merge them into the global codegen layout map before lowering.

**What works well for simple-neovim:**

- It directly targets the current generated-code failure mode.
- It lets a single project compile with `EditorState` shared across modules.
- It needs no surface syntax changes.

**What does not work well:**

- If keyed by short names, it is vulnerable to collisions such as two modules defining `State`.
- It is less natural for packages, incremental builds, and hot reload ABI guards.
- It can blur the difference between public interface metadata and compile-unit implementation detail.

This is the fastest practical bridge, but it should not be the final architecture unless it is made module-qualified and interface-driven.

### Module-interface layout manifests — recommended

Every module interface exports module-qualified ADT layout metadata for public types. The project build and codegen phases consume those manifests transitively.

**What works well for simple-neovim:**

- It supports the desired editor design directly: `EditorMode`, `EditorStatus`, `EditorCommand`, and `EditorState` in a shared `.types.op` file.
- Handler modules can pass and return one `EditorState`.
- Renderer modules can inspect status/mode safely.
- Diagnostics can talk about missing fields and variants instead of crashing in codegen.

**What does not work well:**

- It requires a real metadata model: module-qualified type IDs, field order, variant discriminants, ownership/drop info, and transitive import handling.
- It is more work than a local patch to the layout map.
- It needs careful tests for collisions, aliases, transitive imports, and invalid imports.

This is the recommended v1 because it is the first option that is both good for the editor and consistent with modules, packages, documentation, and future hot reload.

### Generated accessor ABI

The defining module generates constructors, predicates, accessors, and update helpers. Consumers do not need the concrete field layout; direct source syntax can lower to generated helper calls.

**What works well for simple-neovim:**

- Cross-module field access becomes ABI calls instead of layout sharing.
- It is strong for hot reload and separately compiled packages.
- It can preserve layout privacy and allow representation changes behind stable helper signatures.

**What does not work well:**

- It introduces many generated symbols for a small editor.
- Direct record-heavy code may lower to many calls unless optimization inlines them.
- It is more complex to implement than the editor needs right now.
- Without generated update helpers, rebuilding `EditorState` remains verbose.

This is attractive as a future ABI strategy, but it is heavy as the first fix for the simple editor.

### Opaque state module boundary

Avoid cross-module ADT values by keeping `EditorState` and most state-manipulating code inside one module. Other modules exchange primitives or rendered rows.

**What works well for simple-neovim:**

- It can be done with little or no compiler work.
- It localizes ADT use where codegen has the best chance of seeing layouts.
- It encourages encapsulation around editor state transitions.

**What does not work well:**

- It fights the clean module split the post-mortem wanted.
- The state module becomes a large kitchen-sink module.
- It does not solve user-defined ADTs across project modules.
- Renderer and input modules either lose type information or move back into the same file.

This is a workaround architecture, not a language improvement.

## Recommendation

Select **module-interface layout manifests** as the real v1 design.

A timeboxed **whole-project layout merge** can be used as an implementation bridge only if it moves toward the same module-qualified manifest model. It should not create a second public semantic model.

Do not select **status quo integer constants** or **opaque state module boundary** as language direction. They are useful compatibility techniques while compiler support is incomplete.

Do not select **enum-first nominal states** as the final scope. It can be the first implementation milestone, but the proposal succeeds only when product records and payload variants work too.

Keep **generated accessor ABI** as a future extension for package ABI stability and hot reload once the direct manifest model is correct.

## Required fixture ladder

The implementation should be driven by red-green-refactor fixtures. Suggested order:

1. **Product basics in one module**
   - Define `CursorPosition` in `cursor.types.op`.
   - Construct it in `state.op`.
   - Read `cursor.line` in `main.op`.

2. **Enum basics across modules**
   - Define `EditorMode` in `editor.types.op`.
   - Return `new EditorMode.Insert` from `modes.op`.
   - Check `if mode is EditorMode.Insert` in `main.op`.

3. **Payload sum basics across modules**
   - Define `EditorStatus.InvalidKey: key: string`.
   - Construct it in `input.op`.
   - Check `if status is EditorStatus.InvalidKey into invalid` and read `invalid.key` in `render.op`.

4. **Product containing imported ADTs**
   - Define `EditorState` with `mode: EditorMode` and `status: EditorStatus`.
   - Construct in `state.op`.
   - Update and return from `input.op`.
   - Render from `render.op`.

5. **Transitive import**
   - `main.op` imports `run_editor` from `app.op`.
   - `app.op` imports `handle_event` from `input.op`.
   - `input.op` imports `EditorState` from `editor.types.op`.
   - Codegen still has all required layouts.

6. **Name collision rejection or disambiguation**
   - `left.types.op` and `right.types.op` both define `State`.
   - Importing both must either use module-qualified internal IDs safely or produce a precise source diagnostic if the surface names collide.

7. **Simple editor migration fixture**
   - Replace integer constants for mode/status/command with nominal ADTs.
   - Replace wide handler returns with `EditorState` where practical.
   - Keep deterministic terminal fake backend assertions.

## Must not regress

- `.types.op` files remain the only place for type declarations.
- Ordinary `.op` modules must not duplicate type declarations to placate codegen.
- Valid programs must not fail with `missing field layout metadata` panics or equivalent internal errors.
- Public APIs must not require users to know LLVM layout details.
- Type identity must not be keyed only by short display names.
- Invalid fields and variants must remain user-facing type errors.
