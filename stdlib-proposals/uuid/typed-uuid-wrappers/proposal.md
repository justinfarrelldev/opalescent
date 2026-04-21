# typed-uuid-wrappers

## Overview

This proposal introduces two distinct product types, `UuidV4` and `UuidV7`, to provide compile-time type safety for UUID versions. Generation functions return the appropriate type; parsing returns a sum type `UuidV4 | UuidV7`. Conversion functions (`uuid_v4_to_string`, `uuid_v7_to_string`, `uuid_to_bytes`) are version-specific. This design leverages Opalescent's algebraic types to prevent accidental mixing of UUID versions and provides stronger compile-time guarantees.

## Assumes

- `RandomNumberGenerator` type from `random-rng` concern is available
- UUID v4 (random) and v7 (timestamp-based) are the only versions required
- Product types (`UuidV4`, `UuidV7`) and sum types are fully supported
- UUIDs are internally represented as `uint8[]` (16 bytes)
- String representation follows RFC 4122 format: `xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx`

## Syntax Design

```opal


##
  Description: Generate a UUID v4 (random) given an RNG.
##
let generate_uuid_v4 = f(rng: RandomNumberGenerator): UuidV4 =>
    return new UuidV4:
        bytes: bytes

##
  Description: Generate a UUID v7 (timestamp-based) given an RNG and timestamp.
##
let generate_uuid_v7 = f(rng: RandomNumberGenerator, timestamp_milliseconds: int64): UuidV7 =>
    return new UuidV7:
        bytes: bytes
        timestamp_milliseconds: timestamp_milliseconds

##
  Description: Parse a UUID string into either UuidV4 or UuidV7 based on version bits.
##
let parse_uuid_string = f(uuid_str: string): UuidV4 | UuidV7 errors UuidParseError =>
    # Decode and inspect version bits
    return UuidV4 { bytes: bytes }
```

## Example Applications

**Generating and converting v4:**
```opal
let rng = new_random_number_generator(seed)
let uuid_v4 = generate_uuid_v4(rng)
let uuid_v4_str = uuid_v4_to_string(uuid_v4)
```

**Generating and converting v7:**
```opal
let rng = new_random_number_generator(seed)
let now_ms = current_time_milliseconds()
let uuid_v7 = generate_uuid_v7(rng, now_ms)
let uuid_v7_str = uuid_v7_to_string(uuid_v7)
```

**Parsing and pattern matching:**
```opal
guard parse_uuid_string('550e8400-e29b-41d4-a716-446655440000') into uuid else err =>
    print('Invalid UUID')
    return void
match uuid:
    UuidV4 { bytes } => print('v4: {uuid_v4_to_string(uuid)}')
    UuidV7 { bytes, timestamp_milliseconds } => print('v7 at {timestamp_milliseconds}ms')
```

## Strengths

- **Type-safe version tracking**: Compiler prevents mixing v4 and v7 UUIDs.
- **Exhaustive pattern matching**: Callers must handle both versions when parsing.
- **Metadata preservation**: v7 stores timestamp; v4 stores only bytes.
- **Idiomatic Opalescent**: Uses product and sum types as designed.
- **Clear intent**: Type signature reveals which version is expected.

## Weaknesses

- **More boilerplate**: Three conversion functions instead of one generic.
- **Larger API surface**: Two types + three conversion functions + one sum type.
- **Parsing complexity**: Must inspect version bits to determine which variant to return.
- **Less flexible**: Adding v1, v5, or v6 requires new types and functions.

## Impact on Existing Syntax

No breaking changes. This is a new concern introducing two product types, one sum type, and four public functions. No modifications to existing Opalescent syntax.

## Interactions with Other Concerns

- **random-rng**: Depends on `RandomNumberGenerator` type. RNG is passed explicitly to generation functions.
- **error-strategy**: Uses `UuidParseError` for parse failures; integrates with `guard` and `propagate` patterns.
- **type system**: Leverages product and sum types for version safety; pattern matching is required for parsed UUIDs.

## Implementation Difficulty

Low to moderate. Requires defining two product types and implementing version-specific conversion functions. Parsing must inspect version bits (byte 6 for v4, byte 6 for v7) to determine which variant to construct. Estimated effort: 1–2 weeks.

## Must NOT Have

- No `_sync` suffix; generation is CPU-only and pure.
- No `deferred` or `wait_for`; no concurrency primitives.
- No automatic timestamp capture; caller provides timestamp for v7.
- No `Result<T, E>` or `Option<T>` wrappers; use native error handling.
- No semicolons in code.
