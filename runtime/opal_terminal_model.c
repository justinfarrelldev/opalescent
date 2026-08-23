#include "opal_portability.h"
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#ifndef OPAL_FS_HANDLE_RESULT_DEFINED
typedef struct {
    void* value;
    const char* error;
} FsHandleResult;
#define OPAL_FS_HANDLE_RESULT_DEFINED 1
#endif

typedef struct {
    int64_t tag;
    uint8_t payload[64];
} OpalTerminalTaggedValue;

typedef struct {
    bool use_alternate_screen;
    bool hide_cursor;
    bool enable_bracketed_paste;
    bool require_trusted_paste_framing;
    bool enable_enhanced_key_identity;
    bool enable_focus_events;
    void* mouse_tracking;
    bool capture_control_keys;
    bool require_requested_features;
} OpalTerminalSessionFeaturePolicyInputRecord;

typedef struct {
    int64_t mouse_tracking_tag;
    bool use_alternate_screen;
    bool hide_cursor;
    bool enable_bracketed_paste;
    bool require_trusted_paste_framing;
    bool enable_enhanced_key_identity;
    bool enable_focus_events;
    bool capture_control_keys;
    bool require_requested_features;
} OpalTerminalSessionFeaturePolicyRecord;

typedef struct {
    void* input_sequence_timeout;
    void* maximum_committed_text_bytes;
    void* maximum_composition_preedit_bytes;
    void* maximum_paste_chunk_bytes;
    void* maximum_unknown_chunk_bytes;
    void* maximum_pending_sequence_bytes;
    void* maximum_retained_events;
    void* maximum_retained_bytes;
    void* maximum_correlated_events;
    void* maximum_correlated_bytes;
    void* maximum_diagnostics;
    void* maximum_diagnostic_bytes;
} OpalTerminalSessionResourceLimitsInputRecord;

typedef struct {
    int32_t input_sequence_timeout;
    int32_t maximum_committed_text_bytes;
    int32_t maximum_composition_preedit_bytes;
    int32_t maximum_paste_chunk_bytes;
    int32_t maximum_unknown_chunk_bytes;
    int32_t maximum_pending_sequence_bytes;
    int32_t maximum_retained_events;
    int32_t maximum_retained_bytes;
    int32_t maximum_correlated_events;
    int32_t maximum_correlated_bytes;
    int32_t maximum_diagnostics;
    int32_t maximum_diagnostic_bytes;
} OpalTerminalSessionResourceLimitsRecord;

typedef struct OpalTerminalSessionOptions {
    uint32_t magic;
    OpalTerminalSessionFeaturePolicyRecord feature_policy;
    OpalTerminalSessionResourceLimitsRecord resource_limits;
} OpalTerminalSessionOptions;

typedef struct OpalTerminalI32Constrained {
    int32_t value;
} OpalTerminalI32Constrained;

typedef struct OpalTerminalU8Constrained {
    uint8_t value;
} OpalTerminalU8Constrained;

typedef struct OpalTerminalCapabilityValue {
    int64_t tag;
    int32_t evidence;
    int32_t count;
} OpalTerminalCapabilityValue;

typedef struct OpalTerminalCapabilities {
    uint32_t magic;
    uint64_t hidden_stream_id;
    int32_t hidden_correlated_event_limit;
    OpalTerminalCapabilityValue ordinary[9];
    OpalTerminalCapabilityValue trusted_paste;
    OpalTerminalCapabilityValue color;
} OpalTerminalCapabilities;

typedef struct OpalTerminalDiagnostic {
    uint32_t magic;
    int64_t backend;
    int64_t operation;
    int64_t stage;
    int64_t coordinator_state;
    int64_t session_state;
    int64_t os_code_tag;
    int64_t os_code_i64;
    char* detail;
    int64_t retryability;
    bool was_truncated;
    uint64_t accounted_bytes;
} OpalTerminalDiagnostic;

