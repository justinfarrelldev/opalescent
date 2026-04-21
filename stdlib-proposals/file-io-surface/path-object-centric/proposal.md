# Path-Object-Centric File I/O
<!-- Provide a clear, descriptive name for this language alternative or feature proposal. -->

## Overview
This alternative proposes an object-oriented style for file I/O where operations are centered around a `FilesystemPath` type. Instead of passing raw strings to functions, developers use a dedicated path object that provides both path manipulation and I/O capabilities.

This model separates CPU-only path arithmetic from fallible, I/O-intensive operations. It improves type safety by ensuring that path manipulation is handled by a dedicated type, reducing errors related to string concatenation and path delimiters.

## Assumes
This proposal assumes the existence of the `uint8[]` array type and the `string` primitive. It depends on the `filesystem_errors.types.op` file for error definitions.

## Syntax Design
The API introduces the `FilesystemPath` type and associated functions.

```op

# Path manipulation helpers (CPU-only, no _sync)
let join_path_components = f(base: FilesystemPath, component: string): FilesystemPath =>
    return new FilesystemPath:
        raw_path: base.raw_path + '/' + component

let path_parent_directory = f(path: FilesystemPath): FilesystemPath =>
    # ... logic ...
    return path

let path_file_name = f(path: FilesystemPath): string =>
    return 'filename'

let path_file_extension = f(path: FilesystemPath): string =>
    return 'op'

# I/O operations (Filesystem-touching, with _sync)
let read_contents_sync = f(path: FilesystemPath): uint8[] errors 
    FileNotFoundError, 
    PermissionDeniedError, 
    ReadFailureError, 
    IsADirectoryError,
    InvalidPathError =>
    # ... implementation ...

let write_contents_sync = f(path: FilesystemPath, data: uint8[]): void errors 
    WriteFailureError,
    PermissionDeniedError,
    FilesystemFullError,
    InvalidPathError =>
    # ... implementation ...
```

## Example Applications
### Reading Config
```op
let load_config = f(path: FilesystemPath): uint8[] errors 
    FileNotFoundError, 
    PermissionDeniedError, 
    ReadFailureError, 
    IsADirectoryError,
    InvalidPathError =>
    
    guard read_contents_sync(path) into content else err =>
        return propagate err
    return content
```

### Atomic Log Write
```op
let log_entry = f(path: FilesystemPath, data: uint8[]): void errors 
    WriteFailureError, 
    PermissionDeniedError, 
    FilesystemFullError, 
    InvalidPathError =>
    
    let temp_path = join_path_components(path_parent_directory(path), path_file_name(path) + '.tmp')
    
    guard write_contents_sync(temp_path, data) into _success else err =>
        return propagate err
        
    return void
```

## Strengths
- **Type Safety**: Distinguishes between generic strings and filesystem paths.
- **Clarity**: Explicitly separates path manipulation from filesystem interaction.
- **Cross-Platform**: Path manipulation helpers can handle OS-specific delimiters internally.
- **Discoverability**: Functions are logically grouped around the `FilesystemPath` type.

## Weaknesses
- **Verbosity**: Requires wrapping strings in `FilesystemPath` objects before use.
- **Object Overhead**: Small overhead for the path wrapper.
- **Simplicity vs Handle**: Still uses a "whole-file" reading model, lacking the efficiency of handles for large files.

## Impact on Existing Syntax
This is a pure library addition. No changes to the language grammar or parser are required.

## Interactions with Other Concerns
- **Error Handling**: Perfectly compatible with the Opalescent error model.
- **LSP**: Enables better completion for path-related operations when the type is known.

## Implementation Difficulty
Low to Medium. Requires implementing robust path manipulation logic.

## Must NOT Have
- This proposal must NOT have `_sync` on path manipulation helpers.
- It must NOT have semicolons.
- It must NOT use `Result<T,E>` for error handling.
