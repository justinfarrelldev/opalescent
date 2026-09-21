# Opalescent Standard Library

This is the user-facing reference for functions imported from `standard` plus the runtime helpers that the compiler may lower to internally. Every public entry below includes what the function does, not just its name.

## Compatibility notes

- Public string indexing and range-style string helpers use zero-based Unicode scalar positions, not byte offsets.
- `string_is_blank` and `string_trim_whitespace` use the Unicode `White_Space` property for the Unicode version bundled with this Opalescent release.
- `string_split_lines` recognizes `\n`, `\r\n`, and bare `\r` as line terminators and does not create an extra trailing empty line for a final terminator.
- `string_join` is allocation-fallible and must be handled with `propagate` or `guard`.

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

```opal
import print, println, take_input from standard
```

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

### `take_input(): string errors StandardInputReadError`

Reads one line from standard input and returns it without the trailing newline. Callers must handle `StandardInputReadError` with `guard` or `propagate`. Empty input at EOF fails with `StandardInputReadError: EndOfInput`; a final line without a newline is still returned successfully.

```opal
let answer = propagate take_input()
print('you typed {answer}')
```

## Parsing strings into numbers

```opal
import string_to_int8, string_to_int16, string_to_int32, string_to_int64,
    string_to_uint8, string_to_uint16, string_to_uint32, string_to_uint64,
    string_to_float32, string_to_float64
from standard
```

These parse decimal text into the requested numeric type. They skip leading Unicode `White_Space`, require the whole trimmed input to be valid, and fail with `ParseError` for invalid digits, empty input, or values outside the target range.

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

```opal
import int8_to_string, int16_to_string, int32_to_string, int64_to_string,
    uint8_to_string, uint16_to_string, uint32_to_string, uint64_to_string,
    float32_to_string, float64_to_string, bool_to_string
from standard
```

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

```opal
import string_length, string_find_index_or, string_find_last_index_of_text,
    string_split_lines, string_is_blank, string_trim_whitespace,
    string_take_prefix, string_take_suffix, string_extract_range,
    string_insert_at, string_delete_range, string_replace_range,
    terminal_text_cell_width, terminal_text_clip_to_cells,
    string_join, string_builder_new, string_builder_push,
    string_builder_finish
from standard
```

### `string_length(text: string): int64`

Returns the number of Unicode scalar values in `text`. Source code usually uses the `.length` member syntax, which lowers to this helper.

```opal
let text = 'hello'
print('length: {text.length}')
```

### `string_find_index_or(text: string, search_text: string, fallback_index: int64): int64`

Returns the first Unicode scalar index where `search_text` appears in `text`.

- If `search_text` is empty, returns `fallback_index`.
- If `search_text` does not appear, returns `fallback_index`.

```opal
let index = string_find_index_or('hé🙂z', '🙂', -1 as int64)
```

### `string_find_last_index_of_text(text: string, search_text: string): int64 errors StringEmptySearchTextError, StringPatternNotFoundError`

Returns the last Unicode scalar index where `search_text` appears in `text`.

- `StringEmptySearchTextError` — `search_text` is empty.
- `StringPatternNotFoundError` — `search_text` does not appear in `text`.

```opal
let last = propagate string_find_last_index_of_text('bananana', 'ana')
```

### `string_split_lines(text: string): string[] errors AllocationFailureError`

Splits `text` into logical lines.

- Recognized line terminators: `\n`, `\r\n`, and `\r`
- Line terminators are not included in the returned strings
- A final trailing terminator does not create an extra empty string

```opal
let lines = propagate string_split_lines('a\n\nb')
# ['a', '', 'b']
```

### `string_is_blank(text: string): boolean`

Returns `true` when `text` is empty or every Unicode scalar value in `text` has the Unicode `White_Space` property for this Opalescent release.

### `string_trim_whitespace(text: string): string errors AllocationFailureError`

Removes leading and trailing Unicode `White_Space` scalar values. Interior whitespace is preserved.

### `string_take_prefix(text: string, count: int64): string errors StringNegativeCountError, StringRangeOutOfBoundsError, AllocationFailureError`

Returns the first `count` Unicode scalar values from `text`.

- `count == 0` returns `''`
- `count == text.length` returns the full string
- `StringNegativeCountError` — `count < 0`
- `StringRangeOutOfBoundsError` — `count > text.length`

### `string_take_suffix(text: string, count: int64): string errors StringNegativeCountError, StringRangeOutOfBoundsError, AllocationFailureError`

Returns the last `count` Unicode scalar values from `text`.

