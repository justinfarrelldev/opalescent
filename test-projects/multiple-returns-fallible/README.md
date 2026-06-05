# multiple-returns-fallible

A runnable fixture project that validates fallible labeled multiple returns after the explicit success/error ABI support lands.

## Covered behavior

- Label-safe `propagate` destructuring of a fallible labeled pair inside a helper
- Statement-guard multi-bind success with exact-name bindings
- Outer guard handling of a propagated multi-return parse failure

## Expected output

The program prints these exact lines:

- `PROPAGATE_SUM=15`
- `GUARD_SUCCESS=5,6`
- `GUARD_HANDLED=invalid digit 'o' in input`
