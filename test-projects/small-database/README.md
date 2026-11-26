# Small database

This project evaluates Opalescent and its native test runner by building a small relational database. The starting compiler revision is `9e7ee13822d0d3de1afcb5398f9fb10f2d1af37a`.

**Status:** The in-memory REPL supports table creation, typed insertion, wildcard selections, and schema introspection. Explicit projections, predicates, limits, row updates and deletions, and persistence are not implemented yet.

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

```sql
CREATE TABLE tasks (id INT, title TEXT, done BOOL);
INSERT INTO tasks VALUES (1, 'It''s working', true);
SELECT * FROM tasks;
SHOW TABLES;
DESCRIBE tasks;
HELP;
QUIT;
```

Names begin with an ASCII letter or underscore and may then contain ASCII digits as well. Names and keywords are case-insensitive; keywords cannot be identifiers. The parser allows whitespace, including line breaks, between tokens; the REPL reads one complete query per line. Every statement needs its semicolon, with only whitespace allowed after it. Duplicate tables and columns, unknown tables, and inserts with the wrong width or value types are rejected without publishing changes.

A signed decimal `INT` must fit in 64 bits. `BOOL` literals are `true` and `false`. A `TEXT` literal preserves case and UTF-8; enclose it in single quotes, double embedded single quotes, and use `\\`, `\n`, `\r`, and `\t` for literal backslash, newline, carriage return, and tab. Display escapes these control characters and `|`. The underlying `take_input` API hides embedded NUL and any bytes after it, so the REPL cannot validate those hidden bytes. See [COMPILER-OBS-008](../../database-implementation-observations.md#compiler-obs-008--line-input-loses-bytes-after-an-embedded-nul).

`src/value.types.op` defines typed cell values. `src/lexer.op` returns tokens or positioned lexical errors. `src/parser.op` consumes the tokens and builds a statement without modifying database state. See the [implementation observations](../../database-implementation-observations.md) for the language evaluation and blocker evidence.
