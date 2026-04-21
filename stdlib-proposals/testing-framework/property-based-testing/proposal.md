# Property-Based Testing

## Overview
This alternative focuses on property-driven verification where generators produce broad input spaces and the framework checks invariant properties. Instead of hand-writing many examples, developers encode behavioral truths that should hold across hundreds or thousands of cases.

The design is deterministic through explicit seeds and typed generator failures. It complements example-based tests by finding edge cases early while preserving Opalescent’s explicit failure flow.

## Assumes
- Standard library provides deterministic pseudo-random primitives.
- Generator APIs can return typed errors.
- Runner supports configurable iteration counts and seed replay.
- Current scope remains synchronous CPU execution.

## Syntax Design
```opal
let integer_generator = range_generator(-100, 100)

propagate check_property(
    'addition commutes',
    integer_generator,
    f(value: int32): void errors TestFailure =>
        propagate assert_equal(value + 1, 1 + value)
        return void
)
```

## Example Applications
- Numeric and collection invariants.
- Parser round-trip properties.
- Serialization and deserialization stability checks.
- Regression reproduction by replaying saved seeds.

## Strengths
- Finds corner cases that example tests can miss.
- Seed replay makes flaky behavior diagnosable.
- Strong long-term value for library correctness.
- Pairs well with explicit type-driven APIs.

## Weaknesses
- Learning curve is higher than example-based tests.
- Good generator design requires practice.
- Shrinking implementation is non-trivial.
- Failure messages need careful formatting for usability.

## Impact on Existing Syntax
No parser changes required. This alternative is implementable as a library plus runner support for deterministic loops and reporting.

## Interactions with Other Concerns
- **Error strategy**: Generator and property failures remain explicit typed errors.
- **LSP**: Helpful around generator combinator signatures.
- **Serialization**: Useful for round-trip properties over value trees.
- **Later deferred**: Property loop architecture can map to deferred sources later.

## Implementation Difficulty
High. Deterministic generation, shrinking, replay artifacts, and rich reporting demand substantial engineering beyond basic test registration.

## Must NOT Have
- Hidden non-deterministic randomness without seed controls.
- Exception-based property abort paths.
- Silent shrinking failures.
- Async-only generator APIs in this phase.
