#ifndef OPAL_TESTING_LIFECYCLE_H
#define OPAL_TESTING_LIFECYCLE_H
#include "opal_testing_runtime.h"

/* Simple C boundary for compiler-generated typed wrappers. Aborted callbacks
 * never populate an application value or an application error. World slots
 * point to a typed single-state-field value; nominal World references borrow a
 * handle to that slot, while cleanup references borrow its state field directly.
 * The example pointer addresses its constructed typed value, not a name map. */
enum {
    OPAL_TEST_CALLBACK_COMPLETED = 0,
    OPAL_TEST_CALLBACK_APPLICATION_ERROR = 1,
    OPAL_TEST_CALLBACK_ABORTED = 2
};
typedef int (*OpalTestInvoke)(void **worlds, const void *example,
    void **created_value, uint64_t *failure, const char **application_error);
typedef struct {
    OpalTestInvoke invoke;
    const char *file;
    uint64_t offset;
    uint64_t length;
} OpalTestCallback;

/* These are only lifecycle World slots, not acquired-resource tracking. The
 * compiler's drop callback releases ordinary value memory, not user resources. */
typedef struct {
    const char *identity;
    OpalTestCallback setup;
    OpalTestCallback cleanup;
    void (*drop_value)(void *value);
} OpalTestWorldDescriptor;
typedef struct {
    const OpalTestCallback *before;
    size_t before_count;
    const OpalTestCallback *after;
    size_t after_count;
} OpalTestHookScope;
typedef struct {
    const OpalTestWorldDescriptor *worlds;
    size_t world_count;
    const OpalTestHookScope *scopes;
    size_t scope_count;
    OpalTestCallback body;
    OpalTestCallback construct_example;
    void (*drop_example)(void *example);
} OpalTestCaseDescriptor;
typedef struct {
    size_t initialized_worlds;
    size_t entered_scopes;
    int body_started;
    int body_passed;
} OpalTestLifecycleResult;

/* Framework-only synchronous observation of real callback boundaries. */
typedef void (*OpalTestCallbackObserver)(int finished, const OpalTestCallback *callback,
    OpalTestPhase phase, const char *world, int status, int passed,
    uint64_t duration_ms, void *context);
void opal_test_set_callback_observer(OpalTestCallbackObserver observer, void *context);

/* Run one descriptor in the already-started fresh-process attempt. */
int opal_test_run_case(const OpalTestCaseDescriptor *descriptor,
    OpalTestLifecycleResult *result);
#endif
