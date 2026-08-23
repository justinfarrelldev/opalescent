# Decisions

- 2026-08-15: Selected `typed-event-session` only; historical alternatives remain non-v1 records.
- 2026-08-15: Branch target is `feature/terminal-session-input-typed-event-session` from `main`.
- 2026-08-15: Commits must be bisect-green; RED evidence is gated/ignored until activation.
- 2026-08-15: No new Rust dependencies unless explicitly approved later.
- 2026-08-22: Task 23 uses one authoritative prerequisite feature inventory for Tasks 13-22 (`TerminalPublicApiPrerequisites`) instead of scattered booleans or symbol-name checks.
- 2026-08-22: The public adoption gate now opens by default because Tasks 13-22 are implemented, but runtime lowering remains a second mandatory stop; selected terminal/chord/core/test symbols may import and type-check only when prerequisites are satisfied, yet codegen still rejects unimplemented runtime targets with an explicit runtime-readiness diagnostic.
- 2026-08-22: Production vs test-only authority stays unchanged: `standard.testing.terminal` still requires test-only mode, and `standard.system` keeps the narrow implemented error-inspector bypass even when terminal public API prerequisites are disabled.
- 2026-08-23: Task 23 corrective lowering uses a single authoritative runtime-ready inventory for proposal-gated codegen: every `standard.system` core prerequisite function in `CORE_PREREQUISITE_FUNCTIONS` is allowed through once the prerequisite set is satisfied, while selected terminal/chord/test functions remain behind the separate runtime-readiness gate.
- 2026-08-23: The minimal generated-runtime implementation for completed core prerequisites is handle-based and opaque. It intentionally stops short of the later selected terminal/chord public data-model work, but it is real generated-code lowering rather than a facade-only or extern-only path.
