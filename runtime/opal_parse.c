#include "opal_portability.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <errno.h>
#include <limits.h>
#include <float.h>
#include <stdint.h>

typedef struct { int8_t value;   const char* error; } ParseResultI8;
typedef struct { int16_t value;  const char* error; } ParseResultI16;
typedef struct { int32_t value;  const char* error; } ParseResultI32;
#ifndef OPAL_PARSE_RESULT_I64_DEFINED
typedef struct { int64_t value;  const char* error; } ParseResultI64;
#define OPAL_PARSE_RESULT_I64_DEFINED 1
#endif
typedef struct { uint8_t value;  const char* error; } ParseResultU8;
typedef struct { uint16_t value; const char* error; } ParseResultU16;
typedef struct { uint32_t value; const char* error; } ParseResultU32;
#ifndef OPAL_PARSE_RESULT_U64_DEFINED
typedef struct { uint64_t value; const char* error; } ParseResultU64;
#define OPAL_PARSE_RESULT_U64_DEFINED 1
#endif
typedef struct { float value;    const char* error; } ParseResultF32;
typedef struct { double value;   const char* error; } ParseResultF64;

static const char* skip_leading_whitespace(const char* s) {
    while (*s == ' ' || *s == '\t' || *s == '\n' || *s == '\r' || *s == '\f' || *s == '\v') {
        ++s;
    }
    return s;
}

static char* skip_trailing_whitespace_parse(char* s) {
    while (*s == ' ' || *s == '\t' || *s == '\n' || *s == '\r' || *s == '\f' || *s == '\v') {
        ++s;
    }
    return s;
}

static const char* invalid_digit_error_parse(char ch) {
    static OPAL_THREAD_LOCAL char msg[64];
    snprintf(msg, sizeof(msg), "invalid digit '%c' in input", ch);
    return msg;
}

ParseResultI8 string_to_int8(const char* s) {
    ParseResultI8 result = { 0, NULL };
    if (s == NULL) { result.error = "null input"; return result; }
    const char* p = skip_leading_whitespace(s);
    if (*p == '\0') { result.error = "empty input"; return result; }
    errno = 0;
    char* end;
    long long val = strtoll(p, &end, 10);
    if (end == p) { result.error = invalid_digit_error_parse(*p); return result; }
    if (errno == ERANGE || val < INT8_MIN || val > INT8_MAX) { result.error = "overflow: value exceeds int8 range"; return result; }
    end = skip_trailing_whitespace_parse(end);
    if (*end != '\0') { result.error = invalid_digit_error_parse(*end); return result; }
    result.value = (int8_t)val;
    return result;
}

ParseResultI16 string_to_int16(const char* s) {
    ParseResultI16 result = { 0, NULL };
    if (s == NULL) { result.error = "null input"; return result; }
    const char* p = skip_leading_whitespace(s);
    if (*p == '\0') { result.error = "empty input"; return result; }
    errno = 0;
    char* end;
    long long val = strtoll(p, &end, 10);
    if (end == p) { result.error = invalid_digit_error_parse(*p); return result; }
    if (errno == ERANGE || val < INT16_MIN || val > INT16_MAX) { result.error = "overflow: value exceeds int16 range"; return result; }
    end = skip_trailing_whitespace_parse(end);
    if (*end != '\0') { result.error = invalid_digit_error_parse(*end); return result; }
    result.value = (int16_t)val;
    return result;
}

ParseResultI32 string_to_int32(const char* s) {
    ParseResultI32 result = { 0, NULL };
    if (s == NULL) { result.error = "null input"; return result; }
    const char* p = skip_leading_whitespace(s);
    if (*p == '\0') { result.error = "empty input"; return result; }
    errno = 0;
    char* end;
    long long val = strtoll(p, &end, 10);
    if (end == p) { result.error = invalid_digit_error_parse(*p); return result; }
    if (errno == ERANGE || val < INT32_MIN || val > INT32_MAX) { result.error = "overflow: value exceeds int32 range"; return result; }
    end = skip_trailing_whitespace_parse(end);
    if (*end != '\0') { result.error = invalid_digit_error_parse(*end); return result; }
    result.value = (int32_t)val;
    return result;
}

