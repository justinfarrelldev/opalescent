# Simple Neovim-like Editor Readiness Plan

Status: planning only. Do **not** commit this plan or any follow-up changes unless explicitly instructed.

This document is not an implementation patch. It is a readiness plan for the remaining work required before a very small Neovim-like terminal editor can be written and run as an Opalescent generated program.

## 1. Research summary

### 1.1 Repository/spec research already performed

Primary language and stdlib sources reviewed:

- `AGENTS.md`
- `README.md`
- `OPALESCENT_CRASH_COURSE.md`
- `STDLIB.md`
- all files under `language-spec/`
- `ARRAY_FEATURES.md`
- selected terminal-session proposal documents:
  - `stdlib-proposals/terminal-session-input/COMPARISON.md`
  - `stdlib-proposals/terminal-session-input/core-prerequisites.md`
  - `stdlib-proposals/terminal-session-input/typed-event-session/proposal.md`
  - `stdlib-proposals/terminal-session-input/typed-event-session/typed_event_session.types.op`
  - `stdlib-proposals/terminal-session-input/CHORDS.md`
  - `stdlib-proposals/terminal-session-input/terminal_chords.types.op`
  - `stdlib-proposals/terminal-session-input/TESTING.md`
  - `stdlib-proposals/terminal-session-input/terminal_testing.types.op`
- proposal workspace guidance:
  - `stdlib-proposals/README.md`
  - `stdlib-proposals/.template.proposal.md`
  - `stdlib-proposals/.comparison-schema.md`
  - `stdlib-proposals/.reference-patterns.md`
- relevant implementation status files:
  - `src/type_system/module_resolver/terminal_proposal_runtime_ready.rs`
  - `src/type_system/module_resolver/terminal_proposal_symbols.rs`
  - `src/type_system/module_resolver/terminal_proposal_modules.rs`
  - `src/type_system/module_resolver/terminal_proposal_borrows.rs`
  - `src/stdlib/terminal.rs`
  - `src/runtime/terminal/lifecycle.rs`
  - `src/runtime/terminal/model.rs`
  - `src/runtime/terminal/linux_backend.rs`
  - `src/runtime/terminal/windows_backend.rs`
  - `src/runtime/terminal/test_backend.rs`
  - `src/runtime/terminal/chords.rs`
  - `runtime/opal_terminal_model.c`
  - `runtime/opal_io.c`
  - `tests/integration_e2e/terminal_session_input_gated.rs`
  - `.sisyphus/evidence/terminal-session-input-traceability.md`

Important current facts:

1. The language can already express a simple editor architecture: explicit `entry main`, mutable bindings, loops, arrays, strings, `guard`/`propagate`, filesystem APIs, and generated native binaries.
2. Filesystem operations are close to sufficient for a first editor: `read_text_sync`, `read_lines_sync`, `write_text_sync`, `write_text_atomic_sync`, `string_split_lines`, and `string_join` exist and are documented.
3. Arrays support `.length`, `.at`, `.push`, `.pop`, indexed assignment, map/filter/reduce/zip, and flat 2D workarounds. Public line insertion/removal helpers are not documented in `STDLIB.md`.
4. The terminal-session proposal selected `typed-event-session` and Rust-side runtime support now exists for much of the model: lifecycle, one-event reads, pause, output trust, fake backend, Linux/Windows contract models, and chord router.
5. Generated-program lowering is still intentionally limited. `src/type_system/module_resolver/terminal_proposal_runtime_ready.rs` currently marks only the data-model/core-prerequisite subset as runtime-ready for terminal data-model work:
   - `terminal_session_options_default`
   - `terminal_session_options_with_feature_policy`
   - `terminal_session_options_with_resource_limits`
   - `terminal_session_options_validate`
   - `trusted_terminal_output_from_application_text`
   - core prerequisite functions from `standard.system`
6. The editor-critical generated-code calls are still gated or lack C ABI/runtime lowering:
   - `terminal_session_open_sync`
   - `terminal_session_read_event_sync`
   - `terminal_session_write_sync`
   - `terminal_session_flush_sync`
   - `terminal_session_close_sync`
   - `terminal_session_size_sync`
   - `terminal_session_set_cursor_visible_sync`
   - `terminal_session_set_cursor_shape_sync`
   - terminal chord router APIs
