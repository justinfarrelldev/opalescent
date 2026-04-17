#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>
#include <stdint.h>
#include <inttypes.h>
#include <errno.h>
#include <limits.h>
#include <float.h>

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

static const char* invalid_digit_error(char ch) {
    static char msg[64];
    snprintf(msg, sizeof(msg), "invalid digit '%c' in input", ch);
    return msg;
}

void opal_runtime_error(const char* message) {
    fprintf(stderr, "%s\n", message);
    exit(1);
}

static const char* skip_leading_whitespace(const char* s) {
    while (*s == ' ' || *s == '\t' || *s == '\n' || *s == '\r' || *s == '\f' || *s == '\v') {
        ++s;
    }
    return s;
}

static char* skip_trailing_whitespace(char* s) {
    while (*s == ' ' || *s == '\t' || *s == '\n' || *s == '\r' || *s == '\f' || *s == '\v') {
        ++s;
    }
    return s;
}

char* take_input(void) {
    static char buf[1024];
    if (fgets(buf, sizeof(buf), stdin) == NULL) {
        return strdup("");
    }
    size_t len = strlen(buf);
    if (len > 0 && buf[len - 1] == '\n') {
        buf[len - 1] = '\0';
    }
    return strdup(buf);
}

void print_string(const char* s) {
    puts(s);
}

void print_int8(int8_t n) {
    printf("%d\n", (int)n);
}

void print_int16(int16_t n) {
    printf("%d\n", (int)n);
}

void print_int32(int32_t n) {
    printf("%d\n", (int)n);
}

void print_int64(int64_t n) {
    printf("%lld\n", (long long)n);
}

void print_uint8(uint8_t n) {
    printf("%u\n", (unsigned)n);
}

void print_uint16(uint16_t n) {
    printf("%u\n", (unsigned)n);
}

void print_uint32(uint32_t n) {
    printf("%u\n", (unsigned int)n);
}

void print_uint64(uint64_t n) {
    printf("%llu\n", (unsigned long long)n);
}

void print_float32(float n) {
    printf("%.6f\n", (double)n);
}

void print_float64(double n) {
    printf("%.6f\n", n);
}

static void seed_rand_once(void) {
    static int seeded = 0;
    if (!seeded) {
        srand((unsigned int)time(NULL));
        seeded = 1;
    }
}

int8_t random_int8(int8_t min, int8_t max) {
    seed_rand_once();
    if (max <= min) return min;
    return (int8_t)(min + (int8_t)(rand() % (int)(max - min + 1)));
}

int16_t random_int16(int16_t min, int16_t max) {
    seed_rand_once();
    if (max <= min) return min;
    return (int16_t)(min + (int16_t)(rand() % (int)(max - min + 1)));
}

int32_t random_int32(int32_t min, int32_t max) {
    seed_rand_once();
    if (max <= min) return min;
    return min + (int32_t)(rand() % (int)(max - min + 1));
}

int64_t random_int64(int64_t min, int64_t max) {
    seed_rand_once();
    if (max <= min) return min;
    return min + (int64_t)(rand() % (int64_t)(max - min + 1));
}

uint8_t random_uint8(uint8_t min, uint8_t max) {
    seed_rand_once();
    if (max <= min) return min;
    return (uint8_t)(min + (uint8_t)((unsigned)rand() % (unsigned)(max - min + 1)));
}

uint16_t random_uint16(uint16_t min, uint16_t max) {
    seed_rand_once();
    if (max <= min) return min;
    return (uint16_t)(min + (uint16_t)((unsigned)rand() % (unsigned)(max - min + 1)));
}

uint32_t random_uint32(uint32_t min, uint32_t max) {
    seed_rand_once();
    if (max <= min) return min;
    return min + (uint32_t)((unsigned)rand() % (unsigned)(max - min + 1));
}

uint64_t random_uint64(uint64_t min, uint64_t max) {
    seed_rand_once();
    if (max <= min) return min;
    return min + (uint64_t)((uint64_t)rand() % (uint64_t)(max - min + 1));
}

ParseResultI8 string_to_int8(const char* s) {
    ParseResultI8 result = { 0, NULL };
    if (s == NULL) {
        result.error = "null input";
        return result;
    }

    const char* p = skip_leading_whitespace(s);
    if (*p == '\0') {
        result.error = "empty input";
        return result;
    }

    errno = 0;
    char* end;
    long long val = strtoll(p, &end, 10);

    if (end == p) {
        result.error = invalid_digit_error(*p);
        return result;
    }

    if (errno == ERANGE || val < INT8_MIN || val > INT8_MAX) {
        result.error = "overflow: value exceeds int8 range";
        return result;
    }

    end = skip_trailing_whitespace(end);
    if (*end != '\0') {
        result.error = invalid_digit_error(*end);
        return result;
    }

    result.value = (int8_t)val;
    return result;
}

