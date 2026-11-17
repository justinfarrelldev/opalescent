/* Test-only typed matcher helpers. Strings retain ordinary UTF-8 C ABI semantics. */
#include "opal_testing_matchers.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

int opal_test_string_matches(int relation, const char *actual, const char *expected) {
    const char *value = actual ? actual : "";
    const char *other = expected ? expected : "";
    switch (relation) {
        case 0: return strcmp(value, other) == 0;
        case 1: return strstr(value, other) != NULL;
        case 2: return strncmp(value, other, strlen(other)) == 0;
        case 3: {
            size_t size = strlen(value), suffix = strlen(other);
            return size >= suffix && memcmp(value + size - suffix, other, suffix) == 0;
        }
        case 4: return !*value;
        default:
            opal_test_record_failure(OPAL_TEST_FAILURE_INFRASTRUCTURE,
                "invalid native string matcher relation", "", 0, 0);
            return 0;
    }
}

/* Bound diagnostic display without splitting a valid UTF-8 scalar sequence. */
static int display_length(const char *text) {
    size_t length = 0;
    while (length < 1024 && text[length]) ++length;
    while (length && ((unsigned char)text[length] & 0xc0) == 0x80) --length;
    return (int)length;
}

uint64_t opal_test_record_string_mismatch(const char *matcher, const char *actual,
    const char *expected, const char *file, uint64_t offset, uint64_t length) {
    const char *value = actual ? actual : "";
    const char *other = expected ? expected : "(empty)";
    const char *operation = matcher ? matcher : "string comparison";
    int actual_length = display_length(value), expected_length = display_length(other);
    const char *actual_suffix = value[actual_length] ? " [truncated]" : "";
    const char *expected_suffix = other[expected_length] ? " [truncated]" : "";
    const char *format = "expect string %s: actual='%.*s%s', expected string='%.*s%s'";
    int required = snprintf(NULL, 0, format, operation, actual_length, value,
        actual_suffix, expected_length, other, expected_suffix);
    if (required < 0) return opal_test_record_failure(OPAL_TEST_FAILURE_INFRASTRUCTURE,
        "native string diagnostic formatting failed", file, offset, length);
    char *message = (char *)malloc((size_t)required + 1);
    if (!message) return opal_test_record_failure(OPAL_TEST_FAILURE_INFRASTRUCTURE,
        "native string diagnostic allocation failed", file, offset, length);
    (void)snprintf(message, (size_t)required + 1, format, operation, actual_length,
        value, actual_suffix, expected_length, other, expected_suffix);
    uint64_t identity = opal_test_record_failure(OPAL_TEST_FAILURE_ASSERTION,
        message, file, offset, length);
    free(message);
    return identity;
}
