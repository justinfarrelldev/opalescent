# Expand Math Module

## Overview
This proposal expands the existing `math` bare specifier to include a comprehensive set of numeric operations. It maintains the current flat module structure while adding missing integer, floating-point, and bitwise helpers.

The goal is to provide a single, well-known location for all common mathematical operations without introducing new module-resolution complexity.

## Assumes
- The existence of the `math` bare specifier.
- Support for custom error types `IntegerOverflow` and `NegativeInputError`.

## Syntax Design
All functions are accessed via the `math` module. No new syntax or keywords are required.

```op
import greatest_common_divisor from math

let result = greatest_common_divisor(48, 18)
```

## Example Applications
### Fraction Reduction
```op
import greatest_common_divisor from math

let reduce_fraction = f(numerator: int64, denominator: int64): (int64, int64) =>
    let divisor = greatest_common_divisor(numerator, denominator)
    return (numerator / divisor, denominator / divisor)
```

### Overflow-Safe Summation
```op
import checked_integer_addition from math

let safe_sum = f(xs: int64[]): int64 errors IntegerOverflow =>
    let mutable total: int64 = 0
    for x in xs:
        total = propagate checked_integer_addition(total, x)
    return total
```

## Strengths
- **Simplicity**: No changes to module organization or syntax.
- **Discoverability**: All math functions reside in a single, obvious location.
- **Low implementation effort**: Only involves adding functions to the existing `math` module.

## Weaknesses
- **Namespace Pollution**: The `math` module becomes large, containing unrelated integer, float, and bitwise operations.
- **Granularity**: Importing from `math` pulls in the entire module, even if only a single bitwise operation is needed.

## Impact on Existing Syntax
None. This is a purely additive change to the standard library.

## Interactions with Other Concerns
Composes well with the standard error model using `propagate` and `guard`.

## Implementation Difficulty
Low. Requires implementing the functions as standard library intrinsics or Opalescent code.

## Must NOT Have
- Abbreviated names (e.g., `gcd`, `lcm`).
- Floating-point specific functions in the global scope without clear naming.
