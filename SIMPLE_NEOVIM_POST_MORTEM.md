# Simple Neovim-like Editor Post-Mortem

This note is a developer-experience post-mortem for creating
`test-projects/terminal-simple-editor`, a small modal terminal editor written in
Opalescent. The editor opens a file, renders a line-numbered viewport, supports
normal/insert/command modes, edits text, saves with an atomic write, and can be
run deterministically with the terminal fake backend.

## Overall Experience

The project was possible, but it exposed several language and codegen rough
edges that directly shaped the final design. The standard library now has enough
pieces for a small editor: terminal sessions, typed events, trusted terminal
output, string range editing, array insert/remove, filesystem reads/writes, and
terminal cell clipping. Those are substantial wins.

The hardest part was not the editor algorithm. The hard part was representing
editor state cleanly in current Opalescent. The natural design is a few product
and sum types such as `EditorState`, `EditorMode`, `EditorStatus`, and
`CommandBuffer`. In practice, generated-code support for application ADTs across
modules is not mature enough for that shape. The project had to fall back to
module-level named `int64` constants for modes, statuses, command states, and
termination reasons.

The code is now split into modules and is much cleaner than the first version,
but it is still more mechanical than the problem deserves.

## What Worked Well

The terminal-session surface was the biggest success. The editor can open a
session, read `TerminalInputEvent` values, write trusted rows, flush, move the
cursor, ring the bell, and close through `using` cleanup. The deterministic fake
backend made it possible to test an interactive program without manual input:

```text
key:ArrowDown|key:End|text:i|text:!|key:Escape|text::|text:wq|key:Enter|eof
```

That script reliably opens the fixture file, moves to the second line, appends
`!`, writes with `:wq`, and exits with a summary.

The filesystem and string-editing APIs were also a good fit. `read_lines_sync`,
`write_text_atomic_sync`, `string_insert_at`, `string_delete_range`,
`string_extract_range`, and `string_join` made the basic buffer operations
straightforward. The implementation is scalar-indexed rather than
fully grapheme-cluster-aware, but it is explicit about that limitation.

The terminal layout helpers were useful. `terminal_text_clip_to_cells` gave the
renderer a safe way to clip status/help text and buffer rows to the display width
without hand-rolling display-cell behavior.

Explicit errors were valuable. File I/O, terminal operations, string editing,
array editing, and terminal layout all advertise failure modes in signatures.
That made the final entry point honest about what can fail.

## Least Maintainable Portions

The least maintainable part is the state model. `constants.op` contains named
integer constants for modes, command states, statuses, and termination reasons.
This is better than ad hoc functions returning `1`, `2`, `3`, and so on, but it
is still a workaround. The maintainable long-term representation is nominal ADTs:

```opalescent
EditorMode.Normal
EditorMode.Insert
EditorMode.Command
```

and product records for editor state and event results. Today, that shape is not
reliable enough in generated multi-module code, so the code uses integers as a
compatibility layer.

The second least maintainable part is the event-handling return shape. Helpers
such as `apply_text_input` and `apply_named_key` return many labeled values:
updated lines, mode, dirty flag, cursor position, command state, status,
running/termination flags, and bell request. This is readable enough for a small
fixture, but it is not an ideal application architecture. It exists because an
`EditorState` product value with update helpers was not a safe generated-code
path.

The command parser is intentionally tiny but not elegant. It uses a finite
command-state integer rather than accumulating a string. That avoided string
ownership/runtime crashes encountered during development, but it also means the
command layer is not naturally extensible.

The top-level `main.op` still has too much orchestration. It is much smaller
after the module split, but it still has repetitive code for applying the results
of text and named-key handlers. A first-class state record and record update
syntax would remove a lot of boilerplate.

The terminal cursor casts are also not ideal:

```opalescent
(... + 1) as int32
```

They are safe in this fixture because viewport dimensions are tiny constants,
but the compiler quite reasonably warns about narrowing `int64` to `int32`.
There is no ergonomic checked conversion helper in the editor code, so these
warnings remain noisy.

## What Opalescent Could Do Better Long-Term

The biggest long-term improvement would be robust generated-code support for
user-defined ADTs across project modules. A modal editor wants small nominal
state types. If `EditorMode`, `EditorStatus`, `EditorCommand`, and `EditorState`
worked smoothly across module boundaries, the code would become simpler,
safer, and more self-documenting.

A `match` expression with exhaustiveness checking would also help. The current
code uses long chains of `if value is VariantOrConstant:` checks. That is
serviceable, but it is not the right long-term shape for event-heavy terminal
programs.

Product records need to be easier to use as ordinary application state. The
editor should be able to pass around one `EditorState`, return an updated state,
and access fields reliably in generated code. Without that, multi-return
signatures become the substitute for records.

The compiler should make project-level checking as convenient as single-file
checking. `opal check src/main.op` cannot resolve sibling project modules the
same way `opal build` can. For multi-module projects, that makes the faster
feedback loop less useful than it should be.

String ownership semantics need to be less surprising. Returning or storing
string literals through helpers led to runtime invalid-free behavior during
development. The workaround was to create owned copies with a tiny helper. The
language/runtime should either make literal ownership safe automatically or make
owned/borrowed string boundaries much more explicit.

