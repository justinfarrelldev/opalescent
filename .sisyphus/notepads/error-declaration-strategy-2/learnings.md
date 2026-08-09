# Learnings

- Task 1 adds `test_stdlib_error_family_taxonomy_covers_all_current_leaves` and `test_complete_leaf_list_warning_mentions_exact_replaceable_errors` in `src/type_system/tests.rs`.
- The taxonomy test is declarative: it enumerates every approved family/member set and checks every current produced leaf is catalogued; the broad filesystem family additionally retains registered-but-unproduced `LineOutOfRangeError` and `SetPermissionsError`.
- RED evidence is captured with `set -o pipefail` in `.sisyphus/evidence/task-1-taxonomy-red.txt` and `.sisyphus/evidence/task-1-warning-red.txt`.
- Task 2 implements the declaration-only registry in `src/type_system/error_families.rs`, registers names from `TypeChecker::register_standard_builtins`, and retains leaf error signatures unchanged. `error_type_is_covered_by_declared_type` is directional: a declared family covers its listed emitted leaf, and a declared leaf covers only itself.
- Task 2 verification passed: `cargo fmt --check`; `cargo test type_system::tests::test_stdlib_error_family_taxonomy_covers_all_current_leaves -- --nocapture`; `cargo test type_system::tests::test_stdlib_family_preserves_manual_leaf_declarations -- --nocapture`; `cargo test type_system::tests::test_complete_leaf_list_warning_mentions_exact_replaceable_errors -- --nocapture`; `cargo make test`; and `cargo clippy --all-targets --all-features -- -D warnings`.

## Task 3 — family coverage wiring

- Added `TypeChecker::declared_error_type_covers` in `src/type_system/checker.rs`, which delegates nominal leaf/family matching to `error_type_is_covered_by_declared_type` and preserves exact matching for every other type.
- Routed ordinary `propagate`, named guard propagation, and guard-wrapper return compatibility through that shared predicate in `checker/fallible_expressions.rs` and `checker/expressions_guard.rs`; function and lambda clauses share this function-error scope.
- Added `test_stdlib_error_families_cover_function_declaration_members` and `test_stdlib_error_families_cover_lambda_members` in `src/type_system/tests.rs`. They cover ParseError, BytesError, StringRangeError, OutputError, TerminalError, TimeError, ProcessEnvError, FilesystemPathError, manual leaves, and wrong-family lambda rejections.
- Verification passed: both Task 3 focused tests, existing propagate mismatch/lambda and guard regressions, `cargo fmt --check`, `cargo make lint`, and `cargo make test` (1381 passed, 8 ignored).

## Task 3 corrective lint fix

- Root cause: `declared_error_type_covers` matched `&CoreType` values with explicit `&... ref` patterns, which strict Clippy correctly rejected as needless borrowed references.
- Replaced those patterns with match-ergonomic `CoreType::Generic` patterns; the captured names and type-argument vectors remain borrowed, and the registry still receives borrowed `&str` names.
- Verification passed: `cargo test type_system::tests::test_stdlib_error_families_cover_function_declaration_members -- --nocapture`; `cargo test type_system::tests::test_stdlib_error_families_cover_lambda_members -- --nocapture`; `cargo fmt --check`; `cargo clippy --all-targets --all-features -- -D warnings`; and `cargo make test` (1381 passed, 8 ignored).
- Correction: the repository enables conflicting strict pattern lints across standalone Clippy and the hook profile. The final helper therefore preserves exact equality first, then compares rendered nominal names; non-empty generic arguments render as `Name<...>` and cannot match a zero-argument family entry, so directional family coverage is unchanged.
- Final verification passed under both profiles: `cargo clippy --all-targets --all-features -- -D warnings`, `cargo make lint`, both focused Task 3 tests, and `cargo make test` (1381 passed, 8 ignored).

## Task 4 — replaceable stdlib error-list warnings

