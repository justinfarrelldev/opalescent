#include "opal_portability.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include <inttypes.h>

char* int8_to_string(int8_t value) {
    int len = snprintf(NULL, 0, "%d", (int)value);
    char* buf = (char*)malloc(len + 1);
    if (!buf) { fprintf(stderr, "Runtime error: out of memory\n"); exit(1); }
    snprintf(buf, len + 1, "%d", (int)value);
    return buf;
}

char* int16_to_string(int16_t value) {
    int len = snprintf(NULL, 0, "%d", (int)value);
    char* buf = (char*)malloc(len + 1);
    if (!buf) { fprintf(stderr, "Runtime error: out of memory\n"); exit(1); }
    snprintf(buf, len + 1, "%d", (int)value);
    return buf;
}

char* int32_to_string(int32_t value) {
    int len = snprintf(NULL, 0, "%d", value);
    char* buf = (char*)malloc(len + 1);
    if (!buf) { fprintf(stderr, "Runtime error: out of memory\n"); exit(1); }
    snprintf(buf, len + 1, "%d", value);
    return buf;
}

char* int64_to_string(int64_t value) {
    int len = snprintf(NULL, 0, "%" PRId64, value);
    char* buf = (char*)malloc(len + 1);
    if (!buf) { fprintf(stderr, "Runtime error: out of memory\n"); exit(1); }
    snprintf(buf, len + 1, "%" PRId64, value);
    return buf;
}

char* uint8_to_string(uint8_t value) {
    int len = snprintf(NULL, 0, "%u", (unsigned)value);
    char* buf = (char*)malloc(len + 1);
    if (!buf) { fprintf(stderr, "Runtime error: out of memory\n"); exit(1); }
    snprintf(buf, len + 1, "%u", (unsigned)value);
    return buf;
}

char* uint16_to_string(uint16_t value) {
    int len = snprintf(NULL, 0, "%u", (unsigned)value);
    char* buf = (char*)malloc(len + 1);
    if (!buf) { fprintf(stderr, "Runtime error: out of memory\n"); exit(1); }
    snprintf(buf, len + 1, "%u", (unsigned)value);
    return buf;
}

char* uint32_to_string(uint32_t value) {
    int len = snprintf(NULL, 0, "%u", value);
    char* buf = (char*)malloc(len + 1);
    if (!buf) { fprintf(stderr, "Runtime error: out of memory\n"); exit(1); }
    snprintf(buf, len + 1, "%u", value);
    return buf;
}

char* uint64_to_string(uint64_t value) {
    int len = snprintf(NULL, 0, "%" PRIu64, value);
    char* buf = (char*)malloc(len + 1);
    if (!buf) { fprintf(stderr, "Runtime error: out of memory\n"); exit(1); }
    snprintf(buf, len + 1, "%" PRIu64, value);
    return buf;
}

char* float32_to_string(float value) {
    int len = snprintf(NULL, 0, "%g", (double)value);
    char* buf = (char*)malloc(len + 1);
    if (!buf) { fprintf(stderr, "Runtime error: out of memory\n"); exit(1); }
    snprintf(buf, len + 1, "%g", (double)value);
    return buf;
}

char* float64_to_string(double value) {
    int len = snprintf(NULL, 0, "%g", value);
    char* buf = (char*)malloc(len + 1);
    if (!buf) { fprintf(stderr, "Runtime error: out of memory\n"); exit(1); }
    snprintf(buf, len + 1, "%g", value);
    return buf;
}

char* bool_to_string(int8_t value) {
    char* result = opal_strdup(value ? "true" : "false");
    if (!result) { fprintf(stderr, "Runtime error: out of memory\n"); exit(1); }
    return result;
}

int64_t string_length(const char* value) {
    if (!value) { fprintf(stderr, "Runtime error: string_length called with NULL string pointer\n"); exit(1); }
    int64_t length = 0;
    const unsigned char* cursor = (const unsigned char*)value;
    while (*cursor != '\0') {
        if ((*cursor & 0xC0u) != 0x80u) {
            length++;
        }
        cursor++;
    }
    return length;
}

int64_t array_length(const void* array, int64_t length) {
    if (!array && length != 0) { fprintf(stderr, "Runtime error: array_length called with NULL array pointer and non-zero length\n"); exit(1); }
    if (length < 0) { fprintf(stderr, "Runtime error: array_length called with negative length\n"); exit(1); }
    return length;
}
