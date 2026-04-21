# Raw uint8[] Arrays
<!-- Provide a clear, descriptive name for this language alternative or feature proposal. -->

## Overview
This proposal advocates for using the existing `uint8[]` array type as the primary representation for byte buffers. Rather than introducing a specialized type, the standard library provides a suite of helper functions that operate directly on these primitive arrays.

This approach prioritizes simplicity and consistency with the rest of the language's array handling. It avoids the cognitive overhead of learning a new type while leveraging the existing `uint8` primitive.

## Assumes
This proposal assumes the existence of the `uint8` primitive type and `T[]` array syntax as defined in the language specification. It also relies on the standard library's ability to provide free functions that operate on arrays.

## Syntax Design
No new syntax is introduced. All operations are performed via function calls on `uint8[]` values.

```opal
# Example of concatenating two byte arrays
let combined = concatenate_byte_arrays(first, second)

# Example of converting to hex
let hex_string = convert_bytes_to_hex_string(data)
```

## Example Applications
A typical application involves reading a file into a raw array and processing it.

```opal
import read_file_sync from standard
import convert_bytes_to_hex_string from standard

let process_file = f(path: string): string errors FileReadError, FileNotFoundError =>
    let bytes = propagate read_file_sync(path)
    return convert_bytes_to_hex_string(bytes)
```

## Strengths
- **Simplicity**: No new types to learn or manage.
- **Consistency**: Byte buffers behave exactly like any other array in the language.
- **Zero Overhead**: No wrapper structs or additional metadata beyond what arrays already provide.
- **Interoperability**: Functions that accept `uint8[]` can work with byte buffers without conversion.

## Weaknesses
- **Lack of Type Safety**: It's impossible to distinguish a "byte buffer" from a generic "array of small integers" at the type level.
- **Ergonomics**: Operations must be called as free functions rather than methods (e.g., `concatenate_byte_arrays(a, b)` vs `a.concatenate(b)`).
- **Discovery**: Helpers might be harder to find compared to methods on a dedicated type.

## Impact on Existing Syntax
This proposal has zero impact on existing syntax as it uses established patterns.

## Interactions with Other Concerns
- **Error Handling**: Follows the standard `guard` and `propagate` patterns.
- **Memory Model**: Aligns with the Perceus+SCR model used for all arrays.
- **LSP**: Standard autocompletion for free functions in the `standard` module.

## Implementation Difficulty
Low. It only requires adding a set of helper functions to the standard library.

## Must NOT Have
- This proposal must NOT introduce a `Bytes` or `Buffer` type alias or struct.
- It must NOT use `[uint8]` syntax.
- It must NOT use semicolons.
