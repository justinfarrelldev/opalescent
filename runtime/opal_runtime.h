#ifndef OPAL_RUNTIME_H
#define OPAL_RUNTIME_H

#include <stdint.h>

void opal_runtime_init(void);

typedef struct { int8_t value;   const char* error; } ParseResultI8;
typedef struct { int16_t value;  const char* error; } ParseResultI16;
typedef struct { int32_t value;  const char* error; } ParseResultI32;
typedef struct { int64_t value;  const char* error; } ParseResultI64;
typedef struct { uint8_t value;  const char* error; } ParseResultU8;
typedef struct { uint16_t value; const char* error; } ParseResultU16;
typedef struct { uint32_t value; const char* error; } ParseResultU32;
typedef struct { uint64_t value; const char* error; } ParseResultU64;
typedef struct { float value;    const char* error; } ParseResultF32;
typedef struct { double value;   const char* error; } ParseResultF64;

void opal_runtime_error(const char* message);

char* take_input(void);
void print_string(const char* s);

void print_int8(int8_t n);
void print_int16(int16_t n);
void print_int32(int32_t n);
void print_int64(int64_t n);
void print_uint8(uint8_t n);
void print_uint16(uint16_t n);
void print_uint32(uint32_t n);
void print_uint64(uint64_t n);
void print_float32(float n);
void print_float64(double n);

int8_t  random_int8(int8_t min, int8_t max);
int16_t random_int16(int16_t min, int16_t max);
int32_t random_int32(int32_t min, int32_t max);
int64_t random_int64(int64_t min, int64_t max);
uint8_t  random_uint8(uint8_t min, uint8_t max);
uint16_t random_uint16(uint16_t min, uint16_t max);
uint32_t random_uint32(uint32_t min, uint32_t max);
uint64_t random_uint64(uint64_t min, uint64_t max);

ParseResultI8  string_to_int8(const char* s);
ParseResultI16 string_to_int16(const char* s);
ParseResultI32 string_to_int32(const char* s);
ParseResultI64 string_to_int64(const char* s);
ParseResultU8  string_to_uint8(const char* s);
ParseResultU16 string_to_uint16(const char* s);
ParseResultU32 string_to_uint32(const char* s);
ParseResultU64 string_to_uint64(const char* s);
ParseResultF32 string_to_float32(const char* s);
ParseResultF64 string_to_float64(const char* s);

char* int8_to_string(int8_t value);
char* int16_to_string(int16_t value);
char* int32_to_string(int32_t value);
char* int64_to_string(int64_t value);
char* uint8_to_string(uint8_t value);
char* uint16_to_string(uint16_t value);
char* uint32_to_string(uint32_t value);
char* uint64_to_string(uint64_t value);
char* float32_to_string(float value);
char* float64_to_string(double value);
char* bool_to_string(int8_t value);
int64_t string_length(const char* value);
int64_t array_length(const void* array, int64_t length);

/* `Bytes` stdlib surface. `OpalBytes` is an opaque owned heap pointer
 * passed across the FFI boundary as `i8*`. Fallible helpers mirror the
 * `ParseResult*` `{value, error}` convention so `guard`/`propagate`
 * lowering is identical. */
typedef struct OpalBytes OpalBytes;
typedef struct { OpalBytes* value; const char* error; } BytesResult;

OpalBytes* bytes_new(void);
int32_t    bytes_length(OpalBytes* bytes);
char*      bytes_to_hex(OpalBytes* bytes);
OpalBytes* bytes_concatenate(OpalBytes* left, OpalBytes* right);
BytesResult bytes_from_hex(const char* hex);
BytesResult bytes_slice(OpalBytes* source, int32_t start, int32_t end);
char* path_from(const char* raw);
char* join_path_components(const char* base, const char** components, int64_t count);
char* path_parent_directory(const char* path);
char* path_file_name(const char* path);
char* path_file_extension(const char* path);
char* normalize_path(const char* path);
char* path_to_string(const char* path);

