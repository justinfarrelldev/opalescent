/* Deterministic invocation boundaries for compiled native test descriptors. */
#include "opal_portability.h"
#include "opal_testing_lifecycle.h"
#include <stdlib.h>
#include <string.h>

static int worker_case_has_run;
static OpalTestCallbackObserver callback_observer;
static void *callback_observer_context;
void opal_test_set_callback_observer(OpalTestCallbackObserver observer, void *context) {
    callback_observer = observer; callback_observer_context = context;
}

static int infrastructure_failure(const char *message) {
    opal_test_record_failure(OPAL_TEST_FAILURE_INFRASTRUCTURE, message, "", 0, 0);
    return 0;
}

static int known_failure(uint64_t identity) {
    if (!identity) return 0;
    for (size_t index = 0; index < opal_test_failure_count(); ++index) {
        const OpalTestFailure *failure = opal_test_failure_at(index);
        if (failure && failure->id == identity) return 1;
    }
    return 0;
}

static int invoke_inner(const OpalTestCallback *callback,
    void **worlds, const void *example, void **created) {
    if (!callback->invoke) return infrastructure_failure("missing native callback wrapper");
    uint64_t failure = 0;
    const char *error = NULL;
    size_t previous_failures = opal_test_failure_count();
    int status = callback->invoke(worlds, example, created, &failure, &error);
    if (status == OPAL_TEST_CALLBACK_COMPLETED) {
        if (failure || error) return infrastructure_failure("completed callback returned a failure channel");
        return previous_failures == opal_test_failure_count();
    }
    if (status == OPAL_TEST_CALLBACK_APPLICATION_ERROR) {
        if (!error || failure) return infrastructure_failure("invalid completed application error callback result");
        opal_test_record_failure(OPAL_TEST_FAILURE_APPLICATION_ERROR, error,
            callback->file, callback->offset, callback->length);
        return 0;
    }
    if (status == OPAL_TEST_CALLBACK_ABORTED) {
        if (error || !known_failure(failure))
            return infrastructure_failure("invalid internal test abort callback result");
        return 0;
    }
    return infrastructure_failure("unknown native callback outcome tag");
}

static int invoke(const OpalTestCallback *callback, OpalTestPhase phase,
    void **worlds, const void *example, void **created, const char *world) {
    opal_test_set_phase(phase);
    int64_t start = 0, end = 0;
    if (opal_monotonic_time_ms(&start)) return infrastructure_failure("callback clock is unavailable");
    if (callback_observer) callback_observer(0, callback, phase, world, -1, 0, 0, callback_observer_context);
    int passed = invoke_inner(callback, worlds, example, created);
    if (opal_monotonic_time_ms(&end) || end < start) {
        infrastructure_failure("callback clock failed"); passed = 0; end = start;
    }
    int status = 0;
    if (!passed) {
        const OpalTestFailure *failure = opal_test_failure_count() ? opal_test_failure_at(opal_test_failure_count()-1) : NULL;
        status = failure && failure->kind == OPAL_TEST_FAILURE_APPLICATION_ERROR ? 1 :
            (failure && failure->kind != OPAL_TEST_FAILURE_INFRASTRUCTURE ? 2 : 3);
    }
    if (callback_observer) callback_observer(1, callback, phase, world, status, passed,
        (uint64_t)(end-start), callback_observer_context);
    return passed;
}

static int validate_descriptor(const OpalTestCaseDescriptor *descriptor) {
    if (!descriptor || !descriptor->body.invoke)
        return infrastructure_failure("native case has no compiled body wrapper");
    if (descriptor->world_count && !descriptor->worlds)
        return infrastructure_failure("native case has no World descriptors");
    if (descriptor->scope_count && !descriptor->scopes)
        return infrastructure_failure("native case has no hook scopes");
    for (size_t index = 0; index < descriptor->world_count; ++index)
        if (!descriptor->worlds[index].setup.invoke)
            return infrastructure_failure("native World has no setup wrapper");
    for (size_t index = 0; index < descriptor->scope_count; ++index) {
        const OpalTestHookScope *scope = &descriptor->scopes[index];
        if ((scope->before_count && !scope->before) || (scope->after_count && !scope->after))
            return infrastructure_failure("native hook scope has no callback array");
    }
    return 1;
}

