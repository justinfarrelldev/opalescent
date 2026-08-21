#include "opal_portability.h"
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#if defined(OPAL_ENABLE_INTERNAL_TESTING)
#include "opal_test_alloc.h"
#endif

#define OPAL_ERROR_MAX_CAUSE_DEPTH 8
#define OPAL_ERROR_MAX_SUPPRESSED 8
#define OPAL_ERROR_MAX_BYTES 65536u
#define OPAL_ERROR_GRAPH_RECURSION_LIMIT 64

#ifndef OPAL_FS_STRING_RESULT_DEFINED
typedef struct {
    char* value;
    const char* error;
} FsStringResult;
#define OPAL_FS_STRING_RESULT_DEFINED 1
#endif

#ifndef OPAL_ERROR_TRUNCATION_DEFINED
typedef struct OpalErrorTruncation {
    int8_t cause_depth;
    int8_t suppressed_count;
    int8_t bytes;
} OpalErrorTruncation;
#define OPAL_ERROR_TRUNCATION_DEFINED 1
#endif

typedef struct OpalErrorMeta {
    const char* identity;
    const char* cause;
    const char* suppressed[OPAL_ERROR_MAX_SUPPRESSED];
    int64_t suppressed_count;
    OpalErrorTruncation truncation;
    struct OpalErrorMeta* next;
} OpalErrorMeta;

static OpalErrorMeta* opal_error_meta_head = NULL;
static OpalErrorTruncation opal_error_empty_truncation = {0, 0, 0};

static char* opal_error_duplicate_text(const char* text) {
    const char* source = text != NULL ? text : "UnknownError";
    size_t length = strlen(source);
    char* copy = (char*)malloc(length + 1u);
    if (copy == NULL) {
        return (char*)source;
    }
    memcpy(copy, source, length + 1u);
    return copy;
}

static OpalErrorMeta* opal_error_find_meta(const char* identity) {
    for (OpalErrorMeta* meta = opal_error_meta_head; meta != NULL; meta = meta->next) {
        if (meta->identity == identity) {
            return meta;
        }
    }
    return NULL;
}

static OpalErrorMeta* opal_error_alloc_meta(const char* identity) {
    OpalErrorMeta* meta = (OpalErrorMeta*)calloc(1u, sizeof(OpalErrorMeta));
    if (meta == NULL) {
        return NULL;
    }
    meta->identity = identity;
    meta->next = opal_error_meta_head;
    opal_error_meta_head = meta;
    return meta;
}

static OpalErrorMeta* opal_error_clone_meta(const char* clone, const OpalErrorMeta* source) {
    OpalErrorMeta* meta = opal_error_alloc_meta(clone);
    if (meta == NULL) {
        return NULL;
    }
    if (source == NULL) {
        return meta;
    }
    meta->cause = source->cause;
    meta->suppressed_count = source->suppressed_count;
    for (int64_t index = 0; index < source->suppressed_count; ++index) {
        meta->suppressed[index] = source->suppressed[index];
    }
    meta->truncation = source->truncation;
    return meta;
}

static char* opal_error_clone_identity(const char* identity, OpalErrorMeta** out_meta) {
    char* clone = opal_error_duplicate_text(identity);
    if (clone == identity && identity != NULL) {
        *out_meta = NULL;
        return clone;
    }
    *out_meta = opal_error_clone_meta(clone, opal_error_find_meta(identity));
    if (*out_meta == NULL) {
        free(clone);
        return (char*)identity;
    }
    return clone;
}

char* opal_error_new(const char* variant_name) {
    return opal_error_duplicate_text(variant_name);
}

static int opal_error_graph_contains_depth(
    const char* root,
    const char* target,
    int depth
) {
    if (root == NULL || target == NULL || depth > OPAL_ERROR_GRAPH_RECURSION_LIMIT) {
        return 0;
    }
    if (root == target) {
        return 1;
    }
    OpalErrorMeta* meta = opal_error_find_meta(root);
    if (meta == NULL) {
        return 0;
    }
    if (opal_error_graph_contains_depth(meta->cause, target, depth + 1)) {
        return 1;
    }
    for (int64_t index = 0; index < meta->suppressed_count; ++index) {
        if (opal_error_graph_contains_depth(meta->suppressed[index], target, depth + 1)) {
            return 1;
        }
    }
    return 0;
}

static int opal_error_graph_contains(const char* root, const char* target) {
    return opal_error_graph_contains_depth(root, target, 0);
}

static int opal_error_cause_depth(const char* error, int depth) {
    if (error == NULL || depth > OPAL_ERROR_GRAPH_RECURSION_LIMIT) {
        return 0;
    }
    OpalErrorMeta* meta = opal_error_find_meta(error);
    if (meta == NULL || meta->cause == NULL) {
        return 0;
    }
    return 1 + opal_error_cause_depth(meta->cause, depth + 1);
}

static size_t opal_error_graph_bytes_depth(const char* error, int depth) {
    if (error == NULL || depth > OPAL_ERROR_GRAPH_RECURSION_LIMIT) {
        return 0u;
    }
    size_t total = strlen(error) + 1u + sizeof(OpalErrorMeta);
    OpalErrorMeta* meta = opal_error_find_meta(error);
    if (meta == NULL) {
        return total;
    }
    total += opal_error_graph_bytes_depth(meta->cause, depth + 1);
    for (int64_t index = 0; index < meta->suppressed_count; ++index) {
        total += opal_error_graph_bytes_depth(meta->suppressed[index], depth + 1);
    }
    return total;
}