/* Filesystem stdlib surface result types.
 *
 * FsResult success/failure sentinel contract (frozen ABI rule for lowering):
 * - Success is represented exclusively by `error == NULL`.
 * - Failure is represented exclusively by `error != NULL`.
 * - The payload field(s) (`value`, and `count` for array results) are only
 *   semantically valid on success and MUST be treated as undefined on failure.
 * - `FsVoidResult` has no payload semantics; callers and lowering must still
 *   use only `error` as the sentinel, and ignore `value` on both paths.
 *
 * This contract applies to all filesystem result wrappers used by guard/
 * propagate lowering, including:
 * - FsPathResult
 * - FsBytesResult
 * - FsStringResult
 * - FsStringArrayResult
 * - FsVoidResult
 * - FsBooleanResult
 * - FsMetadataResult
 * - FsPathArrayResult
 *
 * Infallible lexical path helper policy (char* ABI, no Fs*Result wrappers):
 * - `path_from(raw)` returns a heap-owned duplicate of `raw`; NULL/empty input
 *   returns the empty-string sentinel "".
 * - `normalize_path(path)` is lexical-only (no filesystem probes): separators are
 *   collapsed, `.` segments removed, and `..` segments resolved. Absolute paths
 *   preserve their leading separator. If an absolute path would escape above
 *   root via `..`, normalization returns the empty-string sentinel "".
 * - `join_path_components(base, parts)` is lexical-only: absolute components
 *   reset the accumulator, separators are deduplicated to one path separator,
 *   and the final path is normalized with the same `.`/`..` rules as
 *   `normalize_path`.
 * - Trailing-separator behavior is preserved for non-empty normalized paths;
 *   empty input and root-escape normalization both resolve to "".
 */
typedef struct { void*      value; const char* error; } FsVoidResult;
typedef struct { OpalBytes* value; const char* error; } FsBytesResult;
typedef struct { char*      value; const char* error; } FsStringResult;
typedef struct { int8_t     value; const char* error; } FsBooleanResult;
typedef struct { int32_t    value; const char* error; } FsInt32Result;
typedef struct { int64_t    value; const char* error; } FsInt64Result;
typedef struct { char*      value; const char* error; } FsPathResult;
typedef struct { char**     value; int64_t count; const char* error; } FsPathArrayResult;
typedef struct { char**     value; int64_t count; const char* error; } FsStringArrayResult;
typedef struct { void*      value; const char* error; } FsMetadataResult;
typedef struct { void*      value; const char* error; } FsPermissionsResult;

FsBytesResult read_contents_sync(const char* path);
FsStringResult read_text_sync(const char* path);
FsStringResult read_first_line_sync(const char* path);
FsStringArrayResult read_lines_sync(const char* path);
FsBytesResult read_bytes_at_offset_sync(const char* path, int64_t offset, int64_t length);
FsVoidResult write_contents_sync(const char* path, OpalBytes* data);
FsVoidResult write_text_sync(const char* path, const char* text);
FsVoidResult write_contents_atomic_sync(const char* path, OpalBytes* data);
FsVoidResult write_text_atomic_sync(const char* path, const char* text);
FsVoidResult append_contents_sync(const char* path, OpalBytes* data);
FsVoidResult append_text_sync(const char* path, const char* text);
FsVoidResult write_bytes_at_offset_sync(const char* path, int64_t offset, OpalBytes* data);
FsVoidResult create_file_sync(const char* path);
FsVoidResult delete_file_sync(const char* path);
FsVoidResult copy_file_sync(const char* source, const char* destination);
FsVoidResult move_path_sync(const char* source, const char* destination);
FsBooleanResult path_exists_sync(const char* path);
FsMetadataResult read_metadata_sync(const char* path);
FsMetadataResult read_metadata_nofollow_sync(const char* path);
FsVoidResult create_directory_sync(const char* path);
FsVoidResult create_directory_recursive_sync(const char* path);
FsVoidResult delete_directory_sync(const char* path);
FsVoidResult delete_directory_recursive_sync(const char* path);
FsPathArrayResult list_directory_sync(const char* path);
FsBooleanResult is_file_sync(const char* path);
FsBooleanResult is_file_nofollow_sync(const char* path);
FsBooleanResult is_directory_sync(const char* path);
FsBooleanResult is_directory_nofollow_sync(const char* path);

#endif
