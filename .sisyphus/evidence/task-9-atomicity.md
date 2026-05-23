# Task 9 Atomicity Evidence

## Scope audited
- `scripts/array_memory_sanitizer.sh`
- `.sisyphus/evidence/task-9-final-verification.txt`
- `.sisyphus/evidence/task-9-atomicity.md`
- `.github/workflows/ci.yml` intentionally unchanged because stress was already explicit opt-in.

## Intended commit boundary
This task should be committed as one atomic verification/evidence change because the sanitizer script wiring and the two Task 9 evidence files describe the same deliverable: deterministic memory verification plus proof that the required commands passed.

### Files that belong together
1. `scripts/array_memory_sanitizer.sh`
   - Adds deterministic exact integration-test selectors for memory verification.
   - Keeps ignored stress behind explicit `OPAL_RUN_STRESS=1` opt-in.
2. `.sisyphus/evidence/task-9-final-verification.txt`
   - Captures the required final command outputs for this exact script wiring.
3. `.sisyphus/evidence/task-9-atomicity.md`
   - Records the intended staging boundary and exclusions for reviewers.

## Explicit exclusions
The current repository contains unrelated in-progress changes from earlier tasks (for example `src/codegen/*`, `tests/integration_e2e/tests.rs`, other `.sisyphus/evidence/*`, and notepad/task files). Those files should **not** be staged as part of Task 9 atomicity evidence.

## Deviations observed during verification
- Initial exact-selector wiring used short test names, which caused Rust's `--exact` filtering to match zero integration tests for the memory hooks.
- Task 9 corrected that by switching to fully-qualified selectors such as `tests::call_temp_leak_regressions::call_temp_take_owned_no_double_free`.
- No CI workflow edit was required because the ignored stress test was already opt-in and remained opt-in.

## Verification summary
- `cargo test --workspace` passed.
- `bash scripts/array_memory_sanitizer.sh` passed with deterministic exact selectors and no sanitizer markers.
- Explicit stress invocation passed with `OPAL_RUN_STRESS=1`.
- `git status`, `git diff`, and `git log --oneline -10` were captured in `task-9-final-verification.txt`.
