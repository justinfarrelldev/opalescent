# Decisions — CLI Help Alignment

## --help behavior
Full pass-through alias: `opal --help pkg` = `opal help pkg`

## Unimplemented command format
Exact string: `error: '<cmd>' not yet implemented`
To: stderr
Exit code: 1

## No-args behavior
UNCHANGED — keep current error + exit 1

## Existing help text
Do NOT change pkg and fmt help text — preserve exactly.

## Visibility
Keep `help_text` and `run_with_args` private (fn, not pub fn).
They only need to be visible within the `#[cfg(test)]` module inside app.rs.

## Commit strategy
- After Task 1: `refactor(cli): extract help text builder and args-based dispatch for testability`
- After Task 3: `feat(cli): expand help to full CLI surface with --help alias and subcommand stubs`
- After Task 5: `docs(readme): update CLI Reference to match expanded help output`

## Task 3 implementation decisions
- Kept help dispatch as explicit early returns for "help" and "--help" to preserve deterministic topic routing.
- Implemented unimplemented subcommand stubs via `if let Some(cmd @ (...))` to satisfy clippy and preserve clear command guard behavior.
- Preserved existing pkg/fmt help text and unknown-topic behavior exactly as required.
