#include "opal_portability.h"
#include "opal_rc.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include <inttypes.h>

#if defined(OPAL_ENABLE_INTERNAL_TESTING)
#define malloc(size) opal_test_malloc_for_test(size)
#define calloc(count, size) opal_test_calloc_for_test(count, size)
#define realloc(ptr, size) opal_test_realloc_for_test(ptr, size)
#endif

uint64_t opal_runtime_string_index_span_start = 0u;
uint64_t opal_runtime_string_index_span_len = 0u;
const char* opal_runtime_string_index_source_path = NULL;
const char* opal_runtime_string_index_source_text = NULL;
void opal_array_bounds_error(uint64_t index, uint64_t length);

char* int8_to_string(int8_t value) {
    int len = snprintf(NULL, 0, "%d", (int)value);
    char* buf = (char*)malloc(len + 1);
    if (!buf) { fprintf(stderr, "Runtime error: out of memory\n"); exit(1); }
    opal_rc_debug_note_alloc(OPAL_RC_DEBUG_COUNTER_STRINGS);
    snprintf(buf, len + 1, "%d", (int)value);
    return buf;
}

char* int16_to_string(int16_t value) {
    int len = snprintf(NULL, 0, "%d", (int)value);
    char* buf = (char*)malloc(len + 1);
    if (!buf) { fprintf(stderr, "Runtime error: out of memory\n"); exit(1); }
    opal_rc_debug_note_alloc(OPAL_RC_DEBUG_COUNTER_STRINGS);
    snprintf(buf, len + 1, "%d", (int)value);
    return buf;
}

char* int32_to_string(int32_t value) {
    int len = snprintf(NULL, 0, "%d", value);
    char* buf = (char*)malloc(len + 1);
    if (!buf) { fprintf(stderr, "Runtime error: out of memory\n"); exit(1); }
    opal_rc_debug_note_alloc(OPAL_RC_DEBUG_COUNTER_STRINGS);
    snprintf(buf, len + 1, "%d", value);
    return buf;
}

char* int64_to_string(int64_t value) {
    int len = snprintf(NULL, 0, "%" PRId64, value);
    char* buf = (char*)malloc(len + 1);
    if (!buf) { fprintf(stderr, "Runtime error: out of memory\n"); exit(1); }
    opal_rc_debug_note_alloc(OPAL_RC_DEBUG_COUNTER_STRINGS);
    snprintf(buf, len + 1, "%" PRId64, value);
    return buf;
}

char* uint8_to_string(uint8_t value) {
    int len = snprintf(NULL, 0, "%u", (unsigned)value);
    char* buf = (char*)malloc(len + 1);
    if (!buf) { fprintf(stderr, "Runtime error: out of memory\n"); exit(1); }
    opal_rc_debug_note_alloc(OPAL_RC_DEBUG_COUNTER_STRINGS);
    snprintf(buf, len + 1, "%u", (unsigned)value);
    return buf;
}

char* uint16_to_string(uint16_t value) {
    int len = snprintf(NULL, 0, "%u", (unsigned)value);
    char* buf = (char*)malloc(len + 1);
    if (!buf) { fprintf(stderr, "Runtime error: out of memory\n"); exit(1); }
    opal_rc_debug_note_alloc(OPAL_RC_DEBUG_COUNTER_STRINGS);
    snprintf(buf, len + 1, "%u", (unsigned)value);
    return buf;
}

char* uint32_to_string(uint32_t value) {
    int len = snprintf(NULL, 0, "%u", value);
    char* buf = (char*)malloc(len + 1);
    if (!buf) { fprintf(stderr, "Runtime error: out of memory\n"); exit(1); }
    opal_rc_debug_note_alloc(OPAL_RC_DEBUG_COUNTER_STRINGS);
    snprintf(buf, len + 1, "%u", value);
    return buf;
}

char* uint64_to_string(uint64_t value) {
    int len = snprintf(NULL, 0, "%" PRIu64, value);
    char* buf = (char*)malloc(len + 1);
    if (!buf) { fprintf(stderr, "Runtime error: out of memory\n"); exit(1); }
    opal_rc_debug_note_alloc(OPAL_RC_DEBUG_COUNTER_STRINGS);
    snprintf(buf, len + 1, "%" PRIu64, value);
    return buf;
}

char* float32_to_string(float value) {
    int len = snprintf(NULL, 0, "%g", (double)value);
    char* buf = (char*)malloc(len + 1);
    if (!buf) { fprintf(stderr, "Runtime error: out of memory\n"); exit(1); }
    opal_rc_debug_note_alloc(OPAL_RC_DEBUG_COUNTER_STRINGS);
    snprintf(buf, len + 1, "%g", (double)value);
    return buf;
}

char* float64_to_string(double value) {
    int len = snprintf(NULL, 0, "%g", value);
    char* buf = (char*)malloc(len + 1);
    if (!buf) { fprintf(stderr, "Runtime error: out of memory\n"); exit(1); }
    opal_rc_debug_note_alloc(OPAL_RC_DEBUG_COUNTER_STRINGS);
    snprintf(buf, len + 1, "%g", value);
    return buf;
}

typedef struct OpalStringBuilder {
    char* buffer;
    size_t length;
    size_t capacity;
    int finished;
} OpalStringBuilder;

typedef struct { void* value; const char* error; } StringBuilderVoidResult;
typedef struct { char* value; const char* error; } StringBuilderStringResult;
typedef struct { char* value; int64_t used_cells; const char* error; } FsStringInt64Result;
#ifndef OPAL_PARSE_RESULT_I64_DEFINED
typedef struct { int64_t value; const char* error; } ParseResultI64;
#define OPAL_PARSE_RESULT_I64_DEFINED 1
#endif
#ifndef OPAL_FS_STRING_RESULT_DEFINED
typedef struct { char* value; const char* error; } FsStringResult;
#define OPAL_FS_STRING_RESULT_DEFINED 1
#endif
#ifndef OPAL_FS_STRING_ARRAY_RESULT_DEFINED
typedef struct { char** value; int64_t count; const char* error; } FsStringArrayResult;
#define OPAL_FS_STRING_ARRAY_RESULT_DEFINED 1
#endif
#ifndef OPAL_FS_STRING_RESULT_TYPES_DEFINED
#define OPAL_FS_STRING_RESULT_TYPES_DEFINED 1
#endif

#if defined(OPAL_ENABLE_INTERNAL_TESTING)
void opal_string_builder_set_length_for_test(OpalStringBuilder* builder, size_t length) {
    if (builder) {
        builder->length = length;
    }
}
#endif

