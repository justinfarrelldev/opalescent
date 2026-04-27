# _fs_append_log

A focused append showcase fixture that appends five distinct lines to a log file and confirms readback line count using `read_lines_sync`.

## How to run

```bash
opal run src/main.op
```

Expected stdout:

```text
appended 5 lines; readback confirmed
```