typedef struct OpalTerminalDiagnosticCollection {
    uint32_t magic;
    OpalTerminalDiagnostic** retained;
    int64_t retained_length;
    uint64_t retained_count;
    uint64_t omitted_count;
    uint64_t retained_bytes;
    uint64_t omitted_bytes;
    bool was_truncated;
} OpalTerminalDiagnosticCollection;

enum {
    OPAL_TERMINAL_OPTIONS_MAGIC = 0x4F54534Fu,
    OPAL_TERMINAL_CAPABILITIES_MAGIC = 0x4F544341u,
    OPAL_TERMINAL_DIAGNOSTIC_MAGIC = 0x4F544449u,
    OPAL_TERMINAL_COLLECTION_MAGIC = 0x4F54434Cu,
    OPAL_TERMINAL_MARKER_SUPPORTED = 1,
    OPAL_TERMINAL_MARKER_UNSUPPORTED = 2,
    OPAL_TERMINAL_MARKER_TRUNCATED = 3
};

static const char* OPAL_TERMINAL_MODEL_ALLOCATION_FAILURE_ERROR =
    "AllocationFailureError";
static const char* OPAL_TERMINAL_MODEL_SESSION_OPTIONS_INVALID_ERROR =
    "TerminalSessionOptionsError.InvalidOptions";

static void* opal_terminal_calloc(size_t count, size_t size) {
    return calloc(count, size);
}

static char* opal_terminal_duplicate(const char* source) {
    const char* text = source != NULL ? source : "";
    size_t length = strlen(text);
    char* copy = (char*)malloc(length + 1u);
    if (copy == NULL) {
        return NULL;
    }
    memcpy(copy, text, length + 1u);
    return copy;
}

static FsHandleResult opal_terminal_handle_error(const char* error) {
    FsHandleResult result = { NULL, error };
    return result;
}

static FsHandleResult opal_terminal_handle_success(void* value) {
    FsHandleResult result = { value, NULL };
    return result;
}

static OpalTerminalCapabilityValue opal_terminal_capability_value(
    int64_t tag,
    int32_t evidence,
    int32_t count
) {
    OpalTerminalCapabilityValue value = { tag, evidence, count };
    return value;
}

static void* opal_terminal_copy_bytes(const void* source, size_t size) {
    void* copy = malloc(size);
    if (copy == NULL) {
        return NULL;
    }
    memcpy(copy, source, size);
    return copy;
}

static uint64_t opal_terminal_diagnostic_accounted_bytes(const char* detail) {
    size_t base_size = sizeof(OpalTerminalDiagnostic) + 32u;
    size_t detail_size = strlen(detail != NULL ? detail : "");
    return (uint64_t)base_size + (uint64_t)detail_size;
}

static uint64_t opal_terminal_collection_metadata_bytes(void) {
    return (uint64_t)(sizeof(OpalTerminalDiagnosticCollection) + 64u);
}

static bool opal_terminal_options_valid(const OpalTerminalSessionOptions* options) {
    return options != NULL && options->magic == OPAL_TERMINAL_OPTIONS_MAGIC;
}

static int64_t opal_terminal_tag(const OpalTerminalTaggedValue* value) {
    return value != NULL ? value->tag : 0;
}

static int32_t opal_terminal_i32_box_value(const void* opaque_box) {
    const OpalTerminalI32Constrained* box = (const OpalTerminalI32Constrained*)opaque_box;
    return box != NULL ? box->value : 0;
}

