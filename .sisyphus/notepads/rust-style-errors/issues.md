# Issues — rust-style-errors

## [2026-04-17] Session ses_26343eccfffe9zYT9G7Hr6r0vr — Plan Start

### Known Issues to Fix
1. **Span off-by-one bug** (CRITICAL — fix in Task 1):
   - `src/error.rs` `LexError::span_from_span` adds `.saturating_add(1)` to length
   - This makes lexer/parser spans 1 byte longer than typechecker spans for same source range
   - Fix: use `span.end.offset.saturating_sub(span.start.offset)` (no +1)

2. **Only first error shown** (fix in Task 5):
   - `compile_to_module` uses `lex_errors.errors.into_iter().next()` — discards all but first
   - Must collect ALL errors via `CompilationErrorReport`

3. **Bare eprintln! everywhere** (fix in Task 7):
   - `src/app.rs` uses `eprintln!("error: compilation failed: {error}")` throughout
   - Must replace with `render_report()` calls

## [2026-04-17] Task 3 — Renderer Module Notes

- Initial compile failed because `DiagnosticWithSource` did not implement `Debug`; fixed by adding `#[derive(Debug)]`.
