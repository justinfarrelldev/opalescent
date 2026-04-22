#include "opal_portability.h"
#include <stdio.h>
#include <stdlib.h>
#include <time.h>
#include <stdint.h>

#if OPAL_WINDOWS
#include <bcrypt.h>
#endif

static uint64_t xorshift128plus_state[2];
static int prng_seeded = 0;

static uint64_t xorshift128plus(void) {
    uint64_t s1 = xorshift128plus_state[0];
    uint64_t s0 = xorshift128plus_state[1];
    xorshift128plus_state[0] = s0;
    s1 ^= s1 << 23;
    s1 ^= s1 >> 17;
    s1 ^= s0;
    s1 ^= s0 >> 26;
    xorshift128plus_state[1] = s1;
    return s0 + s1;
}

static void seed_prng_once(void) {
    if (prng_seeded) return;
    uint64_t s0 = 0, s1 = 0;
    
#if OPAL_WINDOWS
    /* Windows: use BCryptGenRandom for cryptographically secure random bytes */
    NTSTATUS status = BCryptGenRandom(NULL, (PUCHAR)&s0, sizeof(s0), BCRYPT_USE_SYSTEM_PREFERRED_RNG);
    if (BCRYPT_SUCCESS(status)) {
        status = BCryptGenRandom(NULL, (PUCHAR)&s1, sizeof(s1), BCRYPT_USE_SYSTEM_PREFERRED_RNG);
    }
    if (!BCRYPT_SUCCESS(status)) {
        /* Fallback if BCryptGenRandom fails */
        s0 = 0;
        s1 = 0;
    }
#else
    /* Unix: use /dev/urandom */
    FILE* urandom = fopen("/dev/urandom", "rb");
    if (urandom) {
        (void)fread(&s0, sizeof(s0), 1, urandom);
        (void)fread(&s1, sizeof(s1), 1, urandom);
        fclose(urandom);
    }
#endif
    
    if (s0 == 0 && s1 == 0) {
        s0 = (uint64_t)time(NULL) ^ ((uint64_t)clock() << 32);
        s1 = s0 ^ 0x9e3779b97f4a7c15ULL;
    }
    xorshift128plus_state[0] = s0;
    xorshift128plus_state[1] = s1;
    prng_seeded = 1;
}

int8_t random_int8(int8_t min, int8_t max) {
    seed_prng_once();
    if (max <= min) return min;
    uint64_t range = (uint64_t)max - (uint64_t)min + 1ULL;
    return (int8_t)(min + (int8_t)(xorshift128plus() % range));
}

int16_t random_int16(int16_t min, int16_t max) {
    seed_prng_once();
    if (max <= min) return min;
    uint64_t range = (uint64_t)max - (uint64_t)min + 1ULL;
    return (int16_t)(min + (int16_t)(xorshift128plus() % range));
}

int32_t random_int32(int32_t min, int32_t max) {
    seed_prng_once();
    if (max <= min) return min;
    uint64_t range = (uint64_t)max - (uint64_t)min + 1ULL;
    return min + (int32_t)(xorshift128plus() % range);
}

int64_t random_int64(int64_t min, int64_t max) {
    seed_prng_once();
    if (max <= min) return min;
    uint64_t range = (uint64_t)max - (uint64_t)min + 1ULL;
    return min + (int64_t)(xorshift128plus() % range);
}

uint8_t random_uint8(uint8_t min, uint8_t max) {
    seed_prng_once();
    if (max <= min) return min;
    return (uint8_t)(min + (uint8_t)(xorshift128plus() % (uint64_t)(max - min + 1)));
}

uint16_t random_uint16(uint16_t min, uint16_t max) {
    seed_prng_once();
    if (max <= min) return min;
    return (uint16_t)(min + (uint16_t)(xorshift128plus() % (uint64_t)(max - min + 1)));
}

uint32_t random_uint32(uint32_t min, uint32_t max) {
    seed_prng_once();
    if (max <= min) return min;
    return min + (uint32_t)(xorshift128plus() % (uint64_t)(max - min + 1));
}

uint64_t random_uint64(uint64_t min, uint64_t max) {
    seed_prng_once();
    if (max <= min) return min;
    return min + (xorshift128plus() % (max - min + 1));
}