typedef struct OpalStringBuilderNode {
    OpalStringBuilder* builder;
    struct OpalStringBuilderNode* next;
} OpalStringBuilderNode;

static OpalStringBuilderNode* OPAL_STRING_BUILDERS = NULL;
static int OPAL_STRING_BUILDERS_CLEANUP_REGISTERED = 0;

static char* opal_string_duplicate_or_die(const char* source) {
    char* copy = opal_strdup(source ? source : "");
    if (!copy) { fprintf(stderr, "Runtime error: out of memory\n"); exit(1); }
    opal_rc_debug_note_alloc(OPAL_RC_DEBUG_COUNTER_STRINGS);
    return copy;
}

static char* opal_string_duplicate(const char* source) {
    char* copy = opal_strdup(source ? source : "");
    if (!copy) {
        return NULL;
    }
    opal_rc_debug_note_alloc(OPAL_RC_DEBUG_COUNTER_STRINGS);
    return copy;
}

static void opal_string_builder_cleanup_all(void) {
    OpalStringBuilderNode* node = OPAL_STRING_BUILDERS;
    while (node) {
        OpalStringBuilderNode* next = node->next;
        if (node->builder) {
            if (node->builder->buffer) {
                opal_rc_debug_note_free(OPAL_RC_DEBUG_COUNTER_STRINGS);
                free(node->builder->buffer);
                node->builder->buffer = NULL;
            }
            opal_rc_debug_note_free(OPAL_RC_DEBUG_COUNTER_BUILDERS);
            free(node->builder);
        }
        free(node);
        node = next;
    }
    OPAL_STRING_BUILDERS = NULL;
}

static void opal_string_builder_register_for_cleanup(OpalStringBuilder* builder) {
    OpalStringBuilderNode* node = (OpalStringBuilderNode*)malloc(sizeof(OpalStringBuilderNode));
    if (!node) { fprintf(stderr, "Runtime error: out of memory\n"); exit(1); }
    node->builder = builder;
    node->next = OPAL_STRING_BUILDERS;
    OPAL_STRING_BUILDERS = node;
    if (!OPAL_STRING_BUILDERS_CLEANUP_REGISTERED) {
        if (atexit(opal_string_builder_cleanup_all) != 0) {
            fprintf(stderr, "Runtime error: failed to register string builder cleanup\n");
            exit(1);
        }
        OPAL_STRING_BUILDERS_CLEANUP_REGISTERED = 1;
    }
}

static const char* string_builder_ensure_capacity(OpalStringBuilder* builder, size_t required) {
    if (required <= builder->capacity) {
        return NULL;
    }

    size_t new_capacity = builder->capacity == 0 ? 16u : builder->capacity;
    while (new_capacity < required) {
        if (new_capacity > (SIZE_MAX / 2u)) {
            new_capacity = required;
            break;
        }
        new_capacity *= 2u;
    }

    char* resized = (char*)realloc(builder->buffer, new_capacity);
    if (!resized) {
        return "AllocationFailureError";
    }

    builder->buffer = resized;
    builder->capacity = new_capacity;
    return NULL;
}

char* bool_to_string(int8_t value) {
    char* result = opal_strdup(value ? "true" : "false");
    if (!result) { fprintf(stderr, "Runtime error: out of memory\n"); exit(1); }
    return result;
}

static int opal_utf8_is_scalar_start(unsigned char byte) {
    return (byte & 0xC0u) != 0x80u;
}

static int64_t opal_utf8_scalar_index_from_ptr(const char* value, const char* target) {
    int64_t index = 0;
    const unsigned char* cursor = (const unsigned char*)value;
    const unsigned char* end = (const unsigned char*)target;
    while (cursor < end && *cursor != '\0') {
        if (opal_utf8_is_scalar_start(*cursor)) {
            index++;
        }
        cursor++;
    }
    return index;
}

static int opal_utf8_decode_scalar(const unsigned char* cursor, uint32_t* codepoint, size_t* scalar_len) {
    if (!cursor || !codepoint || !scalar_len) {
        return 0;
    }
    if (cursor[0] < 0x80u) {
        *codepoint = cursor[0];
        *scalar_len = 1u;
        return 1;
    }
    if ((cursor[0] & 0xE0u) == 0xC0u) {
        *codepoint = ((uint32_t)(cursor[0] & 0x1Fu) << 6) | (uint32_t)(cursor[1] & 0x3Fu);
        *scalar_len = 2u;
        return 1;
    }
    if ((cursor[0] & 0xF0u) == 0xE0u) {
        *codepoint = ((uint32_t)(cursor[0] & 0x0Fu) << 12)
                   | ((uint32_t)(cursor[1] & 0x3Fu) << 6)
                   | (uint32_t)(cursor[2] & 0x3Fu);
        *scalar_len = 3u;
        return 1;
    }
    if ((cursor[0] & 0xF8u) == 0xF0u) {
        *codepoint = ((uint32_t)(cursor[0] & 0x07u) << 18)
                   | ((uint32_t)(cursor[1] & 0x3Fu) << 12)
                   | ((uint32_t)(cursor[2] & 0x3Fu) << 6)
                   | (uint32_t)(cursor[3] & 0x3Fu);
        *scalar_len = 4u;
        return 1;
    }
    return 0;
}

static int opal_is_unicode_white_space(uint32_t codepoint) {
    switch (codepoint) {
        case 0x0009u:
        case 0x000Au:
        case 0x000Bu:
        case 0x000Cu:
        case 0x000Du:
        case 0x0020u:
        case 0x0085u:
        case 0x00A0u:
        case 0x1680u:
        case 0x2000u:
        case 0x2001u:
        case 0x2002u:
        case 0x2003u:
        case 0x2004u:
        case 0x2005u:
        case 0x2006u:
        case 0x2007u:
        case 0x2008u:
        case 0x2009u:
        case 0x200Au:
        case 0x2028u:
        case 0x2029u:
        case 0x202Fu:
        case 0x205Fu:
        case 0x3000u:
            return 1;
        default:
            return 0;
    }
}

int64_t string_length(const char* value) {
    if (!value) { fprintf(stderr, "Runtime error: string_length called with NULL string pointer\n"); exit(1); }
    int64_t length = 0;
    const unsigned char* cursor = (const unsigned char*)value;
    while (*cursor != '\0') {
        if (opal_utf8_is_scalar_start(*cursor)) {
            length++;
        }
        cursor++;
    }
    return length;
}

