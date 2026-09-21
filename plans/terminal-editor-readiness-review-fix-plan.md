# Terminal Editor Readiness Review Fix Plan

Branch: `fix-terminal-editor-readiness-gaps`

Status: follow-up plan for review findings. Continue to avoid adding the simple Neovim editor fixture/application. Use red-green-refactor for each phase and commit each green/refactor phase atomically.

## Objective

Fix the review findings from the first readiness-gap implementation:

1. Make chord-router generated coverage meaningful instead of summary-only.
2. Make diagnostics status honest: either exercise generated diagnostic APIs where possible or narrow the claims where generated diagnostics remain fixture-placeholder scope.
3. Fix released-input memory safety in the C chord-router ABI.
4. Replace the first-pass chord-router stubs with a minimal real editor-relevant matcher.
5. Replace varargs chord codegen declarations with exact runtime ABI declarations.
6. Tighten real-terminal UTF-8 validation and malformed-byte quarantine.
7. Remove stale ignored compile-gap wording/tests for now-active fixture coverage.
8. Update checklists/docs to reflect the actual final state and atomic commits.

## Non-goals

- Do not create `test-projects/terminal-simple-editor`.
- Do not implement the full terminal proposal/chord-router spec beyond the editor-relevant generated subset tested here.
- Do not bypass or modify pre-commit hooks.

## Phase 1: planning and checklist hygiene

TDD/process steps:

1. RED: record the mismatches found by review in this plan and a checklist.
2. GREEN: create the checklist and commit this plan-only change.
3. REFACTOR: keep subsequent checklist items current as implementation proceeds.

Expected files:

- `plans/terminal-editor-readiness-review-fix-plan.md`
- `plans/checklists/terminal-editor-readiness-review-fix-checklist.md`

## Phase 2: meaningful chord generated coverage and memory safety

Goal: the `terminal-chord-quit` fixture and focused generated tests must construct chords, register bindings, process fake events, inspect released input safely, and activate only matching bindings.

TDD steps:

1. RED: update/add generated integration coverage that calls `terminal_chord_*` APIs and fails on the current stub/crash behavior.
2. GREEN: add exact codegen declarations and return-type metadata for chord APIs.
3. GREEN: implement C runtime structs for chord, sequence, binding, router, and released input.
4. GREEN: make processing compare key/control/named/enhanced-text plus requested modifier state, release nonmatching input, and return Idle/Activated/ReleasedInput for the tested subset.
5. GREEN: allocate/populate released input before exposing `terminal_chord_released_input_at`.
6. REFACTOR: keep helpers small, preserve existing fake event behavior, and add cleanup wrapper support.

Expected files:

- `test-projects/terminal-chord-quit/src/main.op`
- `tests/integration_e2e/terminal_session_input_gated.rs`
- `runtime/opal_io.c`
- `src/codegen/functions_stdlib_terminal_session.rs`
- `src/codegen/statements/runtime_type_info.rs`

## Phase 3: diagnostics honesty and stale red-probe cleanup

Goal: generated fixture metadata/docs no longer claim unexercised structured diagnostics behavior. If diagnostic constructors remain unavailable in production fixtures, narrow that status explicitly.

TDD steps:

1. RED: add/adjust fixture-source assertions so placeholder diagnostics are named as declaration/status coverage rather than behavioral runtime inspection.
2. GREEN: update `terminal-diagnostics-inspector` source comments/metadata and public docs/checklists to match the implemented generated diagnostics subset.
3. GREEN: remove or rename ignored compile-gap tests that now conflict with active compile/run coverage.
4. REFACTOR: keep the active 14-fixture compile/run test as the authoritative generated-fixture gate.

Expected files:

- `test-projects/terminal-diagnostics-inspector/src/main.op`
- `tests/integration_e2e/terminal_session_input_gated.rs`
- `README.md`
- `STDLIB.md`
- prior and new checklists

## Phase 4: UTF-8 malformed-input tightening

Goal: real terminal text decoding rejects overlong, surrogate, out-of-range, and incomplete sequences through UnknownBytes quarantine without emitting invalid text.

TDD steps:

1. RED: add a focused C-runtime unit test/probe for valid multibyte UTF-8 and malformed sequences.
2. GREEN: validate full scalar value and continuation rules in `runtime/opal_io.c`.
3. GREEN: keep Escape/arrow and ASCII behavior unchanged.
4. REFACTOR: isolate UTF-8 decoding helpers.

Expected files:

- `runtime/opal_io.c`
- `src/runtime/tests.rs` or an existing runtime C test harness location

## Phase 5: final verification and commits

Run focused tests after each phase and broader touched-subsystem tests before final response:

```bash
cargo test terminal_generated_testing:: --features integration --test integration_e2e -- --nocapture
cargo test terminal_session_all_fixtures_compile_and_run_with_fake_backend --features integration --test integration_e2e -- --nocapture
cargo test terminal_ --lib -- --nocapture
cargo test codegen_terminal_proposal --lib -- --nocapture
```

Final acceptance:

- Chord fixture uses chord APIs and proves matching/nonmatching behavior.
- Released input access cannot dereference an unallocated event vector.
- Chord runtime-ready declarations match the C ABI.
- Diagnostics docs/metadata do not overclaim behavior.
- Stale ignored compile-gap tests are removed or accurately renamed.
- UTF-8 real input validation rejects malformed sequences.
- No simple editor fixture/application is added.
- Final checklists are accurate, and commits remain atomic.