void* terminal_session_options_default(void) {
    OpalTerminalSessionOptions* options =
        (OpalTerminalSessionOptions*)opal_terminal_calloc(1u, sizeof(OpalTerminalSessionOptions));
    if (options == NULL) {
        return NULL;
    }
    options->magic = OPAL_TERMINAL_OPTIONS_MAGIC;
    options->feature_policy.use_alternate_screen = false;
    options->feature_policy.hide_cursor = false;
    options->feature_policy.enable_bracketed_paste = false;
    options->feature_policy.require_trusted_paste_framing = false;
    options->feature_policy.enable_enhanced_key_identity = false;
    options->feature_policy.enable_focus_events = false;
    options->feature_policy.mouse_tracking_tag = 1;
    options->feature_policy.capture_control_keys = false;
    options->feature_policy.require_requested_features = false;
    options->resource_limits.input_sequence_timeout = 25;
    options->resource_limits.maximum_committed_text_bytes = 4096;
    options->resource_limits.maximum_composition_preedit_bytes = 4096;
    options->resource_limits.maximum_paste_chunk_bytes = 4096;
    options->resource_limits.maximum_unknown_chunk_bytes = 1024;
    options->resource_limits.maximum_pending_sequence_bytes = 1024;
    options->resource_limits.maximum_retained_events = 1024;
    options->resource_limits.maximum_retained_bytes = 0x00100000;
    options->resource_limits.maximum_correlated_events = 64;
    options->resource_limits.maximum_correlated_bytes = 0x00010000;
    options->resource_limits.maximum_diagnostics = 16;
    options->resource_limits.maximum_diagnostic_bytes = 0x00010000;
    return options;
}

FsHandleResult opal_terminal_constrain_i32_range(int32_t value, int32_t min, int32_t max) {
    if (value < min || value > max) {
        return opal_terminal_handle_error("ConstraintViolationError");
    }
    OpalTerminalI32Constrained* boxed =
        (OpalTerminalI32Constrained*)calloc(1u, sizeof(OpalTerminalI32Constrained));
    if (boxed == NULL) {
        return opal_terminal_handle_error(OPAL_TERMINAL_MODEL_ALLOCATION_FAILURE_ERROR);
    }
    boxed->value = value;
    return opal_terminal_handle_success(boxed);
}

FsHandleResult opal_terminal_constrain_u8_control_code(uint8_t value) {
    if (!(value <= 31u || value == 127u)) {
        return opal_terminal_handle_error("ConstraintViolationError");
    }
    OpalTerminalU8Constrained* boxed =
        (OpalTerminalU8Constrained*)calloc(1u, sizeof(OpalTerminalU8Constrained));
    if (boxed == NULL) {
        return opal_terminal_handle_error(OPAL_TERMINAL_MODEL_ALLOCATION_FAILURE_ERROR);
    }
    boxed->value = value;
    return opal_terminal_handle_success(boxed);
}

FsHandleResult terminal_session_options_with_feature_policy(
    void* opaque_options,
    void* opaque_policy
) {
    OpalTerminalSessionOptions* options = (OpalTerminalSessionOptions*)opaque_options;
    OpalTerminalSessionFeaturePolicyInputRecord* policy =
        (OpalTerminalSessionFeaturePolicyInputRecord*)opaque_policy;
    if (!opal_terminal_options_valid(options) || policy == NULL) {
        return opal_terminal_handle_error(OPAL_TERMINAL_MODEL_ALLOCATION_FAILURE_ERROR);
    }
    OpalTerminalSessionOptions* copy =
        (OpalTerminalSessionOptions*)opal_terminal_copy_bytes(options, sizeof(OpalTerminalSessionOptions));
    if (copy == NULL) {
        return opal_terminal_handle_error(OPAL_TERMINAL_MODEL_ALLOCATION_FAILURE_ERROR);
    }
    copy->feature_policy.use_alternate_screen = policy->use_alternate_screen;
    copy->feature_policy.hide_cursor = policy->hide_cursor;
    copy->feature_policy.enable_bracketed_paste = policy->enable_bracketed_paste;
    copy->feature_policy.require_trusted_paste_framing = policy->require_trusted_paste_framing;
    copy->feature_policy.enable_enhanced_key_identity = policy->enable_enhanced_key_identity;
    copy->feature_policy.enable_focus_events = policy->enable_focus_events;
    copy->feature_policy.mouse_tracking_tag = opal_terminal_tag((OpalTerminalTaggedValue*)policy->mouse_tracking);
    copy->feature_policy.capture_control_keys = policy->capture_control_keys;
    copy->feature_policy.require_requested_features = policy->require_requested_features;
    return opal_terminal_handle_success(copy);
}

