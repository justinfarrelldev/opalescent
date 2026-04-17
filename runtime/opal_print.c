#include <stdio.h>
#include <stdint.h>
#include <inttypes.h>

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
    printf("%" PRId64 "\n", n);
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
    printf("%" PRIu64 "\n", n);
}

void print_float32(float n) {
    printf("%.6f\n", (double)n);
}

void print_float64(double n) {
    printf("%.6f\n", n);
}
