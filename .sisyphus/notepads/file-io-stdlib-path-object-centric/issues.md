# Issues — file-io-stdlib-path-object-centric

## [2026-04-22] Session start
No issues yet.

## [2026-04-22] T0 preflight issues encountered/resolved
- Initial preflight runs hit multiple blockers before pass:
  - missing required doc comment on `entry main` in temporary `/tmp/preflight_arrays.op`
  - local array `let` annotation mismatch in this path (worked with inferred `let`)
  - missing `for`-iteration variable materialization in codegen (`unknown variable 'item'`)
- Runtime C integration pitfall: including `opal_runtime.h` inside preflight runtime source produced typedef/prototype redefinition conflicts because runtime C sources are concatenated into one translation unit.
  - Workaround used during preflight: keep preflight C file self-contained with local typedefs/prototype instead of including the header.

## [2026-04-22] For-loop codegen verification notes
- `cargo test --features integration` currently fails in this branch due to pre-existing formatter golden/idempotency failures in `tests/fmt_integration.rs` (colon-block `if ...:` formatting vs expected brace style), unrelated to for-loop codegen changes.

## [2026-04-22] T0 preflight final blockers/nuances
- Acceptance required exact stdout lines only; because compiler run mode prefixes `target/program`, evidence stdout was captured from direct executable invocation (`./target/program`) after compiling `/tmp/preflight_arrays.op`.
- Runtime LSP diagnostics for `runtime/` directory timed out due clang option mismatch (`--background-index`, `--clang-tidy` unsupported in this environment). Rust `src/` diagnostics were clean.

## [2026-04-22] T6 infra wiring completion
-  initially failed twice due to policy constraints (missing doc comment on entry, then unhandled error-producing calls); resolved by adding doc block and guarding all four fallible calls.

## [2026-04-22] T6 infra wiring completion
- check /tmp/read_all.op initially failed twice due to policy constraints (missing doc comment on entry, then unhandled error-producing calls); resolved by adding a doc block and guarding all four fallible calls.

## [2026-04-22T22:00:39-04:00] T7 infra wiring notes
- No blocking implementation issues encountered for T7.
- `cargo test --lib` output is tool-truncated by volume in this environment; full command evidence captured from the saved tool output file (`/home/justi/.local/share/opencode/tool-output/tool_db8112f99001YgEeJ8wq73WC1v`) confirming success.

## [2026-04-22T22:07:07-04:00] T8 infra wiring notes
- No implementation blockers in compiler/runtime wiring.
- Workspace file-write tool cannot target `/tmp`; created `/tmp/fmgmt_all.op` via shell heredoc instead and verified with `opalescent check`.

## [2026-04-22T22:14:04-04:00] T8 verification issue resolved
- QA failed with `Symbol 'm1.size_bytes' not found` because checker-local ADT field registry lacked preloaded standard-module metadata fields.
- Resolved by syncing `standard` `ModuleInterface` into `TypeChecker` initialization paths; exact QA now passes via `opalescent check /tmp/fmgmt_all.op`.