char* string_index(const char* value, int64_t index) {
    if (!value) { fprintf(stderr, "Runtime error: string_index called with NULL string pointer\n"); exit(1); }

    int64_t length = string_length(value);
    if (index < 0 || index >= length) {
        opal_array_bounds_error(index < 0 ? 0u : (uint64_t)index, (uint64_t)length);
    }

    const unsigned char* cursor = (const unsigned char*)value;
    const unsigned char* start = NULL;
    const unsigned char* end = NULL;
    int64_t current_index = 0;
    while (*cursor != '\0') {
        if ((*cursor & 0xC0u) != 0x80u) {
            if (current_index == index) {
                start = cursor;
                end = cursor + 1;
                while (*end != '\0' && (*end & 0xC0u) == 0x80u) {
                    end++;
                }
                break;
            }
            current_index++;
        }
        cursor++;
    }

    if (!start || !end) {
        opal_array_bounds_error((uint64_t)index, (uint64_t)length);
    }

    size_t scalar_length = (size_t)(end - start);
    char* result = (char*)malloc(scalar_length + 1u);
    if (!result) { fprintf(stderr, "Runtime error: out of memory\n"); exit(1); }
    opal_rc_debug_note_alloc(OPAL_RC_DEBUG_COUNTER_STRINGS);
    memcpy(result, start, scalar_length);
    result[scalar_length] = '\0';
    return result;
}

int64_t string_find_index_or(const char* value, const char* search_text, int64_t fallback_index) {
    if (!value) { fprintf(stderr, "Runtime error: string_find_index_or called with NULL string pointer\n"); exit(1); }
    if (!search_text || search_text[0] == '\0') {
        return fallback_index;
    }

    const size_t needle_length = strlen(search_text);
    const unsigned char* cursor = (const unsigned char*)value;
    while (*cursor != '\0') {
        if (opal_utf8_is_scalar_start(*cursor)
            && strncmp((const char*)cursor, search_text, needle_length) == 0) {
            return opal_utf8_scalar_index_from_ptr(value, (const char*)cursor);
        }
        cursor++;
    }

    return fallback_index;
}

ParseResultI64 string_find_last_index_of_text(const char* value, const char* search_text) {
    ParseResultI64 result = { 0, NULL };
    if (!value) { fprintf(stderr, "Runtime error: string_find_last_index_of_text called with NULL string pointer\n"); exit(1); }
    if (!search_text || search_text[0] == '\0') {
        result.error = "StringEmptySearchTextError";
        return result;
    }

    const size_t needle_length = strlen(search_text);
    const unsigned char* cursor = (const unsigned char*)value;
    const char* last_match = NULL;
    while (*cursor != '\0') {
        if (opal_utf8_is_scalar_start(*cursor)
            && strncmp((const char*)cursor, search_text, needle_length) == 0) {
            last_match = (const char*)cursor;
        }
        cursor++;
    }

    if (!last_match) {
        result.error = "StringPatternNotFoundError";
        return result;
    }

    result.value = opal_utf8_scalar_index_from_ptr(value, last_match);
    return result;
}

FsStringArrayResult string_split_lines(const char* value) {
    FsStringArrayResult r;
    r.value = NULL;
    r.count = 0;
    r.error = NULL;

    if (!value) { fprintf(stderr, "Runtime error: string_split_lines called with NULL string pointer\n"); exit(1); }
    if (value[0] == '\0') {
        return r;
    }

    size_t length = strlen(value);
    size_t line_count = 0u;
    size_t start = 0u;
    size_t index = 0u;
    while (index < length) {
        if (value[index] == '\n') {
            line_count++;
            index++;
            start = index;
            continue;
        }
        if (value[index] == '\r') {
            line_count++;
            index++;
            if (index < length && value[index] == '\n') {
                index++;
            }
            start = index;
            continue;
        }
        index++;
    }
    if (start < length) {
        line_count++;
    }
    if (line_count == 0u) {
        return r;
    }
    if (line_count > (size_t)INT64_MAX) {
        r.error = "AllocationFailureError";
        return r;
    }

    char** lines = (char**)calloc(line_count, sizeof(char*));
    if (!lines) {
        r.error = "AllocationFailureError";
        return r;
    }

    size_t out = 0u;
    start = 0u;
    index = 0u;
    while (index < length) {
        if (value[index] == '\n' || value[index] == '\r') {
            size_t segment_len = index - start;
            char* segment = (char*)malloc(segment_len + 1u);
            if (!segment) {
                for (size_t i = 0; i < out; i++) {
                    opal_rc_debug_note_free(OPAL_RC_DEBUG_COUNTER_STRINGS);
                    free(lines[i]);
                }
                free(lines);
                r.error = "AllocationFailureError";
                return r;
            }
            memcpy(segment, value + start, segment_len);
            segment[segment_len] = '\0';
            opal_rc_debug_note_alloc(OPAL_RC_DEBUG_COUNTER_STRINGS);
            lines[out++] = segment;
            index++;
            if (value[index - 1] == '\r' && index < length && value[index] == '\n') {
                index++;
            }
            start = index;
            continue;
        }
        index++;
    }
    if (start < length) {
        size_t segment_len = length - start;
        char* segment = (char*)malloc(segment_len + 1u);
        if (!segment) {
            for (size_t i = 0; i < out; i++) {
                opal_rc_debug_note_free(OPAL_RC_DEBUG_COUNTER_STRINGS);
                free(lines[i]);
            }
            free(lines);
            r.error = "AllocationFailureError";
            return r;
        }
        memcpy(segment, value + start, segment_len);
        segment[segment_len] = '\0';
        opal_rc_debug_note_alloc(OPAL_RC_DEBUG_COUNTER_STRINGS);
        lines[out++] = segment;
    }

    r.value = lines;
    r.count = (int64_t)out;
    return r;
}

int8_t string_is_blank(const char* value) {
    if (!value) { fprintf(stderr, "Runtime error: string_is_blank called with NULL string pointer\n"); exit(1); }
    if (value[0] == '\0') {
        return 1;
    }
    const unsigned char* cursor = (const unsigned char*)value;
    while (*cursor != '\0') {
        uint32_t codepoint = 0u;
        size_t scalar_len = 0u;
        if (!opal_utf8_decode_scalar(cursor, &codepoint, &scalar_len)) {
            return 0;
        }
        if (!opal_is_unicode_white_space(codepoint)) {
            return 0;
        }
        cursor += scalar_len;
    }
    return 1;
}