7. `runtime/opal_terminal_model.c` implements terminal options/constrained values/trusted-output data-model helpers, but not full terminal session lifecycle/input/output.
8. `runtime/opal_io.c` already contains legacy stdout/stdin coordinator checks and diagnostic-lane behavior, but this is not enough for a session-owned full-screen editor.
9. The 14 required terminal project fixtures exist, but active tests mostly verify fixture presence/static properties. The opt-in RED compile-gap probes remain ignored. They document that generated-program lowering is not complete.

### 1.2 External editor/terminal implementation research summary

Research on terminal text editors and terminal UI basics reinforces these requirements:

- A full-screen terminal editor must acquire raw input mode, disable canonical line input/echo, parse key input, and restore terminal state on every exit path.
- Editors normally use an alternate screen, hide/show cursor during redraw, clear/repaint rows, and position the cursor by terminal cells.
- Rendering should be batched into a frame/string to avoid flicker and excessive writes.
- Text editor cursor movement should distinguish bytes, Unicode scalar values, grapheme clusters, and terminal display cells.
- For a first editor, an array/list of lines is the simplest text buffer; gap buffers, piece tables, or ropes are later optimizations.
- A serious Unicode editor eventually needs grapheme cluster boundaries and display-width calculation. A simple v1 can explicitly scope itself to ASCII/single-cell text, but this must be documented and tested.

## 2. Target definition: “simple Neovim-like editor”

The first feasible target is not full Neovim. It is a tiny full-screen modal editor that can be implemented, tested, and maintained within Opalescent’s current language style once the required terminal generated-code work lands.

### 2.1 Required v1 behavior

- Accept a file path from command-line args, or start with one empty buffer if omitted.
- Read UTF-8 text from disk with explicit error handling.
- Store text as `string[]` lines.
- Show line numbers.
- Maintain:
  - buffer lines,
  - cursor line,
  - cursor column,
  - viewport top line,
  - dirty flag,
  - current mode: normal, insert, command/status.
- Scroll up/down when cursor moves past visible viewport.
- Handle at least these keys:
  - insert mode: committed `TextInput`, Enter, Backspace, Escape,
  - normal mode: arrow keys or `h/j/k/l`, `i`, `:`, `q`/`:q`, `:w`, `:wq`, maybe `Ctrl-S`,
  - optional first chord layer: `gg`, `G`, `dd` only after chord router lowering works.
- Save with atomic text write where possible.
- Display a status line with file path, dirty state, mode, and short message.
- Emit an invalid-key feedback event (“chime”) either as:
  - a visible status-line message for v1, and/or
  - a session-safe terminal bell once a bell/chime API is proposed and implemented.
- Close and restore terminal state cleanly.

### 2.2 Explicit non-goals for the first editor

- No syntax highlighting.
- No search/replace.
- No undo/redo.
- No multiple buffers/windows/tabs.
- No mouse support.
- No plugin/runtime config.
- No huge-file/rope support.
- No complete Unicode terminal-cell correctness unless the text-layout proposal below lands first.

## 3. Proposal work required before implementation

Before implementing new public stdlib surface, draft the required stdlib proposals. Follow `stdlib-proposals/.template.proposal.md`, `stdlib-proposals/.comparison-schema.md`, and `stdlib-proposals/README.md` style rules.

### 3.1 Proposal A: session-owned terminal rendering helpers

Create a new concern, for example:

```text
stdlib-proposals/terminal-session-rendering/
    COMPARISON.md
    high-level-session-rendering/proposal.md
    high-level-session-rendering/terminal_rendering.types.op
    high-level-session-rendering/simple_editor_frame.op
    trusted-ansi-builder/proposal.md
    trusted-ansi-builder/terminal_ansi_builder.types.op
    trusted-ansi-builder/simple_editor_frame.op
```

Problem to solve:

- Existing legacy helpers like `terminal_clear_screen_sync` and `terminal_move_cursor_sync` operate through legacy stdout terminal handles. They are rejected while a terminal session owns the coordinator.
- The selected session write API accepts only `TrustedTerminalOutput` or `SafeTerminalDiagnosticOutput`.
- A simple editor needs clear screen/home cursor/draw rows/move cursor/bell without a raw stdout bypass.

Alternatives to compare:

1. **High-level session rendering operations**
   - `terminal_session_clear_screen_sync(ref session)`
   - `terminal_session_move_cursor_sync(ref session, row, column)`
   - `terminal_session_draw_rows_sync(ref session, rows: TrustedTerminalOutput[])`
   - `terminal_session_bell_sync(ref session)` or `terminal_session_chime_sync(ref session, TerminalChime)`
