# Module-per-Collection API

## Overview
This proposal organizes collection operations into specific bare-specifier modules: `array`, `map`, `set`, and `list`. This approach provides namespacing and prevents function name collisions while allowing for shorter, more intuitive operation names like `map` and `filter` when used within the module context.

## Assumes
This proposal assumes the `Absence via Errors` alternative from the `optional-representation` concern for handling cases where a collection search might return no result.

## Syntax Design
Functions are grouped into modules. Users import exactly what they need from the specific collection module.

```opal
import map, filter from array

let numbers: int32[] = [1, 2, 3, 4]
let evens = filter<int32>(numbers, f(n: int32): boolean => (n % 2) is 0)
let squares = map<int32, int32>(evens, f(n: int32): int32 => n * n)
```

## Example Applications
Grouping by collection type simplifies the mental model.

```opal
import map, filter from array
import set_from_array from set

let get_unique_squares = f(input: int32[]): Set<int32> =>
    let squares = map<int32, int32>(input, f(n: int32): int32 => n * n)
    return set_from_array<int32>(squares)
```

## Strengths
- **Namespace Isolation**: Prevents collisions between `array.map` and `map.map`.
- **Succinctness**: Functions can have shorter names like `map`, `filter`, `reduce`.
- **Modularity**: Users only load the code for the collections they actually use.

## Weaknesses
- **Import Boilerplate**: Requires multiple import statements when working with multiple collection types.
- **Qualified Names**: If used without aliasing, can lead to repetitive `array.map` calls.
- **Discoverability**: Relies on knowing which module contains which function.

## Impact on Existing Syntax
No impact on existing syntax. It utilizes the existing module and import system. It requires standardizing the module names `array`, `list`, `set`, `map` as bare specifiers.

## Interactions with Other Concerns
Consistent with the existing module system. Does not require any special compiler support beyond standard module resolution.

## Implementation Difficulty
Low. Primarily involves organizing the standard library into the proposed module structure.

## Must NOT Have
- **Global Pollution**: Should not export collection functions to the global namespace.
- **Inconsistent Naming**: Functions should have consistent names across different collection modules.
- **Deep Nesting**: Modules should be flat (e.g., `array`, not `collections/array`).
