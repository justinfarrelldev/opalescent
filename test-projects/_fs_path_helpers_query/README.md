# _fs_path_helpers_query

A focused fixture that queries three path helpers (`path_file_extension`, `path_file_name`, and `path_parent_directory`) against a fixed five-path matrix.

## How to compile and run

Use the Opalescent compiler to compile and execute the program:

```bash
opal run src/main.op
```

Expected stdout:

```text
/home/user/doc.pdf: ext=pdf, name=doc.pdf, parent=/home/user
/home/user/: ext=, name=, parent=/home/user
noext: ext=, name=noext, parent=.
a/b/c.tar.gz: ext=gz, name=c.tar.gz, parent=a/b
/: ext=, name=, parent=/
```
