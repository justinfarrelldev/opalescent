# Stdlib APIs and Bounded Game of Life

## TL;DR
> **Summary**: Add additive stdlib/runtime APIs for string construction, explicit fallible output, synchronous timing, terminal drawing, and a deterministic 10-frame Game of Life test project. Execute as vertical TDD slices where every slice has RED, GREEN, and REFACTOR gates before moving on.
> **Deliverables**:
> - `string_join(values, separator)` and `StringBuilder` created with `new StringBuilder`
> - `print_text_sync`, `flush_standard_output_sync`, `StdoutWriter`, `stdout_writer`, `writer_write_sync`, `writer_flush_sync`
> - `sleep_ms_sync`, `FrameClock`, `frame_clock_wait_next_sync`
> - `StdoutTerminal`, `stdout_terminal`, `terminal_supports_ansi`, handle-based terminal draw APIs, plus convenience `terminal_clear_screen_sync` and `terminal_move_cursor_sync`
> - `test-projects/game-of-life` bounded to exactly 10 frames with golden fixture verification
> **Effort**: Large
> **Parallel**: YES - 5 waves after foundation, but each API slice must finish RED → GREEN → REFACTOR before dependent slices start.
> **Critical Path**: Task 1 → Task 2 → Task 3 → Task 4 → Task 5 → Task 6 → Task 7 → Task 8 → Final Verification

## Context
### Original Request
Implement stdlib changes for string building, print/output, sleep/timing, and terminal APIs, then use them to create a Game of Life test project. The Game of Life must never run indefinitely; use small bounded programs such as 10 frames. The work must be thoroughly quality-controlled and use TDD with red-green-refactor during every step. Use existing stdlib file functions for file operations and stop if impossible. Report conflicting names or other issues.

### Interview Summary
- Existing repo exploration confirmed file stdlib functions exist via fs integration tests, so no new file primitives are needed.
- Existing `print` is newline-oriented and already used in fixtures; preserve it byte-for-byte.
- User prefers `StdoutWriter` as the public opaque writer handle instead of `TextWriter`.
- `new StringBuilder` is mandatory and should follow the Bytes propertyless-constructor pattern.
- Game of Life must be deterministic, fixed-board, exactly 10 frames, and tested via golden output.

### Metis Review (gaps addressed)
- Added concrete handle semantics: `string_builder_finish` consumes the builder; future pushes/finish calls return `BuilderFinishedError` rather than UB.
- Added writer semantics: `stdout_writer()` returns a fresh lightweight opaque handle to the shared stdout sink; byte ordering is deterministic because all new stdout APIs use the same C `stdout` stream and explicit flushing where requested.
- Added terminal handle origin: `stdout_terminal()` returns a fresh opaque handle over stdout; it shares the same stdout stream as `print_text_sync` and `StdoutWriter`.
- Added error taxonomy and exact terminal escape bytes.
- Added strict TDD substeps with exact RED/GREEN/REFACTOR commands per task.
- Added timing-test tolerances to prevent CI flakiness.
- Added terminal scope guardrails: no color, raw mode, input handling, styling, or alternate screen.

## Work Objectives
### Core Objective
Add production-shaped, additive stdlib/runtime APIs needed for bounded terminal demos while preserving existing behavior and matching the current Bytes-style architecture.

### Deliverables
- New public standard symbols and typechecker signatures.
- C runtime implementations and `runtime/opal_runtime.h` declarations.
- Codegen stdlib declarations in `src/codegen/functions_stdlib.rs` using the existing error aggregate ABI.
- Rust-side stdlib/reference tests where applicable.
- Integration e2e tests and fixtures.
- Bounded Game of Life test project using the new APIs.

### Definition of Done (verifiable conditions with commands)
- `cargo test` exits 0.
- `cargo test --features integration --test integration_e2e -- --nocapture` exits 0.
- `timeout 900 cargo test --all-features` exits 0.
- `cargo clippy --all-targets --all-features -- -D warnings` exits 0.
- `cargo fmt --all -- --check` exits 0.
- Game of Life integration test proves exactly 10 frames and no unbounded loop.

### Must Have
- Additive APIs only; preserve existing `print` behavior.
- Fallible APIs use existing aggregate error ABI: `{value, i8* error}` or void-like `{i8*, i8*}`.
- Every task uses RED → GREEN → REFACTOR and records evidence under `.sisyphus/evidence/`.
- Every runtime handle has explicit invalid-use behavior; no UB.
- Terminal cursor coordinates are 0-based in Opal API and converted to ANSI 1-based in runtime.

### Must NOT Have (guardrails, AI slop patterns, scope boundaries)
- Do not repurpose or rewrite existing `print` semantics.
- Do not add color, raw mode, alternate screen, keyboard input, styling, or terminal UI framework features.
- Do not add StringBuilder capacity/reserve APIs.
- Do not add new file primitives.
- Do not create any indefinite Game of Life loop.
- Do not accept manual visual checks as QA.

## Verification Strategy
> ZERO HUMAN INTERVENTION - all verification is agent-executed.
- Test decision: TDD RED-GREEN-REFACTOR + Rust test harness + `integration_e2e` golden tests.
- Test filter policy: commands using `-- --exact` must use fully qualified Rust test names. For tests inside `tests/integration_e2e/tests.rs` submodules, use `tests::<module>::<test_name>` (example: `tests::fs_markdown_roundtrip::fs_markdown_roundtrip`). For `tests/integration_print.rs`, use `tests::<test_name>` (example: `tests::print_types_compiles_links_and_runs`). If the executor intentionally places a new test at crate root, the plan must state that placement explicitly before using a bare exact name.
- QA policy: Every task has agent-executed scenarios.
- Evidence: `.sisyphus/evidence/task-{N}-{slug}.{ext}`.
- Timing policy: exactly one `sleep_ms_sync(50)` elapsed-time test with bounds `>=45ms` and `<=5000ms`; one `FrameClock(30)` test with 10 waits bounded `>=280ms` and `<=5000ms`.
- ANSI policy: assert exact stdout bytes, not visual terminal state. Chosen bytes: clear screen emits `\x1b[2J\x1b[H`; move cursor emits `\x1b[{row+1};{col+1}H`.

