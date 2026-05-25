# Opalescent Standard Library

This is the user-facing reference for functions imported from `standard` plus the runtime helpers that the compiler may lower to internally. Every public entry below includes what the function does, not just its name.

The authoritative implementation is split across:

- `src/type_system/module_resolver/` for language-level signatures and error types.
- `src/codegen/functions_stdlib.rs` for runtime symbol declarations and import resolution.
- `runtime/*.c` for many generated-program runtime helpers.
- `stdlib/prelude.op` for documentation-oriented signatures.

Names ending in `_sync` are blocking operations. If a signature has `errors ...`, call it with `propagate` or `guard`.

```opal
import path_from, read_text_sync, write_text_sync, string_join from standard

entry main = f(args: string[]): void errors FileNotFoundError, PermissionDeniedError, ReadFailureError, IsADirectoryError, InvalidPathError, InvalidUtf8Error =>
    let path = path_from('README.md')
    let text = propagate read_text_sync(path)
    print(text)
    return void
```

## Console I/O

### `print(value): void`

Prints a displayable value. The surface is generic: strings, booleans, integers, floats, and other supported display values are lowered to the appropriate runtime printing path.

```opal
print('hello')
print(42)
print(true)
```

### `println(text: string): void`

Prints one string line. This is registered as a standard symbol for string-only line output.

```opal
println('hello')
```

### `take_input(): string`

Reads one line from standard input and returns it without the trailing newline. On EOF, the runtime returns an empty string.

```opal
let answer = take_input()
print('you typed {answer}')
```

## Parsing strings into numbers

These parse decimal text into the requested numeric type. They skip leading whitespace, require the whole trimmed input to be valid, and fail with `ParseError` for invalid digits, empty input, or values outside the target range.

| Function | What it returns | Description |
|---|---|---|
| `string_to_int8(text: string): int8 errors ParseError` | `int8` | Parses a signed 8-bit integer. |
| `string_to_int16(text: string): int16 errors ParseError` | `int16` | Parses a signed 16-bit integer. |
| `string_to_int32(text: string): int32 errors ParseError` | `int32` | Parses a signed 32-bit integer. |
| `string_to_int64(text: string): int64 errors ParseError` | `int64` | Parses a signed 64-bit integer. |
| `string_to_uint8(text: string): uint8 errors ParseError` | `uint8` | Parses an unsigned 8-bit integer. |
| `string_to_uint16(text: string): uint16 errors ParseError` | `uint16` | Parses an unsigned 16-bit integer. |
| `string_to_uint32(text: string): uint32 errors ParseError` | `uint32` | Parses an unsigned 32-bit integer. |
| `string_to_uint64(text: string): uint64 errors ParseError` | `uint64` | Parses an unsigned 64-bit integer. |
| `string_to_float32(text: string): float32 errors ParseError` | `float32` | Parses a 32-bit floating-point value. |
| `string_to_float64(text: string): float64 errors ParseError` | `float64` | Parses a 64-bit floating-point value. |

```opal
let n = propagate string_to_int32('123')
```

## Converting values to strings

These allocate and return a decimal or boolean string representation of the input value. They do not declare errors.

| Function | Description |
|---|---|
| `int8_to_string(value: int8): string` | Converts a signed 8-bit integer to decimal text. |
| `int16_to_string(value: int16): string` | Converts a signed 16-bit integer to decimal text. |
| `int32_to_string(value: int32): string` | Converts a signed 32-bit integer to decimal text. |
| `int64_to_string(value: int64): string` | Converts a signed 64-bit integer to decimal text. |
| `uint8_to_string(value: uint8): string` | Converts an unsigned 8-bit integer to decimal text. |
| `uint16_to_string(value: uint16): string` | Converts an unsigned 16-bit integer to decimal text. |
| `uint32_to_string(value: uint32): string` | Converts an unsigned 32-bit integer to decimal text. |
| `uint64_to_string(value: uint64): string` | Converts an unsigned 64-bit integer to decimal text. |
| `float32_to_string(value: float32): string` | Converts a 32-bit float to compact decimal text. |
| `float64_to_string(value: float64): string` | Converts a 64-bit float to compact decimal text. |
| `bool_to_string(value: boolean): string` | Returns `true` or `false`. |

Example from the filesystem roundtrip fixture:

```opal
print('roundtrip: ok ({int64_to_string(actual.length)} bytes match)')
```

