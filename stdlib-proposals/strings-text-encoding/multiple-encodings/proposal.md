# Multiple Encodings Support
## Overview
This proposal allows for multiple encodings to coexist within the standard library. While UTF-8 remains the primary focus, explicit support for UTF-16, ASCII, and ISO-8859-1 is included via dedicated functions.

This design acknowledges the reality of legacy systems and cross-platform development (e.g., Windows and Java/JavaScript environments) where UTF-16 is often the native encoding. By providing clear, type-safe conversion paths for various encodings, we ensure developers can interact with external data without external dependencies.

## Assumes
- Opaque string type.
- `uint8[]` and potentially `uint16[]` for byte-level data.
- Fixed-size integer types.
- Standard error handling via `guard` and `propagate`.

## Syntax Design
Encodings are handled through dedicated, explicitly-named functions to avoid runtime dispatch or configuration objects.

```opal
let utf8_bytes = encode_string_to_utf8_bytes(text)
let utf16_bytes = encode_string_to_utf16_bytes(text)
let ascii_bytes = encode_string_to_ascii_bytes(text)

guard decode_utf16_bytes_to_string(bytes) into str else err =>
    # handle Utf16DecodeError
```

## Example Applications
```opal
import encode_string_to_utf16_bytes from standard
import decode_utf16_bytes_to_string from standard

let serialize_for_windows = f(text: string): uint16[] errors Utf16EncodeError =>
    return propagate encode_string_to_utf16_bytes(text)
```

## Strengths
- **Compatibility**: Direct support for legacy and cross-platform standards.
- **Explicitness**: Each conversion path is distinct, preventing accidental misuse.
- **Safety**: Each encoding has its own dedicated error types and handling logic.
- **Performance**: Native UTF-16 support eliminates the need for intermediate UTF-8 conversion in some environments.

## Weaknesses
- **Complexity**: Larger API surface area in the standard library.
- **Redundancy**: Developers may be confused about which encoding to use for generic tasks.
- **Implementation**: More effort is required to maintain multiple codec implementations.

## Impact on Existing Syntax
None. This proposal adds new functions to the `standard` module.

## Interactions with Other Concerns
- **Error Handling**: Each codec can have its own error types (e.g., `AsciiEncodeError` for characters > 127).
- **FFI**: Crucial for interacting with systems that don't use UTF-8 as their native format.

## Implementation Difficulty
Medium. Requires multiple codec implementations in the compiler or runtime.

## Must NOT Have
- No `Encoding` enum.
- No `string_to_bytes(text, encoding)` with runtime dispatch.
- No support for variable-width encodings other than UTF-8 and UTF-16.
