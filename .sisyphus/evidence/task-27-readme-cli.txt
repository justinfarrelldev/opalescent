Task 27 — README alignment to actual implementation

Checks performed
1) CLI wiring truth check against src/app.rs
- app.rs currently dispatches compile/run flow and help topics only.
- It prints help for fmt/pkg topics but does not execute fmt/pkg/lsp subcommands.

2) README claims reviewed and corrected
- Added explicit status note in CLI Reference indicating:
  - `opal pkg`, `opal fmt`, and `opal lsp --stdio` are documented but not dispatched in src/app.rs.
- Added status note in Package Manager section (not wired through app.rs yet).
- Added status note in Formatter section (not wired through app.rs yet).
- Added status note in LSP section near `opal lsp --stdio` (not wired in app.rs).

3) Numeric types documentation check
- Types section updated with explicit complete matrix:
  - int8, int16, int32, int64
  - uint8, uint16, uint32, uint64
  - float32, float64

4) Legacy runtime prefix check
- `opal_` references in README: none.

Verification commands and results
- `cargo test lsp`: PASS (14 passed)
- `cargo test`: PASS (full suite green)
- `cargo make lint`: PASS

Outcome
- README now reflects current CLI behavior accurately while preserving structure.
- Type section explicitly lists all 10 numeric primitives.
