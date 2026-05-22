# Remove Game of Life Production Hardcoding

## TL;DR
> **Summary**: Remove production/compiler/runtime-facing Game of Life-specific hardcoded limits and probe wiring while preserving legitimate test-only golden fixtures. The main target is `src/bin/gol_memory_probe.rs` and any production-style references to its hardcoded thresholds.
> **Deliverables**:
> - Remove or de-register the GoL memory probe from production source/binary targets.
> - Preserve GoL e2e expected-output tests unless evidence shows they alter compiler/runtime behavior.
> - Remove/update references that treat GoL memory thresholds as compiler/runtime acceptance gates.
> - Produce sweeping-search evidence proving no forbidden GoL production hardcoding remains.
> **Effort**: Short
> **Parallel**: YES - 3 waves
> **Critical Path**: Baseline + classification → Remove/de-register probe + update references → Sweeping verification

## Context
### Original Request
User requested removal of all Opalescent additions that only benefit Game of Life, specifically mentioning hardcoded limits such as `PEAK_LIVE_BYTES_LIMIT`, because such additions have no place in a compiler/runtime. User also requested sweeping analysis afterwards to confirm removal.

### Interview Summary
- Hardcoded values inside end-to-end tests are acceptable when they are solely expected-output fixtures.
- Hardcoded compiler/runtime/source behavior added only to make Game of Life pass is not acceptable.
- GoL e2e golden-output fixtures should remain unless implementation evidence shows they modify compiler/runtime behavior.
- The plan must distinguish allowed test fixture hardcoding from forbidden production/compiler/runtime special-casing.

### Metis Review (gaps addressed)
- Guardrail added: do not delete e2e fixtures merely because they contain deterministic GoL expected values.
- Guardrail added: do not touch generic runtime/array RC functionality in `runtime/opal_rc.c`, `runtime/opal_rc.h`, or generic array tests.
- Guardrail added: inspect module registrations, `Cargo.toml`, scripts, and CI for dangling references after probe removal.
- Guardrail added: sweeping searches must use exact commands and captured evidence, not informal manual claims.

## Work Objectives
### Core Objective
Remove Game of Life-specific production/compiler/runtime hardcoding, especially the GoL memory probe and hardcoded memory thresholds, without removing valid test-only expected-output fixtures or generic runtime/compiler functionality.

### Deliverables
- `src/bin/gol_memory_probe.rs` removed from production binary source or otherwise de-registered so it is not a compiler/runtime-facing binary target.
- All references to `PEAK_LIVE_BYTES_LIMIT`, `STEADY_STATE_SPREAD_LIMIT`, and `gol_memory_probe` as production/acceptance gates removed or rewritten as historical notes outside executable verification.
- GoL e2e test/project/fixture preserved if confirmed to remain test-only:
  - `tests/integration_e2e/game_of_life.rs`
  - `test-projects/game-of-life/src/main.op`
  - `test-projects/game-of-life/fixtures/expected_10_frames.txt`
- Sweeping proof that forbidden GoL production hardcoding is absent.
- Evidence files under `.sisyphus/evidence/` for each verification command.

### Definition of Done (verifiable conditions with commands)
- `test ! -e src/bin/gol_memory_probe.rs && echo OK_REMOVED_GOL_PROBE` exits 0.
- `rg '\bPEAK_LIVE_BYTES_LIMIT\b' --hidden --glob '!target' --glob '!.sisyphus/plans/remove-gol-hardcoding.md' --glob '!.sisyphus/evidence/**' . ; test $? -eq 1 && echo OK_NO_PEAK_LIMIT` exits 0.
- `rg '\bSTEADY_STATE_SPREAD_LIMIT\b' --hidden --glob '!target' --glob '!.sisyphus/plans/remove-gol-hardcoding.md' --glob '!.sisyphus/evidence/**' . ; test $? -eq 1 && echo OK_NO_SPREAD_LIMIT` exits 0.
- `rg '\bgol_memory_probe\b' --hidden --glob '!target' --glob '!.sisyphus/plans/remove-gol-hardcoding.md' --glob '!.sisyphus/evidence/**' . ; test $? -eq 1 && echo OK_NO_GOL_PROBE_REFS` exits 0.
- `cargo build --all-features` exits 0.
- `cargo test --all-features` exits 0 with zero failures.
- `cargo test --features integration --test integration_e2e "game_of_life_ten_frames" -- --nocapture --test-threads=1` exits 0, proving valid test fixture behavior remains.
- `cargo test --features integration --test integration_e2e "memory_model_counters" -- --nocapture --test-threads=1` exits 0, proving generic memory accounting remains.
- `bash scripts/array_memory_sanitizer.sh` exits 0.
- `cargo clippy --all-features --all-targets -- -D warnings` exits 0.
- `cargo fmt --all -- --check` exits 0.

