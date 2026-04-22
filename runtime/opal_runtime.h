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

/* Filesystem stdlib surface result types.
 * Each wraps a typed value pointer and a nullable error string.
 * NULL error means success; non-NULL error means failure (value is undefined). */
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

#endif