ParseResultI64 string_to_int64(const char* s) {
    ParseResultI64 result = { 0, NULL };
    if (s == NULL) { result.error = "null input"; return result; }
    const char* p = skip_leading_whitespace(s);
    if (*p == '\0') { result.error = "empty input"; return result; }
    errno = 0;
    char* end;
    long long val = strtoll(p, &end, 10);
    if (end == p) { result.error = invalid_digit_error_parse(*p); return result; }
    if (errno == ERANGE) { result.error = "overflow: value exceeds int64 range"; return result; }
    end = skip_trailing_whitespace_parse(end);
    if (*end != '\0') { result.error = invalid_digit_error_parse(*end); return result; }
    result.value = (int64_t)val;
    return result;
}

ParseResultU8 string_to_uint8(const char* s) {
    ParseResultU8 result = { 0, NULL };
    if (s == NULL) { result.error = "null input"; return result; }
    const char* p = skip_leading_whitespace(s);
    if (*p == '\0') { result.error = "empty input"; return result; }
    if (*p == '-') { result.error = invalid_digit_error_parse(*p); return result; }
    errno = 0;
    char* end;
    unsigned long long val = strtoull(p, &end, 10);
    if (end == p) { result.error = invalid_digit_error_parse(*p); return result; }
    if (errno == ERANGE || val > UINT8_MAX) { result.error = "overflow: value exceeds uint8 range"; return result; }
    end = skip_trailing_whitespace_parse(end);
    if (*end != '\0') { result.error = invalid_digit_error_parse(*end); return result; }
    result.value = (uint8_t)val;
    return result;
}

ParseResultU16 string_to_uint16(const char* s) {
    ParseResultU16 result = { 0, NULL };
    if (s == NULL) { result.error = "null input"; return result; }
    const char* p = skip_leading_whitespace(s);
    if (*p == '\0') { result.error = "empty input"; return result; }
    if (*p == '-') { result.error = invalid_digit_error_parse(*p); return result; }
    errno = 0;
    char* end;
    unsigned long long val = strtoull(p, &end, 10);
    if (end == p) { result.error = invalid_digit_error_parse(*p); return result; }
    if (errno == ERANGE || val > UINT16_MAX) { result.error = "overflow: value exceeds uint16 range"; return result; }
    end = skip_trailing_whitespace_parse(end);
    if (*end != '\0') { result.error = invalid_digit_error_parse(*end); return result; }
    result.value = (uint16_t)val;
    return result;
}

ParseResultU32 string_to_uint32(const char* s) {
    ParseResultU32 result = { 0, NULL };
    if (s == NULL) { result.error = "null input"; return result; }
    const char* p = skip_leading_whitespace(s);
    if (*p == '\0') { result.error = "empty input"; return result; }
    if (*p == '-') { result.error = invalid_digit_error_parse(*p); return result; }
    errno = 0;
    char* end;
    unsigned long long val = strtoull(p, &end, 10);
    if (end == p) { result.error = invalid_digit_error_parse(*p); return result; }
    if (errno == ERANGE || val > UINT32_MAX) { result.error = "overflow: value exceeds uint32 range"; return result; }
    end = skip_trailing_whitespace_parse(end);
    if (*end != '\0') { result.error = invalid_digit_error_parse(*end); return result; }
    result.value = (uint32_t)val;
    return result;
}

ParseResultU64 string_to_uint64(const char* s) {
    ParseResultU64 result = { 0, NULL };
    if (s == NULL) { result.error = "null input"; return result; }
    const char* p = skip_leading_whitespace(s);
    if (*p == '\0') { result.error = "empty input"; return result; }
    if (*p == '-') { result.error = invalid_digit_error_parse(*p); return result; }
    errno = 0;
    char* end;
    unsigned long long val = strtoull(p, &end, 10);
    if (end == p) { result.error = invalid_digit_error_parse(*p); return result; }
    if (errno == ERANGE) { result.error = "overflow: value exceeds uint64 range"; return result; }
    end = skip_trailing_whitespace_parse(end);
    if (*end != '\0') { result.error = invalid_digit_error_parse(*end); return result; }
    result.value = (uint64_t)val;
    return result;
}

ParseResultF32 string_to_float32(const char* s) {
    ParseResultF32 result = { 0, NULL };
    if (s == NULL) { result.error = "null input"; return result; }
    const char* p = skip_leading_whitespace(s);
    if (*p == '\0') { result.error = "empty input"; return result; }
    errno = 0;
    char* end;
    float val = strtof(p, &end);
    if (end == p) { result.error = invalid_digit_error_parse(*p); return result; }
    if (errno == ERANGE || val > FLT_MAX || val < -FLT_MAX) { result.error = "overflow: value exceeds float32 range"; return result; }
    end = skip_trailing_whitespace_parse(end);
    if (*end != '\0') { result.error = invalid_digit_error_parse(*end); return result; }
    result.value = val;
    return result;
}

