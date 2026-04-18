# Issues — pure-keyword

## [2026-04-18] Session ses_260fd3b04ffecmESbccQV6hvfs

### Watch out for SymbolInfo construction sites
- Task 3 must update ALL `SymbolInfo { ... }` construction sites with `is_pure: false`
- Missing any site will cause a compilation error (Rust struct fields are exhaustive)
- Use `ast_grep_search` pattern `SymbolInfo {` to find all sites
- Pattern in declarations.rs:178-188 currently does NOT destructure `modifiers` — need to add `ref modifiers`

### Integration test syntax note
- Integration tests use BRACE syntax `{ }`, NOT colon-block syntax
- See `test_integration.rs:10-14` comment for details
- Source strings must be valid Opalescent syntax as parsed by the current parser

### Existing tests reference InvalidOperation
- `test_type_check_pure_function_rejects_print_call` (tests.rs:500-553) currently expects `InvalidOperation`
- Task 5 must UPDATE this test to expect `PurityViolation` instead
- This is intentional migration, not a regression

## [2026-04-18] Session task-2-impure-list-expanded

### Test construction gotcha
- A first draft used `guard ... else return ...` inside a `let` initializer in a pure function and produced `InvalidOperation("return outside of function")` under this AST shape.
- Using `propagate string_to_int32(...)` in an errors-declaring pure function avoided the issue and cleanly validated pure builtin allowance.

### Lint gotcha in existing-style matches
- Pattern style `&TypeError::InvalidOperation { ref operation, .. }` triggers Clippy `needless_borrowed_reference` with `-D warnings`.
- Use `TypeError::InvalidOperation { operation, .. }` in `matches!` closures over iterated references.

- `SymbolInfo` field expansion caused exhaustive initializer compile failures across checker and test helpers; resolved with a tree-wide struct initializer update pass plus targeted pure-function registration logic.

## [2026-04-18] Session task-5-transitive-purity-enforcement

- Initial transitive check over `symbol_table.lookup(name)` incorrectly blocked pure stdlib conversion builtins because they are registered as symbols with `is_pure: false` metadata by default.
- Fix: bypass transitive user-function purity check when `environment.lookup_builtin(name)` succeeds, while still rejecting names in `IMPURE_STDLIB_FUNCTIONS`.

## [2026-04-18] Session task-6-pure-entry-rejected

- No implementation blockers; key gotcha was test construction: avoid `create_entry_program` for this case because it appends its own `main`, which can hide or alter the single-entry scenario needed for `pure entry` validation.
