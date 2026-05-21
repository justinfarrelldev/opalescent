#include <stdio.h>
#include <stdlib.h>

#include "runtime/opal_rc.h"

static const char* counter_label(OpalRcDebugCounterKind kind) {
    switch (kind) {
        case OPAL_RC_DEBUG_COUNTER_STRINGS:
            return "strings";
        case OPAL_RC_DEBUG_COUNTER_ARRAYS:
            return "arrays";
        case OPAL_RC_DEBUG_COUNTER_BYTES:
            return "bytes";
        case OPAL_RC_DEBUG_COUNTER_BUILDERS:
            return "builders";
        case OPAL_RC_DEBUG_COUNTER_FILESYSTEM_OBJECTS:
            return "filesystem_objects";
        case OPAL_RC_DEBUG_COUNTER_METADATA_PERMISSIONS:
            return "metadata_permissions";
        case OPAL_RC_DEBUG_COUNTER_ERROR_PAYLOADS:
            return "error_payloads";
        case OPAL_RC_DEBUG_COUNTER_RC_CHILD_ARRAYS:
            return "rc_child_arrays";
        default:
            return "unknown";
    }
}

static int detect_counter_imbalance(OpalRcDebugCounterKind kind) {
    size_t alloc_count = opal_rc_debug_alloc_count_for_test(kind);
    size_t free_count = opal_rc_debug_free_count_for_test(kind);
    size_t live_count = opal_rc_debug_live_count_for_test(kind);

    printf(
        "fixture_counter:%s alloc=%zu free=%zu live=%zu\n",
        counter_label(kind),
        alloc_count,
        free_count,
        live_count
    );

    if (alloc_count == free_count && live_count == 0) {
        printf("fixture_status=balanced\n");
        return 0;
    }

    printf(
        "fixture_status=imbalance-detected\n"
        "fixture_message=rc counter imbalance detected for %s (alloc=%zu free=%zu live=%zu)\n",
        counter_label(kind),
        alloc_count,
        free_count,
        live_count
    );
    return 1;
}

int main(void) {
    int detection = 0;
    opal_rc_debug_reset_counters_for_test();
    opal_rc_debug_note_alloc(OPAL_RC_DEBUG_COUNTER_RC_CHILD_ARRAYS);
    detection = detect_counter_imbalance(OPAL_RC_DEBUG_COUNTER_RC_CHILD_ARRAYS);
    opal_rc_debug_note_free(OPAL_RC_DEBUG_COUNTER_RC_CHILD_ARRAYS);
    return detection == 1 ? 0 : 2;
}
