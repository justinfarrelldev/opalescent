# Process Handle

## Overview
This alternative separates process creation from process completion. `spawn_sync` returns a `ProcessHandle` immediately, then callers read output chunks and explicitly wait for completion through handle methods.

The shape is designed for long-running tools where callers need partial logs, progress polling, or staged shutdown logic, while still remaining fully synchronous and explicit under current Opalescent constraints.

## Assumes
- The runtime can expose handle identity and buffered stream reads through stable types.
- Handle methods can validate process state and return explicit typed errors.
- `_sync` naming remains mandatory for subprocess lifecycle operations.

## Syntax Design
```opal
let process_handle = propagate spawn_sync('worker', ['--mode', 'batch'])
let standard_output_chunk = propagate read_stdout_chunk_sync(process_handle, 4096)
let standard_error_chunk = propagate read_stderr_chunk_sync(process_handle, 4096)
let completion_output = propagate wait_sync(process_handle)
```

The handle API models a process lifecycle: spawn, consume output, wait for exit. Every stage has exhaustive error declarations.

## Example Applications
- `monitor_background_indexer.op`: spawns indexer, streams logs, then waits.
- `capture_incremental_logs.op`: repeatedly reads chunks before final completion.
- `coordinate_staged_shutdown.op`: sends terminate signal and waits with explicit failure handling.

## Strengths
- Best support for long-running process supervision.
- Natural model for incremental output processing.
- Strong future growth path toward structured process orchestration.

## Weaknesses
- Most complex API among alternatives.
- Higher cognitive load for simple one-shot command usage.
- Requires careful lifecycle-state validation in runtime implementation.

## Impact on Existing Syntax
No core language syntax changes. Adds multiple stdlib functions and handle types using existing constructs.

## Interactions with Other Concerns
- Integrates with **logging** alternatives by streaming stderr/stdout into structured sinks.
- Works with **testing-framework** alternatives where fake handles can model timeout and exit races.
- Aligns with **error-strategy/open-error-set** by making each lifecycle phase list precise failure types.

## Implementation Difficulty
Large. Handle state tracking, stream buffering, and deterministic shutdown semantics increase complexity.

## Must NOT Have
- No hidden process cleanup that skips explicit wait or terminate calls.
- No deferred surface or unsuffixed lifecycle methods.
- No collapsing distinct lifecycle failures into a single generic error variant.