static size_t opal_error_graph_bytes(const char* error) {
    return opal_error_graph_bytes_depth(error, 0);
}

static char* opal_error_clone_with_marker(
    const char* primary,
    int8_t cause_depth,
    int8_t suppressed_count,
    int8_t bytes
) {
    OpalErrorMeta* meta = NULL;
    char* clone = opal_error_clone_identity(primary, &meta);
    if (meta == NULL) {
        return clone;
    }
    meta->truncation.cause_depth = (int8_t)(meta->truncation.cause_depth || cause_depth);
    meta->truncation.suppressed_count = (int8_t)(
        meta->truncation.suppressed_count || suppressed_count
    );
    meta->truncation.bytes = (int8_t)(meta->truncation.bytes || bytes);
    return clone;
}

char* opal_error_attach_cause(const char* primary, const char* cause) {
    if (primary == NULL) {
        return NULL;
    }
    if (cause == NULL) {
        return (char*)primary;
    }

    OpalErrorMeta* primary_meta = opal_error_find_meta(primary);
    const char* immediate_cause = primary_meta != NULL ? primary_meta->cause : NULL;

    if (immediate_cause == cause || opal_error_graph_contains(primary, cause)) {
        return (char*)primary;
    }

    if (primary == cause || opal_error_graph_contains(cause, primary)) {
        return opal_error_clone_with_marker(
            primary,
            immediate_cause == NULL,
            immediate_cause != NULL,
            0
        );
    }

    if (immediate_cause == NULL) {
        if (opal_error_cause_depth(cause, 0) + 1 > OPAL_ERROR_MAX_CAUSE_DEPTH) {
            return opal_error_clone_with_marker(primary, 1, 0, 0);
        }
        if (opal_error_graph_bytes(primary) + opal_error_graph_bytes(cause) > OPAL_ERROR_MAX_BYTES) {
            return opal_error_clone_with_marker(primary, 0, 0, 1);
        }
        OpalErrorMeta* derived_meta = NULL;
        char* derived = opal_error_clone_identity(primary, &derived_meta);
        if (derived_meta == NULL) {
            return opal_error_clone_with_marker(primary, 0, 0, 1);
        }
        derived_meta->cause = cause;
        return derived;
    }

    int64_t suppressed_count = primary_meta != NULL ? primary_meta->suppressed_count : 0;
    if (suppressed_count >= OPAL_ERROR_MAX_SUPPRESSED) {
        return opal_error_clone_with_marker(primary, 0, 1, 0);
    }
    if (opal_error_graph_bytes(primary) + opal_error_graph_bytes(cause) > OPAL_ERROR_MAX_BYTES) {
        return opal_error_clone_with_marker(primary, 0, 0, 1);
    }

    OpalErrorMeta* derived_meta = NULL;
    char* derived = opal_error_clone_identity(primary, &derived_meta);
    if (derived_meta == NULL) {
        return opal_error_clone_with_marker(primary, 0, 0, 1);
    }
    derived_meta->suppressed[derived_meta->suppressed_count] = cause;
    derived_meta->suppressed_count += 1;
    return derived;
}

FsStringResult error_cause(const char* error_value) {
    FsStringResult result;
    result.value = NULL;
    result.error = NULL;
    OpalErrorMeta* meta = opal_error_find_meta(error_value);
    if (meta != NULL && meta->cause != NULL) {
        result.value = (char*)meta->cause;
        return result;
    }
    result.error = opal_error_new("ErrorAttachmentAbsentError");
    return result;
}

int64_t error_suppressed_length(const char* error_value) {
    OpalErrorMeta* meta = opal_error_find_meta(error_value);
    return meta != NULL ? meta->suppressed_count : 0;
}

FsStringResult error_suppressed_at(const char* error_value, int64_t index) {
    FsStringResult result;
    result.value = NULL;
    result.error = NULL;
    OpalErrorMeta* meta = opal_error_find_meta(error_value);
    if (meta != NULL && index >= 0 && index < meta->suppressed_count) {
        result.value = (char*)meta->suppressed[index];
        return result;
    }
    result.error = opal_error_new("IndexOutOfBoundsError");
    return result;
}

OpalErrorTruncation* error_attachment_truncation(const char* error_value) {
    OpalErrorTruncation* truncation = (OpalErrorTruncation*)malloc(sizeof(OpalErrorTruncation));
    if (truncation == NULL) {
        return &opal_error_empty_truncation;
    }
    OpalErrorMeta* meta = opal_error_find_meta(error_value);
    *truncation = meta != NULL ? meta->truncation : opal_error_empty_truncation;
    return truncation;
}

int8_t error_attachment_truncation_cause_depth(OpalErrorTruncation* truncation) {
    return truncation != NULL ? truncation->cause_depth : 0;
}

int8_t error_attachment_truncation_suppressed_count(OpalErrorTruncation* truncation) {
    return truncation != NULL ? truncation->suppressed_count : 0;
}

int8_t error_attachment_truncation_bytes(OpalErrorTruncation* truncation) {
    return truncation != NULL ? truncation->bytes : 0;
}

void opal_runtime_error(const char* message) {
    fprintf(stderr, "%s\n", message != NULL ? message : "Runtime error");
    exit(1);
}
