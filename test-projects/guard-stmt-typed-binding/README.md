# guard-stmt-typed-binding

An end-to-end fixture project that validates statement guards accept typed mutable success bindings and expose the bound value after successful completion.

## Expected output markers

The program prints deterministic markers that integration tests assert exactly:

- `TYPED_BINDING_VALUE=42`
- `TYPED_BINDING_TYPED_MUTABLE=accepted`
- `typed-binding-ok`