### Must Have
- Preserve legitimate test-only GoL fixture hardcoding.
- Remove production/source binary GoL memory limits.
- Preserve generic runtime heap accounting and array RC APIs.
- Capture evidence outputs to `.sisyphus/evidence/task-{N}-*.txt`.
- Review each remaining GoL hit as allowed test fixture, active plan/evidence self-reference, or forbidden production hardcoding.

### Must NOT Have (guardrails, AI slop patterns, scope boundaries)
- MUST NOT delete `tests/integration_e2e/game_of_life.rs` solely because it is GoL-specific.
- MUST NOT delete `test-projects/game-of-life/` or `expected_10_frames.txt` if they are only e2e expected-output fixtures.
- MUST NOT edit `runtime/opal_rc.c` or `runtime/opal_rc.h` unless a direct forbidden GoL special case is discovered there; current research says none exists.
- MUST NOT edit `tests/array_integration.rs` unless a direct GoL production-limit invocation is discovered; current research says it is generic.
- MUST NOT generalize or rename the GoL probe as a way to keep hardcoded GoL thresholds.
- MUST NOT fix unrelated clippy, formatting, or test issues by broad refactors.
- MUST NOT claim the sweep passed without exact commands, exit codes, and evidence files.

## Verification Strategy
> ZERO HUMAN INTERVENTION - all verification is agent-executed.
- Test decision: tests-after + baseline; existing Rust/Cargo test infrastructure.
- QA policy: Every task has agent-executed scenarios.
- Evidence: `.sisyphus/evidence/task-{N}-{slug}.txt`

## Execution Strategy
### Parallel Execution Waves
> Target: 5-8 tasks per wave. <3 per wave (except final) = under-splitting.
> Extract shared dependencies as Wave-1 tasks for max parallelism.

Wave 1: Baseline and classification tasks (`unspecified-high`, `quick`)
Wave 2: Probe removal/de-registration and reference cleanup (`quick`, `unspecified-low`)
Wave 3: Sweeping verification and final review (`unspecified-high`, `deep`)

### Dependency Matrix (full, all tasks)
- Task 1 blocks Tasks 2-6.
- Task 2 blocks Tasks 4, 6, and final verification.
- Task 3 can run after Task 1 and informs Task 4.
- Task 4 depends on Tasks 2-3.
- Task 5 depends on Task 1 and can run parallel to Tasks 2-4.
- Task 6 depends on Tasks 2-5.
- Final Verification Wave depends on all implementation tasks.

### Agent Dispatch Summary (wave → task count → categories)
- Wave 1 → 1 task → `unspecified-high`
- Wave 2 → 3 tasks → `quick`, `unspecified-low`, `quick`
- Wave 3 → 1 task → `unspecified-high`

## TODOs
> Implementation + Test = ONE task. Never separate.
> EVERY task MUST have: Agent Profile + Parallelization + QA Scenarios.

