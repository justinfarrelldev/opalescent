#include <stdio.h>
#include <stdlib.h>

void opal_runtime_error(const char* message) {
    fprintf(stderr, "%s\n", message);
    exit(1);
}