## Strings

### `string_length(text: string): int64`

Returns the number of Unicode scalar values in `text`. Source code usually uses the `.length` member syntax, which lowers to this helper.

```opal
let text = 'hello'
print('length: {text.length}')
```

### `string_join(parts: string[], separator: string): string`

Returns one string made by placing `separator` between each element of `parts`. Use it for line rendering and simple accumulation.

```opal
let lines: string[] = ['a', 'b', 'c']
let text = string_join(lines, '\n')
```

### `string_builder_new(): StringBuilder`

Creates an empty string builder. A builder is useful when repeated string concatenation would be noisy or inefficient.

### `string_builder_push(builder: StringBuilder, text: string): void errors BuilderFinishedError, AllocationFailureError`

Appends `text` to `builder`. It fails if the builder has already been finished or if the runtime cannot allocate storage for the appended text.

### `string_builder_finish(builder: StringBuilder): string errors BuilderFinishedError, AllocationFailureError`

Finishes the builder and returns the accumulated string. Calling push or finish again after finishing is an error.

## Random integer helpers

These return a pseudo-random value in the requested inclusive range. The type-specific runtime helpers exist for code generation; the language-facing surface is still being refined, so prefer examples already present in tests.

| Function | Description |
|---|---|
| `random_int8(min: int8, max: int8): int8` | Returns a pseudo-random signed 8-bit integer between `min` and `max`. |
| `random_int16(min: int16, max: int16): int16` | Returns a pseudo-random signed 16-bit integer between `min` and `max`. |
| `random_int32(min: int32, max: int32): int32` | Returns a pseudo-random signed 32-bit integer between `min` and `max`. |
| `random_int64(min: int64, max: int64): int64` | Returns a pseudo-random signed 64-bit integer between `min` and `max`. |
| `random_uint8(min: uint8, max: uint8): uint8` | Returns a pseudo-random unsigned 8-bit integer between `min` and `max`. |
| `random_uint16(min: uint16, max: uint16): uint16` | Returns a pseudo-random unsigned 16-bit integer between `min` and `max`. |
| `random_uint32(min: uint32, max: uint32): uint32` | Returns a pseudo-random unsigned 32-bit integer between `min` and `max`. |
| `random_uint64(min: uint64, max: uint64): uint64` | Returns a pseudo-random unsigned 64-bit integer between `min` and `max`. |

```opal
let roll = random_int32(1, 6)
print('roll: {roll}')
```

## Arrays and collections

Most users should prefer source-level array syntax and member operations. These helpers exist for lowering and library-style array construction.

### `.length` / `array_length`

`values.length` returns the number of elements in an array. Internally, the compiler may lower this through `array_length`.

```opal
let values: int32[] = [1, 2, 3]
print(values.length)
```

### `array_filled<T>(count: int64, value: T): T[]`

Creates an array of `count` elements, each initialized to `value`.

### `reserve<T>(array: T[], capacity: int64): T[]`

Returns an array with capacity reserved for at least `capacity` elements. This is a low-level helper for avoiding repeated reallocations.

### `clear<T>(array: T[]): T[]`

Returns an empty array of the same element type.

### Member operations

Arrays also support member-style operations such as:

```opal
let mutable values: int32[] = []
values.push(1)
values.push(2)
let last = values.pop()
```

Implemented/tested fixture areas include `array-map`, `array-filter`, `array-reduce`, `array-zip`, and `array-pair` under `test-projects/`.

## Bytes

`Bytes` is an opaque immutable byte buffer for binary data.

### `bytes_new(): Bytes`

Returns an empty `Bytes` value. Newer examples usually prefer propertyless construction:

```opal
let buffer: Bytes = new Bytes
```

### `bytes_length(buffer: Bytes): int32`

Returns the number of bytes in `buffer`. Source usually uses `buffer.length`, which lowers to this helper.

### `bytes_to_hex(buffer: Bytes): string`

Encodes the buffer as lowercase hexadecimal text.

### `bytes_from_hex(text: string): Bytes errors HexDecodeError`

Decodes hexadecimal text into bytes. It accepts uppercase and lowercase hex digits and fails for odd-length input or non-hex characters.

### `bytes_concatenate(left: Bytes, right: Bytes): Bytes`

Returns a new buffer containing `left` followed by `right`.

### `bytes_slice(source: Bytes, start: int32, end: int32): Bytes errors SliceRangeError`

