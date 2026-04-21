# Split Math Into Modules

## Overview
This proposal splits mathematical operations into submodules based on their domain. It introduces `math/integer`, `math/floating_point`, and `math/bitwise` as distinct modules.

This approach organizes the standard library more cleanly and allows developers to import only the operations they need.

## Assumes
- Depends on the "Namespaced Stdlib" alternative for module organization.
- Support for hierarchical bare specifiers (e.g., `math/integer`).

## Syntax Design
Modules are imported using the `/` separator.

```op
import greatest_common_divisor from math/integer
import count_leading_zeros from math/bitwise
import floating_point_is_nan from math/floating_point
```

## Example Applications
### Integer Geometry
```op
import greatest_common_divisor from math/integer

let simplify_aspect_ratio = f(width: int64, height: int64): (int64, int64) =>
    let gcd = greatest_common_divisor(width, height)
    return (width / gcd, height / gcd)
```

### Bitwise Width
```op
import count_leading_zeros from math/bitwise

let bit_width = f(n: uint64): int64 =>
    let zeros = count_leading_zeros(n)
    return 64 - zeros
```

## Strengths
- **Logical Organization**: Functions are grouped by data type and operation category.
- **Granularity**: Smaller modules reduce potential name collisions.
- **Scalability**: New math categories (e.g., `math/complex`, `math/linear_algebra`) can be added without bloating the core `math` namespace.

## Weaknesses
- **Import Verbosity**: Developers must manage multiple imports if they need operations from multiple categories.
- **Complexity**: Slightly higher implementation cost for the module resolver.

## Impact on Existing Syntax
Requires the standard library to support nested module paths. Existing `import ... from math` would be deprecated or removed.

## Interactions with Other Concerns
Consistent with the Namespaced Stdlib proposal.

## Implementation Difficulty
Medium. Requires updates to the module resolution logic and the standard library structure.

## Must NOT Have
- Flat, top-level `math` module without nesting.
- Abbreviated names like `gcd` or `lcm`.