2. **Trusted ANSI builder**
   - pure helpers return `TrustedTerminalOutput` for clear/home/move/alternate-screen/bell sequences;
   - caller writes them through `terminal_session_write_sync`.
3. **Frame/canvas object**
   - application builds `TerminalFrame`, sets rows/cursor/status, then commits one frame.

Selection criteria:

- Must preserve selected terminal output trust boundary.
- Must not reintroduce `terminal_session_output_terminal` or `AcquireOutputTerminal`.
- Must work with fake backend tests.
- Must be async/deferred-ready later.
- Must not require terminal raw string writes from untrusted input.

Likely v1 recommendation:

- Start with **trusted ANSI builder + minimal session convenience helpers**.
- Keep the high-level output path explicit: application creates reviewed/trusted frame text, then writes via session.
- Add a dedicated `terminal_session_bell_sync` or `trusted_terminal_output_bell()` only if the “chime” requirement really means audible/visible bell; otherwise use status-line messages first.

Red fixtures required:

- `terminal-session-render-clear-home`
- `terminal-session-render-cursor-position`
- `terminal-session-render-bell-or-status-chime`
- compile-fail/security test: direct raw `string` to session render remains rejected.

### 3.2 Proposal B: string editing primitives

Create a new concern, for example:

```text
stdlib-proposals/string-editing-primitives/
    COMPARISON.md
    scalar-range-functions/proposal.md
    scalar-range-functions/string_editing.op
    grapheme-aware-functions/proposal.md
    grapheme-aware-functions/string_grapheme_editing.op
    line-buffer-object/proposal.md
    line-buffer-object/text_buffer.types.op
    line-buffer-object/simple_editor_buffer.op
```

Problem to solve:

- Existing strings have scalar `.length`, `.at`, `string_extract_range`, `string_take_prefix`, `string_take_suffix`, `string_split_lines`, `string_join`, and `StringBuilder`.
- A text editor needs frequent insert/delete/split/join operations.
- App-local implementation is possible but noisy and error-prone.

Alternatives:

1. **Scalar range functions**
   - `string_insert_at(text, scalar_index, inserted): string errors StringRangeError, AllocationFailureError`
   - `string_delete_range(text, start, end): string errors StringRangeError, AllocationFailureError`
   - `string_replace_range(text, start, end, replacement): string errors StringRangeError, AllocationFailureError`
2. **Grapheme-aware functions**
   - `string_grapheme_length`
   - `string_grapheme_extract_range`
   - `string_grapheme_insert_at`
   - `string_grapheme_delete_range`
3. **Line-buffer object**
   - `TextBuffer` affine or immutable object with insert/delete/split/join.

Likely v1 recommendation:

- Implement scalar range functions first because they align with current public string scalar semantics.
- Explicitly document that simple editor v1 edits by Unicode scalar index, not grapheme cluster.
- Draft grapheme-aware proposal as future required work for non-ASCII-correct editor behavior.

Red fixtures required:

- `string-edit-insert-delete-ascii`
- `string-edit-split-join-line`
- `string-edit-range-errors`
- `string-edit-allocation-error-contract`

### 3.3 Proposal C: terminal text layout/display-width helpers

Create a new concern, for example:

```text
stdlib-proposals/terminal-text-layout/
    COMPARISON.md
    scalar-width-baseline/proposal.md
    scalar-width-baseline/layout.op
    unicode-grapheme-cell-width/proposal.md
    unicode-grapheme-cell-width/layout.op
```

Problem to solve:

- Terminal cursor positions are display cells, not bytes or Unicode scalars.
- Line numbers and scrolling require stable cell clipping.
- Wide CJK/emoji and combining marks will make a naive scalar-column editor visually wrong.

Alternatives:

1. **ASCII/single-cell baseline**
   - Documented limitation for first editor.
   - Helpers may reject/escape non-ASCII.
2. **Unicode scalar width**
   - `terminal_scalar_display_width(scalar_text: string): int32`
   - limited combining handling.
3. **Grapheme cell width**
   - `terminal_text_cell_width(text: string): int64`
   - `terminal_text_clip_to_cells(text, max_cells): clipped: string, used_cells: int64 errors ...`
   - follows Unicode grapheme and East Asian Width/wcwidth-like behavior.