FsStringResult string_trim_whitespace(const char* value) {
    FsStringResult r;
    r.value = NULL;
    r.error = NULL;

    if (!value) { fprintf(stderr, "Runtime error: string_trim_whitespace called with NULL string pointer\n"); exit(1); }
    if (value[0] == '\0') {
        r.value = opal_string_duplicate("");
        if (!r.value) {
            r.error = "AllocationFailureError";
        }
        return r;
    }

    const unsigned char* cursor = (const unsigned char*)value;
    const char* start = NULL;
    const char* end = NULL;
    while (*cursor != '\0') {
        uint32_t codepoint = 0u;
        size_t scalar_len = 0u;
        if (!opal_utf8_decode_scalar(cursor, &codepoint, &scalar_len)) {
            r.value = opal_string_duplicate((const char*)cursor);
            if (!r.value) {
                r.error = "AllocationFailureError";
            }
            return r;
        }
        if (!opal_is_unicode_white_space(codepoint)) {
            if (!start) {
                start = (const char*)cursor;
            }
            end = (const char*)cursor + scalar_len;
        }
        cursor += scalar_len;
    }

    if (!start) {
        r.value = opal_string_duplicate("");
        if (!r.value) {
            r.error = "AllocationFailureError";
        }
        return r;
    }

    size_t trimmed_len = (size_t)(end - start);
    char* result = (char*)malloc(trimmed_len + 1u);
    if (!result) {
        r.error = "AllocationFailureError";
        return r;
    }
    memcpy(result, start, trimmed_len);
    result[trimmed_len] = '\0';
    opal_rc_debug_note_alloc(OPAL_RC_DEBUG_COUNTER_STRINGS);
    r.value = result;
    return r;
}

FsStringResult string_take_prefix(const char* value, int64_t count) {
    FsStringResult r;
    r.value = NULL;
    r.error = NULL;
    if (!value) { fprintf(stderr, "Runtime error: string_take_prefix called with NULL string pointer\n"); exit(1); }
    if (count < 0) {
        r.error = "StringNegativeCountError";
        return r;
    }
    int64_t length = string_length(value);
    if (count > length) {
        r.error = "StringRangeOutOfBoundsError";
        return r;
    }
    if (count == 0) {
        r.value = opal_string_duplicate("");
        if (!r.value) {
            r.error = "AllocationFailureError";
        }
        return r;
    }
    const unsigned char* cursor = (const unsigned char*)value;
    int64_t seen = 0;
    while (*cursor != '\0' && seen < count) {
        uint32_t codepoint = 0u;
        size_t scalar_len = 0u;
        if (!opal_utf8_decode_scalar(cursor, &codepoint, &scalar_len)) {
            break;
        }
        cursor += scalar_len;
        seen++;
    }
    size_t prefix_len = (size_t)((const char*)cursor - value);
    char* result = (char*)malloc(prefix_len + 1u);
    if (!result) {
        r.error = "AllocationFailureError";
        return r;
    }
    memcpy(result, value, prefix_len);
    result[prefix_len] = '\0';
    opal_rc_debug_note_alloc(OPAL_RC_DEBUG_COUNTER_STRINGS);
    r.value = result;
    return r;
}

FsStringResult string_take_suffix(const char* value, int64_t count) {
    FsStringResult r;
    r.value = NULL;
    r.error = NULL;
    if (!value) { fprintf(stderr, "Runtime error: string_take_suffix called with NULL string pointer\n"); exit(1); }
    if (count < 0) {
        r.error = "StringNegativeCountError";
        return r;
    }
    int64_t length = string_length(value);
    if (count > length) {
        r.error = "StringRangeOutOfBoundsError";
        return r;
    }
    if (count == 0) {
        r.value = opal_string_duplicate("");
        if (!r.value) {
            r.error = "AllocationFailureError";
        }
        return r;
    }
    int64_t start_index = length - count;
    const unsigned char* cursor = (const unsigned char*)value;
    const char* start = value;
    int64_t seen = 0;
    while (*cursor != '\0') {
        if (opal_utf8_is_scalar_start(*cursor)) {
            if (seen == start_index) {
                start = (const char*)cursor;
                break;
            }
            seen++;
        }
        cursor++;
    }
    r.value = opal_string_duplicate(start);
    if (!r.value) {
        r.error = "AllocationFailureError";
    }
    return r;
}

static const char* opal_string_scalar_boundary(const char* value, int64_t scalar_index) {
    if (scalar_index == 0) {
        return value;
    }
    const unsigned char* cursor = (const unsigned char*)value;
    int64_t seen = 0;
    while (*cursor != '\0') {
        if (opal_utf8_is_scalar_start(*cursor)) {
            if (seen == scalar_index) {
                return (const char*)cursor;
            }
            seen++;
        }
        cursor++;
    }
    return (const char*)cursor;
}

FsStringResult string_extract_range(const char* value, int64_t start_index, int64_t end_index) {
    FsStringResult r;
    r.value = NULL;
    r.error = NULL;
    if (!value) { fprintf(stderr, "Runtime error: string_extract_range called with NULL string pointer\n"); exit(1); }
    if (start_index < 0 || end_index < 0) {
        r.error = "StringRangeOutOfBoundsError";
        return r;
    }
    if (end_index < start_index) {
        r.error = "StringRangeOrderError";
        return r;
    }
    int64_t length = string_length(value);
    if (end_index > length) {
        r.error = "StringRangeOutOfBoundsError";
        return r;
    }
    const char* start = opal_string_scalar_boundary(value, start_index);
    const char* end = opal_string_scalar_boundary(value, end_index);
    size_t range_len = (size_t)(end - start);
    char* result = (char*)malloc(range_len + 1u);
    if (!result) {
        r.error = "AllocationFailureError";
        return r;
    }
    memcpy(result, start, range_len);
    result[range_len] = '\0';
    opal_rc_debug_note_alloc(OPAL_RC_DEBUG_COUNTER_STRINGS);
    r.value = result;
    return r;
}

