/* Attempt-local precompiled typed dispatch. No AST interpreter, symbol patching,
 * borrowed-address history, resource ledger, rollback, or user cleanup hooks. */
#include "opal_testing_doubles.h"
#include <stdlib.h>
#include <string.h>

typedef struct DoubleRule {
    int kind;
    void *payload;
    OpalTestDoubleMatch match;
    OpalTestDoubleDrop release;
    const char *file;
    uint64_t offset, length, hits;
    struct DoubleRule *next;
} DoubleRule;

typedef struct DoubleCall {
    void *snapshot;
    struct DoubleCall *next;
} DoubleCall;

typedef struct DoubleHandle {
    const OpalTestDoubleDescriptor *descriptor;
    int active, fallback_kind;
    void *fallback;
    OpalTestDoubleDrop drop_fallback;
    uint64_t calls;
    DoubleRule *rules;
    DoubleCall *history;
    struct DoubleHandle *next;
} DoubleHandle;

static DoubleHandle *handles;

static uint64_t infrastructure(const char *message) {
    return opal_test_record_failure(OPAL_TEST_FAILURE_INFRASTRUCTURE,
        message, "", 0, 0);
}

/* Validate opaque source handles by membership before dereferencing them. */
static DoubleHandle *find_handle(const void *value) {
    for (DoubleHandle *handle = handles; handle; handle = handle->next)
        if (handle == value) return handle;
    return NULL;
}

static DoubleHandle *active_edge(const char *edge) {
    if (!edge) return NULL;
    for (DoubleHandle *handle = handles; handle; handle = handle->next)
        if (handle->active && !strcmp(handle->descriptor->edge, edge)) return handle;
    return NULL;
}

static int valid_kind(int kind) {
    return kind >= OPAL_TEST_DOUBLE_ORIGINAL && kind <= OPAL_TEST_DOUBLE_REPLACEMENT;
}

uint64_t opal_test_double_activate(const OpalTestDoubleDescriptor *descriptor,
    void **output) {
    if (!output || !descriptor || !descriptor->edge || !descriptor->identity ||
        !descriptor->edge[0] || !descriptor->identity[0] ||
        (!!descriptor->snapshot != !!descriptor->drop_snapshot))
        return infrastructure("double activation requires a valid immutable typed descriptor");
    if (active_edge(descriptor->edge))
        return infrastructure("conflicting active doubles target the same canonical import edge");
    DoubleHandle *handle = calloc(1, sizeof(*handle));
    if (!handle) return infrastructure("could not allocate double activation state");
    handle->descriptor = descriptor;
    handle->active = 1;
    handle->fallback_kind = OPAL_TEST_DOUBLE_FAIL;
    handle->next = handles;
    handles = handle;
    *output = handle;
    return 0;
}

uint64_t opal_test_double_restore(void *value) {
    DoubleHandle *handle = find_handle(value);
    if (!handle) return infrastructure("restore requires an activated nominal double handle");
    handle->active = 0;
    return 0;
}

uint64_t opal_test_double_add_exact(void *value, int kind, void *payload,
    OpalTestDoubleMatch match, OpalTestDoubleMatch overlap,
    OpalTestDoubleDrop release, const char *file, uint64_t offset, uint64_t length) {
    DoubleHandle *handle = find_handle(value);
    const char *error = NULL;
    if (!handle || !handle->active || !payload || !match || !overlap || !release ||
        (kind != OPAL_TEST_DOUBLE_RETURN && kind != OPAL_TEST_DOUBLE_APPLICATION_ERROR))
        error = "exact rules require an active double and compiler-typed owned configuration";
    if (!error) {
        for (DoubleRule *rule = handle->rules; rule; rule = rule->next)
            if (overlap(rule->payload, payload)) {
                error = "duplicate or overlapping exact double rules are not permitted";
                break;
            }
    }
    if (error) {
        if (release && payload) release(payload);
        return opal_test_record_failure(OPAL_TEST_FAILURE_INFRASTRUCTURE,
            error, file ? file : "", offset, length);
    }
    DoubleRule *rule = calloc(1, sizeof(*rule));
    if (!rule) {
        release(payload);
        return infrastructure("could not allocate typed exact double rule");
    }
    rule->kind = kind;
    rule->payload = payload;
    rule->match = match;
    rule->release = release;
    rule->file = file ? file : "";
    rule->offset = offset;
    rule->length = length;
    rule->next = handle->rules;
    handle->rules = rule;
    return 0;
}

uint64_t opal_test_double_fallback(void *value, int kind, void *payload,
    OpalTestDoubleDrop release) {
    DoubleHandle *handle = find_handle(value);
    if (!handle || !handle->active || !valid_kind(kind) ||
        (payload && !release) ||
        ((kind == OPAL_TEST_DOUBLE_RETURN || kind == OPAL_TEST_DOUBLE_APPLICATION_ERROR ||
          kind == OPAL_TEST_DOUBLE_REPLACEMENT) && !payload)) {
        if (release && payload) release(payload);
        return infrastructure("fallback requires an active double and valid typed configuration");
    }
    if (handle->drop_fallback && handle->fallback)
        handle->drop_fallback(handle->fallback);
    handle->fallback_kind = kind;
    handle->fallback = payload;
    handle->drop_fallback = release;
    return 0;
}

