# _absolute_path_sync

A focused fixture that exercises `absolute_path_sync` deterministically for four path classes: existing relative, non-existing relative, `..` path normalization, and already-absolute input.

## Behavior contract documented from current runtime implementation

This fixture reflects the current runtime behavior as implemented in `runtime/opal_fs.c::absolute_path_sync` plus `runtime/opal_portability.h::opal_realpath`:

- Empty input fails with `InvalidPathError` (fixture does not include empty input).
- Non-empty input calls `opal_realpath`.
- On POSIX, `opal_realpath` first tries `realpath` and, when `errno == ENOENT`, falls back to lexical absolute resolution (cwd + normalize `.` / `..`).
- Therefore, a non-existing relative path resolves successfully to an absolute lexical path (not an error).
- Output format is deterministic per line: `<input> -> <absolute>` on success, `<input>: error` on failure.

## How to run

```bash
opal run src/main.op
```

Expected stdout shape:

```text
./test-projects/_absolute_path_sync/src/main.op -> /...
./test-projects/_absolute_path_sync/does_not_exist.txt -> /...
./test-projects/_absolute_path_sync/src/../README.md -> /...
/ -> /
```