Returns bytes in the half-open range `[start, end)`. It fails when the range is inverted or outside the buffer.

```opal
guard bytes_from_hex('deadbeef') into data else err =>
    print(err)
    propagate err

let rendered = bytes_to_hex(data)
print(rendered)
```

## Filesystem path helpers

Filesystem APIs use `FilesystemPath` rather than plain strings for most operations.

### `path_from(raw: string): FilesystemPath`

Wraps a raw string as a filesystem path.

### `join_path_components(base: FilesystemPath, components: string[]): FilesystemPath`

Joins a base path with one or more child components and normalizes separators.

### `path_parent_directory(path: FilesystemPath): FilesystemPath`

Returns the parent directory of `path`.

### `path_file_name(path: FilesystemPath): string`

Returns the final path component, or an empty string when there is no file name.

### `path_file_extension(path: FilesystemPath): string`

Returns the text after the final dot in the file name, or an empty string if no extension exists.

### `normalize_path(path: FilesystemPath): FilesystemPath`

Normalizes path syntax by removing redundant separators and resolving simple `.` and `..` segments where possible.

### `path_to_string(path: FilesystemPath): string`

Returns the string representation of a filesystem path.

### `absolute_path_sync(path: FilesystemPath): FilesystemPath errors InvalidPathError, PermissionDeniedError`

Resolves `path` to an absolute path using the host filesystem. It can fail when the path is invalid or inaccessible.

```opal
let root = path_from('test-projects')
let project = join_path_components(root, ['hello-world', 'src', 'main.op'])
print(path_to_string(project))
```

## Reading files

### `read_contents_sync(path: FilesystemPath): Bytes errors FileNotFoundError, PermissionDeniedError, ReadFailureError, IsADirectoryError, InvalidPathError`

Reads the whole file as raw bytes.

### `read_text_sync(path: FilesystemPath): string errors FileNotFoundError, PermissionDeniedError, ReadFailureError, IsADirectoryError, InvalidPathError, InvalidUtf8Error`

Reads the whole file as UTF-8 text. It fails with `InvalidUtf8Error` if the bytes are not valid UTF-8.

### `read_first_line_sync(path: FilesystemPath): string errors FileNotFoundError, PermissionDeniedError, IsADirectoryError, InvalidUtf8Error, OffsetOutOfRangeError, ReadFailureError`

Reads and returns the first line of a text file.

### `read_lines_sync(path: FilesystemPath): string[] errors FileNotFoundError, PermissionDeniedError, ReadFailureError, IsADirectoryError, InvalidPathError, InvalidUtf8Error`

Reads a UTF-8 text file and returns its lines as a string array.

### `read_bytes_at_offset_sync(path: FilesystemPath, offset: int64, count: int64): Bytes errors FileNotFoundError, PermissionDeniedError, ReadFailureError, OffsetOutOfRangeError, InvalidPathError`

Reads `count` bytes beginning at `offset`. It fails if the range is outside the file.

Beginner rule: use `read_text_sync` or `read_lines_sync` for human text, and `read_contents_sync` for binary data.

## Writing files

### `write_contents_sync(path: FilesystemPath, content: Bytes): void errors PermissionDeniedError, WriteFailureError, IsADirectoryError, InvalidPathError, FilesystemFullError`

Overwrites a file with raw bytes, creating it when the platform permits.

### `write_text_sync(path: FilesystemPath, text: string): void errors PermissionDeniedError, WriteFailureError, IsADirectoryError, InvalidPathError, FilesystemFullError`

Overwrites a file with UTF-8 text.

### `write_contents_atomic_sync(path: FilesystemPath, content: Bytes): void errors PermissionDeniedError, WriteFailureError, IsADirectoryError, InvalidPathError, FilesystemFullError`

Writes bytes through a temporary file and replaces the target, reducing the chance of leaving partial output.

### `write_text_atomic_sync(path: FilesystemPath, text: string): void errors PermissionDeniedError, WriteFailureError, IsADirectoryError, InvalidPathError, FilesystemFullError`

Text version of the atomic write operation.

### `append_contents_sync(path: FilesystemPath, content: Bytes): void errors FileNotFoundError, PermissionDeniedError, WriteFailureError, IsADirectoryError, InvalidPathError, FilesystemFullError`

Appends raw bytes to an existing file.

