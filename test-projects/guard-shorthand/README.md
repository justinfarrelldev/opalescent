# guard-shorthand

An end-to-end fixture project that validates statement guard shorthand (`guard <call> else <err> =>`) and named-binding guards (`guard <call> into value else <err> =>`) in one host integration run.

## Expected output markers

The program prints deterministic markers that integration tests assert exactly:

- `GUARD_SHORTHAND_SUCCESS=ok`
- `GUARD_SHORTHAND_ERROR=handled`
- `GUARD_NAMED_BINDING=41`
