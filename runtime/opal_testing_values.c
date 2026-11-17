/* FilesystemPath currently has an immutable UTF-8 char* value representation.
 * Framework snapshots own managed copies; no borrowed source address is saved. */
#include "opal_portability.h"
#include "opal_runtime.h"
#include "opal_testing_values.h"
#include <string.h>
#include <stdint.h>

char *opal_test_path_snapshot(const char *path) {
    if (!path) return NULL;
    size_t length = strlen(path);
    if (length == SIZE_MAX) return NULL;
    char *copy = opal_string_alloc((uint64_t)(length + 1));
    if (copy) memcpy(copy, path, length + 1);
    return copy;
}
