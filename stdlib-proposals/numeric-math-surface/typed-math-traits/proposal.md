# Typed Math Traits

## Overview
This proposal organizes mathematical operations as type-associated functions. Instead of free-floating functions in a module, math helpers are accessed via their target type (e.g., `int64.greatest_common_divisor(a, b)`).

This approach leverages the type system to provide a more object-oriented-like discoverability while maintaining the language's functional nature.

## Assumes
- Support for type-associated functions or "traits" in the compiler.
- Dispatch mechanism to resolve `type.function` calls.

## Syntax Design
Functions are called by prefixing them with the type name.

```op
let gcd = int64.greatest_common_divisor(48, 18)
let clz = uint32.count_leading_zeros(n)
let is_nan = float64.floating_point_is_nan(f)
```

## Example Applications
### Type-Specific Reduction
```op
let reduce_fraction = f(num: int64, den: int64): (int64, int64) =>
    let gcd = int64.greatest_common_divisor(num, den)
    return (num / gcd, den / gcd)
```

### Generic Summation
```op
let sum_xs = f(xs: int64[]): int64 errors IntegerOverflow =>
    let mutable total: int64 = 0
    for x in xs:
        guard int64.checked_integer_addition(total, x) into total_next else err =>
            return err
        total = total_next
    return total
```

## Strengths
- **Discoverability**: LSP completions on types reveal relevant mathematical operations.
- **Precision**: Operations are tied directly to the types they operate on, reducing ambiguity.
- **No Import Noise**: Mathematical operations don't need to be imported individually.

## Weaknesses
- **Verbosity**: Calling `int64.checked_integer_addition` is more verbose than `checked_integer_addition`.
- **Implementation Effort**: Requires a robust trait or associated function mechanism in the compiler.

## Impact on Existing Syntax
None. This adds a new way to access functions via types. Existing module-based imports could coexist but might become redundant.

## Interactions with Other Concerns
Interacts heavily with the type system and dispatch logic. Makes generic programming more powerful by allowing traits to be defined for numeric types.

## Implementation Difficulty
High. Requires significant compiler work to support type-associated functions and dispatch.

## Must NOT Have
- Global, free-floating math functions without type association.
- Abbreviated names like `gcd` or `lcm`.