## Execution Strategy
### Parallel Execution Waves
> Target: 5-8 tasks per wave. <3 per wave (except final) = under-splitting.
> Extract shared dependencies as Wave-1 tasks for max parallelism.

Wave 1: Task 1 foundation string join and registration baseline.
Wave 2: Task 2 StringBuilder handle.
Wave 3: Task 3 print/flush plus Task 4 StdoutWriter after Task 3 green.
Wave 4: Task 5 sleep plus Task 6 FrameClock after Task 5 green.
Wave 5: Task 7 terminal APIs after Task 3 green.
Wave 6: Task 8 Game of Life after Tasks 1-7 green.
Wave 7: Final Verification Wave F1-F4.

### Dependency Matrix (full, all tasks)
| Task | Depends On | Blocks |
| --- | --- | --- |
| 1 string_join | none | 2, 8 |
| 2 StringBuilder | 1 | 8 |
| 3 print_text/flush | none | 4, 7, 8 |
| 4 StdoutWriter | 3 | 8 |
| 5 sleep_ms_sync | none | 6, 8 |
| 6 FrameClock | 5 | 8 |
| 7 Terminal APIs | 3 | 8 |
| 8 Game of Life | 1,2,3,4,5,6,7 | Final Verification |

### Agent Dispatch Summary (wave → task count → categories)
| Wave | Task Count | Categories |
| --- | ---: | --- |
| 1 | 1 | deep |
| 2 | 1 | deep |
| 3 | 2 | deep |
| 4 | 2 | deep |
| 5 | 1 | deep |
| 6 | 1 | deep |
| 7 | 4 review agents | oracle, unspecified-high, unspecified-high, deep |

## TODOs
> Implementation + Test = ONE task. Never separate.
> EVERY task MUST have: Agent Profile + Parallelization + QA Scenarios.

- [x] 1. Add `string_join(values, separator)`

  **What to do**: Add pure string joining as the first low-risk stdlib vertical slice. Public Opal signature: `string_join(values: string[], separator: string): string`. Runtime C symbol: `string_join`. Semantics: empty array returns empty string; one item returns that item unchanged; N items concatenate items with separator between items only; separator may be empty; no trailing separator. Preserve immutable result semantics.

  **TDD — RED**:
  - Add a Rust unit/type visibility test proving `string_join` resolves from `standard` and a compile-run integration test `string_join_basic_smoke` using a fixture program that prints `alpha,beta,gamma`.
  - Run: `cargo test --features integration --test integration_e2e tests::string_join_stdlib::string_join_basic_smoke -- --exact`.
  - Expected RED: fails because `string_join` is unresolved or codegen cannot declare the runtime symbol.

  **TDD — GREEN**:
  - Add typechecker/module resolver symbols.
  - Add `STDLIB_NAMES` entry and LLVM declaration in `src/codegen/functions_stdlib.rs`.
  - Implement C runtime function and header declaration.
  - Add Rust-side pure helper tests in a new or existing string stdlib module selected by local module organization; if no string module exists, create one consistent with `src/stdlib/bytes.rs` and wire it through `src/stdlib.rs`.

  **TDD — REFACTOR**:
  - Deduplicate string allocation helpers with existing string runtime utilities where safe.
  - Run focused tests, then `cargo fmt --all -- --check` and `cargo clippy --all-targets --all-features -- -D warnings`.

  **Must NOT do**: Do not introduce `StringBuilder` in this task. Do not mutate input arrays. Do not change interpolation behavior.

  **Recommended Agent Profile**:
  - Category: `deep` - Reason: Requires coordinated typechecker, codegen, runtime, and tests.
  - Skills: [] - No special skill required.
  - Omitted: [`frontend-ui-ux`] - No UI work.

  **Parallelization**: Can Parallel: NO | Wave 1 | Blocks: [2, 8] | Blocked By: []

  **References** (executor has NO interview context - be exhaustive):
  - Pattern: `src/type_system/checker/bytes_builtins.rs:31-74` - builtin registration shape.
  - Pattern: `src/type_system/module_resolver/standard_symbols_core_io_and_bytes.rs` - standard symbol provider entries.
  - Pattern: `src/codegen/functions_stdlib.rs:602-692` - `STDLIB_NAMES` registry.
  - Pattern: `tests/integration_e2e/bytes_stdlib.rs` - compile/link/run integration harness.
  - Pattern: `runtime/opal_runtime.h` - runtime prototypes.

  **Acceptance Criteria** (agent-executable only):
  - [ ] `cargo test --features integration --test integration_e2e tests::string_join_stdlib::string_join_basic_smoke -- --exact` exits 0 and stdout assertion equals `alpha,beta,gamma\n`.
  - [ ] `cargo test --features integration --test integration_e2e tests::string_join_stdlib::string_join_empty_and_single -- --exact` exits 0.
  - [ ] `cargo test` exits 0.

  **QA Scenarios** (MANDATORY - task incomplete without these):
  ```
  Scenario: Happy path joins three rows
    Tool: Bash
    Steps: cargo test --features integration --test integration_e2e tests::string_join_stdlib::string_join_basic_smoke -- --exact
    Expected: exit 0; test output contains `test string_join_basic_smoke ... ok`; captured binary stdout equals `alpha,beta,gamma\n`.
    Evidence: .sisyphus/evidence/task-1-string-join.txt

  Scenario: Empty and single item arrays
    Tool: Bash
    Steps: cargo test --features integration --test integration_e2e tests::string_join_stdlib::string_join_empty_and_single -- --exact
    Expected: exit 0; empty array renders empty string and single item has no separator.
    Evidence: .sisyphus/evidence/task-1-string-join-edge.txt
  ```

  **Commit**: YES | Message: `feat(stdlib): add string_join` | Files: [`src/type_system/**`, `src/codegen/functions_stdlib.rs`, `runtime/opal_runtime.h`, `runtime/*.c`, `tests/integration_e2e/**`, `test-projects/**`]