Likely v1 recommendation:

- Permit editor v1 with ASCII/single-cell limitation if explicitly tested.
- Draft the Unicode/grapheme proposal before claiming general text-editor correctness.

Red fixtures required:

- `terminal-layout-ascii-clip`
- `terminal-layout-wide-char-known-limitation` or future `terminal-layout-wide-char-width`
- `terminal-editor-ascii-mode-rejects-or-escapes-wide-input` if v1 remains ASCII-limited.

### 3.4 Proposal D: array/line collection mutation helpers

Create or extend a collections concern, for example:

```text
stdlib-proposals/collections-editing/
    COMPARISON.md
    method-style-array-editing/proposal.md
    method-style-array-editing/array_editing.op
    free-function-array-editing/proposal.md
    free-function-array-editing/array_editing.op
```

Problem to solve:

- Editor lines are naturally `string[]`.
- Current STDLIB documents `.push`, `.pop`, `.at`, and helpers, but not public `insert`/`remove_at`.
- The Rust `OpalVec<T>` has `insert`/`remove`, but generated Opalescent public surface and docs do not expose stable signatures.

Possible functions:

- `array_insert<T>(values: T[], index: int64, value: T): T[] errors IndexOutOfBoundsError, AllocationFailureError`
- `array_remove_at<T>(values: T[], index: int64): updated: T[], removed: T errors IndexOutOfBoundsError, AllocationFailureError`
- method forms: `values.insert(index, value)`, `values.remove_at(index)`

Likely v1 recommendation:

- Use app-local rebuild loops for the very first editor only if needed.
- Draft and implement stable helpers before promoting editor examples as idiomatic.

Red fixtures required:

- `array-insert-remove-lines`
- `array-insert-remove-errors`
- `editor-line-split-uses-array-insert`

### 3.5 Proposal E: generated terminal testing/fixture activation update

This is partly an update to the existing terminal-session-input proposal rather than a wholly new concern.

Problem to solve:

- Existing fake backend/test-only terminal support is Rust-side and proposal-defined, but generated `test-projects/terminal-*` fixtures are mostly static/ignored compile-gap checks.
- A real editor fixture needs deterministic terminal input/output in generated-code integration tests.

Required proposal amendments or companion proposal:

- Clarify how generated Opalescent test artifacts import and use `standard.testing.terminal`.
- Specify how the integration runner injects deterministic terminal events into generated programs.
- Specify expected stdout/stderr/exit assertions after terminal close.
- Preserve test-only availability and prevent production import leakage.

Red fixtures required:

- activate existing 14 terminal fixtures as real compile/run tests, not only static source checks.
- add `terminal-simple-editor-edit-save` fixture once terminal generated-code support lands.

## 4. Core implementation phases

Every implementation phase must be TDD: write failing tests first, make the minimal green implementation, then refactor.

### Phase 1: normalize selected terminal imports and documentation

Goal:

- Make the public import story coherent before deeper codegen work.

Tasks:

1. Decide canonical import path:
   - likely `standard.terminal` for selected terminal session/chord APIs,
   - `standard.system` for wait/timer/cancellation/process-control,
   - `standard.testing.terminal` for test-only factories.
2. Update proposal examples and terminal fixture sources to use the canonical modules.
3. Update `STDLIB.md` terminal-session section to distinguish:
   - Rust/runtime model support,
   - generated-program runtime-ready subset,
   - remaining C ABI/codegen gap.
4. Update `README.md` if status wording changes.

Red tests:

- `opal check test-projects/terminal-key-log/src/main.op` should first fail for current gap, but no longer due to wrong module path after fixes.
- Add a type-system test that importing terminal lifecycle symbols from `standard` fails with a helpful suggestion to use `standard.terminal`.
- Add a test that importing from `standard.terminal` succeeds through type check once gate/prerequisites permit it.

Definition of done:

- No fixture uses stale `from standard` terminal-session imports unless intentionally testing legacy APIs.
- Docs and tests agree on module paths.

### Phase 2: design and implement the generated terminal C ABI

Goal:

- Generated Opalescent binaries can own a terminal session and exchange events/output with the runtime.

Major design decision:

- The current generated runtime is C (`runtime/*.c`). Rust runtime models in `src/runtime/terminal/*` are compiler/test-side support, not automatically linked into generated programs.
- Choose one strategy:
  1. port the selected terminal runtime subset into C, or
  2. build/link a Rust static library with C ABI exports into generated programs.

