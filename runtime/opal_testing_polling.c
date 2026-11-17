/* Synchronous monotonic polling uses existing portability shims, not async state. */
#include "opal_portability.h"
#include "opal_testing_polling.h"
#include <math.h>
#include <stdint.h>

static uint64_t poll_error(const char *message) {
    return opal_test_record_failure(OPAL_TEST_FAILURE_INFRASTRUCTURE,
        message, "", 0, 0);
}

uint64_t opal_test_poll_begin(double seconds, uint64_t *deadline) {
    int64_t now = 0;
    if (!deadline || !isfinite(seconds) || seconds < 0.0)
        return poll_error("polling requires a finite nonnegative deadline");
    if (opal_monotonic_time_ms(&now) || now < 0)
        return poll_error("polling could not read the monotonic clock");
    double milliseconds = seconds * 1000.0;
    if (!isfinite(milliseconds) || milliseconds >= (double)(INT64_MAX - now))
        return poll_error("polling deadline exceeds the supported monotonic range");
    uint64_t budget = (uint64_t)milliseconds;
    if ((double)budget < milliseconds) ++budget;
    *deadline = (uint64_t)now + budget;
    return 0;
}

uint64_t opal_test_poll_retry(uint64_t deadline, int *retry) {
    int64_t now = 0;
    if (!retry) return poll_error("polling has no retry output");
    *retry = 0;
    if (opal_monotonic_time_ms(&now) || now < 0)
        return poll_error("polling could not read the monotonic clock");
    if ((uint64_t)now >= deadline) return 0;
    uint64_t remaining = deadline - (uint64_t)now;
    int32_t wait = remaining < 5 ? (int32_t)remaining : 5;
    if (opal_sleep_ms(wait)) return poll_error("polling monotonic wait failed");
    if (opal_monotonic_time_ms(&now) || now < 0)
        return poll_error("polling could not read the monotonic clock");
    *retry = (uint64_t)now < deadline;
    return 0;
}