ParseResultF64 string_to_float64(const char* s) {
    ParseResultF64 result = { 0, NULL };
    if (s == NULL) { result.error = "null input"; return result; }
    const char* p = skip_leading_whitespace(s);
    if (*p == '\0') { result.error = "empty input"; return result; }
    errno = 0;
    char* end;
    double val = strtod(p, &end);
    if (end == p) { result.error = invalid_digit_error_parse(*p); return result; }
    if (errno == ERANGE || val > DBL_MAX || val < -DBL_MAX) { result.error = "overflow: value exceeds float64 range"; return result; }
    end = skip_trailing_whitespace_parse(end);
    if (*end != '\0') { result.error = invalid_digit_error_parse(*end); return result; }
    result.value = val;
    return result;
}

#define OPAL_INTEGER_RANGE_ERROR "IntegerRangeError"

#define OPAL_DEFINE_SIGNED_TO_SIGNED(NAME, SRC_TYPE, DST_TYPE, RESULT_TYPE, MIN_VALUE, MAX_VALUE) \
RESULT_TYPE NAME(SRC_TYPE value) { \
    RESULT_TYPE result = { 0, NULL }; \
    if (value < (SRC_TYPE)(MIN_VALUE) || value > (SRC_TYPE)(MAX_VALUE)) { \
        result.error = OPAL_INTEGER_RANGE_ERROR; \
        return result; \
    } \
    result.value = (DST_TYPE)value; \
    return result; \
}

#define OPAL_DEFINE_UNSIGNED_TO_UNSIGNED(NAME, SRC_TYPE, DST_TYPE, RESULT_TYPE, MAX_VALUE) \
RESULT_TYPE NAME(SRC_TYPE value) { \
    RESULT_TYPE result = { 0, NULL }; \
    if (value > (SRC_TYPE)(MAX_VALUE)) { \
        result.error = OPAL_INTEGER_RANGE_ERROR; \
        return result; \
    } \
    result.value = (DST_TYPE)value; \
    return result; \
}

#define OPAL_DEFINE_SIGNED_TO_UNSIGNED(NAME, SRC_TYPE, DST_TYPE, RESULT_TYPE, MAX_VALUE) \
RESULT_TYPE NAME(SRC_TYPE value) { \
    RESULT_TYPE result = { 0, NULL }; \
    if (value < 0) { \
        result.error = OPAL_INTEGER_RANGE_ERROR; \
        return result; \
    } \
    uint64_t unsigned_value = (uint64_t)value; \
    if (unsigned_value > (uint64_t)(MAX_VALUE)) { \
        result.error = OPAL_INTEGER_RANGE_ERROR; \
        return result; \
    } \
    result.value = (DST_TYPE)value; \
    return result; \
}

#define OPAL_DEFINE_UNSIGNED_TO_SIGNED(NAME, SRC_TYPE, DST_TYPE, RESULT_TYPE, MAX_VALUE) \
RESULT_TYPE NAME(SRC_TYPE value) { \
    RESULT_TYPE result = { 0, NULL }; \
    if (value > (SRC_TYPE)(MAX_VALUE)) { \
        result.error = OPAL_INTEGER_RANGE_ERROR; \
        return result; \
    } \
    result.value = (DST_TYPE)value; \
    return result; \
}

#define OPAL_DEFINE_FLOAT_TO_SIGNED(NAME, SRC_TYPE, DST_TYPE, RESULT_TYPE, LOWER_BOUND, UPPER_BOUND) \
RESULT_TYPE NAME(SRC_TYPE value) { \
    RESULT_TYPE result = { 0, NULL }; \
    long double wide_value = (long double)value; \
    if (!(value == value) || wide_value < (LOWER_BOUND) || wide_value >= (UPPER_BOUND)) { \
        result.error = OPAL_INTEGER_RANGE_ERROR; \
        return result; \
    } \
    DST_TYPE converted = (DST_TYPE)value; \
    if ((long double)converted != wide_value) { \
        result.error = OPAL_INTEGER_RANGE_ERROR; \
        return result; \
    } \
    result.value = converted; \
    return result; \
}

