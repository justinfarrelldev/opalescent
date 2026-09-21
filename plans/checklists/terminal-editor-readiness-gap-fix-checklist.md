# Terminal Editor Readiness Gap Fix Checklist

Branch: `fix-terminal-editor-readiness-gaps`

Keep this checklist up to date during implementation.

## Setup

- [x] Created branch `fix-terminal-editor-readiness-gaps` from current `main`.
- [x] Created implementation plan in `plans/terminal-editor-readiness-gap-fix-plan.md`.
- [x] Created this checklist.

## Phase 1: fixture activation and stale-source cleanup

- [x] RED: add/activate tests that prove current 14 terminal fixture sources are stale or not real compile/run coverage.
- [x] GREEN: fix terminal fixture imports and required `ref`/`mutable ref` call shapes.
- [x] GREEN: add deterministic compile/run assertions for fixtures supported by current and subsequent phases.
- [x] REFACTOR: remove stale ignored compile-gap wording or narrow remaining future-scope markers.
- [ ] COMMIT: atomic fixture activation/source cleanup commit.

## Phase 2: fake backend event DSL expansion

- [x] RED: add generated fake-event tests for rich event specs.
- [x] GREEN: implement fake parser/events for named/text/control keys, modifiers, paste, unknown bytes/native, input reset reasons, timeout, cancelled, resize, eof.
- [x] GREEN: preserve backward-compatible existing fake specs.
- [x] REFACTOR: centralize parser helpers and allocation/error behavior.
- [ ] COMMIT: atomic fake backend DSL commit.

## Phase 3: pause/resume and diagnostics generated APIs

- [x] RED: add compile/run tests for pause/resume and selected diagnostic APIs.
- [x] GREEN: add runtime-ready inventory entries and codegen declarations.
- [x] GREEN: implement C runtime ABI support for deterministic fixture behavior.
- [x] REFACTOR: align error names/type metadata/docs.
- [ ] COMMIT: atomic pause/resume/diagnostics commit.

## Phase 4: generated chord-router support

- [x] RED: add generated chord-router integration test coverage through fixture/readiness paths.
- [x] RED: run/activate `terminal-chord-quit` expected-summary test.
- [x] GREEN: add chord runtime-readiness and codegen declarations for the editor-relevant subset.
- [x] GREEN: implement C runtime ABI for editor-relevant chord-router subset.
- [x] REFACTOR: align tested Rust/C chord semantics for activated/released/idle outputs used by editor-style tests.
- [ ] COMMIT: atomic chord-router generated support commit.

## Phase 5: real terminal UTF-8 input

- [x] RED: add test/probe demonstrating current byte-at-a-time UTF-8 input gap.
- [x] GREEN: implement UTF-8 decoding for real terminal text input.
- [x] GREEN: handle malformed sequences through quarantine/unknown behavior, not invalid text.
- [x] REFACTOR: preserve Escape/arrow handling.
- [ ] COMMIT: atomic UTF-8 input commit.

## Phase 6: docs/status refresh

- [x] RED: add/update docs consistency test if practical, or record manual doc mismatch evidence.
- [x] GREEN: update `README.md` terminal status.
- [x] GREEN: update `STDLIB.md` terminal session status.
- [x] GREEN: update `OPALESCENT_CRASH_COURSE.md` if needed (no stale generated-terminal status found there).
- [x] REFACTOR: remove stale fake-backend-only/generated-gap claims.
- [ ] COMMIT: atomic docs/status commit.

## Final verification

- [x] Run `cargo test terminal_generated_testing:: --features integration --test integration_e2e -- --nocapture`.
- [x] Run `cargo test terminal_session_all_fixtures_compile_and_run_with_fake_backend --features integration --test integration_e2e -- --nocapture`.
- [x] Run focused lib/runtime terminal/codegen tests (`cargo test codegen_terminal_proposal --lib -- --nocapture`).
- [x] Run broader touched-subsystem tests or document blockers (`cargo test terminal_ --lib -- --nocapture`).
- [x] Ensure no simple Neovim editor fixture/application code was added.
- [x] Ensure checklist is accurate.
- [ ] Ensure all commits are atomic and pre-commit hook was not modified or bypassed (pending final commits).
