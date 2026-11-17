/* One reusable native artifact, one fresh process per selected case attempt. */
#include "opal_testing_worker.h"
#include "opal_testing_doubles.h"
#include <errno.h>
#include <inttypes.h>
#include <limits.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

typedef struct {
    FILE *stream;
    int failed;
    uint64_t callback_index;
} OpalTestTransport;

static void json_text(FILE *stream, const char *text) {
    const unsigned char *cursor = (const unsigned char *)(text ? text : "");
    fputc('"', stream);
    for (; *cursor; ++cursor) {
        if (*cursor == '"' || *cursor == '\\') {
            fputc('\\', stream); fputc(*cursor, stream);
        } else if (*cursor < 32) {
            fprintf(stream, "\\u%04x", (unsigned int)*cursor);
        } else {
            fputc(*cursor, stream);
        }
    }
    fputc('"', stream);
}

static void phase_event(OpalTestPhase phase, void *context) {
    OpalTestTransport *transport = (OpalTestTransport *)context;
    if (fprintf(transport->stream, "{\"event\":\"phase\",\"phase\":%d}\n", (int)phase) < 0 ||
        fflush(transport->stream)) transport->failed = 1;
}

static void callback_event(int finished, const OpalTestCallback *callback,
    OpalTestPhase phase, const char *world, int status, int passed, uint64_t duration_ms, void *context) {
    OpalTestTransport *transport = context;
    if (!finished) ++transport->callback_index;
    FILE *stream = transport->stream;
    fprintf(stream, "{\"event\":\"callback\",\"stage\":\"%s\",\"index\":%" PRIu64 ",\"phase\":%d,\"world\":",
        finished ? "finish" : "start", transport->callback_index, (int)phase);
    json_text(stream, world);
    fputs(",\"file\":", stream); json_text(stream, callback->file);
    fprintf(stream, ",\"offset\":%" PRIu64 ",\"length\":%" PRIu64 ",\"status\":%d,\"passed\":%s,\"duration_ms\":%" PRIu64 "}\n",
        callback->offset, callback->length, status, passed ? "true" : "false", duration_ms);
    if (ferror(stream) || fflush(stream)) transport->failed = 1;
}

static void failure_record(FILE *stream, const OpalTestFailure *failure) {
    fprintf(stream, "{\"id\":%" PRIu64 ",\"primary_failure_id\":%" PRIu64
        ",\"parent_failure_id\":%" PRIu64 ",\"kind\":%d,\"phase\":%d,\"message\":",
        failure->id, failure->primary_failure_id, failure->parent_failure_id,
        (int)failure->kind, (int)failure->phase);
    json_text(stream, failure->message);
    fputs(",\"file\":", stream); json_text(stream, failure->file);
    fprintf(stream, ",\"offset\":%" PRIu64 ",\"length\":%" PRIu64 "}",
        failure->offset, failure->length);
}

static int finish_result(OpalTestTransport *transport, int failed,
    const OpalTestLifecycleResult *result) {
    FILE *stream = transport->stream;
    fputs("{\"event\":\"result\",\"case_id\":", stream);
    json_text(stream, opal_test_case_id());
    fprintf(stream, ",\"attempt\":%" PRIu64 ",\"status\":\"%s\",\"primary_failure_id\":%" PRIu64
        ",\"body_started\":%s,\"body_passed\":%s,\"initialized_worlds\":%zu,\"entered_scopes\":%zu,\"failures\":[",
        opal_test_attempt_number(), failed ? "failed" : "passed", opal_test_primary_failure_id(),
        result->body_started ? "true" : "false", result->body_passed ? "true" : "false",
        result->initialized_worlds, result->entered_scopes);
    for (size_t index = 0; index < opal_test_failure_count(); ++index) {
        if (index) fputc(',', stream);
        failure_record(stream, opal_test_failure_at(index));
    }
    fputs("]}\n", stream);
    if (ferror(stream) || fflush(stream)) transport->failed = 1;
    return transport->failed ? 2 : (failed ? 1 : 0);
}

static int infrastructure_failure(FILE *stream, const char *message) {
    fputs("{\"event\":\"infrastructure\",\"message\":", stream);
    json_text(stream, message); fputs("}\n", stream);
    (void)fclose(stream);
    return 2;
}

int main(int argc, char **argv) {
    if (argc != 4) return 2;
    /* Exclusive creation prevents accidentally reporting a stale prior attempt. */
    FILE *stream = fopen(argv[3], "wx");
    if (!stream) return 2;
    errno = 0;
    char *end = NULL;
    unsigned long long parsed = strtoull(argv[2], &end, 10);
    if (errno || !argv[2][0] || argv[2][0] == '-' || !end || *end ||
        !parsed || parsed > INT64_MAX) return infrastructure_failure(stream, "invalid native attempt number");
    const OpalTestCaseDescriptor *descriptor = NULL;
    for (size_t index = 0; index < opal_test_compiled_case_count; ++index) {
        if (!strcmp(argv[1], opal_test_compiled_cases[index].identity)) {
            descriptor = opal_test_compiled_cases[index].descriptor; break;
        }
    }
    if (!descriptor) return infrastructure_failure(stream, "unknown native case identity");
    if (!opal_test_attempt_begin(argv[1], (uint64_t)parsed))
        return infrastructure_failure(stream, "native worker could not begin its attempt");
    OpalTestTransport transport = {stream, 0, 0};
    opal_test_set_callback_observer(callback_event, &transport);
    opal_test_set_phase_observer(phase_event, &transport);
    phase_event(OPAL_TEST_PHASE_STARTUP, &transport);
    OpalTestLifecycleResult result;
    int failed = opal_test_run_case(descriptor, &result);
    int status = finish_result(&transport, failed, &result);
    opal_test_set_phase_observer(NULL, NULL);
    opal_test_set_callback_observer(NULL, NULL);
    opal_test_doubles_end();
    opal_test_attempt_end();
    if (fclose(stream)) return 2;
    return status;
}