- [x] 1. Baseline current allowed tests and forbidden probe surface

  **What to do**: Capture the current state before removals. Run baseline tests for all features, the GoL e2e expected-output test, and memory model counters. Record whether `src/bin/gol_memory_probe.rs` exists and capture exact hits for forbidden symbols.
  **Must NOT do**: Do not delete or edit anything in this task. Do not treat GoL e2e fixture hits as failures.

  **Recommended Agent Profile**:
  - Category: `unspecified-high` - Reason: Requires careful evidence collection and classification, not code changes.
  - Skills: [] - No specialized skill needed.
  - Omitted: [`frontend-ui-ux`] - No UI work.

  **Parallelization**: Can Parallel: NO | Wave 1 | Blocks: [2, 3, 4, 5, 6] | Blocked By: []

  **References** (executor has NO interview context - be exhaustive):
  - Pattern: `src/bin/gol_memory_probe.rs` - forbidden production/source binary target containing GoL memory thresholds.
  - Test: `tests/integration_e2e/game_of_life.rs` - allowed e2e golden-output test; preserve if test-only.
  - Fixture: `test-projects/game-of-life/fixtures/expected_10_frames.txt` - allowed expected-output fixture.
  - Runtime: `runtime/opal_rc.c`, `runtime/opal_rc.h` - generic runtime accounting and array RC; do not remove.
  - Script: `scripts/array_memory_sanitizer.sh` - currently generic memory sanitizer script; earlier research found no direct probe invocation.

  **Acceptance Criteria** (agent-executable only):
  - [ ] `mkdir -p .sisyphus/evidence && cargo test --all-features 2>&1 | tee .sisyphus/evidence/task-1-baseline-all-tests.txt` exits 0.
  - [ ] `cargo test --features integration --test integration_e2e "game_of_life_ten_frames" -- --nocapture --test-threads=1 2>&1 | tee .sisyphus/evidence/task-1-baseline-gol-e2e.txt` exits 0.
  - [ ] `cargo test --features integration --test integration_e2e "memory_model_counters" -- --nocapture --test-threads=1 2>&1 | tee .sisyphus/evidence/task-1-baseline-memory-counters.txt` exits 0.
  - [ ] `test -e src/bin/gol_memory_probe.rs && echo PRESENT_FOR_REMOVAL | tee .sisyphus/evidence/task-1-probe-present.txt` exits 0.
  - [ ] `rg '\bPEAK_LIVE_BYTES_LIMIT\b|\bSTEADY_STATE_SPREAD_LIMIT\b|\bgol_memory_probe\b' --hidden --glob '!target' . 2>&1 | tee .sisyphus/evidence/task-1-forbidden-surface.txt` captures all current forbidden-surface hits.

  **QA Scenarios** (MANDATORY - task incomplete without these):
  ```
  Scenario: Baseline tests pass before removal
    Tool: Bash
    Steps: Run `cargo test --all-features` and targeted e2e/memory counter commands exactly as listed in acceptance criteria.
    Expected: All commands exit 0 and evidence files exist under `.sisyphus/evidence/`.
    Evidence: .sisyphus/evidence/task-1-baseline-all-tests.txt

  Scenario: Forbidden surface is visible before removal
    Tool: Bash
    Steps: Run the forbidden-surface `rg` command exactly as listed.
    Expected: Output includes `src/bin/gol_memory_probe.rs` and threshold/probe names so later removal has a baseline.
    Evidence: .sisyphus/evidence/task-1-forbidden-surface.txt
  ```

  **Commit**: NO | Message: N/A | Files: [.sisyphus/evidence/task-1-*]

