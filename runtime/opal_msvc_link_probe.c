#include "opal_runtime.h"
#include "opal_fs_errors.h"
#include "opal_portability.h"

static const char* k_probe_error_discriminant = OPAL_FS_ERR_NOT_FOUND;

typedef FsStringResult (*opal_read_text_sync_fn_t)(const char* path);

int main(void) {
    char sep = opal_path_separator();
    void* read_text_sync_addr = (void*)(opal_read_text_sync_fn_t)&read_text_sync;

    if (sep == '\0' || k_probe_error_discriminant == NULL || read_text_sync_addr == NULL) {
        return 1;
    }

    return 0;
}
