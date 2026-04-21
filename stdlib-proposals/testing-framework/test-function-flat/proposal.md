# Test Function Flat

## Overview
This alternative centers the testing API around a single top-level `test` function and small assertion helpers. It minimizes nesting and favors a linear file flow that feels natural in expression-oriented modules.

The primary objective is to deliver a low-friction MVP that still respects Opalescent’s explicit error model. Tests are ordinary function values that return `void` and explicitly declare `errors` when assertions may fail.

## Assumes
- The standard library provides a lightweight `standard/testing` runner.
- Assertion helpers return typed failures, not exceptions.
- Test files can register cases at module load time.
- Initial scope is synchronous CPU execution only.

## Syntax Design
```opal
test("adds values", f(): void errors TestFailure =>
    propagate assert_equal(2 + 2, 4)
    return void
)

test("handles invalid parse", f(): void errors TestFailure =>
    propagate assert_throws(f(): void errors ParseError =>
        propagate parse_decimal('nan')
        return void
    ))
    return void
)
```

## Example Applications
- Utility modules where only a handful of tests are needed.
- CI smoke suites that value compact test files.
- Package templates that want simple defaults before richer features.
- Teams that prefer explicit helper calls over matcher chains.

## Strengths
- Smallest conceptual surface and fastest onboarding.
- Lowest implementation complexity among alternatives.
- Strong fit for explicit `guard` and `propagate` workflows.
- Easy to read in plain, top-down source order.

## Weaknesses
- Less expressive grouping than `describe` hierarchies.
- Hook patterns become manual without extra helper APIs.
- Interaction testing is less discoverable unless mock helpers are imported.
- Large files can become noisy without suite structure.

## Impact on Existing Syntax
No language syntax changes are needed. This can ship as standard-library calls plus a lightweight test runner registry.

## Interactions with Other Concerns
- **Error strategy**: Natural alignment with explicit assertion failures.
- **LSP**: Minimal completion burden, mostly around helper function signatures.
- **Hot reload**: Flat registration model is straightforward for incremental reload.
- **Mocking concerns**: Can still consume the same mock/stub/spy core as richer APIs.

## Implementation Difficulty
Low. Core runner logic and assertion helpers can be implemented quickly, then extended with optional modules for hooks, spies, and snapshots.

## Must NOT Have
- Hidden implicit suite state.
- Exception-based assertion failures.
- Async-only hooks or APIs in this phase.
- Magic global mutation that breaks deterministic execution.
