# Issues — fmt-tabs-spaces-fix

## [2026-04-15] Session Start

### I1: Suspected Bug Location
`rules::apply_all()` takes no config param. If it performs any indentation normalization,
it cannot respect `use_tabs`. Task 1 will confirm whether this is the actual bug.

### I2: Doc Comments Dropped by Formatter
`print_decl` for `Decl::Function` ignores `doc_comment: Option<Documentation>`.
Formatting a file with doc comments produces output without them — effectively destroying the file.
MITIGATION: All test `.op` files must avoid doc comments. This is NOT fixed in this plan.

### I3: Line Comments Also Dropped
`#` comments are lexer tokens, not AST nodes. No comment-printing logic in printer.
Same mitigation: avoid `#` comments in test files (or accept they'll be stripped in formatter output).

## [2026-04-15] Task 1 execution note

### I4: Pre-commit hook sensitivity to unrelated working-tree size checks
- Repo hook enforces Rust file line-count limits globally and can block a narrow commit if unrelated files exceed limits in the working tree state.
- Isolating staged content allowed committing only `src/formatter/tests.rs` while preserving unrelated local changes.
