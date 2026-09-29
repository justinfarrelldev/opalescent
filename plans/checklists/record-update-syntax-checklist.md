# Record Update Syntax Implementation Checklist

- [x] Red: add parser/type-checker/codegen/formatter fixtures for block `with:` record updates.
- [x] Red: add terminal-simple-editor usage of block `with:` syntax.
- [x] Green: add AST and parser support for indentation-sensitive `expression with:` blocks only.
- [x] Green: add type checking for product receivers, duplicate fields, unknown fields, and value type compatibility.
- [x] Green: add codegen lowering that reconstructs nominal product values while preserving unchanged fields.
- [x] Green: add formatter support for block `with:` expressions.
- [x] Green: update terminal-simple-editor state helpers to use block `with:`.
- [x] Refactor: review implementation for maintainability and avoid inline `with` support.
- [x] Verify: run targeted tests and terminal-simple-editor checks/builds.
- [x] Review: perform final code review against the proposal and update this checklist.
- [x] Commit: create atomic commits for tests, implementation, and editor adoption.
