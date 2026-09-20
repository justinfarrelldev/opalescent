# Implement Terminal Session Input (typed-event-session)

## TL;DR
> **Summary**: Implement the selected `typed-event-session` terminal session/input proposal as a same-release language, core, runtime, standard-library, test-runner, platform, and chord feature. Do not expose the public terminal API until every prerequisite in `core-prerequisites.md` is implemented and verified.
> **Deliverables**:
> - Branch `feature/terminal-session-input-typed-event-session` created from `main` with bisect-green atomic commits.
> - Exhaustive implementor TODO list and proposal traceability matrix.
> - Proposal-first fixtures for all 14 required terminal projects, then green implementations.
> - Language/parser/type-system support for annotations, constrained/opaque/affine declarations, borrows, `using`, `is ... into`, `constrain`, and immutable-error propagation.
> - Core wait/cancellation/timer/process-control/error/test-only prerequisites.
> - Legacy I/O coordinator and selected terminal/chord/test-only APIs and runtime behavior.
> - Linux and Windows terminal backend behavior per selected contract; Windows process-control remains unsupported as specified.
> - Documentation, ABI/history audit, security/compile-fail coverage, and final verification evidence.
> **Effort**: XL
> **Parallel**: YES - 6 waves, but public API exposure is gated on prerequisite completion.
> **Critical Path**: Task 1 → Task 2 → Tasks 3-5 → Tasks 6-18 → Tasks 19-23 → Tasks 24-32 → Tasks 33-37 → Final Verification Wave

## Context
### Original Request
- “Please implement the Terminal Session Input proposal as-specified, choosing typed-event-session as listed in COMPARISON.md. You must read the full proposal. Create a new branch based off of main for this work. Use atomic commits for each step. The TODO list created by the implementor must be exhaustive.”

### Interview Summary
- No follow-up questions are required. The user explicitly selected `typed-event-session` and requested implementation as specified.
- Defaults applied to Metis-identified ambiguities:
  - Branch name: `feature/terminal-session-input-typed-event-session`.
  - Atomic commit policy: **bisect-green commits**. Proposal-first red fixtures are committed gated/ignored/expected-fail, with RED evidence saved separately; each fixture is activated in the same commit that makes it pass.
  - Dependencies: **no new Rust dependencies** unless a future human explicitly approves; first attempt must use existing dependencies/std.
  - Windows support: implement selected Windows Console/ConPTY terminal contracts where existing CI/tooling can verify; POSIX process-control remains unsupported on Windows exactly as specified.
  - Conflict authority: active `.types.op` declarations own current IDs/fields/constructor visibility/evolution; `abi-history.md` owns retirements; `proposal.md`, `core-prerequisites.md`, `CHORDS.md`, and `TESTING.md` own behavioral contracts.

### Research Summary
- Proposal package read in full: `COMPARISON.md`, `core-prerequisites.md`, `typed-event-session/proposal.md`, `typed_event_session.types.op`, `abi-history.md`, `CHORDS.md`, `terminal_chords.types.op`, `TESTING.md`, `terminal_testing.types.op`, production examples, and historical alternatives.
- Codebase exploration found no existing session/typed-event implementation; closest patterns are runtime/stdlib I/O, stdout terminal builtins, module resolver symbol tables, codegen stdlib dispatch, LSP match dispatch, and hot-reload event/mock patterns.
- Testing infrastructure exists: `cargo test`, `cargo test --features integration`, CI all-features tests, clippy `-D warnings`, fmt check, `cargo make c-quality`, integration harnesses with temp-dir isolation and exact stdout/status assertions.

### Oracle Review (gaps addressed)
- Treat as same-release language/core/runtime/test-runner architecture work, not a terminal stdlib patch.
- Prevent half-adoption: no user-visible `TerminalSession` APIs before affine cleanup, sealed availability, immutable errors, wait/timer/cancellation, process-control, and legacy I/O coordination are complete.
- Fixture-first implementation must coexist with bisect-green atomic commits via gated/ignored RED fixtures.
- Exclude historical packet/batch APIs, async schedulers, subprocess/RPC/watch scope, editor buffers, callback storage in chord router, typestate sessions, and Windows process-control equivalents.

### Metis Review (gaps addressed)
- Added commit policy, branch convention, dependency default, traceability matrix, fixture-by-name coverage, platform acceptance criteria, and explicit guardrails.
- All Metis questions are resolved by defaults above; no user decision blocks plan execution.

## Work Objectives
### Core Objective
Implement the selected terminal session/input API and its required same-release prerequisites exactly enough that the 14 proposal-mandated project fixtures and security/platform/regression tests pass without weakening proposal source, authority boundaries, or runtime invariants.

### Deliverables
- Exhaustive implementor TODO list matching all tasks in this plan.
- Traceability matrix mapping proposal/core/chord/testing/ABI obligations to tasks/tests/commits.
- Parser/type-system/runtime/codegen/stdlib/test-runner support for all selected public and test-only signatures.
- Runtime terminal coordinator, session lifecycle, one-event input decoding, structured diagnostics, recovery tokens, output trust boundary, legacy I/O coordination, and platform backends.
- Chord router and test-only terminal factories/fake backend.
- Documentation updates and ABI/history audit.

### Definition of Done (verifiable conditions with commands)
- `git rev-parse --abbrev-ref HEAD` prints `feature/terminal-session-input-typed-event-session`.
- `git merge-base --is-ancestor main HEAD` exits `0`.
- `git status --short` is empty after final commit.
- `.sisyphus/evidence/terminal-session-input-traceability.md` lists every obligation source and maps each item to a task, test, and commit hash.
- `cargo fmt --all -- --check` passes.
- `cargo clippy --all-targets --all-features -- -D warnings` passes.
- `timeout 900 cargo test --all-features` passes.
- `cargo test --features integration` passes.
- `cargo make c-quality` passes.
- On Windows/Wine-capable CI, `cargo test --features "integration windows-wine" --test integration_e2e -- --nocapture wine_msvc_file_ops` passes or the existing harness records an explicit environment skip without test failure.
- All 14 required project fixtures under `test-projects/terminal-*` build/run through the normal integration harness and produce deterministic final summary lines.

### Must Have
- Use selected `typed-event-session` only.
- Implement every normative prerequisite in `core-prerequisites.md` in the same compatible release as selected terminal v1.
- Preserve active/retired ABI authority and disjoint ID sets.
- Keep all verification agent-executed with evidence under `.sisyphus/evidence/`.
- Keep each commit atomic and bisect-green; save RED evidence without committing a broken default test suite.

### Must NOT Have
- No `batched-event-pump` or `portable-input-packet-stream` public v1 API.
- No public OS handles, raw terminal handles, generic session-derived `StdoutTerminal`, direct string session writes, implicit trust conversion, stringly diagnostics, unbounded retention, polling scheduler, terminal process-control events, state-indexed v1 session types, mutable public batch/array event carriers, editor buffers, callback storage in chord router, generalized async runtime, subprocess/RPC/watch additions, or Windows process-control equivalent.
- No broad parser/typechecker/runtime refactors unrelated to this proposal.
- No new Rust dependency unless separately approved by a human in a future session.

## Verification Strategy
> ZERO HUMAN INTERVENTION - all verification is agent-executed.
- Test decision: **TDD / RED-GREEN-REFACTOR**. RED evidence is recorded in `.sisyphus/evidence/` while commits remain bisect-green through ignored/expected-fail gating until the implementation commit activates the test.
- QA policy: Every task below has agent-executed happy and failure scenarios.
- Evidence: `.sisyphus/evidence/task-{N}-{slug}.{ext}`.
- Required commands at the relevant checkpoints: `cargo test`, `cargo test --features integration`, targeted `cargo test <name>`, `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features -- -D warnings`, `timeout 900 cargo test --all-features`, `cargo make c-quality`.

## Execution Strategy
### Parallel Execution Waves
> Target: 5-8 tasks per wave. <3 per wave (except final) = under-splitting.
> Extract shared dependencies as Wave-1 tasks for max parallelism.

Wave 1: Tasks 1-5 — branch/traceability and proposal-first gated fixtures.
Wave 2: Tasks 6-12 — parser/declaration/module surface and ABI metadata.
Wave 3: Tasks 13-18 — type-system ownership/effects/error/test-only prerequisites.
Wave 4: Tasks 19-23 — core runtime/stdlib prerequisites and legacy I/O coordination.
Wave 5: Tasks 24-32 — selected terminal runtime, platform backends, test-only support, chords.
Wave 6: Tasks 33-37 — fixture activation, docs, security, final full-suite audit.

### Dependency Matrix (full, all tasks)
| Task | Depends On | Blocks |
|---|---|---|
| 1 | none | all tasks |
| 2 | 1 | 3-37 |
| 3 | 2 | 33 |
| 4 | 2 | 33 |
| 5 | 2 | 34 |
| 6 | 2 | 7-18, 24-32 |
| 7 | 6 | 13-18, 24-32 |
| 8 | 6 | 15-18, 24-32 |
| 9 | 6 | 10, 12, 24 |
| 10 | 6, 9 | 23, 24-32 |
| 11 | 6, 9 | 24-32 |
| 12 | 6, 9 | 24-32 |
| 13 | 7, 9, 12 | 14, 18, 24-32 |
| 14 | 7, 13 | 24-32 |
| 15 | 8, 12 | 24-32 |
| 16 | 8, 12, 15 | 24-32 |
| 17 | 9, 10, 12 | 31, 35 |
| 18 | 13, 14 | 32 |
| 19 | 13 | 20-23, 27, 32 |
| 20 | 19 | 27, 32 |
| 21 | 19, 20 | 28, 29 |
| 22 | 13, 15, 16, 19 | 23-30 |
| 23 | 10, 19, 22 | 24-32 |
| 24 | 9-17, 23 | 25-32 |
| 25 | 11, 12, 16, 24 | 26-32 |
| 26 | 13-16, 19-25 | 27-31, 33-34 |
| 27 | 19, 20, 24-26 | 31-34 |
| 28 | 21, 24-27 | 33-34 |
| 29 | 21, 24-27 | 33-34 |
| 30 | 22, 24-27 | 33-35 |
| 31 | 17, 24-30 | 33-35 |
| 32 | 18, 20, 24, 27, 31 | 34-35 |
| 33 | 3, 4, 24-31 | 36-37 |
| 34 | 5, 24-32 | 36-37 |
| 35 | 17, 30-32 | 36-37 |
| 36 | 24-35 | 37 |
| 37 | 1-36 | final verification |

### Agent Dispatch Summary (wave → task count → categories)
| Wave | Task Count | Categories |
|---|---:|---|
| 1 | 5 | quick, writing, deep |
| 2 | 7 | deep, unspecified-high |
| 3 | 6 | ultrabrain, deep |
| 4 | 5 | deep, unspecified-high |
| 5 | 9 | ultrabrain, deep, unspecified-high |
| 6 | 5 | deep, writing, unspecified-high |

### Traceability Obligations
| Source | Binding plan tasks |
|---|---|
| `COMPARISON.md:3-18`, `COMPARISON.md:20-41` selected authority and historical deference | 1, 2, 24, 36, 37 |
| `core-prerequisites.md:9-108` legacy I/O coordination | 22, 23, 30, 33, 35 |
| `core-prerequisites.md:109-164` wait/cancellation | 19, 27, 32, 33, 34 |
| `core-prerequisites.md:166-193` monotonic timers | 20, 32, 34 |
| `core-prerequisites.md:195-245` process control | 21, 28, 29, 33 |
| `core-prerequisites.md:247-294` `using` and affine cleanup | 13, 14, 18, 26, 31, 32 |
| `core-prerequisites.md:295-327` immutable errors/attachments | 15, 16, 25, 26, 31 |
| `core-prerequisites.md:327-342` test-only availability | 17, 31, 35 |
| `typed-event-session/proposal.md:59-145` constructor visibility, lifecycle, recovery | 12, 24, 26, 31, 33, 35 |
| `typed-event-session/proposal.md:146-155` readiness/one-event draining | 19, 27, 33, 34 |
| `typed-event-session/proposal.md:156-218` public API | 23, 24, 25, 26, 30, 31 |
| `typed-event-session/proposal.md:219-274` options/capabilities/diagnostics/events/accounting | 24, 25, 27, 31, 33, 34 |
| `typed-event-session/proposal.md:275-290` process-control workflow | 21, 28, 33 |
| `typed-event-session/proposal.md:292-314` output trust and platform contracts | 28, 29, 30, 35 |
| `typed-event-session/proposal.md:316-341` required project fixtures | 3, 4, 5, 33, 34 |
| `typed-event-session/proposal.md:343-351` verification/exclusions | 2, 35, 37 |
| `typed_event_session.types.op:1-1165` selected declarations | 9, 11, 12, 24, 25, 26, 27 |
| `abi-history.md:25-216` ABI inventory/retirements | 2, 9, 24, 36, 37 |
| `CHORDS.md:1-117`, `terminal_chords.types.op:1-293` chord companion | 18, 20, 27, 31, 32, 34, 35 |
| `TESTING.md:1-134`, `terminal_testing.types.op:1-427` test-only terminal support | 17, 31, 35 |

## TODOs
> Implementation + Test = ONE task. Never separate.
> EVERY task MUST have: Agent Profile + Parallelization + QA Scenarios.

