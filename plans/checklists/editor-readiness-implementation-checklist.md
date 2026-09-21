# Editor Readiness Implementation Checklist

Branch: `implement-editor-readiness-proposals`

## 1. Scalar range string editing primitives

- [x] RED: add failing tests/fixtures for `string_insert_at`, `string_delete_range`, and `string_replace_range` success behavior.
- [x] RED: add failing tests/fixtures for scalar range error behavior.
- [x] GREEN: register the functions in type checking and module resolution.
- [x] GREEN: implement Rust runtime helpers.
- [x] GREEN: implement C runtime ABI helpers for generated programs.
- [x] GREEN: add codegen declarations and runtime return type mapping.
- [x] GREEN: update user-facing docs/prelude if needed.
- [x] REFACTOR: run focused tests and clean duplication.
- [x] COMMIT: atomic scalar-range string editing commit.

## 2. Method-style array editing

- [ ] RED: add failing tests/fixtures for `values.insert(index, value)`.
- [ ] RED: add failing tests/fixtures for `values.remove_at(index)` and labeled multiple return use.
- [ ] RED: add compile-fail/error tests for invalid indices or unhandled errors.
- [ ] GREEN: add parser/type-checker/member dispatch support for method-style array insert/remove.
- [ ] GREEN: implement generated runtime lowering for insert/remove.
- [ ] GREEN: update docs/prelude if needed.
- [ ] REFACTOR: run focused array tests and clean duplication.
- [ ] COMMIT: atomic method-style array editing commit.

## 3. High-level session rendering

- [ ] RED: add failing Rust/runtime tests for session clear/move/draw/bell helpers against the fake backend.
- [ ] RED: add generated-code/type-check probes for high-level rendering symbols.
- [ ] GREEN: add Rust stdlib/runtime model helpers.
- [ ] GREEN: add selected symbols, borrow metadata, and generated runtime readiness where supported.
- [ ] GREEN: add C runtime ABI helpers if generated programs can call these helpers in this phase.
- [ ] GREEN: update docs/prelude if needed.
- [ ] REFACTOR: run terminal rendering tests and clean duplication.
- [ ] COMMIT: atomic high-level session rendering commit.

## 4. Full grapheme cell width terminal layout

- [ ] RED: add failing tests for ASCII, combining mark, emoji, CJK wide, and clipping behavior.
- [ ] GREEN: implement grapheme segmentation and terminal cell width following Rust/Golang precedent.
- [ ] GREEN: expose `terminal_text_cell_width` and `terminal_text_clip_to_cells` through stdlib/type-check/codegen/runtime as appropriate.
- [ ] GREEN: update docs/prelude if needed.
- [ ] REFACTOR: run focused layout/string tests and clean duplication.
- [ ] COMMIT: atomic grapheme cell width commit.

## 5. Harness-injected fake backend

- [ ] RED: add failing integration test proving a generated terminal fixture can receive injected fake events and assert captured output.
- [ ] GREEN: implement test harness injection path without production `standard.testing.terminal` leakage.
- [ ] GREEN: activate the smallest terminal generated fixture as a real compile/run test.
- [ ] REFACTOR: run focused terminal generated tests and clean duplication.
- [ ] COMMIT: atomic harness-injected fake backend commit.

## Final verification

- [ ] Run focused unit/integration tests for all implemented areas.
- [ ] Run broader test suite required by touched subsystems or document blockers.
- [ ] Ensure checklist accurately reflects final status.
- [ ] Ensure all commits are atomic and pre-commit hook is not modified or bypassed.
