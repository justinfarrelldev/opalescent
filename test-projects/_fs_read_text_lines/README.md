# _fs_read_text_lines

A focused read-family fixture that exercises `read_lines_sync`, `read_first_line_sync`, and `read_text_sync` together against a mixed line-ending sample file.

## How to run

```bash
opal run src/main.op
```

Expected stdout on the happy path:

```text
lines=4
first=alpha
match=true
```
