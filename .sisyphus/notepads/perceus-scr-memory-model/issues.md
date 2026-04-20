# Issues — perceus-scr-memory-model

## [2026-04-20] Known Issues / Gotchas

### C Runtime Linking (CRITICAL)
- NO `build.rs` in this project
- C runtime embedded via `include_str!` in `src/compiler.rs`
- Must add `opal_rc.c` to that concat — NOT just to `runtime/opal_runtime.c`

### RESERVED_KEYWORDS Sync
- `RESERVED_KEYWORDS` array in `src/lexer.rs` is used by both lexer AND parser tests
- Must keep alphabetically ordered and in sync when adding `ref` and `weak`

### Parameter Construction Sites
- Adding `passing_mode` field to `Parameter` will cause compile errors at ALL construction sites
- Must update every `Parameter { name, param_type, span }` to include `passing_mode: PassingMode::Owned`

## [2026-04-20] Task 13 Issues / Gotchas

### LLVM value-shape mismatch when emitting RC calls
- Problem: direct `into_pointer_value()` panics when `build_load` returns non-pointer LLVM values (observed with `string[]` entry param lowering yielding `ArrayValue`).
- Mitigation: guard RC emission with `loaded.is_pointer_value()` before pointer conversion in function-entry and return-path RC emission.

### Full suite regressions can surface outside targeted RC tests
- New RC emission initially passed targeted tests but broke existing entry-wrapper/e2e tests.
- Resolution required re-running full `cargo test --lib` and hardening pointer-shape checks.