FsHandleResult terminal_session_options_with_resource_limits(
    void* opaque_options,
    void* opaque_limits
) {
    OpalTerminalSessionOptions* options = (OpalTerminalSessionOptions*)opaque_options;
    OpalTerminalSessionResourceLimitsInputRecord* limits =
        (OpalTerminalSessionResourceLimitsInputRecord*)opaque_limits;
    if (!opal_terminal_options_valid(options) || limits == NULL) {
        return opal_terminal_handle_error(OPAL_TERMINAL_MODEL_ALLOCATION_FAILURE_ERROR);
    }
    OpalTerminalSessionOptions* copy =
        (OpalTerminalSessionOptions*)opal_terminal_copy_bytes(options, sizeof(OpalTerminalSessionOptions));
    if (copy == NULL) {
        return opal_terminal_handle_error(OPAL_TERMINAL_MODEL_ALLOCATION_FAILURE_ERROR);
    }
    copy->resource_limits.input_sequence_timeout = opal_terminal_i32_box_value(limits->input_sequence_timeout);
    copy->resource_limits.maximum_committed_text_bytes = opal_terminal_i32_box_value(limits->maximum_committed_text_bytes);
    copy->resource_limits.maximum_composition_preedit_bytes = opal_terminal_i32_box_value(limits->maximum_composition_preedit_bytes);
    copy->resource_limits.maximum_paste_chunk_bytes = opal_terminal_i32_box_value(limits->maximum_paste_chunk_bytes);
    copy->resource_limits.maximum_unknown_chunk_bytes = opal_terminal_i32_box_value(limits->maximum_unknown_chunk_bytes);
    copy->resource_limits.maximum_pending_sequence_bytes = opal_terminal_i32_box_value(limits->maximum_pending_sequence_bytes);
    copy->resource_limits.maximum_retained_events = opal_terminal_i32_box_value(limits->maximum_retained_events);
    copy->resource_limits.maximum_retained_bytes = opal_terminal_i32_box_value(limits->maximum_retained_bytes);
    copy->resource_limits.maximum_correlated_events = opal_terminal_i32_box_value(limits->maximum_correlated_events);
    copy->resource_limits.maximum_correlated_bytes = opal_terminal_i32_box_value(limits->maximum_correlated_bytes);
    copy->resource_limits.maximum_diagnostics = opal_terminal_i32_box_value(limits->maximum_diagnostics);
    copy->resource_limits.maximum_diagnostic_bytes = opal_terminal_i32_box_value(limits->maximum_diagnostic_bytes);
    return opal_terminal_handle_success(copy);
}

FsHandleResult terminal_session_options_validate(void* opaque_options) {
    OpalTerminalSessionOptions* options = (OpalTerminalSessionOptions*)opaque_options;
    if (!opal_terminal_options_valid(options)) {
        return opal_terminal_handle_error(OPAL_TERMINAL_MODEL_SESSION_OPTIONS_INVALID_ERROR);
    }
    if ((uint64_t)options->resource_limits.maximum_retained_bytes <
        (uint64_t)options->resource_limits.maximum_correlated_bytes) {
        return opal_terminal_handle_error(OPAL_TERMINAL_MODEL_SESSION_OPTIONS_INVALID_ERROR);
    }
    if ((uint64_t)options->resource_limits.maximum_correlated_events >
        (uint64_t)options->resource_limits.maximum_retained_events) {
        return opal_terminal_handle_error(OPAL_TERMINAL_MODEL_SESSION_OPTIONS_INVALID_ERROR);
    }
    return opal_terminal_handle_success(opaque_options);
}

