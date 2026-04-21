# UTF-8 Bytes Only
## Overview
This proposal mandates UTF-8 as the sole encoding for all string-to-byte conversions. Strings in Opalescent are opaque sequences of Unicode scalar values, and this alternative provides a single, unambiguous path for serializing them to and from binary data.

By restricting the standard library to UTF-8, we eliminate the complexity of encoding parameters and ensure maximum compatibility with modern web and system standards. This design prioritizes simplicity and safety over legacy encoding support.

## Assumes
- Opaque string type representing Unicode text.
- `uint8[]` or a dedicated `Bytes` type for binary buffers.
- Fixed-size integer types (int32, uint32).
- Native error handling via `guard` and `propagate`.

## Syntax Design
All encoding and decoding functions are explicitly named to reflect the UTF-8 requirement.

```opal
let bytes = encode_string_to_utf8_bytes(text)
guard decode_utf8_bytes_to_string(bytes) into str else err =>
    # handle Utf8DecodeError
```

## Example Applications
```opal
import encode_string_to_utf8_bytes from standard
import decode_utf8_bytes_to_string from standard

let round_trip = f(input: string): string errors Utf8DecodeError =>
    let bytes = encode_string_to_utf8_bytes(input)
    guard decode_utf8_bytes_to_string(bytes) into output else err =>
        propagate err
    return output
```

## Strengths
- **Simplicity**: No "encoding" argument needed for standard operations.
- **Safety**: One well-defined path reduces the risk of encoding-related bugs.
- **Performance**: UTF-8 is the native format for most modern systems, minimizing conversion overhead.
- **Interoperability**: UTF-8 is the de facto standard for internet protocols and modern file formats.

## Weaknesses
- **Legacy Support**: No native support for ASCII, UTF-16, or ISO-8859-1.
- **Inflexibility**: Applications requiring other encodings must implement their own logic or use external libraries.

## Impact on Existing Syntax
None. This is a library-level proposal adding new functions to the `standard` module.

## Interactions with Other Concerns
- **Error Handling**: Decoding can fail due to malformed sequences, requiring exhaustive `errors` clauses.
- **FFI**: May require conversion when interacting with APIs that expect UTF-16 (e.g., Windows API).

## Implementation Difficulty
Low. UTF-8 codec logic is well-understood and easy to implement.

## Must NOT Have
- No generic `encode(string, encoding)` function.
- No `Result<string, Utf8DecodeError>` return type.
- No support for stateful encodings or BOM (Byte Order Mark) detection.