FsStringResult string_insert_at(const char* value, int64_t scalar_index, const char* inserted) {
    FsStringResult r;
    r.value = NULL;
    r.error = NULL;
    if (!value) { fprintf(stderr, "Runtime error: string_insert_at called with NULL string pointer\n"); exit(1); }
    if (!inserted) { fprintf(stderr, "Runtime error: string_insert_at called with NULL inserted pointer\n"); exit(1); }
    if (scalar_index < 0) {
        r.error = "StringRangeOutOfBoundsError";
        return r;
    }
    int64_t length = string_length(value);
    if (scalar_index > length) {
        r.error = "StringRangeOutOfBoundsError";
        return r;
    }
    const char* boundary = opal_string_scalar_boundary(value, scalar_index);
    size_t prefix_len = (size_t)(boundary - value);
    size_t inserted_len = strlen(inserted);
    size_t suffix_len = strlen(boundary);
    if (prefix_len > SIZE_MAX - inserted_len || prefix_len + inserted_len > SIZE_MAX - suffix_len - 1u) {
        r.error = "AllocationFailureError";
        return r;
    }
    size_t total_len = prefix_len + inserted_len + suffix_len;
    char* result = (char*)malloc(total_len + 1u);
    if (!result) {
        r.error = "AllocationFailureError";
        return r;
    }
    memcpy(result, value, prefix_len);
    memcpy(result + prefix_len, inserted, inserted_len);
    memcpy(result + prefix_len + inserted_len, boundary, suffix_len);
    result[total_len] = '\0';
    opal_rc_debug_note_alloc(OPAL_RC_DEBUG_COUNTER_STRINGS);
    r.value = result;
    return r;
}

FsStringResult string_delete_range(const char* value, int64_t start_index, int64_t end_index) {
    FsStringResult r;
    r.value = NULL;
    r.error = NULL;
    if (!value) { fprintf(stderr, "Runtime error: string_delete_range called with NULL string pointer\n"); exit(1); }
    if (start_index < 0 || end_index < 0) {
        r.error = "StringRangeOutOfBoundsError";
        return r;
    }
    if (end_index < start_index) {
        r.error = "StringRangeOrderError";
        return r;
    }
    int64_t length = string_length(value);
    if (end_index > length) {
        r.error = "StringRangeOutOfBoundsError";
        return r;
    }
    const char* start = opal_string_scalar_boundary(value, start_index);
    const char* end = opal_string_scalar_boundary(value, end_index);
    size_t prefix_len = (size_t)(start - value);
    size_t suffix_len = strlen(end);
    if (prefix_len > SIZE_MAX - suffix_len - 1u) {
        r.error = "AllocationFailureError";
        return r;
    }
    size_t total_len = prefix_len + suffix_len;
    char* result = (char*)malloc(total_len + 1u);
    if (!result) {
        r.error = "AllocationFailureError";
        return r;
    }
    memcpy(result, value, prefix_len);
    memcpy(result + prefix_len, end, suffix_len);
    result[total_len] = '\0';
    opal_rc_debug_note_alloc(OPAL_RC_DEBUG_COUNTER_STRINGS);
    r.value = result;
    return r;
}

FsStringResult string_replace_range(const char* value, int64_t start_index, int64_t end_index, const char* replacement) {
    FsStringResult r;
    r.value = NULL;
    r.error = NULL;
    if (!value) { fprintf(stderr, "Runtime error: string_replace_range called with NULL string pointer\n"); exit(1); }
    if (!replacement) { fprintf(stderr, "Runtime error: string_replace_range called with NULL replacement pointer\n"); exit(1); }
    if (start_index < 0 || end_index < 0) {
        r.error = "StringRangeOutOfBoundsError";
        return r;
    }
    if (end_index < start_index) {
        r.error = "StringRangeOrderError";
        return r;
    }
    int64_t length = string_length(value);
    if (end_index > length) {
        r.error = "StringRangeOutOfBoundsError";
        return r;
    }
    const char* start = opal_string_scalar_boundary(value, start_index);
    const char* end = opal_string_scalar_boundary(value, end_index);
    size_t prefix_len = (size_t)(start - value);
    size_t replacement_len = strlen(replacement);
    size_t suffix_len = strlen(end);
    if (prefix_len > SIZE_MAX - replacement_len || prefix_len + replacement_len > SIZE_MAX - suffix_len - 1u) {
        r.error = "AllocationFailureError";
        return r;
    }
    size_t total_len = prefix_len + replacement_len + suffix_len;
    char* result = (char*)malloc(total_len + 1u);
    if (!result) {
        r.error = "AllocationFailureError";
        return r;
    }
    memcpy(result, value, prefix_len);
    memcpy(result + prefix_len, replacement, replacement_len);
    memcpy(result + prefix_len + replacement_len, end, suffix_len);
    result[total_len] = '\0';
    opal_rc_debug_note_alloc(OPAL_RC_DEBUG_COUNTER_STRINGS);
    r.value = result;
    return r;
}