- [x] 2. Add `StringBuilder` with `new StringBuilder`

  **What to do**: Add an opaque `StringBuilder` handle following the Bytes initialization pattern and user instruction. Public Opal surface: `new StringBuilder`, `string_builder_push(builder: StringBuilder, value: string): void errors BuilderFinishedError, AllocationFailureError`, `string_builder_finish(builder: StringBuilder): string errors BuilderFinishedError, AllocationFailureError`. Runtime names: `string_builder_new`, `string_builder_push`, `string_builder_finish`. Semantics: `finish` consumes the builder; subsequent `push` or `finish` returns `BuilderFinishedError`. No capacity/reserve API.

  **TDD — RED**:
  - Add tests for propertyless constructor resolution, push/finish output, and use-after-finish failure.
  - Run: `cargo test --features integration --test integration_e2e tests::string_builder_stdlib::string_builder_push_finish -- --exact`.
  - Expected RED: `StringBuilder` type or constructor is unresolved.

  **TDD — GREEN**:
  - Add nominal type and error types in typechecker/module resolver.
  - Add propertyless constructor mapping `StringBuilder -> string_builder_new`.
  - Add codegen declarations and C runtime handle implementation.
  - Add integration fixtures.

  **TDD — REFACTOR**:
  - Confirm allocation/error string helper consistency with Bytes runtime.
  - Run focused tests, `cargo test`, fmt, and clippy.

  **Must NOT do**: Do not add reserve/capacity APIs. Do not make repeated string interpolation the blessed performance path. Do not expose raw pointers.

  **Recommended Agent Profile**:
  - Category: `deep` - Reason: New opaque handle crosses parser/typechecker/codegen/runtime.
  - Skills: [] - No special skill required.
  - Omitted: [`frontend-ui-ux`] - No UI work.

  **Parallelization**: Can Parallel: NO | Wave 2 | Blocks: [8] | Blocked By: [1]

  **References**:
  - Pattern: `src/type_system/propertyless_constructors.rs:16-21` - `Bytes -> bytes_new` mapping.
  - Pattern: `runtime/opal_bytes.c:10-22` - opaque runtime representation comments.
  - Pattern: `runtime/opal_bytes.c:47-65` - allocation pattern.
  - Pattern: `src/stdlib/bytes.rs:115-131` - Rust constructor semantics.
  - Pattern: `tests/integration_e2e/bytes_stdlib.rs` - end-to-end runtime tests.

  **Acceptance Criteria**:
  - [ ] `cargo test --features integration --test integration_e2e tests::string_builder_stdlib::string_builder_push_finish -- --exact` exits 0 and stdout equals `row-1\nrow-2\n`.
  - [ ] `cargo test --features integration --test integration_e2e tests::string_builder_stdlib::string_builder_use_after_finish_errors -- --exact` exits 0 and asserts `BuilderFinishedError`.
  - [ ] `cargo test` exits 0.

  **QA Scenarios**:
  ```
  Scenario: Builder renders accumulated frame
    Tool: Bash
    Steps: cargo test --features integration --test integration_e2e tests::string_builder_stdlib::string_builder_push_finish -- --exact
    Expected: exit 0; output exactly matches expected multi-line string.
    Evidence: .sisyphus/evidence/task-2-string-builder.txt

  Scenario: Builder rejects use after finish
    Tool: Bash
    Steps: cargo test --features integration --test integration_e2e tests::string_builder_stdlib::string_builder_use_after_finish_errors -- --exact
    Expected: exit 0; compiled program reports/propagates BuilderFinishedError, not segfault or UB.
    Evidence: .sisyphus/evidence/task-2-string-builder-error.txt
  ```

  **Commit**: YES | Message: `feat(stdlib): add string builder` | Files: [`src/type_system/**`, `src/codegen/functions_stdlib.rs`, `runtime/opal_runtime.h`, `runtime/*.c`, `tests/integration_e2e/**`, `test-projects/**`]

