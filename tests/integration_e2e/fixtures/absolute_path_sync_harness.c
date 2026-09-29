#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "runtime/opal_runtime.h"

FsPathResult absolute_path_sync(const char* path);

int main(void) {
    FsPathResult empty = absolute_path_sync("");
    if (empty.error == NULL) {
        fprintf(stderr, "ERR:missing-empty-error\n");
        return 2;
    }
    if (strstr(empty.error, "InvalidPathError:") == NULL) {
        fprintf(stderr, "ERR:bad-empty-error=%s\n", empty.error);
        free((void*)empty.error);
        return 3;
    }
    free((void*)empty.error);

    char long_input[512];
    for (size_t i = 0; i < sizeof(long_input) - 1; i++) {
        long_input[i] = 'a';
    }
    long_input[sizeof(long_input) - 1] = '\0';

    FsPathResult invalid = absolute_path_sync(long_input);
    if (invalid.error == NULL) {
        fprintf(stderr, "ERR:missing-invalid-error\n");
        return 4;
    }
    if (strstr(invalid.error, "InvalidPathError:") == NULL) {
        fprintf(stderr, "ERR:bad-invalid-error=%s\n", invalid.error);
        free((void*)invalid.error);
        return 5;
    }
    free((void*)invalid.error);

    FsPathResult linux_abs = absolute_path_sync("/tmp");
    if (linux_abs.error != NULL) {
        fprintf(stderr, "ERR:linux-abs-failed=%s\n", linux_abs.error);
        free((void*)linux_abs.error);
        return 6;
    }
    if (linux_abs.value == NULL) {
        fprintf(stderr, "ERR:linux-abs-null\n");
        return 7;
    }
    printf("linux-abs=%s\n", linux_abs.value);
    free(linux_abs.value);

    FsPathResult drive_abs = absolute_path_sync("C:\\Users\\foo");
    if (drive_abs.error != NULL) {
        fprintf(stderr, "ERR:drive-abs-failed=%s\n", drive_abs.error);
        free((void*)drive_abs.error);
        return 8;
    }
    if (drive_abs.value == NULL) {
        fprintf(stderr, "ERR:drive-abs-null\n");
        return 9;
    }
    printf("drive-abs=%s\n", drive_abs.value);
    free(drive_abs.value);

    FsPathResult unc_abs = absolute_path_sync("\\\\server\\share\\dir\\file.ext");
    if (unc_abs.error != NULL) {
        fprintf(stderr, "ERR:unc-abs-failed=%s\n", unc_abs.error);
        free((void*)unc_abs.error);
        return 10;
    }
    if (unc_abs.value == NULL) {
        fprintf(stderr, "ERR:unc-abs-null\n");
        return 11;
    }
    printf("unc-abs=%s\n", unc_abs.value);
    free(unc_abs.value);

    return 0;
}