- `count == 0` returns `''`
- `count == text.length` returns the full string
- `StringNegativeCountError` — `count < 0`
- `StringRangeOutOfBoundsError` — `count > text.length`

### `string_extract_range(text: string, start: int64, end: int64): string errors StringRangeOrderError, StringRangeOutOfBoundsError, AllocationFailureError`

Returns the Unicode scalar range `[start, end)`.

- `start == end` returns `''`
- `[0, text.length)` returns the full string
- `StringRangeOrderError` — `end < start`
- `StringRangeOutOfBoundsError` — negative index or `end > text.length`

### `string_insert_at(text: string, scalar_index: int64, inserted: string): string errors StringRangeOutOfBoundsError, AllocationFailureError`

Returns a new string with `inserted` placed before the Unicode scalar at `scalar_index`.

- `scalar_index == 0` inserts at the beginning
- `scalar_index == text.length` appends at the end
- `StringRangeOutOfBoundsError` — negative index or `scalar_index > text.length`

### `string_delete_range(text: string, start: int64, end: int64): string errors StringRangeOrderError, StringRangeOutOfBoundsError, AllocationFailureError`

Returns a new string with the Unicode scalar range `[start, end)` removed.

- `start == end` returns the original text content in a new string value
- `StringRangeOrderError` — `end < start`
- `StringRangeOutOfBoundsError` — negative index or `end > text.length`

### `string_replace_range(text: string, start: int64, end: int64, replacement: string): string errors StringRangeOrderError, StringRangeOutOfBoundsError, AllocationFailureError`

Returns a new string with the Unicode scalar range `[start, end)` replaced by `replacement`.

- `start == end` inserts `replacement` at `start`
- `StringRangeOrderError` — `end < start`
- `StringRangeOutOfBoundsError` — negative index or `end > text.length`

### `terminal_text_cell_width(text: string): int64`

Returns the terminal display-cell width of `text` using Opalescent's Unicode terminal width policy. The current policy tracks Unicode 15.1-style extended grapheme behavior for combining marks, variation selectors, emoji modifiers, regional-indicator flag pairs, keycap sequences, and zero-width-joiner emoji sequences. Ambiguous-width scalars are treated as single-cell; CJK wide characters and supported emoji presentation scalars count as two cells.

### `terminal_text_clip_to_cells(text: string, max_cells: int64): string, int64 errors TerminalTextLayoutError, AllocationFailureError`

Clips `text` to the largest grapheme-boundary prefix that fits within `max_cells` terminal cells and returns `clipped, used_cells`.

- `TerminalTextLayoutError.NegativeCellLimit` (runtime leaf string: `NegativeCellLimit`) — `max_cells < 0`
- `AllocationFailureError` — allocating the clipped string fails
- Clipping never splits a combining sequence or zero-width-joiner emoji sequence

### `string_join(parts: string[], separator: string): string errors AllocationFailureError`

Returns one string made by placing `separator` between each element of `parts`. Use it for line rendering and simple accumulation.

```opal
let lines: string[] = ['a', 'b', 'c']
let text = propagate string_join(lines, '\n')
```

`string_join` now participates in explicit error handling because allocation may fail.

### `string_builder_new(): StringBuilder`

Creates an empty string builder. A builder is useful when repeated string concatenation would be noisy or inefficient.

### `string_builder_push(builder: StringBuilder, text: string): void errors BuilderFinishedError, AllocationFailureError`

Appends `text` to `builder`. It fails if the builder has already been finished or if the runtime cannot allocate storage for the appended text.

### `string_builder_finish(builder: StringBuilder): string errors BuilderFinishedError, AllocationFailureError`

Finishes the builder and returns the accumulated string. Calling push or finish again after finishing is an error.

## Random integer helpers

```opal
import random_int8, random_int16, random_int32, random_int64,
    random_uint8, random_uint16, random_uint32, random_uint64
from math
```

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

```opal
import array_length, array_filled, reserve, clear from standard
```

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
let inserted = propagate values.insert(1 as int64, 9 as int32)
let updated, removed = propagate inserted.remove_at(0 as int64)
```

- `values.insert(index, value): T[] errors IndexOutOfBoundsError, AllocationFailureError` returns a new array with `value` inserted before zero-based `index`. `index == values.length` appends.
- `values.remove_at(index): updated: T[], removed: T errors IndexOutOfBoundsError, AllocationFailureError` returns a new array with the element removed and also returns the removed element.

Implemented/tested fixture areas include `array-map`, `array-filter`, `array-reduce`, `array-zip`, `array-pair`, and method-style array insert/remove fixtures under `test-projects/`.

### `.at(index: int64): T errors IndexOutOfBoundsError`

`values.at(index)` returns the element at a zero-based index. Negative, empty-array, and out-of-range accesses return `IndexOutOfBoundsError`, so callers must use `guard` or `propagate`.

```opal
let value = propagate values.at(0)
```

## Bytes

```opal
import bytes_new, bytes_length, bytes_to_hex, bytes_from_hex,
    bytes_concatenate, bytes_slice
