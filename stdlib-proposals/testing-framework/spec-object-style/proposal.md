# Spec Object Style

## Overview
This alternative models tests as explicit data objects (specs) that the runner executes. Instead of free-form registration calls, each case is represented by a typed record with metadata, setup hooks, and executable body fields.

The approach emphasizes declarative structure and toolability. Because specs are plain values, filtering, tagging, retries, and reporting can evolve without changing core syntax.

## Assumes
- The language supports rich record types and closures in fields.
- Runner APIs can iterate arrays of spec objects.
- Assertion failures remain typed and explicit.
- Scope is synchronous CPU tests only.

## Syntax Design
```opal

let math_spec = new TestSpec:
    name: 'adds values'
    tags: ['unit', 'math']
    setup: f(): void => return void
    execute: f(): void errors TestFailure =>
        propagate assert_equal(2 + 2, 4)
        return void
    teardown: f(): void => return void

propagate run_specs([math_spec])
```

## Example Applications
- Metadata-heavy enterprise test pipelines.
- Tag-driven execution for CI shards.
- Policy-enforced suites requiring uniform setup/teardown fields.
- IDE tooling that inspects typed test descriptors.

## Strengths
- Excellent for structured metadata and reporting integrations.
- Strong extensibility for retries, ownership tags, and test categories.
- Explicit object shape improves static analysis and refactoring.
- Natural fit for deterministic execution plans.

## Weaknesses
- More verbose than callback-based styles.
- Slightly less friendly for quick one-off tests.
- Requires careful typing of function fields in spec records.
- Can feel ceremony-heavy for tiny packages.

## Impact on Existing Syntax
No parser changes are required. This approach relies on existing record construction and function field syntax in `.op` and `.types.op` files.

## Interactions with Other Concerns
- **Error strategy**: Spec executors declare `errors` directly.
- **LSP**: Type-aware completion for spec fields is a strong benefit.
- **Hot reload**: Spec objects can be reloaded and diffed deterministically.
- **Testing extensions**: Property and snapshot modes can be additional spec kinds.

## Implementation Difficulty
Medium. Runner internals are moderate, but typed metadata model and execution-plan engine provide long-term benefits for ecosystem tooling.

## Must NOT Have
- Reflection-heavy runtime magic for field discovery.
- Untyped metadata blobs as the only extension mechanism.
- Exception-based spec execution flow.
- Async-only runner assumptions in this phase.
