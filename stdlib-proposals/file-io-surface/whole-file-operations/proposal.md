# Whole-File Operations
<!-- Provide a clear, descriptive name for this language alternative or feature proposal. -->

## Overview
This alternative proposes a high-level, atomic-first approach to file I/O in Opalescent. Instead of exposing low-level primitives like file handles or pointers, it provides functions that operate on entire files at once. 

This model is designed for simplicity and safety, preventing common handle-related bugs such as leaks, partial writes, or race conditions during multi-step operations. It is particularly suited for application-level development where reading a configuration file or writing a log entry is a frequent task.

## Assumes
This proposal assumes the existence of the `uint8[]` array type and the `string` primitive. It also assumes the standard Opalescent error handling model using `guard` and `propagate`. It depends on the `filesystem_errors.types.op` file for error definitions.

## Syntax Design
All functions are provided as top-level synchronization-explicit procedures.

```op
import FileNotFoundError, PermissionDeniedError, ReadFailureError from ./filesystem_errors.types

let read_file_to_bytes_sync = f(path: string): uint8[] errors 
    FileNotFoundError, 
    PermissionDeniedError, 
    ReadFailureError, 
    IsADirectoryError,
    InvalidPathError =>
    # ... implementation ...
```

The API includes:
- `read_file_to_bytes_sync(path: string): uint8[]`
- `read_file_to_string_sync(path: string): string`
- `write_bytes_to_file_sync(path: string, data: uint8[]): void`
- `write_string_to_file_sync(path: string, content: string): void`
- `append_bytes_to_file_sync(path: string, data: uint8[]): void`
- `delete_file_sync(path: string): void`
- `file_exists_sync(path: string): bool`
- `create_directory_sync(path: string): void`
- `list_directory_entries_sync(path: string): string[]`

## Example Applications
### Reading Config
```op
let load_config = f(path: string): string errors FileNotFoundError, PermissionDeniedError, ReadFailureError, IsADirectoryError, InvalidPathError =>
    guard read_file_to_string_sync(path) into content else err =>
        return propagate err
    return content
```

### Atomic Log Write
```op
let log_entry = f(path: string, entry: string): void errors 
    WriteFailureError, 
    PermissionDeniedError, 
    FilesystemFullError,
    InvalidPathError =>
    let temp_path = path + '.tmp'
    guard write_string_to_file_sync(temp_path, entry) into _success else err =>
        return propagate err
    # ... logic for rename would go here ...
    return void
```

## Strengths
- **Simplicity**: No need to manage file lifetimes or close handles.
- **Safety**: Eliminates resource leaks and reduces the chance of data corruption from interrupted writes.
- **Ergonomics**: Common tasks like "read this whole file" are single-line operations.
- **Atomicity**: Encourages atomic patterns (read everything, process, write everything).

## Weaknesses
- **Memory Pressure**: Large files must be loaded entirely into memory, which is inefficient for gigabyte-scale data.
- **Performance**: Lack of random access or partial updates makes modifying small parts of large files expensive.
- **Streaming**: Not suitable for log tailing or socket-like stream processing.

## Impact on Existing Syntax
This is a pure library addition. No changes to the language grammar or parser are required.

## Interactions with Other Concerns
- **Memory Model**: Aligns with Perceus GC as byte arrays are managed like any other heap object.
- **Error Handling**: Fits perfectly with the exhaustive `errors` clause and `guard` syntax.

## Implementation Difficulty
Low. This is primarily a wrapper around standard OS syscalls (read, write, stat).

## Must NOT Have
- This proposal must NOT have streaming or chunked reads.
- It must NOT have file handles or pointers.
- It must NOT have semicolons.
