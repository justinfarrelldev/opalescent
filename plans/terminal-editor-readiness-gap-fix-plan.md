# Terminal Editor Readiness Gap Fix Plan

Branch: `fix-terminal-editor-readiness-gaps`

Status: implementation plan. Follow red-green-refactor and make atomic commits. Do not add the simple Neovim editor fixture/application in this plan.

## Objective

Close the remaining terminal/editor-readiness gaps identified after repository research, excluding the actual `terminal-simple-editor` application fixture:

1. Turn the existing 14 terminal proposal fixtures from stale/static/ignored compile-gap artifacts into meaningful generated compile/run coverage where practical.
2. Add generated-program chord-router support.
3. Expand the generated fake terminal backend event DSL enough to exercise the 14 fixtures and future editor tests deterministically.
4. Add generated runtime readiness/lowering for pause/resume and diagnostic terminal APIs required by the existing fixtures.
5. Improve real terminal UTF-8 text input handling beyond single-byte text events.
6. Update public docs/status to reflect the actual implemented generated-terminal surface.

## Non-goals

- Do not create or implement `test-projects/terminal-simple-editor`.
- Do not add syntax highlighting, search, undo, ropes, plugin/config systems, or editor-specific application code.
- Do not bypass or modify pre-commit hooks.
- Do not remove terminal security/trust boundaries; direct raw `string` session writes must remain rejected.

## Research baseline

Relevant current facts:

- Targeted generated terminal tests under `tests/integration_e2e/terminal_generated_testing.rs` pass for open/read/write/flush/close, size/capabilities, high-level rendering, typed event payload access, fake backend injection, production open, and `using` cleanup.
- `tests/integration_e2e/terminal_session_input_gated.rs` still contains ignored compile-gap probes and fixture metadata for 14 terminal proposal fixtures.
- Several fixture sources are stale: they omit terminal type imports and use old non-`ref` call shapes for borrowed terminal APIs.
- `src/type_system/module_resolver/terminal_proposal_runtime_ready.rs` and `src/codegen/functions_stdlib_terminal_session.rs` include session lifecycle/rendering but not chord-router functions, pause/resume, or diagnostics.
- `runtime/opal_io.c` contains generated C terminal session support and a small fake-event parser, but lacks chord-router C ABI and only performs byte-at-a-time real text input.
- `README.md`/`STDLIB.md` still contain stale terminal status language.

## Phase 1: fixture activation and stale-source cleanup

Goal: make the existing 14 terminal proposal fixtures compile/run deterministically or split unsupported future-scope fixtures with explicit active tests proving the boundary.

TDD steps:

1. RED: replace or supplement ignored compile-gap probes with active generated compile/run tests for the smallest fixture group.
2. RED: assert each fixture imports required terminal types from `standard` or `standard.terminal` and uses required `ref`/`mutable ref` borrow syntax.
3. GREEN: update fixture imports and call shapes only; do not change fixture behavior beyond current selected APIs.
4. GREEN: add per-fixture fake event plans and expected output assertions for fixtures that can run after current/next phases.
5. REFACTOR: remove obsolete ignored compile-gap wording once tests are active or rename remaining future-scope tests to reflect actual unsupported features.

Expected files:

- `tests/integration_e2e/terminal_session_input_gated.rs`
- `test-projects/terminal-*/src/main.op`

## Phase 2: expand fake backend event DSL

Goal: generated terminal fixtures can receive deterministic rich events without production-only test symbol leakage.

TDD steps:

1. RED: add generated integration tests for fake specs covering text, named keys, text/control chord-relevant keys, modifiers, paste, unknown bytes/native, input reset reasons, timeout, cancelled, resize, and eof.
2. GREEN: extend `runtime/opal_io.c` fake parser and event constructors to produce the relevant `TerminalInputEvent` variants/payloads.
3. GREEN: keep simple existing specs backward compatible.
4. REFACTOR: centralize parser helpers and keep allocation/error behavior consistent.

Expected files:

- `runtime/opal_io.c`
- `tests/integration_e2e/terminal_generated_testing.rs`
- possibly `src/codegen` tests if payload access uncovers lowering gaps.

## Phase 3: generated pause/resume and diagnostics APIs

