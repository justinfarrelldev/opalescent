# Named Error Sets Implementation Checklist

- [x] Create implementation branch `implement-named-error-sets`.
- [x] Read required language goal documentation before implementation.
- [x] Red: add parser/fixture coverage for `error set` declarations.
- [x] Green: add AST/parser/formatter/doc support for error-set declarations.
- [x] Red: add resolver/type-checker coverage for imported, nested, invalid, duplicate, and cyclic error sets.
- [x] Green: implement canonical expansion, import/export metadata, and Miette diagnostics.
- [x] Red: add propagation/guard/function-compatibility coverage using named sets.
- [x] Green: expand named sets before type checking and compatibility checks.
- [x] Red: add lint coverage for collapsible lists, redundant members, overlaps, and broad unused members.
- [x] Green: implement warning diagnostics and body-derived escaping-error tracking.
- [x] Add `standard.errors` canonical named error-set catalog and compatibility aliases.
- [x] Rewrite applicable `test-projects/` fixtures to canonical aliases.
- [x] Update STDLIB, crash course, docs, LSP hover, generated docs, formatter tests.
- [x] Run targeted tests after each slice and keep status current.
- [x] Run full pre-commit suite without modifying/bypassing hooks.
- [x] Perform final code review for correctness, maintainability, API compatibility, and diagnostics.
