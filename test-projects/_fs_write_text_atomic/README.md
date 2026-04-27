# _fs_write_text_atomic

A focused fixture that demonstrates an atomic-write pattern using `write_text_sync` to a temporary file, then `move_path_sync` into place.

## How to run

```bash
opal run src/main.op
```

Expected stdout:

```text
wrote atomically: 14
```