Goal: generated programs can use terminal pause/resume and diagnostic inspector APIs required by existing terminal fixtures.

TDD steps:

1. RED: add compile/run tests for `terminal_session_pause_sync`, `terminal_session_resume_sync`, `terminal_pause_events_length`, `terminal_pause_events_at`, and selected diagnostic accessors/formatters.
2. GREEN: add runtime-readiness inventory entries and codegen declarations.
3. GREEN: implement minimal C runtime ABI in `runtime/opal_io.c` sufficient for deterministic fixture behavior.
4. REFACTOR: align type metadata, runtime error names, and docs.

Expected files:

- `src/type_system/module_resolver/terminal_proposal_runtime_ready.rs`
- `src/codegen/functions_stdlib_terminal_session.rs`
- `runtime/opal_io.c`
- `tests/integration_e2e/terminal_generated_testing.rs`

## Phase 4: generated chord-router support

Goal: `terminal-chord-quit` and editor-style chord tests can compile/run as generated programs.

TDD steps:

1. RED: add generated integration test for a minimal Ctrl-Q/Escape chord-router flow using fake events.
2. RED: compile/run `test-projects/terminal-chord-quit` with deterministic fake events and expected summary.
3. GREEN: add chord runtime-readiness inventory and codegen declarations for selected `terminal_chord_*` APIs.
4. GREEN: implement C runtime ABI for minimal chord construction, sequence, router registration, process, expire/reset, released-input access, and binding id ordinal.
5. REFACTOR: keep Rust model and C behavior semantically aligned for the tested subset.

Expected files:

- `src/type_system/module_resolver/terminal_proposal_runtime_ready.rs`
- `src/codegen/functions_stdlib_terminal_session.rs` or new chord-specific codegen module
- `runtime/opal_io.c`
- `tests/integration_e2e/terminal_generated_testing.rs`
- `tests/integration_e2e/terminal_session_input_gated.rs`

## Phase 5: real terminal UTF-8 input

Goal: real terminal input should produce valid UTF-8 text events for multibyte typed text instead of one byte per text event.

TDD steps:

1. RED: add unit-level C/Rust-facing parser tests where possible, or generated fake/real helper tests if exposed through the harness, for UTF-8 sequences.
2. GREEN: implement a small UTF-8 decoder around the existing byte reader in `runtime/opal_io.c` for real terminal text input.
3. GREEN: malformed sequences should produce quarantine/unknown input behavior rather than invalid text strings.
4. REFACTOR: keep escape-sequence parsing for arrows/Escape working.

Expected files:

- `runtime/opal_io.c`
- tests appropriate to the available harness.

## Phase 6: docs/status refresh

Goal: docs describe the current terminal/editor-relevant implementation accurately.

TDD/documentation steps:

1. RED: add or update a docs/status consistency test if an existing pattern is available; otherwise document manual review in checklist.
2. GREEN: update `README.md`, `STDLIB.md`, and possibly `OPALESCENT_CRASH_COURSE.md` terminal sections.
3. REFACTOR: remove stale fake-backend-only/generated-gap claims.

Expected files:

- `README.md`
- `STDLIB.md`
- `OPALESCENT_CRASH_COURSE.md` if needed.

## Phase 7: final verification and atomic commits

Run focused tests after each phase and broader touched-subsystem tests before completion. Each phase should be committed atomically after green/refactor.

Suggested focused commands:

```bash
cargo test terminal_generated_testing:: --features integration --test integration_e2e -- --nocapture
cargo test terminal_session_input_gated:: --features integration --test integration_e2e -- --nocapture
cargo test terminal_stdlib:: --features integration --test integration_e2e -- --nocapture
cargo test terminal_ --lib -- --nocapture
```

Final acceptance:

- No ignored compile-gap tests remain for features implemented in this plan.
- The 14 existing terminal fixtures either compile/run deterministically or are explicitly and narrowly marked future-scope with active tests proving that boundary.
- Chord-router generated support works for the tested editor-relevant subset.
- Fake backend can express the fixture/event scenarios needed by the 14 terminal fixtures.
- Pause/resume and diagnostics generated tests pass for the fixture-required subset.
- Real terminal UTF-8 input no longer emits invalid one-byte text for multibyte input.
- Docs match implementation status.
