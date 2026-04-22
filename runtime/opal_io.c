#include "opal_portability.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static char* duplicate_without_trailing_newline(const char* source) {
    char* raw = strdup(source);
    if (!raw) {
        fprintf(stderr, "Runtime error: out of memory\n");
        exit(1);
    }
    size_t len = strlen(raw);
    if (len > 0 && raw[len - 1] == '\n') {
        raw[len - 1] = '\0';
    }

    size_t trimmed_len = strlen(raw);
    char* out = (char*)malloc(trimmed_len + 1);
    if (!out) {
        fprintf(stderr, "Runtime error: out of memory\n");
        exit(1);
    }
    memcpy(out, raw, trimmed_len + 1);
    free(raw);
    return out;
}

char* take_input(void) {
    char* line = NULL;
    size_t len = 0;
    ssize_t read = getline(&line, &len, stdin);

    if (read == -1) {
        if (line != NULL) {
            free(line);
        }
        return duplicate_without_trailing_newline("");
    }

    char* result = duplicate_without_trailing_newline(line);
    free(line);
    return result;
}

void print_string(const char* s) {
    puts(s);
}
