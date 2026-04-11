# Decisions

## [2026-04-11] Architectural Decisions

### Library Crate Structure
- Create `src/lib.rs` with all `pub mod` declarations
- `src/main.rs` becomes a thin binary that `use opalescent::...`
- This enables `tests/` directory to access compiler internals

### compile_to_module Signature
- Accepts `&'ctx Context` from caller
- Returns `Module<'ctx>` tied to that lifetime
- Location: `src/compiler.rs` or `src/codegen/driver.rs` (Task 1 decides)

### compile_program Signature
- `compile_program(source: &str, output_dir: &Path) -> Result<PathBuf, CompileError>`
- Creates Context internally (no lifetime leaks)
- Outputs `program.o` and `program` inside `output_dir`

### Stdlib Mapping (Task 3)
- `print` → `declare i32 @puts(i8*)`
- `printf` → `declare i32 @printf(i8*, ...)`
- Unknown functions → meaningful error (NOT silent i64 fallback)

### Feature Flag
- Feature name: `integration`
- Location: `Cargo.toml [features]` section
- All file-writing tests use `#[cfg(feature = "integration")]`
