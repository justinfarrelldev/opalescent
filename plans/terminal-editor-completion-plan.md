# Terminal editor completion plan

## Objective

Finish the missing generated-program terminal/editor readiness features needed before a small Neovim-style terminal editor can be authored in Opalescent on the current branch.

## Scope

- Keep `from standard` imports valid for the selected terminal APIs and types while preserving the existing `standard.terminal` module surface.
- Implement generated-code support for terminal input event variant testing and payload access.
- Replace fake-backend-only generated terminal open with a production-capable session path that uses real stdio/raw mode when available while retaining deterministic fake-backend tests.
- Add generated lowerings for terminal size/capability inspectors that are currently declared but not runtime-ready.
- Link the terminal `using` cleanup scaffold.
- Add regression tests first, then implement, then refactor.

## Out of scope

- Full chord-router lowering and complete native-terminal protocol coverage beyond basic editor input keys/text/resize/end-of-input.
- Full conformance for every future terminal/core prerequisite API not needed by the simple editor readiness target.
