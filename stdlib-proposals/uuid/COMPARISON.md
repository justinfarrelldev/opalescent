# UUID Generation Concern — Comparison

This document compares two alternatives for UUID generation in Opalescent: **v4-and-v7-separate** and **typed-uuid-wrappers**. Both alternatives compose with the `random-rng` concern and provide RFC 4122-compliant UUID generation and parsing.

## Summary Matrix

| Axis | v4-and-v7-separate | typed-uuid-wrappers |
|------|-------------------|---------------------|
| **Ergonomics** | ★★★★☆ | ★★★☆☆ |
| **Error-model fit** | ★★★★★ | ★★★★★ |
| **Opalescent-idiom fit** | ★★★☆☆ | ★★★★★ |
| **Implementation effort** | Low (1–2 weeks) | Low (1–2 weeks) |
| **Extensibility** | ★★☆☆☆ | ★★★☆☆ |
| **Async readiness** | ★★★★☆ | ★★★★☆ |

## Analysis

### v4-and-v7-separate: The Explicit Path

**Ergonomics**: High. Minimal API surface (3 functions); caller explicitly chooses version. No type overhead; straightforward byte-array returns.

**Error-model fit**: Perfect. `UuidParseError` integrates naturally with `guard` and `propagate` patterns. No wrapper types to unwrap.

**Opalescent-idiom fit**: Moderate. Uses explicit functions and error handling, but does not leverage product/sum types for version safety. Caller must track which version was generated.

**Implementation effort**: Low. Straightforward bit-packing for v4 and v7; standard hex-decode for parsing. No complex type machinery.

**Extensibility**: Limited. Adding v1, v5, or v6 requires new functions. No unified interface for all versions. Caller must know which function to call.

**Async readiness**: High. Pure computation; no blocking I/O or concurrency concerns. Async wrapper would be trivial.

### typed-uuid-wrappers: The Type-Safe Path

**Ergonomics**: Moderate. More boilerplate (3 conversion functions); pattern matching required for parsed UUIDs. Type safety adds cognitive load but prevents bugs.

**Error-model fit**: Perfect. `UuidParseError` integrates naturally. Sum type `UuidV4 | UuidV7` forces exhaustive handling.

**Opalescent-idiom fit**: Excellent. Leverages product types (`UuidV4`, `UuidV7`) and sum types for version safety. Aligns with "immutable-by-default" and "explicit but concise" philosophy. Pattern matching is idiomatic.

**Implementation effort**: Low. Same bit-packing and hex-decode logic; adds type definitions and version-specific conversion functions. No additional complexity.

**Extensibility**: Moderate. Adding v1, v5, or v6 requires new product types and conversion functions. Sum type grows but remains manageable. Caller intent is always clear from type signature.

**Async readiness**: High. Pure computation; no blocking I/O or concurrency concerns. Async wrapper would be trivial.

## Recommendation

**For Opalescent's design philosophy, `typed-uuid-wrappers` is the stronger choice.**

- **Type safety**: Compile-time guarantees prevent version confusion.
- **Idiomatic**: Aligns with Opalescent's emphasis on algebraic types and pattern matching.
- **Maintainability**: Version-specific types make intent explicit; future maintainers cannot accidentally mix versions.
- **Extensibility**: While adding new versions requires new types, the pattern is clear and scalable.

**Trade-off**: Slightly more boilerplate than `v4-and-v7-separate`, but the safety and clarity gains justify the cost.

## Composition with random-rng

Both alternatives thread `RandomNumberGenerator` through generation functions as an explicit parameter. This aligns with the `random-rng` concern's design: RNG is a value, not a global resource. No implicit state or side effects.

## Error Handling

Both alternatives use `UuidParseError` for parse failures. Integration with `guard` and `propagate` is identical:

```opal
guard parse_uuid_string(uuid_str) into uuid else err =>
    print('Parse error: {err.message}')
    return void
```

## Round-Trip Guarantees

Both alternatives support round-trip parsing and serialization:

1. Generate a UUID (v4 or v7)
2. Convert to string via `uuid_*_to_string` or `uuid_bytes_to_string`
3. Parse the string back via `parse_uuid_string`
4. Verify bytes match original

This is demonstrated in both `parse_and_display.op` files.