- [x] 1. Create branch, clean-tree gate, and exhaustive implementor TODO

  **What to do**: Use `/git-master` workflow. Verify current branch is `main`, verify clean tree, update `main`, create `feature/terminal-session-input-typed-event-session`, and create the implementor’s exhaustive TODO list containing Tasks 1-37 plus F1-F4 before any implementation. Save the TODO snapshot and branch checks to `.sisyphus/evidence/task-1-branch-todo.md`.
  **Must NOT do**: Do not edit source before branch creation. Do not create a branch from a dirty worktree. Do not omit final verification tasks from the TODO list.

  **Recommended Agent Profile**:
  - Category: `quick` - Reason: procedural git/setup task with exact commands.
  - Skills: [`git-master`] - required for branch/commit discipline.
  - Omitted: [`frontend-ui-ux`] - no UI design work.

  **Parallelization**: Can Parallel: NO | Wave 1 | Blocks: 2-37 | Blocked By: none

  **References** (executor has NO interview context - be exhaustive):
  - User requirement: branch based off `main`, atomic commits, exhaustive TODO list.
  - Pattern: `.github/workflows/ci.yml` - final command expectations.
  - Pattern: `Makefile.toml` - local cargo-make verification tasks.
  - Planning rule: commits are bisect-green; gated RED evidence only.

  **Acceptance Criteria** (agent-executable only):
  - [ ] `git rev-parse --abbrev-ref HEAD` outputs exactly `feature/terminal-session-input-typed-event-session`.
  - [ ] `git merge-base --is-ancestor main HEAD` exits `0`.
  - [ ] `git status --short` is empty immediately before first implementation edit.
  - [ ] `.sisyphus/evidence/task-1-branch-todo.md` contains all Tasks 1-37 and F1-F4.

  **QA Scenarios** (MANDATORY - task incomplete without these):
  ```
  Scenario: Branch created from main
    Tool: Bash
    Steps: run `git rev-parse --abbrev-ref HEAD`; run `git merge-base --is-ancestor main HEAD`; run `git status --short`
    Expected: branch is `feature/terminal-session-input-typed-event-session`; merge-base command exits 0; status has no output
    Evidence: .sisyphus/evidence/task-1-branch-todo.md

  Scenario: Dirty-tree guard catches accidental prework
    Tool: Bash
    Steps: run `git status --short` before implementation; if nonempty, stop and record blocker instead of proceeding
    Expected: no source work starts unless status is empty
    Evidence: .sisyphus/evidence/task-1-branch-todo-error.md
  ```

  **Commit**: NO | Message: n/a | Files: [none]