void* terminal_capabilities_feature(void* opaque_capabilities, void* opaque_feature) {
    OpalTerminalCapabilities* capabilities = (OpalTerminalCapabilities*)opaque_capabilities;
    OpalTerminalTaggedValue* feature = (OpalTerminalTaggedValue*)opaque_feature;
    if (capabilities == NULL || capabilities->magic != OPAL_TERMINAL_CAPABILITIES_MAGIC || feature == NULL) {
        return NULL;
    }
    int64_t tag = opal_terminal_tag(feature);
    if (tag < 1 || tag > 9) {
        return NULL;
    }
    return opal_terminal_copy_bytes(&capabilities->ordinary[tag - 1], sizeof(OpalTerminalCapabilityValue));
}

void* terminal_capabilities_trusted_paste_framing(void* opaque_capabilities) {
    OpalTerminalCapabilities* capabilities = (OpalTerminalCapabilities*)opaque_capabilities;
    if (capabilities == NULL || capabilities->magic != OPAL_TERMINAL_CAPABILITIES_MAGIC) {
        return NULL;
    }
    return opal_terminal_copy_bytes(&capabilities->trusted_paste, sizeof(OpalTerminalCapabilityValue));
}

void* terminal_capabilities_color(void* opaque_capabilities) {
    OpalTerminalCapabilities* capabilities = (OpalTerminalCapabilities*)opaque_capabilities;
    if (capabilities == NULL || capabilities->magic != OPAL_TERMINAL_CAPABILITIES_MAGIC) {
        return NULL;
    }
    return opal_terminal_copy_bytes(&capabilities->color, sizeof(OpalTerminalCapabilityValue));
}

FsHandleResult trusted_terminal_output_from_application_text(const char* text) {
    char* trusted = opal_terminal_duplicate(text);
    if (trusted == NULL) {
        return opal_terminal_handle_error(OPAL_TERMINAL_MODEL_ALLOCATION_FAILURE_ERROR);
    }
    return opal_terminal_handle_success(trusted);
}

static bool opal_terminal_should_escape(uint32_t codepoint) {
    return (codepoint <= 0x1Fu)
        || (codepoint >= 0x7Fu && codepoint <= 0x9Fu)
        || codepoint == 0x200Eu || codepoint == 0x200Fu
        || (codepoint >= 0x202Au && codepoint <= 0x202Eu)
        || (codepoint >= 0x2066u && codepoint <= 0x2069u)
        || (codepoint >= 0xFDD0u && codepoint <= 0xFDEFu)
        || ((codepoint & 0xFFFFu) == 0xFFFEu)
        || ((codepoint & 0xFFFFu) == 0xFFFFu);
}

static char* opal_terminal_escape_and_bound(const char* input) {
    const char* text = input != NULL ? input : "";
    size_t capacity = 0x00010000u;
    char* output = (char*)calloc(capacity + 1u, 1u);
    size_t written = 0u;
    if (output == NULL) {
        return NULL;
    }
    for (const unsigned char* cursor = (const unsigned char*)text; *cursor != '\0'; ++cursor) {
        char fragment[16];
        size_t fragment_length = 0u;
        if (opal_terminal_should_escape(*cursor)) {
            fragment_length = (size_t)snprintf(fragment, sizeof(fragment), "\\u{%X}", *cursor);
        } else {
            fragment[0] = (char)*cursor;
            fragment[1] = '\0';
            fragment_length = 1u;
        }
        if (written + fragment_length > capacity) {
            const char* marker = "...[truncated]";
            size_t marker_length = strlen(marker);
            if (marker_length < capacity) {
                written = capacity - marker_length;
                memcpy(output + written, marker, marker_length + 1u);
            }
            return output;
        }
        memcpy(output + written, fragment, fragment_length);
        written += fragment_length;
        output[written] = '\0';
    }
    return output;
}