Plan recommendation:

- Prefer C runtime implementation for the first generated-program milestone because existing generated binaries already link C runtime files and `runtime/opal_terminal_model.c` contains terminal data-model ABI work.
- Keep Rust runtime tests as the semantic oracle.

Required ABI surfaces:

- Opaque pointer types:
  - `TerminalSession`
  - `TerminalCapabilities`
  - `TerminalInputEvent`
  - `TerminalPauseEvents`
  - `TrustedTerminalOutput`
  - diagnostics/errors as needed.
- Result wrappers:
  - pointer + error for handle/object-returning functions,
  - void + error for void functions,
  - integer + error where applicable.
- Event representation:
  - stable tag accessor or compiler-known struct/tag layout,
  - payload accessors for every editor-relevant variant:
    - Key,
    - TextInput,
    - Resize,
    - TimedOut,
    - Cancelled,
    - EndOfInput,
    - InputReset,
    - UnknownBytes/Paste for quarantine tests.
- Nested payload access:
  - `TerminalLogicalKey.Named`, `TerminalLogicalKey.Text`, etc.
  - modifiers fields.
- Error identity mapping:
  - preserve declared families `TerminalSessionOpenError`, `TerminalSessionReadError`, `TerminalSessionWriteError`, `TerminalSessionStateError`, `TerminalSessionRestoreError`.

Red tests:

- Convert `terminal_session_input_gated_selected_api_red` from ignored opt-in to active red.
- Add minimal generated source:
  - open session,
  - read one deterministic event,
  - write trusted summary,
  - close session.
- Assert it fails before ABI implementation, then green after.

Definition of done:

- A generated Opalescent program can call open/read/write/flush/close without runtime-readiness gate errors.
- Runtime restores coordinator state after close.
- Legacy I/O rejection still works while active.

### Phase 3: lower selected terminal lifecycle calls in codegen

Goal:

- Remove lifecycle/read/write symbols from generated-code gate only after the C ABI exists.

Tasks:

1. Extend `terminal_proposal_runtime_ready.rs` inventory.
2. Extend `functions_stdlib` declarations for selected terminal functions.
3. Ensure `resolve_imported_runtime_name` supports `standard.terminal` lifecycle names.
4. Lower borrowed parameters correctly:
   - `ref session` vs `mutable ref session`,
   - cancellation token refs,
   - trusted output refs.
5. Ensure fallible return wrapper handling maps to existing `propagate`/`guard` machinery.
6. Ensure cleanup lowering for `using session = propagate terminal_session_open_sync(options):` calls `terminal_session_close_sync` exactly once.

Red tests:

- Codegen tests that currently expect lifecycle gate diagnostics should be inverted only when ABI exists.
- New generated IR tests for declarations:
  - `terminal_session_open_sync`
  - `terminal_session_read_event_sync`
  - `terminal_session_write_sync`
  - `terminal_session_flush_sync`
  - `terminal_session_close_sync`
- Runtime integration fixture `terminal-key-log` compiles and runs under deterministic fake backend.

Definition of done:

- All 14 terminal fixtures can be made active compile/run tests one by one.

### Phase 4: implement generated-code ADT refinement/payload access for terminal events

Goal:

- Editor source can write idiomatic code such as:

```opal
if event is TerminalInputEvent.Key:
    if event.key is TerminalLogicalKey.Named:
        if event.key.key is TerminalNamedKey.ArrowDown:
            # move cursor
```

Tasks:

1. Verify parser/type checker already supports one-`is` refinement and branch-local payload access for proposal ADTs.
2. Fill any codegen gaps for nested sum variants and payload fields.
3. Ensure payloadless variants (`TimedOut`, `Cancelled`, `EndOfInput`) lower correctly.
4. Ensure non-exhaustive variants are safe to ignore.
5. Ensure unknown variants do not crash generated programs.

Red tests:

- Generated program handles:
  - `TerminalInputEvent.TextInput`
  - `TerminalInputEvent.Key` with `TerminalLogicalKey.Named`
  - `TerminalInputEvent.Key` with `TerminalLogicalKey.Text`
  - `TerminalInputEvent.Resize`
  - terminal status events.
- Compile-fail tests reject invalid `into` usage on equality or payloadless variants.

Definition of done:

- Editor key loop can be expressed in normal Opalescent without private runtime helpers.

