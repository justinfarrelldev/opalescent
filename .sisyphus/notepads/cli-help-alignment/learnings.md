# Learnings — CLI Help Alignment

## Project conventions
- Tests use `#[cfg(test)] mod tests` at bottom of each file (see `src/compiler.rs:302`)
- Error messages use `eprintln!("error: ...")` pattern
- Functions are private by default (`fn`, not `pub fn`)
- `run()` is public; `run_impl()` is private — delegation pattern
- Strict clippy: `cargo clippy -- -D warnings`

## Key files
- `src/app.rs` — primary target, 126 lines
- `README.md` — primary target, CLI Reference at lines 88–174
- Only these two files must be modified

## Dispatch ordering (CRITICAL)
When adding subcommands, the order MUST be:
1. Check `help` / `--help` (before any other dispatch)
2. Check subcommands (`pkg`, `fmt`, `lsp`, `test`, `doc`, `bench`)
3. Collect `--run` flag
4. File-path / compile logic

Reason: Without this order, `opal pkg` would try `fs::read_to_string("pkg")` which errors wrong.
Also: `--run` flag must be after subcommand dispatch to prevent `opal pkg --run` from setting run_flag.

## Subcommand flag surface (from source files)
- pkg: init, add, remove, install, publish
- fmt: --check, --config <path>
- lsp: --stdio
- test: --target <triple>, --filter <pattern>
- doc: --format <md|html>
- bench: (no flags)

## Unimplemented error format
`error: '<cmd>' not yet implemented` — to stderr, exit 1

## Task 3 green phase
- Added help_text topics for lsp, test, doc, and bench before unknown-topic fallback.
- Expanded top-level help command list to include --help alias and documented subcommand surface.
- Added run_with_args dispatch ordering: help -> --help -> subcommand stubs -> --run/file flow.
- Added temporary test-only clippy allowances (with reason) for existing test style in this file.
