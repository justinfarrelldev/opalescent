# Registered Error Hierarchy

## Overview
The Registered Error Hierarchy strategy defines a central `error_types` module that all other modules import from. This approach creates a single source of truth for all errors in the standard library and ensures consistency across different modules.

By registering errors centrally, the standard library can define shared error categories (e.g., `FileSystemError`, `NetworkError`) that multiple modules can use.

## Assumes
This proposal assumes that the standard library can have a shared `error_types` module and that the type system supports importing these types across different modules.

## Syntax Design
No new language keywords are required. The proposal focus is on a convention for the standard library structure. All standard library modules import their errors from a common module:

```opal
import IoError, FileNotFoundError from standard/error_types

let read_file_sync = f(path: string): string errors IoError, FileNotFoundError =>
    # Implementation
    return "content"
```

The hierarchy is achieved through documentation and organization in the `error_types` module, rather than through complex type inheritance.

## Example Applications
A file system module using registered errors:

```opal
import FileSystemError, AccessDenied from standard/error_types

let delete_file_sync = f(path: string): void errors FileSystemError, AccessDenied =>
    # ...
    return void
```

A database module using shared registered errors:

```opal
import ConnectionError, Timeout from standard/error_types

let connect_sync = f(connection_string: string): void errors ConnectionError, Timeout =>
    # ...
    return void
```

## Strengths
- **Consistency**: All standard library errors follow a uniform structure.
- **Interoperability**: Different modules can share error types, making it easier for callers to handle common failure modes.
- **Discoverability**: A single module provides a clear catalog of all possible errors.
- **Predictability**: Callers know exactly where to look for error definitions.

## Weaknesses
- **Centralization**: Adding new errors requires modifying the central `error_types` module, which can be a bottleneck for parallel development.
- **Tight Coupling**: Many modules depend on the central `error_types` module.
- **Namespace Pressure**: A single module containing all errors may become cluttered without careful sub-module organization.

## Impact on Existing Syntax
This is a convention-based proposal and does not change any core Opalescent syntax. It adds organization to the standard library.

## Interactions with Other Concerns
- **LSP**: The LSP can easily find all error definitions in one central place, providing excellent documentation and autocompletion.
- **Hot Reloading**: Changes to the central `error_types` module might trigger more rebuilds than decentralized errors.

## Implementation Difficulty
Low. This is mostly an architectural decision for the standard library's structure.

## Must NOT Have
- **Complexity**: No complex inheritance hierarchies; just a well-organized set of registered error types.
- **Implicit Knowledge**: All errors used in a function must still be explicitly listed in its `errors` clause.
- **Dynamic Registry**: Errors are statically defined at compile time, not registered at runtime.