- [x] 2. Build proposal traceability matrix and gated TDD policy harness

  **What to do**: Create `.sisyphus/evidence/terminal-session-input-traceability.md` mapping every normative source row in this plan’s Traceability Obligations to at least one task, expected test, and eventual commit placeholder. Add or update test harness metadata so proposal-first fixtures can be committed as ignored/expected-fail/gated while `cargo test` remains green. Define the exact opt-in RED command that asserts fixture failure before implementation.
  **Must NOT do**: Do not weaken fixture source. Do not activate failing tests in default CI. Do not map historical alternatives as positive APIs.

  **Recommended Agent Profile**:
  - Category: `deep` - Reason: cross-document accounting and harness design.
  - Skills: [] - repository tools suffice.
  - Omitted: [`git-master`] - only needed at commit time after implementation.

  **Parallelization**: Can Parallel: NO | Wave 1 | Blocks: 3-37 | Blocked By: 1

  **References**:
  - Authority: `stdlib-proposals/terminal-session-input/COMPARISON.md:7-18` selected reading order.
  - Authority: `stdlib-proposals/terminal-session-input/typed-event-session/proposal.md:316-341` project-first fixture obligations.
  - Authority: `stdlib-proposals/terminal-session-input/typed-event-session/proposal.md:343-351` verification exclusions.
  - Pattern: `tests/integration_e2e/tests.rs:67-90` e2e module registration.
  - Pattern: `tests/integration_e2e/game_of_life_full_memory_stress.rs` ignored/stress gate style.

  **Acceptance Criteria**:
  - [ ] `.sisyphus/evidence/terminal-session-input-traceability.md` has rows for `COMPARISON`, `core-prerequisites`, selected proposal, selected declarations, ABI history, chords, and testing contracts.
  - [ ] The matrix includes columns: Obligation ID, Source, Task(s), Test(s), Commit hash, Status.
  - [ ] `cargo test` passes with gated fixtures present.
  - [ ] An opt-in RED command exists and records expected failure without failing the default suite.

  **QA Scenarios**:
  ```
  Scenario: Default suite remains green with gated fixtures
    Tool: Bash
    Steps: run `cargo test`
    Expected: command exits 0; gated terminal fixtures are not active in the default suite
    Evidence: .sisyphus/evidence/task-2-traceability.txt

  Scenario: RED command proves fixtures are not yet implemented
    Tool: Bash
    Steps: run the documented opt-in terminal fixture RED command and assert nonzero exit as expected
    Expected: command failure is captured as RED evidence; default `cargo test` still passes afterward
    Evidence: .sisyphus/evidence/task-2-red-fixture-gate.txt
  ```

  **Commit**: YES | Message: `test(terminal): add proposal traceability and gated fixture policy` | Files: [.sisyphus/evidence/terminal-session-input-traceability.md, tests/** as needed]

- [x] 3. Add first required fixture set: key log, safe text echo, size probe, pause counter

  **What to do**: Add complete Opalescent projects under `test-projects/` for `terminal-key-log`, `terminal-text-echo-safe`, `terminal-size-probe`, and `terminal-pause-counter`. Use real selected public signatures/proposed syntax only. Register them in the gated terminal fixture harness with deterministic terminal input plans, expected stdout/stderr/status, and final summary lines.
  **Must NOT do**: Do not use host-language shims, pseudocode, callbacks, private runtime escape hatches, or unproposed language features. Do not allow direct `string` session writes in `terminal-text-echo-safe`.

  **Recommended Agent Profile**:
  - Category: `deep` - Reason: fixture source must encode proposal semantics exactly.
  - Skills: [] - no specialized skill required.
  - Omitted: [`frontend-ui-ux`] - fixtures assert terminal behavior, not aesthetics.

  **Parallelization**: Can Parallel: YES | Wave 1 | Blocks: 33 | Blocked By: 2

  **References**:
  - Required fixtures: `typed-event-session/proposal.md:326-329`.
  - API: `typed-event-session/proposal.md:160-213`.
  - Declarations: `typed-event-session/typed_event_session.types.op:760-816` `TerminalInputEvent` and `TerminalWait`.
  - Pattern: `tests/integration_e2e/interactive_io.rs:15-235` stdin/stdout exact assertion style.
  - Pattern: `tests/integration_e2e/terminal_stdlib.rs:10-230` exact terminal-byte assertions.

  **Acceptance Criteria**:
  - [ ] Four project directories exist with `opal.toml` and `src/main.op`.
  - [ ] Each fixture has a deterministic final summary line.
  - [ ] Gated RED run reports parse/type/runtime failure from missing terminal implementation, not malformed fixture setup.
  - [ ] `cargo test` remains green because these fixtures are gated.

  **QA Scenarios**:
  ```
  Scenario: Fixture inventory exists and is gated
    Tool: Bash
    Steps: run `test -f test-projects/terminal-key-log/src/main.op`; repeat for the other three fixtures; run `cargo test`
    Expected: all files exist; default tests pass
    Evidence: .sisyphus/evidence/task-3-fixtures-core.txt

  Scenario: Safe echo forbids direct string session writes
    Tool: Bash
    Steps: run the gated RED command for `terminal-text-echo-safe` and inspect failure/evidence for missing selected trust boundary support
    Expected: fixture source uses `trusted_terminal_output_from_application_text`; no direct `terminal_session_write_sync(session, some_string)` appears
    Evidence: .sisyphus/evidence/task-3-safe-echo-red.txt
  ```

  **Commit**: YES | Message: `test(terminal): add core terminal session fixtures` | Files: [test-projects/terminal-key-log/**, test-projects/terminal-text-echo-safe/**, test-projects/terminal-size-probe/**, test-projects/terminal-pause-counter/**, tests/**]

- [x] 4. Add second required fixture set: legacy I/O, timeout menu, cancel demo, diagnostics inspector

  **What to do**: Add complete gated project fixtures `terminal-legacy-io-rejection`, `terminal-timeout-menu`, `terminal-cancel-demo`, and `terminal-diagnostics-inspector`. Include deterministic test-runner terminal input/fault plans and expected final summary lines.
  **Must NOT do**: Do not make legacy I/O rejection consume stdin or mutate stdout. Do not treat diagnostics as token authority. Do not use real wall-clock duration for timeout assertions.

  **Recommended Agent Profile**:
  - Category: `deep` - Reason: fixtures encode coordination, cancellation, diagnostics, and timing semantics.
  - Skills: [] - no specialized skill required.
  - Omitted: [`git-master`] - only needed for commit execution.

  **Parallelization**: Can Parallel: YES | Wave 1 | Blocks: 33 | Blocked By: 2

  **References**:
  - Required fixtures: `typed-event-session/proposal.md:330-333`.
  - Legacy I/O prerequisite: `core-prerequisites.md:51-91`, `core-prerequisites.md:93-108`.
  - Diagnostics: `typed-event-session/proposal.md:229-240`, `typed_event_session.types.op:942-1015`.
  - Cancellation: `core-prerequisites.md:154-164`.
  - Pattern: `src/type_system/module_resolver/standard_symbols_core_io_and_bytes.rs:44-50`, `462-605` existing standard I/O/terminal symbols.

  **Acceptance Criteria**:
  - [ ] Four project directories exist with deterministic expected outputs.
  - [ ] `terminal-legacy-io-rejection` names at least `take_input`, `stdout_writer`, `stdout_terminal`, `terminal_supports_ansi`, one writer write/flush, and one cursor/screen mutation.
  - [ ] `terminal-diagnostics-inspector` refines typed errors and calls every stable diagnostic inspector.
  - [ ] Gated RED evidence shows missing implementation, not fixture syntax mistakes unrelated to proposal support.

  **QA Scenarios**:
  ```
  Scenario: Legacy rejection fixture covers full inventory
    Tool: Bash
    Steps: inspect `test-projects/terminal-legacy-io-rejection/src/main.op` for required function names; run `cargo test`
    Expected: required names are present; default suite passes
    Evidence: .sisyphus/evidence/task-4-legacy-fixture.txt

  Scenario: Timeout/cancel fixtures are deterministic
    Tool: Bash
    Steps: run the gated RED command for `terminal-timeout-menu` and `terminal-cancel-demo`
    Expected: failures are expected RED due missing terminal test backend; neither fixture depends on real elapsed time
    Evidence: .sisyphus/evidence/task-4-time-cancel-red.txt
  ```

  **Commit**: YES | Message: `test(terminal): add coordination and diagnostics fixtures` | Files: [test-projects/terminal-legacy-io-rejection/**, test-projects/terminal-timeout-menu/**, test-projects/terminal-cancel-demo/**, test-projects/terminal-diagnostics-inspector/**, tests/**]

- [x] 5. Add remaining required fixtures: chord quit, paste quarantine, interactive apps

  **What to do**: Add complete gated project fixtures `terminal-chord-quit`, `terminal-paste-quarantine`, `terminal-game-of-life-interactive`, `terminal-sokoban-mini`, `terminal-file-picker`, and `terminal-stopwatch-pomodoro`. Reuse existing `test-projects/game-of-life-full` style only for board/rules organization, then add deterministic terminal controls and final summaries. File picker must use a fixed manifest or explicitly ordered fixture list.
  **Must NOT do**: Do not depend on randomness, real wall-clock durations, host directory iteration order, editor buffers, callback APIs, or unimplemented sorting APIs.

  **Recommended Agent Profile**:
  - Category: `deep` - Reason: six app-level fixtures with cross-feature semantics.
  - Skills: [] - no specialized skill required.
  - Omitted: [`frontend-ui-ux`] - terminal rendering is asserted by deterministic bytes/summaries.

  **Parallelization**: Can Parallel: YES | Wave 1 | Blocks: 34 | Blocked By: 2

  **References**:
  - Required fixtures: `typed-event-session/proposal.md:334-339`.
  - Chord contract: `CHORDS.md:9-29`, `CHORDS.md:115-117`.
  - Pattern: `tests/integration_e2e/game_of_life.rs` and `test-projects/game-of-life-full/` for existing project structure.
  - Pattern: `tests/integration_e2e/fs_helpers.rs` and `fs_state_guard.rs` for deterministic file fixture isolation.

  **Acceptance Criteria**:
  - [ ] Six project directories exist with `opal.toml`, `src/main.op`, deterministic expected outputs, and final summary lines.
  - [ ] `terminal-chord-quit` covers Ctrl-Q and Escape bindings with one activation.
  - [ ] `terminal-file-picker` uses fixed ordering, not host directory order.
  - [ ] Gated RED command captures expected missing implementation failures; default `cargo test` passes.

  **QA Scenarios**:
  ```
  Scenario: Remaining fixture inventory exists
    Tool: Bash
    Steps: run file-existence checks for all six `test-projects/terminal-*` project `src/main.op` files; run `cargo test`
    Expected: all files exist and default test suite exits 0
    Evidence: .sisyphus/evidence/task-5-remaining-fixtures.txt

  Scenario: Interactive app fixtures are deterministic
    Tool: Bash
    Steps: run gated RED command for `terminal-game-of-life-interactive`, `terminal-file-picker`, and `terminal-stopwatch-pomodoro`
    Expected: failures are due missing proposed terminal/test support; final summaries do not depend on randomness, real duration, or host directory order
    Evidence: .sisyphus/evidence/task-5-interactive-red.txt
  ```

  **Commit**: YES | Message: `test(terminal): add interactive terminal fixtures` | Files: [test-projects/terminal-chord-quit/**, test-projects/terminal-paste-quarantine/**, test-projects/terminal-game-of-life-interactive/**, test-projects/terminal-sokoban-mini/**, test-projects/terminal-file-picker/**, test-projects/terminal-stopwatch-pomodoro/**, tests/**]

- [x] 6. Parse proposal declaration metadata and type forms

  **What to do**: Extend lexer/parser/AST support for declaration metadata and type declarations used by selected proposal files: `@availability`, `@constructor_visibility`, `@abi_type_id`, `@abi_evolution`, `namespace`, `constrained type ... where`, `opaque immutable type`, `compiler_registered affine resource type`, `public non_exhaustive type`, variant explicit IDs (`Variant = 1`), and variant payloads. Add parser tests for accepted proposal declarations and rejected malformed annotations.
  **Must NOT do**: Do not assign runtime semantics yet. Do not accept annotations outside documented forms. Do not alter existing `.op` syntax behavior.

  **Recommended Agent Profile**:
  - Category: `deep` - Reason: parser and AST expansion across declaration grammar.
  - Skills: [] - no specialized skill required.
  - Omitted: [`git-master`] - commit only after tests pass.

  **Parallelization**: Can Parallel: YES | Wave 2 | Blocks: 7-18, 24-32 | Blocked By: 2

  **References**:
  - Declaration examples: `typed_event_session.types.op:1-12`, `typed_event_session.types.op:259-272`, `terminal_testing.types.op:4-24`.
  - Parser files: `src/parser/declarations.rs`, `src/parser/types.rs`, `src/parser/imports.rs`, `src/parser/tests.rs`.
  - Type AST files: `src/type_system/types.rs`, `src/type_system/checker/declarations.rs`.

  **Acceptance Criteria**:
  - [ ] Parser accepts representative snippets from selected/chord/testing `.types.op` declarations.
  - [ ] Parser rejects unknown annotation names, duplicate ABI IDs on one declaration, malformed `where`, and `@availability(test_only)` on local statements.
  - [ ] Existing parser tests pass with `cargo test parser` or equivalent targeted parser test command.
  - [ ] Full `cargo test` passes.

  **QA Scenarios**:
  ```
  Scenario: Proposal declaration forms parse
    Tool: Bash
    Steps: run targeted parser tests covering constrained, opaque immutable, affine resource, constructor visibility, ABI metadata, namespace, and explicit variant IDs
    Expected: targeted tests pass
    Evidence: .sisyphus/evidence/task-6-parser-declarations.txt

  Scenario: Malformed metadata is rejected
    Tool: Bash
    Steps: run parser negative tests for unknown/duplicate/misplaced annotations and malformed `where`
    Expected: each invalid snippet returns a parser diagnostic and does not panic
    Evidence: .sisyphus/evidence/task-6-parser-declarations-error.txt
  ```

  **Commit**: YES | Message: `feat(parser): parse terminal proposal declaration metadata` | Files: [src/parser/**, src/type_system/types.rs, src/parser/tests.rs]

- [x] 7. Parse borrows, `using`, `is ... into`, `constrain`, and cause propagation syntax

  **What to do**: Extend expression/statement parsing for canonical `ref` and `mutable ref` parameter/argument forms, `using binding = acquisition(): body`, `if value is Family.Variant into payload:`, `constrain Type from value`, `propagate error_value`, `propagate error_value cause prior_error`, and `propagate call() cause prior_error`. Add precedence and guard-handler parser tests.
  **Must NOT do**: Do not add `match`, exceptions, `defer`, alternate parser productions for `is`, or callback/defer semantics.

  **Recommended Agent Profile**:
  - Category: `deep` - Reason: parser changes interact with expression precedence and statements.
  - Skills: [] - no specialized skill required.
  - Omitted: [`frontend-ui-ux`] - no UI.

  **Parallelization**: Can Parallel: YES | Wave 2 | Blocks: 13-18, 24-32 | Blocked By: 6

  **References**:
  - `is ... into`: `typed-event-session/proposal.md:15-30`.
  - `constrain`: `typed-event-session/proposal.md:31-40`.
  - `using` and propagation: `core-prerequisites.md:247-327`.
  - Parser files: `src/parser/expressions.rs`, `src/parser/statements.rs`, `src/parser/statements_guard.rs`, `src/parser/precedence.rs`, `src/parser/types.rs`.

  **Acceptance Criteria**:
  - [ ] Accepted parser tests cover each syntax form used in proposal examples.
  - [ ] Negative tests reject `into` after equality, `is not ... into`, compound-left refinement, payloadless `into`, and `using` owner escaping syntactically where parser can know.
  - [ ] Existing guard, import, and statement parser tests continue passing.

  **QA Scenarios**:
  ```
  Scenario: Selected expression syntax parses
    Tool: Bash
    Steps: run targeted parser tests for `using`, `is ... into`, `constrain`, and cause propagation
    Expected: all selected syntax tests pass
    Evidence: .sisyphus/evidence/task-7-parser-expressions.txt

  Scenario: Forbidden syntax is rejected
    Tool: Bash
    Steps: run negative parser tests for `is not X into y`, equality `into`, `match`, `defer`, and malformed `using`
    Expected: each invalid form produces a diagnostic; no unsupported syntax is accepted
    Evidence: .sisyphus/evidence/task-7-parser-expressions-error.txt
  ```

  **Commit**: YES | Message: `feat(parser): parse affine and refinement syntax` | Files: [src/parser/**, src/parser/tests.rs]

- [x] 8. Preserve new proposal syntax in formatter, doc generation, and diagnostics

  **What to do**: Update formatter/doc/diagnostic paths so new annotations, namespaces, constrained/opaque/affine declarations, borrows, `using`, `is ... into`, `constrain`, and cause propagation round-trip without losing comments or changing semantics. Add fmt-check and doc-generation tests where existing infrastructure supports them.
  **Must NOT do**: Do not rewrite proposal files or source fixtures into different API names. Do not silently drop annotations.

  **Recommended Agent Profile**:
  - Category: `unspecified-high` - Reason: cross-tool syntax preservation.
  - Skills: [] - no specialized skill required.
  - Omitted: [`ai-slop-remover`] - not a cleanup-only task.

  **Parallelization**: Can Parallel: YES | Wave 2 | Blocks: 15-18, 24-32 | Blocked By: 6

  **References**:
  - CLI docs: README fmt/doc command descriptions.
  - Parser/formatter files: `src/parser/declarations.rs`, `src/parser/expressions.rs`, `src/parser/statements.rs`, `src/formatter.rs`, `src/errors/formatter.rs`.
  - Documentation files: `src/doc_gen.rs`, `src/ast/documentation.rs`.
  - Proposal examples: `inspect_terminal_capabilities.op`, `run_editor_event_loop.op`, `configure_editor_chords.op`.

  **Acceptance Criteria**:
  - [ ] Formatting preserves all proposal annotations and declaration modifiers in representative snippets.
  - [ ] Doc generation either documents new public declarations or emits a precise unsupported diagnostic until semantic implementation lands.
  - [ ] `cargo fmt --all -- --check` passes.
  - [ ] `cargo test` passes.

  **QA Scenarios**:
  ```
  Scenario: Formatter preserves proposal syntax
    Tool: Bash
    Steps: run targeted formatter tests for annotated constrained, opaque, affine, and test-only declarations
    Expected: output retains annotations, namespace, visibility, fields, and comments
    Evidence: .sisyphus/evidence/task-8-format-doc.txt

  Scenario: Formatter rejects/diagnoses unsupported malformed syntax
    Tool: Bash
    Steps: run fmt/doc tests on malformed annotation examples
    Expected: precise diagnostics; no annotation stripping or panic
    Evidence: .sisyphus/evidence/task-8-format-doc-error.txt
  ```

  **Commit**: YES | Message: `feat(fmt): preserve proposal syntax metadata` | Files: [src/formatter.rs, src/errors/formatter.rs, src/doc_gen.rs, src/ast/documentation.rs, src/parser/** as needed, tests/**]

- [x] 9. Load selected/chord/test declaration files as authoritative module inputs

  **What to do**: Wire `.types.op` declaration loading for the selected terminal declaration files and test-only namespace/module declarations. Register `standard`, terminal companion, and `standard.testing.terminal` symbols through existing module resolver conventions while preserving constructor visibility, availability, and authority boundaries.
  **Must NOT do**: Do not copy declarations into unrelated Rust constants without source traceability. Do not assign terminal ABI IDs to core/system/test-only types. Do not make test-only symbols importable from production code.

  **Recommended Agent Profile**:
  - Category: `deep` - Reason: module resolution and authority boundary design.
  - Skills: [] - no specialized skill required.
  - Omitted: [`frontend-ui-ux`] - no UI.

  **Parallelization**: Can Parallel: YES | Wave 2 | Blocks: 10-12, 24-32 | Blocked By: 6

  **References**:
  - Existing modules: `src/type_system/module_resolver/standard_modules.rs`, `src/type_system/module_resolver/standard_symbols_core_io_and_bytes.rs`, `src/type_system/module_resolver/standard_symbols_process.rs`.
  - Terminal declarations: `typed_event_session.types.op:1-1165`, `terminal_chords.types.op:1-293`, `terminal_testing.types.op:1-427`.
  - Authority: `abi-history.md:25-43` active vs retired ownership.

  **Acceptance Criteria**:
  - [ ] Production imports can resolve selected terminal and chord public types/functions only after gate conditions in Task 23.
  - [ ] Test-only imports resolve only under test artifact compilation.
  - [ ] Production import of `standard.testing.terminal` fails with a precise availability diagnostic.
  - [ ] Existing imports from `standard`, `math`, and `process` still pass.

  **QA Scenarios**:
  ```
  Scenario: Terminal declaration modules resolve under correct availability
    Tool: Bash
    Steps: run targeted module resolver tests for production terminal/chord symbols and test-only terminal symbols in test mode
    Expected: correct symbols resolve with preserved metadata
    Evidence: .sisyphus/evidence/task-9-module-resolution.txt

  Scenario: Production test-only import is rejected
    Tool: Bash
    Steps: run compile-fail test importing `standard.testing.terminal` from a production project
    Expected: precise test-only availability diagnostic; no artifact is emitted
    Evidence: .sisyphus/evidence/task-9-module-resolution-error.txt
  ```

  **Commit**: YES | Message: `feat(type-system): load terminal proposal declarations` | Files: [src/type_system/module_resolver/**, src/type_system/checker/**, tests/**]

- [x] 10. Enforce terminal/chord ABI metadata and append-only history checks

  **What to do**: Add ABI validation that reads active declaration metadata and verifies selected type IDs, explicit variant IDs, retired IDs, intentional gaps, and chord IDs against `abi-history.md`. The check must confirm exactly 87 active selected production type IDs and 23 active chord production type IDs until declarations intentionally change.
  **Must NOT do**: Do not treat `abi-history.md` as active declaration source. Do not retire uncommitted candidate value `TerminalSessionRestoreError.GenerationExhausted=14`. Do not allocate terminal IDs to core/system/test-only declarations.

  **Recommended Agent Profile**:
  - Category: `deep` - Reason: ABI correctness is cross-file and security-sensitive.
  - Skills: [] - no specialized skill required.
  - Omitted: [`git-master`] - commit only after tests pass.

  **Parallelization**: Can Parallel: YES | Wave 2 | Blocks: 23, 24-32, 36 | Blocked By: 9

  **References**:
  - `abi-history.md:45-62` active selected inventory.
  - `abi-history.md:64-120` selected active variant IDs.
  - `abi-history.md:121-178` active chord inventory and representation notes.
  - `abi-history.md:179-216` retired/uncommitted IDs.

  **Acceptance Criteria**:
  - [ ] ABI validation test passes for current selected and chord declarations.
  - [ ] Negative fixture with reused retired ID fails.
  - [ ] Negative fixture with terminal ABI ID on test-only declaration fails.
  - [ ] Traceability matrix records ABI check task/test names.

  **QA Scenarios**:
  ```
  Scenario: Active ABI inventory matches authority
    Tool: Bash
    Steps: run targeted ABI validation tests
    Expected: exactly 87 selected and 23 chord production type IDs are reported; intentional gaps are not failures
    Evidence: .sisyphus/evidence/task-10-abi.txt

  Scenario: Retired ID reuse is rejected
    Tool: Bash
    Steps: run ABI negative test using `TerminalOperation.AcquireOutputTerminal=6` or retired session-state ID
    Expected: validation fails with retired-ID diagnostic
    Evidence: .sisyphus/evidence/task-10-abi-error.txt
  ```

  **Commit**: YES | Message: `feat(abi): validate terminal declaration history` | Files: [src/** abi/declaration validation files, tests/**]

- [x] 11. Add gated standard-library API symbol table and typechecker scaffolding

  **What to do**: Register every selected terminal public function signature, chord function signature, core prerequisite signature, and test-only signature in standard symbol/type tables behind the adoption gate. Add type-check tests proving signatures, error families, constructor visibility, and trust-boundary types resolve correctly while incomplete runtime calls remain gated until Task 23.
  **Must NOT do**: Do not expose selected APIs to normal production programs before Task 23 gate passes. Do not allow direct construction of runtime-only sealed values. Do not add `TerminalSessionStateError` to unrelated APIs.

  **Recommended Agent Profile**:
  - Category: `deep` - Reason: large symbol surface with strict availability and error family rules.
  - Skills: [] - no specialized skill required.
  - Omitted: [`frontend-ui-ux`] - no UI.

  **Parallelization**: Can Parallel: YES | Wave 2 | Blocks: 24-32 | Blocked By: 9

  **References**:
  - Public terminal API: `typed-event-session/proposal.md:160-213`.
  - Chord API: `CHORDS.md:13-29`.
  - Test-only API: `TESTING.md:15-71`.
  - Existing symbol patterns: `src/type_system/checker/stdout_text_builtins.rs:33-249`, `src/type_system/module_resolver/standard_symbols_core_io_and_bytes.rs:462-605`.

  **Acceptance Criteria**:
  - [ ] Type-check tests cover every public terminal/chord/test-only function signature.
  - [ ] Constructor visibility rejects public construction of `TerminalSession`, `TerminalRecoveryToken`, runtime events, diagnostics, and test-runner-only authority.
  - [ ] Direct `string` passed to `terminal_session_write_sync` is a type error.
  - [ ] Existing stdlib symbol tests pass.

  **QA Scenarios**:
  ```
  Scenario: Selected public signatures type-check under gate
    Tool: Bash
    Steps: run targeted type-system tests for terminal/chord/core/test-only signatures
    Expected: all signatures resolve with exact parameter, return, and error-family sets
    Evidence: .sisyphus/evidence/task-11-symbols.txt

  Scenario: Sealed construction and direct string writes fail
    Tool: Bash
    Steps: run compile-fail tests constructing `TerminalSession` and calling `terminal_session_write_sync(session, 'raw')`
    Expected: constructor-visibility/type diagnostics; no runtime artifact
    Evidence: .sisyphus/evidence/task-11-symbols-error.txt
  ```

  **Commit**: YES | Message: `feat(stdlib): register gated terminal session APIs` | Files: [src/type_system/checker/**, src/type_system/module_resolver/**, tests/**]

- [x] 12. Add codegen/runtime declaration scaffolding without public execution

  **What to do**: Extend codegen dispatch and runtime type-info tables so every gated terminal/core/chord/test-only symbol has a declared lowering target or explicit gated unsupported diagnostic before runtime implementation. Ensure no unresolved symbol crashes the compiler and no terminal runtime function is callable before Task 23 gate opens.
  **Must NOT do**: Do not emit public calls to unimplemented C runtime functions in default production mode. Do not silently lower unsupported terminal operations to no-ops.

  **Recommended Agent Profile**:
  - Category: `unspecified-high` - Reason: central codegen dispatch and failure-mode handling.
  - Skills: [] - no specialized skill required.
  - Omitted: [`frontend-ui-ux`] - no UI.

  **Parallelization**: Can Parallel: YES | Wave 2 | Blocks: 24-32 | Blocked By: 9

  **References**:
  - Existing dispatch: `src/codegen/functions_stdlib.rs:92-314`, `src/codegen/functions_stdlib.rs:828-909`.
  - Runtime type mapping: `src/codegen/statements/runtime_type_info.rs:41-167`, `187-277`.
  - Existing special string-array call: `src/codegen/functions_call/string_array_calls.rs:92-174`.

  **Acceptance Criteria**:
  - [ ] Compile tests for gated terminal calls fail with explicit “terminal proposal gate not complete” diagnostic before runtime implementation.
  - [ ] No LLVM unresolved external is emitted for gated terminal calls in default mode.
  - [ ] Existing codegen tests for `take_input` and terminal stdlib continue passing.
  - [ ] `cargo test` passes.

  **QA Scenarios**:
  ```
  Scenario: Gated terminal codegen fails explicitly
    Tool: Bash
    Steps: run compile-fail codegen test invoking `terminal_session_open_sync` before Task 23 gate opens
    Expected: explicit gate diagnostic; no unresolved external or panic
    Evidence: .sisyphus/evidence/task-12-codegen-gate.txt

  Scenario: Existing stdlib codegen unchanged
    Tool: Bash
    Steps: run targeted `src/codegen/tests.rs` tests for `take_input` and existing terminal declarations
    Expected: existing tests pass
    Evidence: .sisyphus/evidence/task-12-codegen-regression.txt
  ```

  **Commit**: YES | Message: `feat(codegen): add gated terminal runtime declarations` | Files: [src/codegen/**, tests/**]

- [x] 13. Implement affine resource ownership and second-class borrow checking

  **What to do**: Add compiler/type-system support for `compiler_registered affine resource` values, noncopyable ownership, move/use-after-move tracking, second-class `ref` and `mutable ref` borrows, borrow lifetime limits, and rejection of storing/returning/capturing owners or borrows before cleanup authority is consumed. Cover transitive restrictions through aggregates and aliases where the current language can represent them.
  **Must NOT do**: Do not introduce v1 typestate session types. Do not permit clone/copy of affine resources. Do not allow borrow escape through fields, arrays, globals, closures, returns, `break`, or `continue` values.

  **Recommended Agent Profile**:
  - Category: `ultrabrain` - Reason: ownership/effect semantics are high-risk compiler work.
  - Skills: [] - no specialized skill required.
  - Omitted: [`frontend-ui-ux`] - no UI.

  **Parallelization**: Can Parallel: NO | Wave 3 | Blocks: 14, 18, 24-32 | Blocked By: 7, 9, 12

  **References**:
  - Ownership rules: `core-prerequisites.md:247-294`.
  - `TerminalSession` affine declaration: `typed_event_session.types.op:255-263`.
  - Existing reference rules: `src/type_system/checker/ref_rules.rs`.
  - Existing checker statements/expressions: `src/type_system/checker/statements.rs`, `src/type_system/checker/expressions.rs`.

  **Acceptance Criteria**:
  - [ ] Type tests reject copying, double move, use-after-move, return/store/capture of affine owners, and escaping borrows.
  - [ ] Type tests allow `ref session: TerminalSession` and `mutable ref session: TerminalSession` only for one call/full-expression.
  - [ ] Existing reference-rule tests pass.
  - [ ] Gated fixture type checking reaches expected missing-runtime errors after ownership syntax is accepted.

  **QA Scenarios**:
  ```
  Scenario: Affine owners are accepted for valid borrowing
    Tool: Bash
    Steps: run targeted type tests with `using session = ...` and calls accepting `ref`/`mutable ref`
    Expected: valid borrow patterns type-check
    Evidence: .sisyphus/evidence/task-13-affine.txt

  Scenario: Affine escape and double-use are rejected
    Tool: Bash
    Steps: run compile-fail tests for copy, use-after-move, storing owner in array/field, returning owner, and borrow escape
    Expected: precise ownership diagnostics; no artifact emitted
    Evidence: .sisyphus/evidence/task-13-affine-error.txt
  ```

  **Commit**: YES | Message: `feat(type-system): enforce affine resource ownership` | Files: [src/type_system/**, tests/**]

- [x] 14. Implement `using` cleanup effects and cleanup-authority transfer

  **What to do**: Implement compiler-visible affine resource registrations, `using` acquisition/body/cleanup effect union, cleanup on fallthrough/return/break/continue/propagation, reverse acquisition order, explicit close obligation consumption, declared cleanup-authority transfer only for registered results, and primary/cause/suppressed cleanup error ordering.
  **Must NOT do**: Do not infer transfer from comments, return type names, or ordinary moves. Do not skip cleanup after body errors. Do not duplicate cleanup after successful explicit close.

  **Recommended Agent Profile**:
  - Category: `ultrabrain` - Reason: control-flow/effect interaction and cleanup correctness.
  - Skills: [] - no specialized skill required.
  - Omitted: [`frontend-ui-ux`] - no UI.

  **Parallelization**: Can Parallel: NO | Wave 3 | Blocks: 24-32 | Blocked By: 13

  **References**:
  - Cleanup contract: `core-prerequisites.md:247-294`.
  - Resource registrations: `core-prerequisites.md:255-265`, `TESTING.md:111-126`, `CHORDS.md:35-47`.
  - Checker control flow: `src/type_system/checker/control_flow.rs`, `src/type_system/checker/statements.rs`, `src/type_system/checker/fallible_expressions.rs`.
  - Codegen cleanup patterns: `src/codegen/functions_call/call_arg_cleanup.rs`, `src/codegen/scope_tracker.rs`.

  **Acceptance Criteria**:
  - [ ] Type/effect tests require enclosing `errors` clauses to include acquisition, body, and cleanup error families.
  - [ ] Runtime/codegen tests prove cleanup runs on fallthrough, return, break, continue, and propagation.
  - [ ] Nested `using` cleanup order is reverse acquisition order.
  - [ ] Transfer is legal only for `TerminalSessionRestoreError.CloseRestorePending` cleanup transfer for `TerminalSession` and exact registered test/chord rules.

  **QA Scenarios**:
  ```
  Scenario: Cleanup runs on every lexical exit
    Tool: Bash
    Steps: run targeted unit/integration tests covering fallthrough, return, break, continue, and propagated error exits
    Expected: cleanup counter/evidence confirms exactly-once cleanup
    Evidence: .sisyphus/evidence/task-14-using.txt

  Scenario: Undeclared cleanup transfer is rejected
    Tool: Bash
    Steps: run compile-fail tests attempting to move/return/transfer an affine owner through an unregistered operation
    Expected: precise cleanup-authority diagnostic
    Evidence: .sisyphus/evidence/task-14-using-error.txt
  ```

  **Commit**: YES | Message: `feat(type-system): implement affine using cleanup` | Files: [src/type_system/**, src/codegen/**, tests/**]

- [x] 15. Implement nominal variant refinement, heterogeneous guard unions, and `constrain`

  **What to do**: Implement one `is` expression classification: nominal variant refinement only when the right side resolves to `Type.Variant` and the left side is a direct identifier; otherwise equality. Add branch-local immutable `into` payload bindings, heterogeneous guard error union propagation rules, and `constrain Type from value` for constrained runtime values with `ConstraintViolationError`.
  **Must NOT do**: Do not add pattern matching, exhaustiveness checking, exceptions, structural error unions, `Option`/`Result` wrappers, or public intrinsic calls for `contains_no_nul`, `utf8_byte_length`, or `unicode_scalar_count` outside `where` expressions.

  **Recommended Agent Profile**:
  - Category: `ultrabrain` - Reason: refinement and error-union semantics affect type soundness.
  - Skills: [] - no specialized skill required.
  - Omitted: [`frontend-ui-ux`] - no UI.

  **Parallelization**: Can Parallel: YES | Wave 3 | Blocks: 16, 24-32 | Blocked By: 8, 12

  **References**:
  - Refinement: `typed-event-session/proposal.md:15-30`.
  - Constraints: `typed-event-session/proposal.md:31-40`.
  - Guard errors: `core-prerequisites.md:309-317`.
  - Existing checker files: `src/type_system/checker/expressions_guard.rs`, `src/type_system/checker/fallible_expressions.rs`, `src/type_system/constraints.rs`, `src/type_system/error_families.rs`.

  **Acceptance Criteria**:
  - [ ] Valid `if event is TerminalInputEvent.Key into key_event:` narrows payload in branch.
  - [ ] Equality `if left is right:` remains unchanged for non-variant right side.
  - [ ] Unrefined guard error propagation requires all member families in enclosing `errors` clause.
  - [ ] `constrain TerminalWaitMilliseconds from runtime_value` succeeds/fails with typed constraint behavior.

  **QA Scenarios**:
  ```
  Scenario: Variant refinement narrows payload
    Tool: Bash
    Steps: run type tests refining `TerminalInputEvent.Key` and `TerminalSessionOpenError.InvalidOptions`
    Expected: payload fields are available only in true branch
    Evidence: .sisyphus/evidence/task-15-refinement.txt

  Scenario: Invalid refinement and incomplete error propagation fail
    Tool: Bash
    Steps: run compile-fail tests for compound-left `into`, payloadless `into`, and unrefined guard propagation missing one family
    Expected: precise diagnostics
    Evidence: .sisyphus/evidence/task-15-refinement-error.txt
  ```

  **Commit**: YES | Message: `feat(type-system): implement nominal refinement and constraints` | Files: [src/type_system/**, tests/**]

- [x] 16. Implement immutable error values, cause/suppressed attachments, and inspectors

  **What to do**: Implement immutable acyclic error values, `propagate error_value`, `propagate ... cause ...`, deterministic cause/suppressed/truncation behavior, attachment limits (cause depth 8, suppressed count 8, 64 KiB storage), allocation failure semantics, and standard inspectors from `core-prerequisites.md`.
  **Must NOT do**: Do not mutate existing error aliases. Do not replace existing causes. Do not flatten terminal diagnostics or recovery tokens into text attachments. Do not make attachment truncation affect typed recovery-token fields.

  **Recommended Agent Profile**:
  - Category: `ultrabrain` - Reason: runtime representation and error semantics are complex.
  - Skills: [] - no specialized skill required.
  - Omitted: [`frontend-ui-ux`] - no UI.

  **Parallelization**: Can Parallel: YES | Wave 3 | Blocks: 24-32 | Blocked By: 8, 12, 15

  **References**:
  - Error contract: `core-prerequisites.md:295-327`.
  - Existing error files: `src/runtime/errors.rs`, `src/type_system/errors.rs`, `src/type_system/error_families.rs`, `src/codegen/error.rs`, `src/codegen/error_abi.rs`.
  - Proposal example: `inspect_terminal_capabilities.op:21-38`, `56-121`.

  **Acceptance Criteria**:
  - [ ] Runtime tests cover cause insertion, existing-cause suppressed fallback, idempotent reattachment, cycle prevention, truncation markers, and bounds.
  - [ ] Type tests cover `propagate error_value` family inclusion rules.
  - [ ] `error_cause`, `error_suppressed_length`, `error_suppressed_at`, and `error_attachment_truncation*` resolve and run.
  - [ ] Existing error-handling tests pass.

  **QA Scenarios**:
  ```
  Scenario: Cause and suppressed attachments are deterministic
    Tool: Bash
    Steps: run targeted runtime/type tests for cause insertion, existing cause, suppressed ordering, and inspectors
    Expected: exact expected cause/suppressed/truncation results
    Evidence: .sisyphus/evidence/task-16-errors.txt

  Scenario: Cycles and attachment overflow are bounded
    Tool: Bash
    Steps: run tests that attempt identity/cycle attachment and exceed depth/count/byte limits
    Expected: no cycle is published; truncation flags are set deterministically
    Evidence: .sisyphus/evidence/task-16-errors-error.txt
  ```

  **Commit**: YES | Message: `feat(errors): support immutable error attachments` | Files: [src/runtime/**, src/type_system/**, src/codegen/**, tests/**]

- [x] 17. Enforce test-only availability and sealed test-runner authority

  **What to do**: Implement `@availability(test_only)` enforcement, `@constructor_visibility(test_runner)`, test artifact identity, and `test_runner_terminal_authority()` availability. Ensure production modules, exported signatures, metadata, manifests, reflection, generated exports, and artifacts cannot name or erase test-only symbols.
  **Must NOT do**: Do not use a runtime boolean for availability. Do not permit production import of `standard.testing.terminal`. Do not let tests construct sessions, recovery tokens, coordinator leases, or host handles through authority.

  **Recommended Agent Profile**:
  - Category: `deep` - Reason: compiler availability boundary with security implications.
  - Skills: [] - no specialized skill required.
  - Omitted: [`frontend-ui-ux`] - no UI.

  **Parallelization**: Can Parallel: YES | Wave 3 | Blocks: 31, 35 | Blocked By: 9, 10, 12

  **References**:
  - Availability contract: `core-prerequisites.md:327-342`.
  - Test contract: `TESTING.md:3-10`, `TESTING.md:15-73`.
  - Test declarations: `terminal_testing.types.op:4-24`, `377-388`, `395-427`.
  - Module resolver: `src/type_system/module_resolver/standard_modules.rs`.

  **Acceptance Criteria**:
  - [ ] Test artifact compilation can import `standard.testing.terminal` and call `test_runner_terminal_authority()`.
  - [ ] Production compilation rejects imports, fields, exported signatures, metadata, and manifests that name test-only symbols.
  - [ ] Test-only declarations carry no production terminal ABI IDs.
  - [ ] Existing non-terminal test infrastructure still works.

  **QA Scenarios**:
  ```
  Scenario: Test-only authority is available in test artifacts
    Tool: Bash
    Steps: run targeted test-mode compile/type-check test importing `standard.testing.terminal` and calling `test_runner_terminal_authority()`
    Expected: test artifact compiles under test mode
    Evidence: .sisyphus/evidence/task-17-test-only.txt

  Scenario: Production cannot name test-only symbols
    Tool: Bash
    Steps: run compile-fail production tests importing `standard.testing.terminal` and exporting a test-only type
    Expected: precise availability diagnostics; no production artifact
    Evidence: .sisyphus/evidence/task-17-test-only-error.txt
  ```

  **Commit**: YES | Message: `feat(type-system): enforce test-only availability` | Files: [src/type_system/**, src/codegen/**, tests/**]

- [x] 18. Implement transactional affine aggregate construction needed by chord runtime examples

  **What to do**: Implement declaration support and checker/codegen behavior for opt-in transactional affine aggregate construction and same-owner transitions: provisional member obligations, reverse rollback on constructor failure, allocation-free seal, declared layout transitions, and deterministic cleanup order. Cover `EditorChordRuntime`-style private aggregate semantics required by `CHORDS.md` and `configure_editor_chords.op`.
  **Must NOT do**: Do not add public general field moves, owner exports, or unregistered aggregate transitions. Do not allow fallible/observable operations between successful retarget and matching commit.

  **Recommended Agent Profile**:
  - Category: `ultrabrain` - Reason: affine aggregate semantics are complex and safety-critical.
  - Skills: [] - no specialized skill required.
  - Omitted: [`frontend-ui-ux`] - no UI.

  **Parallelization**: Can Parallel: NO | Wave 3 | Blocks: 32 | Blocked By: 13, 14

  **References**:
  - Aggregate contract: `core-prerequisites.md:267-280`.
  - Chord aggregate usage: `CHORDS.md:41-47`, `configure_editor_chords.op:29-74`.
  - Type checker: `src/type_system/checker/constructors.rs`, `src/type_system/checker/declarations.rs`, `src/type_system/checker/ref_rules.rs`.

  **Acceptance Criteria**:
  - [ ] Tests prove constructor failure cleans initialized provisional members exactly once in reverse acquisition order.
  - [ ] Tests prove seal publishes all obligations atomically with no provisional leaks.
  - [ ] Tests reject undeclared layout transitions, duplicate obligations, missing cleanup, exported member owner, and interposed fallible operation after retarget.
  - [ ] Chord example type-checks through private aggregate declarations once symbols exist.

  **QA Scenarios**:
  ```
  Scenario: Transactional aggregate rollback is exact
    Tool: Bash
    Steps: run unit/integration tests with counters for provisional member cleanup after constructor failure
    Expected: initialized members cleaned once in reverse order; no partial aggregate returned
    Evidence: .sisyphus/evidence/task-18-aggregate.txt

  Scenario: Illegal aggregate transitions are rejected
    Tool: Bash
    Steps: run compile-fail tests for undeclared transition, exported member owner, duplicate obligation, and fallible interposition
    Expected: precise affine aggregate diagnostics
    Evidence: .sisyphus/evidence/task-18-aggregate-error.txt
  ```

  **Commit**: YES | Message: `feat(type-system): support transactional affine aggregates` | Files: [src/type_system/**, src/codegen/**, tests/**]

- [x] 19. Implement generic wait sets, readiness sources, owned registrations, and cancellation

  **What to do**: Implement core/system `SystemWaitSet`, `SystemReadinessSource`, `SystemWaitRegistration`, `SystemOwnedWaitRegistration`, `SystemWaitWake`, `CancellationSource`, and `CancellationToken` semantics. Include stable source identity, bounded-fair registration order, transition wakes, stale wake handling, sticky generation-scoped cancellation, registration removal/retarget/drop lifetimes, and cancellation priority after already-published work.
  **Must NOT do**: Do not expose OS descriptors/HANDLEs/registration keys. Do not create a polling scheduler. Do not make `SystemWaitWake.Cancelled` carry source/generation.

  **Recommended Agent Profile**:
  - Category: `deep` - Reason: core runtime abstraction with deterministic tests.
  - Skills: [] - no specialized skill required.
  - Omitted: [`frontend-ui-ux`] - no UI.

  **Parallelization**: Can Parallel: YES | Wave 4 | Blocks: 20-23, 27, 32 | Blocked By: 13

  **References**:
  - Wait/cancellation contract: `core-prerequisites.md:109-164`.
  - Runtime patterns: `src/stdlib/system/process.rs:72-127`, `src/hot_reload/change_detection.rs:35-49`, `80-158`.
  - Runtime modules: `src/runtime/io.rs`, `src/stdlib/system.rs`, `src/stdlib/system/process.rs`.

  **Acceptance Criteria**:
  - [ ] Unit tests cover register/remove, repeated remove idempotence, wrong-set/unauthenticated/lifetime errors, owned registration retarget/remove/drop, wait-set destruction, source clone lifetime, bounded-fair selection, stale Ready handling, and cancellation priority.
  - [ ] Test-only fake sources can deterministically publish readiness for terminal/chord tests.
  - [ ] No public API exposes host handles.
  - [ ] `cargo test` passes.

  **QA Scenarios**:
  ```
  Scenario: Wait set fairness and lifetime rules hold
    Tool: Bash
    Steps: run targeted wait-set unit tests with three level-ready sources and owned registration retarget/removal
    Expected: bounded-fair order; exact retain/release counts; stale wakes rejected by source/generation comparison
    Evidence: .sisyphus/evidence/task-19-wait.txt

  Scenario: Cancellation wins after queued work drains
    Tool: Bash
    Steps: run cancellation tests with already-published work followed by simultaneous cancellation/new readiness
    Expected: queued work is delivered first; then `Cancelled` wins before new source consumption
    Evidence: .sisyphus/evidence/task-19-wait-error.txt
  ```

  **Commit**: YES | Message: `feat(runtime): add generic wait and cancellation primitives` | Files: [src/runtime/**, src/stdlib/system/**, src/type_system/**, src/codegen/**, tests/**]

- [x] 20. Implement affine monotonic timers

  **What to do**: Implement `MonotonicTimer`, `MonotonicDeadline`, `monotonic_clock_now`, timer readiness source, arm/disarm/generation/deadline functions, equality-as-expiration, never-reused generations, stale wake filtering support, and generation exhaustion behavior. Integrate with wait sets from Task 19.
  **Must NOT do**: Do not implement wall-clock timers here. Do not wrap/reset/reuse generations. Do not expose a terminal-specific timer identity.

  **Recommended Agent Profile**:
  - Category: `deep` - Reason: core runtime timing with wait integration.
  - Skills: [] - no specialized skill required.
  - Omitted: [`frontend-ui-ux`] - no UI.

  **Parallelization**: Can Parallel: YES | Wave 4 | Blocks: 27, 32, 34 | Blocked By: 19

  **References**:
  - Timer contract: `core-prerequisites.md:166-193`.
  - Existing time stdlib tests: `tests/integration_e2e/time_stdlib.rs`.
  - Chord timer use: `CHORDS.md:73-83`, `configure_editor_chords.op:98-199`.

  **Acceptance Criteria**:
  - [ ] Timer tests cover new, readiness source stability, arm, disarm, repeated disarm advancing generation, deadline read, not-armed error, equality expiration, stale wake filtering, and generation exhaustion.
  - [ ] Wait-set tests observe timer readiness through generic source identity.
  - [ ] Chord timer scenarios have deterministic fake clock support.
  - [ ] Existing time stdlib tests pass.

  **QA Scenarios**:
  ```
  Scenario: Timer generation and stale wake behavior
    Tool: Bash
    Steps: run targeted monotonic timer tests arming, disarming, rearming, and comparing wake generation
    Expected: generations never reuse; stale wakes do not expire newer deadlines
    Evidence: .sisyphus/evidence/task-20-timer.txt

  Scenario: Generation exhaustion fails without mutation
    Tool: Bash
    Steps: run test injecting near-maximum generation and attempt arm/disarm
    Expected: `MonotonicTimerError.GenerationExhausted` with unchanged prior timer state
    Evidence: .sisyphus/evidence/task-20-timer-error.txt
  ```

  **Commit**: YES | Message: `feat(runtime): add affine monotonic timers` | Files: [src/runtime/**, src/stdlib/system/**, src/type_system/**, src/codegen/**, tests/**]

- [x] 21. Implement separate POSIX process-control source with Windows unavailable

  **What to do**: Implement `ProcessControlSource`, readiness source, `ProcessControlPollResult`, `ProcessControlNotification`, `process_control_poll`, `process_control_acknowledge_suspend`, and `process_control_resume_application` per prerequisite. POSIX hosts observe catchable job-control suspension/continuation; Windows returns `ProcessControlUnavailableError.UnsupportedHost` before allocation.
  **Must NOT do**: Do not synthesize terminal input events for suspend/continue. Do not add Windows console-control equivalents. Do not acknowledge/resume terminal/application work implicitly.

  **Recommended Agent Profile**:
  - Category: `deep` - Reason: platform-specific source with strict ordering.
  - Skills: [] - no specialized skill required.
  - Omitted: [`frontend-ui-ux`] - no UI.

  **Parallelization**: Can Parallel: YES | Wave 4 | Blocks: 28, 29 | Blocked By: 19, 20

  **References**:
  - Process-control contract: `core-prerequisites.md:195-245`.
  - Terminal workflow: `typed-event-session/proposal.md:275-290`.
  - Example: `run_editor_event_loop.op:72-89`, `159-192`.
  - Existing process patterns: `src/stdlib/system/process.rs`, `tests/integration_e2e/process_api_smoke.rs`.

  **Acceptance Criteria**:
  - [ ] POSIX tests cover notification queue, Idle stale poll, generation exhaustion, acknowledge retry, continuation, explicit application resume, wrong/stale generation errors.
  - [ ] Windows tests or cfg tests verify `UnsupportedHost` before allocation.
  - [ ] Process-control notifications are never `TerminalInputEvent` variants.
  - [ ] Existing process API smoke tests pass.

  **QA Scenarios**:
  ```
  Scenario: POSIX process-control ordering is explicit
    Tool: Bash
    Steps: run targeted process-control tests with fake host indications for suspend/continue and stale wakes
    Expected: SuspendRequested -> explicit acknowledge -> Continued -> explicit resume; Idle is non-mutating
    Evidence: .sisyphus/evidence/task-21-process-control.txt

  Scenario: Windows process-control is unavailable
    Tool: Bash
    Steps: run cfg/unit test for Windows process-control construction path or Wine-capable test where available
    Expected: `ProcessControlUnavailableError.UnsupportedHost`; no source allocated
    Evidence: .sisyphus/evidence/task-21-process-control-error.txt
  ```

  **Commit**: YES | Message: `feat(runtime): add process control source` | Files: [src/runtime/**, src/stdlib/system/**, src/type_system/**, src/codegen/**, tests/**]

- [x] 22. Implement terminal coordinator and legacy standard I/O coordination

  **What to do**: Add process-global terminal coordinator states (`Free`, `Opening`, `Active`, `Paused`, `RestorePending`, `FailedOpenRecovery`, `FailedCloseRecovery`), generation-bound legacy stdout leases, future-fallible `take_input`, coordinator-aware stdout writer/terminal functions, diagnostic-lane `print`/`println`, stale-handle rejection, and rejection-before-consumption/mutation semantics.
  **Must NOT do**: Do not consume stdin or mutate stdout on rejection. Do not refresh stale legacy leases after any Free→Opening attempt. Do not add coordinator errors to unrelated APIs. Do not let `print`/`println` touch raw stdout in states marked drop.

  **Recommended Agent Profile**:
  - Category: `deep` - Reason: central stdlib behavior with high regression risk.
  - Skills: [] - no specialized skill required.
  - Omitted: [`frontend-ui-ux`] - no UI.

  **Parallelization**: Can Parallel: NO | Wave 4 | Blocks: 23-30 | Blocked By: 13, 15, 16, 19

  **References**:
  - Legacy coordination: `core-prerequisites.md:9-108`.
  - Existing I/O: `src/runtime/io.rs:7-73`, `src/stdlib/io.rs:19-114`.
  - Existing terminal symbols: `src/type_system/checker/stdout_text_builtins.rs:33-249`, `tests/integration_e2e/terminal_stdlib.rs:10-230`.
  - Existing interactive tests: `tests/integration_e2e/interactive_io.rs:15-235`.

  **Acceptance Criteria**:
  - [ ] `take_input` future signature and behavior reject with `StandardInputReadError.TerminalCoordinatorUnavailable` when coordinator is not `Free`.
  - [ ] stdout writer/terminal acquisition/use and terminal operations reject with exact operation/state payloads and no mutation.
  - [ ] Stale handles acquired before Free→Opening remain stale forever.
  - [ ] Existing interactive and terminal stdlib regression tests pass.

  **QA Scenarios**:
  ```
  Scenario: Legacy I/O rejects while session/coordinator is active
    Tool: Bash
    Steps: run targeted coordinator tests opening a fake active coordinator then calling `take_input`, writer acquisition/use, terminal capability/cursor/screen operations
    Expected: every call returns coordinator-unavailable before input/output mutation
    Evidence: .sisyphus/evidence/task-22-legacy-io.txt

  Scenario: Existing Free-state legacy I/O still works
    Tool: Bash
    Steps: run `cargo test --features integration --test integration_e2e -- --nocapture interactive_io terminal_stdlib`
    Expected: existing interactive and terminal stdlib tests pass in Free state
    Evidence: .sisyphus/evidence/task-22-legacy-io-regression.txt
  ```

  **Commit**: YES | Message: `feat(stdlib): coordinate legacy I/O with terminal ownership` | Files: [src/runtime/**, src/stdlib/**, src/type_system/**, src/codegen/**, tests/**]

- [x] 23. Open public terminal API gate only after prerequisite validation

  **What to do**: Replace the temporary gate from Tasks 11-12 with a prerequisite validation gate that opens selected public terminal/chord/core/test APIs only when Tasks 13-22 prerequisites are implemented. Add tests proving no public API is reachable if any required prerequisite is disabled in test configuration.
  **Must NOT do**: Do not expose partial `TerminalSession` APIs. Do not leave stale “proposal gate not complete” diagnostics after all prerequisites are verified. Do not bypass ABI validation.

  **Recommended Agent Profile**:
  - Category: `deep` - Reason: release/adoption safety gate.
  - Skills: [] - no specialized skill required.
  - Omitted: [`frontend-ui-ux`] - no UI.

  **Parallelization**: Can Parallel: NO | Wave 4 | Blocks: 24-32 | Blocked By: 10, 19, 22

  **References**:
  - Adoption warning: `COMPARISON.md:3-18`, `typed-event-session/proposal.md:5-12`, `core-prerequisites.md:340-342`.
  - Codegen gate from Task 12: `src/codegen/functions_stdlib.rs`, `src/codegen/statements/runtime_type_info.rs`.
  - Symbol gate from Task 11: `src/type_system/module_resolver/**`, `src/type_system/checker/**`.

  **Acceptance Criteria**:
  - [ ] Public selected symbols resolve and codegen only when prerequisite feature set is complete.
  - [ ] Test configuration disabling one prerequisite restores explicit gate diagnostic.
  - [ ] Traceability matrix marks core prerequisite rows complete before gate-open commit hash.
  - [ ] `cargo test` passes.

  **QA Scenarios**:
  ```
  Scenario: API gate opens after prerequisites
    Tool: Bash
    Steps: run type/codegen tests for `terminal_session_open_sync`, wait/timer/process-control, and chord symbols after prerequisite flags are enabled
    Expected: symbols lower to real runtime targets, not gate diagnostics
    Evidence: .sisyphus/evidence/task-23-api-gate.txt

  Scenario: Missing prerequisite prevents exposure
    Tool: Bash
    Steps: run targeted test configuration with one prerequisite disabled
    Expected: terminal API import/call fails with explicit prerequisite diagnostic
    Evidence: .sisyphus/evidence/task-23-api-gate-error.txt
  ```

  **Commit**: YES | Message: `feat(stdlib): open terminal API after prerequisites` | Files: [src/type_system/**, src/codegen/**, tests/**, .sisyphus/evidence/terminal-session-input-traceability.md]

- [x] 24. Implement selected terminal option, capability, diagnostic, event, and trust data model

  **What to do**: Implement runtime/stdlib representations for `TerminalSessionOptions`, `TerminalSessionFeaturePolicy`, `TerminalSessionResourceLimits`, `TerminalCapabilities`, all selected constrained types, `TerminalInputEvent`, `TerminalDiagnostic`, `TerminalDiagnosticCollection`, `TrustedTerminalOutput`, `SafeTerminalDiagnosticOutput`, inspectors, safe formatting, and explicit trust conversion. Include hidden stream identity, delivery ordinal, correlated limit metadata, retained/omitted accounting, truncation, and constructor visibility.
  **Must NOT do**: Do not expose hidden stream/session/host handles. Do not permit direct string writes. Do not treat diagnostic text as authority. Do not let test-only or core/system types receive terminal ABI IDs.

  **Recommended Agent Profile**:
  - Category: `deep` - Reason: large sealed data-model implementation.
  - Skills: [] - no specialized skill required.
  - Omitted: [`frontend-ui-ux`] - no UI.

  **Parallelization**: Can Parallel: NO | Wave 5 | Blocks: 25-32 | Blocked By: 9-17, 23

  **References**:
  - Data-model declarations: `typed_event_session.types.op:10-1165`.
  - Options/capabilities: `typed-event-session/proposal.md:219-228`.
  - Diagnostics/accounting: `typed-event-session/proposal.md:229-240`.
  - Events/accounting: `typed-event-session/proposal.md:241-274`.
  - Output trust: `typed-event-session/proposal.md:292-299`.

  **Acceptance Criteria**:
  - [ ] Unit tests cover every constrained type boundary and constructor visibility path.
  - [ ] Capability inspectors return every ordinary feature, trusted-paste capability, and color capability.
  - [ ] Diagnostic collection accounting matches retained/omitted count/byte/truncation rules including saturation.
  - [ ] Trust conversion is explicit; `SafeTerminalDiagnosticOutput` escapes controls and bounds output.

  **QA Scenarios**:
  ```
  Scenario: Terminal data inspectors are complete
    Tool: Bash
    Steps: run targeted tests for options defaults, capabilities, diagnostics, event variants, and trust conversion
    Expected: all stable inspectors return exact fields; hidden metadata is not inspectable
    Evidence: .sisyphus/evidence/task-24-data-model.txt

  Scenario: Invalid construction and unsafe output are rejected
    Tool: Bash
    Steps: run compile/runtime tests attempting public runtime-only constructors, invalid constrained values, direct string session output, and unsafe diagnostic controls
    Expected: construction/type diagnostics or safe escaped output; no authority leak
    Evidence: .sisyphus/evidence/task-24-data-model-error.txt
  ```

  **Commit**: YES | Message: `feat(terminal): implement selected data model` | Files: [src/runtime/**, src/stdlib/**, src/type_system/**, src/codegen/**, tests/**]

- [x] 25. Implement TerminalSession lifecycle, coordinator state machine, and recovery tokens

  **What to do**: Implement affine `TerminalSession` runtime state (`Active`, `Paused`, `RestorePending`, `Closed`), coordinator state transitions, option validation/open, explicit close, lexical cleanup transfer, failed-open/failed-close process recovery, sealed immutable recovery-token aliases, exact token validation order, one-shot consumption, retryable partial recovery, diagnostics, and generation reservation/exhaustion rules.
  **Must NOT do**: Do not expose sessions in `Opening`, `FailedOpenRecovery`, or `FailedCloseRecovery`. Do not issue recovery tokens for direct close while live binding owns retry authority. Do not allow authority-free recovery or token authority in causes/text.

  **Recommended Agent Profile**:
  - Category: `ultrabrain` - Reason: core lifecycle/recovery state machine is the proposal’s highest-risk runtime area.
  - Skills: [] - no specialized skill required.
  - Omitted: [`frontend-ui-ux`] - no UI.

  **Parallelization**: Can Parallel: NO | Wave 5 | Blocks: 26-32 | Blocked By: 11, 12, 16, 24

  **References**:
  - Lifecycle matrix: `typed-event-session/proposal.md:67-107`.
  - Recovery lifecycle: `typed-event-session/proposal.md:108-145`.
  - Restore errors: `typed_event_session.types.op:1118-1165`.
  - Proposal example: `inspect_terminal_capabilities.op:123-178`.

  **Acceptance Criteria**:
  - [x] Unit tests cover all session operation/state matrix entries and non-mutating state rejection.
  - [x] Recovery-token tests cover WrongSession, WrongKind, Consumed, Stale, RecoveryInProgress, retryable partial failure, and one-shot success.
  - [x] Generation reservation/exhaustion tests prove open preflight behavior and no restore-family generation-exhaustion path.
  - [x] Cleanup transfer returns `CloseRestorePending` only from lexical cleanup transfer path.

  **QA Scenarios**:
  ```
  Scenario: Session state matrix is exact
    Tool: Bash
    Steps: run targeted lifecycle tests through Active, Paused, RestorePending, Closed operations
    Expected: every valid operation succeeds and every invalid state returns matching `TerminalSessionStateError` before mutation
    Evidence: .sisyphus/evidence/task-25-lifecycle.txt

  Scenario: Recovery token authority is exact and one-shot
    Tool: Bash
    Steps: run fake-backend tests producing open rollback and close restore tokens, then wrong-kind/stale/consumed/concurrent retries
    Expected: validation order and mutation/no-mutation effects match proposal
    Evidence: .sisyphus/evidence/task-25-lifecycle-error.txt
  ```

  **Commit**: YES | Message: `feat(terminal): implement session lifecycle and recovery` | Files: [src/runtime/**, src/stdlib/**, src/type_system/**, src/codegen/**, tests/**]

- [x] 26. Implement one-event read, parser queues, input accounting, pause, close, EOF, cancellation

  **What to do**: Implement `terminal_session_readiness_source`, `terminal_session_read_event_sync`, size/read/pause/close behavior, one-event-per-success API, bounded-fair Poll drain semantics, queued events before EOF/cancellation, sticky EOF, sticky identifier exhaustion, pause delivery ending in `InputReset.PauseBoundary`, retained byte/event accounting, close `DiscardedInput`, and no-drop backpressure.
  **Must NOT do**: Do not return hidden batches. Do not starve wait-set cancellation/process/timer sources under sustained input. Do not silently drop consumed undecoded bytes. Do not split correlated Key/TextInput groups.

  **Recommended Agent Profile**:
  - Category: `ultrabrain` - Reason: parser/queue/accounting/cancellation interplay is complex.
  - Skills: [] - no specialized skill required.
  - Omitted: [`frontend-ui-ux`] - no UI.

  **Parallelization**: Can Parallel: NO | Wave 5 | Blocks: 27-32, 33-34 | Blocked By: 13-16, 19-25

  **References**:
  - Readiness/draining: `typed-event-session/proposal.md:146-155`.
  - Events/correlation: `typed-event-session/proposal.md:241-258`.
  - EOF/cancellation/accounting: `typed-event-session/proposal.md:265-274`.
  - Pause fixture requirements: `typed-event-session/proposal.md:329`.

  **Acceptance Criteria**:
  - [x] Tests prove one event per successful read and strict positive finite drain budget in example/harness code.
  - [x] Tests prove `TimedOut` is returned for stale/drained Poll before budget exhaustion.
  - [x] Tests prove pause returns events ending with one `InputReset.PauseBoundary` even with no parser bytes.
  - [x] Tests prove identifier exhaustion is sticky and does not publish reused IDs.
  - [x] Tests prove close `DiscardedInput` reports discarded bytes/events without error.

  **QA Scenarios**:
  ```
  Scenario: One-event draining preserves bounded fairness
    Tool: Bash
    Steps: run deterministic fake-backend test with sustained input, timer readiness, process readiness, and cancellation
    Expected: reads return one event; drain stops at budget or terminal stop; other sources get wait-set opportunities
    Evidence: .sisyphus/evidence/task-26-read.txt

  Scenario: Pause/EOF/cancellation/accounting edge cases
    Tool: Bash
    Steps: run tests for empty pause, queued events before EOF, cancellation after queue empty, identifier exhaustion, close discarded input
    Expected: ordering and accounting match proposal exactly
    Evidence: .sisyphus/evidence/task-26-read-error.txt
  ```

  **Commit**: YES | Message: `feat(terminal): implement one-event input reading` | Files: [src/runtime/**, src/stdlib/**, src/type_system/**, src/codegen/**, tests/**]

- [ ] 27. Implement Linux terminal backend contract

  **What to do**: Implement Linux/VT backend for snapshot/restore, nonblocking descriptor setup, termios flag changes, raw input decoding, SIGWINCH self-pipe/equivalent, poll/ppoll wait integration, resize ordering, EOF/HUP handling, cancellation wake, parser deadlines, output mode/cursor/flush behavior, ledger inverses, and exact restoration.
  **Must NOT do**: Do not flush pending input with `TCSANOW`. Do not clear `ISIG` unless control-key capture is requested. Do not process `POLLHUP` before draining readable bytes. Do not perform non-async-signal-safe SIGWINCH handler work.

  **Recommended Agent Profile**:
  - Category: `deep` - Reason: platform backend with OS-specific invariants.
  - Skills: [] - no specialized skill required.
  - Omitted: [`frontend-ui-ux`] - no UI.

  **Parallelization**: Can Parallel: YES | Wave 5 | Blocks: 31, 33-34 | Blocked By: 19, 20, 24-26

  **References**:
  - Linux contract: `typed-event-session/proposal.md:300-306`.
  - Runtime C integration: `runtime/opal_runtime.c` per architecture memory.
  - Existing terminal e2e style: `tests/integration_e2e/terminal_stdlib.rs:10-230`.

  **Acceptance Criteria**:
  - [ ] Linux unit/integration tests cover termios snapshot/restore, descriptor flags, nonblocking reads, resize self-pipe, input-before-resize ordering, cancellation, HUP/EOF, and inverse ledger order.
  - [ ] Fault injection proves partial restoration enters `RestorePending` with diagnostics and retry authority.
  - [ ] No test leaks raw terminal mode after failure.
  - [ ] Linux terminal fixtures pass under deterministic fake backend and host-backed smoke where CI TTY support exists.

  **QA Scenarios**:
  ```
  Scenario: Linux snapshot/restore and read ordering
    Tool: Bash
    Steps: run Linux backend tests with fake termios/descriptors plus optional pseudo-terminal smoke test
    Expected: exact flag changes/restoration; input drains before resize; EOF sticky after queued data
    Evidence: .sisyphus/evidence/task-27-linux.txt

  Scenario: Linux restoration failure enters retryable state
    Tool: Bash
    Steps: run fake backend fault test failing inverse restoration step
    Expected: `RestorePending` or tokenized cleanup transfer per lifecycle path; diagnostics ordered by attempted inverse
    Evidence: .sisyphus/evidence/task-27-linux-error.txt
  ```

  **Commit**: YES | Message: `feat(terminal): implement linux terminal backend` | Files: [src/runtime/**, runtime/**, src/stdlib/**, tests/**]

- [ ] 28. Implement Windows Console and ConPTY backend contract

  **What to do**: Implement Windows Console and ConPTY/VT backend behavior: snapshot modes/cursor/original screen buffer, restore active buffer before closing alternate, input mode changes, window/mouse mode rules, `WaitForMultipleObjects`, `ReadConsoleInputW`, viewport coordinate normalization, UTF-16 surrogate handling, unknown native records, ConPTY overlapped/VT stream wait integration, cancellation, resize, parser deadlines, and restoration ledger.
  **Must NOT do**: Do not invent Windows process-control source. Do not lossy-convert invalid UTF-16 to replacement text. Do not leave Quick Edit flags changed. Do not close original screen buffer.

  **Recommended Agent Profile**:
  - Category: `deep` - Reason: platform-specific backend requiring cfg/Wine coverage.
  - Skills: [] - no specialized skill required.
  - Omitted: [`frontend-ui-ux`] - no UI.

  **Parallelization**: Can Parallel: YES | Wave 5 | Blocks: 33-34 | Blocked By: 21, 24-27

  **References**:
  - Windows contract: `typed-event-session/proposal.md:308-314`.
  - Existing Windows/Wine tests: `tests/integration_e2e/windows_wine.rs`, `tests/integration_e2e/windows_wine_helpers.rs`.
  - CI command: `cargo test --features "integration windows-wine" --test integration_e2e -- --nocapture wine_msvc_file_ops`.

  **Acceptance Criteria**:
  - [ ] cfg/unit tests cover Console mode/cursor/buffer snapshot and restore order.
  - [ ] Tests cover input records, resize viewport math, mouse coordinates, unpaired surrogate -> `UnknownNative` + `BackendReset`, and ConPTY VT parser path.
  - [ ] Windows process-control construction returns `UnsupportedHost` per Task 21.
  - [ ] Wine/Windows integration command passes or records existing environment skip without failure.

  **QA Scenarios**:
  ```
  Scenario: Windows Console backend preserves state
    Tool: Bash
    Steps: run Windows cfg/unit tests or Wine-backed tests for mode/buffer/cursor snapshot/restore
    Expected: original active buffer restored before alternate close; cursor and modes restored exactly
    Evidence: .sisyphus/evidence/task-28-windows.txt

  Scenario: Windows invalid native input is quarantined
    Tool: Bash
    Steps: run fake Console input tests with unpaired surrogate and unsupported native records
    Expected: bounded `UnknownNative` then one `BackendReset`; no replacement text or command event
    Evidence: .sisyphus/evidence/task-28-windows-error.txt
  ```

  **Commit**: YES | Message: `feat(terminal): implement windows terminal backends` | Files: [src/runtime/**, runtime/**, src/stdlib/**, tests/**]

- [ ] 29. Integrate terminal pause/resume with process-control workflow

  **What to do**: Implement the application-visible workflow support for process-control readiness alongside terminal readiness: stop ordinary work before pause, deliver every pause event through final `PauseBoundary`, acknowledge suspend only after pause boundary, wait only for matching `Continued` while paused, resume terminal, resume application process, then allow redraw/work. Add deterministic tests and update examples if needed.
  **Must NOT do**: Do not read terminal input, expire chords, render, or resume child/application work while awaiting continuation. Do not acknowledge if pause fails and leaves session `Active`. Do not resume application if terminal resume enters `RestorePending`.

  **Recommended Agent Profile**:
  - Category: `deep` - Reason: cross-source workflow sequencing.
  - Skills: [] - no specialized skill required.
  - Omitted: [`frontend-ui-ux`] - no UI.

  **Parallelization**: Can Parallel: YES | Wave 5 | Blocks: 33 | Blocked By: 21, 24-28

  **References**:
  - Workflow: `typed-event-session/proposal.md:275-290`.
  - Example: `run_editor_event_loop.op:119-202`.
  - Wait/cancellation rules: `core-prerequisites.md:150-164`, `core-prerequisites.md:231-245`.

  **Acceptance Criteria**:
  - [ ] Integration tests cover normal suspend/continue, stale process wake `Idle`, duplicate/out-of-order notifications, pause failure, resume failure, and Windows unsupported source.
  - [ ] Tests prove no terminal read or timer/chord expiry occurs while awaiting matching continuation.
  - [ ] Fail-closed cases propagate structured errors and keep application work stopped.
  - [ ] Example/harness follows exact order from proposal.

  **QA Scenarios**:
  ```
  Scenario: Suspend/continue workflow order is exact
    Tool: Bash
    Steps: run fake process-control + terminal session integration test through SuspendRequested and Continued
    Expected: stop work -> pause -> deliver PauseBoundary -> acknowledge -> wait Continued -> terminal resume -> process resume -> redraw/work
    Evidence: .sisyphus/evidence/task-29-process-terminal.txt

  Scenario: Failure cases fail closed
    Tool: Bash
    Steps: run tests for pause Active failure, RestorePending, terminal resume failure, stale continuation, and Windows unsupported source
    Expected: no implicit acknowledgement/resume/work restart; structured error propagates
    Evidence: .sisyphus/evidence/task-29-process-terminal-error.txt
  ```

  **Commit**: YES | Message: `feat(terminal): integrate process-control workflow` | Files: [src/runtime/**, src/stdlib/**, tests/**]

- [ ] 30. Enforce session output trust boundary and legacy I/O regressions end-to-end

  **What to do**: Wire `terminal_session_write_sync`, diagnostic write, flush, cursor visibility/shape, and safe diagnostic formatting through the trust boundary and coordinator. Add compile-fail/security tests rejecting direct strings, implicit conversions, session-derived `StdoutTerminal`, stale legacy leases, and public host handles. Re-run existing stdout/text/writer/terminal integration suites.
  **Must NOT do**: Do not add `terminal_session_output_terminal`. Do not create `AcquireOutputTerminal`. Do not let environment/input/diagnostic text become trusted output without explicit application conversion.

  **Recommended Agent Profile**:
  - Category: `unspecified-high` - Reason: security boundary plus regressions.
  - Skills: [] - no specialized skill required.
  - Omitted: [`frontend-ui-ux`] - no UI.

  **Parallelization**: Can Parallel: YES | Wave 5 | Blocks: 33, 35 | Blocked By: 22, 24-27

  **References**:
  - Output trust: `typed-event-session/proposal.md:292-299`.
  - No output terminal: `typed-event-session/proposal.md:215-218`.
  - Legacy API inventory: `core-prerequisites.md:61-91`.
  - Existing tests: `tests/integration_e2e/stdout_text_stdlib.rs`, `stdout_writer_stdlib.rs`, `terminal_stdlib.rs`.

  **Acceptance Criteria**:
  - [ ] Compile-fail tests reject direct `string` session writes and implicit conversions.
  - [ ] Compile-fail/import tests confirm no `terminal_session_output_terminal` or `AcquireOutputTerminal` exists.
  - [ ] Runtime tests prove diagnostic write escapes unsafe controls and bounds output.
  - [ ] Existing stdout/terminal integration tests pass in Free state and coordinator-unavailable states.

  **QA Scenarios**:
  ```
  Scenario: Trusted output boundary blocks raw text
    Tool: Bash
    Steps: run compile-fail tests passing raw string/input/diagnostic text to session write APIs
    Expected: exact type errors; only `TrustedTerminalOutput` or `SafeTerminalDiagnosticOutput` accepted
    Evidence: .sisyphus/evidence/task-30-output-trust.txt

  Scenario: Legacy output regressions remain stable
    Tool: Bash
    Steps: run targeted integration tests for stdout text, stdout writer, and terminal stdlib
    Expected: Free-state behavior unchanged; coordinator-active behavior rejects before mutation
    Evidence: .sisyphus/evidence/task-30-output-trust-regression.txt
  ```

  **Commit**: YES | Message: `feat(terminal): enforce output trust boundary` | Files: [src/runtime/**, src/stdlib/**, src/type_system/**, src/codegen/**, tests/**]

- [ ] 31. Implement test-only terminal factories, fake backend, and activation scopes

  **What to do**: Implement `standard.testing.terminal` authority, `TerminalTestScenario`, synthetic event/capability/diagnostic factories, diagnostic collection factory, deterministic fake backend, backend binding/activation, task-local LIFO activation, fault matching, and authentic recovery-token production only through ordinary lifecycle faults. Integrate with gated fixtures and test harness.
  **Must NOT do**: Do not allow test factories to construct `TerminalSession`, recovery tokens, coordinator leases, host handles, `TimedOut`, `Cancelled`, or `EndOfInput` directly. Do not leak test-only symbols into production artifacts.

  **Recommended Agent Profile**:
  - Category: `deep` - Reason: test authority must preserve production invariants.
  - Skills: [] - no specialized skill required.
  - Omitted: [`frontend-ui-ux`] - no UI.

  **Parallelization**: Can Parallel: NO | Wave 5 | Blocks: 33-35 | Blocked By: 17, 24-30

  **References**:
  - Test-only API: `TESTING.md:15-71`.
  - Factory invariants: `TESTING.md:75-110`.
  - Fake backend/recovery: `TESTING.md:111-134`.
  - Declarations: `terminal_testing.types.op:1-427`.

  **Acceptance Criteria**:
  - [ ] Factory tests cover event IDs, composition IDs, trusted-paste evidence, every synthetic event category, stream identity, hidden delivery ordinal, wrong-stream/replay/out-of-order scenarios, and delivery ordinal exhaustion.
  - [ ] Capability/diagnostic factory tests cover duplicate/missing/unknown ordinary features, compatible evidence, illegal coordinator/session pairing, collection truncation/accounting, and saturation.
  - [ ] Fake backend tests cover binding, activation LIFO, nested distinct scenarios, same-scenario/other-task rejection, deterministic lifecycle faults, and authentic recovery-token generation via ordinary errors.
  - [ ] Production import/export tests for test-only symbols fail.

  **QA Scenarios**:
  ```
  Scenario: Test factories preserve sealed invariants
    Tool: Bash
    Steps: run targeted `standard.testing.terminal` tests constructing events/capabilities/diagnostics from one scenario and attempting cross-scenario misuse
    Expected: valid values carry hidden identity/order; invalid relationships return `TerminalTestFactoryError`; no handles/tokens exposed
    Evidence: .sisyphus/evidence/task-31-test-backend.txt

  Scenario: Fake backend recovery tokens are authentic lifecycle outputs
    Tool: Bash
    Steps: run deterministic fault plan with Open(1) AfterMutation and inverse-step failure
    Expected: ordinary `RollbackFailed { recovery_token }` is produced; backend does not construct token directly
    Evidence: .sisyphus/evidence/task-31-test-backend-error.txt
  ```

  **Commit**: YES | Message: `feat(testing): add terminal fake backend support` | Files: [src/runtime/**, src/stdlib/**, src/type_system/**, src/codegen/**, tests/**]

- [ ] 32. Implement terminal chord router and application-side lifecycle support

  **What to do**: Implement selected companion chord API: modifiers, chord construction, sequence append, router construction from authenticated `TerminalCapabilities`, registration/replace/unregister, binding IDs, process/expire/reset, released input accessors, hidden stream/delivery-order validation, prefix policies, timer deadline outputs, capacity recovery protocol, no-callback side-map separation, and affine router cleanup. Add tests using test-only scenario events.
  **Must NOT do**: Do not store application commands, callbacks, closures, payloads, module objects, or map nodes inside router. Do not interpret TextInput/paste/unknown/native/reset events as commands. Do not split correlated groups or create a scheduler/timer inside router.

  **Recommended Agent Profile**:
  - Category: `ultrabrain` - Reason: chord ordering/capacity/timer recovery is complex.
  - Skills: [] - no specialized skill required.
  - Omitted: [`frontend-ui-ux`] - no UI.

  **Parallelization**: Can Parallel: NO | Wave 5 | Blocks: 34-35 | Blocked By: 18, 20, 24, 27, 31

  **References**:
  - Chord API and semantics: `CHORDS.md:9-117`.
  - Chord declarations: `terminal_chords.types.op:1-293`.
  - Example: `configure_editor_chords.op:76-340`.
  - ABI history: `abi-history.md:121-178` chord inventory and representation notes.

  **Acceptance Criteria**:
  - [ ] Tests cover Control/Named/Function/EnhancedText construction, modifiers, lock masks, sequence limits, duplicate/prefix ambiguity policies, registration count, ID ordinal preservation/retirement, replace/unregister atomicity, and capability validation.
  - [ ] Process tests cover all event categories in expected order, wrong stream, already consumed, out of order, capacity overflow, allocation failure before mutation, release-once, activation-once, and one activation per Key occurrence count.
  - [ ] Timer tests cover Pending deadline, AwaitingCorrelatedInput disarm, stale wake filtering, Idle, expiration allocation failure, reset release order, and generation-exhaustion spare/retarget recovery.
  - [ ] Router cleanup never touches application side-map payloads and releases buffered input/registrations deterministically.

  **QA Scenarios**:
  ```
  Scenario: Chord router activates and releases input exactly once
    Tool: Bash
    Steps: run test-only synthetic stream through Ctrl-Q, Escape, mismatches, linked text, paste, unknown, reset, and EndOfInput
    Expected: commands activate only for Key events; released input appears once in original order
    Evidence: .sisyphus/evidence/task-32-chords.txt

  Scenario: Chord capacity and timer recovery is deterministic
    Tool: Bash
    Steps: run tests for `BufferedCapacityExceeded`, timer `GenerationExhausted`, spare swap, owned-registration retarget failure/success, stale old-source wake
    Expected: current event retries only after release/timer safety; effects never replay; fairness positions preserved
    Evidence: .sisyphus/evidence/task-32-chords-error.txt
  ```

  **Commit**: YES | Message: `feat(terminal): implement chord router` | Files: [src/runtime/**, src/stdlib/**, src/type_system/**, src/codegen/**, tests/**]

- [ ] 33. Activate and pass required terminal session fixtures 1-8

  **What to do**: Ungate/activate `terminal-key-log`, `terminal-text-echo-safe`, `terminal-size-probe`, `terminal-pause-counter`, `terminal-legacy-io-rejection`, `terminal-timeout-menu`, `terminal-cancel-demo`, and `terminal-diagnostics-inspector` in the normal integration harness. Ensure each uses the real selected public signatures, deterministic fake backend, expected stdout/stderr/status, and final summary lines.
  **Must NOT do**: Do not weaken fixture source, replace selected APIs with shims, or keep these tests ignored after implementation.

  **Recommended Agent Profile**:
  - Category: `deep` - Reason: broad fixture activation and end-to-end debugging.
  - Skills: [] - no specialized skill required.
  - Omitted: [`frontend-ui-ux`] - no browser UI.

  **Parallelization**: Can Parallel: YES | Wave 6 | Blocks: 36-37 | Blocked By: 3, 4, 24-31

  **References**:
  - Fixture obligations: `typed-event-session/proposal.md:326-333`.
  - Harness patterns: `tests/integration_e2e/tests.rs`, `interactive_io.rs`, `terminal_stdlib.rs`.
  - Isolation: `tests/integration_e2e/fs_helpers.rs`, `fs_state_guard.rs`.

  **Acceptance Criteria**:
  - [ ] All eight fixtures are active in the normal integration suite and no longer gated/ignored.
  - [ ] Each fixture final summary line exactly matches harness expectation.
  - [ ] Security companion test for `terminal-text-echo-safe` rejects direct string session writes.
  - [ ] `cargo test --features integration` passes for these fixtures.

  **QA Scenarios**:
  ```
  Scenario: Core terminal fixtures pass end-to-end
    Tool: Bash
    Steps: run targeted integration tests for fixtures 1-8 by name, then `cargo test --features integration`
    Expected: all eight build/run with expected stdout/stderr/status and final summaries
    Evidence: .sisyphus/evidence/task-33-fixtures-1-8.txt

  Scenario: Fixture source remains selected-proposal compliant
    Tool: Bash
    Steps: run static checks for direct string writes, host-language shims, private escape hatches, and historical API names in fixtures 1-8
    Expected: no forbidden patterns found
    Evidence: .sisyphus/evidence/task-33-fixtures-1-8-error.txt
  ```

  **Commit**: YES | Message: `test(terminal): activate core terminal fixtures` | Files: [test-projects/terminal-*/**, tests/**]

- [ ] 34. Activate and pass required terminal session fixtures 9-14

  **What to do**: Ungate/activate `terminal-chord-quit`, `terminal-paste-quarantine`, `terminal-game-of-life-interactive`, `terminal-sokoban-mini`, `terminal-file-picker`, and `terminal-stopwatch-pomodoro` in the normal integration harness. Ensure deterministic fake backend input/timer plans, trusted output conversion, exact summaries, and no external nondeterminism.
  **Must NOT do**: Do not depend on real time, randomness, host directory order, editor buffers, callbacks, or historical APIs. Do not leave these tests ignored.

  **Recommended Agent Profile**:
  - Category: `deep` - Reason: application-level fixture activation across terminal/chord/timer/file APIs.
  - Skills: [] - no specialized skill required.
  - Omitted: [`frontend-ui-ux`] - terminal rendering is deterministic bytes/summaries.

  **Parallelization**: Can Parallel: YES | Wave 6 | Blocks: 36-37 | Blocked By: 5, 24-32

  **References**:
  - Fixture obligations: `typed-event-session/proposal.md:334-339`.
  - Chord contract: `CHORDS.md:115-117`.
  - Existing game fixture style: `test-projects/game-of-life-full/`, `tests/integration_e2e/game_of_life.rs`.
  - File isolation: `tests/integration_e2e/fs_helpers.rs`.

  **Acceptance Criteria**:
  - [ ] All six fixtures are active in normal integration suite and no longer gated/ignored.
  - [ ] `terminal-file-picker` uses explicit order fixture/manifest and deterministic selected file.
  - [ ] `terminal-stopwatch-pomodoro` uses deterministic short test ticks, not real duration.
  - [ ] `cargo test --features integration` passes for all 14 terminal fixtures.

  **QA Scenarios**:
  ```
  Scenario: Interactive terminal fixtures pass end-to-end
    Tool: Bash
    Steps: run targeted integration tests for fixtures 9-14 by name, then run all terminal fixture integration tests
    Expected: each fixture passes with exact final summary line
    Evidence: .sisyphus/evidence/task-34-fixtures-9-14.txt

  Scenario: No nondeterministic fixture dependencies remain
    Tool: Bash
    Steps: run static checks for randomness, real-time sleeps, host directory iteration, callback APIs, historical packet/batch names, and editor-buffer dependencies in fixtures 9-14
    Expected: no forbidden dependencies found
    Evidence: .sisyphus/evidence/task-34-fixtures-9-14-error.txt
  ```

  **Commit**: YES | Message: `test(terminal): activate interactive terminal fixtures` | Files: [test-projects/terminal-*/**, tests/**]

- [ ] 35. Add security, compile-fail, and scope-fidelity regression suite

  **What to do**: Add comprehensive compile-fail/security tests for forged sealed values/evidence, wrong-session/stale/replayed recovery tokens, direct string writes, implicit trust conversion, stale legacy leases, public host handles, production test-only imports, affine/ref escapes, chord wrong-stream/replay/out-of-order events, historical API names, and scope-exclusion attempts.
  **Must NOT do**: Do not rely on runtime-only checks where the compiler should reject. Do not leave errors vague. Do not permit compatibility shims for historical alternatives.

  **Recommended Agent Profile**:
  - Category: `unspecified-high` - Reason: adversarial negative coverage across compiler/runtime.
  - Skills: [] - no specialized skill required.
  - Omitted: [`frontend-ui-ux`] - no UI.

  **Parallelization**: Can Parallel: YES | Wave 6 | Blocks: 36-37 | Blocked By: 17, 30-32

  **References**:
  - Exclusions: `typed-event-session/proposal.md:343-351`.
  - Historical alternatives deference: `batched-event-pump/proposal.md:3-30`, `portable-input-packet-stream/proposal.md:3-8`.
  - Compile-fail harness: `tests/integration_e2e/compile_failures.rs`.
  - Security obligations: `TESTING.md:75-134`, `CHORDS.md:85-113`.

  **Acceptance Criteria**:
  - [ ] Compile-fail suite covers every forbidden construction/import/output/trust/historical API class listed in this task.
  - [ ] Runtime security tests cover token validation, stale lease rejection, fake backend sealed authority, and chord stream/order rejection.
  - [ ] Diagnostics are precise enough to distinguish availability, constructor visibility, affine ownership, trust boundary, ABI, and coordinator errors.
  - [ ] `cargo test --features integration` passes.

  **QA Scenarios**:
  ```
  Scenario: Forbidden APIs and trust bypasses fail
    Tool: Bash
    Steps: run compile-fail tests for historical API names, direct strings, implicit conversion, public host handles, production test-only imports, sealed constructors
    Expected: exact expected diagnostics; no artifacts emitted
    Evidence: .sisyphus/evidence/task-35-security.txt

  Scenario: Runtime authority attacks fail without mutation
    Tool: Bash
    Steps: run tests for wrong-session/stale/replayed tokens, stale leases, forged evidence, wrong-stream/replayed chord events
    Expected: structured rejection before mutation/authority consumption
    Evidence: .sisyphus/evidence/task-35-security-error.txt
  ```

  **Commit**: YES | Message: `test(terminal): add security and scope regressions` | Files: [tests/**, test-projects/** as needed]

- [ ] 36. Update documentation, examples, ABI history, and traceability evidence

  **What to do**: Update user-facing docs (`README.md`, `STDLIB.md`, `CONTRIBUTING.md`, and any standard-library reference files) to describe implemented selected terminal support, prerequisites, test-only availability, branch/commit evidence, platform support, and explicit exclusions. Update `abi-history.md` only if implementation/declaration changes require same-revision history entries. Fill all commit hashes/statuses in `.sisyphus/evidence/terminal-session-input-traceability.md`.
  **Must NOT do**: Do not advertise historical alternatives as implemented. Do not claim Windows process-control support. Do not alter append-only ABI history except with evidenced retirements/representation records.

  **Recommended Agent Profile**:
  - Category: `writing` - Reason: documentation and traceability writing with technical accuracy.
  - Skills: [] - no specialized skill required.
  - Omitted: [`frontend-ui-ux`] - no UI design.

  **Parallelization**: Can Parallel: YES | Wave 6 | Blocks: 37 | Blocked By: 10, 24-35

  **References**:
  - README proposal status section already references terminal-session-input.
  - `stdlib-proposals/terminal-session-input/COMPARISON.md:7-18` reading order.
  - `abi-history.md:25-43` append-only authority rules.
  - Docs style: `STDLIB.md`, `CONTRIBUTING.md`.

  **Acceptance Criteria**:
  - [ ] Docs state selected `typed-event-session` is implemented and historical alternatives remain non-v1 records.
  - [ ] Docs list Windows process-control as unsupported per contract.
  - [ ] Traceability matrix has no empty Task(s), Test(s), Commit hash, or Status cells.
  - [ ] ABI validation still passes after docs/history updates.
  - [ ] `cargo fmt --all -- --check` passes.

  **QA Scenarios**:
  ```
  Scenario: Documentation matches implemented scope
    Tool: Bash
    Steps: run doc/static checks for `typed-event-session`, historical alternative exclusions, test-only availability, Windows process-control unsupported text
    Expected: docs are internally consistent and do not overclaim
    Evidence: .sisyphus/evidence/task-36-docs.txt

  Scenario: Traceability matrix is complete
    Tool: Bash
    Steps: run script or manual command to detect blank Task/Test/Commit/Status cells in `.sisyphus/evidence/terminal-session-input-traceability.md`
    Expected: no blanks; every obligation maps to completed task/test/commit
    Evidence: .sisyphus/evidence/task-36-traceability.txt
  ```

  **Commit**: YES | Message: `docs(terminal): document selected terminal session support` | Files: [README.md, STDLIB.md, CONTRIBUTING.md, stdlib-proposals/terminal-session-input/typed-event-session/abi-history.md if needed, .sisyphus/evidence/terminal-session-input-traceability.md]

- [ ] 37. Run final full-suite verification and atomic-commit audit

  **What to do**: Run all final commands, inspect git history for atomic/bisect-green commits, verify branch ancestry, verify no uncommitted changes, verify no unapproved dependencies, verify all 14 fixtures active, and update final evidence. Do not mark final verification wave complete until review agents and user approval are done.
  **Must NOT do**: Do not skip slow/all-features tests. Do not mark F1-F4 checked before user approval. Do not squash/amend/rebase unless user explicitly requests.

  **Recommended Agent Profile**:
  - Category: `unspecified-high` - Reason: comprehensive verification/audit.
  - Skills: [`git-master`] - commit/audit discipline.
  - Omitted: [`frontend-ui-ux`] - no UI.

  **Parallelization**: Can Parallel: NO | Wave 6 | Blocks: Final Verification Wave | Blocked By: 1-36

  **References**:
  - CI commands: `.github/workflows/ci.yml`.
  - Local tasks: `Makefile.toml`.
  - Test strategy memory: `cargo test`, `cargo test --features integration`, all-features, clippy, fmt.

  **Acceptance Criteria**:
  - [ ] `git rev-parse --abbrev-ref HEAD` outputs `feature/terminal-session-input-typed-event-session`.
  - [ ] `git merge-base --is-ancestor main HEAD` exits `0`.
  - [ ] `git status --short` is empty.
  - [ ] `cargo fmt --all -- --check` passes.
  - [ ] `cargo clippy --all-targets --all-features -- -D warnings` passes.
  - [ ] `timeout 900 cargo test --all-features` passes.
  - [ ] `cargo test --features integration` passes.
  - [ ] `cargo make c-quality` passes.
  - [ ] All 14 `test-projects/terminal-*` fixtures are active and not ignored.

  **QA Scenarios**:
  ```
  Scenario: Full verification suite passes
    Tool: Bash
    Steps: run fmt, clippy, all-features tests, integration tests, cargo-make C quality, and Wine command where environment supports it
    Expected: all required commands exit 0 or Wine command records existing environment skip without failure
    Evidence: .sisyphus/evidence/task-37-final-suite.txt

  Scenario: Atomic commit and dependency audit passes
    Tool: Bash
    Steps: inspect `git log --oneline main..HEAD`, `git diff main..HEAD -- Cargo.toml Cargo.lock`, branch ancestry, and traceability commit hashes
    Expected: atomic commits are present, no unapproved dependencies, branch is based on main, traceability hashes match log
    Evidence: .sisyphus/evidence/task-37-commit-audit.txt
  ```

  **Commit**: YES | Message: `test(terminal): verify selected terminal session implementation` | Files: [.sisyphus/evidence/** if repository policy allows evidence commits; otherwise no source files]

## Final Verification Wave (MANDATORY — after ALL implementation tasks)
> 4 review agents run in PARALLEL. ALL must APPROVE. Present consolidated results to user and get explicit "okay" before completing.
> **Do NOT auto-proceed after verification. Wait for user's explicit approval before marking work complete.**
> **Never mark F1-F4 as checked before getting user's okay.** Rejection or user feedback -> fix -> re-run -> present again -> wait for okay.
- [ ] F1. Plan Compliance Audit — oracle
  - Check every task acceptance criterion, evidence file, commit hash, and traceability row against this plan.
- [ ] F2. Code Quality Review — unspecified-high
  - Review Rust architecture, module boundaries, error handling, allocations, platform abstractions, and absence of AI slop.
- [ ] F3. Real Manual QA — unspecified-high
  - Execute terminal integration scenarios via the deterministic fake backend and existing integration harness; use Playwright only if a UI artifact appears, otherwise Bash.
- [ ] F4. Scope Fidelity Check — deep
  - Verify selected `typed-event-session` only, no historical API revival, no unapproved dependencies, and no unrelated language/runtime expansion.

## Commit Strategy
- Use `/git-master` workflow for all branch and commit operations.
- Start from a clean `main` and create `feature/terminal-session-input-typed-event-session`.
- Every source-changing task commits exactly the files it intentionally modifies.
- Commit messages use `type(scope): desc`, examples in each task.
- Commits must be bisect-green. RED TDD evidence is captured in `.sisyphus/evidence/` using commands that assert expected failure; default `cargo test` must not be broken by committed gated fixtures.
- Before each commit: inspect `git status`, `git diff`, and relevant test output; stage only intended files; never commit secrets or generated build artifacts.

## Success Criteria
- All Definition of Done commands pass.
- Every task checkbox is complete with evidence.
- The 14 required fixtures are active, green, and listed in traceability.
- Terminal API adoption is all-or-nothing with core prerequisites; no half-exposed public surface remains.
- Final verification agents F1-F4 approve and the user explicitly okays completion.