FsHandleResult safe_terminal_diagnostic_format(void* opaque_diagnostic) {
    OpalTerminalDiagnostic* diagnostic = (OpalTerminalDiagnostic*)opaque_diagnostic;
    if (diagnostic == NULL || diagnostic->magic != OPAL_TERMINAL_DIAGNOSTIC_MAGIC) {
        return opal_terminal_handle_error(OPAL_TERMINAL_MODEL_ALLOCATION_FAILURE_ERROR);
    }
    char buffer[512];
    snprintf(
        buffer,
        sizeof(buffer),
        "backend=%lld operation=%lld stage=%lld coordinator=%lld session=%lld retryability=%lld os_tag=%lld os_value=%lld truncated=%d detail=%s",
        (long long)diagnostic->backend,
        (long long)diagnostic->operation,
        (long long)diagnostic->stage,
        (long long)diagnostic->coordinator_state,
        (long long)diagnostic->session_state,
        (long long)diagnostic->retryability,
        (long long)diagnostic->os_code_tag,
        (long long)diagnostic->os_code_i64,
        diagnostic->was_truncated ? 1 : 0,
        diagnostic->detail != NULL ? diagnostic->detail : ""
    );
    char* escaped = opal_terminal_escape_and_bound(buffer);
    if (escaped == NULL) {
        return opal_terminal_handle_error(OPAL_TERMINAL_MODEL_ALLOCATION_FAILURE_ERROR);
    }
    return opal_terminal_handle_success(escaped);
}

FsHandleResult safe_terminal_diagnostic_collection_format(void* opaque_collection) {
    OpalTerminalDiagnosticCollection* collection = (OpalTerminalDiagnosticCollection*)opaque_collection;
    if (collection == NULL || collection->magic != OPAL_TERMINAL_COLLECTION_MAGIC) {
        return opal_terminal_handle_error(OPAL_TERMINAL_MODEL_ALLOCATION_FAILURE_ERROR);
    }
    char buffer[512];
    snprintf(
        buffer,
        sizeof(buffer),
        "retained_count=%llu omitted_count=%llu retained_bytes=%llu omitted_bytes=%llu collection_truncated=%d",
        (unsigned long long)collection->retained_count,
        (unsigned long long)collection->omitted_count,
        (unsigned long long)collection->retained_bytes,
        (unsigned long long)collection->omitted_bytes,
        collection->was_truncated ? 1 : 0
    );
    char* escaped = opal_terminal_escape_and_bound(buffer);
    if (escaped == NULL) {
        return opal_terminal_handle_error(OPAL_TERMINAL_MODEL_ALLOCATION_FAILURE_ERROR);
    }
    return opal_terminal_handle_success(escaped);
}

void* terminal_diagnostic_backend(void* opaque_diagnostic) {
    OpalTerminalDiagnostic* diagnostic = (OpalTerminalDiagnostic*)opaque_diagnostic;
    if (diagnostic == NULL) {
        return NULL;
    }
    OpalTerminalTaggedValue* value = (OpalTerminalTaggedValue*)calloc(1u, sizeof(OpalTerminalTaggedValue));
    if (value == NULL) {
        return NULL;
    }
    value->tag = diagnostic->backend;
    return value;
}

void* terminal_diagnostic_operation(void* opaque_diagnostic) {
    OpalTerminalDiagnostic* diagnostic = (OpalTerminalDiagnostic*)opaque_diagnostic;
    if (diagnostic == NULL) {
        return NULL;
    }
    OpalTerminalTaggedValue* value = (OpalTerminalTaggedValue*)calloc(1u, sizeof(OpalTerminalTaggedValue));
    if (value == NULL) {
        return NULL;
    }
    value->tag = diagnostic->operation;
    return value;
}

void* terminal_diagnostic_stage(void* opaque_diagnostic) {
    OpalTerminalDiagnostic* diagnostic = (OpalTerminalDiagnostic*)opaque_diagnostic;
    if (diagnostic == NULL) {
        return NULL;
    }
    OpalTerminalTaggedValue* value = (OpalTerminalTaggedValue*)calloc(1u, sizeof(OpalTerminalTaggedValue));
    if (value == NULL) {
        return NULL;
    }
    value->tag = diagnostic->stage;
    return value;
}

