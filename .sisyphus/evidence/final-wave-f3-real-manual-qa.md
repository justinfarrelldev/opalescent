# Final Wave F3 — Real Manual QA (Execution-Focused)

Date (UTC): 2026-05-23T01:25:41Z
Plan Context: `.sisyphus/plans/game-of-life-memory-leaks.md` (read-only)

## Verdict

**APPROVE**

The required command-level QA was executed hands-on in this workspace and all mandatory gates passed with concrete observed output. Stress remains opt-in, deterministic memory hooks execute real fully-qualified exact selectors, and the stress run completed in bounded time under the defined hard-timeout envelope.

## Commands Executed and Observed Outcomes

### 1) Workspace regression gate
```bash
cargo test --workspace
```
- Observed result: **PASS**
- Observed summary from captured run output:
  - `test result: ok. 1272 passed; 0 failed; 5 ignored; 0 measured; 0 filtered out`
  - Integration/doc-test phases also completed without failures.

### 2) Deterministic sanitizer + memory hook gate
```bash
bash scripts/array_memory_sanitizer.sh
```
- Observed result: **PASS**
- Observed behavior:
  - Targeted sanitizer selectors ran serially and passed.
  - Mandatory memory verification hooks ran using exact selectors, each as a single test (`running 1 test ... ok`).
  - Stress remained opt-in by default, with explicit skip message when unset:
    - `INFO: skipping ignored stress verification; set OPAL_RUN_STRESS=1 to enable 'tests::game_of_life_full_memory_stress::game_of_life_full_memory_stress'.`
  - Final script line:
    - `PASS: array memory sanitizer regression completed with no sanitizer error markers.`

### 3) Opt-in stress gate (explicit)
```bash
OPAL_RUN_STRESS=1 cargo test --features integration --test integration_e2e tests::game_of_life_full_memory_stress::game_of_life_full_memory_stress -- --ignored --exact --nocapture --test-threads=1
```
- Observed result: **PASS**
- Observed output:
  - `running 1 test`
  - `test tests::game_of_life_full_memory_stress::game_of_life_full_memory_stress ... ok`
  - `test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 181 filtered out; finished in 15.48s`

## Manual QA Checks Against Required Assertions

### A) Stress is opt-in only
- Verified in script (`scripts/array_memory_sanitizer.sh`):
  - `if [[ "${OPAL_RUN_STRESS:-0}" != "1" ]]; then ... skipping ... fi`
  - Opt-in execution path uses:
    - `OPAL_RUN_STRESS=1 cargo test ... -- --ignored --exact --nocapture --test-threads=1`
- Verified in test (`tests/integration_e2e/game_of_life_full_memory_stress.rs`):
  - `#[ignore = "stress test: opt-in via --ignored and OPAL_RUN_STRESS=1"]`
  - runtime gate `should_run_stress()` checks `OPAL_RUN_STRESS == "1"`.

### B) Selectors are fully-qualified and exact-filter safe
- Verified in script `MEMORY_VERIFICATION_TESTS` entries, e.g.:
  - `tests::memory_model_counters::memory_model_counters`
  - `tests::rc_store_leak_regressions::...`
  - `tests::call_temp_leak_regressions::...`
- Verified execution mode:
  - `cargo test --features integration --test integration_e2e "${test_name}" -- --exact --nocapture --test-threads=1`
- Hands-on run confirmed non-zero actual execution per selector (`running 1 test` for each), i.e., no accidental zero-match exact filters.

### C) Stress cannot run indefinitely (timeout/kill path)
- Implementation presence verified in `tests/integration_e2e/game_of_life_full_memory_stress.rs`:
  - `STRESS_WINDOW = 15s`
  - `HARD_TIMEOUT = 20s`
  - `kill_and_reap_child(...)` used for cleanup/reap.
- Observed execution remained bounded in practice:
  - explicit stress run finished in `15.48s`, below hard timeout.
- Existing prior evidence (`.sisyphus/evidence/task-8-stress-timeout.txt`) is consistent with this behavior (`15.38s`, exit 0).

## Claimed Evidence vs Actual Behavior (Mismatch Audit)

- **No contradiction found** between required claims and hands-on execution outcomes.
- Nuance noted: green-path logs do not print explicit `kill_result` telemetry; kill/reap instrumentation is code-level and failure-message-level, while bounded completion is what is directly observable in successful command output.

## Final Decision

**APPROVE** — Required hands-on QA commands were executed, outcomes were directly observed, memory verification hooks were confirmed to run intended exact tests, stress remained correctly opt-in, and bounded stress execution behavior was validated.