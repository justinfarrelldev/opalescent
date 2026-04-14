#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>
#include <stdint.h>

char* take_input(void) {
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

void print_string(const char* s) {
    puts(s);
}

void print_int8(int8_t n) {
    printf("%d\n", (int)n);
}

void print_int16(int16_t n) {
    printf("%d\n", (int)n);
}

void print_int32(int32_t n) {
    printf("%d\n", (int)n);
}

void print_int64(int64_t n) {
    printf("%lld\n", (long long)n);
}

void print_uint8(uint8_t n) {
    printf("%u\n", (unsigned)n);
}

void print_uint16(uint16_t n) {
    printf("%u\n", (unsigned)n);
}

void print_uint32(uint32_t n) {
    printf("%u\n", (unsigned int)n);
}

void print_uint64(uint64_t n) {
    printf("%llu\n", (unsigned long long)n);
}

void print_float32(float n) {
    printf("%.6f\n", (double)n);
}

void print_float64(double n) {
    printf("%.6f\n", n);
}

static void seed_rand_once(void) {
    static int seeded = 0;
    if (!seeded) {
        srand((unsigned int)time(NULL));
        seeded = 1;
    }
}

int8_t random_int8(int8_t min, int8_t max) {
    seed_rand_once();
    if (max <= min) return min;
    return (int8_t)(min + (int8_t)(rand() % (int)(max - min + 1)));
}

int16_t random_int16(int16_t min, int16_t max) {
    seed_rand_once();
    if (max <= min) return min;
    return (int16_t)(min + (int16_t)(rand() % (int)(max - min + 1)));
}

int32_t random_int32(int32_t min, int32_t max) {
    seed_rand_once();
    if (max <= min) return min;
    return min + (int32_t)(rand() % (int)(max - min + 1));
}

int64_t random_int64(int64_t min, int64_t max) {
    seed_rand_once();
    if (max <= min) return min;
    return min + (int64_t)(rand() % (int64_t)(max - min + 1));
}

uint8_t random_uint8(uint8_t min, uint8_t max) {
    seed_rand_once();
    if (max <= min) return min;
    return (uint8_t)(min + (uint8_t)((unsigned)rand() % (unsigned)(max - min + 1)));
}

uint16_t random_uint16(uint16_t min, uint16_t max) {
    seed_rand_once();
    if (max <= min) return min;
    return (uint16_t)(min + (uint16_t)((unsigned)rand() % (unsigned)(max - min + 1)));
}

uint32_t random_uint32(uint32_t min, uint32_t max) {
    seed_rand_once();
    if (max <= min) return min;
    return min + (uint32_t)((unsigned)rand() % (unsigned)(max - min + 1));
}

uint64_t random_uint64(uint64_t min, uint64_t max) {
    seed_rand_once();
    if (max <= min) return min;
    return min + (uint64_t)((uint64_t)rand() % (uint64_t)(max - min + 1));
}

int8_t string_to_int8(const char* s) {
    if (s == NULL) return 0;
    char* end;
    long long val = strtoll(s, &end, 10);
    if (end == s || *end != '\0') return 0;
    return (int8_t)val;
}

int16_t string_to_int16(const char* s) {
    if (s == NULL) return 0;
    char* end;
    long long val = strtoll(s, &end, 10);
    if (end == s || *end != '\0') return 0;
    return (int16_t)val;
}

int32_t string_to_int32(const char* s) {
    if (s == NULL) return 0;
    char* end;
    long long val = strtoll(s, &end, 10);
    if (end == s || *end != '\0') return 0;
    return (int32_t)val;
}

int64_t string_to_int64(const char* s) {
    if (s == NULL) return 0;
    char* end;
    long long val = strtoll(s, &end, 10);
    if (end == s || *end != '\0') return 0;
    return (int64_t)val;
}

uint8_t string_to_uint8(const char* s) {
    if (s == NULL) return 0;
    char* end;
    unsigned long long val = strtoull(s, &end, 10);
    if (end == s || *end != '\0') return 0;
    return (uint8_t)val;
}

uint16_t string_to_uint16(const char* s) {
    if (s == NULL) return 0;
    char* end;
    unsigned long long val = strtoull(s, &end, 10);
    if (end == s || *end != '\0') return 0;
    return (uint16_t)val;
}

uint32_t string_to_uint32(const char* s) {
    if (s == NULL) return 0;
    char* end;
    unsigned long long val = strtoull(s, &end, 10);
    if (end == s || *end != '\0') return 0;
    return (uint32_t)val;
}

uint64_t string_to_uint64(const char* s) {
    if (s == NULL) return 0;
    char* end;
    unsigned long long val = strtoull(s, &end, 10);
    if (end == s || *end != '\0') return 0;
    return (uint64_t)val;
}