void* terminal_diagnostic_coordinator_state(void* opaque_diagnostic) {
    OpalTerminalDiagnostic* diagnostic = (OpalTerminalDiagnostic*)opaque_diagnostic;
    if (diagnostic == NULL) {
        return NULL;
    }
    OpalTerminalTaggedValue* value = (OpalTerminalTaggedValue*)calloc(1u, sizeof(OpalTerminalTaggedValue));
    if (value == NULL) {
        return NULL;
    }
    value->tag = diagnostic->coordinator_state;
    return value;
}

void* terminal_diagnostic_session_state(void* opaque_diagnostic) {
    OpalTerminalDiagnostic* diagnostic = (OpalTerminalDiagnostic*)opaque_diagnostic;
    if (diagnostic == NULL) {
        return NULL;
    }
    OpalTerminalTaggedValue* value = (OpalTerminalTaggedValue*)calloc(1u, sizeof(OpalTerminalTaggedValue));
    if (value == NULL) {
        return NULL;
    }
    value->tag = diagnostic->session_state;
    return value;
}

void* terminal_diagnostic_os_code(void* opaque_diagnostic) {
    OpalTerminalDiagnostic* diagnostic = (OpalTerminalDiagnostic*)opaque_diagnostic;
    if (diagnostic == NULL) {
        return NULL;
    }
    OpalTerminalTaggedValue* value = (OpalTerminalTaggedValue*)calloc(1u, sizeof(OpalTerminalTaggedValue));
    if (value == NULL) {
        return NULL;
    }
    value->tag = diagnostic->os_code_tag;
    memcpy(value->payload, &diagnostic->os_code_i64, sizeof(diagnostic->os_code_i64));
    return value;
}

void* terminal_diagnostic_detail(void* opaque_diagnostic) {
    OpalTerminalDiagnostic* diagnostic = (OpalTerminalDiagnostic*)opaque_diagnostic;
    if (diagnostic == NULL) {
        return NULL;
    }
    return opal_terminal_duplicate(diagnostic->detail);
}

void* terminal_diagnostic_retryability(void* opaque_diagnostic) {
    OpalTerminalDiagnostic* diagnostic = (OpalTerminalDiagnostic*)opaque_diagnostic;
    if (diagnostic == NULL) {
        return NULL;
    }
    OpalTerminalTaggedValue* value = (OpalTerminalTaggedValue*)calloc(1u, sizeof(OpalTerminalTaggedValue));
    if (value == NULL) {
        return NULL;
    }
    value->tag = diagnostic->retryability;
    return value;
}

_Bool terminal_diagnostic_was_truncated(void* opaque_diagnostic) {
    OpalTerminalDiagnostic* diagnostic = (OpalTerminalDiagnostic*)opaque_diagnostic;
    return diagnostic != NULL && diagnostic->was_truncated;
}

int64_t terminal_diagnostics_length(void* opaque_collection) {
    OpalTerminalDiagnosticCollection* collection = (OpalTerminalDiagnosticCollection*)opaque_collection;
    return collection != NULL ? collection->retained_length : 0;
}

void* terminal_diagnostics_at(void* opaque_collection, int64_t index) {
    OpalTerminalDiagnosticCollection* collection = (OpalTerminalDiagnosticCollection*)opaque_collection;
    if (collection == NULL || index < 0 || index >= collection->retained_length) {
        return NULL;
    }
    return collection->retained[index];
}

uint64_t terminal_diagnostics_retained_count(void* opaque_collection) {
    OpalTerminalDiagnosticCollection* collection = (OpalTerminalDiagnosticCollection*)opaque_collection;
    return collection != NULL ? collection->retained_count : 0u;
}

