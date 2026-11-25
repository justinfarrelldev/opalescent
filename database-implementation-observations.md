# Small database implementation observations

Starting revision: `9e7ee13822d0d3de1afcb5398f9fb10f2d1af37a`. Initial verification uses the freshly built Linux host compiler (`cargo build --locked --offline --bin opalescent`, LLVM 14). The project treats the compiler, runtime, standard library, and test runner as fixed dependencies.

## DB-OBS-001 — Native tests exercise the real executable and filesystem boundaries

- **Feature / classification / severity:** native Worlds, terminal driver, and typed doubles; positive; informational.
- **Expected:** isolated tests can exercise the real application and substitute just the failing filesystem operation when needed.
- **Actual:** the terminal editor's six native cases passed, including its POSIX PTY session and its two injected `FilesystemFullError` cases. The tests use a dependency World for temporary directories, explicit cleanup, one-argument launch, polling observations, and a transcript attachment. The test-only API is distinct from the application API.
- **Evidence:** run `./target/debug/opalescent test --root test-projects/terminal-simple-editor`; see `test-projects/terminal-simple-editor/tests/{editor,terminal}.test.op`, `docs/native-testing-terminal.md`, and `docs/native-testing-workspaces.md`. `native-testing-products` and `native-testing-filesystem-double` additionally contain intentionally failing cases that test failure reporting; they are not standalone all-green application suites.
- **Impact / alternative:** reuse the editor's patterns rather than inventing a harness or changing the test runner; no compromise to correctness. Launch must always supply one argument, and unsupported hosts cannot use the PTY driver.
- **Suggested improvement / status:** keep those constraints documented; supported on the tested Linux host.

## Investigation notes

- Reviewed the language specification, tutorial, standard-library reference, native testing docs, terminal editor, and Game of Life. Cross-file products, tag checks with `is … into`, string ownership, and fallible access have working fixture patterns. Typed payload assertions should check the tag and compare the extracted fields rather than require whole-payload equality.
- For persistence, the public atomic text operation lowers to `write_text_atomic_sync` (`src/codegen/functions_stdlib.rs`). The runtime writes and closes a same-directory temporary file before replacing the destination (`runtime/opal_fs.c`, `write_contents_atomic_bytes_sync`). It does not explicitly flush to stable storage. Persistence will not claim power-loss durability.
- `take_input` uses C-string operations after reading a line (`runtime/opal_io.c`); inspect the implications for input-boundary validation before choosing a REPL input API. `read_text_sync` validates UTF-8 over the byte count but internally returns a C string, so snapshot loading must not assume it can detect NUL through string indexing alone; `read_contents_sync` exposes the raw data.
