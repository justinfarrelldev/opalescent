# Small Database

Small Database is a local, interactive database for modest datasets. Use it to keep a personal task list or inventory, try out a table schema, or learn how filtering and updates work. It runs in one terminal with no server or external database dependency.

| Good fit | Trade-off |
| --- | --- |
| Small collections of integers, UTF-8 text, and booleans | Everything lives in memory. Queries scan rows without indexes, and large batches of inserts can be slow. |
| Typing SQL-style queries and seeing results immediately | This is a SQL subset: no joins, aggregation, sorting, `NULL`, keys, or multi-statement transactions. |
| Keeping a small, readable snapshot you can inspect or copy | Saving is explicit and rewrites the whole file. There is no autosave, file locking, or guaranteed power-loss recovery. |

Use it for local experiments and data you can back up or recreate. It is not designed for production workloads, critical records, or databases shared by multiple processes.

## Quick start

From the repository root, with `opal` available in your shell:

```sh
cd test-projects/small-database
opal build
./target/program tasks.odb
```

`tasks.odb` is the file to load at startup and replace when you save. It does not need to exist yet. To experiment without saving a file, leave off the filename: `./target/program`.

Enter one statement per line, including its semicolon:

```text
db> CREATE TABLE tasks (id INT, title TEXT, done BOOL);
Table 'tasks' created.
db*> INSERT INTO tasks VALUES (1, 'Review snapshot', false);
1 row inserted.
db*> SELECT title FROM tasks WHERE done = false LIMIT 5;
title
Review snapshot
1 rows returned.
db*> SAVE;
Database saved.
db> QUIT;
Goodbye.
```

Run `SHOW TABLES;` to list tables, `DESCRIBE tasks;` to see a table's columns and types, and `HELP;` for the command list. Run the program again with the same filename to check your saved data.

## Command reference

These are all supported commands. Keywords are case-insensitive, and every command ends with `;`. Brackets in the signatures below denote optional syntax; don't type the brackets.

| Command | What it does |
| --- | --- |
| `CREATE TABLE name (column type, ...);` | Creates an empty table with one or more uniquely named columns. |
| `INSERT INTO name VALUES (literal, ...);` | Appends one row; give exactly one value per column in schema order. Duplicate rows are allowed. |
| `SELECT projection FROM name [WHERE expression] [LIMIT n];` | Reads rows; `projection` is `*` or a comma-separated list of column names. |
| `UPDATE name SET column = literal [, ...] [WHERE expression];` | Replaces the given columns of matching rows. Each column can be assigned only once. |
| `DELETE FROM name [WHERE expression];` | Removes matching rows without removing the table. |
| `SHOW TABLES;` | Lists tables in creation order. |
| `DESCRIBE name;` | Lists a table's columns and types in schema order. |
| `SAVE;` | Replaces the configured snapshot with the active data. |
| `LOAD;` | Replaces the active data with the configured snapshot if it validates. |
| `HELP;` | Shows the command list and persistence behavior. |
| `QUIT;` | Exits without saving. |

```sql
SELECT * FROM tasks;
SELECT title, id FROM tasks WHERE done = false AND id >= 1 LIMIT 10;
UPDATE tasks SET done = true, title = 'Reviewed' WHERE id = 1;
DELETE FROM tasks WHERE id = 1;
```

### Values and expressions

| Type | Accepted literals |
| --- | --- |
| `INT` | Decimal digits with an optional leading `-`. Range: `-9223372036854775808` through `9223372036854775807`. |
| `TEXT` | Single-quoted UTF-8 text, including `''`. Text keeps its case. Double an embedded quote (`'It''s working'`). The only backslash escapes are `\\` (backslash), `\n` (newline), `\r` (carriage return), and `\t` (tab). |
| `BOOL` | `true` or `false`, case-insensitive. |

Values never convert between types: `42`, `'42'`, and `true` are distinct. There is no `NULL` or floating-point type. A limit must be an integer value from 0 through `9223372036854775807`.