- [x] 3. Add fallible no-newline stdout APIs

  **What to do**: Preserve existing `print`; add `print_text_sync(value: string): void errors WriteFailureError, SinkClosedError` and `flush_standard_output_sync(): void errors FlushFailureError, SinkClosedError`. Runtime names match public names. Use C `stdout`; `print_text_sync` writes bytes without newline and does not implicitly flush; `flush_standard_output_sync` calls `fflush(stdout)` and reports failure.

  **TDD — RED**:
  - Add integration test `print_text_flush_writes_without_newline` and failure-path plumbing test for declared errors.
  - Run: `cargo test --features integration --test integration_e2e tests::stdout_text_stdlib::print_text_flush_writes_without_newline -- --exact`.
  - Expected RED: unresolved symbols or missing runtime declarations.

  **TDD — GREEN**:
  - Add error types, type signatures, codegen declarations, runtime C functions and header prototypes.
  - Ensure void-with-error aggregate ABI is used consistently.

  **TDD — REFACTOR**:
  - Verify existing print tests remain unchanged.
  - Run `cargo test --features integration --test integration_print tests::print_types_compiles_links_and_runs -- --exact`.

  **Must NOT do**: Do not modify legacy `print` mapping or `print_line` behavior. Do not flush after every `print_text_sync`.

  **Recommended Agent Profile**:
  - Category: `deep` - Reason: Fallible void ABI and compatibility-sensitive output behavior.
  - Skills: [] - No special skill required.
  - Omitted: [`playwright`] - Terminal output is byte-captured, not browser UI.

  **Parallelization**: Can Parallel: NO | Wave 3 | Blocks: [4, 7, 8] | Blocked By: []

  **References**:
  - Pattern: `runtime/opal_print.c` - existing print helpers to preserve.
  - Pattern: `src/codegen/error_abi.rs:9-22` - void + error aggregate convention.
  - Pattern: `src/runtime/io.rs:54-65` - runtime IO error mapping precedent.
  - Proposal: `game-of-life-proposals/print/print-text-and-flush/proposal.md:25-43` - requested API shape.
  - Test: `tests/integration_print.rs` - existing print regression tests.

  **Acceptance Criteria**:
  - [ ] `cargo test --features integration --test integration_e2e tests::stdout_text_stdlib::print_text_flush_writes_without_newline -- --exact` exits 0 and captured stdout equals `abc` before process exit.
  - [ ] Existing print integration tests pass unchanged.
  - [ ] `cargo test` exits 0.

  **QA Scenarios**:
  ```
  Scenario: No-newline write with explicit flush
    Tool: Bash
    Steps: cargo test --features integration --test integration_e2e tests::stdout_text_stdlib::print_text_flush_writes_without_newline -- --exact
    Expected: exit 0; captured stdout bytes are exactly `abc`, no newline.
    Evidence: .sisyphus/evidence/task-3-print-text.txt

  Scenario: Existing print compatibility
    Tool: Bash
    Steps: cargo test --features integration --test integration_print tests::print_types_compiles_links_and_runs -- --exact
    Expected: exit 0; existing newline-oriented print output expectations remain unchanged.
    Evidence: .sisyphus/evidence/task-3-print-regression.txt
  ```

  **Commit**: YES | Message: `feat(stdlib): add fallible stdout text APIs` | Files: [`src/type_system/**`, `src/codegen/functions_stdlib.rs`, `runtime/opal_runtime.h`, `runtime/*.c`, `tests/integration_e2e/**`]

- [x] 4. Add `StdoutWriter` handle APIs

  **What to do**: Add public opaque handle `StdoutWriter` per user preference. Public API: `stdout_writer(): StdoutWriter`, `writer_write_sync(writer: StdoutWriter, value: string): void errors WriteFailureError, SinkClosedError`, `writer_flush_sync(writer: StdoutWriter): void errors FlushFailureError, SinkClosedError`. Runtime symbols: `stdout_writer`, `writer_write_sync`, `writer_flush_sync`. Semantics: each `stdout_writer()` returns a fresh lightweight handle to shared stdout; byte order is deterministic with `print_text_sync` because both write to C `stdout`; explicit flush controls visibility.

  **TDD — RED**:
  - Add integration test `stdout_writer_write_flush` and interleaving test `stdout_writer_interleaves_with_print_text`.
  - Run: `cargo test --features integration --test integration_e2e tests::stdout_writer_stdlib::stdout_writer_write_flush -- --exact`.
  - Expected RED: `StdoutWriter` type or `stdout_writer` unresolved.

  **TDD — GREEN**:
  - Add nominal handle type and signatures.
  - Add codegen declarations and C runtime handle implementation.
  - Reuse underlying stdout write/flush helpers from Task 3.

  **TDD — REFACTOR**:
  - Deduplicate stdout write error handling between `print_text_sync` and writer APIs.
  - Re-run Task 3 tests plus writer tests.

  **Must NOT do**: Do not expose public `TextWriter` type in this implementation. Do not add file/stderr/memory writers. Do not add methods.

  **Recommended Agent Profile**:
  - Category: `deep` - Reason: New handle plus output interleaving semantics.
  - Skills: [] - No special skill required.
  - Omitted: [`frontend-ui-ux`] - No UI work.

  **Parallelization**: Can Parallel: NO | Wave 3 | Blocks: [8] | Blocked By: [3]

  **References**:
  - User decision: public handle is `StdoutWriter`, not `TextWriter`.
  - Pattern: `runtime/opal_bytes.c` - opaque handle implementation style.
  - Proposal context: `game-of-life-proposals/print/text-writer-sink/proposal.md:25-39` - writer free-function API, adapted to `StdoutWriter`.
  - Pattern: `src/stdlib/io.rs:20-112` - mockable Rust handler shape.

  **Acceptance Criteria**:
  - [ ] `cargo test --features integration --test integration_e2e tests::stdout_writer_stdlib::stdout_writer_write_flush -- --exact` exits 0 and stdout equals `frame`.
  - [ ] `cargo test --features integration --test integration_e2e tests::stdout_writer_stdlib::stdout_writer_interleaves_with_print_text -- --exact` exits 0 and stdout equals `A1B2`.
  - [ ] `cargo test --features integration --test integration_e2e tests::stdout_text_stdlib::print_text_flush_writes_without_newline -- --exact` still exits 0.

  **QA Scenarios**:
  ```
  Scenario: Writer writes and flushes stdout
    Tool: Bash
    Steps: cargo test --features integration --test integration_e2e tests::stdout_writer_stdlib::stdout_writer_write_flush -- --exact
    Expected: exit 0; captured stdout bytes exactly `frame`.
    Evidence: .sisyphus/evidence/task-4-stdout-writer.txt

  Scenario: Writer and print_text deterministic interleaving
    Tool: Bash
    Steps: cargo test --features integration --test integration_e2e tests::stdout_writer_stdlib::stdout_writer_interleaves_with_print_text -- --exact
    Expected: exit 0; captured stdout bytes exactly `A1B2`.
    Evidence: .sisyphus/evidence/task-4-stdout-writer-interleave.txt
  ```

  **Commit**: YES | Message: `feat(stdlib): add stdout writer handle` | Files: [`src/type_system/**`, `src/codegen/functions_stdlib.rs`, `runtime/opal_runtime.h`, `runtime/*.c`, `tests/integration_e2e/**`]