#define OPAL_DEFINE_FLOAT_TO_UNSIGNED(NAME, SRC_TYPE, DST_TYPE, RESULT_TYPE, UPPER_BOUND) \
RESULT_TYPE NAME(SRC_TYPE value) { \
    RESULT_TYPE result = { 0, NULL }; \
    long double wide_value = (long double)value; \
    if (!(value == value) || wide_value < 0.0L || wide_value >= (UPPER_BOUND)) { \
        result.error = OPAL_INTEGER_RANGE_ERROR; \
        return result; \
    } \
    DST_TYPE converted = (DST_TYPE)value; \
    if ((long double)converted != wide_value) { \
        result.error = OPAL_INTEGER_RANGE_ERROR; \
        return result; \
    } \
    result.value = converted; \
    return result; \
}

static uint64_t opal_signed_magnitude_to_u64(int64_t value) {
    if (value >= 0) {
        return (uint64_t)value;
    }
    return (uint64_t)(-(value + 1)) + 1U;
}

static int opal_u64_bit_width(uint64_t value) {
    int width = 0;
    while (value != 0) {
        ++width;
        value >>= 1;
    }
    return width;
}

static int opal_u64_trailing_zeros(uint64_t value) {
    int zeros = 0;
    while ((value & 1U) == 0U) {
        ++zeros;
        value >>= 1;
    }
    return zeros;
}

static int opal_u64_exact_in_binary_float(uint64_t value, int significand_bits) {
    if (value == 0) {
        return 1;
    }
    int significant_bits = opal_u64_bit_width(value) - opal_u64_trailing_zeros(value);
    return significant_bits <= significand_bits;
}

#define OPAL_DEFINE_SIGNED_INT_TO_FLOAT(NAME, SRC_TYPE, DST_TYPE, RESULT_TYPE, SIGNIFICAND_BITS) \
RESULT_TYPE NAME(SRC_TYPE value) { \
    RESULT_TYPE result = { 0, NULL }; \
    if (!opal_u64_exact_in_binary_float(opal_signed_magnitude_to_u64((int64_t)value), SIGNIFICAND_BITS)) { \
        result.error = OPAL_INTEGER_RANGE_ERROR; \
        return result; \
    } \
    result.value = (DST_TYPE)value; \
    return result; \
}

#define OPAL_DEFINE_UNSIGNED_INT_TO_FLOAT(NAME, SRC_TYPE, DST_TYPE, RESULT_TYPE, SIGNIFICAND_BITS) \
RESULT_TYPE NAME(SRC_TYPE value) { \
    RESULT_TYPE result = { 0, NULL }; \
    if (!opal_u64_exact_in_binary_float((uint64_t)value, SIGNIFICAND_BITS)) { \
        result.error = OPAL_INTEGER_RANGE_ERROR; \
        return result; \
    } \
    result.value = (DST_TYPE)value; \
    return result; \
}

OPAL_DEFINE_SIGNED_TO_SIGNED(int16_to_int8, int16_t, int8_t, ParseResultI8, INT8_MIN, INT8_MAX)
OPAL_DEFINE_SIGNED_TO_SIGNED(int32_to_int8, int32_t, int8_t, ParseResultI8, INT8_MIN, INT8_MAX)
OPAL_DEFINE_SIGNED_TO_SIGNED(int32_to_int16, int32_t, int16_t, ParseResultI16, INT16_MIN, INT16_MAX)
OPAL_DEFINE_SIGNED_TO_SIGNED(int64_to_int8, int64_t, int8_t, ParseResultI8, INT8_MIN, INT8_MAX)
OPAL_DEFINE_SIGNED_TO_SIGNED(int64_to_int16, int64_t, int16_t, ParseResultI16, INT16_MIN, INT16_MAX)
OPAL_DEFINE_SIGNED_TO_SIGNED(int64_to_int32, int64_t, int32_t, ParseResultI32, INT32_MIN, INT32_MAX)