- **Names:** ASCII letters, digits, and underscores, starting with a letter or underscore (`[A-Za-z_][A-Za-z0-9_]*`). Names are case-insensitive. Reserved: `AND`, `BOOL`, `CREATE`, `DELETE`, `DESCRIBE`, `FALSE`, `FROM`, `HELP`, `INSERT`, `INT`, `INTO`, `LIMIT`, `LOAD`, `OR`, `QUIT`, `SAVE`, `SELECT`, `SET`, `SHOW`, `TABLE`, `TABLES`, `TEXT`, `TRUE`, `UPDATE`, `VALUES`, `WHERE`.
- **Filtering:** All types support `=`, `!=`, and `<>`. Only integers support `<`, `<=`, `>`, and `>=`. `AND` binds more tightly than `OR`; use parentheses to change the grouping. Predicates are validated even for `LIMIT 0` and are evaluated with short-circuiting. The parser bounds predicate size and nesting.
- **Results:** `SELECT` retains insertion order and the requested column order, even if you name a column twice. `LIMIT` is applied after filtering; `LIMIT 0` returns just the column headings and row count.
- **Changes:** Without `WHERE`, `UPDATE` and `DELETE` affect every row in the table. An update counts matching rows even if the assigned value was already present. Invalid statements report an error and do not partially change data.

### Grammar

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
command    = 'SHOW' 'TABLES' ';' EOF
           | 'DESCRIBE' name ';' EOF
           | ('SAVE' | 'LOAD' | 'HELP' | 'QUIT') ';' EOF
```

The engine parser accepts whitespace and line breaks between tokens. The REPL reads one complete statement per line. Text output escapes line breaks, tabs, backslashes, and `|` so one value cannot introduce another table row or column. The line-input API hides embedded NUL and any bytes after it; don't feed the REPL binary input.

## Saving and loading

- `db*>` means there are unsaved changes. Read-only queries, failed mutations, no-op updates, and deletes that find no rows do not make a clean database dirty.
- `SAVE;` writes a complete snapshot to the configured file. `LOAD;` replaces the active data **and discards unsaved changes** if the snapshot is valid. Failed saves or loads leave the active data and dirty flag unchanged.
- `QUIT;` and end of input never save automatically. If you configured a file, the REPL warns when exiting with unsaved changes.
- On startup, a missing file means a new empty database. An existing corrupt or unreadable file is an error; the program does not silently start empty or overwrite the file. Without a filename, `SAVE;` and `LOAD;` report that there is no destination.

Saving writes and closes a temporary file in the destination directory before atomically replacing the destination. It does not truncate the old file first, but it does not explicitly flush data to stable storage. Keep backups and don't read or modify the snapshot concurrently with this database. Loading uses separate raw-byte and UTF-8 reads because there is no direct conversion from `Bytes` to text.

### Snapshot format

```text
OPALDB 1
CREATE TABLE tasks (id INT, title TEXT, done BOOL);
INSERT INTO tasks VALUES (1, 'Review snapshot', false);
END 1 1
```

A canonical version 1 snapshot is UTF-8, with LF separators and a final LF. The header is exactly `OPALDB 1`; the trailer is `END <table-count> <total-row-count>` with single spaces and decimal counts. Between them are one `CREATE TABLE` record per table, in creation order, and then that table's `INSERT INTO` records in row order. Each record is one terminated statement. Column order and typed literals use the query grammar and escaping above; escaped line breaks are not record boundaries. The decoder also accepts CRLF, bare CR, and an absent final line terminator. It rejects other statement types, invalid schemas or rows, unknown versions, incorrect counts, trailing records, invalid UTF-8, and NUL bytes in files. Counts detect some framing errors; they are not a checksum.

## Development and tests

The project is also a substantial Opalescent language and native testing fixture. It started at compiler revision `9e7ee13822d0d3de1afcb5398f9fb10f2d1af37a`. See the [implementation observations](../../database-implementation-observations.md) for the language evaluation, ownership regressions, and workload measurements.

From the repository root:

```sh
opal test --root test-projects/small-database
opal test --root test-projects/small-database/workload --timeout 180s
```

The regular native suite includes real production-PTY tests, verified on Linux. The separate 1,000- and 10,000-row workloads use the engine directly, save and reload actual files, and require a longer timeout on the measured host.

The pipeline is `lexer.op` → `parser.op` → schema and predicate validation → `database.op`. The executor consumes normalized names and typed values and returns structured results without I/O. `repl.op` uses `formatting.op` for display; `persistence.op` encodes, validates, and stores snapshots independently of the terminal.