from standard
```

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

```opal
import path_from, join_path_components, path_parent_directory,
    path_file_name, path_file_extension, normalize_path,
    path_to_string, absolute_path_sync
from standard
```

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

```opal
import read_contents_sync, read_text_sync, read_first_line_sync,
    read_lines_sync, read_bytes_at_offset_sync
from standard
```

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

```opal
import write_contents_sync, write_text_sync, write_contents_atomic_sync,
    write_text_atomic_sync, append_contents_sync, append_text_sync,
    write_bytes_at_offset_sync
from standard
```

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

```opal
import create_file_sync, delete_file_sync, copy_file_sync, move_path_sync,
    path_exists_sync
from standard
```

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

```opal
import read_metadata_sync, read_metadata_nofollow_sync, create_directory_sync,
    create_directory_recursive_sync, delete_directory_sync,
    delete_directory_recursive_sync, list_directory_sync, is_file_sync,
    is_file_nofollow_sync, is_directory_sync, is_directory_nofollow_sync
from standard
```

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

```opal
import print_text_sync, flush_standard_output_sync, stdout_writer,
    writer_write_sync, writer_flush_sync, stdout_terminal,
    terminal_supports_ansi, terminal_clear_screen_on_sync,
    terminal_move_cursor_on_sync, terminal_draw_rows_sync,
    terminal_clear_screen_sync, terminal_move_cursor_sync
from standard
```

These functions are useful for programs that need more control than `print`.

### `print_text_sync(text: string): void errors WriteFailureError, SinkClosedError`

Writes text to standard output without adding a newline.

### `flush_standard_output_sync(): void errors FlushFailureError, SinkClosedError`

Flushes standard output.

### `stdout_writer(): StdoutWriter errors StandardOutputHandleError`

Returns a writer handle for standard output; callers must handle coordinator acquisition failures.

### `writer_write_sync(writer: StdoutWriter, text: string): void errors WriteFailureError, SinkClosedError`

Writes text through a writer handle.

### `writer_flush_sync(writer: StdoutWriter): void errors FlushFailureError, SinkClosedError`

Flushes a writer handle.

### `stdout_terminal(): StdoutTerminal errors StandardOutputHandleError`

Returns a terminal handle for standard output; callers must handle coordinator acquisition failures.

### `terminal_supports_ansi(terminal: StdoutTerminal): boolean errors StandardOutputCapabilityError`

Returns whether the terminal supports ANSI control sequences; stale or unavailable leases are reported through `StandardOutputCapabilityError`.

### `terminal_clear_screen_on_sync(terminal: StdoutTerminal): void errors TerminalWriteFailureError, SinkClosedError`

Clears the screen for the given terminal handle.

### `terminal_move_cursor_on_sync(terminal: StdoutTerminal, row: int32, column: int32): void errors TerminalWriteFailureError, InvalidCursorPositionError, SinkClosedError`

Moves the cursor for the given terminal handle. Invalid row or column values fail with `InvalidCursorPositionError`.

### `terminal_draw_rows_sync(terminal: StdoutTerminal, rows: string[]): void errors TerminalWriteFailureError, SinkClosedError`

Draws multiple rows to the terminal.

### `terminal_clear_screen_sync(): void errors TerminalWriteFailureError, SinkClosedError`

Convenience form that clears standard output's terminal.

### `terminal_move_cursor_sync(row: int32, column: int32): void errors TerminalWriteFailureError, InvalidCursorPositionError, SinkClosedError`

Convenience form that moves the cursor on standard output's terminal.

The Game of Life project uses this family to redraw the terminal. See `test-projects/game-of-life-full/src/render.op`.

## Terminal session/input APIs

The selected terminal session/input v1 is the `typed-event-session` proposal. The current branch implements the Rust runtime/stdlib model for owning the process interactive standard-input/raw-output terminal pair through a coordinator, exposing typed `TerminalInputEvent` values, requiring explicit `TrustedTerminalOutput` or `SafeTerminalDiagnosticOutput` for session writes, and rejecting legacy standard I/O while a session owns the terminal. Generated Opalescent programs lower the runtime-ready data-model/core-prerequisite subset plus a test-harness injected fake-backend path for `terminal_session_open_sync`, one-event reads, trusted writes, flush, close, and high-level rendering helpers (`terminal_session_clear_screen_sync`, `terminal_session_move_cursor_sync`, `terminal_session_draw_rows_sync`, and `terminal_session_bell_sync`). There is intentionally no `terminal_session_output_terminal` or `AcquireOutputTerminal` API.

Supported implementation surfaces include selected session lifecycle, one-event reads, pause/resume workflow, Linux and Windows backend contracts, safe diagnostics, deterministic test-only fake backend support under `standard.testing.terminal`, and the companion terminal chord router under `standard.terminal.chords`. Windows process-control remains unavailable and returns the unsupported-host contract from the process-control prerequisite; it is not synthesized as terminal input.

Historical alternatives such as portable input packet streams or batched event pumps remain archived proposal records only. Use the selected declarations in `stdlib-proposals/terminal-session-input/typed-event-session/typed_event_session.types.op` and `terminal_chords.types.op` as the source of truth.

## Time APIs

```opal
import sleep_ms_sync, frame_clock_new, frame_clock_wait_next_sync from standard
```

### `sleep_ms_sync(milliseconds: int32): void errors InvalidDurationError`

Blocks the current thread for the requested number of milliseconds. Negative or otherwise invalid durations fail.

### `frame_clock_new(frames_per_second: int32): FrameClock errors InvalidFrameRateError`

Creates a frame clock for fixed-rate loops. Invalid frame rates fail.

### `frame_clock_wait_next_sync(clock: FrameClock): void errors InvalidFrameRateError`

Waits until the next frame deadline for the frame clock and updates the next deadline.

Source syntax usually constructs `FrameClock` with `new FrameClock:` rather than calling `frame_clock_new` directly:

```opal
let clock = propagate new FrameClock:
    frames_per_second: 15
