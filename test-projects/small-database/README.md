# Small database

This project evaluates Opalescent and its native test runner by building a small relational database. The starting compiler revision is `9e7ee13822d0d3de1afcb5398f9fb10f2d1af37a`.

**Status:** Scalar values (`INT`, `TEXT`, `BOOL`), query tokens, and `CREATE TABLE` parsing and execution are implemented. The `src/main.op` executable exercises table creation; it is not yet a REPL. Persistence is not implemented.

## Build and verify

From the repository root (Linux, with LLVM 14 installed):

```sh
export LLVM_SYS_140_PREFIX=/usr/lib/llvm-14
cargo build --locked --bin opalescent
./target/debug/opalescent test --root test-projects/small-database
(cd test-projects/small-database && ../../target/debug/opalescent run)
```

See [COMPILER-002](../../database-implementation-observations.md#blocker-compiler-002--leaving-scope-after-reading-an-array-field-can-invalidate-the-record) for the resolved ownership defect that previously blocked table creation.

## Supported syntax so far

The parser recognizes `CREATE TABLE name (column INT, other TEXT, flag BOOL);`. It normalizes ASCII names to lowercase and retains column order. Whitespace, including line breaks, is allowed between tokens. `;` is required, and only whitespace may follow it. Execution rejects duplicate normalized table and column names. The tokenizer recognizes signed decimal `INT` literals, `true` and `false`, and single-quoted text with doubled quotes and `\\`, `\n`, `\r`, and `\t` escapes. The other statements are not yet executable.

`src/value.types.op` defines typed cell values. `src/lexer.op` returns tokens or positioned lexical errors. `src/parser.op` consumes the tokens and builds a statement without modifying database state. See the [implementation observations](../../database-implementation-observations.md) for the language evaluation and blocker evidence.
