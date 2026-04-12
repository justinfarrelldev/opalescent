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

## T4 & T6: int32 and is Operator Status
- **Status**: Already implemented ✓
- **int32 token**: TokenType::Int32 exists in src/token.rs:140
- **int32 lexer**: Keyword "int32" → TokenType::Int32 (src/lexer.rs:128)
- **int32 codegen**: LLVM i32 type supported, sign-extension at runtime boundaries
- **is operator token**: TokenType::Is exists in src/token.rs:204
- **is operator lexer**: "is" keyword → TokenType::Is (src/lexer.rs:157)
- **is operator parser**: Recognized as BinaryOp::Is at Precedence::Equality (src/parser/precedence.rs:42)
- **is operator codegen**: Emits IntPredicate::EQ for i32/i64 (src/codegen/expressions.rs)
- **Unit tests passing**: test_codegen_is_operator_on_int64_emits_icmp_eq ✓
- **Manual QA**: Created test_is_op.op with int32 and int64 comparisons, both work correctly

## T5 follow-up: lexer file-size hook workaround
- Moving `#[cfg(test)] mod tests` out of `src/lexer.rs` into `src/lexer/tests.rs` and using `#[path = "lexer/tests.rs"] mod tests;` keeps public API stable while dropping `src/lexer.rs` under the 1000-line pre-commit threshold.

## [2026-04-12] Task: T5+T7+T8
- Parser must consume lexer-emitted `Indent`/`Dedent` in all declaration and statement block entry points introduced by `:` / `=>`.
- Added indentation-block parsing via `parse_indent_block()` in `src/parser/statements.rs` and reused it for `if`, `while`, `for`, `loop`, and function/lambda bodies after `=>`.
- `if` now supports colon-block branches and `else:` blocks while preserving existing brace syntax and `else if` chaining.
- Type declarations now require and consume outer `Indent`/`Dedent` around type bodies, and sum-type variant payload fields consume nested `Indent`/`Dedent` blocks.
- Blockless body termination now treats `Dedent` as a hard boundary (`is_blockless_body_terminated`), preventing indented bodies from leaking into top-level declaration parsing.
- Verification: `cargo test` passed with `728 passed; 0 failed; 8 ignored`.