propagate frame_clock_wait_next_sync(clock)
```

## Process module

The `process` module provides functions for interacting with the current process, including environment variables, working directory, and process termination.

```opal
import current_working_directory_sync,
    current_executable_path_sync,
    current_executable_directory_sync,
    set_current_working_directory_sync,
    get_environment_variable,
    get_environment_variable_or,
    environment_variable_exists,
    exit_process
from process
```

### `current_working_directory_sync(): FilesystemPath`

Returns the absolute path of the current working directory.

**Errors:** `PermissionDeniedError`, `InvalidPathError`, `CurrentWorkingDirectoryUnavailableError`

### `current_executable_path_sync(): FilesystemPath`

Returns the absolute path of the currently running executable.

**Errors:** `PermissionDeniedError`, `InvalidPathError`, `CurrentExecutablePathUnavailableError`

### `current_executable_directory_sync(): FilesystemPath`

Returns the absolute path of the directory containing the currently running executable. This is derived from the executable path.

**Errors:** `PermissionDeniedError`, `InvalidPathError`, `CurrentExecutablePathUnavailableError`

### `set_current_working_directory_sync(path: FilesystemPath): void`

Changes the current working directory to `path`.

**Errors:** `FileNotFoundError`, `PermissionDeniedError`, `IsNotADirectoryError`, `InvalidPathError`

### `get_environment_variable(name: string): string`

Returns the value of the environment variable `name`.

**Errors:** `EnvironmentVariableNotFoundError`, `InvalidEnvironmentVariableNameError`, `InvalidUtf8Error`

### `get_environment_variable_or(name: string, default_value: string): string`

Returns the value of the environment variable `name`, or `default_value` if the variable is not found.

**Errors:** `InvalidEnvironmentVariableNameError`, `InvalidUtf8Error`

### `environment_variable_exists(name: string): boolean`

Returns whether the environment variable `name` exists.

**Errors:** `InvalidEnvironmentVariableNameError`

### `exit_process(code: int32): void`

Immediately terminates the current process with the given exit code.

**Footgun:** This function currently returns `void` but should be treated as if it never returns. It will eventually return a `never` type once supported by the language. Portable shell-facing exit codes should be low nonnegative values (e.g., `42`).

```opal
exit_process(42)
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

## Standard-library error families

Function signatures above deliberately list the precise leaf errors their implementation can emit. The family declarations below are compatibility declarations: a family may be used in an `errors` clause to cover exactly its listed leaves, without changing the function's emitted leaf set.