- [x] 5. Add `sleep_ms_sync(milliseconds)`

  **What to do**: Add blocking sleep for small demos. Public API: `sleep_ms_sync(milliseconds: int32): void errors InvalidDurationError`. Runtime symbol: `sleep_ms_sync`. Semantics: `milliseconds < 0` returns `InvalidDurationError`; zero returns immediately; positive values block the current thread. Use platform-specific C runtime implementation (`nanosleep`/`Sleep`) behind portability guards.

  **TDD — RED**:
  - Add integration tests `sleep_ms_sync_rejects_negative` and `sleep_ms_sync_50ms_timing`.
  - Run: `cargo test --features integration --test integration_e2e tests::time_stdlib::sleep_ms_sync_rejects_negative -- --exact`.
  - Expected RED: unresolved symbol or missing `InvalidDurationError`.

  **TDD — GREEN**:
  - Add error type, signatures, codegen declaration, runtime function, header prototype.
  - Implement one timing test only: elapsed time for 50ms must be `>=45ms` and `<=5000ms`.

  **TDD — REFACTOR**:
  - Keep platform-specific code in runtime portability helpers if needed.
  - Run focused tests, `cargo test`, fmt, and clippy.

  **Must NOT do**: Do not add async sleep, deadlines, timers, or general time APIs. Do not put sleep calls into fast unit tests except the single timing integration test.

  **Recommended Agent Profile**:
  - Category: `deep` - Reason: Cross-platform runtime primitive and fallible ABI.
  - Skills: [] - No special skill required.
  - Omitted: [`frontend-ui-ux`] - No UI work.

  **Parallelization**: Can Parallel: NO | Wave 4 | Blocks: [6, 8] | Blocked By: []

  **References**:
  - Pattern: `src/bounded_proc.rs` - existing host timing tolerance patterns.
  - Pattern: `src/app.rs` - host `std::thread::sleep` usage for reference only.
  - Pattern: `runtime/opal_portability.h` - platform portability location.
  - Pattern: `src/codegen/error_abi.rs:9-22` - void + error aggregate.

  **Acceptance Criteria**:
  - [ ] `cargo test --features integration --test integration_e2e tests::time_stdlib::sleep_ms_sync_rejects_negative -- --exact` exits 0 and asserts `InvalidDurationError`.
  - [ ] `cargo test --features integration --test integration_e2e tests::time_stdlib::sleep_ms_sync_50ms_timing -- --exact` exits 0 with elapsed wall time `>=45ms` and `<=5000ms`.
  - [ ] `cargo test` exits 0.

  **QA Scenarios**:
  ```
  Scenario: 50ms sleep timing gate
    Tool: Bash
    Steps: cargo test --features integration --test integration_e2e tests::time_stdlib::sleep_ms_sync_50ms_timing -- --exact
    Expected: exit 0; measured elapsed time is >=45ms and <=5000ms.
    Evidence: .sisyphus/evidence/task-5-sleep.txt

  Scenario: Negative duration rejected
    Tool: Bash
    Steps: cargo test --features integration --test integration_e2e tests::time_stdlib::sleep_ms_sync_rejects_negative -- --exact
    Expected: exit 0; program propagates InvalidDurationError for -1.
    Evidence: .sisyphus/evidence/task-5-sleep-error.txt
  ```

  **Commit**: YES | Message: `feat(stdlib): add blocking sleep` | Files: [`src/type_system/**`, `src/codegen/functions_stdlib.rs`, `runtime/opal_runtime.h`, `runtime/*.c`, `tests/integration_e2e/**`]