- [x] 2. Remove the production/source GoL memory probe binary

  **What to do**: Delete `src/bin/gol_memory_probe.rs`. Inspect `Cargo.toml` for an explicit `[[bin]]` entry named `gol_memory_probe`; remove only that entry if present. If no explicit `[[bin]]` entry exists, record that auto-discovery was the only registration.
  **Must NOT do**: Do not move the probe to another production source path. Do not replace hardcoded constants with renamed equivalents. Do not edit runtime heap accounting.

  **Recommended Agent Profile**:
  - Category: `quick` - Reason: Focused removal plus registration check.
  - Skills: [] - No specialized skill needed.
  - Omitted: [`ai-slop-remover`] - No code cleanup pass is needed.

  **Parallelization**: Can Parallel: YES | Wave 2 | Blocks: [4, 6] | Blocked By: [1]

  **References** (executor has NO interview context - be exhaustive):
  - Remove: `src/bin/gol_memory_probe.rs` - contains `PEAK_LIVE_BYTES_LIMIT`, `STEADY_STATE_SPREAD_LIMIT`, and GoL C harness template.
  - Inspect: `Cargo.toml` - verify whether binary target is explicit or auto-discovered.
  - Preserve: `runtime/opal_rc.c`, `runtime/opal_rc.h` - generic runtime accounting used by more than GoL.

  **Acceptance Criteria** (agent-executable only):
  - [ ] `test ! -e src/bin/gol_memory_probe.rs && echo OK_REMOVED_GOL_PROBE | tee .sisyphus/evidence/task-2-probe-removed.txt` exits 0.
  - [ ] `rg 'name\s*=\s*"gol_memory_probe"|src/bin/gol_memory_probe.rs' Cargo.toml 2>&1 | tee .sisyphus/evidence/task-2-cargo-registration.txt; test ${PIPESTATUS[0]} -eq 1` exits 0, proving no explicit Cargo registration remains.
  - [ ] `cargo check --all-features 2>&1 | tee .sisyphus/evidence/task-2-cargo-check.txt` exits 0.

  **QA Scenarios** (MANDATORY - task incomplete without these):
  ```
  Scenario: Probe file is gone and Cargo still checks
    Tool: Bash
    Steps: Run the three acceptance commands exactly.
    Expected: Probe file does not exist; no Cargo registration remains; `cargo check --all-features` exits 0.
    Evidence: .sisyphus/evidence/task-2-cargo-check.txt

  Scenario: Generic runtime accounting untouched
    Tool: Bash
    Steps: Run `git diff -- runtime/opal_rc.c runtime/opal_rc.h`.
    Expected: Empty diff.
    Evidence: .sisyphus/evidence/task-2-runtime-untouched.txt
  ```

  **Commit**: NO | Message: N/A | Files: [src/bin/gol_memory_probe.rs, Cargo.toml if explicit bin entry exists]

- [x] 3. Confirm and preserve allowed GoL e2e fixtures

  **What to do**: Inspect the GoL e2e test and fixture paths to confirm they remain test-only expected-output artifacts. Leave them in place. Add evidence that the test still passes after probe removal.
  **Must NOT do**: Do not delete `tests/integration_e2e/game_of_life.rs`, `test-projects/game-of-life/src/main.op`, or `expected_10_frames.txt` unless direct evidence shows they alter compiler/runtime behavior.

  **Recommended Agent Profile**:
  - Category: `quick` - Reason: Focused classification and targeted test run.
  - Skills: [] - No specialized skill needed.
  - Omitted: [`playwright`] - No browser testing.

  **Parallelization**: Can Parallel: YES | Wave 2 | Blocks: [6] | Blocked By: [1]

  **References** (executor has NO interview context - be exhaustive):
  - Test: `tests/integration_e2e/game_of_life.rs` - compiles and runs GoL sample, compares output to golden fixture.
  - Fixture: `test-projects/game-of-life/fixtures/expected_10_frames.txt` - allowed hardcoded expected output.
  - Source sample: `test-projects/game-of-life/src/main.op` - sample program used by e2e test.
  - User decision: test-only expected values are allowed; compiler/runtime changes to satisfy GoL/tests are not.

  **Acceptance Criteria** (agent-executable only):
  - [ ] `test -e tests/integration_e2e/game_of_life.rs && test -e test-projects/game-of-life/src/main.op && test -e test-projects/game-of-life/fixtures/expected_10_frames.txt && echo OK_FIXTURES_PRESERVED | tee .sisyphus/evidence/task-3-fixtures-preserved.txt` exits 0.
  - [ ] `cargo test --features integration --test integration_e2e "game_of_life_ten_frames" -- --nocapture --test-threads=1 2>&1 | tee .sisyphus/evidence/task-3-gol-e2e-after-removal.txt` exits 0.
  - [ ] `rg '\bPEAK_LIVE_BYTES_LIMIT\b|\bSTEADY_STATE_SPREAD_LIMIT\b|\bgol_memory_probe\b' tests test-projects --hidden 2>&1 | tee .sisyphus/evidence/task-3-test-probe-reference-scan.txt; test ${PIPESTATUS[0]} -eq 1` exits 0, proving tests do not invoke the removed probe or thresholds.

  **QA Scenarios** (MANDATORY - task incomplete without these):
  ```
  Scenario: Allowed fixture hardcoding remains valid
    Tool: Bash
    Steps: Run the e2e test command for `game_of_life_ten_frames`.
    Expected: Test exits 0 and validates golden output; this confirms test-only hardcoding is preserved.
    Evidence: .sisyphus/evidence/task-3-gol-e2e-after-removal.txt

  Scenario: Tests do not enforce removed production probe limits
    Tool: Bash
    Steps: Run the `rg` command against `tests` and `test-projects` for probe/threshold symbols.
    Expected: Exit code 1 from `rg`, meaning no test references those forbidden probe symbols.
    Evidence: .sisyphus/evidence/task-3-test-probe-reference-scan.txt
  ```

  **Commit**: NO | Message: N/A | Files: [.sisyphus/evidence/task-3-*]

