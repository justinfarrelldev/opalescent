# Namespaced Standard Library

## Overview
This proposal introduces a single bare specifier, `standard`, which acts as the root for all standard library modules. Sub-modules are accessed using a hierarchical path syntax, such as `standard/math`, `standard/regex`, etc.

The core idea is to group all standard library functionality under a common namespace to clearly distinguish it from local files and third-party packages. This reduces top-level namespace pollution and provides a logical structure for organization.

## Assumes
- The module resolver can handle forward slashes in bare specifiers to resolve sub-paths.
- No third-party package or local file can use the `standard` name at the root of a path.

## Syntax Design
Imports use the hierarchical path within the `from` clause. The leading part of the specifier is always `standard`.

```opal
import sqrt from standard/math
import regex from standard/regex
import read_file_sync from standard/filesystem
import sha256_sync from standard/crypto
```

## Example Applications
A typical application might import several modules from the same `standard` namespace:

```opal
import print from standard
import now_sync from standard/time
import random_int32 from standard/random

let main = f(): void =>
    let current_time = now_sync()
    print(current_time)
    return void
```

## Strengths
- **Namespace Cleanliness**: Only one top-level name is reserved for the entire standard library.
- **Explicit Hierarchy**: Clear distinction between standard library and other modules.
- **Grouping**: Logical grouping of related functionality (e.g., all crypto related stuff could be under `standard/crypto/...`).
- **Safety**: Reduces the risk of accidental name collisions with local variables or modules.

## Weaknesses
- **Verbosity**: Every import from the standard library requires the `standard/` prefix.
- **Consistency**: Slightly different from the existing `import ... from math` pattern if it were to be replaced.
- **Path-like Syntax**: Might be confused with local relative paths, although the absence of `./` or `../` distinguishes it.

## Impact on Existing Syntax
This would require changing the existing `import ... from math` to `import ... from standard/math`. The `import ... from standard` would remain the same for the core prelude.

## Interactions with Other Concerns
- **LSP**: The LSP can provide nested completions when a user types `standard/`.
- **Error Handling**: Standard sub-modules follow the same error handling conventions.

## Implementation Difficulty
Medium. The module resolver needs to be updated to parse and resolve hierarchical bare specifiers.

## Must NOT Have
- Deep nesting that makes imports unreadable.
- Use of the same specifier for both a module and a namespace.
- Confusion with package imports (which use `@`).
