# multiple-returns-cross-module

A runnable fixture project that validates imported multi-return label metadata remains available to callers.

## Covered behavior

- Exact-name destructuring against labels declared in another module
- Explicit `returned_label: local_name` renaming against imported label metadata
- Imported return pass-through that preserves ordered labels

## Expected output

The program prints these exact lines:

- `IMPORTED=3,4`
- `RENAMED=3,4`
- `FORWARDED=7`