- Added `Warning::ReplaceableErrorList` with code `opalescent::type_system::warning::replaceable_error_list`, a `complete replaceable error list` label, dynamic exact replacement help, source span, and suppression support.
- Added `TypeChecker::warn_for_replaceable_error_list`, which normalizes duplicate resolved names and chooses one eligible registry family by `specificity_rank`, member count, then lexical name. It suppresses a candidate when that family is already declared and preserves the taxonomy member order in help.
- Called the selector after error resolution in function declarations and lambda expressions; it only collects warnings and never rewrites source or returns an error.
- `test_complete_leaf_list_warning_mentions_exact_replaceable_errors` now checks actual non-fatal collection, exact code/help/label, `BytesError`, and ordered `HexDecodeError, SliceRangeError`. `test_stdlib_error_family_warning_selection_for_lambda_overlap_and_extra_errors` covers lambda `TimeError`, `SinkClosedError` overlap selection (`OutputError`), duplicates, and unrelated extra errors. `test_stdlib_error_family_warning_negative_cases` covers partial, declared-family, singleton `ParseError`, and intentional manual leaves.
- Verification passed: the three direct warning tests, evidence commands in `.sisyphus/evidence/task-4-warning-exact.txt` and `task-4-warning-negative.txt`.

## Task 5 — rendered error-family diagnostics

- CLI `opal check` previously collected type-check warnings but dropped them on its success path; it now renders every `checker.warnings()` entry with `render_diagnostic` to stderr before printing `check passed`.
- `tests/integration_e2e/warning_diagnostics.rs` creates temporary valid sources and invokes the built CLI, proving both a top-level function and lambda warning exit successfully while rendering the replacement warning code, warning presentation, `main.op` source label, `complete replaceable error list` label, BytesError help, and `HexDecodeError, SliceRangeError` leaves.
- No test-project fixtures were modified or added; temporary sources keep the rendered-warning checks deterministic and independent of runtime failures.
- Verification passed: the two targeted rendered-warning tests, `cargo test --features integration`, `cargo fmt --check`, strict Clippy, the pre-commit line-count check, `cargo make lint`, `cargo make test`, and `cargo make build`.


## Task 7 — test-project error-family remediation

- Exhaustive `grep`/CLI audit covered every syntactic `errors ... =>` clause under `test-projects/**/src/**/*.op`; `.sisyphus/evidence/task-7-errors-clause-inventory.md` records all classifications. Sixteen complete leaf lists remain intentionally exact because standalone `opal check <file>` cannot render a warning for modules or multi-file fixtures.
- `tests/integration_e2e/stdlib_error_family_test_projects.rs` retains pre-remediation CLI assertions for all 31 confirmed replacements (29 initially visible plus the independently exposed `OutputError` list in `game-of-life` and `ProcessEnvError` list in `process-api-smoke`). Each assertion checks successful exit, `check passed`, warning code, exact family, taxonomy leaf order, and help text; it reconstructs the old declaration after remediation.
- Remediated fixture sources: `_absolute_path_sync`, `_fs_append_log`, `_fs_dir_inventory`, `_fs_read_text_lines`, `_fs_write_text_atomic`, `bytes-hex-roundtrip`, `delete-downloads`, `delete-downloads-strict`, `fs-directory-operations`, `fs-markdown-roundtrip`, `fs-path-manipulation`, `game-of-life`, `op-cat`, `print-text-flush-without-newline`, `process-api-smoke`, `process-cwd`, `process-env`, `process-paths`, `saferm/main_backup`, both stdout-writer fixtures, both string-builder fixtures, `string-ranges-stdlib`, `string-search-stdlib`, all three terminal-move-cursor fixtures, and `windows-file-ops`.
- Evidence: `task-7-pre-remediation-all-checks.txt`, `task-7-test-project-warnings-confirmed.txt`, `task-7-post-remediation-all-checks.txt`, `task-7-test-projects-remediated.txt`, and `task-7-errors-clause-inventory.md` under `.sisyphus/evidence/`.


## Task 7 corrective project-level audit

- `opal build` used the full project checker but previously discarded `checker.warnings()`; `src/compiler.rs` now renders those warnings with each module's source, matching the successful `opal check` path without changing warning selection.
- Project-level `opal build` evidence in `task-7-project-level-pre-remediation.txt` proved six `FilesystemPathError` warnings in `saferm` (`src/main.op:23` and `src/trash.op:28,43,70,95,110`). Each was replaced only after the retained ignored pre-remediation build assertion passed; the normal project build assertion proves `saferm` is warning-free afterward.
- `_fs_append_log`, `_fs_write_text_atomic`, `fs-directory-operations`, and `fs-markdown-roundtrip` build successfully with no replacement warning. `game-of-life-full` fails its existing full-project compilation path before rendering a replacement warning. These ten clauses remain exact with command/output-based inventory reasons.

