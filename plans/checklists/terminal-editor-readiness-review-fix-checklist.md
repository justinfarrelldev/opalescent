# Terminal Editor Readiness Review Fix Checklist

Keep this checklist current while fixing the review findings.

## Phase 1: planning and checklist hygiene

- [x] RED: record the review findings and follow-up scope in a plan.
- [x] GREEN: create this checklist.
- [x] COMMIT: plan/checklist follow-up commit.

## Phase 2: meaningful chord generated coverage and memory safety

- [x] RED: add/update generated chord fixture/test coverage that fails against the summary-only/stub implementation.
- [x] GREEN: exact chord codegen declarations and return-type metadata.
- [x] GREEN: C runtime chord structs/sequences/router/released-input storage.
- [x] GREEN: match control, named, enhanced-text, and modifiers for the tested editor subset.
- [x] GREEN: released input vectors are allocated/populated and bounds-checked.
- [x] REFACTOR: cleanup helper layout and add using-cleanup wrapper if needed.
- [x] COMMIT: atomic chord coverage/runtime/codegen fix.

## Phase 3: diagnostics honesty and stale red-probe cleanup

- [x] RED: add/adjust assertions showing diagnostics fixture is declaration/status coverage unless it exercises real diagnostic objects.
- [x] GREEN: update diagnostics fixture comments/metadata/docs to avoid overclaiming structured runtime inspection.
- [x] GREEN: remove or accurately rename stale ignored compile-gap probes now superseded by active compile/run coverage.
- [x] REFACTOR: keep active 14-fixture compile/run test authoritative.
- [x] COMMIT: atomic diagnostics/status cleanup.

## Phase 4: UTF-8 malformed-input tightening

- [x] RED: add focused runtime test/probe for valid multibyte and malformed UTF-8 decoding.
- [x] GREEN: validate scalar ranges and continuation rules.
- [x] GREEN: quarantine malformed sequences as UnknownBytes without invalid text emission.
- [x] REFACTOR: isolate UTF-8 helper logic while preserving ASCII/Escape behavior.
- [x] COMMIT: atomic UTF-8 tightening fix.

## Final verification

- [x] Run `cargo test terminal_generated_testing:: --features integration --test integration_e2e -- --nocapture`.
- [x] Run `cargo test terminal_session_all_fixtures_compile_and_run_with_fake_backend --features integration --test integration_e2e -- --nocapture`.
- [x] Run `cargo test terminal_ --lib -- --nocapture`.
- [x] Run `cargo test codegen_terminal_proposal --lib -- --nocapture`.
- [x] Run broader pre-commit checks through normal commit hook.
- [x] Ensure no simple Neovim editor fixture/application was added.
- [x] Ensure prior and follow-up checklists are accurate.
