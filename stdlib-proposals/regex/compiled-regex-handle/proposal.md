# Compiled Regex Handle

## Overview
This alternative introduces an explicit `Regex` handle value created by a one-time compile operation. Callers compile a pattern once, then invoke methods such as `.match`, `.find_all`, `.replace`, and `.split` repeatedly against input strings.

The central motivation is predictable performance and explicit intent. If a caller plans to reuse the same pattern across many inputs, the source code should show that reuse directly instead of recompiling implicitly at each call.

## Assumes
- Opalescent keeps explicit `errors` clauses and guard-based handling as the only failure path
- Standard library types can expose methods while remaining plain values
- Regex execution is CPU-bound and remains synchronous without `_sync` naming
- The runtime can store compiled automata inside a stable `Regex` representation

## Syntax Design
The shape is an explicit constructor function and four core methods:

```opal
let compile_regex = f(pattern_text: string): Regex errors InvalidPattern =>
    # implementation detail
    return compiled

let match = f(self: Regex, input_text: string): boolean errors MatchError =>
    # implementation detail
    return did_match
```

The handle methods remain fallible for engine-level failures and malformed replacement templates. This keeps behavior explicit in signatures and aligns with existing error strategy proposals.

## Example Applications
Typical use cases include log scanning, routing, and tokenization where one pattern is reused across many inputs. A service can compile once during setup and then apply methods to each event without re-validating pattern syntax repeatedly.

Another common path is data cleanup: compile a whitespace or delimiter regex once, then call `.replace` and `.split` in different transformation stages while sharing exactly the same compiled semantics.

## Strengths
- Explicit compile step makes reuse obvious and intentional
- Strong performance profile for repeated matching
- Natural home for future method growth like `find_first`, `captures`, or options objects
- Works cleanly with explicit Opalescent error clauses

## Weaknesses
- Slightly more verbose for one-off matches
- Introduces object-like method surface that needs consistent stdlib naming guidance
- Requires users to manage a handle lifecycle, even if lightweight

## Impact on Existing Syntax
No parser changes are required if methods on value types are already supported as standard library conventions. Existing code keeps working because this adds a new module surface rather than altering language grammar.

The only migration pressure is stylistic: teams may choose compiled handles for repeated regex workloads to avoid module-function duplication.

## Interactions with Other Concerns
This model composes well with error strategy alternatives because `compile_regex` and each method can declare precise error enums. It also supports module organization concerns by allowing a clear `regex` namespace with a primary type and helper constructors.

For future deferred discussions, the handle shape can be retained while only adding deferred-capable input providers if needed, without redefining the core regex abstraction.

## Implementation Difficulty
Medium. Compiler work is minimal, but runtime and standard library integration are non-trivial: compiled program representation, method dispatch surface, replacement-template validation, and deterministic error mapping must be implemented carefully.

## Must NOT Have
- Implicit exceptions or hidden panic behavior
- Async-only variants in this proposal
- `_sync` suffix naming for CPU-bound regex methods
- Untyped or catch-all dynamic error returns