## Task 6 — stdlib error-family documentation reconciliation

- `STDLIB.md` now lists all 21 approved declaration families with exact taxonomy members, keeps precise emitted leaf signatures distinct from family coverage, and explicitly states that manual leaf declarations remain valid while replacement warnings are non-fatal suggestions that never rewrite or reject source.
- Corrected `InvalidSleepDurationError` to `InvalidDurationError`; documented `InvalidFrameRateError` on `frame_clock_wait_next_sync`, `SinkClosedError` on all terminal operations that emit it, and `IndexOutOfBoundsError` for array `.at(...)`. The family table explicitly covers `ParseError`, `HexDecodeError`, `SliceRangeError`, and `InvalidUtf8Error`.
- Resolver audit: `ModuleResolver` exposes only the registered public `standard`/`process` symbol lists, while `register_stdlib_error_family_types` registers declaration-only family names directly in `TypeChecker`; no module-resolver exports or import fixture changes are required.
- Verification passed: Task 6 canonical/family documentation assertions (evidence: `task-6-docs-canonical.txt`, `task-6-family-docs.txt`), `cargo fmt --check`, targeted function/lambda/manual-leaf family tests, `cargo make lint`, `cargo make test` (1383 passed, 8 ignored), and `cargo make build`.


## Task 8 - stdlib error helper consolidation and final audit

- Consolidated duplicated payload-free stdlib error CoreType constructors into error_families::stdlib_error_core_type; bytes, string, stdout/terminal, time, filesystem, process, and family registration now use it without changing names, taxonomy membership, or error signatures.
- .sisyphus/evidence/task-8-stdlib-error-audit.txt records all 39 currently produced unique leaves, their declared-family coverage, and the intentionally registered-but-unproduced LineOutOfRangeError and SetPermissionsError.
- Final verification passed: cargo test (1383 passed, 8 ignored), cargo test --features integration (219 passed, 4 ignored), cargo fmt --check, and cargo clippy --all-targets --all-features -- -D warnings.

- Saferm pre-remediation warning proof must copy the complete `test-projects/saferm` tree, restore `FilesystemPathError` to the historical `PermissionDeniedError, InvalidPathError` leaves in both `src/main.op` and `src/trash.op`, and assert all six emitted replacement diagnostics before retaining the warning-free checked-in project build.


## Saferm warning-location corrective proof

- The reconstructed Saferm pre-remediation integration assertion now splits rendered diagnostics by the replacement-warning code and consumes one block per expected clause. Each expected block must contain the source file and line (`src/main.op:23` or `src/trash.op:28,43,70,95,110`), declaration label, exact `FilesystemPathError` family, ordered leaves, and exact replacement help; duplicate or misattributed warnings cannot satisfy multiple expectations.
- Verification passed: focused Saferm pre-remediation test, `cargo fmt --check`, `cargo make lint`, `cargo make test`, `cargo make build`, and `cargo test --features integration`.

## Final compliance review after `ab0b99a`

- The exact-source helper rejects `src/main.op:230` when `src/main.op:23` is expected, and all six `stdlib_error_family_test_projects` integration assertions pass. Current `cargo fmt --check` fails because the `&& [fixture.label, fixture.family, fixture.leaves, help.as_str()]` expression should be formatted across multiple lines; plan final verification is therefore not yet compliant.


## Saferm warning-location boundary fix

- Tightened `block_contains_exact_source_location` in `tests/integration_e2e/stdlib_error_family_test_projects.rs` so a matched source token only counts when the next byte is a real boundary (`:`, whitespace, or end-of-input). This rejects `src/main.op:23x` as intended while preserving the existing `src/main.op:23:` warning format.
- Added regression coverage for both bad continuations: `src/main.op:230` and `src/main.op:23x`.
- Verification passed: `cargo fmt --check`; `cargo test --features integration stdlib_error_family_test_projects -- --nocapture`; and the focused Saferm boundary test under the `integration` feature.
