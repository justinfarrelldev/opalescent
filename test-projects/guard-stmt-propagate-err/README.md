# guard-stmt-propagate-err

An end-to-end fixture project that validates a statement guard error clause can perform handling work and then forward the original error with a final `propagate err`.

## Expected output markers

The program prints deterministic markers that integration tests assert exactly:

- `INNER_GUARD_HANDLED=invalid digit 'o' in input`
- `propagated-after-handling-ok`