ParseResultI16 string_to_int16(const char* s) {
    ParseResultI16 result = { 0, NULL };
    if (s == NULL) {
        result.error = "null input";
        return result;
    }

    const char* p = skip_leading_whitespace(s);
    if (*p == '\0') {
        result.error = "empty input";
        return result;
    }

    errno = 0;
    char* end;
    long long val = strtoll(p, &end, 10);

    if (end == p) {
        result.error = invalid_digit_error(*p);
        return result;
    }

    if (errno == ERANGE || val < INT16_MIN || val > INT16_MAX) {
        result.error = "overflow: value exceeds int16 range";
        return result;
    }

    end = skip_trailing_whitespace(end);
    if (*end != '\0') {
        result.error = invalid_digit_error(*end);
        return result;
    }

    result.value = (int16_t)val;
    return result;
}

ParseResultI32 string_to_int32(const char* s) {
    ParseResultI32 result = { 0, NULL };
    if (s == NULL) {
        result.error = "null input";
        return result;
    }

    const char* p = skip_leading_whitespace(s);
    if (*p == '\0') {
        result.error = "empty input";
        return result;
    }

    errno = 0;
    char* end;
    long long val = strtoll(p, &end, 10);

    if (end == p) {
        result.error = invalid_digit_error(*p);
        return result;
    }

    if (errno == ERANGE || val < INT32_MIN || val > INT32_MAX) {
        result.error = "overflow: value exceeds int32 range";
        return result;
    }

    end = skip_trailing_whitespace(end);
    if (*end != '\0') {
        result.error = invalid_digit_error(*end);
        return result;
    }

    result.value = (int32_t)val;
    return result;
}

ParseResultI64 string_to_int64(const char* s) {
    ParseResultI64 result = { 0, NULL };
    if (s == NULL) {
        result.error = "null input";
        return result;
    }

    const char* p = skip_leading_whitespace(s);
    if (*p == '\0') {
        result.error = "empty input";
        return result;
    }

    errno = 0;
    char* end;
    long long val = strtoll(p, &end, 10);

    if (end == p) {
        result.error = invalid_digit_error(*p);
        return result;
    }

    if (errno == ERANGE) {
        result.error = "overflow: value exceeds int64 range";
        return result;
    }

    end = skip_trailing_whitespace(end);
    if (*end != '\0') {
        result.error = invalid_digit_error(*end);
        return result;
    }

    result.value = (int64_t)val;
    return result;
}

ParseResultU8 string_to_uint8(const char* s) {
    ParseResultU8 result = { 0, NULL };
    if (s == NULL) {
        result.error = "null input";
        return result;
    }

    const char* p = skip_leading_whitespace(s);
    if (*p == '\0') {
        result.error = "empty input";
        return result;
    }

    if (*p == '-') {
        result.error = invalid_digit_error(*p);
        return result;
    }

    errno = 0;
    char* end;
    unsigned long long val = strtoull(p, &end, 10);

    if (end == p) {
        result.error = invalid_digit_error(*p);
        return result;
    }

    if (errno == ERANGE || val > UINT8_MAX) {
        result.error = "overflow: value exceeds uint8 range";
        return result;
    }

    end = skip_trailing_whitespace(end);
    if (*end != '\0') {
        result.error = invalid_digit_error(*end);
        return result;
    }

    result.value = (uint8_t)val;
    return result;
}

ParseResultU16 string_to_uint16(const char* s) {
    ParseResultU16 result = { 0, NULL };
    if (s == NULL) {
        result.error = "null input";
        return result;
    }

    const char* p = skip_leading_whitespace(s);
    if (*p == '\0') {
        result.error = "empty input";
        return result;
    }

    if (*p == '-') {
        result.error = invalid_digit_error(*p);
        return result;
    }

    errno = 0;
    char* end;
    unsigned long long val = strtoull(p, &end, 10);

    if (end == p) {
        result.error = invalid_digit_error(*p);
        return result;
    }

    if (errno == ERANGE || val > UINT16_MAX) {
        result.error = "overflow: value exceeds uint16 range";
        return result;
    }

    end = skip_trailing_whitespace(end);
    if (*end != '\0') {
        result.error = invalid_digit_error(*end);
        return result;
    }

    result.value = (uint16_t)val;
    return result;
}

ParseResultU32 string_to_uint32(const char* s) {
    ParseResultU32 result = { 0, NULL };
    if (s == NULL) {
        result.error = "null input";
        return result;
    }

    const char* p = skip_leading_whitespace(s);
    if (*p == '\0') {
        result.error = "empty input";
        return result;
    }

    if (*p == '-') {
        result.error = invalid_digit_error(*p);
        return result;
    }

    errno = 0;
    char* end;
    unsigned long long val = strtoull(p, &end, 10);

    if (end == p) {
        result.error = invalid_digit_error(*p);
        return result;
    }

    if (errno == ERANGE || val > UINT32_MAX) {
        result.error = "overflow: value exceeds uint32 range";
        return result;
    }

    end = skip_trailing_whitespace(end);
    if (*end != '\0') {
        result.error = invalid_digit_error(*end);
        return result;
    }

    result.value = (uint32_t)val;
    return result;
}

