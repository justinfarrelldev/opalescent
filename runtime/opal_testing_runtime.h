#ifndef OPAL_TESTING_RUNTIME_H
#define OPAL_TESTING_RUNTIME_H

/* Test-artifact-only ABI. This header is not part of the production runtime. */
#include <stddef.h>
#include <stdint.h>

#define OPAL_TEST_RUNTIME_ABI_VERSION 1

typedef enum {
    OPAL_TEST_PHASE_EXAMPLE = 1,
    OPAL_TEST_PHASE_WORLD_SETUP = 2,
    OPAL_TEST_PHASE_BEFORE_HOOK = 3,
    OPAL_TEST_PHASE_BODY = 4,
    OPAL_TEST_PHASE_AFTER_HOOK = 5,
    OPAL_TEST_PHASE_WORLD_CLEANUP = 6,
    OPAL_TEST_PHASE_STARTUP = 7
} OpalTestPhase;

typedef enum {
    OPAL_TEST_FAILURE_ASSERTION = 1,
    OPAL_TEST_FAILURE_UNEXPECTED_CALL = 2,
    OPAL_TEST_FAILURE_UNUSED_RULE = 3,
    OPAL_TEST_FAILURE_APPLICATION_ERROR = 4,
    OPAL_TEST_FAILURE_INFRASTRUCTURE = 5
} OpalTestFailureKind;

/* Immutable diagnostic view, valid until attempt_end; no external resources. */
typedef struct {
    uint64_t id;
    uint64_t primary_failure_id;
    uint64_t parent_failure_id;
    OpalTestFailureKind kind;
    OpalTestPhase phase;
    const char *message;
    const char *file;
    uint64_t offset;
    uint64_t length;
} OpalTestFailure;

/* Exactly one attempt may start in a worker, even after its memory is released. */
int opal_test_attempt_begin(const char *case_id, uint64_t attempt);
void opal_test_attempt_end(void);
/* Optional synchronous progress observer for coordinator result transport only.
 * This is not acquired-resource registration or a user cleanup mechanism. */
typedef void (*OpalTestPhaseObserver)(OpalTestPhase phase, void *context);
void opal_test_set_phase_observer(OpalTestPhaseObserver observer, void *context);
void opal_test_set_phase(OpalTestPhase phase);
const char *opal_test_case_id(void);
uint64_t opal_test_attempt_number(void);
int opal_test_attempt_failed(void);
uint64_t opal_test_primary_failure_id(void);
size_t opal_test_failure_count(void);
const OpalTestFailure *opal_test_failure_at(size_t index);

/* Returns a nonzero internal identity, never an application error or value. */
uint64_t opal_test_record_failure(OpalTestFailureKind kind, const char *message,
    const char *file, uint64_t offset, uint64_t length);
void opal_test_record_secondary_application_error(uint64_t originating_failure,
    const char *error);

#endif
