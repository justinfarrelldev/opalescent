# Simple Neovim-like Editor Post-Mortem

This note is a developer-experience post-mortem for
`test-projects/terminal-simple-editor`, a small modal terminal editor written in
Opalescent. The editor opens a file, renders a line-numbered viewport, supports
normal/insert/command modes, edits text, saves with an atomic write, and can be
run deterministically with the terminal fake backend.

## Overall Experience

The project is now a stronger end-to-end stress test for generated multi-module
Opalescent applications. The standard library has enough pieces for a small
editor: terminal sessions, typed events, trusted terminal output, string range
editing, array insert/remove, filesystem reads/writes, and terminal cell
clipping.

The latest refactor replaced the old integer-constant state model with shared
public ADTs in `editor.types.op`: `EditorMode`, `EditorCommand`,
`EditorStatus`, `EditorTermination`, `CursorPosition`, `EditorState`, and
`EditorTransition`. Those types are constructed, refined, and field-accessed
from multiple importing modules, which makes the editor a realistic exercise of
module-interface ADT layout manifests.

The design is much closer to the natural architecture for a modal editor. The
remaining compromise is that the mutable `string[]` text buffer still lives next
to `EditorState` rather than inside it. During stress testing, array-owning
product state was the riskiest path; keeping the buffer separate preserves a
successful generated build/run while still using nominal state for modes,
commands, statuses, cursor position, transitions, and loop control.

## What Worked Well

The terminal-session surface is the biggest success. The editor can open a
session, read `TerminalInputEvent` values, write trusted rows, flush, move the
cursor, ring the bell, and close through `using` cleanup. The deterministic fake
backend makes an interactive program testable without manual input:

```text
key:ArrowDown|key:End|text:i|text:!|key:Escape|text::|text:wq|key:Enter|eof
```

That script reliably opens a file, moves to the second line, appends `!`, writes
with `:wq`, and exits with a summary.

The new ADT layout manifests worked for the important editor-shaped cases:
propertyless variants, payload variants, product field access, cross-module
constructor calls, imported variant checks, and transition products returned from
input handlers.

The filesystem and string-editing APIs are also a good fit.
`read_lines_sync`, `write_text_atomic_sync`, `string_insert_at`,
`string_delete_range`, `string_extract_range`, and `string_join` make the basic
buffer operations straightforward. The implementation is scalar-indexed rather
than fully grapheme-cluster-aware, and the project documents that limitation.

## Least Maintainable Portions

The top-level `main.op` still has orchestration boilerplate. It applies the
`EditorTransition` returned from text and named-key handlers, updates the
separate buffer value, and rings the bell when requested. This is much cleaner
than the old ten-label state tuples, but a future record-update syntax would
remove more repetition.

The command parser is intentionally tiny. It now uses `EditorCommand` variants
instead of integer states, including an `Invalid(text)` payload, but it is still
a small finite parser rather than a general command-line buffer.

The terminal cursor casts remain noisy:

```opalescent
(... + 1) as int32
```

They are safe in this fixture because viewport dimensions are tiny constants,
but the compiler correctly warns about narrowing `int64` to `int32`. A checked
conversion helper or cursor-position type would make this cleaner.

## What Opalescent Could Do Better Long-Term

A `match` expression with exhaustiveness checking would help. The current code
uses long chains of `if value is Variant:` checks, which is serviceable but not
ideal for event-heavy terminal programs.

Product records should become easier to use as complete application state. The
editor now passes around an `EditorState` product, but the line buffer remains
separate. The ideal future shape is still one state value containing the buffer,
cursor, viewport, dirty flag, mode, command, status, and termination metadata.

String and array ownership semantics need to be less surprising. The editor
still uses `copy_text` when returning owned label strings, and the buffer is kept
outside `EditorState` to avoid making array ownership inside long-lived products
the critical runtime path.

Project-level checking should be as convenient as single-file checking.
`opal check src/main.op` cannot resolve sibling project modules the same way
`opal build` can, so multi-module projects still use the slower build feedback
loop.

## Language Issues Encountered That Were Language/Compiler Faults

The original editor exposed missing imported ADT field-layout data for generated
project builds. That gap has now been addressed by module-interface layout
manifests, and this editor refactor exercises the new path directly.

The refactor also showed that source annotations still matter around imported
nominal values. Explicit local type annotations for `CursorPosition` and
payload-bearing `EditorStatus` values kept codegen on the manifest-backed
nominal layout path.

The string-literal ownership issue remains a language/runtime concern from an
application author's perspective. A helper returning a string label should not
require defensive cloning to avoid invalid frees.

## What Would Make This More Seamless

The ideal editor version would eventually use one state record:

```opalescent
EditorState = { lines, cursor, viewport, dirty, mode, command, status }
```

Then handlers could look like:

```opalescent
let next = propagate handle_event(state, event)
```

instead of returning an updated buffer alongside a transition.

A `match` form would make terminal event handling much cleaner:

```opalescent
match event:
    TerminalInputEvent.TextInput(text): ...
    TerminalInputEvent.Key(key): ...
    TerminalInputEvent.Resize(size): ...
```

A standard `int64_to_int32_checked` or cursor-position type would eliminate the
current cast warnings. A first-class terminal fixture runner would also help by
mapping a script file to `OPAL_TERMINAL_FAKE_EVENTS`, capturing output, and
checking saved files.

## What Opalescent Excelled At

Opalescent did well at making effects explicit. A reader can see that this
program touches the terminal, filesystem, layout engine, strings, arrays, and
ADT state because those effects and types appear in signatures and imports.

The terminal trust boundary is a strong design. The editor cannot accidentally
write arbitrary untrusted strings directly to a session; it constructs
`TrustedTerminalOutput` and writes through the session.

The fake terminal backend is excellent for this kind of project. Interactive
terminal programs are normally difficult to test deterministically, but the
scripted event stream makes this modal editing path reproducible in CI.

## Suggested Follow-Ups

- Investigate array/string ownership in product ADTs deeply enough that the line
  buffer can move into `EditorState` safely.
- Add match/exhaustiveness support for event and command handling.
- Add checked integer conversion helpers for terminal cursor APIs.
- Keep expanding generated-code regression coverage around imported ADT payloads,
  nested products, and transition records.

## Final Assessment

The simple editor is now a useful stress test for module-interface layout
manifests. It proves that Opalescent can build a deterministic terminal
application that reads input, edits a buffer, renders a screen, saves to disk,
uses shared nominal ADTs across modules, and exits cleanly.
