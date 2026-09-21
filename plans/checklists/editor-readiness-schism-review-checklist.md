# Editor Readiness Schism Review Checklist

Branch: `implement-editor-readiness-proposals`

Method: each schism and correctness gap must first be proven with a failing test, compile-time probe, runtime fixture, or source-level conformance check before it is fixed or disproven. Use red-green-refactor and atomic commits. Correctness gaps include declared-but-not-returned errors, Rust/C behavioral divergence, incomplete Unicode conformance, missing generated-runtime coverage, and any docs/proposal mismatch discovered during the review wave.

## 1. Full grapheme/cell-width layout is incomplete

- [x] PROVE: add failing Rust runtime tests for regional-indicator flag graphemes, keycap emoji sequences, and clipping that must not split them.
- [x] PROVE: add/generated fixture coverage for at least flag and keycap layout behavior in compiled Opalescent.
- [x] FIX/DISPROVE: update Rust terminal text layout segmentation/width policy to pass the new cases or document why the suspected case is outside the proposal.
- [x] FIX/DISPROVE: update C generated-runtime layout segmentation/width policy to match Rust behavior.
- [x] FIX/DISPROVE: document Unicode version/scope and ambiguous-width policy.
- [x] REFACTOR: run focused layout unit/integration tests and formatting.
- [x] COMMIT: atomic grapheme conformance schism commit.

## 2. `TerminalTextLayoutError` shape is collapsed versus proposal variants

- [x] PROVE: add a failing fixture/test showing negative limits surface only generic `TerminalTextLayoutError` instead of a specific `NegativeCellLimit` variant/leaf.
- [x] FIX/DISPROVE: implement a specific negative-limit error name/variant and register/import it consistently, or prove current language error model cannot express proposal variants yet and update proposal/docs accordingly.
- [x] REFACTOR: run focused layout error tests.
- [x] COMMIT: atomic terminal text layout error-shape commit.

## 3. Array editing declares `AllocationFailureError` but generated lowering traps or ignores allocation failures

- [x] PROVE: add a codegen or runtime-internal test showing `insert` allocation failure currently lowers to `opal_runtime_error`/trap instead of an error aggregate.
- [x] PROVE: add a test showing string `remove_at` removed-value duplication ignores the `string_insert_at` error field.
- [x] FIX/DISPROVE: change `insert` and `remove_at` lowering to return `AllocationFailureError` for array allocation failure.
- [x] FIX/DISPROVE: handle removed string duplication allocation failure through the declared error path.
- [x] REFACTOR: run focused array/codegen tests.
- [x] COMMIT: atomic array allocation-error contract commit.

## 4. High-level session rendering is only a subset of the proposed surface

- [x] PROVE: add failing generated/runtime tests for `terminal_session_set_cursor_visible_sync` and `terminal_session_set_cursor_shape_sync` readiness or explicitly prove they are out of selected v1 scope.
- [x] FIX/DISPROVE: either implement generated C/runtime readiness for cursor visibility/shape or update proposal/checklist/docs to state clear/move/draw/bell is the selected v1 subset.
- [x] REFACTOR: run focused terminal rendering tests.
- [x] COMMIT: atomic session rendering surface commit.

## 5. Invalid cursor position error behavior diverges between Rust and C/session metadata

- [ ] PROVE: add a failing Rust runtime test that expects invalid cursor position to be distinguishable from generic session write failure.
- [ ] PROVE: add/focus a generated C fixture or test for invalid cursor position error naming.
- [ ] FIX/DISPROVE: align Rust terminal session move-cursor invalid-position error with generated C/type metadata, or adjust type metadata to the actual Rust error model.
- [ ] REFACTOR: run focused terminal error tests.
- [ ] COMMIT: atomic invalid-cursor-position alignment commit.

## 6. Harness-injected fake backend is minimal compared with selected generated testing concern

- [ ] PROVE: add failing tests for multi-event fake input or typed event inspection expectations, or prove selected implementation intentionally chose a smoke subset.
- [ ] PROVE: add a source-level check that required production leakage protections remain enforced.
- [ ] FIX/DISPROVE: implement multi-event scripted fake backend support and output assertions or document narrow v1 scope with tests.
- [ ] REFACTOR: run focused terminal fake-backend tests.
- [ ] COMMIT: atomic fake-backend scope/conformance commit.

## 7. Method-style array editing only supports identifier receivers in codegen

- [ ] PROVE: add failing fixture/test for expression receiver chaining such as `make_values().insert(...)` or prove proposal only requires identifier receivers.
- [ ] FIX/DISPROVE: support expression receivers or update docs/proposal to explicitly constrain v1 lowering to identifier receivers.
- [ ] REFACTOR: run focused array expression-receiver tests.
- [ ] COMMIT: atomic array receiver-scope commit.

## 8. Array editing error coverage is too thin

- [ ] PROVE: add currently missing red/focused tests for invalid `remove_at`, negative `insert`, negative `remove_at`, and removed-value behavior.
- [ ] FIX/DISPROVE: implement fixes if any new tests expose contract failures.
- [ ] REFACTOR: run focused array tests.
- [ ] COMMIT: atomic array coverage commit.

## 9. Generated terminal session rendering remains fake-backend-only

- [ ] PROVE: add a test or documentation check showing generated `terminal_session_open_sync` without fake backend fails.
- [ ] FIX/DISPROVE: either implement production generated terminal opening or document that generated production terminal sessions remain gated/future work.
- [ ] REFACTOR: run focused terminal generated tests.
- [ ] COMMIT: atomic generated terminal production-scope commit.

## 10. Named harness-injected fake-backend proposal file is absent

- [ ] PROVE: add a repository/docs check or manual checklist entry showing `stdlib-proposals/terminal-generated-testing/harness-injected-fake-backend/proposal.md` is absent.
- [ ] FIX/DISPROVE: add the proposal file or update references to the selected comparison section.
- [ ] REFACTOR: validate proposal formatting/links if applicable.
- [ ] COMMIT: atomic proposal-file consistency commit.

## Final verification

- [ ] Run all focused tests added during schism review.
- [ ] Run broader pre-commit verification.
- [ ] Ensure every schism section has PROVE evidence marked complete before FIX/DISPROVE completion.
- [ ] Ensure commits remain atomic and pre-commit hook is not modified or bypassed.
