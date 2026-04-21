# Run Command Function

## Overview
This alternative provides a single synchronous API for subprocess execution: `run_command_sync(program, arguments)`. It intentionally favors directness over configurability, so most users can run external commands with one call and one obvious error path.

The design keeps process execution compatible with Opalescent's explicit error handling by returning `CommandOutput` and declaring failures in an exhaustive `errors` clause.

## Assumes
- Opalescent keeps explicit `errors` clauses as the only fallible-flow mechanism.
- Subprocess concerns remain synchronous at this stage, so every public subprocess function uses `_sync`.
- The concern can define `CommandOutput` and related error types in a local `*.types.op` file.

## Syntax Design
```opal
let run_command_sync = f(program: string, arguments: string[]): CommandOutput errors SpawnError, NonZeroExitCode, IoError =>
    # Runtime-owned implementation
    return output
```

The function accepts a program path plus a `string[]` argument list, executes immediately, and either:
- returns `CommandOutput` when exit code is zero
- returns `SpawnError`, `NonZeroExitCode`, or `IoError` on failure

## Example Applications
- `capture_git_revision.op`: runs `git rev-parse HEAD` for build metadata.
- `check_toolchain_version.op`: runs `opalescent --version` and records stdout.
- `run_database_migration.op`: runs a migration binary and maps known failures.

## Strengths
- Minimal API surface and low learning overhead.
- Strongly aligned with explicit `guard`/`propagate` flow.
- Straightforward runtime implementation and documentation.

## Weaknesses
- No fluent configuration for environment variables or stdin policies.
- Harder to extend without adding optional parameters or new helper functions.
- Less ergonomic for advanced invocation scenarios with many options.

## Impact on Existing Syntax
No parser or core syntax changes. This is a standard library API shape using existing function and error grammar.

## Interactions with Other Concerns
- Composes with **error-strategy/error-code-enum-module** by expressing all subprocess failures as explicit enum variants.
- Aligns with **logging** concerns where stderr output is forwarded to structured logs.
- Can integrate with **testing-framework** concerns by injecting fake command runners in test doubles.

## Implementation Difficulty
Quick. One public API plus data/error type definitions, with no builder state machine.

## Must NOT Have
- No exceptions or hidden panic-based subprocess signaling.
- No deferred, callbacks, futures, or unsuffixed subprocess execution names.
- No implicit error swallowing of non-zero exit codes.
