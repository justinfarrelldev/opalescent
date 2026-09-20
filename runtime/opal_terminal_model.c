#include "opal_portability.h"
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>

#if defined(OPAL_ENABLE_INTERNAL_TESTING)
#include "opal_test_alloc.h"
#endif

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

typedef struct OpalTerminalInvalidOptionsSlot {
    bool used;
    uint8_t kind;
    uint64_t required;
    uint64_t configured;
    char identity[sizeof("TerminalSessionOptionsError.InvalidOptions")];
} OpalTerminalInvalidOptionsSlot;

enum {
    OPAL_TERMINAL_OPTIONS_MAGIC = 0x4F54534Fu,
    OPAL_TERMINAL_INVALID_OPTIONS_RETAINED_BYTES = 2,
    OPAL_TERMINAL_INVALID_OPTIONS_CORRELATED_EVENTS = 3,
    OPAL_TERMINAL_INVALID_OPTIONS_POOL_CAPACITY = 256
};

static const char* OPAL_TERMINAL_MODEL_ALLOCATION_FAILURE_ERROR =
    "AllocationFailureError";
static const char* OPAL_TERMINAL_MODEL_CONSTRAINT_VIOLATION_ERROR =
    "ConstraintViolationError";
static const char* OPAL_TERMINAL_MODEL_SESSION_OPTIONS_INVALID_ERROR =
    "TerminalSessionOptionsError.InvalidOptions";
static OpalTerminalInvalidOptionsSlot
    opal_terminal_invalid_options_slots[OPAL_TERMINAL_INVALID_OPTIONS_POOL_CAPACITY];
static size_t opal_terminal_invalid_options_next_slot = 0u;

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