### `append_text_sync(path: FilesystemPath, text: string): void errors FileNotFoundError, PermissionDeniedError, WriteFailureError, IsADirectoryError, InvalidPathError, FilesystemFullError`

Appends text to an existing file.

### `write_bytes_at_offset_sync(path: FilesystemPath, offset: int64, content: Bytes): void errors FileNotFoundError, PermissionDeniedError, WriteFailureError, OffsetOutOfRangeError, InvalidPathError, FilesystemFullError`

Writes bytes at a specific file offset. It fails when the offset is invalid or outside the allowed range.

## File management

### `create_file_sync(path: FilesystemPath): void errors FileAlreadyExistsError, PermissionDeniedError, CreateFailureError, InvalidPathError, FilesystemFullError`

Creates a new empty file and fails if the file already exists.

### `delete_file_sync(path: FilesystemPath): void errors FileNotFoundError, PermissionDeniedError, DeleteFailureError, IsADirectoryError, InvalidPathError`

Deletes a file. It fails if the path is missing or points to a directory.

### `copy_file_sync(source: FilesystemPath, destination: FilesystemPath): void errors FileNotFoundError, PermissionDeniedError, CopyFailureError, IsADirectoryError, InvalidPathError, FilesystemFullError`

Copies one file to another path.

### `move_path_sync(source: FilesystemPath, destination: FilesystemPath): void errors FileNotFoundError, PermissionDeniedError, MoveFailureError, FileAlreadyExistsError, InvalidPathError`

Moves or renames a path.

### `path_exists_sync(path: FilesystemPath): boolean errors PermissionDeniedError, InvalidPathError`

Returns whether a path exists, while still surfacing permission and invalid-path failures.

## Metadata and directories

### `read_metadata_sync(path: FilesystemPath): FileMetadata errors FileNotFoundError, PermissionDeniedError, MetadataUnavailableError, InvalidPathError`

Reads metadata such as size, directory status, symlink status, and modification time.

### `read_metadata_nofollow_sync(path: FilesystemPath): FileMetadata errors FileNotFoundError, PermissionDeniedError, MetadataUnavailableError, InvalidPathError`

Reads metadata without following symlinks.

### `create_directory_sync(path: FilesystemPath): void errors FileAlreadyExistsError, PermissionDeniedError, CreateFailureError, InvalidPathError, FilesystemFullError`

Creates one directory and fails if it already exists.

### `create_directory_recursive_sync(path: FilesystemPath): void errors PermissionDeniedError, CreateFailureError, InvalidPathError, FilesystemFullError`

Creates a directory and any missing parents.

### `delete_directory_sync(path: FilesystemPath): void errors DirectoryNotFoundError, PermissionDeniedError, DeleteFailureError, DirectoryNotEmptyError, IsNotADirectoryError, InvalidPathError`

Deletes an empty directory.

### `delete_directory_recursive_sync(path: FilesystemPath): void errors DirectoryNotFoundError, PermissionDeniedError, DeleteFailureError, IsNotADirectoryError, InvalidPathError`

Deletes a directory tree recursively.

### `list_directory_sync(path: FilesystemPath): FilesystemPath[] errors DirectoryNotFoundError, PermissionDeniedError, ReadFailureError, IsNotADirectoryError, InvalidPathError`

Returns the entries inside a directory as filesystem paths.

### `is_file_sync(path: FilesystemPath): boolean errors PermissionDeniedError, InvalidPathError`

Returns true if the path is a file, following symlinks.

### `is_file_nofollow_sync(path: FilesystemPath): boolean errors PermissionDeniedError, InvalidPathError`

Returns true if the path itself is a file without following symlinks.

### `is_directory_sync(path: FilesystemPath): boolean errors PermissionDeniedError, InvalidPathError`

Returns true if the path is a directory, following symlinks.

### `is_directory_nofollow_sync(path: FilesystemPath): boolean errors PermissionDeniedError, InvalidPathError`

Returns true if the path itself is a directory without following symlinks.

## Stdout writer and terminal APIs

These functions are useful for programs that need more control than `print`.

### `print_text_sync(text: string): void errors WriteFailureError, SinkClosedError`

Writes text to standard output without adding a newline.

### `flush_standard_output_sync(): void errors FlushFailureError, SinkClosedError`

Flushes standard output.

### `stdout_writer(): StdoutWriter`

Returns a writer handle for standard output.

### `writer_write_sync(writer: StdoutWriter, text: string): void errors WriteFailureError, SinkClosedError`

