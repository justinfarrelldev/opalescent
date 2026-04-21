# Codepoint-First String API
## Overview
This proposal shifts the focus of the string API from byte-level representation to Unicode codepoints. In this model, strings are primarily treated as sequences of Unicode scalar values (represented as `uint32[]` arrays), and conversion to and from bytes is a secondary operation.

By prioritizing codepoints, we provide a more intuitive API for text processing that remains consistent regardless of the underlying storage format. This approach simplifies operations like string length, indexing, and character manipulation by making them scalar-aware by default.

## Assumes
- Opaque string type.
- `uint32[]` array for representing codepoint sequences.
- Fixed-size integer types (int32, uint32).
- Native error handling via `guard` and `propagate`.

## Syntax Design
Functions for converting between strings and codepoint arrays are central to this design.

```opal
let codepoints: uint32[] = string_to_codepoint_array(text)
let str: string = codepoint_array_to_string(codepoints)

# length is naturally codepoint-based
let len: int32 = string_codepoint_length(text)
```

## Example Applications
```opal
import string_to_codepoint_array, codepoint_array_to_string from standard

let process_codepoints = f(text: string): string =>
    let codepoints = string_to_codepoint_array(text)
    # perform some codepoint-aware logic...
    return codepoint_array_to_string(codepoints)
```

## Strengths
- **Intuitive**: Matches the conceptual model of text as a sequence of characters/codepoints.
- **Consistency**: Length and indexing behave predictably without byte-level surprises.
- **Robustness**: Reduces the likelihood of bugs caused by splitting multi-byte sequences.
- **Text-Focused**: Excellent for advanced text analysis and manipulation.

## Weaknesses
- **Memory Overhead**: Representing every codepoint as a `uint32` is less memory-efficient than UTF-8 for many languages.
- **Conversion Performance**: Requires an intermediate codepoint array for most processing, adding overhead.
- **Complexity**: Developers still need byte-level operations for I/O, requiring two sets of conversion functions.

## Impact on Existing Syntax
None. This proposal adds new functions to the `standard` module.

## Interactions with Other Concerns
- **Error Handling**: Conversion from codepoint arrays to strings must validate that each `uint32` is a valid Unicode scalar value.
- **FFI**: Requires conversion when interacting with systems that expect specific byte encodings.

## Implementation Difficulty
Low to Medium. The logic for codepoint conversion is straightforward but requires careful validation.

## Must NOT Have
- No direct indexing into strings that returns raw bytes.
- No support for surrogate pairs in the codepoint representation.
- No `Codepoint` object or class; use `uint32` for scalar values.
