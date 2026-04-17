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
typedef struct { int64_t value;  const char* error; } ParseResultI64;
typedef struct { uint8_t value;  const char* error; } ParseResultU8;
typedef struct { uint16_t value; const char* error; } ParseResultU16;
typedef struct { uint32_t value; const char* error; } ParseResultU32;
typedef struct { uint64_t value; const char* error; } ParseResultU64;
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
    static _Thread_local char msg[64];
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
