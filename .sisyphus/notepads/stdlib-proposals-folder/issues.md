# Issues

## [2026-04-21] Session ses_251dc2dfeffej3OqV1HXfx3obc — None yet

## [2026-04-21] Logging concern authoring
- lsp_diagnostics could not validate .op or .md files in this environment because no LSP server is configured for those extensions.
- Repository-wide ./.style-gate.sh reports many pre-existing violations outside stdlib-proposals/logging, so scoped verification used concern-only checks.

## [2026-04-21] Testing framework concern authoring
- lsp_diagnostics could not be run for `.op`/`.md` artifacts in this environment due to missing LSP configuration for those extensions.

## [2026-04-21] Stdlib proposals style-gate + coverage sweep
- Ran `cat stdlib-proposals/.style-gate.sh` and executed full gate; initial run reported forbidden `[T]` style, missing `_sync` names in time-date parsing APIs, missing scenario comments in many `.op` files, and coverage-check call-site misses.
- Fixed violations only inside `stdlib-proposals/`: added scenario comments, corrected `[T]` usages to `T[]` forms, renamed time-date parse functions to `_sync`, added missing proposal method call-site markers required by coverage checker, and removed forbidden async terms from proposal content.
- Final verification: `bash stdlib-proposals/.style-gate.sh` exits 0; `proposal.md` count is 58 (>=58); `.op` count is 168 (>=100); no empty directories; all `proposal.md` files are <=250 lines.
- Async keyword grep in `stdlib-proposals/` is clean excluding `stdlib-proposals/.style-gate.sh` itself (script intentionally contains the detection regex).
- `lsp_diagnostics` could not validate `.op`/`.md` in this environment because no LSP server is configured for those extensions.

## [2026-04-21] Cross-concern consistency audit (all 19 concerns)
- Audited all 19 concern folders and all alternatives; verified every concern has `COMPARISON.md`, every alternative has `proposal.md`, and every alternative has at least 2 `.op` files.
- Fixed `_sync` naming consistency in file I/O samples by renaming: `record_log_event` → `record_log_event_sync` and `load_app_config` → `load_app_config_sync` across `path-object-centric`, `handle-based`, and `whole-file-operations` variants.
- Fixed type-placement violations by moving inline `type` declarations from usage `.op` files into new `*.types.op` files and importing them:
  - `error-strategy/layered-error-wrapping/layered_errors.types.op`
  - `error-strategy/error-code-enum-module/error_codes.types.op`
  - `error-strategy/open-error-set/open_errors.types.op`
  - `optional-representation/absence-via-errors/absence_errors.types.op`
  - `optional-representation/maybe-tagged-union/server_config.types.op`
- Removed inline `type ...` blocks from 13 affected `proposal.md` files to keep types out of prose docs.
- Forbidden token sweep (`async|await|Promise|Future`) is now clean across `stdlib-proposals/`.
- Semicolon scan in `*.op` is clean.
- `[T]` array declaration syntax scan (`:\s*\[[A-Za-z_]`) is clean in `*.op`.
- One residual non-scope issue remains: `stdlib-proposals/.reference-patterns.md` still contains an illustrative `type` snippet; left unchanged because it is not a concern folder/alternative artifact and was outside required fixes.
- Environment limitation: `lsp_diagnostics` for `.op`/`.md` cannot run here (no LSP configured for those extensions).

## [2026-04-21] Momus review blockers
- Completeness mismatch: `stdlib-proposals/README.md` lists `testing-framework` with 1 alternative, but the folder currently has 5 alternatives (`vitest-style-describe-it`, `test-function-flat`, `spec-object-style`, `property-based-testing`, `snapshot-testing`).
- Syntax mismatch versus language-spec baseline: proposal `.op` files still contain `bool` type usages (e.g., `serialization/json-plus-toml-uniform-api/examples.op`, `compression/stream-compressor-object/compression_stream.types.op`, `testing-framework/vitest-style-describe-it/testing.types.op`) while spec examples use `boolean`.
- Verification environment limitation persists: `lsp_diagnostics` cannot validate `.op` files in this environment because no `.op` LSP is configured.
