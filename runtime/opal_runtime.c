#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>
#include <stdint.h>

char* opal_take_input(void) {
    static char buf[1024];
    if (fgets(buf, sizeof(buf), stdin) == NULL) {
        return strdup("");
    }
    size_t len = strlen(buf);
    if (len > 0 && buf[len - 1] == '\n') {
        buf[len - 1] = '\0';
    }
    return strdup(buf);
}

int64_t opal_random_int32(int64_t min, int64_t max) {
    static int seeded = 0;
    if (!seeded) {
        srand((unsigned int)time(NULL));
        seeded = 1;
    }
    if (max <= min) return min;
    return min + (int64_t)(rand() % (int)(max - min + 1));
}

int64_t opal_string_to_int32(const char* s) {
    if (s == NULL) return 0;
    char* end;
    long long val = strtoll(s, &end, 10);
    if (end == s || *end != '\0') return 0;
    return (int64_t)val;
}

void opal_print_string(const char* s) {
    puts(s);
}

void opal_print_int(int64_t n) {
    printf("%lld\n", (long long)n);
}
