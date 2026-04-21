# v4-and-v7-Separate

## Overview

This proposal provides UUID generation via two explicit, version-specific functions: `generate_uuid_v4(rng: RandomNumberGenerator)` and `generate_uuid_v7(rng: RandomNumberGenerator, timestamp_milliseconds: int64)`. Each function is a pure computation given an RNG and optional timestamp, returning a 128-bit UUID as a byte array. Parsing is unified via `parse_uuid_string(string): uint8[] errors UuidParseError`. This design prioritizes clarity: callers explicitly choose which UUID version they need, and the API surface is minimal.

## Assumes

- `RandomNumberGenerator` type from `random-rng` concern is available and provides stateless RNG operations
- UUID v4 (random) and v7 (timestamp-based) are the only versions required
- UUIDs are represented as `uint8[]` (16 bytes)
- String representation follows RFC 4122 format: `xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx`
- No deferred/wait_for or concurrency primitives are needed for generation

## Syntax Design

```opal
##
  Description: Generate a UUID v4 (random) given an RNG.
##
let generate_uuid_v4 = f(rng: RandomNumberGenerator): uint8[] =>
    # Generate 16 random bytes, set version/variant bits
    return bytes

##
  Description: Generate a UUID v7 (timestamp-based) given an RNG and timestamp.
##
let generate_uuid_v7 = f(rng: RandomNumberGenerator, timestamp_milliseconds: int64): uint8[] =>
    # Encode timestamp in first 48 bits, random in remaining bits
    return bytes

##
  Description: Parse a UUID string into bytes, or error if invalid.
##
let parse_uuid_string = f(uuid_str: string): uint8[] errors UuidParseError =>
    # Validate format, decode hex, return 16 bytes
    return bytes
```

## Example Applications

**Generating a v4 UUID:**
```opal
let rng = new_random_number_generator(seed)
let uuid_v4 = generate_uuid_v4(rng)
```

**Generating a v7 UUID with current timestamp:**
```opal
let rng = new_random_number_generator(seed)
let now_ms = current_time_milliseconds()
let uuid_v7 = generate_uuid_v7(rng, now_ms)
```

**Parsing and displaying:**
```opal
guard parse_uuid_string('550e8400-e29b-41d4-a716-446655440000') into bytes else err =>
    print('Invalid UUID')
    return void
let hex_str = uuid_bytes_to_string(bytes)
print('Parsed: {hex_str}')
```

## Strengths

- **Explicit version selection**: Caller must choose v4 or v7; no hidden defaults.
- **Minimal API surface**: Only three functions; easy to learn and maintain.
- **Pure computation**: No side effects; generation is deterministic given RNG + timestamp.
- **Composable with random-rng**: RNG is threaded through as a parameter.
- **Clear error handling**: `UuidParseError` is the only error type.

## Weaknesses

- **No type safety on UUID versions**: Both v4 and v7 return `uint8[]`; caller must track which version was generated.
- **Caller responsibility**: Timestamp must be provided explicitly for v7; no automatic "now" capture.
- **Limited extensibility**: Adding v1, v5, or v6 requires new functions; no unified interface.

## Impact on Existing Syntax

No breaking changes. This is a new concern that does not modify existing Opalescent syntax or semantics. It introduces three new public functions and one error type.

## Interactions with Other Concerns

- **random-rng**: Depends on `RandomNumberGenerator` type and RNG operations. RNG is passed explicitly to generation functions.
- **error-strategy**: Uses `UuidParseError` for parse failures; integrates with `guard` and `propagate` patterns.
- **time-date-api** (future): v7 generation requires a timestamp; could integrate with a future time concern for automatic "now" capture.

## Implementation Difficulty

Low to moderate. UUID v4 is straightforward (16 random bytes with version/variant bits set). UUID v7 requires bit-packing a 48-bit timestamp and 80 random bits. Parsing is a standard hex-decode with format validation. Estimated effort: 1–2 weeks.

## Must NOT Have

- No `_sync` suffix; generation is CPU-only and pure.
- No `deferred` or `wait_for`; no concurrency primitives.
- No automatic timestamp capture; caller provides timestamp for v7.
- No `Result<T, E>` or `Option<T>` wrappers; use native error handling.
- No semicolons in code.
