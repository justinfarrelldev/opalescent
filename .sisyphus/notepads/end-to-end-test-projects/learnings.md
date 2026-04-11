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

## [2026-04-11] Task 3: resolve_callee_function stdlib registry
- Where the function is in the codebase: `src/codegen/functions.rs:257` (`resolve_callee_function`) and `src/codegen/functions.rs:338` (`declare_stdlib_function` helper)
- How you structured the registry: a dedicated matcher helper (`declare_stdlib_function`) that declares/reuses precise LLVM externs for known stdlib names and returns `None` for unknown names
- What "print" maps to: puts (`declare i32 @puts(i8*)`)
- How existing callers were affected: `codegen_call_expression` keeps calling through `resolve_callee_function`; now identifier callees either resolve to existing module functions, stdlib externs (`puts`/`printf`), or return a `CodegenError` for unknown names instead of emitting invalid fallback signatures
- Any surprises or edge cases found: preserving monomorphization behavior required skipping explicit generic monomorphization for stdlib aliases (`print`/`printf`) to avoid generating invalid specialized symbols for external C functions

## [2026-04-11] Task 6: String interpolation codegen
- Expr::StringInterpolation structure: `parts: Vec<StringPart>` where each part is either `StringPart::Literal(String)` or `StringPart::Expression(Expr)`.
- Implementation approach: dedicated lowering in `src/codegen/expressions_string.rs` using literal fast-path plus `sprintf` for mixed literal/expression interpolation.
- Buffer size used: 256 bytes stack buffer (`alloca [256 x i8]`) and returned as `i8*`.
- How format string is built: concatenate literal text (escaping `%` to `%%`) and append conversion specifiers per interpolated expression (`%s`, `%lld`, `%d`, `%f`).
- How expression parts are codegen'd: expression values are lowered recursively, then widened/coerced for variadic `sprintf` ABI (`i1 -> i32`, integers -> `i64`, floats -> `f64`).
- Result type: `i8*` pointer to the interpolation buffer so `print(...)` can pass it directly to `puts`.
- Any edge cases or limitations: fixed stack buffer can truncate large outputs; numeric interpolation currently defaults to signed integer widening (`%lld`) and does not yet use static type info for signed/unsigned formatting.

## [2026-04-11] Task 7: hello-world end-to-end integration test
- Added `hello_world_compiles_links_and_runs` to `tests/integration_e2e.rs` using existing `prepare_dir`/`cleanup_dir` helpers and assertion style.
- Test reads source from `test-projects/hello-world/src/main.op` via `std::fs::read_to_string`, compiles with `compile_program(source_str.as_str(), Path::new("test-projects/hello-world/target"))`, executes binary, asserts stdout contains `Hello world`, and asserts successful exit status.
- Cleanup is guaranteed by running `cleanup_dir(temp_dir)` after execution block and asserting cleanup success before final outcome assertion, preventing lingering artifacts even when execution checks fail.
- Linux linker compatibility required adding `-no-pie` in `link_object_file` (`src/compiler.rs`) to avoid PIE relocation failure (`R_X86_64_32S`) when linking emitted object files during integration execution.

## [2026-04-11] Task 9: `is` operator integer equality codegen verification
- `BinaryOp::Is` was already correctly lowered in `src/codegen/expressions.rs` through the shared comparison path (`codegen_cmp`) to `IntPredicate::EQ` for integer operands, producing LLVM `icmp eq`.
- Added TDD regression coverage in `src/codegen/tests.rs`:
  - `test_codegen_is_operator_on_int64_emits_icmp_eq` verifies direct expression lowering includes `icmp eq i64`.
  - `test_fibonacci_if_n_is_zero_compiles_to_valid_llvm_ir` compiles recursive fib source using `if n is 0 { ... }`, verifies module validity, and asserts IR contains integer equality compare.
- RED step exposed a test harness issue (module had no emitted instructions because literals-only compare was constant-folded from IR print perspective); resolved by binding and comparing an identifier (`x is 5`) so emitted IR deterministically contains `icmp eq i64`.
- Additional fib test adjustment: use `entry main = f(): void` and avoid `print(result)` in this regression source to prevent unrelated pre-existing stdlib call-signature verification failures from masking `is` behavior.
