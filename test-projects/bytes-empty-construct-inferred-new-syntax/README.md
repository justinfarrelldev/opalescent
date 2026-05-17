# bytes-empty-construct-inferred-new-syntax

An end-to-end test project for inferred `new Bytes` constructor typing. It verifies that an empty Bytes buffer compiles without an explicit type annotation, runs, and reports a length of zero.

## How to compile and run

Use the Opalescent compiler to compile and execute the program:

```bash
opal src/main.op
```

This will print `inferred syntax length: 0` to stdout.