Writes text through a writer handle.

### `writer_flush_sync(writer: StdoutWriter): void errors FlushFailureError, SinkClosedError`

Flushes a writer handle.

### `stdout_terminal(): StdoutTerminal`

Returns a terminal handle for standard output.

### `terminal_supports_ansi(terminal: StdoutTerminal): boolean`

Returns whether the terminal supports ANSI control sequences.

### `terminal_clear_screen_on_sync(terminal: StdoutTerminal): void errors TerminalWriteFailureError`

Clears the screen for the given terminal handle.

### `terminal_move_cursor_on_sync(terminal: StdoutTerminal, row: int32, column: int32): void errors TerminalWriteFailureError, InvalidCursorPositionError`

Moves the cursor for the given terminal handle. Invalid row or column values fail with `InvalidCursorPositionError`.

### `terminal_draw_rows_sync(terminal: StdoutTerminal, rows: string[]): void errors TerminalWriteFailureError`

Draws multiple rows to the terminal.

### `terminal_clear_screen_sync(): void errors TerminalWriteFailureError`

Convenience form that clears standard output's terminal.

### `terminal_move_cursor_sync(row: int32, column: int32): void errors TerminalWriteFailureError, InvalidCursorPositionError`

Convenience form that moves the cursor on standard output's terminal.

The Game of Life project uses this family to redraw the terminal. See `test-projects/game-of-life-full/src/render.op`.

## Time APIs

### `sleep_ms_sync(milliseconds: int32): void errors InvalidSleepDurationError`

Blocks the current thread for the requested number of milliseconds. Negative or otherwise invalid durations fail.

### `frame_clock_new(frames_per_second: int32): FrameClock errors InvalidFrameRateError`

Creates a frame clock for fixed-rate loops. Invalid frame rates fail.

### `frame_clock_wait_next_sync(clock: FrameClock): void`

Waits until the next frame deadline for the frame clock and updates the next deadline.

Source syntax usually constructs `FrameClock` with `new FrameClock:` rather than calling `frame_clock_new` directly:

```opal
let clock = propagate new FrameClock:
    frames_per_second: 15
propagate frame_clock_wait_next_sync(clock)
```

## Internal runtime helpers

These names appear in the compiler/runtime registry but are not normal user-facing functions. They exist so generated programs can call the correct runtime entry points.

| Helper | Purpose |
|---|---|
| `printf` | Declares C `printf` for low-level formatted output. |
| `print_string` | Runtime helper for printing strings. |
| `print_int8`, `print_int16`, `print_int32`, `print_int64` | Runtime helpers for printing signed integers. |
| `print_uint8`, `print_uint16`, `print_uint32`, `print_uint64` | Runtime helpers for printing unsigned integers. |
| `print_float32`, `print_float64` | Runtime helpers for printing floats. |
| `array_length` | Lowering helper for array length. Prefer `.length` in source. |
| `opal_array_bounds_error` | Reports generated-code array bounds failures. |
| `opal_runtime_error` | Reports generated-code runtime failures. |

## Error names used by the standard library

Common standard-library error types include:

- `ParseError`
- `HexDecodeError`
- `SliceRangeError`
- `FileNotFoundError`
- `PermissionDeniedError`
- `ReadFailureError`
- `WriteFailureError`
- `FlushFailureError`
- `BuilderFinishedError`
- `AllocationFailureError`
- `SinkClosedError`
- `IsADirectoryError`
- `IsNotADirectoryError`
- `DirectoryNotFoundError`
- `DirectoryNotEmptyError`
- `InvalidPathError`
- `InvalidUtf8Error`
- `OffsetOutOfRangeError`
- `FilesystemFullError`
- `CreateFailureError`
- `DeleteFailureError`
- `CopyFailureError`
- `MoveFailureError`
- `MetadataUnavailableError`
- `TerminalWriteFailureError`
- `InvalidCursorPositionError`
- `InvalidSleepDurationError`
- `InvalidFrameRateError`

If a function lists an error type, the compiler expects callers to handle or propagate it.

## Planned standard-library areas

The following are proposal areas, not finished public APIs:

- Regex
- Crypto hashing
- Network/HTTP
- Subprocess execution
- Serialization
- Compression
- UUIDs
- Rich testing DSLs
- Expanded time/date APIs

See `stdlib-proposals/` for design drafts.
