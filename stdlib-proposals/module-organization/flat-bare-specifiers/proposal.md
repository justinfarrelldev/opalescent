# Flat Bare Specifiers

## Overview
This proposal expands the current model where the standard library is accessible via flat, top-level bare specifiers. In this model, every major functional area of the standard library is given its own unique name that can be imported directly without any prefix or path.

The core idea is to maintain the simplicity of the current `standard` and `math` specifiers by extending them to cover the entire standard library surface. This avoids hierarchical paths and keeps import statements concise.

## Assumes
- The compiler/linker can resolve bare specifiers to specific internal locations.
- No third-party package or local file can use a bare specifier name that conflicts with the standard library.

## Syntax Design
Imports continue to use the existing `import ... from <specifier>` syntax. Every standard library module is a first-class bare specifier.

```opal
import sqrt from math
import log, Info from logging
import sha256 from crypto
import read_file_sync from filesystem
```

The list of reserved bare specifiers includes: `standard`, `math`, `bytes`, `strings`, `time`, `random`, `filesystem`, `network`, `serialization`, `crypto`, `compression`, `logging`, `regex`, `uuid`, `subprocess`, `testing`.

## Example Applications
A typical application might import from multiple flat modules:

```opal
import print from standard
import now from time
import random_int32 from random

let main = f(): void =>
    let current_time = now()
    print(current_time)
    return void
```

## Strengths
- **Simplicity**: No need to remember which module belongs to which namespace.
- **Conciseness**: Import statements are short and readable.
- **Consistency**: Extends the existing pattern used for `standard` and `math`.
- **Discovery**: Flat namespace makes it easy to list all available standard modules.

## Weaknesses
- **Namespace Pollution**: Consumes many top-level names, potentially conflicting with common variable names if not careful (though `import` syntax helps).
- **Scalability**: As the standard library grows, the number of bare specifiers becomes large and harder to manage.
- **Ambiguity**: Without a `standard/` prefix, it might not be immediately obvious that a module is part of the stdlib vs a third-party package (though packages use `@`).

## Impact on Existing Syntax
This is a non-breaking expansion of the current syntax. It codifies the existing behavior and adds more names to the reserved list of bare specifiers.

## Interactions with Other Concerns
- **LSP**: The LSP can easily provide completions for all top-level bare specifiers.
- **Error Handling**: Standard library modules will continue to use explicit `errors` clauses, which are handled normally via `guard` or `propagate`.

## Implementation Difficulty
Low. It requires updating the module resolution logic to recognize a larger set of strings as standard library identifiers.

## Must NOT Have
- Hierarchical paths (e.g., `standard/math`).
- Aliasing of bare specifiers to anything other than the standard library.
- Dynamically loaded bare specifiers.
