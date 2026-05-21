#include "opal_portability.h"
#include "opal_rc.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include <inttypes.h>

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

char* string_join(const char** values, int64_t count, const char* separator) {
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

    char* result = (char*)malloc(total_length + 1u);
    if (!result) { fprintf(stderr, "Runtime error: out of memory\n"); exit(1); }
    opal_rc_debug_note_alloc(OPAL_RC_DEBUG_COUNTER_STRINGS);

    size_t offset = 0;
    for (int64_t index = 0; index < count; index++) {
        const char* value = values[index] ? values[index] : "";
        size_t value_length = strlen(value);
        memcpy(result + offset, value, value_length);
        offset += value_length;
        if (index + 1 < count) {
            memcpy(result + offset, safe_separator, separator_length);
            offset += separator_length;
        }
    }
    result[offset] = '\0';
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

void opal_array_bounds_error(uint64_t index, uint64_t length) {
    fprintf(stderr, "index %" PRIu64 " is out of bounds for length %" PRIu64 "\n", index, length);
    exit(1);
}
