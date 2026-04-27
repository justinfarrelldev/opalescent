# _fs_path_from

A minimal fixture that exercises baseline `path_from` end-to-end. `src/main.op` validates identity behavior (`hello/world` remains unchanged in output), and `src/paths.op` is included as an auxiliary second module to keep this fixture explicitly multi-file.

## How to compile and run

Use the Opalescent compiler to compile and execute the program:

```bash
opal run src/main.op
```

Expected stdout:

```text
path=hello/world
```
