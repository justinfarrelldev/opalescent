# Learnings — fmt-preserve-comments

## Project Conventions
- Commit style: Conventional Commits — `fix(fmt):`, `test(formatter):`, `refactor(ast):`, `chore(sisyphus):`
- Build: `cargo build` / `cargo test` / `cargo clippy -- -D warnings`
- Integration tests: `cargo test --features integration --test fmt_integration`

## Formatter Pipeline
- `FormatCommand` → `Formatter::format_source` → `Lexer::tokenize` → `Parser::parse` → `print_program` (AST printer) → `rules::apply_all`
- Lexer correctly produces `TokenType::Comment` and `TokenType::DocComment` tokens
- Parser's `skip_newlines_and_comments()` (26 call sites) discards them entirely
- Doc comments before declarations ARE collected into `Decl::doc_comment: Option<Documentation>` but printer ignores them (uses `..`)

## Key File Locations
- Printer: `src/formatter/printer.rs` — `print_decl()` line 296, `print_program()` line 283
- AST: `src/ast.rs` — `Stmt` enum line 569, `Decl` enum line 729
- Parser helpers: `src/parser/helpers.rs` — `skip_newlines_and_comments()` line 223
- Parser top-level: `src/parser.rs` (NOT src/parser/mod.rs)
- Documentation struct: `src/ast/documentation.rs` — `Documentation { raw, sections, attributes, span }`
- Unit tests: `src/formatter/tests.rs`
- Integration tests: `tests/fmt_integration.rs`
- Golden fixtures: `test-projects/fmt-test/src/` and `test-projects/fmt-test/expected/`

## Documentation.raw
- Contains comment text sans `##` delimiters
- Reconstruction: `##\n{raw}\n##`

## Decl variants with doc_comment
- `Decl::Function`, `Decl::Type`, `Decl::Let` all have `doc_comment: Option<Documentation>`
- `Decl::Import` has NO `doc_comment` field

## Scope Boundaries
- IN: Doc comments before declarations, single-line comments between declarations, comments between statements in function bodies, file-header comments
- OUT: Inline trailing comments, comments inside type definition variant bodies, comments inside expressions/match arms
