# Regex Module Functions

## Overview
This alternative keeps regex APIs as flat module-level functions such as `regex_match(pattern, input)`, `regex_find_all(pattern, input)`, `regex_replace(pattern, input, replacement)`, and `regex_split(pattern, input)`.

The core idea is immediate usability: callers can perform regex work with no explicit compile step and no intermediate handle type. This favors short scripts and one-off operations where setup overhead would dominate readability.

## Assumes
- Opalescent standard library can expose pure top-level functions with explicit errors
- Runtime can compile patterns internally per call or with hidden cache policy
- Regex remains CPU-bound and synchronous for current scope
- Error handling remains guard/propagate without exceptions

## Syntax Design
The syntax is direct and function-centric:

```opal
let regex_match = f(pattern_text: string, input_text: string): boolean errors InvalidPattern, MatchError =>
    # implementation detail
    return did_match

let regex_find_all = f(pattern_text: string, input_text: string): string[] errors InvalidPattern, MatchError =>
    # implementation detail
    return values
```

Each call takes pattern text directly, which keeps call sites concise but can repeat parse overhead when called inside loops.

## Example Applications
This design is ideal for validation checks in command-line tooling, where each pattern appears once and code should remain compact. It also works well for migration utilities that process fields with small, isolated regex operations.

Another common case is tests and prototyping: developers can quickly assert matching behavior without building additional setup objects.

## Strengths
- Minimal ceremony and very short call sites
- Easy discoverability from auto-complete on module functions
- Lowest implementation complexity among alternatives
- Strong fit for one-off or low-frequency regex usage

## Weaknesses
- Repeated calls with identical patterns can waste compile effort
- Harder to communicate reuse intent in source code
- Later advanced options may force parameter bloat

## Impact on Existing Syntax
No language syntax changes are needed. This proposal is a pure standard-library API design choice and can be introduced incrementally.

Existing projects can adopt it file by file because usage is stateless and does not require introducing new types.

## Interactions with Other Concerns
Error handling remains straightforward because each function explicitly lists `InvalidPattern` and operation-specific errors. Module organization is simple: one `regex` module can host all function entry points and companion types.

If future proposals introduce richer typed captures, this alternative may need additive overloads or new function families to avoid breaking existing simple signatures.

## Implementation Difficulty
Low. Most work is in runtime bindings and deterministic error translation. There is little checker complexity because no long-lived regex value needs method typing.

## Must NOT Have
- Hidden exception channels for invalid patterns
- Async API additions in this concern scope
- `_sync` suffixes for pure CPU behavior
- Implicit global mutable state required by callers
