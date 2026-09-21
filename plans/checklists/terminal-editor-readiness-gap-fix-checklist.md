# Terminal Editor Readiness Gap Fix Checklist

Branch: `fix-terminal-editor-readiness-gaps`

Keep this checklist up to date during implementation.

## Setup

- [x] Created branch `fix-terminal-editor-readiness-gaps` from current `main`.
- [x] Created implementation plan in `plans/terminal-editor-readiness-gap-fix-plan.md`.
- [x] Created this checklist.

## Phase 1: fixture activation and stale-source cleanup

- [ ] RED: add/activate tests that prove current 14 terminal fixture sources are stale or not real compile/run coverage.
- [ ] GREEN: fix terminal fixture imports and required `ref`/`mutable ref` call shapes.
- [ ] GREEN: add deterministic compile/run assertions for fixtures supported by current and subsequent phases.
- [ ] REFACTOR: remove stale ignored compile-gap wording or narrow remaining future-scope markers.
- [ ] COMMIT: atomic fixture activation/source cleanup commit.

## Phase 2: fake backend event DSL expansion

- [ ] RED: add generated fake-event tests for rich event specs.
- [ ] GREEN: implement fake parser/events for named/text/control keys, modifiers, paste, unknown bytes/native, input reset reasons, timeout, cancelled, resize, eof.
- [ ] GREEN: preserve backward-compatible existing fake specs.
- [ ] REFACTOR: centralize parser helpers and allocation/error behavior.
- [ ] COMMIT: atomic fake backend DSL commit.

## Phase 3: pause/resume and diagnostics generated APIs

- [ ] RED: add compile/run tests for pause/resume and selected diagnostic APIs.
- [ ] GREEN: add runtime-ready inventory entries and codegen declarations.
- [ ] GREEN: implement C runtime ABI support for deterministic fixture behavior.
- [ ] REFACTOR: align error names/type metadata/docs.
- [ ] COMMIT: atomic pause/resume/diagnostics commit.

## Phase 4: generated chord-router support

- [ ] RED: add generated chord-router integration test.
- [ ] RED: run/activate `terminal-chord-quit` expected-summary test.
- [ ] GREEN: add chord runtime-readiness and codegen declarations.
- [ ] GREEN: implement C runtime ABI for editor-relevant chord-router subset.
- [ ] REFACTOR: align tested Rust/C chord semantics.
- [ ] COMMIT: atomic chord-router generated support commit.

## Phase 5: real terminal UTF-8 input

- [ ] RED: add test/probe demonstrating current byte-at-a-time UTF-8 input gap.
- [ ] GREEN: implement UTF-8 decoding for real terminal text input.
- [ ] GREEN: handle malformed sequences through quarantine/unknown behavior, not invalid text.
- [ ] REFACTOR: preserve Escape/arrow handling.
- [ ] COMMIT: atomic UTF-8 input commit.

## Phase 6: docs/status refresh

- [ ] RED: add/update docs consistency test if practical, or record manual doc mismatch evidence.
- [ ] GREEN: update `README.md` terminal status.
- [ ] GREEN: update `STDLIB.md` terminal session status.
- [ ] GREEN: update `OPALESCENT_CRASH_COURSE.md` if needed.
- [ ] REFACTOR: remove stale fake-backend-only/generated-gap claims.
- [ ] COMMIT: atomic docs/status commit.

## Final verification

- [ ] Run `cargo test terminal_generated_testing:: --features integration --test integration_e2e -- --nocapture`.
- [ ] Run `cargo test terminal_session_input_gated:: --features integration --test integration_e2e -- --nocapture`.
- [ ] Run focused lib/runtime terminal tests.
- [ ] Run broader touched-subsystem tests or document blockers.
- [ ] Ensure no simple Neovim editor fixture/application code was added.
- [ ] Ensure checklist is accurate.
- [ ] Ensure all commits are atomic and pre-commit hook was not modified or bypassed.
