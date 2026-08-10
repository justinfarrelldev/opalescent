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
#ifndef OPAL_PARSE_RESULT_I64_DEFINED
typedef struct { int64_t value; const char* error; } ParseResultI64;
#define OPAL_PARSE_RESULT_I64_DEFINED 1
#endif
#ifndef OPAL_FS_STRING_RESULT_TYPES_DEFINED
typedef struct { char* value; const char* error; } FsStringResult;
typedef struct { char** value; int64_t count; const char* error; } FsStringArrayResult;
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
    const unsigned char* cursor = (const unsigned char*)value;
    const char* start = value;
    const char* end = value;
    int64_t seen = 0;
    while (*cursor != '\0') {
        if (opal_utf8_is_scalar_start(*cursor)) {
            if (seen == start_index) {
                start = (const char*)cursor;
            }
            if (seen == end_index) {
                end = (const char*)cursor;
                break;
            }
            seen++;
        }
        cursor++;
    }
    if (end_index == length) {
        end = (const char*)cursor;
    }
    if (start_index == length) {
        start = (const char*)cursor;
        end = (const char*)cursor;
    }
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
