# Decisions

## [2026-04-21] Session ses_251dc2dfeffej3OqV1HXfx3obc

- No async/await anywhere — deferred by user
- `_sync` suffix reserved for I/O-bearing fns
- Cap: 5 alternatives per concern
- No exceptions, Result<T,E>, Option<T> in example code
- Task 0 is idempotent — stubs may or may not exist
- Wave 1 commit groups Tasks 0-4
- Wave 2 commit groups Tasks 5-8
- Wave 3 commit groups Tasks 9-23
- Wave 4 commit groups Tasks 24-26

## [2026-04-21] Logging concern authoring
- Kept three alternatives exactly as requested: global-logger-module, logger-handle, structured-log-events.
- Used flush_sync consistently for I/O-bound durability points; kept level calls as bare log_info/log_warn/log_error/log_debug.
- Added one logging.types.op per alternative so all public ADTs remain in *.types.op files.

## [2026-04-21] Testing framework concern authoring
- Authored exactly five alternatives with required directory names and required file set under `stdlib-proposals/testing-framework/`.
- Added `testing.types.op` to `test-function-flat` and `property-based-testing` to keep type declarations centralized per style rules.
- Kept framework sync-only and CPU-bound, with no `_sync` suffix anywhere in testing API examples.

## [2026-04-21] Momus review decision
- Verdict classified as **NOT OKAY** until completeness metadata is reconciled (`README.md` concern index/counts) and syntax consistency is resolved for `bool` vs `boolean` in proposal `.op` artifacts.