- [x] 4. Remove or rewrite production-style probe references in scripts, CI, and Sisyphus artifacts

  **What to do**: Search `.github/`, `scripts/`, `.sisyphus/`, root docs, and manifests for `gol_memory_probe`, `PEAK_LIVE_BYTES_LIMIT`, and `STEADY_STATE_SPREAD_LIMIT`. Remove references that instruct execution or acceptance of GoL probe thresholds. If a historical note is kept, rewrite it so it does not define a current compiler/runtime acceptance gate. Prefer deletion of obsolete GoL-probe evidence files if they assert current threshold success.
  **Must NOT do**: Do not delete ordinary GoL e2e test docs or fixtures just because they mention Game of Life. Do not rewrite active plan self-references in this file until final sweep classification.

  **Recommended Agent Profile**:
  - Category: `unspecified-low` - Reason: Reference cleanup across docs/scripts, with careful distinction between current gates and historical notes.
  - Skills: [] - No specialized skill needed.
  - Omitted: [`git-master`] - No commit requested during task.

  **Parallelization**: Can Parallel: YES | Wave 2 | Blocks: [6] | Blocked By: [1, 2]

  **References** (executor has NO interview context - be exhaustive):
  - Search targets: `.github/`, `scripts/`, `.sisyphus/`, `Cargo.toml`, `Makefile.toml`, `README.md`, `ARRAY_FEATURES.md`.
  - Known GoL-probe artifacts from research: `.sisyphus/plans/array-cow-rc-game-of-life.md`, `.sisyphus/evidence/task-1-baseline-memory.txt`, `.sisyphus/evidence/task-8-gol-memory.txt`, `.sisyphus/evidence/task-8-gol-stability.txt`, `.sisyphus/verification/verifier-2.md`, `.sisyphus/notepads/array-cow-rc-game-of-life/learnings.md`.
  - Script: `scripts/array_memory_sanitizer.sh` - earlier research says it does not invoke the probe; verify and leave unchanged if true.

  **Acceptance Criteria** (agent-executable only):
  - [ ] `rg '\bgol_memory_probe\b|\bPEAK_LIVE_BYTES_LIMIT\b|\bSTEADY_STATE_SPREAD_LIMIT\b' .github scripts Cargo.toml Makefile.toml README.md ARRAY_FEATURES.md .sisyphus --hidden 2>&1 | tee .sisyphus/evidence/task-4-reference-cleanup-scan.txt; true` captures remaining references for classification.
  - [ ] Every hit in `.sisyphus/evidence/task-4-reference-cleanup-scan.txt` is removed, rewritten as non-current historical context, or explicitly classified as active-plan/evidence self-reference in `.sisyphus/evidence/task-4-reference-classification.txt`.
  - [ ] `rg '\bgol_memory_probe\b|\bPEAK_LIVE_BYTES_LIMIT\b|\bSTEADY_STATE_SPREAD_LIMIT\b' .github scripts Cargo.toml Makefile.toml README.md ARRAY_FEATURES.md --hidden 2>&1 | tee .sisyphus/evidence/task-4-production-reference-final-scan.txt; test ${PIPESTATUS[0]} -eq 1` exits 0, proving no production/script/CI/root-doc current references remain.

  **QA Scenarios** (MANDATORY - task incomplete without these):
  ```
  Scenario: CI and scripts no longer reference removed probe
    Tool: Bash
    Steps: Run `rg '\bgol_memory_probe\b|\bPEAK_LIVE_BYTES_LIMIT\b|\bSTEADY_STATE_SPREAD_LIMIT\b' .github scripts --hidden`.
    Expected: Exit code 1 from `rg`, meaning no CI/script references remain.
    Evidence: .sisyphus/evidence/task-4-reference-cleanup-scan.txt

  Scenario: Historical Sisyphus references cannot act as current acceptance gates
    Tool: Bash
    Steps: Review remaining `.sisyphus` hits and write classification to `.sisyphus/evidence/task-4-reference-classification.txt`.
    Expected: Any remaining `.sisyphus` hit is active plan/evidence only or clearly marked historical; no stale instruction asks future agents to run `cargo run --bin gol_memory_probe` as current verification.
    Evidence: .sisyphus/evidence/task-4-reference-classification.txt
  ```

  **Commit**: NO | Message: N/A | Files: [.github/* if needed, scripts/* if needed, .sisyphus/* probe references, root docs if needed]

- [x] 5. Preserve generic runtime and memory verification behavior

  **What to do**: Prove generic runtime memory accounting and array sanitizer behavior still work after removing the GoL probe. Run memory model counters and the array memory sanitizer script. Confirm no diffs to generic runtime/array files unless direct forbidden GoL special-casing was found.
  **Must NOT do**: Do not weaken or remove generic runtime memory accounting APIs. Do not skip sanitizer because the GoL probe was removed.

  **Recommended Agent Profile**:
  - Category: `unspecified-high` - Reason: Potentially long-running verification and careful failure interpretation.
  - Skills: [] - No specialized skill needed.
  - Omitted: [`frontend-ui-ux`] - No UI.

  **Parallelization**: Can Parallel: YES | Wave 2 | Blocks: [6] | Blocked By: [1]

  **References** (executor has NO interview context - be exhaustive):
  - Runtime: `runtime/opal_rc.c`, `runtime/opal_rc.h` - generic runtime memory accounting.
  - Test: `tests/integration_e2e/memory_model_counters.rs` - generic memory counter harness.
  - Script: `scripts/array_memory_sanitizer.sh` - CI-style ASAN/LSAN or valgrind memory verification.
  - Generic tests: `tests/array_integration.rs` - keep unless direct GoL production threshold invocation exists.

  **Acceptance Criteria** (agent-executable only):
  - [ ] `git diff -- runtime/opal_rc.c runtime/opal_rc.h tests/array_integration.rs 2>&1 | tee .sisyphus/evidence/task-5-generic-files-diff.txt` shows no unintended changes, unless the evidence includes direct forbidden GoL special-casing and justification.
  - [ ] `cargo test --features integration --test integration_e2e "memory_model_counters" -- --nocapture --test-threads=1 2>&1 | tee .sisyphus/evidence/task-5-memory-counters.txt` exits 0.
  - [ ] `bash scripts/array_memory_sanitizer.sh 2>&1 | tee .sisyphus/evidence/task-5-array-memory-sanitizer.txt` exits 0.

  **QA Scenarios** (MANDATORY - task incomplete without these):
  ```
  Scenario: Generic memory counters remain valid
    Tool: Bash
    Steps: Run the targeted `memory_model_counters` test command.
    Expected: Exit 0; output indicates memory accounting remains balanced.
    Evidence: .sisyphus/evidence/task-5-memory-counters.txt

  Scenario: CI-style memory sanitizer remains valid
    Tool: Bash
    Steps: Run `bash scripts/array_memory_sanitizer.sh`.
    Expected: Exit 0; no removed GoL probe dependency appears.
    Evidence: .sisyphus/evidence/task-5-array-memory-sanitizer.txt
  ```

  **Commit**: NO | Message: N/A | Files: [.sisyphus/evidence/task-5-*]

- [x] 6. Perform sweeping post-removal analysis and full verification

  **What to do**: Run exact sweeping searches and full verification. Classify any remaining `game-of-life` hits as allowed test-only fixtures, active plan/evidence self-reference, or forbidden production hardcoding. There must be zero forbidden production/compiler/runtime hits.
  **Must NOT do**: Do not count `rg` exit code 0 as success for no-match searches. Do not ignore hidden directories. Do not include `target/` in source sweeps.

  **Recommended Agent Profile**:
  - Category: `unspecified-high` - Reason: Cross-repo verification and classification.
  - Skills: [] - No specialized skill needed.
  - Omitted: [`playwright`] - No browser.

  **Parallelization**: Can Parallel: NO | Wave 3 | Blocks: [F1, F2, F3, F4] | Blocked By: [2, 3, 4, 5]

  **References** (executor has NO interview context - be exhaustive):
  - Forbidden symbols: `PEAK_LIVE_BYTES_LIMIT`, `STEADY_STATE_SPREAD_LIMIT`, `gol_memory_probe`, GoL production probe harness identifiers such as `HARNESS_TEMPLATE` if tied to the probe.
  - Allowed fixture paths: `tests/integration_e2e/game_of_life.rs`, `test-projects/game-of-life/`, `expected_10_frames.txt`.
  - Verification commands: Cargo build/test/clippy/fmt, targeted e2e, memory counters, array sanitizer.

  **Acceptance Criteria** (agent-executable only):
  - [ ] `rg '\bPEAK_LIVE_BYTES_LIMIT\b' --hidden --glob '!target' --glob '!.sisyphus/plans/remove-gol-hardcoding.md' --glob '!.sisyphus/evidence/**' . 2>&1 | tee .sisyphus/evidence/task-6-sweep-peak-limit.txt; test ${PIPESTATUS[0]} -eq 1` exits 0.
  - [ ] `rg '\bSTEADY_STATE_SPREAD_LIMIT\b' --hidden --glob '!target' --glob '!.sisyphus/plans/remove-gol-hardcoding.md' --glob '!.sisyphus/evidence/**' . 2>&1 | tee .sisyphus/evidence/task-6-sweep-spread-limit.txt; test ${PIPESTATUS[0]} -eq 1` exits 0.
  - [ ] `rg '\bgol_memory_probe\b' --hidden --glob '!target' --glob '!.sisyphus/plans/remove-gol-hardcoding.md' --glob '!.sisyphus/evidence/**' . 2>&1 | tee .sisyphus/evidence/task-6-sweep-probe-name.txt; test ${PIPESTATUS[0]} -eq 1` exits 0.
  - [ ] `rg -i '\bgame[_-]?of[_-]?life\b|\bglider\b|\bexpected_10_frames\b' --hidden --glob '!target' --glob '!.sisyphus/plans/remove-gol-hardcoding.md' --glob '!.sisyphus/evidence/**' . 2>&1 | tee .sisyphus/evidence/task-6-sweep-gol-terms.txt; true` runs and every hit is classified as allowed test fixture, historical non-gating doc, or forbidden production hardcoding in `.sisyphus/evidence/task-6-sweep-classification.txt`.
  - [ ] `.sisyphus/evidence/task-6-sweep-classification.txt` states `FORBIDDEN_PRODUCTION_GOL_HITS=0`.
  - [ ] `cargo build --all-features 2>&1 | tee .sisyphus/evidence/task-6-build-all-features.txt` exits 0.
  - [ ] `cargo test --all-features 2>&1 | tee .sisyphus/evidence/task-6-test-all-features.txt` exits 0.
  - [ ] `cargo clippy --all-features --all-targets -- -D warnings 2>&1 | tee .sisyphus/evidence/task-6-clippy.txt` exits 0.
  - [ ] `cargo fmt --all -- --check 2>&1 | tee .sisyphus/evidence/task-6-fmt-check.txt` exits 0.

  **QA Scenarios** (MANDATORY - task incomplete without these):
  ```
  Scenario: Forbidden hardcoded limits are gone
    Tool: Bash
    Steps: Run the exact `rg` commands for `PEAK_LIVE_BYTES_LIMIT`, `STEADY_STATE_SPREAD_LIMIT`, and `gol_memory_probe` with `--hidden --glob '!target'`.
    Expected: Exit code 1 for threshold terms; no unclassified production/source hits for probe name.
    Evidence: .sisyphus/evidence/task-6-sweep-classification.txt

  Scenario: Repository remains healthy after removal
    Tool: Bash
    Steps: Run build, all tests, clippy, and fmt commands exactly as listed.
    Expected: All commands exit 0.
    Evidence: .sisyphus/evidence/task-6-test-all-features.txt
  ```

  **Commit**: YES | Message: `refactor(runtime): remove game of life production hardcoding` | Files: [intentional source/config/doc removals or edits, .sisyphus/evidence/removal proof if tracked]

## Final Verification Wave (MANDATORY — after ALL implementation tasks)
> 4 review agents run in PARALLEL. ALL must APPROVE. Present consolidated results to user and get explicit "okay" before completing.
> **Do NOT auto-proceed after verification. Wait for user's explicit approval before marking work complete.**
> **Never mark F1-F4 as checked before getting user's okay.** Rejection or user feedback -> fix -> re-run -> present again -> wait for okay.
- [x] F1. Plan Compliance Audit — oracle

  **Invocation**:
  ```
  task(subagent_type="oracle", load_skills=[], run_in_background=true,
    description="Audit plan compliance",
    prompt="Audit the completed changes against .sisyphus/plans/remove-gol-hardcoding.md. Verify every implementation task acceptance criterion has evidence, forbidden production GoL hardcoding is removed, allowed e2e fixtures are preserved, and no generic runtime/compiler functionality was weakened. Return APPROVE only if all checks pass; otherwise list blocking defects with file paths and missing evidence.")
  ```
  **Expected Approval Artifact**: `.sisyphus/evidence/f1-plan-compliance-audit.md` containing `APPROVE` or blocking defects.
  **Pass Condition**: Agent output includes `APPROVE`; evidence file exists; no blocking defects remain.

- [x] F2. Code Quality Review — unspecified-high

  **Invocation**:
  ```
  task(category="unspecified-high", load_skills=[], run_in_background=true,
    description="Review code quality",
    prompt="Review the completed removal diff for code quality. Confirm src/bin/gol_memory_probe.rs is removed/de-registered, no replacement hardcoded GoL production knob was introduced, Cargo/module registrations are valid, and changes are minimal. Check git diff and relevant files. Return APPROVE only if clean; otherwise list exact fixes.")
  ```
  **Expected Approval Artifact**: `.sisyphus/evidence/f2-code-quality-review.md` containing `APPROVE` or exact fixes.
  **Pass Condition**: Agent output includes `APPROVE`; evidence file exists; no requested fixes remain.

- [x] F3. Real Manual QA — unspecified-high

  **Invocation**:
  ```
  task(category="unspecified-high", load_skills=[], run_in_background=true,
    description="Run manual QA",
    prompt="Execute real QA for .sisyphus/plans/remove-gol-hardcoding.md after implementation. Run or inspect evidence for cargo build --all-features, cargo test --all-features, the game_of_life_ten_frames test, memory_model_counters, scripts/array_memory_sanitizer.sh, clippy, fmt, and sweep commands excluding active plan/evidence. Return APPROVE only if commands pass and evidence is present; otherwise list failed commands and outputs.")
  ```
  **Expected Approval Artifact**: `.sisyphus/evidence/f3-real-manual-qa.md` containing `APPROVE` or failed commands.
  **Pass Condition**: Agent output includes `APPROVE`; evidence file exists; all listed commands pass.

- [x] F4. Scope Fidelity Check — deep

  **Invocation**:
  ```
  task(category="deep", load_skills=[], run_in_background=true,
    description="Check scope fidelity",
    prompt="Check scope fidelity for the completed removal. Confirm only production/compiler/runtime-facing Game of Life hardcoding was removed; legitimate test-only fixtures in tests/integration_e2e/game_of_life.rs and test-projects/game-of-life/ remain; runtime/opal_rc.c, runtime/opal_rc.h, and generic array tests were not weakened; no unrelated cleanup or broad refactor occurred. Return APPROVE only if scope matches exactly; otherwise list deviations.")
  ```
  **Expected Approval Artifact**: `.sisyphus/evidence/f4-scope-fidelity-check.md` containing `APPROVE` or deviations.
  **Pass Condition**: Agent output includes `APPROVE`; evidence file exists; no deviations remain.

## Commit Strategy
- One commit after all verification passes.
- Message: `refactor(runtime): remove game of life production hardcoding`
- Include only files intentionally changed by the removal and evidence if repository policy tracks `.sisyphus/evidence`.
- Do not commit generated `target/` artifacts.

## Success Criteria
- No production/source binary GoL memory probe remains.
- No `PEAK_LIVE_BYTES_LIMIT` or `STEADY_STATE_SPREAD_LIMIT` remain outside deleted history.
- GoL e2e expected-output test remains passing.
- Generic runtime memory accounting tests remain passing.
- Sweeping search evidence distinguishes allowed test fixture references from forbidden production hardcoding, with no forbidden hits.
