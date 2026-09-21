# Terminal editor completion checklist

- [x] Red: add focused regression tests for `from standard` terminal imports, event variant/payload access, size/capability lowerings, production open, and terminal `using` cleanup linking.
- [x] Green: re-export selected terminal/core symbols and terminal type declarations from `standard` without breaking existing `standard.terminal` imports.
- [x] Green: lower terminal sum variant `is` checks and `into` refinements with payload field access in generated code.
- [x] Green: make generated terminal sessions open without `OPAL_TERMINAL_FAKE_BACKEND`, using raw mode on real TTYs and retaining fake injected events for deterministic tests.
- [x] Green: implement runtime/codegen declarations for `terminal_session_size_sync`, `terminal_session_capabilities`, and basic capability inspectors.
- [x] Green: provide the terminal `using` cleanup scaffold symbol.
- [x] Refactor: keep helper code localized and avoid one-off fixture hacks.
- [x] Self-validation: run targeted Rust and integration tests that demonstrate all previously observed missing-feature errors are resolved.

## Self-validation evidence

- [x] `from standard` terminal function/type imports compile in generated terminal fixture coverage.
- [x] Generated `TerminalInputEvent` `is` checks, `into` refinements, and payload field access compile and run against fake typed events.
- [x] `terminal_session_open_sync` no longer requires `OPAL_TERMINAL_FAKE_BACKEND` for generated programs.
- [x] `terminal_session_size_sync` and basic capability inspector lowerings compile, link, and run.
- [x] Lexical `using session = ...` links `__opal_using_cleanup_terminal_session_close_sync` and runs cleanup successfully.
- [x] Existing `standard.terminal` imports still compile and run in terminal stdlib/generated integration tests.
