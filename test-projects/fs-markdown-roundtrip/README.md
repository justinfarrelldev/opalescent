# fs-markdown-roundtrip

An end-to-end Opalescent fixture that reads a committed markdown file, applies a deterministic line-based paragraph-to-blockquote transform, writes the result to `workspace/output.md`, and verifies the output bytes match the expected fixture.

## How to run

```bash
opal run src/main.op
```

Expected stdout:

```text
roundtrip: ok (547 bytes match)
```
