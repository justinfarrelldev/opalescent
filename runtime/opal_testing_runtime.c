/* Failure state for one fresh-process native test attempt. No resource ledger,
 * external-resource discovery, automatic rollback, or user cleanup lives here. */
#include "opal_testing_runtime.h"
#include <stdlib.h>
#include <string.h>

typedef struct {
    int active;
    char *case_id;
    uint64_t attempt;
    OpalTestPhase phase;
    uint64_t next_failure;
    uint64_t primary;
    OpalTestFailure **failures;
    size_t count;
    size_t capacity;
    int has_emergency;
    OpalTestFailure emergency;
} OpalTestAttemptState;

static OpalTestAttemptState state;
static int worker_has_started;

static char *copy_text(const char *text) {
    const char *value = text ? text : "";
    size_t length = strlen(value);
    if (length == SIZE_MAX) return NULL;
    char *copy = (char *)malloc(length + 1);
    if (copy) memcpy(copy, value, length + 1);
    return copy;
}

/* Allocation failure is an explicit infrastructure failure, not a fabricated
 * application result. A static diagnostic keeps the abort channel available. */
static uint64_t infrastructure_failure(const char *message) {
    if (!state.has_emergency) {
        state.has_emergency = 1;
        if (!state.primary) state.primary = UINT64_MAX;
        state.emergency.id = UINT64_MAX;
        state.emergency.primary_failure_id = state.primary;
        state.emergency.parent_failure_id = 0;
        state.emergency.kind = OPAL_TEST_FAILURE_INFRASTRUCTURE;
        state.emergency.phase = state.phase ? state.phase : OPAL_TEST_PHASE_STARTUP;
        state.emergency.message = message;
        state.emergency.file = "";
        state.emergency.offset = 0;
        state.emergency.length = 0;
    }
    return state.emergency.id;
}

int opal_test_attempt_begin(const char *case_id, uint64_t attempt) {
    if (worker_has_started || !case_id || !attempt) return 0;
    worker_has_started = 1;
    state.case_id = copy_text(case_id);
    if (!state.case_id) {
        infrastructure_failure("native testing runtime could not allocate case identity");
        return 0;
    }
    state.active = 1;
    state.attempt = attempt;
    state.phase = OPAL_TEST_PHASE_STARTUP;
    state.next_failure = 1;
    return 1;
}

void opal_test_attempt_end(void) {
    for (size_t index = 0; index < state.count; ++index) {
        free((void *)state.failures[index]->message);
        free((void *)state.failures[index]->file);
        free(state.failures[index]);
    }
    free(state.failures);
    free(state.case_id);
    memset(&state, 0, sizeof(state));
    /* worker_has_started deliberately remains set: no implicit worker reuse. */
}

void opal_test_set_phase(OpalTestPhase phase) {
    if (phase < OPAL_TEST_PHASE_EXAMPLE || phase > OPAL_TEST_PHASE_STARTUP) {
        infrastructure_failure("native testing runtime received an invalid lifecycle phase");
        return;
    }
    state.phase = phase;
}

const char *opal_test_case_id(void) { return state.case_id ? state.case_id : ""; }
uint64_t opal_test_attempt_number(void) { return state.attempt; }
int opal_test_attempt_failed(void) { return state.count != 0 || state.has_emergency; }
uint64_t opal_test_primary_failure_id(void) { return state.primary; }
size_t opal_test_failure_count(void) { return state.count + (size_t)state.has_emergency; }
const OpalTestFailure *opal_test_failure_at(size_t index) {
    if (index < state.count) return state.failures[index];
    if (index == state.count && state.has_emergency) return &state.emergency;
    return NULL;
}

static int reserve_failure(void) {
    if (state.count < state.capacity) return 1;
    size_t capacity = state.capacity ? state.capacity * 2 : 8;
    if (capacity < state.capacity || capacity > SIZE_MAX / sizeof(OpalTestFailure *)) return 0;
    OpalTestFailure **failures = (OpalTestFailure **)realloc(state.failures,
        capacity * sizeof(OpalTestFailure *));
    if (!failures) return 0;
    state.failures = failures;
    state.capacity = capacity;
    return 1;
}

static uint64_t append_failure(OpalTestFailureKind kind, const char *message,
    const char *file, uint64_t offset, uint64_t length, uint64_t parent) {
    if (!state.active) return infrastructure_failure("test failure outside an active attempt");
    if (kind < OPAL_TEST_FAILURE_ASSERTION || kind > OPAL_TEST_FAILURE_INFRASTRUCTURE)
        return infrastructure_failure("native testing runtime received an invalid failure kind");
    if (state.next_failure == UINT64_MAX)
        return infrastructure_failure("native testing runtime exhausted failure identities");
    OpalTestFailure *failure = (OpalTestFailure *)malloc(sizeof(OpalTestFailure));
    char *owned_message = copy_text(message);
    char *owned_file = copy_text(file);
    if (!failure || !owned_message || !owned_file || !reserve_failure()) {
        free(failure);
        free(owned_message);
        free(owned_file);
        return infrastructure_failure("native testing runtime could not allocate a failure diagnostic");
    }
    uint64_t identity = state.next_failure++;
    if (!state.primary) state.primary = identity;
    state.failures[state.count++] = failure;
    failure->id = identity;
    failure->primary_failure_id = state.primary;
    failure->parent_failure_id = parent;
    failure->kind = kind;
    failure->phase = state.phase;
    failure->message = owned_message;
    failure->file = owned_file;
    failure->offset = offset;
    failure->length = length;
    return identity;
}

uint64_t opal_test_record_failure(OpalTestFailureKind kind, const char *message,
    const char *file, uint64_t offset, uint64_t length) {
    return append_failure(kind, message, file, offset, length, 0);
}

void opal_test_record_secondary_application_error(uint64_t originating_failure,
    const char *error) {
    int found = state.has_emergency && state.emergency.id == originating_failure;
    for (size_t index = 0; !found && index < state.count; ++index)
        found = state.failures[index]->id == originating_failure;
    if (!found) {
        infrastructure_failure("lexical cleanup referenced an unknown test failure identity");
        return;
    }
    append_failure(OPAL_TEST_FAILURE_APPLICATION_ERROR, error, "", 0, 0,
        originating_failure);
}