OPAL_DEFINE_UNSIGNED_TO_UNSIGNED(uint16_to_uint8, uint16_t, uint8_t, ParseResultU8, UINT8_MAX)
OPAL_DEFINE_UNSIGNED_TO_UNSIGNED(uint32_to_uint8, uint32_t, uint8_t, ParseResultU8, UINT8_MAX)
OPAL_DEFINE_UNSIGNED_TO_UNSIGNED(uint32_to_uint16, uint32_t, uint16_t, ParseResultU16, UINT16_MAX)
OPAL_DEFINE_UNSIGNED_TO_UNSIGNED(uint64_to_uint8, uint64_t, uint8_t, ParseResultU8, UINT8_MAX)
OPAL_DEFINE_UNSIGNED_TO_UNSIGNED(uint64_to_uint16, uint64_t, uint16_t, ParseResultU16, UINT16_MAX)
OPAL_DEFINE_UNSIGNED_TO_UNSIGNED(uint64_to_uint32, uint64_t, uint32_t, ParseResultU32, UINT32_MAX)

OPAL_DEFINE_SIGNED_TO_UNSIGNED(int8_to_uint8, int8_t, uint8_t, ParseResultU8, UINT8_MAX)
OPAL_DEFINE_SIGNED_TO_UNSIGNED(int8_to_uint16, int8_t, uint16_t, ParseResultU16, UINT16_MAX)
OPAL_DEFINE_SIGNED_TO_UNSIGNED(int8_to_uint32, int8_t, uint32_t, ParseResultU32, UINT32_MAX)
OPAL_DEFINE_SIGNED_TO_UNSIGNED(int8_to_uint64, int8_t, uint64_t, ParseResultU64, UINT64_MAX)
OPAL_DEFINE_SIGNED_TO_UNSIGNED(int16_to_uint8, int16_t, uint8_t, ParseResultU8, UINT8_MAX)
OPAL_DEFINE_SIGNED_TO_UNSIGNED(int16_to_uint16, int16_t, uint16_t, ParseResultU16, UINT16_MAX)
OPAL_DEFINE_SIGNED_TO_UNSIGNED(int16_to_uint32, int16_t, uint32_t, ParseResultU32, UINT32_MAX)
OPAL_DEFINE_SIGNED_TO_UNSIGNED(int16_to_uint64, int16_t, uint64_t, ParseResultU64, UINT64_MAX)
OPAL_DEFINE_SIGNED_TO_UNSIGNED(int32_to_uint8, int32_t, uint8_t, ParseResultU8, UINT8_MAX)
OPAL_DEFINE_SIGNED_TO_UNSIGNED(int32_to_uint16, int32_t, uint16_t, ParseResultU16, UINT16_MAX)
OPAL_DEFINE_SIGNED_TO_UNSIGNED(int32_to_uint32, int32_t, uint32_t, ParseResultU32, UINT32_MAX)
OPAL_DEFINE_SIGNED_TO_UNSIGNED(int32_to_uint64, int32_t, uint64_t, ParseResultU64, UINT64_MAX)
OPAL_DEFINE_SIGNED_TO_UNSIGNED(int64_to_uint8, int64_t, uint8_t, ParseResultU8, UINT8_MAX)
OPAL_DEFINE_SIGNED_TO_UNSIGNED(int64_to_uint16, int64_t, uint16_t, ParseResultU16, UINT16_MAX)
OPAL_DEFINE_SIGNED_TO_UNSIGNED(int64_to_uint32, int64_t, uint32_t, ParseResultU32, UINT32_MAX)
OPAL_DEFINE_SIGNED_TO_UNSIGNED(int64_to_uint64, int64_t, uint64_t, ParseResultU64, UINT64_MAX)

OPAL_DEFINE_UNSIGNED_TO_SIGNED(uint8_to_int8, uint8_t, int8_t, ParseResultI8, INT8_MAX)
OPAL_DEFINE_UNSIGNED_TO_SIGNED(uint16_to_int8, uint16_t, int8_t, ParseResultI8, INT8_MAX)
OPAL_DEFINE_UNSIGNED_TO_SIGNED(uint16_to_int16, uint16_t, int16_t, ParseResultI16, INT16_MAX)
OPAL_DEFINE_UNSIGNED_TO_SIGNED(uint32_to_int8, uint32_t, int8_t, ParseResultI8, INT8_MAX)
OPAL_DEFINE_UNSIGNED_TO_SIGNED(uint32_to_int16, uint32_t, int16_t, ParseResultI16, INT16_MAX)
OPAL_DEFINE_UNSIGNED_TO_SIGNED(uint32_to_int32, uint32_t, int32_t, ParseResultI32, INT32_MAX)
OPAL_DEFINE_UNSIGNED_TO_SIGNED(uint64_to_int8, uint64_t, int8_t, ParseResultI8, INT8_MAX)
OPAL_DEFINE_UNSIGNED_TO_SIGNED(uint64_to_int16, uint64_t, int16_t, ParseResultI16, INT16_MAX)
OPAL_DEFINE_UNSIGNED_TO_SIGNED(uint64_to_int32, uint64_t, int32_t, ParseResultI32, INT32_MAX)
OPAL_DEFINE_UNSIGNED_TO_SIGNED(uint64_to_int64, uint64_t, int64_t, ParseResultI64, INT64_MAX)

