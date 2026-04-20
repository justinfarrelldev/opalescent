# Learnings

## 2026-04-20 Session ses_2547b9221ffecHt6C3Ua0J6XH1 — Plan Start

### Codebase Conventions
- Integration tests gated behind `#[cfg(feature = "integration")]`
- Test projects live in `test-projects/`, follow hello-world structure: `opal.toml`, `.gitignore`, `src/main.op`
- `compile_program(source, output_dir)` is the integration test harness entry point
- `cargo test --features integration` runs e2e tests

### Key Pattern: LLVM Type Dispatch (from lower_interpolation_argument, expressions_string.rs:190-259)
```rust
if value.is_pointer_value() { /* string path */ }
if value.is_int_value() {
    let int_value = value.into_int_value();
    let bit_width = int_value.get_type().get_bit_width();
    // dispatch on bit_width: 1 (bool), 8, 16, 32, 64
}
if value.is_float_value() {
    let float_value = value.into_float_value();
    let bit_width = float_value.get_type().get_bit_width();
    // dispatch on bit_width: 32, 64
}
```

### Critical: Bool Memory Management
- `bool_to_string` in runtime/opal_string.c uses `strdup()` — HEAP ALLOCATES
- Must call `free()` on the returned pointer after `puts()`
- Declare `free` inline (not in STDLIB_NAMES): `void(i8*)` signature

### Callee Detection
- MUST detect `print` via AST `Expr::Identifier { name, .. }` where `name == "print"`
- MUST NOT use `FunctionValue.get_name()` — that returns `"puts"` (the resolved function)

### Void Return Handling
- `print_int*` and `print_float*` return void
- Use `try_as_basic_value()` fallback pattern (already in codegen_call_expression ~line 123-137)
- Return `codegen_context.context.struct_type(&[], false).const_zero().as_basic_value_enum()` for void

### Guardrails
- Do NOT modify `resolve_callee_function` or `declare_stdlib_function`'s "print" → puts mapping
- Do NOT modify `resolve_print_to_puts` unit test (lines 682-728 of functions_call.rs)
- Do NOT add unsigned dispatch (print_uint*)
- Do NOT handle arrays/structs/custom types/multi-arg print

## 2026-04-20 F3 Manual QA Learnings
- Manual CLI compile+run flow that matches compile_program usage can be reproduced via: `cargo run --quiet -- <project>/src/main.op` then executing `./target/program`.
- `print-types` manual stdout observed all expected tokens (`42`, `true`, `false`, `hello`) with zero exit status.
- `should-print-final-result` manual stdin piping (`printf "3\n4\n"`) produced expected sum `7` with zero exit status.
