# Command Builder

## Overview
This alternative models subprocess execution as a fluent configuration object. Callers construct `Command` values, append arguments and environment variables, and then execute with `run_sync()`, which keeps advanced invocation readable without large parameter lists.

The builder shape is still explicit and synchronous, so it preserves Opalescent's error-handling and `_sync` naming discipline while allowing richer command configuration than a single free function.

## Assumes
- Opalescent supports method-style calls on values returned by constructors.
- The proposal can define `Command`, `CommandOutput`, and subprocess error enums in `*.types.op` files.
- Builder methods that can fail still use explicit `errors` clauses and must be called with `guard` or `propagate`.

## Syntax Design
```opal
let command = new Command:
    program: 'git'
    arguments: []
    environment_overrides: []

let configured = propagate command.arg('status')
let fully_configured = propagate configured.env('LANG', 'C')
let output = propagate fully_configured.run_sync()
```

`arg` and `env` return a new `Command` with appended configuration. `run_sync` executes and returns `CommandOutput` for a zero exit status.

## Example Applications
- `build_git_command.op`: creates a multi-argument git command with deterministic environment.
- `execute_release_notes_generator.op`: builds and executes a release-notes tool command.
- `prepare_backup_command.op`: configures an archival command with explicit runtime options.

## Strengths
- Clear readability for complex command setup.
- Easier API growth for cwd, stdin policy, and output strategy options.
- Aligns with immutable-by-default style by returning updated `Command` values.

## Weaknesses
- More surface area than a single function.
- Requires users to understand builder lifecycle before first subprocess call.
- Slightly higher implementation complexity around validation sequencing.

## Impact on Existing Syntax
No core syntax changes. Uses existing constructors, method calls, and error declarations in library code.

## Interactions with Other Concerns
- Pairs well with **module-organization** alternatives that prefer cohesive type-plus-method modules.
- Composes with **error-strategy/layered-error-wrapping** when `run_sync` attaches execution context.
- Works with **testing-framework** proposals by allowing command objects to be inspected before execution.

## Implementation Difficulty
Medium. Requires immutable builder operations, method validation, and clear error reporting for malformed commands.

## Must NOT Have
- No mutating global subprocess state.
- No unsuffixed execution call such as `run()` while deferred is unresolved.
- No hidden fallback from non-zero exit into successful output.
