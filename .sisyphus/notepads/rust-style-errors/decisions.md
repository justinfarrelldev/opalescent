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
