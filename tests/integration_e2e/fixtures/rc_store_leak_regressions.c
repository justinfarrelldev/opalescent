#include <stdio.h>
#include <stdlib.h>

#include "runtime/opal_rc.h"

typedef struct {
    OpalRcDebugCounterKind kind;
    const char* label;
} CounterLabel;

static const CounterLabel REQUIRED_COUNTERS[] = {
    { OPAL_RC_DEBUG_COUNTER_ARRAYS, "arrays" },
};

static void report_rc_store_counters(void) {
    size_t i = 0;
    int counters_balanced = 1;
    for (i = 0; i < (sizeof(REQUIRED_COUNTERS) / sizeof(REQUIRED_COUNTERS[0])); ++i) {
        const CounterLabel* counter = &REQUIRED_COUNTERS[i];
        size_t alloc_count = opal_rc_debug_alloc_count_for_test(counter->kind);
        size_t free_count = opal_rc_debug_free_count_for_test(counter->kind);
        size_t live_count = opal_rc_debug_live_count_for_test(counter->kind);
        if (alloc_count == 0 || free_count != alloc_count || live_count != 0) {
            counters_balanced = 0;
        }
        printf(
            "rc_store_counter:%s alloc=%zu free=%zu live=%zu\n",
            counter->label,
            alloc_count,
            free_count,
            live_count
        );
    }

    {
        size_t live_heap = opal_runtime_live_heap_bytes();
        size_t peak_heap = opal_runtime_peak_heap_bytes();
        int heap_balanced = (live_heap == 0 && peak_heap > 0) ? 1 : 0;

        printf("rc_store_live_heap_bytes=%zu\n", live_heap);
        printf("rc_store_peak_heap_bytes=%zu\n", peak_heap);
        printf("rc_store_counter_status=%s\n", counters_balanced ? "balanced" : "imbalanced");
        printf("rc_store_heap_status=%s\n", heap_balanced ? "balanced" : "imbalanced");
    }
}

__attribute__((constructor))
static void init_rc_store_counter_harness(void) {
    opal_runtime_reset_heap_accounting();
    opal_rc_debug_reset_counters_for_test();
    if (atexit(report_rc_store_counters) != 0) {
        fprintf(stderr, "failed to register rc store leak reporter\n");
    }
}