### Phase 5: activate deterministic generated terminal test backend

Goal:

- Test projects can run with scripted input plans and expected output.

Tasks:

1. Decide whether to expose `standard.testing.terminal` to generated test artifacts now or to let the Rust integration harness inject backend state externally.
2. Implement enough test-only C ABI or harness-side runtime injection to run terminal fixtures.
3. Enforce test-only availability: production builds cannot import test-only symbols.
4. Convert terminal fixtures from static source checks to real compile/run checks.

Red tests:

- `terminal-key-log` real run.
- `terminal-text-echo-safe` real run.
- `terminal-size-probe` real run.
- Continue fixture activation in small groups.

Definition of done:

- Existing 14 fixtures run deterministically in normal or clearly documented feature-gated integration mode.
- Security tests still reject direct string writes and test-only symbol leakage.

### Phase 6: implement session rendering/chime surface from Proposal A

Goal:

- Editor can clear/redraw/move cursor/chime without raw legacy stdout.

Tasks after proposal acceptance:

1. Implement chosen rendering APIs in Rust model and C runtime.
2. Add codegen declarations and runtime-ready inventory.
3. Add fake-backend output assertions.
4. Update `STDLIB.md`.

Red tests:

- Render clear/home and cursor movement in a generated fixture.
- Ensure no legacy stdout handle is used while session active.
- Chime test:
  - if audible BEL: output contains safe runtime-controlled bell/chime marker in fake backend,
  - if status-only: invalid command updates status line.

Definition of done:

- Editor redraw can be one trusted frame per event plus final cursor positioning.

### Phase 7: implement string/array editing primitives or app-local baseline

Goal:

- Provide reliable buffer editing operations.

Minimum app-local baseline:

- Use `string_extract_range` and `string_builder` to implement:
  - insert at scalar index,
  - delete previous scalar,
  - delete current scalar,
  - split line,
  - join line.
- Use array rebuild loops for insert/remove line if public helpers are not ready.

Preferred stdlib implementation after proposals:

- `string_insert_at`
- `string_delete_range`
- `string_replace_range`
- `array_insert`
- `array_remove_at`

Red tests:

- In-memory editor-buffer unit fixtures as Opalescent projects:
  - insert characters,
  - enter splits line,
  - backspace at start joins with previous line,
  - save joins lines with `\n`.

Definition of done:

- Buffer editing works before terminal UI integration.

### Phase 8: build the minimal editor fixture first

Create:

```text
test-projects/terminal-simple-editor/
    opal.toml
    src/main.op
    fixtures/input.txt
```

Start with a deterministic scripted test, not a freeform interactive test.

Scenario 1: open/edit/save/quit.

Input file:

```text
alpha
beta
```

Scripted input:

1. open file,
2. go to second line,
3. enter insert mode,
4. type `!`,
5. save,
6. quit.

Expected output summary after terminal close:

```text
EDITOR_SUMMARY opened=true saved=true dirty=false lines=2 cursor=1,5 viewport=0 status=ok
```

Expected saved file:

```text
alpha
beta!
```

Tasks:

1. Write fixture source and expected integration harness first; keep it red.
2. Implement app-level editor state:
   - `EditorMode` type,
   - `EditorState` type,
   - file path,
   - lines,
   - cursor,
   - viewport,
   - dirty flag,
   - status message.
3. Implement pure buffer operations in separate local module.
4. Implement event handling in separate local module.
5. Implement rendering in separate local module.
6. Use terminal session `using` cleanup once generated support is ready.
7. Use `write_text_atomic_sync` for save.

Definition of done:

- The fixture compiles/runs deterministically.
- Output summary and saved file match expected assertions.
- No direct legacy stdout terminal API use while session active.

### Phase 9: add line numbers, scrolling, and command mode

Goal:

- Reach the user-requested simple Neovim-like behavior.

Tasks:

1. Render line numbers with a separator.
2. Maintain vertical scrolling:
   - cursor below viewport => increment viewport,
   - cursor above viewport => decrement viewport.
3. Add normal mode movement:
   - arrows,
   - `h/j/k/l` if Text key identity is available.
4. Add insert mode:
   - `i` to enter,
   - Escape to leave.
5. Add command mode:
   - `:` enters command buffer,
   - `w` save,
   - `q` quit if not dirty,
   - `q!` quit discarding changes,
   - `wq` save and quit.
6. Add invalid-command chime/status message.

