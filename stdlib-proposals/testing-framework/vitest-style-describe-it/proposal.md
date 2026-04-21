# Vitest-Style Describe/It

## Overview
This alternative introduces a suite-oriented testing API shaped like Vitest while remaining fully aligned with Opalescent’s explicit error model. The core surface is `describe`, `it` and `test`, with `expect` matchers, lifecycle hooks, and complete mocking utilities.

The design goal is to provide a familiar test authoring model for application and library developers without introducing exceptions, implicit failures, or deferred-only assumptions. All test failures are modeled as typed errors and are handled via `guard` or `propagate` in runner internals.

## Assumes
- The standard library can expose a `standard/testing` module.
- Test failures are represented as explicit sum types.
- The runtime can execute closures and capture deterministic call metadata for spies.
- Current scope is sync-only CPU tests.

## Syntax Design
```opal
# Suite organization
describe("math/addition", f(): void =>
    before_all(f(): void =>
        return void
    )

    before_each(f(): void =>
        return void
    )

    it("adds positive values", f(): void errors TestFailure =>
        expect(2 + 2).to_equal(4)
        expect(4 > 0).to_be_truthy()
        return void
    )

    test("throws on invalid parse", f(): void errors TestFailure =>
        expect(f(): void errors ParseError =>
            propagate parse_decimal('nan')
            return void
        ).to_throw()
        return void
    )

    after_each(f(): void =>
        return void
    )

    after_all(f(): void =>
        return void
    )

    return void
)

# Full mocking toolkit
let logger_mock = mock_fn(f(message: string): void =>
    return void
)

let service_spy = spy_on(payment_gateway, 'charge_customer')
let clock_stub = stub(clock_module, 'now_millis', f(): int64 =>
    return 1700000000
)

expect(logger_mock).to_have_been_called_times(1)
expect(logger_mock).to_have_been_called_with('charged customer')
```

## Example Applications
- Unit testing for parsing, business rules, and domain logic.
- Contract testing with deterministic stubs for clocks and random sources.
- Interaction testing where spy call traces verify orchestration logic.
- Regression suites that combine suite hooks with mock reset policies.

## Strengths
- Familiar suite syntax with minimal onboarding friction.
- Complete mock/stub/spy surface for realistic service testing.
- Hooks support predictable setup and teardown lifecycles.
- Works naturally with typed `errors` and explicit propagation.
- Clear migration path for future snapshot and property plugins.

## Weaknesses
- Largest API surface among alternatives.
- Requires robust matcher and call-trace internals.
- Slightly heavier boilerplate than one-function test style.
- Hook ordering and isolation semantics must be specified carefully.

## Impact on Existing Syntax
No parser changes are required if this is implemented as library calls and closures. Optional formatter/linter updates may be useful for nested suite readability and matcher-chain style consistency.

## Interactions with Other Concerns
- **Error strategy**: Test assertions produce typed failures and avoid exception semantics.
- **LSP**: Enables completion for matcher chains and mock metadata helpers.
- **Hot reload**: Suite discovery remains function-call based and tool-friendly.
- **Serialization**: Can interoperate with snapshot serializers later.

## Implementation Difficulty
High. Requires runner scheduling, matcher subsystem, lifecycle state machine, and first-class instrumentation for mocks, stubs, and spies. Core effort is substantial but reusable by other alternatives.

## Must NOT Have
- Hidden exception-based assertion control flow.
- Async-only API requirements in this phase.
- Implicit global mutable state that leaks across suites.
- Non-deterministic call counting in mocks or spies.