static int opal_terminal_layout_is_extend(uint32_t codepoint) {
    return (codepoint >= 0x0300u && codepoint <= 0x036Fu)
        || (codepoint >= 0x0483u && codepoint <= 0x0489u)
        || (codepoint >= 0x0591u && codepoint <= 0x05BDu)
        || codepoint == 0x05BFu
        || (codepoint >= 0x05C1u && codepoint <= 0x05C2u)
        || (codepoint >= 0x05C4u && codepoint <= 0x05C5u)
        || codepoint == 0x05C7u
        || (codepoint >= 0x0610u && codepoint <= 0x061Au)
        || (codepoint >= 0x064Bu && codepoint <= 0x065Fu)
        || codepoint == 0x0670u
        || (codepoint >= 0x06D6u && codepoint <= 0x06DCu)
        || (codepoint >= 0x06DFu && codepoint <= 0x06E4u)
        || (codepoint >= 0x06E7u && codepoint <= 0x06E8u)
        || (codepoint >= 0x06EAu && codepoint <= 0x06EDu)
        || (codepoint >= 0x0711u && codepoint <= 0x074Au)
        || (codepoint >= 0x07A6u && codepoint <= 0x07B0u)
        || (codepoint >= 0x07EBu && codepoint <= 0x07F3u)
        || (codepoint >= 0x0816u && codepoint <= 0x082Du)
        || (codepoint >= 0x0859u && codepoint <= 0x085Bu)
        || (codepoint >= 0x08D3u && codepoint <= 0x0903u)
        || codepoint == 0x093Au
        || codepoint == 0x093Cu
        || (codepoint >= 0x0941u && codepoint <= 0x0948u)
        || codepoint == 0x094Du
        || (codepoint >= 0x0951u && codepoint <= 0x0957u)
        || (codepoint >= 0x0962u && codepoint <= 0x0963u)
        || codepoint == 0x0981u
        || codepoint == 0x09BCu
        || (codepoint >= 0x09C1u && codepoint <= 0x09C4u)
        || codepoint == 0x09CDu
        || (codepoint >= 0x09E2u && codepoint <= 0x09E3u)
        || (codepoint >= 0x0A01u && codepoint <= 0x0A02u)
        || codepoint == 0x0A3Cu
        || (codepoint >= 0x0A41u && codepoint <= 0x0A4Du)
        || codepoint == 0x0A51u
        || (codepoint >= 0x0A70u && codepoint <= 0x0A75u)
        || (codepoint >= 0x0A81u && codepoint <= 0x0A82u)
        || codepoint == 0x0ABCu
        || (codepoint >= 0x0AC1u && codepoint <= 0x0ACDu)
        || (codepoint >= 0x0AE2u && codepoint <= 0x0AE3u)
        || codepoint == 0x0B01u
        || codepoint == 0x0B3Cu
        || codepoint == 0x0B3Fu
        || (codepoint >= 0x0B41u && codepoint <= 0x0B44u)
        || codepoint == 0x0B4Du
        || codepoint == 0x0B56u
        || (codepoint >= 0x0B62u && codepoint <= 0x0B63u)
        || codepoint == 0x0B82u
        || codepoint == 0x0BC0u
        || codepoint == 0x0BCDu
        || codepoint == 0x0C00u
        || codepoint == 0x0C04u
        || (codepoint >= 0x0C3Eu && codepoint <= 0x0C4Du)
        || (codepoint >= 0x0C55u && codepoint <= 0x0C56u)
        || (codepoint >= 0x0C62u && codepoint <= 0x0C63u)
        || codepoint == 0x0C81u
        || codepoint == 0x0CBCu
        || codepoint == 0x0CBFu
        || codepoint == 0x0CC6u
        || (codepoint >= 0x0CCCu && codepoint <= 0x0CCDu)
        || (codepoint >= 0x0CE2u && codepoint <= 0x0CE3u)
        || (codepoint >= 0x0D00u && codepoint <= 0x0D01u)
        || (codepoint >= 0x0D3Bu && codepoint <= 0x0D3Cu)
        || (codepoint >= 0x0D41u && codepoint <= 0x0D44u)
        || codepoint == 0x0D4Du
        || (codepoint >= 0x0D62u && codepoint <= 0x0D63u)
        || codepoint == 0x0DCAu
        || (codepoint >= 0x0DD2u && codepoint <= 0x0DD6u)
        || codepoint == 0x0E31u
        || (codepoint >= 0x0E34u && codepoint <= 0x0E3Au)
        || (codepoint >= 0x0E47u && codepoint <= 0x0E4Eu)
        || codepoint == 0x0EB1u
        || (codepoint >= 0x0EB4u && codepoint <= 0x0EBCu)
        || (codepoint >= 0x0EC8u && codepoint <= 0x0ECDu)
        || (codepoint >= 0x0F18u && codepoint <= 0x0F19u)
        || codepoint == 0x0F35u
        || codepoint == 0x0F37u
        || codepoint == 0x0F39u
        || (codepoint >= 0x0F71u && codepoint <= 0x0F84u)
        || (codepoint >= 0x0F86u && codepoint <= 0x0F87u)
        || (codepoint >= 0x0F8Du && codepoint <= 0x0FBCu)
        || codepoint == 0x0FC6u
        || (codepoint >= 0x1AB0u && codepoint <= 0x1AFFu)
        || (codepoint >= 0x1DC0u && codepoint <= 0x1DFFu)
        || (codepoint >= 0x20D0u && codepoint <= 0x20FFu)
        || (codepoint >= 0xFE00u && codepoint <= 0xFE0Fu)
        || (codepoint >= 0xE0100u && codepoint <= 0xE01EFu)
        || (codepoint >= 0x1F3FBu && codepoint <= 0x1F3FFu);
}

static int opal_terminal_layout_is_regional_indicator(uint32_t codepoint) {
    return codepoint >= 0x1F1E6u && codepoint <= 0x1F1FFu;
}

static int opal_terminal_layout_is_wide(uint32_t codepoint) {
    return (codepoint >= 0x1100u && codepoint <= 0x115Fu)
        || (codepoint >= 0x231Au && codepoint <= 0x231Bu)
        || (codepoint >= 0x2329u && codepoint <= 0x232Au)
        || (codepoint >= 0x23E9u && codepoint <= 0x23ECu)
        || codepoint == 0x23F0u
        || codepoint == 0x23F3u
        || (codepoint >= 0x25FDu && codepoint <= 0x25FEu)
        || (codepoint >= 0x2614u && codepoint <= 0x2615u)
        || (codepoint >= 0x2648u && codepoint <= 0x2653u)
        || codepoint == 0x267Fu
        || codepoint == 0x2693u
        || codepoint == 0x26A1u
        || (codepoint >= 0x26AAu && codepoint <= 0x26ABu)
        || (codepoint >= 0x26BDu && codepoint <= 0x26BEu)
        || (codepoint >= 0x26C4u && codepoint <= 0x26C5u)
        || codepoint == 0x26CEu
        || codepoint == 0x26D4u
        || codepoint == 0x26EAu
        || (codepoint >= 0x26F2u && codepoint <= 0x26F3u)
        || codepoint == 0x26F5u
        || codepoint == 0x26FAu
        || codepoint == 0x26FDu
        || codepoint == 0x2705u
        || (codepoint >= 0x270Au && codepoint <= 0x270Bu)
        || codepoint == 0x2728u
        || codepoint == 0x274Cu
        || codepoint == 0x274Eu
        || (codepoint >= 0x2753u && codepoint <= 0x2755u)
        || codepoint == 0x2757u
        || (codepoint >= 0x2795u && codepoint <= 0x2797u)
        || codepoint == 0x27B0u
        || codepoint == 0x27BFu
        || (codepoint >= 0x2B1Bu && codepoint <= 0x2B1Cu)
        || codepoint == 0x2B50u
        || codepoint == 0x2B55u
        || (codepoint >= 0x2E80u && codepoint <= 0xA4CFu)
        || (codepoint >= 0xAC00u && codepoint <= 0xD7A3u)
        || (codepoint >= 0xF900u && codepoint <= 0xFAFFu)
        || (codepoint >= 0xFE10u && codepoint <= 0xFE19u)
        || (codepoint >= 0xFE30u && codepoint <= 0xFE6Fu)
        || (codepoint >= 0xFF00u && codepoint <= 0xFF60u)
        || (codepoint >= 0xFFE0u && codepoint <= 0xFFE6u)
        || (codepoint >= 0x1F000u && codepoint <= 0x1FAFFu)
        || (codepoint >= 0x20000u && codepoint <= 0x3FFFDu);
}

static int opal_terminal_layout_is_emoji(uint32_t codepoint) {
    return (codepoint >= 0x2600u && codepoint <= 0x27BFu)
        || (codepoint >= 0x1F000u && codepoint <= 0x1FAFFu);
}

