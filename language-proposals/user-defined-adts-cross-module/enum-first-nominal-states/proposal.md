# Enum-first Nominal States

## Overview

This alternative implements the smallest useful slice of cross-module user ADT support first: propertyless enum-style variants. The immediate goal is to replace integer constants for `EditorMode`, simple `EditorCommand` values, and termination reasons with nominal types while deferring full product and payload-variant support.

It is a stepping-stone option. It improves the simple editor more than the status quo, but it does not deliver the full post-mortem goal because `EditorState` and payload statuses still need reliable product/sum layout support.

## Assumes

- `.types.op` files already parse enum-like sum declarations.
- A propertyless variant can be represented as a discriminant without payload layout.
- Generated code can compare discriminants across modules.
- The compiler can keep nominal type identity separate from the integer representation.
- Product field layout and payload field layout may remain incomplete in this phase.

## Syntax Design

No new surface syntax is required. The proposal makes existing enum-style syntax reliable across project modules.

### Step 1: define a propertyless enum in `.types.op`

```opal
# editor_mode.types.op

##
  Description: The editor input mode.
##
public type EditorMode:
    Normal
    Insert
    Command
```

### Step 2: import the enum type where it is used

```opal
# modes.op
import type EditorMode from ./editor_mode.types

##
  Description: Return normal mode.
##
public let normal_mode = f(): EditorMode =>
    return new EditorMode.Normal

##
  Description: Return insert mode.
##
public let insert_mode = f(): EditorMode =>
    return new EditorMode.Insert
```

### Step 3: compare variants by nominal name

```opal
# render_mode.op
import type EditorMode from ./editor_mode.types

##
  Description: Render an editor mode name for the status bar.
##
public let editor_mode_name = f(mode: EditorMode): string =>
    if mode is EditorMode.Normal:
        return 'NORMAL'
    if mode is EditorMode.Insert:
        return 'INSERT'
    if mode is EditorMode.Command:
        return 'COMMAND'
    return 'UNKNOWN'
```

### Step 4: define simple command values

```opal
# editor_command.types.op

##
  Description: Parsed command-mode command without payloads.
##
public type EditorCommand:
    None
    Write
    Quit
    WriteQuit
    ForceQuit
    Invalid
```

This keeps the enum-first phase small. A later phase can replace `Invalid` with `Invalid: text: string` once payload variants are supported.

### Step 5: parse commands into nominal values

```opal
# commands.op
import type EditorCommand from ./editor_command.types

##
  Description: Parse command text into a nominal editor command.
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
    return new EditorCommand.Invalid
```

## Example Applications

Enum-first support lets the main loop stop accepting arbitrary `int64` modes:

```opal
import type EditorMode from ./editor_mode.types
import type EditorCommand from ./editor_command.types
import insert_mode, normal_mode from ./modes
import parse_editor_command from ./commands
import editor_mode_name from ./render_mode

entry main = f(args: string[]): void =>
    let mutable mode: EditorMode = normal_mode()
    let command: EditorCommand = parse_editor_command('wq')

    if command is EditorCommand.WriteQuit:
        mode = insert_mode()

    print('mode={editor_mode_name(mode)}')
    return void
```

This already prevents a status value or termination reason from being assigned to `mode`, as long as those categories also become separate enum types.

## Simple-neovim fit

### Works well

- `EditorMode` becomes nominal and self-documenting.
- `EditorCommand` can become nominal for simple no-payload commands.
- Termination reasons such as `Continue`, `Quit`, and `WriteQuit` can stop being raw integers.
- Renderer code reads more naturally: `if mode is EditorMode.Insert` instead of `if mode is editor_mode_insert()`.
- The implementation surface is smaller than full ADT layout support.

### Does not work well

- It does not solve `EditorState` as a cross-module product record.
- It does not solve `EditorStatus.InvalidKey: key: string` or other payload statuses.
- It does not remove wide multi-return handler signatures.
- It still requires primitive side channels for status messages and invalid command text.
- It risks splitting the editor into nominal modes but primitive everything else.

## Strengths

- **Small first milestone:** Propertyless variants require discriminants but no payload field layout.
- **Immediate safety win:** Modes and commands stop being interchangeable integers.
- **No syntax change:** The existing ADT syntax is enough.
- **Good compiler test wedge:** Establishes module-qualified type identity and variant lookup before field layout complexity.
- **Better docs and LSP:** Variant names are discoverable.

## Weaknesses

- **Incomplete editor architecture:** The state record remains unresolved.
- **Payload deferral:** Real statuses and events often need data.
- **Potential false sense of completion:** A half-nominal editor is better but still not the post-mortem target.
- **Duplicate work risk:** If implemented with a special enum-only path, it may need refactoring when full sum support lands.

## Impact on Existing Syntax

No syntax changes. It requires the parser, type checker, module resolver, and codegen to agree on imported propertyless variant identity and representation.

## Interactions with Other Concerns

- **Match/exhaustiveness:** This option becomes much more valuable once `match` exists, but must not depend on it.
- **Error handling:** Error sum types can use the same enum path when they have no payloads.
- **Module interfaces:** Still needs module-qualified exported variant metadata.
- **Hot reload:** Discriminant stability must be considered if public enum order changes.
- **Simple editor terminal input:** Helps command and mode branching but not terminal event payload access.

## Implementation Difficulty

Low to medium.

Main tasks:

1. Store module-qualified enum type identity in module interfaces.
2. Export ordered propertyless variant metadata.
3. Lower `new Type.Variant` for imported user types.
4. Lower `if value is Type.Variant` for imported user types.
5. Add diagnostics for unknown variants and wrong-type comparisons.
6. Add compile/run fixtures for mode and command enums across modules.

## Must NOT Have

- Must not key type identity only by short type name.
- Must not erase enum values to plain `int64` in user-facing type checking.
- Must not treat enum-only support as completion of this concern.
- Must not make payload variants syntactically invalid; they are deferred, not rejected as a language direction.
