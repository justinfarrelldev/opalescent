#include <errno.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <sys/types.h>

#include "runtime/opal_rc.h"
#include "runtime/opal_runtime.h"

typedef struct {
    size_t length;
    uint8_t* data;
} OwnedBytesLayout;

typedef struct {
    int64_t size_bytes;
    int8_t is_directory;
    int8_t is_symlink;
    int64_t modified_unix_seconds;
} OpalFileMetadata;

typedef struct {
    OpalRcDebugCounterKind kind;
    const char* label;
} CounterLabel;

static const CounterLabel REQUIRED_COUNTERS[] = {
    { OPAL_RC_DEBUG_COUNTER_STRINGS, "strings" },
    { OPAL_RC_DEBUG_COUNTER_ARRAYS, "arrays" },
    { OPAL_RC_DEBUG_COUNTER_BYTES, "bytes" },
    { OPAL_RC_DEBUG_COUNTER_BUILDERS, "builders" },
    { OPAL_RC_DEBUG_COUNTER_FILESYSTEM_OBJECTS, "filesystem_objects" },
    { OPAL_RC_DEBUG_COUNTER_METADATA_PERMISSIONS, "metadata_permissions" },
    { OPAL_RC_DEBUG_COUNTER_ERROR_PAYLOADS, "error_payloads" },
    { OPAL_RC_DEBUG_COUNTER_RC_CHILD_ARRAYS, "rc_child_arrays" },
};

static void report_counters(void) {
    size_t i = 0;
    int balanced = 1;
    for (i = 0; i < (sizeof(REQUIRED_COUNTERS) / sizeof(REQUIRED_COUNTERS[0])); ++i) {
        const CounterLabel* counter = &REQUIRED_COUNTERS[i];
        size_t alloc_count = opal_rc_debug_alloc_count_for_test(counter->kind);
        size_t free_count = opal_rc_debug_free_count_for_test(counter->kind);
        size_t live_count = opal_rc_debug_live_count_for_test(counter->kind);
        if (alloc_count == 0 || free_count != alloc_count || live_count != 0) {
            balanced = 0;
        }
        printf(
            "counter:%s alloc=%zu free=%zu live=%zu\n",
            counter->label,
            alloc_count,
            free_count,
            live_count
        );
    }
    printf("counter_status=%s\n", balanced ? "balanced" : "imbalanced");
}

static int make_directory_if_needed(const char* path) {
    if (mkdir(path, 0700) == 0 || errno == EEXIST) {
        return 0;
    }
    fprintf(stderr, "failed to create directory '%s': %s\n", path, strerror(errno));
    return 1;
}

static int write_fixture_file(const char* path, const char* payload) {
    FILE* file = fopen(path, "wb");
    size_t payload_len = strlen(payload);
    if (!file) {
        fprintf(stderr, "failed to open '%s' for writing: %s\n", path, strerror(errno));
        return 1;
    }
    if (fwrite(payload, 1, payload_len, file) != payload_len) {
        fprintf(stderr, "failed to write fixture payload to '%s'\n", path);
        fclose(file);
        return 1;
    }
    if (fclose(file) != 0) {
        fprintf(stderr, "failed to close '%s' after writing: %s\n", path, strerror(errno));
        return 1;
    }
    return 0;
}

static int build_path(char* out, size_t out_size, const char* base, const char* leaf) {
    int written = snprintf(out, out_size, "%s/%s", base, leaf);
    if (written < 0 || (size_t)written >= out_size) {
        fprintf(stderr, "path overflow while joining '%s' and '%s'\n", base, leaf);
        return 1;
    }
    return 0;
}

static void free_owned_string(char* value) {
    if (!value) {
        return;
    }
    opal_rc_debug_note_free(OPAL_RC_DEBUG_COUNTER_STRINGS);
    free(value);
}

static void free_owned_bytes(OpalBytes* bytes) {
    OwnedBytesLayout* owned = (OwnedBytesLayout*)bytes;
    if (!owned) {
        return;
    }
    free(owned->data);
    opal_rc_debug_note_free(OPAL_RC_DEBUG_COUNTER_BYTES);
    free(owned);
}