static const unsigned char* opal_terminal_layout_next_scalar_end(const unsigned char* cursor) {
    if (!cursor || *cursor == '\0') {
        return cursor;
    }
    cursor++;
    while (*cursor != '\0' && !opal_utf8_is_scalar_start(*cursor)) {
        cursor++;
    }
    return cursor;
}

static const unsigned char* opal_terminal_layout_next_grapheme_end(const unsigned char* start) {
    const unsigned char* cursor = opal_terminal_layout_next_scalar_end(start);
    uint32_t first_codepoint = 0u;
    size_t first_len = 0u;
    int join_next = 0;
    if (opal_utf8_decode_scalar(start, &first_codepoint, &first_len)
        && opal_terminal_layout_is_regional_indicator(first_codepoint)
        && *cursor != '\0') {
        uint32_t second_codepoint = 0u;
        size_t second_len = 0u;
        if (opal_utf8_decode_scalar(cursor, &second_codepoint, &second_len)
            && opal_terminal_layout_is_regional_indicator(second_codepoint)) {
            return cursor + second_len;
        }
    }
    while (*cursor != '\0') {
        uint32_t codepoint = 0u;
        size_t scalar_len = 0u;
        if (!opal_utf8_decode_scalar(cursor, &codepoint, &scalar_len)) {
            return opal_terminal_layout_next_scalar_end(cursor);
        }
        if (opal_terminal_layout_is_extend(codepoint)) {
            cursor += scalar_len;
            continue;
        }
        if (codepoint == 0x200Du) {
            cursor += scalar_len;
            join_next = 1;
            continue;
        }
        if (join_next) {
            cursor += scalar_len;
            join_next = 0;
            continue;
        }
        break;
    }
    return cursor;
}

static int64_t opal_terminal_layout_grapheme_width(const unsigned char* start, const unsigned char* end) {
    int has_joiner = 0;
    int has_emoji = 0;
    int64_t width = 0;
    const unsigned char* cursor = start;
    while (cursor < end && *cursor != '\0') {
        uint32_t codepoint = 0u;
        size_t scalar_len = 0u;
        if (!opal_utf8_decode_scalar(cursor, &codepoint, &scalar_len)) {
            break;
        }
        if (codepoint == 0x200Du) {
            has_joiner = 1;
        }
        if (opal_terminal_layout_is_emoji(codepoint)) {
            has_emoji = 1;
        }
        if (codepoint == 0x200Du || opal_terminal_layout_is_extend(codepoint) || codepoint < 0x20u || (codepoint >= 0x7Fu && codepoint < 0xA0u)) {
            width += 0;
        } else if (opal_terminal_layout_is_regional_indicator(codepoint)) {
            width += 1;
        } else if (opal_terminal_layout_is_wide(codepoint)) {
            width += 2;
        } else {
            width += 1;
        }
        cursor += scalar_len;
    }
    int regional_count = 0;
    int has_keycap = 0;
    cursor = start;
    while (cursor < end && *cursor != '\0') {
        uint32_t codepoint = 0u;
        size_t scalar_len = 0u;
        if (!opal_utf8_decode_scalar(cursor, &codepoint, &scalar_len)) {
            break;
        }
        if (opal_terminal_layout_is_regional_indicator(codepoint)) {
            regional_count++;
        }
        if (codepoint == 0x20E3u) {
            has_keycap = 1;
        }
        cursor += scalar_len;
    }
    if ((has_joiner && has_emoji) || regional_count >= 2 || has_keycap) {
        return 2;
    }
    return width;
}

int64_t terminal_text_cell_width(const char* value) {
    if (!value) { fprintf(stderr, "Runtime error: terminal_text_cell_width called with NULL string pointer\n"); exit(1); }
    int64_t width = 0;
    const unsigned char* cursor = (const unsigned char*)value;
    while (*cursor != '\0') {
        const unsigned char* next = opal_terminal_layout_next_grapheme_end(cursor);
        width += opal_terminal_layout_grapheme_width(cursor, next);
        cursor = next;
    }
    return width;
}

FsStringInt64Result terminal_text_clip_to_cells(const char* value, int64_t max_cells) {
    FsStringInt64Result r;
    r.value = NULL;
    r.used_cells = 0;
    r.error = NULL;
    if (!value) { fprintf(stderr, "Runtime error: terminal_text_clip_to_cells called with NULL string pointer\n"); exit(1); }
    if (max_cells < 0) {
        r.error = "NegativeCellLimit";
        return r;
    }

    const unsigned char* start = (const unsigned char*)value;
    const unsigned char* cursor = start;
    const unsigned char* end = start;
    int64_t used_cells = 0;
    while (*cursor != '\0') {
        const unsigned char* next = opal_terminal_layout_next_grapheme_end(cursor);
        int64_t width = opal_terminal_layout_grapheme_width(cursor, next);
        if (width > max_cells - used_cells) {
            break;
        }
        used_cells += width;
        end = next;
        cursor = next;
    }

    size_t byte_len = (size_t)(end - start);
    char* result = (char*)malloc(byte_len + 1u);
    if (!result) {
        r.error = "AllocationFailureError";
        return r;
    }
    memcpy(result, value, byte_len);
    result[byte_len] = '\0';
    opal_rc_debug_note_alloc(OPAL_RC_DEBUG_COUNTER_STRINGS);
    r.value = result;
    r.used_cells = used_cells;
    return r;
}

StringBuilderStringResult string_join(const char** values, int64_t count, const char* separator) {
    StringBuilderStringResult result;
    result.value = NULL;
    result.error = NULL;
    if (!values && count != 0) { fprintf(stderr, "Runtime error: string_join called with NULL array pointer and non-zero length\n"); exit(1); }
    if (count < 0) { fprintf(stderr, "Runtime error: string_join called with negative length\n"); exit(1); }

    const char* safe_separator = separator ? separator : "";
    size_t separator_length = strlen(safe_separator);
    size_t total_length = 0;

    for (int64_t index = 0; index < count; index++) {
        const char* value = values[index] ? values[index] : "";
        size_t value_length = strlen(value);
        if (SIZE_MAX - total_length < value_length) {
            fprintf(stderr, "Runtime error: string_join size overflow\n");
            exit(1);
        }
        total_length += value_length;
        if (index + 1 < count) {
            if (SIZE_MAX - total_length < separator_length) {
                fprintf(stderr, "Runtime error: string_join separator overflow\n");
                exit(1);
            }
            total_length += separator_length;
        }
    }

    char* joined = (char*)malloc(total_length + 1u);
    if (!joined) {
        result.error = "AllocationFailureError";
        return result;
    }
    opal_rc_debug_note_alloc(OPAL_RC_DEBUG_COUNTER_STRINGS);

    size_t offset = 0;
    for (int64_t index = 0; index < count; index++) {
        const char* value = values[index] ? values[index] : "";
        size_t value_length = strlen(value);
        memcpy(joined + offset, value, value_length);
        offset += value_length;
        if (index + 1 < count) {
            memcpy(joined + offset, safe_separator, separator_length);
            offset += separator_length;
        }
    }
    joined[offset] = '\0';
    result.value = joined;
    return result;
}

