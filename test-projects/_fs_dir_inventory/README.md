# _fs_dir_inventory

A focused directory-family fixture that creates an inventory directory, writes and lists three files, verifies each readback value, and performs full cleanup.

## How to run

```bash
opal run src/main.op
```

Expected stdout:

```text
inventory: 3 files; cleanup ok
```
