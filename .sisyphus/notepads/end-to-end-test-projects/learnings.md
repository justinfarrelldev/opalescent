# Learnings

## [2026-04-11] Plan Analysis

### Project Structure
- Binary-only crate: `src/main.rs` (no `src/lib.rs` yet)
- Module declarations are all in `src/main.rs:11-41`
- Existing tests live in: `src/codegen/tests.rs`, `src/type_system/tests.rs`, `src/type_system/test_integration.rs`

### Inkwell Pattern
- `compile_to_module<'ctx>(context: &'ctx Context, source: &str) -> Result<Module<'ctx>, CompileError>`
- Caller creates `Context::create()` and passes `&context` so Module lifetime is valid
- `compile_program` creates its own Context internally (no lifetime leaks)

### Codegen Known Issues
- `resolve_callee_function` uses fallback `i64 fn()` for ALL unknown functions including stdlib
- `Expr::StringInterpolation` has no codegen match arm
- Import system has no codegen handler

### Type System Known Issues
- `string_to_int32`: registered as `f(string): int32 errors ParseError` — needs update to `f(string): int64` (no errors) in BOTH checker.rs AND module_resolver.rs
- `random_int32`: registered as `f(int32, int32): int32` — needs update to `f(int64, int64): int64` in BOTH files
- Tests at `src/type_system/tests.rs:~4612` and `~4629` must be updated in Task 13
- `src/type_system/test_integration.rs:460` uses `guard string_to_int32(...)` — must be updated to plain call

### .op File Syntax Rules
- Brace syntax `{ }` only (NOT colon-block)
- Entry functions: `f(): void` (zero args)
- Integer types: `int64` only (NOT `int32`)
- Mutable vars: `let mutable` (NOT `let mut`)
- No loop-as-expression, no multi-binding let, no labeled break payloads

### Runtime Linking
- `runtime/opal_runtime.c` linked as source: `cc program.o runtime/opal_runtime.c -o program`
- No pre-compiled .o files outside `test-projects/<name>/target/`

### Integration Tests
- Gate ALL file-writing tests behind `#[cfg(feature = "integration")]`
- Place in `tests/integration_e2e.rs`
- Add `integration` feature to `Cargo.toml [features]`
- Write to `test-projects/<name>/target/` ONLY
- MUST clean up all artifacts after each test

## [2026-04-11] Task 1: compile_to_module + lib.rs
- compile_to_module placed in: `src/compiler.rs`
- CompileError placed in: `src/compiler.rs`
- Module path exposed as: `opalescent::compiler::compile_to_module`
- Cargo.toml changes: added explicit `[lib]` and `[[bin]]` targets for dual crate structure
- Any surprises or issues encountered: needed a small return-lowering fix in `src/codegen/control_flow.rs` so `return void` emits a valid LLVM `ret void` for module verification

## [2026-04-11] Task 2: object emission + linker invocation + e2e compile
- `emit_object_file(module, path)` must initialize native target support before creating `TargetMachine`; otherwise object emission fails at runtime on fresh processes.
- `compile_program(source, output_dir)` should create its own `Context` and produce deterministic artifacts: `program.o` then `program` in the caller-provided output directory.
- Integration tests need strict hygiene: gate under `feature = "integration"`, keep temporary artifacts inside `test-projects/<name>/target`, and remove outputs after assertions to avoid repository pollution.
- Linker error ergonomics are much better when `CompileError::Linker` captures stderr from `cc`; this makes e2e failures actionable without rerunning under verbose shell tracing.
