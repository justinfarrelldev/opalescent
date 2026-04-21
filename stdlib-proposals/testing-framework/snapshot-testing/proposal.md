# Snapshot Testing

## Overview
This alternative provides snapshot assertions for complex outputs such as structured records, formatted text, and serialized trees. A test compares current output against a stored snapshot and reports a typed mismatch when drift occurs.

The model is optimized for regression detection with minimal assertion boilerplate. It is intentionally deterministic and explicit: snapshot read, compare, and update flows are all modeled through typed errors.

## Assumes
- A stable serialization format exists for snapshot payloads.
- Snapshot storage paths are deterministic within the test runner.
- Snapshot I/O errors are represented as explicit types.
- Current phase remains synchronous CPU execution.

## Syntax Design
```opal
propagate assert_snapshot_equal(
    'user-card-renders',
    render_user_card(user_record),
    f(name: string): string errors SnapshotError =>
        propagate load_snapshot(name)
    )
```

## Example Applications
- UI text rendering snapshots.
- Large configuration output regression checks.
- Serializer output stability over version changes.
- Golden-file style testing for compiler diagnostics.

## Strengths
- Very high signal for broad output regressions.
- Low assertion noise for complex structures.
- Easy review workflows when snapshot diffs are clear.
- Integrates well with deterministic runner outputs.

## Weaknesses
- Requires snapshot hygiene and review discipline.
- Risk of over-approving incorrect updates.
- Less precise than targeted assertions for small logic checks.
- Storage and diff tooling add maintenance overhead.

## Impact on Existing Syntax
No language changes required. Snapshot support is library-plus-runner behavior with deterministic file conventions.

## Interactions with Other Concerns
- **Error strategy**: Snapshot compare and storage failures remain typed.
- **Serialization**: Strong dependency on canonical output encoding.
- **LSP**: Optional editor support for snapshot navigation and updates.
- **Testing ecosystem**: Can compose with describe/it and flat test styles.

## Implementation Difficulty
Medium. Core compare API is straightforward, but robust tooling for update workflows, stable diff formatting, and storage conventions requires careful design.

## Must NOT Have
- Implicit auto-update of snapshots without explicit opt-in.
- Exception-based mismatch signaling.
- Non-deterministic snapshot ordering.
- Async-only snapshot APIs in this phase.