Red fixtures:

- `terminal-simple-editor-scroll-lines`
- `terminal-simple-editor-command-save-quit`
- `terminal-simple-editor-dirty-quit-blocked`
- `terminal-simple-editor-invalid-command-chime`

Definition of done:

- Reading, writing, editing, line numbers, scrolling up/down, and saving are covered.

### Phase 10: optional chord-router integration

Goal:

- Neovim-like multi-key command handling once chord router lowering is generated-program-ready.

Tasks:

1. Lower `standard.terminal.chords` functions into generated programs.
2. Implement side-map pattern for command payloads outside router.
3. Register basic chords:
   - `Ctrl-S` save,
   - `Ctrl-Q` quit,
   - Escape leave insert/command,
   - maybe `gg`, `G`, `dd` in later fixture.
4. Ensure non-command input is released and applied exactly once.

Red fixtures:

- `terminal-simple-editor-ctrl-s-save`
- `terminal-simple-editor-escape-mode`
- `terminal-simple-editor-released-input-on-non-command`

Definition of done:

- Chord handling works without callbacks or command payloads stored in router.

### Phase 11: Unicode correctness follow-up

Goal:

- Move beyond ASCII/single-cell limitation.

Tasks:

1. Implement accepted `terminal-text-layout` proposal.
2. Replace scalar-column cursor math with grapheme/cell-aware math.
3. Add explicit tests for:
   - combining marks,
   - emoji/wide glyphs,
   - clipping line numbers/status rows,
   - backspace over grapheme cluster.

Definition of done:

- Editor does not corrupt UTF-8 and cursor/rendering remains consistent for tested Unicode classes.

## 5. Minimal dependency chain

The shortest chain to make the simple editor possible is:

1. Canonicalize terminal module imports/docs.
2. Implement generated terminal C ABI for open/read/write/flush/close/size.
3. Lower terminal lifecycle APIs in codegen.
4. Lower terminal event ADTs/refinement/payload access.
5. Activate deterministic terminal generated-code fixture runs.
6. Add session-safe render/chime API from a drafted/accepted stdlib proposal, or explicitly use a reviewed trusted frame builder.
7. Implement minimal buffer editing helpers locally or via drafted string/array proposals.
8. Write `terminal-simple-editor` fixture red first, then implement.

## 6. Items probably not required for the first simple editor

These can stay out of the initial scope:

- regex,
- networking/HTTP,
- subprocess execution,
- package manager,
- LSP completeness,
- hot reload,
- syntax highlighting,
- config files/serialization,
- plugin architecture,
- ropes/piece tables,
- mouse support,
- complete Unicode grapheme/cell-width correctness if the v1 editor is explicitly ASCII/single-cell scoped.

## 7. Major risks

1. **C ABI size and ADT lowering complexity**
   - Terminal events are nested ADTs with many variants. The editor only needs a subset, but the public ABI must remain coherent.
2. **Runtime ownership/restoration bugs**
   - A broken terminal restore path is user-hostile. Keep fake-backend and host smoke tests separate and strict.
3. **Output trust boundary regressions**
   - Do not allow direct `string` writes to session output.
4. **Import namespace drift**
   - Fix before writing more fixtures.
5. **Unicode scope creep**
   - Decide whether v1 is ASCII/single-cell. If not, draft and implement layout primitives before editor UI.
6. **Fixture overclaiming**
   - Static fixture presence is not enough. The simple editor must compile and run as generated Opalescent.

## 8. Final acceptance criteria

The simple Neovim-like editor is “possible” only when all of the following are true:

- A generated Opalescent program can open a terminal session, read typed events, write trusted output, flush, and close.
- Existing terminal fixtures are real generated compile/run tests or there is a clearly documented feature-gated deterministic terminal test mode.
- A session-safe rendering/chime API is proposed, accepted, documented, and implemented, or the editor uses an accepted trusted frame-builder proposal.
- Buffer editing helpers exist either locally in the editor fixture or as accepted stdlib string/array helpers.
- `test-projects/terminal-simple-editor` demonstrates:
  - file read,
  - edit,
  - line numbers,
  - scrolling,
  - save,
  - quit,
  - deterministic summary,
  - terminal cleanup.
- `STDLIB.md`, `README.md`, and `OPALESCENT_CRASH_COURSE.md` accurately describe the public terminal/editor-relevant surface without claiming unsupported behavior.
