# Small database

This project evaluates Opalescent and its native test runner by building a small relational database. The starting compiler revision is `9e7ee13822d0d3de1afcb5398f9fb10f2d1af37a`.

**Status:** The in-memory query engine and REPL implement the grammar below. Persistence is not implemented yet.

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
SELECT title, id FROM tasks WHERE done = true AND id >= 1 LIMIT 10;
UPDATE tasks SET title = 'Reviewed' WHERE id = 1;
DELETE FROM tasks WHERE id = 1;
SHOW TABLES;
DESCRIBE tasks;
HELP;
QUIT;
```

```text
query      = statement ';' EOF
statement  = 'CREATE' 'TABLE' name '(' column (',' column)* ')'
           | 'INSERT' 'INTO' name 'VALUES' '(' literal (',' literal)* ')'
           | 'SELECT' projection 'FROM' name ['WHERE' expression] ['LIMIT' nonnegative-int]
           | 'UPDATE' name 'SET' assignment (',' assignment)* ['WHERE' expression]
           | 'DELETE' 'FROM' name ['WHERE' expression]
column     = name ('INT' | 'TEXT' | 'BOOL')
projection = '*' | name (',' name)*
assignment = name '=' literal
expression = and-expr ('OR' and-expr)*
and-expr   = primary ('AND' primary)*
primary    = '(' expression ')' | name comparison literal
comparison = '=' | '!=' | '<>' | '<' | '<=' | '>' | '>='
literal    = signed-int | quoted-text | 'true' | 'false'
```

`AND` binds more tightly than `OR`. Equality and inequality work for all types; ordering is integer-only. Predicates are fully validated even for `LIMIT 0`, then evaluated left to right with short-circuiting. Limits apply after filtering. Repeated projected columns are preserved. Without `WHERE`, `UPDATE` and `DELETE` affect all rows; updates count matched rows even when the values already match. Predicate size and nesting are bounded to protect the parser and evaluator.

Names begin with an ASCII letter or underscore and may then contain ASCII digits as well. Names and keywords are case-insensitive; keywords cannot be identifiers. The parser allows whitespace, including line breaks, between tokens; the REPL reads one complete query per line. Every statement needs its semicolon, with only whitespace allowed after it. Duplicate tables and columns, unknown tables, and inserts with the wrong width or value types are rejected without publishing changes.

A signed decimal `INT` must fit in 64 bits. `BOOL` literals are `true` and `false`. A `TEXT` literal preserves case and UTF-8; enclose it in single quotes, double embedded single quotes, and use `\\`, `\n`, `\r`, and `\t` for literal backslash, newline, carriage return, and tab. Display escapes these control characters and `|`. The underlying `take_input` API hides embedded NUL and any bytes after it, so the REPL cannot validate those hidden bytes. See [COMPILER-OBS-008](../../database-implementation-observations.md#compiler-obs-008--line-input-loses-bytes-after-an-embedded-nul).

`src/value.types.op` defines typed cell values. `src/lexer.op` emits positioned tokens; `src/parser.op` builds statements without touching database state. `src/schema.op` and `src/predicate.op` resolve references and validate types, `src/database.op` applies mutations or returns structured results, and `src/repl.op` uses `src/formatting.op` for output. See the [implementation observations](../../database-implementation-observations.md) for the language evaluation and blocker evidence.