static int run_positive_scenario(const char* workspace_root) {
    char payload_path[1024];
    void* string_obj = NULL;
    void* array_obj = NULL;
    void* rc_child_array_obj = NULL;
    BytesResult hex_result;
    OpalBytes* hex_bytes = NULL;
    char* hex_text = NULL;
    OpalStringBuilder* builder = NULL;
    StringBuilderVoidResult push_result;
    StringBuilderStringResult finish_result;
    char* finished = NULL;
    char* payload_path_owned = NULL;
    FsBytesResult read_result;
    OpalBytes* file_bytes = NULL;
    char* metadata_path_owned = NULL;
    FsMetadataResult metadata_result;
    OpalFileMetadata* metadata = NULL;
    char* missing_path_owned = NULL;
    FsMetadataResult missing_result;

    if (build_path(payload_path, sizeof(payload_path), workspace_root, "payload.txt") != 0) {
        return 10;
    }

    if (make_directory_if_needed(workspace_root) != 0) {
        return 11;
    }
    if (write_fixture_file(payload_path, "payload-data") != 0) {
        return 12;
    }

    string_obj = opal_rc_alloc_tracked(sizeof(int), NULL, OPAL_RC_DEBUG_COUNTER_STRINGS);
    if (!string_obj) {
        fprintf(stderr, "tracked string allocation returned null\n");
        return 19;
    }
    opal_rc_dec(string_obj);

    array_obj = opal_array_alloc(sizeof(int), _Alignof(int), 2, 2, NULL);
    if (!array_obj) {
        fprintf(stderr, "tracked array allocation returned null\n");
        return 20;
    }
    opal_rc_dec(array_obj);

    rc_child_array_obj = opal_rc_alloc_tracked(sizeof(int), NULL, OPAL_RC_DEBUG_COUNTER_RC_CHILD_ARRAYS);
    if (!rc_child_array_obj) {
        fprintf(stderr, "tracked rc child array allocation returned null\n");
        return 21;
    }
    opal_rc_dec(rc_child_array_obj);

    hex_result = bytes_from_hex("414243");
    if (hex_result.error != NULL || hex_result.value == NULL) {
        fprintf(stderr, "bytes_from_hex should succeed, error=%s\n", hex_result.error ? hex_result.error : "<null>");
        return 22;
    }
    hex_bytes = hex_result.value;
    hex_text = bytes_to_hex(hex_bytes);
    if (!hex_text || strcmp(hex_text, "414243") != 0) {
        fprintf(stderr, "bytes_to_hex should roundtrip 414243, got %s\n", hex_text ? hex_text : "<null>");
        free_owned_bytes(hex_bytes);
        free_owned_string(hex_text);
        return 23;
    }
    free_owned_string(hex_text);
    free_owned_bytes(hex_bytes);

    builder = string_builder_new();
    push_result = string_builder_push(builder, "builder-payload");
    if (push_result.error != NULL) {
        fprintf(stderr, "string_builder_push should succeed, error=%s\n", push_result.error);
        return 24;
    }
    finish_result = string_builder_finish(builder);
    if (finish_result.error != NULL || finish_result.value == NULL) {
        fprintf(stderr, "string_builder_finish should succeed, error=%s\n", finish_result.error ? finish_result.error : "<null>");
        return 25;
    }
    finished = finish_result.value;
    if (strcmp(finished, "builder-payload") != 0) {
        fprintf(stderr, "string_builder_finish returned unexpected value '%s'\n", finished);
        free_owned_string(finished);
        return 26;
    }
    free_owned_string(finished);

    payload_path_owned = path_from(payload_path);
    read_result = read_contents_sync(payload_path_owned);
    free_owned_string(payload_path_owned);
    if (read_result.error != NULL || read_result.value == NULL) {
        fprintf(stderr, "read_contents_sync should succeed, error=%s\n", read_result.error ? read_result.error : "<null>");
        return 27;
    }
    file_bytes = read_result.value;
    if (bytes_length(file_bytes) <= 0) {
        fprintf(stderr, "read_contents_sync should return non-empty bytes\n");
        free_owned_bytes(file_bytes);
        return 28;
    }
    free_owned_bytes(file_bytes);

    metadata_path_owned = path_from(payload_path);
    metadata_result = read_metadata_sync(metadata_path_owned);
    free_owned_string(metadata_path_owned);
    if (metadata_result.error != NULL || metadata_result.value == NULL) {
        fprintf(stderr, "read_metadata_sync should succeed, error=%s\n", metadata_result.error ? metadata_result.error : "<null>");
        return 29;
    }
    metadata = (OpalFileMetadata*)metadata_result.value;
    if (metadata->size_bytes <= 0) {
        fprintf(stderr, "read_metadata_sync should report positive file size, got %lld\n", (long long)metadata->size_bytes);
        opal_rc_debug_note_free(OPAL_RC_DEBUG_COUNTER_METADATA_PERMISSIONS);
        free(metadata);
        return 30;
    }
    opal_rc_debug_note_free(OPAL_RC_DEBUG_COUNTER_METADATA_PERMISSIONS);
    free(metadata);

    missing_path_owned = path_from("/tmp/opalescent-missing-memory-model-counters-fixture");
    missing_result = read_metadata_sync(missing_path_owned);
    free_owned_string(missing_path_owned);
    if (missing_result.error == NULL) {
        fprintf(stderr, "missing metadata probe should produce an error payload\n");
        return 31;
    }
    if (strstr(missing_result.error, "FileNotFoundError") == NULL) {
        fprintf(stderr, "missing metadata probe should mention FileNotFoundError, got %s\n", missing_result.error);
        opal_rc_debug_note_free(OPAL_RC_DEBUG_COUNTER_ERROR_PAYLOADS);
        free((void*)missing_result.error);
        return 32;
    }
    opal_rc_debug_note_free(OPAL_RC_DEBUG_COUNTER_ERROR_PAYLOADS);
    free((void*)missing_result.error);

    return 0;
}

int main(int argc, char** argv) {
    int scenario_status = 0;
    if (argc != 2) {
        fprintf(stderr, "usage: %s <workspace>\n", argv[0]);
        return 64;
    }
    if (atexit(report_counters) != 0) {
        fprintf(stderr, "failed to register counter reporter\n");
        return 65;
    }
    opal_rc_debug_reset_counters_for_test();
    scenario_status = run_positive_scenario(argv[1]);
    if (scenario_status != 0) {
        return scenario_status;
    }
    return 0;
}
