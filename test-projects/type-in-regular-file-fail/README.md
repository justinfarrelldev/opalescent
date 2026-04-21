# type-in-regular-file-fail

This fixture is intentionally invalid. It places a `type` declaration inside `src/main.op`,
which violates the file-role separation rule: type declarations must live in `.types.op` files.

Expected compile error: `TypeError::TypeDeclarationOutsideTypesFile`