static int initialize_worlds(const OpalTestCaseDescriptor *descriptor,
    OpalTestLifecycleResult *result, void **worlds, const void *example) {
    if (descriptor->world_count && !worlds) return infrastructure_failure("World initialization has no state slots");
    for (size_t index = 0; index < descriptor->world_count; ++index) {
        void *created = NULL;
        if (!invoke(&descriptor->worlds[index].setup, OPAL_TEST_PHASE_WORLD_SETUP,
                worlds, example, &created, descriptor->worlds[index].identity)) return 0;
        if (!created) return infrastructure_failure("World setup completed without its typed state slot");
        worlds[index] = created;
        ++result->initialized_worlds;
    }
    return 1;
}

static int enter_scopes(const OpalTestCaseDescriptor *descriptor,
    OpalTestLifecycleResult *result, void **worlds, const void *example) {
    for (size_t index = 0; index < descriptor->scope_count; ++index) {
        const OpalTestHookScope *scope = &descriptor->scopes[index];
        ++result->entered_scopes;
        for (size_t before = 0; before < scope->before_count; ++before) {
            void *unused = NULL;
            if (!invoke(&scope->before[before], OPAL_TEST_PHASE_BEFORE_HOOK,
                    worlds, example, &unused, "")) return 0;
        }
    }
    return 1;
}

static void exit_scopes(const OpalTestCaseDescriptor *descriptor,
    const OpalTestLifecycleResult *result, void **worlds, const void *example) {
    if (result->entered_scopes > descriptor->scope_count) {
        infrastructure_failure("native entered-scope count exceeds its descriptor");
        return;
    }
    for (size_t index = result->entered_scopes; index > 0; --index) {
        const OpalTestHookScope *scope = &descriptor->scopes[index - 1];
        for (size_t after = scope->after_count; after > 0; --after) {
            void *unused = NULL;
            invoke(&scope->after[after - 1], OPAL_TEST_PHASE_AFTER_HOOK,
                worlds, example, &unused, "");
        }
    }
}

static void cleanup_worlds(const OpalTestCaseDescriptor *descriptor,
    const OpalTestLifecycleResult *result, void **worlds, const void *example) {
    if (result->initialized_worlds > descriptor->world_count ||
            (result->initialized_worlds && !worlds)) {
        infrastructure_failure("native initialized-World count exceeds its state slots");
        return;
    }
    for (size_t index = result->initialized_worlds; index > 0; --index) {
        const OpalTestWorldDescriptor *world = &descriptor->worlds[index - 1];
        if (world->cleanup.invoke) {
            void *unused = NULL;
            invoke(&world->cleanup, OPAL_TEST_PHASE_WORLD_CLEANUP, worlds, example, &unused, world->identity);
        }
        /* Ordinary typed value memory only. No synthesized resource cleanup. */
        if (world->drop_value) world->drop_value(worlds[index - 1]);
    }
}

int opal_test_run_case(const OpalTestCaseDescriptor *descriptor,
    OpalTestLifecycleResult *output) {
    OpalTestLifecycleResult local_result = {0};
    OpalTestLifecycleResult *result = output ? output : &local_result;
    memset(result, 0, sizeof(*result));
    if (worker_case_has_run || !opal_test_attempt_number()) {
        infrastructure_failure("native case requires one fresh, active worker attempt");
        return 1;
    }
    worker_case_has_run = 1;
    if (!validate_descriptor(descriptor) || opal_test_attempt_failed()) return 1;
    if (descriptor->world_count > SIZE_MAX / sizeof(void *)) {
        infrastructure_failure("native World slot array exceeds addressable memory");
        return 1;
    }
    void **worlds = descriptor->world_count ?
        (void **)calloc(descriptor->world_count, sizeof(void *)) : NULL;
    if (descriptor->world_count && !worlds) {
        infrastructure_failure("native case could not allocate World slots");
        return 1;
    }
    void *example = NULL;
    if (descriptor->construct_example.invoke) {
        if (!invoke(&descriptor->construct_example, OPAL_TEST_PHASE_EXAMPLE,
                worlds, NULL, &example, "")) {
            free(worlds);
            return 1;
        }
        if (!example) {
            infrastructure_failure("example construction completed without typed data");
            free(worlds);
            return 1;
        }
    }
    if (initialize_worlds(descriptor, result, worlds, example) &&
            enter_scopes(descriptor, result, worlds, example)) {
        void *unused = NULL;
        result->body_started = 1;
        result->body_passed = invoke(&descriptor->body, OPAL_TEST_PHASE_BODY,
            worlds, example, &unused, "");
    }
    exit_scopes(descriptor, result, worlds, example);
    cleanup_worlds(descriptor, result, worlds, example);
    if (example && descriptor->drop_example) descriptor->drop_example(example);
    free(worlds);
    return opal_test_attempt_failed();
}
