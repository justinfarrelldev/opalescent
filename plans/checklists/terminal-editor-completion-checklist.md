# Terminal editor completion checklist

- [ ] Red: add focused regression tests for `from standard` terminal imports, event variant/payload access, size/capability lowerings, production open, and terminal `using` cleanup linking.
- [ ] Green: re-export selected terminal/core symbols and terminal type declarations from `standard` without breaking existing `standard.terminal` imports.
- [ ] Green: lower terminal sum variant `is` checks and `into` refinements with payload field access in generated code.
- [ ] Green: make generated terminal sessions open without `OPAL_TERMINAL_FAKE_BACKEND`, using raw mode on real TTYs and retaining fake injected events for deterministic tests.
- [ ] Green: implement runtime/codegen declarations for `terminal_session_size_sync`, `terminal_session_capabilities`, and basic capability inspectors.
- [ ] Green: provide the terminal `using` cleanup scaffold symbol.
- [ ] Refactor: keep helper code localized and avoid one-off fixture hacks.
- [ ] Self-validation: run targeted Rust and integration tests that demonstrate all previously observed missing-feature errors are resolved.
