#ifndef OPAL_TESTING_WORKER_H
#define OPAL_TESTING_WORKER_H
#include "opal_testing_lifecycle.h"
/* Compiler-emitted registry is immutable; each process selects one attempt. */
typedef struct {
    const char *identity;
    const OpalTestCaseDescriptor *descriptor;
} OpalTestCompiledCase;
extern const OpalTestCompiledCase opal_test_compiled_cases[];
extern const size_t opal_test_compiled_case_count;
#endif