Terminal cursor APIs should have one clear coordinate contract. Legacy terminal
helpers and session helpers do not feel fully aligned: one path had zero-based
expectations, while the session cursor movement runtime rejects zero and expects
one-based positions. An editor author should not need to rediscover that through
`InvalidCursorPositionError`.

Finally, warnings need better severity and suppression ergonomics. The cast
warnings are useful, but unavoidable safe casts in tiny bounded code should have
a clean checked/saturated/narrowing helper or a local way to prove the bound.

## Language Issues Encountered That Were Language/Compiler Faults

The clearest compiler-side issue was imported ADT field layout for project
builds. Terminal event payload field access, such as `text_event.text` and
`named.key`, required project-build codegen to know field layouts from imported
standard terminal modules. The implementation needed a compiler fix so project
builds merge imported standard ADT field layouts into the global codegen layout
map.

Application ADTs were not dependable enough for the editor state model. Probes
with user-defined mode/state types failed in generated code paths where the type
checker accepted or nearly accepted the shape, but codegen could not lower the
field or variant access cleanly. That forced the integer-constant state model.

The string-literal ownership crash was also a language/runtime issue from an app
author's perspective. A simple helper returning a string label should not result
in `munmap_chunk(): invalid pointer`. Whether the eventual fix is runtime
ownership metadata, literal interning, or stricter typing, the current behavior
is too easy to trip.

Project-level ergonomics were another language tooling issue. Single-file
checking is useful for one-file examples, but it does not provide the right
feedback loop for an idiomatic multi-module project.

## What Would Make This More Seamless

The ideal editor version would have these types:

```opalescent
EditorMode = Normal | Insert | Command
EditorCommand = None | Write | Quit | WriteQuit | ForceQuit | Invalid
EditorStatus = Opened | Written | UnsavedChanges | InvalidKey | ...
EditorState = { lines, cursor, viewport, dirty, mode, command, status }
```

Then handlers would look like:

```opalescent
let next = propagate handle_event(state, event)
```

instead of returning ten labeled values.

A `match` form would make terminal event handling much cleaner:

```opalescent
match event:
    TerminalInputEvent.TextInput(text): ...
    TerminalInputEvent.Key(key): ...
    TerminalInputEvent.Resize(size): ...
```

A standard `int64_to_int32_checked` or cursor-position type would eliminate the
current cast warnings. Better yet, session cursor movement could use the same
integer width and coordinate convention as the rest of the terminal layout APIs.

A built-in test harness command for terminal fake backend scenarios would also
help. The existing environment-variable approach works, but a first-class
fixture format would make editor tests easier to read and repeat.

Finally, reliable project-level `opal check` would make iteration much faster.
The editor is intentionally multi-module now; the tooling should make that the
smooth path.

## What Opalescent Excelled At

Opalescent did well at making effects explicit. A reader can see that this
program touches the terminal, filesystem, layout engine, strings, and arrays
because those errors are all in the signatures.

The terminal trust boundary is a strong design. The editor cannot accidentally
write arbitrary untrusted strings directly to a session; it has to construct
`TrustedTerminalOutput`. That is slightly verbose, but it is the right default
for terminal applications.

The fake terminal backend is excellent for this kind of project. Interactive
terminal programs are normally difficult to test deterministically. Here, the
scripted event stream made a modal editing path reproducible under `timeout`.

The standard library had the right primitives at the right level. String range
editing, array insert/remove, atomic text writing, line splitting, terminal cell
clipping, and typed terminal events are exactly the pieces a small editor needs.

The language also stayed readable. Even with the workarounds, the modules are
plain text with explicit imports, explicit returns, and simple control flow.
There is very little hidden magic.

## Suggested Follow-Ups

The compiler should prioritize generated-code support for user ADTs across
module boundaries. That single improvement would remove the least maintainable
part of this editor.

The string ownership issue around returned label strings should be investigated.
Application authors should not need to defensively clone string literals to avoid
invalid frees.

The terminal session cursor API should be documented and, if possible, aligned
with the legacy zero-based helper behavior or renamed to make one-based positions
obvious.

A small terminal fixture runner would be valuable. Something that maps a script
file to `OPAL_TERMINAL_FAKE_EVENTS`, captures output, and checks a saved file
would make future editor work much cleaner.

The editor itself could eventually grow a real `EditorState` once ADT/codegen
support is ready. Until then, the current constant-based design is acceptable for
a fixture, but it should not be treated as the final idiom for Opalescent apps.

## Final Assessment

The simple editor is a useful stress test. It proves that Opalescent can now
build a deterministic terminal application that reads input, edits a buffer,
renders a screen, saves to disk, and exits cleanly. It also exposes the next set
of language priorities very clearly.

Opalescent is strongest when code can be written with primitive values, arrays,
explicit errors, and standard-library runtime handles. It becomes much less
smooth when an application wants rich domain state. For long-term maintainable
applications, robust ADTs, records, pattern matching, string ownership clarity,
and project-level checking are the features that would make the biggest
difference.
