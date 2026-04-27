# _fs_normalize_path

A focused fixture that showcases lexical `normalize_path` behavior across canonical collapse cases, including the infallible empty-sentinel branch when an absolute path escapes root.

## How to compile and run

Use the Opalescent compiler to compile and execute the program:

```bash
opal run src/main.op
```

Expected stdout:

```text
a//b -> a/b
a/./b -> a/b
a/b/.. -> a
a/b/../../c -> c
./a -> a
/a/b/../../.. -> empty-sentinel
```
