# Issues / Gotchas

## [2026-04-17] Known issues
- Lambda bodies completely missing — `resolve_callee_function` only emits `emit_default_return`
- Captured var fallback silently pushes `const_zero()` — data corruption
- String interp uses fixed 256-byte malloc + sprintf without bounds checking
- `codegen_cast` uses `sitofp` unconditionally regardless of source signedness
- `codegen_assignment` doesn't check `is_mutable`; `VariableBinding` lacks that field
- Runtime has 0 free() calls across 12 allocation sites
- 123 `#[path]` attributes across 18 files
- `pure` and `untested` keywords completely absent from token.rs/lexer.rs