/* Arrival counts include unexpected calls. Histories contain copied values,
 * never arbitrary borrowed argument-pack addresses from a completed call. */
static uint64_t record_call(DoubleHandle *handle, const void *arguments) {
    if (handle->calls == UINT64_MAX)
        return infrastructure("double call count exceeds the supported range");
    ++handle->calls;
    if (!handle->descriptor->snapshot) return 0;
    void *snapshot = handle->descriptor->snapshot(arguments);
    if (!snapshot) return infrastructure("could not capture typed double argument history");
    DoubleCall *call = malloc(sizeof(*call));
    if (!call) {
        handle->descriptor->drop_snapshot(snapshot);
        return infrastructure("could not allocate double call history");
    }
    call->snapshot = snapshot;
    call->next = handle->history;
    handle->history = call;
    return 0;
}

/* Bound diagnostic text at a UTF-8 boundary; descriptors are immutable source data. */
static uint64_t unexpected_call(const DoubleHandle *handle) {
    const char prefix[] = "unexpected invocation of active double ";
    char message[512];
    size_t start = sizeof(prefix) - 1;
    memcpy(message, prefix, start);
    size_t room = sizeof(message) - start - 4;
    size_t count = 0;
    const char *name = handle->descriptor->identity;
    while (count < room && name[count]) ++count;
    int truncated = name[count] != 0;
    if (truncated)
        while (count && (((unsigned char)name[count] & 0xc0) == 0x80)) --count;
    memcpy(message + start, name, count);
    start += count;
    if (truncated) { memcpy(message + start, "...", 3); start += 3; }
    message[start] = 0;
    return opal_test_record_failure(OPAL_TEST_FAILURE_UNEXPECTED_CALL,
        message, "", 0, 0);
}

uint64_t opal_test_double_select(const char *edge, const void *arguments,
    OpalTestDoubleSelection *selection) {
    if (!edge || !selection) return infrastructure("double dispatch requires a canonical edge and output");
    DoubleHandle *handle = active_edge(edge);
    if (!handle) {
        selection->kind = OPAL_TEST_DOUBLE_ORIGINAL;
        selection->payload = NULL;
        return 0;
    }
    uint64_t failure = record_call(handle, arguments);
    if (failure) return failure;
    for (DoubleRule *rule = handle->rules; rule; rule = rule->next) {
        if (rule->match(rule->payload, arguments)) {
            if (rule->hits != UINT64_MAX) ++rule->hits;
            selection->kind = rule->kind;
            selection->payload = rule->payload;
            return 0;
        }
    }
    if (handle->fallback_kind == OPAL_TEST_DOUBLE_FAIL)
        return unexpected_call(handle);
    selection->kind = handle->fallback_kind;
    selection->payload = handle->fallback;
    return 0;
}

uint64_t opal_test_double_verify(void *value) {
    DoubleHandle *handle = find_handle(value);
    if (!handle) return infrastructure("verification requires an activated nominal double handle");
    uint64_t first = 0;
    for (DoubleRule *rule = handle->rules; rule; rule = rule->next) {
        if (rule->hits) continue;
        uint64_t identity = opal_test_record_failure(OPAL_TEST_FAILURE_UNUSED_RULE,
            "configured exact double rule was never exercised", rule->file, rule->offset, rule->length);
        if (!first) first = identity;
    }
    return first;
}

uint64_t opal_test_double_count(void *value, uint64_t *count) {
    DoubleHandle *handle = find_handle(value);
    if (!handle || !count) return infrastructure("call-count inspection requires a valid double handle");
    *count = handle->calls;
    return 0;
}

uint64_t opal_test_double_history_matches(void *value, const void *expected,
    OpalTestDoubleMatch match, int *found) {
    DoubleHandle *handle = find_handle(value);
    if (!handle || !found || !match || !handle->descriptor->snapshot)
        return infrastructure("argument-history inspection requires supported typed snapshots");
    for (DoubleCall *call = handle->history; call; call = call->next)
        if (match(expected, call->snapshot)) { *found = 1; return 0; }
    *found = 0;
    return 0;
}

void opal_test_doubles_end(void) {
    while (handles) {
        DoubleHandle *handle = handles;
        handles = handle->next;
        while (handle->rules) {
            DoubleRule *rule = handle->rules;
            handle->rules = rule->next;
            rule->release(rule->payload);
            free(rule);
        }
        while (handle->history) {
            DoubleCall *call = handle->history;
            handle->history = call->next;
            handle->descriptor->drop_snapshot(call->snapshot);
            free(call);
        }
        if (handle->drop_fallback && handle->fallback)
            handle->drop_fallback(handle->fallback);
        free(handle);
    }
}
