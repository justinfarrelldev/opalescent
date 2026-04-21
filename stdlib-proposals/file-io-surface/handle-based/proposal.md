# Handle-Based File I/O
<!-- Provide a clear, descriptive name for this language alternative or feature proposal. -->

## Overview
This alternative proposes a classic handle-based approach to file I/O, similar to systems programming languages like C or Rust. It introduces an opaque `FileHandle` type that represents an open file resource.

By using handles, the language allows for fine-grained control over file access, including random access (seeking) and incremental reads/writes. This model is highly efficient for processing large files that cannot or should not be loaded entirely into memory.

## Assumes
This proposal assumes the existence of the `uint8[]` array type and the `string` primitive. It depends on the `filesystem_errors.types.op` file for error definitions. It also assumes the ability to define opaque types in the standard library.

## Syntax Design
The API centers around the `FileHandle` type and functions that operate on it.

```op

let open_file_sync = f(path: string, mode: FileMode): FileHandle errors 
    FileNotFoundError, 
    PermissionDeniedError, 
    InvalidPathError,
    IsADirectoryError =>
    # ... implementation ...

let read_from_file_handle_sync = f(handle: FileHandle, count: int32): uint8[] errors 
    ReadFailureError =>
    # ... implementation ...

let write_to_file_handle_sync = f(handle: FileHandle, data: uint8[]): void errors 
    WriteFailureError,
    FilesystemFullError =>
    # ... implementation ...

let close_file_handle_sync = f(handle: FileHandle): void =>
    # ... implementation ...
```

Supported modes include `Read`, `Write`, `Append`.

## Example Applications
### Reading Config
```op
let load_config = f(path: string): uint8[] errors 
    FileNotFoundError, 
    PermissionDeniedError, 
    ReadFailureError, 
    InvalidPathError,
    IsADirectoryError =>
    
    guard open_file_sync(path, FileMode.Read) into handle else err =>
        return propagate err
        
    let content = propagate read_from_file_handle_sync(handle, 4096)
    close_file_handle_sync(handle)
    return content
```

### Atomic Log Write
```op
let log_entry = f(path: string, entry: uint8[]): void errors 
    WriteFailureError, 
    PermissionDeniedError, 
    FilesystemFullError, 
    InvalidPathError =>
    
    let temp_path = path + '.tmp'
    guard open_file_sync(temp_path, FileMode.Write) into handle else err =>
        return propagate err
        
    guard write_to_file_handle_sync(handle, entry) into _success else err =>
        close_file_handle_sync(handle)
        return propagate err
        
    close_file_handle_sync(handle)
    # rename logic would follow
    return void
```

## Strengths
- **Efficiency**: Allows processing large files in chunks, minimizing memory usage.
- **Flexibility**: Supports random access and various open modes (read-only, write-only, etc.).
- **Performance**: Minimizes syscall overhead for repeated small operations on the same file.
- **Familiarity**: Maps directly to standard POSIX file operations.

## Weaknesses
- **Resource Management**: Developers must remember to close handles, leading to potential leaks.
- **Safety**: Errors during incremental operations can leave files in inconsistent states.
- **Boilerplate**: Requires more lines of code for simple "read whole file" tasks.

## Impact on Existing Syntax
This is a pure library addition. No changes to the language grammar or parser are required.

## Interactions with Other Concerns
- **Memory Model**: File handles are external resources; the compiler must ensure they are closed properly, possibly via the Perceus GC finalize mechanism.
- **Error Handling**: Fits well with `guard` for handling failures at each step (open, read, write).

## Implementation Difficulty
Medium. Requires managing a table of active file descriptors and ensuring correct mapping to OS-specific syscalls.

## Must NOT Have
- This proposal must NOT have high-level "whole-file" functions.
- It must NOT have semicolons.
- It must NOT use `Result<T,E>` for error handling.