ParseResultU64 string_to_uint64(const char* s) {
    ParseResultU64 result = { 0, NULL };
    if (s == NULL) {
        result.error = "null input";
        return result;
    }

    const char* p = skip_leading_whitespace(s);
    if (*p == '\0') {
        result.error = "empty input";
        return result;
    }

    if (*p == '-') {
        result.error = invalid_digit_error(*p);
        return result;
    }

    errno = 0;
    char* end;
    unsigned long long val = strtoull(p, &end, 10);

    if (end == p) {
        result.error = invalid_digit_error(*p);
        return result;
    }

    if (errno == ERANGE) {
        result.error = "overflow: value exceeds uint64 range";
        return result;
    }

    end = skip_trailing_whitespace(end);
    if (*end != '\0') {
        result.error = invalid_digit_error(*end);
        return result;
    }

    result.value = (uint64_t)val;
    return result;
}

ParseResultF32 string_to_float32(const char* s) {
    ParseResultF32 result = { 0, NULL };
    if (s == NULL) {
        result.error = "null input";
        return result;
    }

    const char* p = skip_leading_whitespace(s);
    if (*p == '\0') {
        result.error = "empty input";
        return result;
    }

    errno = 0;
    char* end;
    float val = strtof(p, &end);

    if (end == p) {
        result.error = invalid_digit_error(*p);
        return result;
    }

    if (errno == ERANGE || val > FLT_MAX || val < -FLT_MAX) {
        result.error = "overflow: value exceeds float32 range";
        return result;
    }

    end = skip_trailing_whitespace(end);
    if (*end != '\0') {
        result.error = invalid_digit_error(*end);
        return result;
    }

    result.value = val;
    return result;
}

ParseResultF64 string_to_float64(const char* s) {
    ParseResultF64 result = { 0, NULL };
    if (s == NULL) {
        result.error = "null input";
        return result;
    }

    const char* p = skip_leading_whitespace(s);
    if (*p == '\0') {
        result.error = "empty input";
        return result;
    }

    errno = 0;
    char* end;
    double val = strtod(p, &end);

    if (end == p) {
        result.error = invalid_digit_error(*p);
        return result;
    }

    if (errno == ERANGE || val > DBL_MAX || val < -DBL_MAX) {
        result.error = "overflow: value exceeds float64 range";
        return result;
    }

    end = skip_trailing_whitespace(end);
    if (*end != '\0') {
        result.error = invalid_digit_error(*end);
        return result;
    }

    result.value = val;
    return result;
}

/* ── Numeric-to-string conversion functions (infallible) ── */

char* int8_to_string(int8_t value) {
    int len = snprintf(NULL, 0, "%d", (int)value);
    char* buf = (char*)malloc(len + 1);
    snprintf(buf, len + 1, "%d", (int)value);
    return buf;
}

char* int16_to_string(int16_t value) {
    int len = snprintf(NULL, 0, "%d", (int)value);
    char* buf = (char*)malloc(len + 1);
    snprintf(buf, len + 1, "%d", (int)value);
    return buf;
}

char* int32_to_string(int32_t value) {
    int len = snprintf(NULL, 0, "%d", value);
    char* buf = (char*)malloc(len + 1);
    snprintf(buf, len + 1, "%d", value);
    return buf;
}

char* int64_to_string(int64_t value) {
    int len = snprintf(NULL, 0, "%" PRId64, value);
    char* buf = (char*)malloc(len + 1);
    snprintf(buf, len + 1, "%" PRId64, value);
    return buf;
}

char* uint8_to_string(uint8_t value) {
    int len = snprintf(NULL, 0, "%u", (unsigned)value);
    char* buf = (char*)malloc(len + 1);
    snprintf(buf, len + 1, "%u", (unsigned)value);
    return buf;
}

char* uint16_to_string(uint16_t value) {
    int len = snprintf(NULL, 0, "%u", (unsigned)value);
    char* buf = (char*)malloc(len + 1);
    snprintf(buf, len + 1, "%u", (unsigned)value);
    return buf;
}

char* uint32_to_string(uint32_t value) {
    int len = snprintf(NULL, 0, "%u", value);
    char* buf = (char*)malloc(len + 1);
    snprintf(buf, len + 1, "%u", value);
    return buf;
}

char* uint64_to_string(uint64_t value) {
    int len = snprintf(NULL, 0, "%" PRIu64, value);
    char* buf = (char*)malloc(len + 1);
    snprintf(buf, len + 1, "%" PRIu64, value);
    return buf;
}

char* float32_to_string(float value) {
    int len = snprintf(NULL, 0, "%g", (double)value);
    char* buf = (char*)malloc(len + 1);
    snprintf(buf, len + 1, "%g", (double)value);
    return buf;
}

char* float64_to_string(double value) {
    int len = snprintf(NULL, 0, "%g", value);
    char* buf = (char*)malloc(len + 1);
    snprintf(buf, len + 1, "%g", value);
    return buf;
}

char* bool_to_string(int8_t value) {
    return strdup(value ? "true" : "false");
}
