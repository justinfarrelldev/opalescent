#ifndef OPAL_TESTING_DOUBLES_H
#define OPAL_TESTING_DOUBLES_H
#include "opal_testing_runtime.h"

/* Compiler-generated, C-convention helpers only. Clone never retains borrowed
 * addresses; release performs ordinary value-memory release, not user cleanup. */
typedef void *(*OpalTestDoubleSnapshot)(const void *arguments);
typedef void (*OpalTestDoubleDrop)(void *value);
typedef int (*OpalTestDoubleMatch)(const void *expected, const void *actual);
/* Pure compiler-owned value rendering, never application callbacks. */
typedef void (*OpalTestDoubleDescribe)(void *diagnostic, const void *arguments);

typedef struct {
    const char *edge;
    const char *identity;
    OpalTestDoubleSnapshot snapshot;
    OpalTestDoubleDrop drop_snapshot;
} OpalTestDoubleDescriptor;

typedef void (*OpalTestDoubleObserver)(const OpalTestDoubleDescriptor *descriptor, int active, void *context);
void opal_test_set_double_observer(OpalTestDoubleObserver observer, void *context);

typedef enum {
    OPAL_TEST_DOUBLE_ORIGINAL = 0,
    OPAL_TEST_DOUBLE_RETURN = 1,
    OPAL_TEST_DOUBLE_APPLICATION_ERROR = 2,
    OPAL_TEST_DOUBLE_FAIL = 3,
    OPAL_TEST_DOUBLE_REPLACEMENT = 4
} OpalTestDoubleKind;

typedef struct {
    int kind;
    void *payload;
} OpalTestDoubleSelection;

/* Descriptors are immutable compiler data with worker lifetime. Outputs are
 * published only on completion; nonzero returns carry independent abort IDs. */
uint64_t opal_test_double_activate(const OpalTestDoubleDescriptor *descriptor,
    void **handle);
uint64_t opal_test_double_restore(void *handle);
/* Configuration always consumes its owned payload, including rejection. */
uint64_t opal_test_double_add_exact(void *handle, int kind, void *payload,
    OpalTestDoubleMatch match, OpalTestDoubleMatch overlap,
    OpalTestDoubleDrop release, const char *file, uint64_t offset, uint64_t length);
uint64_t opal_test_double_fallback(void *handle, int kind, void *payload,
    OpalTestDoubleDrop release);
uint64_t opal_test_double_select(const char *edge, const void *arguments,
    OpalTestDoubleSelection *selection);
uint64_t opal_test_double_verify(void *handle);
uint64_t opal_test_double_count(void *handle, uint64_t *count);
uint64_t opal_test_double_history_matches(void *handle, const void *expected,
    OpalTestDoubleMatch match, int *found);
uint64_t opal_test_double_history_failure(void *handle, const void *expected,
    OpalTestDoubleDescribe describe, const char *file, uint64_t offset, uint64_t length);
/* Disposes framework configuration/history memory only, never resource recovery. */
void opal_test_doubles_end(void);
#endif