OpalStringBuilder* string_builder_new(void) {
    OpalStringBuilder* builder = (OpalStringBuilder*)calloc(1u, sizeof(OpalStringBuilder));
    if (!builder) { fprintf(stderr, "Runtime error: out of memory\n"); exit(1); }
    opal_rc_debug_note_alloc(OPAL_RC_DEBUG_COUNTER_BUILDERS);
    builder->buffer = opal_string_duplicate_or_die("");
    builder->capacity = 1u;
    builder->length = 0u;
    builder->finished = 0;
    opal_string_builder_register_for_cleanup(builder);
    return builder;
}

StringBuilderVoidResult string_builder_push(OpalStringBuilder* builder, const char* value) {
    if (!builder || builder->finished) {
        return (StringBuilderVoidResult){ NULL, "BuilderFinishedError" };
    }

    const char* safe_value = value ? value : "";
    size_t value_length = strlen(safe_value);
    if (builder->length == SIZE_MAX ||
        value_length > SIZE_MAX - builder->length - 1u) {
        return (StringBuilderVoidResult){ NULL, "AllocationFailureError" };
    }

    const char* capacity_error = string_builder_ensure_capacity(builder, builder->length + value_length + 1u);
    if (capacity_error) {
        return (StringBuilderVoidResult){ NULL, capacity_error };
    }

    memcpy(builder->buffer + builder->length, safe_value, value_length);
    builder->length += value_length;
    builder->buffer[builder->length] = '\0';
    return (StringBuilderVoidResult){ NULL, NULL };
}

StringBuilderStringResult string_builder_finish(OpalStringBuilder* builder) {
    if (!builder || builder->finished) {
        return (StringBuilderStringResult){ NULL, "BuilderFinishedError" };
    }

    char* result = opal_string_duplicate_or_die(builder->buffer ? builder->buffer : "");
    builder->finished = 1;
    if (builder->buffer) {
        opal_rc_debug_note_free(OPAL_RC_DEBUG_COUNTER_STRINGS);
        free(builder->buffer);
        builder->buffer = NULL;
    }
    builder->length = 0u;
    builder->capacity = 0u;
    return (StringBuilderStringResult){ result, NULL };
}

int64_t array_length(const void* array, int64_t length) {
    if (!array && length != 0) { fprintf(stderr, "Runtime error: array_length called with NULL array pointer and non-zero length\n"); exit(1); }
    if (length < 0) { fprintf(stderr, "Runtime error: array_length called with negative length\n"); exit(1); }
    return length;
}

static void opal_write_repeated(FILE* stream, char value, size_t count) {
    for (size_t index = 0; index < count; index++) {
        fputc((int)value, stream);
    }
}

static size_t opal_utf8_scalar_count(const char* text, size_t length) {
    size_t count = 0u;
    for (size_t index = 0; index < length; index++) {
        if ((((unsigned char)text[index]) & 0xC0u) != 0x80u) {
            count++;
        }
    }
    return count;
}

static int opal_runtime_has_string_index_source_context(void) {
    return opal_runtime_string_index_source_path != NULL
        && opal_runtime_string_index_source_text != NULL
        && opal_runtime_string_index_source_text[0] != '\0'
        && opal_runtime_string_index_span_len > 0u;
}

static void opal_render_string_index_bounds_error(uint64_t index, uint64_t length) {
    const char* source_path = opal_runtime_string_index_source_path;
    const char* source_text = opal_runtime_string_index_source_text;
    size_t source_length = strlen(source_text);
    size_t span_start = (size_t)opal_runtime_string_index_span_start;
    size_t span_length = (size_t)opal_runtime_string_index_span_len;
    if (span_start > source_length) {
        span_start = source_length;
    }
    if (span_length > source_length - span_start) {
        span_length = source_length - span_start;
    }
    if (span_length == 0u) {
        span_length = 1u;
    }

    size_t line_number = 1u;
    size_t line_start = 0u;
    for (size_t offset = 0; offset < span_start; offset++) {
        if (source_text[offset] == '\n') {
            line_number++;
            line_start = offset + 1u;
        }
    }

    size_t line_end = line_start;
    while (line_end < source_length && source_text[line_end] != '\n') {
        line_end++;
    }

    size_t column_number = opal_utf8_scalar_count(source_text + line_start, span_start - line_start) + 1u;
    size_t highlight_length = span_length;
    if (span_start + highlight_length > line_end) {
        highlight_length = line_end > span_start ? line_end - span_start : 1u;
    }
    highlight_length = opal_utf8_scalar_count(source_text + span_start, highlight_length);
    if (highlight_length == 0u) {
        highlight_length = 1u;
    }

    fprintf(
        stderr,
        "error[opalescent::runtime::index_out_of_bounds]: string index %" PRIu64 " is out of bounds for length %" PRIu64 "\n",
        index,
        length
    );
    fprintf(stderr, "  --> %s:%zu:%zu\n", source_path, line_number, column_number);
    fprintf(stderr, "   |\n");
    fprintf(stderr, "%4zu | %.*s\n", line_number, (int)(line_end - line_start), source_text + line_start);
    fprintf(stderr, "   | ");
    opal_write_repeated(stderr, ' ', column_number - 1u);
    opal_write_repeated(stderr, '^', highlight_length);
    fprintf(stderr, " string index is out of bounds for this string\n");
    fprintf(stderr, "   |\n");
    fprintf(stderr, "   = help: Ensure the index is within 0 <= index < string.length\n");
}

void opal_array_bounds_error(uint64_t index, uint64_t length) {
    if (opal_runtime_has_string_index_source_context()) {
        opal_render_string_index_bounds_error(index, length);
        exit(1);
    }
    fprintf(stderr, "index %" PRIu64 " is out of bounds for length %" PRIu64 "\n", index, length);
    exit(1);
}