Every individual leaf remains valid. For example, `errors HexDecodeError` is still accepted even though `BytesError` covers `HexDecodeError` and `SliceRangeError`. `ParseError` and `IndexAccessError` are singleton declaration families, so they provide coverage but no shorter-list suggestion.

| Family | Exact leaf members | Suggestion warning |
|---|---|---:|
| `ParseError` | `ParseError` | No |
| `BytesError` | `HexDecodeError`, `SliceRangeError` | Yes |
| `StringSearchError` | `StringEmptySearchTextError`, `StringPatternNotFoundError` | Yes |
| `StringRangeError` | `StringNegativeCountError`, `StringRangeOutOfBoundsError`, `StringRangeOrderError` | Yes |
| `StringBuilderError` | `BuilderFinishedError`, `AllocationFailureError` | Yes |
| `OutputError` | `WriteFailureError`, `FlushFailureError`, `SinkClosedError` | Yes |
| `TerminalError` | `TerminalWriteFailureError`, `InvalidCursorPositionError`, `SinkClosedError` | Yes |
| `TimeError` | `InvalidDurationError`, `InvalidFrameRateError` | Yes |
| `ProcessPathError` | `PermissionDeniedError`, `InvalidPathError`, `CurrentWorkingDirectoryUnavailableError`, `CurrentExecutablePathUnavailableError`, `FileNotFoundError`, `IsNotADirectoryError` | Yes |
| `ProcessEnvError` | `EnvironmentVariableNotFoundError`, `InvalidEnvironmentVariableNameError`, `InvalidUtf8Error` | Yes |
| `FilesystemPathError` | `InvalidPathError`, `PermissionDeniedError` | Yes |
| `FilesystemReadError` | `FileNotFoundError`, `PermissionDeniedError`, `ReadFailureError`, `IsADirectoryError`, `InvalidPathError`, `InvalidUtf8Error`, `OffsetOutOfRangeError` | Yes |
| `FilesystemWriteError` | `FileNotFoundError`, `PermissionDeniedError`, `WriteFailureError`, `IsADirectoryError`, `InvalidPathError`, `FilesystemFullError`, `OffsetOutOfRangeError` | Yes |
| `FilesystemCreateError` | `FileAlreadyExistsError`, `PermissionDeniedError`, `CreateFailureError`, `InvalidPathError`, `FilesystemFullError` | Yes |
| `FilesystemDeleteError` | `FileNotFoundError`, `PermissionDeniedError`, `DeleteFailureError`, `IsADirectoryError`, `InvalidPathError` | Yes |
| `FilesystemDirectoryDeleteError` | `DirectoryNotFoundError`, `PermissionDeniedError`, `DeleteFailureError`, `DirectoryNotEmptyError`, `IsNotADirectoryError`, `InvalidPathError` | Yes |
| `FilesystemCopyMoveError` | `FileNotFoundError`, `PermissionDeniedError`, `CopyFailureError`, `MoveFailureError`, `IsADirectoryError`, `FileAlreadyExistsError`, `InvalidPathError`, `FilesystemFullError` | Yes |
| `FilesystemMetadataError` | `FileNotFoundError`, `PermissionDeniedError`, `MetadataUnavailableError`, `InvalidPathError` | Yes |
| `FilesystemListError` | `DirectoryNotFoundError`, `PermissionDeniedError`, `ReadFailureError`, `IsNotADirectoryError`, `InvalidPathError` | Yes |
| `FilesystemError` | The union of the filesystem leaves above, plus registered-but-unproduced `LineOutOfRangeError` and `SetPermissionsError` | No |
| `IndexAccessError` | `IndexOutOfBoundsError` | No |

A complete explicit leaf list may produce a non-fatal warning that suggests the applicable family. This is guidance only: it never rewrites source, never rejects a declaration, and never turns a valid manual leaf declaration into an error.

```opal
let decode = f(text: string): Bytes errors BytesError =>
    return propagate bytes_from_hex(text)

let decode_with_leaf = f(text: string): Bytes errors HexDecodeError =>
    return propagate bytes_from_hex(text)
```

If a function lists an error type, the compiler expects callers to handle or propagate it.

## Planned standard-library areas

The following are proposal areas, not finished public APIs:

- Regex
- Crypto hashing
- Network/HTTP
- Subprocess execution
- Terminal sessions and interactive input
- Serialization
- Compression
- UUIDs
- Rich testing DSLs
- Expanded time/date APIs

See `stdlib-proposals/` for design drafts.
