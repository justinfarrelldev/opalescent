# Learnings — rust-style-errors

## [2026-04-17] Session ses_26343eccfffe9zYT9G7Hr6r0vr — Plan Start

### Key Technical Facts
- `Position` struct: `line: usize`, `column: usize`, `offset: usize` (1-based line/column)
- `Span` struct: `start: Position`, `end: Position`
- `Span::len()` uses exclusive end: `end.offset - start.offset` (NO +1)
- `LexError::span_from_span` has a BUG: adds `.saturating_add(1)` to length — must fix first
- `TypeError::span_from_span` is CORRECT (exclusive end, no +1)
- Tab normalization: `source.replace('\t', "    ")` in `compile_to_module` before lexing
- All span offsets reference the tab-normalized source
- `unknown_span()` returns `SourceSpan(0, 0)` — must handle gracefully (no panic)
- miette 7.0 with `fancy` feature is already a dependency but NEVER used for rendering
- CLI uses bare `eprintln!("error: compilation failed: {error}")` everywhere
- Only the FIRST error is ever shown: `lex_errors.errors.into_iter().next()` discards the rest
- `CompilationErrorReport` exists but is only used by the LSP

### Opalescent Syntax (for suggestions)
- `let mutable x = ...` (NOT `let mut`)
- `f(params): return_type =>` (NOT `fn`)
- Single-quoted strings `'hello'` (NOT `"hello"`)
- `entry main = f(args: string[]): void =>` (entry point format)
- `propagate` keyword (like `?` in Rust)
- `guard ... into ... else { }` for local error handling

### Guardrails
- Do NOT add `#[source_code]` to `LexError`, `ParseError`, or `TypeError` enums
- Do NOT modify `#[diagnostic]`, `#[label]`, or `#[help]` attributes on error variants
- Do NOT change LSP diagnostic BEHAVIOR — only update `src/lsp/diagnostics.rs` for type/signature compatibility
- Do NOT add suggestions to all 55 error variants — limit to ~12 most common
- Do NOT thread spans through all codegen expression functions — function-level only

## [2026-04-17] Task 3 — Renderer Module

### Implementation Learnings
- `GraphicalReportHandler::render_report` can write directly into a `String` sink, so renderer helpers can remain allocation-light and avoid stderr hooks.
- A wrapper diagnostic must implement `Debug` because `miette::Diagnostic` inherits from `std::error::Error` (`Error: Debug + Display`).
- Attaching `NamedSource<String>` at render time works cleanly by delegating all `Diagnostic` methods to the inner diagnostic and overriding only `source_code()`.
- Zero-width spans from `TypeError::unknown_span()` render safely with miette graphical output without special-casing.

## [2026-04-17] Task 5 — Multi-error collection in compiler pipeline

### Implementation Learnings
- `compile_to_module` can mirror the proven LSP diagnostics collection pattern by initializing one `CompilationErrorReport` and using `extend_lex_errors`, `extend_parse_errors`, and `extend_type_errors` at each phase boundary.
- Returning `Err((report, normalized_source))` preserves the exact tab-normalized source used for lexing/parsing/type-checking, which is required for accurate source-span rendering in downstream `render_report` usage.
- For compatibility with existing external error shape, `compile_program` can down-map a single codegen entry in the report back into `CompileError::Codegen`, while surfacing frontend multi-errors through a dedicated `CompileError::Report` variant that carries both report and normalized source.
- Multi-error tests are more reliable when using independent declaration-level type failures (e.g., one function return type mismatch plus one undefined symbol in another function), avoiding short-circuit behavior that can arise inside a single declaration body.