OPAL_DEFINE_FLOAT_TO_SIGNED(float32_to_int8, float, int8_t, ParseResultI8, -128.0L, 128.0L)
OPAL_DEFINE_FLOAT_TO_SIGNED(float32_to_int16, float, int16_t, ParseResultI16, -32768.0L, 32768.0L)
OPAL_DEFINE_FLOAT_TO_SIGNED(float32_to_int32, float, int32_t, ParseResultI32, -2147483648.0L, 2147483648.0L)
OPAL_DEFINE_FLOAT_TO_SIGNED(float32_to_int64, float, int64_t, ParseResultI64, -9223372036854775808.0L, 9223372036854775808.0L)
OPAL_DEFINE_FLOAT_TO_UNSIGNED(float32_to_uint8, float, uint8_t, ParseResultU8, 256.0L)
OPAL_DEFINE_FLOAT_TO_UNSIGNED(float32_to_uint16, float, uint16_t, ParseResultU16, 65536.0L)
OPAL_DEFINE_FLOAT_TO_UNSIGNED(float32_to_uint32, float, uint32_t, ParseResultU32, 4294967296.0L)
OPAL_DEFINE_FLOAT_TO_UNSIGNED(float32_to_uint64, float, uint64_t, ParseResultU64, 18446744073709551616.0L)
OPAL_DEFINE_FLOAT_TO_SIGNED(float64_to_int8, double, int8_t, ParseResultI8, -128.0L, 128.0L)
OPAL_DEFINE_FLOAT_TO_SIGNED(float64_to_int16, double, int16_t, ParseResultI16, -32768.0L, 32768.0L)
OPAL_DEFINE_FLOAT_TO_SIGNED(float64_to_int32, double, int32_t, ParseResultI32, -2147483648.0L, 2147483648.0L)
OPAL_DEFINE_FLOAT_TO_SIGNED(float64_to_int64, double, int64_t, ParseResultI64, -9223372036854775808.0L, 9223372036854775808.0L)
OPAL_DEFINE_FLOAT_TO_UNSIGNED(float64_to_uint8, double, uint8_t, ParseResultU8, 256.0L)
OPAL_DEFINE_FLOAT_TO_UNSIGNED(float64_to_uint16, double, uint16_t, ParseResultU16, 65536.0L)
OPAL_DEFINE_FLOAT_TO_UNSIGNED(float64_to_uint32, double, uint32_t, ParseResultU32, 4294967296.0L)
OPAL_DEFINE_FLOAT_TO_UNSIGNED(float64_to_uint64, double, uint64_t, ParseResultU64, 18446744073709551616.0L)

OPAL_DEFINE_SIGNED_INT_TO_FLOAT(int32_to_float32, int32_t, float, ParseResultF32, 24)
OPAL_DEFINE_UNSIGNED_INT_TO_FLOAT(uint32_to_float32, uint32_t, float, ParseResultF32, 24)
OPAL_DEFINE_SIGNED_INT_TO_FLOAT(int64_to_float32, int64_t, float, ParseResultF32, 24)
OPAL_DEFINE_UNSIGNED_INT_TO_FLOAT(uint64_to_float32, uint64_t, float, ParseResultF32, 24)
OPAL_DEFINE_SIGNED_INT_TO_FLOAT(int64_to_float64, int64_t, double, ParseResultF64, 53)
OPAL_DEFINE_UNSIGNED_INT_TO_FLOAT(uint64_to_float64, uint64_t, double, ParseResultF64, 53)

ParseResultF32 float64_to_float32(double value) {
    ParseResultF32 result = { 0, NULL };
    if (!(value == value) || value > FLT_MAX || value < -FLT_MAX) {
        result.error = OPAL_INTEGER_RANGE_ERROR;
        return result;
    }
    float converted = (float)value;
    if ((double)converted != value) {
        result.error = OPAL_INTEGER_RANGE_ERROR;
        return result;
    }
    result.value = converted;
    return result;
}
