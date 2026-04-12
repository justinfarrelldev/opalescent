# Decisions — spec-alignment-runtime-embedding

## Architectural Decisions

### Runtime Embedding
- Use include_str! macro at src/compiler.rs (not build.rs)
- Write embedded source to temp file with .c extension for `cc`
- Use std::env::temp_dir() for portability (Linux + macOS)

### Colon-Block vs Brace-Block
- Both syntaxes MUST coexist (no removal of brace support)
- Colon-blocks: detect Colon after condition → emit Indent/Dedent via lexer
- Brace-blocks: existing behavior unchanged

### int32 vs int64
- Both types supported — no removal of int64
- int32 maps to LLVM i32 type
- Widen to int64_t when calling C runtime functions

### Entry Function Args
- Accept f(args: string[]): void but pass dummy/empty value
- None of the 4 test programs actually USE args — dummy binding is fine

### Guard Semantics (Simplified)
- Treat `guard expr into binding else e => ...` as `let binding = expr`
- Skip error path for now — full error type system is future work
- Goal: make simple_quiz.op compile, not implement full error handling

### Import System (Soft)
- No real module loading — just name aliasing to existing runtime functions
- take_input → opal_take_input
- string_to_int32 → opal_string_to_int32
- random_int32 → opal_random_int32

### Indent/Dedent Triggering
- Activate after BOTH: Colon (control flow) AND Arrow => (loop/guard/function bodies)
- Type annotation colons (n: int32) do NOT trigger indent tracking
- Accept both tabs and spaces — do not enforce consistency
