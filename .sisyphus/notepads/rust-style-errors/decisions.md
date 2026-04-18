# Decisions — rust-style-errors

## [2026-04-17] Session ses_26343eccfffe9zYT9G7Hr6r0vr — Plan Start

### Renderer approach
- Use `DiagnosticWithSource` wrapper struct that attaches `NamedSource` at render time
- Do NOT add `#[source_code]` to error enums (breaks derives)
- Delegate `Diagnostic` trait methods to inner error

### Multi-error collection
- `compile_to_module` returns `Result<Module, (CompilationErrorReport, String)>` where String is normalized source
- Follow proven pattern from `src/lsp/diagnostics.rs:get_diagnostics()`

### CodegenError
- Add `pub span: Option<miette::SourceSpan>` field
- `CompilerError::Codegen` stores `CodegenError` not `String`
- `src/lsp/diagnostics.rs` must be updated for type compatibility (behavior unchanged)

### Suggestions scope
- ~12 most common error types get suggestions
- NOT all 55 variants

### formatter.rs fate
- Keep during transition, do NOT delete
- Old tests continue to pass

## [2026-04-17] Task 3 — Renderer Module Decisions

### Rendering boundaries
- `render_diagnostic(filename, source, error)` owns miette graphical rendering and always attaches source via `DiagnosticWithSource` + `NamedSource`.
- `render_report(filename, source, report)` iterates `CompilationErrorReport::entries()` and renders lexer/parser/type-checker diagnostics with source context.
- `CompilerError::Codegen(String)` currently renders as plain `error: ...` text in report output because no diagnostic span/source metadata is available in current state.

### Summary footer contract
- Use rust/cargo-style footer text: `error: aborting due to {N} previous error(s)` with explicit singular form for `N=1`.

## [2026-04-17] Task 9 — E2E tests decisions

### Test placement and scope
- Place the six new end-to-end renderer tests in `src/errors/tests.rs` under a dedicated `mod e2e_tests` block to keep error-system tests co-located.
- Use `compile_to_module` (not `compile_program`) so lex/parse/type failures return early as `Err((CompilationErrorReport, normalized_source))` without requiring full external execution.

### Assertion strategy
- Keep rendered-output assertions resilient across miette formatting by checking for source context snippets and broad error indicators (`error` or `×`) rather than brittle full-string snapshots.
