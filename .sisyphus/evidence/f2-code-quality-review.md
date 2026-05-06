# F2 Code Quality Review — Pair/zip/double-array continuation

Date: 2026-05-05
Reviewer: Sisyphus-Junior
Base commit inspected: `f3d066a`
Scope focus: `src/codegen/expressions_array.rs`, `src/type_system/checker/expr_collections.rs`, `src/type_system/checker/helpers.rs`, `tests/array_integration.rs`

## Command evidence (required gates)

1. `cargo test --all-features` **FAILED**
   - Unit/lib bins: `ok. 1165 passed; 0 failed; 5 ignored`
   - `tests/array_integration.rs`: `21 passed; 1 failed`
   - Failing test: `tests::array_push_on_immutable_receiver_fails_at_compile_time`
   - Failure detail: panic at `tests/array_integration.rs:211:9` with message `immutable push fixture should fail compilation`

2. `cargo clippy --all-targets --all-features -- -D warnings` **PASSED**
   - Output: `Finished 'dev' profile ...`

3. `cargo fmt --all -- --check` **PASSED**
   - No output (clean formatting)

4. `cargo build --release` **PASSED**
   - Output ends with: `Finished 'release' profile [optimized] target(s) in 17.88s`

5. `git diff --check` **PASSED**
   - No output (no whitespace/conflict-marker issues)

## Static/diff quality review findings

### Scope fidelity checks

- **Pair visible type path preserved**: **PASS**
  - `test-projects/array-zip/src/main.op` exercises expression receiver field reads (`pairs[0].first`, `pairs[0].second`).
  - `tests/array_integration.rs` contains and executes zip-field assertions (`array_zip_runs`, `array_zip_equal_lengths`) and these cases passed in the run.

- **`.zip` truncation behavior preserved**: **PASS**
  - Fixture expected output for unequal arrays is length 2 (`test-projects/array-zip/expected/stdout.txt`).
  - Runtime checks include equal-length and empty-side variants; `array_zip_runs`, `array_zip_equal_lengths`, `array_zip_empty_side` all passed.

- **double-array row-specific length + nested bounds behavior present**: **PASS**
  - In `src/codegen/expressions_array.rs`, nested row metadata is extracted/published (`array.row.len`) and bounds trap uses runtime `length_value`.
  - Integration test `array_double_nested_out_of_bounds_reports_row_length` passed and asserts message `index 0 is out of bounds for length 0`.

### Anti-pattern scan

- `TODO|FIXME|HACK` in reviewed target files: **none found**.
- Tuple-type syntax additions in reviewed checker/codegen files (`Tuple|tuple|CoreType::Tuple`): **none found**.
  - Note: `.0/.1` occurrences in `expr_collections.rs` are tuple-field access on local Rust helper pairs, not language-level tuple feature additions.

## Blockers / non-blockers

### Blockers

1. **Blocking regression in mandatory gate**: `cargo test --all-features` fails.
   - Concrete failure is immutable array push compile-time rejection test unexpectedly succeeding.
   - This is blocker-level for final-wave quality gate regardless of likely origin.

### Non-blockers

1. Clippy strict gate passes with `-D warnings`.
2. Formatting gate passes.
3. Release build passes.
4. Diff hygiene (`git diff --check`) passes.
5. Continuation-direction checks for Pair visibility, zip truncation, and nested-array bounds semantics are present and validated by passing integration cases.

## VERDICT

**REJECT**

Rationale: At least one required quality gate (`cargo test --all-features`) is failing, with a concrete integration regression in immutable `push` compile-time behavior. Despite strong evidence that Pair/zip/double-array continuation direction is implemented and scoped correctly, the mandatory blocker prevents approval.

---

## F2 RERUN (post immutable-array-push checker fix) — 2026-05-05

### Rerun scope additions

- Fix file reviewed: `src/type_system/checker/call_resolution.rs`
- Required static checks run on fix file:
  - Grep anti-pattern scan (`TODO|FIXME|HACK`) → none
  - `lsp_diagnostics` on file → no diagnostics

### Mandatory gates (fresh rerun)

1. `cargo test --all-features` **PASSED**
   - Key evidence from run log:
     - `tests::array_push_on_immutable_receiver_fails_at_compile_time ... ok`
     - `tests::array_zip_runs ... ok`
     - `tests::array_double_nested_out_of_bounds_reports_row_length ... ok`
   - Aggregate summaries in the same run include:
     - `test result: ok. 1165 passed; 0 failed; 5 ignored` (unit/lib)
     - `test result: ok. 22 passed; 0 failed` (`tests/array_integration.rs`)
     - remaining integration/doc test suites also `ok`

2. `cargo clippy --all-targets --all-features -- -D warnings` **PASSED**
   - Output: `Finished 'dev' profile ...`

3. `cargo fmt --all -- --check` **PASSED**
   - No output

4. `cargo build --release` **PASSED**
   - Output: `Finished 'release' profile [optimized] target(s) in 9.89s`

5. `git diff --check` **PASSED**
   - No output

### Fix review findings (`call_resolution.rs`)

- `type_check_call_expr_impl` now calls `ensure_mutating_member_receiver_is_mutable(callee)?` before call-type resolution.
- `ensure_mutating_member_receiver_is_mutable` specifically guards mutating array member calls (`push` / `pop`) by:
  - matching member-call receiver identifiers,
  - checking symbol-table mutability,
  - emitting `TypeError::InvalidOperation` when receiver is immutable.
- This directly addresses the previous blocker test expectation under `opal check` semantics.

### Continuation-scope regression status (Pair/zip/double-array)

- **Pair visibility path:** preserved (zip-field access tests still passing).
- **`.zip` truncation behavior:** preserved (`array_zip_runs` passing, plus equal/empty-side coverage in `array_integration`).
- **double-array nested row-length bounds:** preserved (`array_double_nested_out_of_bounds_reports_row_length` passing).

### Blockers / non-blockers (rerun)

- **Blockers:** none detected in required F2 gates.
- **Non-blockers:** one external librarian background task failed due key quota (`bg_28eb7487`), but this is non-blocking for repo-local gate verification and did not affect required checks.

### RERUN VERDICT

**APPROVE**

Rationale: The prior blocker is cleared (`array_push_on_immutable_receiver_fails_at_compile_time` now passes), all mandatory F2 gates pass, and no blocker-level regression appears in Pair/zip/double-array continuation scope.