static void* opal_terminal_copy_bytes(const void* source, size_t size) {
    void* copy = malloc(size);
    if (copy == NULL) {
        return NULL;
    }
    memcpy(copy, source, size);
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

static bool opal_terminal_options_valid(const OpalTerminalSessionOptions* options) {
    return options != NULL && options->magic == OPAL_TERMINAL_OPTIONS_MAGIC;
}

static int64_t opal_terminal_tag(const void* opaque_value) {
    const OpalTerminalTaggedValue* value = (const OpalTerminalTaggedValue*)opaque_value;
    return value != NULL ? value->tag : 0;
}

static int32_t opal_terminal_i32_box_value(const void* opaque_box) {
    const OpalTerminalI32Constrained* box = (const OpalTerminalI32Constrained*)opaque_box;
    return box != NULL ? box->value : 0;
}

static bool opal_terminal_limits_input_valid(
    const OpalTerminalSessionResourceLimitsInputRecord* limits
) {
    return limits != NULL && limits->input_sequence_timeout != NULL
        && limits->maximum_committed_text_bytes != NULL
        && limits->maximum_composition_preedit_bytes != NULL
        && limits->maximum_paste_chunk_bytes != NULL
        && limits->maximum_unknown_chunk_bytes != NULL
        && limits->maximum_pending_sequence_bytes != NULL
        && limits->maximum_retained_events != NULL
        && limits->maximum_retained_bytes != NULL
        && limits->maximum_correlated_events != NULL
        && limits->maximum_correlated_bytes != NULL
        && limits->maximum_diagnostics != NULL
        && limits->maximum_diagnostic_bytes != NULL;
}

static const char* opal_terminal_record_invalid_options(
    uint8_t kind,
    uint64_t required,
    uint64_t configured
) {
    size_t slot_index;
    OpalTerminalInvalidOptionsSlot* slot;

    if (opal_terminal_invalid_options_next_slot
        >= (size_t)OPAL_TERMINAL_INVALID_OPTIONS_POOL_CAPACITY) {
        return OPAL_TERMINAL_MODEL_SESSION_OPTIONS_INVALID_ERROR;
    }

    slot_index = opal_terminal_invalid_options_next_slot;
    slot = &opal_terminal_invalid_options_slots[slot_index];
    memset(slot, 0, sizeof(*slot));
    memcpy(
        slot->identity,
        OPAL_TERMINAL_MODEL_SESSION_OPTIONS_INVALID_ERROR,
        sizeof(slot->identity)
    );
    slot->used = true;
    slot->kind = kind;
    slot->required = required;
    slot->configured = configured;
    opal_terminal_invalid_options_next_slot = slot_index + 1u;
    return slot->identity;
}

static const OpalTerminalInvalidOptionsSlot* opal_terminal_find_invalid_options_slot(
    const char* error
) {
    size_t index;
    if (error == NULL) {
        return NULL;
    }
    for (index = 0u; index < opal_terminal_invalid_options_next_slot; ++index) {
        const OpalTerminalInvalidOptionsSlot* slot = &opal_terminal_invalid_options_slots[index];
        if (slot->used && slot->identity == error) {
            return slot;
        }
    }
    return NULL;
}

void* terminal_session_options_default(void) {
    OpalTerminalSessionOptions* options =
        (OpalTerminalSessionOptions*)opal_terminal_calloc(1u, sizeof(OpalTerminalSessionOptions));
    if (options == NULL) {
        return NULL;
    }
    options->magic = OPAL_TERMINAL_OPTIONS_MAGIC;
    options->feature_policy.mouse_tracking_tag = 1;
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
    OpalTerminalI32Constrained* boxed;
    if (value < min || value > max) {
        return opal_terminal_handle_error(OPAL_TERMINAL_MODEL_CONSTRAINT_VIOLATION_ERROR);
    }
    boxed = (OpalTerminalI32Constrained*)calloc(1u, sizeof(OpalTerminalI32Constrained));
    if (boxed == NULL) {
        return opal_terminal_handle_error(OPAL_TERMINAL_MODEL_ALLOCATION_FAILURE_ERROR);
    }
    boxed->value = value;
    return opal_terminal_handle_success(boxed);
}

FsHandleResult opal_terminal_constrain_u8_control_code(uint8_t value) {
    OpalTerminalU8Constrained* boxed;
    if (!(value <= 31u || value == 127u)) {
        return opal_terminal_handle_error(OPAL_TERMINAL_MODEL_CONSTRAINT_VIOLATION_ERROR);
    }
    boxed = (OpalTerminalU8Constrained*)calloc(1u, sizeof(OpalTerminalU8Constrained));
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
    const OpalTerminalSessionFeaturePolicyInputRecord* policy =
        (const OpalTerminalSessionFeaturePolicyInputRecord*)opaque_policy;
    OpalTerminalSessionOptions* copy;
    if (!opal_terminal_options_valid(options) || policy == NULL || policy->mouse_tracking == NULL) {
        return opal_terminal_handle_error(OPAL_TERMINAL_MODEL_ALLOCATION_FAILURE_ERROR);
    }
    copy = (OpalTerminalSessionOptions*)opal_terminal_copy_bytes(
        options,
        sizeof(OpalTerminalSessionOptions)
    );
    if (copy == NULL) {
        return opal_terminal_handle_error(OPAL_TERMINAL_MODEL_ALLOCATION_FAILURE_ERROR);
    }
    copy->feature_policy.mouse_tracking_tag = opal_terminal_tag(policy->mouse_tracking);
    copy->feature_policy.use_alternate_screen = policy->use_alternate_screen;
    copy->feature_policy.hide_cursor = policy->hide_cursor;
    copy->feature_policy.enable_bracketed_paste = policy->enable_bracketed_paste;
    copy->feature_policy.require_trusted_paste_framing = policy->require_trusted_paste_framing;
    copy->feature_policy.enable_enhanced_key_identity = policy->enable_enhanced_key_identity;
    copy->feature_policy.enable_focus_events = policy->enable_focus_events;
    copy->feature_policy.capture_control_keys = policy->capture_control_keys;
    copy->feature_policy.require_requested_features = policy->require_requested_features;
    return opal_terminal_handle_success(copy);
}

FsHandleResult terminal_session_options_with_resource_limits(
    void* opaque_options,
    void* opaque_limits
) {
    OpalTerminalSessionOptions* options = (OpalTerminalSessionOptions*)opaque_options;
    const OpalTerminalSessionResourceLimitsInputRecord* limits =
        (const OpalTerminalSessionResourceLimitsInputRecord*)opaque_limits;
    OpalTerminalSessionOptions* copy;
    if (!opal_terminal_options_valid(options) || !opal_terminal_limits_input_valid(limits)) {
        return opal_terminal_handle_error(OPAL_TERMINAL_MODEL_ALLOCATION_FAILURE_ERROR);
    }
    copy = (OpalTerminalSessionOptions*)opal_terminal_copy_bytes(
        options,
        sizeof(OpalTerminalSessionOptions)
    );
    if (copy == NULL) {
        return opal_terminal_handle_error(OPAL_TERMINAL_MODEL_ALLOCATION_FAILURE_ERROR);
    }
    copy->resource_limits.input_sequence_timeout =
        opal_terminal_i32_box_value(limits->input_sequence_timeout);
    copy->resource_limits.maximum_committed_text_bytes =
        opal_terminal_i32_box_value(limits->maximum_committed_text_bytes);
    copy->resource_limits.maximum_composition_preedit_bytes =
        opal_terminal_i32_box_value(limits->maximum_composition_preedit_bytes);
    copy->resource_limits.maximum_paste_chunk_bytes =
        opal_terminal_i32_box_value(limits->maximum_paste_chunk_bytes);
    copy->resource_limits.maximum_unknown_chunk_bytes =
        opal_terminal_i32_box_value(limits->maximum_unknown_chunk_bytes);
    copy->resource_limits.maximum_pending_sequence_bytes =
        opal_terminal_i32_box_value(limits->maximum_pending_sequence_bytes);
    copy->resource_limits.maximum_retained_events =
        opal_terminal_i32_box_value(limits->maximum_retained_events);
    copy->resource_limits.maximum_retained_bytes =
        opal_terminal_i32_box_value(limits->maximum_retained_bytes);
    copy->resource_limits.maximum_correlated_events =
        opal_terminal_i32_box_value(limits->maximum_correlated_events);
    copy->resource_limits.maximum_correlated_bytes =
        opal_terminal_i32_box_value(limits->maximum_correlated_bytes);
    copy->resource_limits.maximum_diagnostics =
        opal_terminal_i32_box_value(limits->maximum_diagnostics);
    copy->resource_limits.maximum_diagnostic_bytes =
        opal_terminal_i32_box_value(limits->maximum_diagnostic_bytes);
    return opal_terminal_handle_success(copy);
}

FsHandleResult terminal_session_options_validate(void* opaque_options) {
    OpalTerminalSessionOptions* options = (OpalTerminalSessionOptions*)opaque_options;
    uint64_t retained_bytes;
    uint64_t correlated_bytes;
    uint64_t retained_events;
    uint64_t correlated_events;
    if (!opal_terminal_options_valid(options)) {
        return opal_terminal_handle_error(OPAL_TERMINAL_MODEL_SESSION_OPTIONS_INVALID_ERROR);
    }
    retained_bytes = (uint64_t)options->resource_limits.maximum_retained_bytes;
    correlated_bytes = (uint64_t)options->resource_limits.maximum_correlated_bytes;
    if (retained_bytes < correlated_bytes) {
        return opal_terminal_handle_error(opal_terminal_record_invalid_options(
            OPAL_TERMINAL_INVALID_OPTIONS_RETAINED_BYTES,
            correlated_bytes,
            retained_bytes
        ));
    }
    retained_events = (uint64_t)options->resource_limits.maximum_retained_events;
    correlated_events = (uint64_t)options->resource_limits.maximum_correlated_events;
    if (correlated_events > retained_events) {
        return opal_terminal_handle_error(opal_terminal_record_invalid_options(
            OPAL_TERMINAL_INVALID_OPTIONS_CORRELATED_EVENTS,
            correlated_events,
            retained_events
        ));
    }
    return opal_terminal_handle_success(opaque_options);
}

FsHandleResult trusted_terminal_output_from_application_text(const char* text) {
    char* trusted = opal_terminal_duplicate(text);
    if (trusted == NULL) {
        return opal_terminal_handle_error(OPAL_TERMINAL_MODEL_ALLOCATION_FAILURE_ERROR);
    }
    return opal_terminal_handle_success(trusted);
}

#ifdef OPAL_ENABLE_INTERNAL_TESTING
void opal_terminal_test_reset_invalid_options_errors(void) {
    memset(
        opal_terminal_invalid_options_slots,
        0,
        sizeof(opal_terminal_invalid_options_slots)
    );
    opal_terminal_invalid_options_next_slot = 0u;
}

int32_t opal_terminal_test_options_use_alternate_screen(void* opaque_options) {
    const OpalTerminalSessionOptions* options =
        (const OpalTerminalSessionOptions*)opaque_options;
    return opal_terminal_options_valid(options)
        ? (options->feature_policy.use_alternate_screen ? 1 : 0)
        : 0;
}

int64_t opal_terminal_test_options_mouse_tracking_tag(void* opaque_options) {
    const OpalTerminalSessionOptions* options =
        (const OpalTerminalSessionOptions*)opaque_options;
    return opal_terminal_options_valid(options)
        ? options->feature_policy.mouse_tracking_tag
        : 0;
}

int32_t opal_terminal_test_options_maximum_retained_bytes(void* opaque_options) {
    const OpalTerminalSessionOptions* options =
        (const OpalTerminalSessionOptions*)opaque_options;
    return opal_terminal_options_valid(options)
        ? options->resource_limits.maximum_retained_bytes
        : 0;
}

int32_t opal_terminal_test_options_maximum_correlated_bytes(void* opaque_options) {
    const OpalTerminalSessionOptions* options =
        (const OpalTerminalSessionOptions*)opaque_options;
    return opal_terminal_options_valid(options)
        ? options->resource_limits.maximum_correlated_bytes
        : 0;
}

int32_t opal_terminal_test_options_maximum_retained_events(void* opaque_options) {
    const OpalTerminalSessionOptions* options =
        (const OpalTerminalSessionOptions*)opaque_options;
    return opal_terminal_options_valid(options)
        ? options->resource_limits.maximum_retained_events
        : 0;
}

int32_t opal_terminal_test_options_maximum_correlated_events(void* opaque_options) {
    const OpalTerminalSessionOptions* options =
        (const OpalTerminalSessionOptions*)opaque_options;
    return opal_terminal_options_valid(options)
        ? options->resource_limits.maximum_correlated_events
        : 0;
}

int32_t opal_terminal_test_invalid_options_kind(const char* error) {
    const OpalTerminalInvalidOptionsSlot* slot = opal_terminal_find_invalid_options_slot(error);
    return slot != NULL ? (int32_t)slot->kind : 0;
}

uint64_t opal_terminal_test_invalid_options_required(const char* error) {
    const OpalTerminalInvalidOptionsSlot* slot = opal_terminal_find_invalid_options_slot(error);
    return slot != NULL ? slot->required : 0u;
}

uint64_t opal_terminal_test_invalid_options_configured(const char* error) {
    const OpalTerminalInvalidOptionsSlot* slot = opal_terminal_find_invalid_options_slot(error);
    return slot != NULL ? slot->configured : 0u;
}
#endif
