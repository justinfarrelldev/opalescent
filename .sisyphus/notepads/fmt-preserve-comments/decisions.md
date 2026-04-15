# Decisions — fmt-preserve-comments

## AST Design
- Use NEW enum variants (`Stmt::Comment`, `Decl::Comment`) rather than trivia fields on existing variants — avoids breaking all construction sites
- `text` field stores raw comment text INCLUDING `#` prefix (e.g., `"# This is a comment"`)
- For multi-line comment blocks (non-doc `## ... ##`), store entire block as-is including delimiters

## Parser Strategy
- Rather than modifying all 26 `skip_newlines_and_comments()` call sites:
  1. Handle `Comment` tokens at top-level in `Parser::parse()` in `src/parser.rs` → emit `Decl::Comment`
  2. Handle `Comment` tokens in `parse_indent_block` in `src/parser/statements.rs` → emit `Stmt::Comment`
  3. Leave `skip_newlines_and_comments()` unchanged for contexts where comments should still be skipped

## Scope Exclusions (Explicit)
- No inline trailing comment support
- No comments inside type definitions
- No comments inside expressions
- No comment text normalization
