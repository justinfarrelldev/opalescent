# value-in-types-file-fail

This fixture is intentionally invalid. It places a `let` declaration inside `src/models.types.op`,
which violates the STRICT separation rule: `.types.op` files may only contain type declarations.

Expected compile error: `TypeError::NonTypeDeclarationInTypesFile`
