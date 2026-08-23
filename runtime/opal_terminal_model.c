#include "opal_portability.h"
#include <stdint.h>
#include <stdlib.h>
#include <string.h>

#ifndef OPAL_FS_HANDLE_RESULT_DEFINED
typedef struct { void* value; const char* error; } FsHandleResult;
#define OPAL_FS_HANDLE_RESULT_DEFINED 1
#endif

typedef struct OpalTerminalSessionOptions {
    uint32_t magic;
} OpalTerminalSessionOptions;

enum {
    OPAL_TERMINAL_OPTIONS_MAGIC = 0x4F54534Fu
};

static char* opal_terminal_model_duplicate(const char* source) {
    const char* text = source != NULL ? source : "";
    size_t length = strlen(text);
    char* copy = (char*)malloc(length + 1u);
    if (copy == NULL) {
        return NULL;
    }
    memcpy(copy, text, length + 1u);
    return copy;
}

void* terminal_session_options_default(void) {
    OpalTerminalSessionOptions* options =
        (OpalTerminalSessionOptions*)calloc(1u, sizeof(OpalTerminalSessionOptions));
    if (options == NULL) {
        return NULL;
    }
    options->magic = OPAL_TERMINAL_OPTIONS_MAGIC;
    return options;
}

FsHandleResult terminal_session_options_validate(void* opaque_options) {
    OpalTerminalSessionOptions* options = (OpalTerminalSessionOptions*)opaque_options;
    if (options == NULL || options->magic != OPAL_TERMINAL_OPTIONS_MAGIC) {
        return (FsHandleResult){
            NULL,
            "TerminalSessionOptionsError.InvalidOptions"
        };
    }
    return (FsHandleResult){ opaque_options, NULL };
}

FsHandleResult trusted_terminal_output_from_application_text(const char* text) {
    char* trusted = opal_terminal_model_duplicate(text);
    if (trusted == NULL) {
        return (FsHandleResult){ NULL, "AllocationFailureError" };
    }
    return (FsHandleResult){ trusted, NULL };
}
