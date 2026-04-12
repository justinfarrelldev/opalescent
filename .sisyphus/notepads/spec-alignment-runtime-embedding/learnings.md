# Learnings — spec-alignment-runtime-embedding

## Project: Opalescent Compiler
- Language: Rust
- Build: `cargo build`, `cargo test`, `cargo test --features integration`
- Binary: `./target/debug/opalescent <file.op>`
- Runtime C file: `runtime/opal_runtime.c` (43 lines, to be embedded)

## Key Architecture Facts
- Parser: `src/parser/` — statements.rs, expressions.rs, declarations.rs, helpers.rs
- Lexer: `src/lexer.rs` — does NOT yet emit Indent/Dedent tokens
- Token types: `src/token.rs` — Indent/Dedent exist but unused
- Type system: `src/type_system/` (NOT src/typechecker/)
- Codegen: `src/codegen/`
- AST: `src/ast.rs`
- Compiler entry: `src/compiler.rs` — compile_program(), link_object_file()
- Integration tests: `tests/integration_e2e.rs`

## Runtime Embedding Pattern
- Use `include_str!("../runtime/opal_runtime.c")` in src/compiler.rs
- Write to temp file → pass to `cc` → clean up after link
- Do NOT delete runtime/ folder — it remains as source of truth

## Language Spec Files (source of truth — DO NOT MODIFY)
- language-spec/hello_world.op — 20 lines, tabs, f(args: string[]): void
- language-spec/fib_recursive.op — 21 lines, 4-space indent, int32, if n is 0:
- language-spec/fib_iterative.op — 33 lines, while i <= n: colon-block
- language-spec/simple_quiz.op — 74 lines, import...from, loop =>, guard...into...else, break labels, continue
