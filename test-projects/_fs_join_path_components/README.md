# _fs_join_path_components

A focused fixture that demonstrates infallible `join_path_components` behavior across five locked lexical join cases.

## How to compile and run

Use the Opalescent compiler to compile and execute the program:

```bash
opal run src/main.op
```

Expected stdout:

```text
home + [user, docs] -> home/user/docs
a/ + [b] -> a/b
a + [/b, c] -> /b/c
a + [] -> a
`` + [x] -> x
```
