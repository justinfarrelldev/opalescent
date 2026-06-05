# multiple-returns-basic

A runnable fixture project that validates ordinary labeled multiple returns in user-facing project form.

## Covered behavior

- Exact-name destructuring from a labeled multi-return call
- Explicit `returned_label: local_name` renaming during destructuring
- Return pass-through through another labeled multi-return helper

## Expected output

The program prints these exact lines:

- `EXACT=11,22`
- `RENAMED=11,22`
- `PASS_THROUGH=33`