- [x] 6. Add `FrameClock`

  **What to do**: Add an opaque `FrameClock` handle for frame pacing using a public function constructor to avoid unsupported fallible constructor-expression propagation. Public Opal API: `frame_clock_new(frames_per_second: int32): FrameClock errors InvalidFrameRateError` and `frame_clock_wait_next_sync(clock: FrameClock): void errors InvalidFrameRateError`. Runtime constructor symbol: `frame_clock_new`. Semantics: fps must be positive; `0` and negatives fail during initialization with `InvalidFrameRateError`. The clock stores next-frame deadline and waits as needed using `sleep_ms_sync`/runtime helper. Document as a deliberate deviation from the proposal syntax `propagate new FrameClock: ...` because `src/type_system/checker/expressions.rs` currently only type-checks `propagate` for call-shaped error expressions; do not add fallible constructor-expression language support in this stdlib task.

  **TDD — RED**:
  - Add tests for invalid fps and 10 waits at 30fps.
  - Run: `cargo test --features integration --test integration_e2e tests::time_stdlib::frame_clock_rejects_invalid_fps -- --exact`.
  - Expected RED: `FrameClock` type or `frame_clock_new` function unresolved.

  **TDD — GREEN**:
  - Add handle type, `frame_clock_new` function signature, codegen declarations, C runtime struct, wait function.
  - Use monotonic runtime clock internally; if no monotonic helper exists, add private runtime helper, not public stdlib API.

  **TDD — REFACTOR**:
  - Ensure timing logic has loose CI bounds and does not duplicate sleep platform code.
  - Run Task 5 tests plus FrameClock tests.

  **Must NOT do**: Do not expose `Deadline`, monotonic now, pause/resume, async clocks, or frame-skipping policy in this iteration.

  **Recommended Agent Profile**:
  - Category: `deep` - Reason: Stateful timing handle with CI-flaky edge risks.
  - Skills: [] - No special skill required.
  - Omitted: [`frontend-ui-ux`] - No UI work.

  **Parallelization**: Can Parallel: NO | Wave 4 | Blocks: [8] | Blocked By: [5]

  **References**:
  - Proposal: `game-of-life-proposals/sleep/` - frame-clock design if present.
  - Pattern: `runtime/opal_bytes.c` - opaque C handle style.
  - Guardrail: `src/type_system/checker/expressions.rs:345-467` - propagate currently expects call-shaped error expressions; do not scope fallible constructor-expression support here.
  - Pattern: `src/bounded_proc.rs` - timing/tolerance reference.

  **Acceptance Criteria**:
  - [ ] `cargo test --features integration --test integration_e2e tests::time_stdlib::frame_clock_rejects_invalid_fps -- --exact` exits 0 and asserts `InvalidFrameRateError` for `0` and `-1`.
  - [ ] `cargo test --features integration --test integration_e2e tests::time_stdlib::frame_clock_30fps_ten_waits_timing -- --exact` exits 0 with 10 waits elapsed `>=280ms` and `<=5000ms`.
  - [ ] `cargo test --features integration --test integration_e2e tests::time_stdlib::sleep_ms_sync_50ms_timing -- --exact` still exits 0.

  **QA Scenarios**:
  ```
  Scenario: Ten 30fps waits are paced
    Tool: Bash
    Steps: cargo test --features integration --test integration_e2e tests::time_stdlib::frame_clock_30fps_ten_waits_timing -- --exact
    Expected: exit 0; measured elapsed time >=280ms and <=5000ms.
    Evidence: .sisyphus/evidence/task-6-frame-clock.txt

  Scenario: Invalid fps rejected at construction
    Tool: Bash
    Steps: cargo test --features integration --test integration_e2e tests::time_stdlib::frame_clock_rejects_invalid_fps -- --exact
    Expected: exit 0; construction with 0 and -1 reports InvalidFrameRateError.
    Evidence: .sisyphus/evidence/task-6-frame-clock-error.txt
  ```

  **Commit**: YES | Message: `feat(stdlib): add frame clock` | Files: [`src/type_system/**`, `src/codegen/functions_stdlib.rs`, `runtime/opal_runtime.h`, `runtime/*.c`, `tests/integration_e2e/**`]

