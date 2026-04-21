# Tiered Standard Library

## Overview
This proposal organizes the standard library into three distinct tiers based on their expected use and impact on the binary: `core`, `standard`, and `standard_extra`.

The core idea is to provide a tiered approach to importing standard functionality. `core` contains functions that are always available, `standard` provides the common library surface as opt-in bare specifiers, and `standard_extra` provides less common or more platform-dependent functionality that might require specific build flags to enable.

## Assumes
- The build system can selectively include `standard_extra` based on configuration.
- The language supports a "prelude" (core) that is always imported.

## Syntax Design
Imports from the three tiers are distinguished by their bare specifier name:

```opal
# core: Always imported (like print)
import print from core

# standard: Typical opt-in modules
import sqrt from math
import read_file_sync from filesystem

# standard_extra: Specialized/platform-dependent modules
import sha512_sync from crypto_extra
import bzip2_sync from compression_extra
```

## Example Applications
A typical application might use modules from all three tiers:

```opal
import print from core
import random_int32 from random
import compress_sync from compression_extra

let main = f(): void =>
    let random_val = random_int32(1, 100)
    print(random_val)
    return void
```

## Strengths
- **Binary Size Control**: Only the code from the imported tiers and modules is included in the final binary.
- **Clear Separation**: Distinguishes between fundamental functions and more advanced/specialized ones.
- **Extensibility**: New, experimental, or heavy modules can be added to the `standard_extra` tier without bloating the core.

## Weaknesses
- **Complexity**: Developers need to know which tier a module belongs to.
- **Potential Confusion**: Distinguishing between `crypto` and `crypto_extra` might be frustrating.
- **Build Flags**: Some tiers might require extra steps to enable, adding overhead to the development process.

## Impact on Existing Syntax
This would refine the current model by categorizing existing functions and modules into the three tiers. The `core` tier would formalize the implicit prelude.

## Interactions with Other Concerns
- **Build System**: The build system must be aware of the tiers to correctly link and optimize.
- **Error Handling**: Tiered modules continue to use standard error handling.

## Implementation Difficulty
High. It requires both compiler support for tiered resolution and build system integration for optional tiers.

## Must NOT Have
- Circular dependencies between tiers.
- Overlapping functionality between `standard` and `standard_extra` without clear reasoning.
- Too many tiers that become hard to manage.
