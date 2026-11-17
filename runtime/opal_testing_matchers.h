#ifndef OPAL_TESTING_MATCHERS_H
#define OPAL_TESTING_MATCHERS_H
#include "opal_testing_runtime.h"
/* Fixed string matcher vocabulary: equality, containment, prefix, suffix, empty. */
int opal_test_string_matches(int relation, const char *actual, const char *expected);
uint64_t opal_test_record_string_mismatch(const char *matcher, const char *actual,
    const char *expected, const char *file, uint64_t offset, uint64_t length);
#endif