- [x] 7. Add terminal clear/move/draw APIs

  **What to do**: Add stdout terminal handle and ANSI drawing helpers while also shipping minimal convenience wrappers. Public handle API: `stdout_terminal(): StdoutTerminal`, `terminal_supports_ansi(terminal: StdoutTerminal): bool`, `terminal_clear_screen_on_sync(terminal: StdoutTerminal): void errors TerminalWriteFailureError, SinkClosedError`, `terminal_move_cursor_on_sync(terminal: StdoutTerminal, row: int32, column: int32): void errors TerminalWriteFailureError, InvalidCursorPositionError, SinkClosedError`, `terminal_draw_rows_sync(terminal: StdoutTerminal, rows: string[]): void errors TerminalWriteFailureError, SinkClosedError`. Public convenience API: `terminal_clear_screen_sync(): void errors TerminalWriteFailureError, SinkClosedError` and `terminal_move_cursor_sync(row: int32, column: int32): void errors TerminalWriteFailureError, InvalidCursorPositionError, SinkClosedError`; these construct/use stdout terminal internally. Coordinates are 0-based in Opal; runtime emits 1-based ANSI. Exact bytes: clear screen `\x1b[2J\x1b[H`; move `(0,0)` `\x1b[1;1H`; draw rows joins with `\n` and writes without an extra trailing newline unless rows already contain it.

  **TDD — RED**:
  - Add byte-capture tests for clear, move, draw rows, and invalid cursor positions.
  - Run: `cargo test --features integration --test integration_e2e tests::terminal_stdlib::terminal_move_cursor_zero_based_ansi_bytes -- --exact`.
  - Expected RED: unresolved `StdoutTerminal` or terminal functions.

  **TDD — GREEN**:
  - Add handle type, signatures, codegen declarations, runtime C implementation.
  - Implement `terminal_supports_ansi` deterministically enough for tests: when stdout is captured/non-TTY, function may return false, but direct clear/move/draw still emit ANSI bytes for explicit calls. If capability detection is platform-specific, isolate it and add tests that do not depend on host TTY.

  **TDD — REFACTOR**:
  - Deduplicate terminal writes with stdout write helper from Task 3.
  - Re-run print and writer tests to ensure output ordering unchanged.

  **Must NOT do**: Do not add color, raw mode, alternate screen, input handling, styling, terminal size, or Windows full capability parity beyond safe ANSI support/detection.

  **Recommended Agent Profile**:
  - Category: `deep` - Reason: Platform-sensitive terminal bytes and fallible output ABI.
  - Skills: [] - No special skill required.
  - Omitted: [`playwright`] - Byte-capture tests replace visual/manual browser testing.

  **Parallelization**: Can Parallel: NO | Wave 5 | Blocks: [8] | Blocked By: [3]

  **References**:
  - Proposal: `game-of-life-proposals/terminal/ansi-control-functions/proposal.md:35-41` - minimal terminal use.
  - Pattern: `runtime/opal_runtime.c` - Windows console initialization.
  - Pattern: `runtime/opal_portability.h` - platform branches.
  - Pattern: `src/codegen/functions_stdlib.rs` - runtime symbol declaration.
  - Pattern: `tests/integration_e2e/bytes_stdlib.rs` - e2e test layout.

  **Acceptance Criteria**:
  - [ ] `cargo test --features integration --test integration_e2e tests::terminal_stdlib::terminal_clear_screen_ansi_bytes -- --exact` exits 0 and stdout bytes equal `\x1b[2J\x1b[H`.
  - [ ] `cargo test --features integration --test integration_e2e tests::terminal_stdlib::terminal_move_cursor_zero_based_ansi_bytes -- --exact` exits 0 and stdout bytes equal `\x1b[1;1H` for `(0,0)`.
  - [ ] `cargo test --features integration --test integration_e2e tests::terminal_stdlib::terminal_move_cursor_rejects_negative -- --exact` exits 0 and asserts `InvalidCursorPositionError`.
  - [ ] `cargo test --features integration --test integration_e2e tests::terminal_stdlib::terminal_draw_rows_bytes -- --exact` exits 0 and stdout bytes equal `##\n..` for rows [`##`, `..`].

  **QA Scenarios**:
  ```
  Scenario: Move cursor converts 0-based to ANSI 1-based
    Tool: Bash
    Steps: cargo test --features integration --test integration_e2e tests::terminal_stdlib::terminal_move_cursor_zero_based_ansi_bytes -- --exact
    Expected: exit 0; stdout bytes exactly ESC `[1;1H`.
    Evidence: .sisyphus/evidence/task-7-terminal-move.txt

  Scenario: Invalid cursor position rejected
    Tool: Bash
    Steps: cargo test --features integration --test integration_e2e tests::terminal_stdlib::terminal_move_cursor_rejects_negative -- --exact
    Expected: exit 0; negative row or column reports InvalidCursorPositionError.
    Evidence: .sisyphus/evidence/task-7-terminal-error.txt
  ```

  **Commit**: YES | Message: `feat(stdlib): add terminal drawing APIs` | Files: [`src/type_system/**`, `src/codegen/functions_stdlib.rs`, `runtime/opal_runtime.h`, `runtime/*.c`, `tests/integration_e2e/**`]

- [x] 8. Add bounded Game of Life test project

  **What to do**: Create `test-projects/game-of-life` using the new APIs. The program must render a fixed board for exactly 10 frames and then exit. It must use `string_join` for row/frame assembly, `StringBuilder` where useful for full transcript/frame construction, `StdoutWriter` or `print_text_sync` for output, `FrameClock` or `sleep_ms_sync` with a test-safe pacing choice, and terminal draw APIs for frame rendering. For integration tests, use zero/minimal sleep path if language code can parameterize it; otherwise use `FrameClock` only in its own timing test and keep Game of Life output test deterministic and fast. Golden fixture path: `test-projects/game-of-life/fixtures/expected_10_frames.txt`.

  **TDD — RED**:
  - Add `game_of_life_ten_frames` integration test and golden fixture before implementation.
  - Add a timeout guard in the Rust integration test using existing bounded process helpers.
  - Run: `cargo test --features integration --test integration_e2e tests::game_of_life::game_of_life_ten_frames -- --exact`.
  - Expected RED: missing project/program or golden mismatch.

  **TDD — GREEN**:
  - Implement bounded Opal project under `test-projects/game-of-life/src/main.op`.
  - Use fixed initial board such as a 5x5 blinker/glider-compatible pattern and exactly 10 iterations with an integer loop bound.
  - Output a transcript with frame headers `Frame 0` through `Frame 9`; no `while true`; no unbounded recursion.

  **TDD — REFACTOR**:
  - Simplify rendering helpers and ensure no duplicated board constants.
  - Run all new API tests plus final integration test.

  **Must NOT do**: Do not create a demo that runs indefinitely. Do not rely on manual Ctrl-C. Do not require a real terminal for the golden test. Do not compare host-dependent ANSI capability detection results.

  **Recommended Agent Profile**:
  - Category: `deep` - Reason: End-to-end example combining all new stdlib APIs with bounded QA.
  - Skills: [] - No special skill required.
  - Omitted: [`frontend-ui-ux`] - Output is terminal byte transcript, not interactive UI.

  **Parallelization**: Can Parallel: NO | Wave 6 | Blocks: [Final Verification] | Blocked By: [1, 2, 3, 4, 5, 6, 7]

  **References**:
  - Pattern: `test-projects/fs-markdown-roundtrip/src/main.op` - fixture project shape.
  - Pattern: `test-projects/fs-markdown-roundtrip/fixtures/expected_output.md` - golden fixture style.
  - Test: `tests/integration_e2e/fs_markdown_roundtrip.rs` - byte-for-byte golden compare.
  - Pattern: `tests/integration_e2e/fs_helpers.rs` - workspace isolation helpers.
  - Pattern: `src/bounded_proc.rs` - timeout guard to prevent hangs.

  **Acceptance Criteria**:
  - [ ] `cargo test --features integration --test integration_e2e tests::game_of_life::game_of_life_ten_frames -- --exact` exits 0 and compares stdout byte-for-byte to `test-projects/game-of-life/fixtures/expected_10_frames.txt`.
  - [ ] The Opal source contains no `while true` and no unbounded recursion; integration test enforces process timeout.
  - [ ] Golden output includes exactly 10 frame headers: `Frame 0` through `Frame 9`, and no `Frame 10`.
  - [ ] `timeout 900 cargo test --all-features` exits 0.

  **QA Scenarios**:
  ```
  Scenario: Game of Life emits exactly 10 frames
    Tool: Bash
    Steps: cargo test --features integration --test integration_e2e tests::game_of_life::game_of_life_ten_frames -- --exact
    Expected: exit 0; stdout equals expected_10_frames.txt byte-for-byte; contains Frame 0..Frame 9 only.
    Evidence: .sisyphus/evidence/task-8-game-of-life.txt

  Scenario: Game of Life cannot hang CI
    Tool: Bash
    Steps: timeout 30 cargo test --features integration --test integration_e2e tests::game_of_life::game_of_life_ten_frames -- --exact
    Expected: exit 0 before timeout; process helper also uses bounded execution for compiled binary.
    Evidence: .sisyphus/evidence/task-8-game-of-life-timeout.txt
  ```

  **Commit**: YES | Message: `test(examples): add bounded game of life project` | Files: [`test-projects/game-of-life/**`, `tests/integration_e2e/**`]

## Final Verification Wave (MANDATORY — after ALL implementation tasks)
> 4 review agents run in PARALLEL. ALL must APPROVE. Present consolidated results to user and get explicit "okay" before completing.
> **Do NOT auto-proceed after verification. Wait for user's explicit approval before marking work complete.**
> **Never mark F1-F4 as checked before getting user's okay.** Rejection or user feedback -> fix -> re-run -> present again -> wait for okay.
- [x] F1. Plan Compliance Audit — oracle

  **Agent Invocation**:
  ```
  task(subagent_type="oracle", load_skills=[], run_in_background=true,
    prompt="Audit the completed implementation against .sisyphus/plans/stdlib-game-of-life.md. Verify every TODO acceptance criterion was executed, every API listed in the plan exists, existing print behavior is unchanged, Game of Life is bounded to exactly 10 frames, and no Must NOT Have item was violated. Return APPROVE or REJECT with exact file/command evidence.")
  ```

  **QA Scenario**:
  ```
  Scenario: Plan compliance audit
    Tool: task oracle
    Steps: Run the Agent Invocation above after all implementation tasks pass; collect the task result.
    Expected: Oracle returns APPROVE and lists evidence for Tasks 1-8 plus final commands; any REJECT blocks completion.
    Evidence: .sisyphus/evidence/f1-plan-compliance.md
  ```

- [x] F2. Code Quality Review — unspecified-high

  **Agent Invocation**:
  ```
  task(category="unspecified-high", load_skills=[], run_in_background=true,
    prompt="Review the completed code changes for quality. Check runtime C safety, Rust typechecker/codegen maintainability, duplicated ABI declarations, handle invalid-use behavior, platform guards, and AI-slop patterns. Also run or verify: cargo fmt --all -- --check; cargo clippy --all-targets --all-features -- -D warnings. Return APPROVE or REJECT with exact findings.")
  ```

  **QA Scenario**:
  ```
  Scenario: Code quality review
    Tool: task unspecified-high
    Steps: Run the Agent Invocation above after all implementation tasks pass; collect the task result.
    Expected: Reviewer returns APPROVE; fmt and clippy commands exit 0; any REJECT blocks completion.
    Evidence: .sisyphus/evidence/f2-code-quality.md
  ```

- [x] F3. Real Manual QA — unspecified-high

  **Agent Invocation**:
  ```
  task(category="unspecified-high", load_skills=[], run_in_background=true,
    prompt="Perform agent-executed manual-style QA for the stdlib APIs and bounded Game of Life. Run exact integration commands from .sisyphus/plans/stdlib-game-of-life.md for Tasks 1-8, inspect captured stdout/golden output bytes, confirm ANSI byte expectations, confirm timing bounds, and confirm Game of Life exits before timeout with exactly Frame 0 through Frame 9. Return APPROVE or REJECT with command outputs summarized.")
  ```

  **QA Scenario**:
  ```
  Scenario: Manual-style QA without human intervention
    Tool: task unspecified-high
    Steps: Run the Agent Invocation above after all implementation tasks pass; collect the task result.
    Expected: QA agent returns APPROVE; all task-level commands exit 0; golden output and ANSI byte checks are exact; any REJECT blocks completion.
    Evidence: .sisyphus/evidence/f3-real-qa.md
  ```

- [x] F4. Scope Fidelity Check — deep

  **Agent Invocation**:
  ```
  task(category="deep", load_skills=[], run_in_background=true,
    prompt="Check scope fidelity for the completed work against the original request and .sisyphus/plans/stdlib-game-of-life.md. Verify no unrequested file primitives, no public TextWriter type, no terminal color/raw/input/alternate-screen features, no unbounded loops, no print semantic changes, and no fallible constructor-expression language expansion. Return APPROVE or REJECT with exact evidence.")
  ```

  **QA Scenario**:
  ```
  Scenario: Scope fidelity review
    Tool: task deep
    Steps: Run the Agent Invocation above after all implementation tasks pass; collect the task result.
    Expected: Scope checker returns APPROVE and confirms every Must NOT Have remains absent; any REJECT blocks completion.
    Evidence: .sisyphus/evidence/f4-scope-fidelity.md
  ```

## Commit Strategy
- Commit after each vertical slice only when its RED, GREEN, REFACTOR, and acceptance commands pass.
- Suggested messages:
  - `feat(stdlib): add string_join`
  - `feat(stdlib): add string builder`
  - `feat(stdlib): add fallible stdout text APIs`
  - `feat(stdlib): add stdout writer handle`
  - `feat(stdlib): add blocking sleep`
  - `feat(stdlib): add frame clock`
  - `feat(stdlib): add terminal drawing APIs`
  - `test(examples): add bounded game of life project`
- Do not commit `.sisyphus/evidence/` unless project convention requires it.

## Success Criteria
- All Definition of Done commands pass.
- Existing print-related integration tests pass unchanged.
- Every new API has happy-path and failure-path coverage where fallible.
- Game of Life e2e proves exact 10-frame termination and byte-for-byte golden output.