uint64_t terminal_diagnostics_omitted_count(void* opaque_collection) {
    OpalTerminalDiagnosticCollection* collection = (OpalTerminalDiagnosticCollection*)opaque_collection;
    return collection != NULL ? collection->omitted_count : 0u;
}

uint64_t terminal_diagnostics_retained_bytes(void* opaque_collection) {
    OpalTerminalDiagnosticCollection* collection = (OpalTerminalDiagnosticCollection*)opaque_collection;
    return collection != NULL ? collection->retained_bytes : 0u;
}

uint64_t terminal_diagnostics_omitted_bytes(void* opaque_collection) {
    OpalTerminalDiagnosticCollection* collection = (OpalTerminalDiagnosticCollection*)opaque_collection;
    return collection != NULL ? collection->omitted_bytes : 0u;
}

_Bool terminal_diagnostics_was_truncated(void* opaque_collection) {
    OpalTerminalDiagnosticCollection* collection = (OpalTerminalDiagnosticCollection*)opaque_collection;
    return collection != NULL && collection->was_truncated;
}

#ifdef OPAL_ENABLE_INTERNAL_TESTING
void* opal_terminal_test_make_capabilities(void) {
    OpalTerminalCapabilities* capabilities = (OpalTerminalCapabilities*)calloc(1u, sizeof(OpalTerminalCapabilities));
    if (capabilities == NULL) {
        return NULL;
    }
    capabilities->magic = OPAL_TERMINAL_CAPABILITIES_MAGIC;
    capabilities->hidden_stream_id = 1u;
    capabilities->hidden_correlated_event_limit = 64;
    for (int index = 0; index < 9; ++index) {
        capabilities->ordinary[index] = opal_terminal_capability_value(2, 1, 0);
    }
    capabilities->trusted_paste = opal_terminal_capability_value(3, 1, 0);
    capabilities->color = opal_terminal_capability_value(3, 3, 256);
    return capabilities;
}

void* opal_terminal_test_make_diagnostic(const char* detail, _Bool truncated) {
    OpalTerminalDiagnostic* diagnostic = (OpalTerminalDiagnostic*)calloc(1u, sizeof(OpalTerminalDiagnostic));
    if (diagnostic == NULL) {
        return NULL;
    }
    diagnostic->magic = OPAL_TERMINAL_DIAGNOSTIC_MAGIC;
    diagnostic->backend = 1;
    diagnostic->operation = 16;
    diagnostic->stage = 13;
    diagnostic->coordinator_state = 1;
    diagnostic->session_state = 1;
    diagnostic->os_code_tag = 2;
    diagnostic->os_code_i64 = 22;
    diagnostic->detail = opal_terminal_duplicate(detail);
    diagnostic->retryability = 1;
    diagnostic->was_truncated = truncated;
    diagnostic->accounted_bytes = opal_terminal_diagnostic_accounted_bytes(detail != NULL ? detail : "");
    return diagnostic;
}

void* opal_terminal_test_make_collection(void) {
    OpalTerminalDiagnosticCollection* collection = (OpalTerminalDiagnosticCollection*)calloc(1u, sizeof(OpalTerminalDiagnosticCollection));
    if (collection == NULL) {
        return NULL;
    }
    collection->magic = OPAL_TERMINAL_COLLECTION_MAGIC;
    collection->retained = (OpalTerminalDiagnostic**)calloc(1u, sizeof(OpalTerminalDiagnostic*));
    if (collection->retained == NULL) {
        free(collection);
        return NULL;
    }
    collection->retained[0] = (OpalTerminalDiagnostic*)opal_terminal_test_make_diagnostic("diag", false);
    collection->retained_length = 1;
    collection->retained_count = 1u;
    collection->omitted_count = 2u;
    collection->retained_bytes = opal_terminal_collection_metadata_bytes() + collection->retained[0]->accounted_bytes;
    collection->omitted_bytes = UINT64_MAX;
    collection->was_truncated = true;
    return collection;
}
#endif
