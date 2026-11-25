# Small database implementation observations

Starting revision: `9e7ee13822d0d3de1afcb5398f9fb10f2d1af37a`. Initial verification uses the freshly built Linux host compiler (`cargo build --locked --offline --bin opalescent`, LLVM 14). The project treats the compiler, runtime, standard library, and test runner as fixed dependencies.

## DB-OBS-001 — Native tests exercise the real executable and filesystem boundaries

- **Feature / classification / severity:** native Worlds, terminal driver, and typed doubles; positive; informational.
- **Expected:** isolated tests can exercise the real application and substitute just the failing filesystem operation when needed.
- **Actual:** the terminal editor's six native cases passed, including its POSIX PTY session and its two injected `FilesystemFullError` cases. The tests use a dependency World for temporary directories, explicit cleanup, one-argument launch, polling observations, and a transcript attachment. The test-only API is distinct from the application API.
- **Evidence:** run `./target/debug/opalescent test --root test-projects/terminal-simple-editor`; see `test-projects/terminal-simple-editor/tests/{editor,terminal}.test.op`, `docs/native-testing-terminal.md`, and `docs/native-testing-workspaces.md`. `native-testing-products` and `native-testing-filesystem-double` additionally contain intentionally failing cases that test failure reporting; they are not standalone all-green application suites.
- **Impact / alternative:** reuse the editor's patterns rather than inventing a harness or changing the test runner; no compromise to correctness. Launch must always supply one argument, and unsupported hosts cannot use the PTY driver.
- **Suggested improvement / status:** keep those constraints documented; supported on the tested Linux host.

## DB-OBS-002 — Explicit bindings stabilize code generation for compound values

- **Feature / classification / severity:** fallible functions, variant refinement, local inference, and native builds; compiler friction; moderate.
- **Expected:** accepted inferred code and an equivalent explicitly typed local compile and behave alike. Native parse failures should identify the failing syntax.
- **Actual:** directly returning a constructed sum from the fallible tokenizer scaffold failed native codegen with `native completed result does not match the original application ABI`. Binding it to `let result: LexResult` and returning the local compiled. For test bindings, inferred `let tokens = success.tokens` produced `unknown field 'length' on receiver expression`; inferred `let first = propagate tokens.at(0)` then produced `unknown field 'kind'`. Explicit `Token[]` and `Token` locals resolved both. In the scanner, `let scalar = propagate source.at(index)` compiled but did not advance over an underscore after being refined by a string equality test. The same statement with `let scalar: string` passed both native lexer tests.
- **Intentional restriction vs. defect:** `if first.kind is TokenKind.Identifier into identifier:` is intentionally rejected (see `test_terminal_proposal_refinement_and_using_rejections` in `src/parser/tests.rs`). A simple local before `is … into` is the supported alternative. The test CLI only reported the file path for this parse error; `NativeLoadError::Frontend` in `src/testing/native/project.rs` carries a source report but formats this generic message.
- **Impact / alternative:** typed locals and separate refinement bindings preserve the data model and behavior without changing dependencies. Extra declarations add some boilerplate.
- **Suggested improvement / status:** unify typed and inferred lowering for these shapes and surface test frontend spans. Non-blocking with the listed alternatives.

## DB-OBS-003 — Native string module values fault, but literal-returning functions work

- **Feature / classification / severity:** managed module-level string values; confirmed native-build defect; moderate.
- **Expected:** `let lowercase = 'abcdefghijklmnopqrstuvwxyz'` can be read repeatedly from functions in the same module, including in native tests.
- **Actual:** the tokenizer worker crashed. The native worker backtrace showed `opal_rc_inc` called by the generated module-value accessor for `lowercase` before `is_identifier_start` could return.
- **Impact / alternative:** replace module-level alphabet values with small literal-returning functions (`lowercase`, `uppercase`, `digits` in `test-projects/small-database/src/lexer.op`). This preserves correctness and retains meaningful tests. No performance claim has been made about the extra function calls.
- **Suggested improvement / status:** verify ownership handling of static string initializers in native module accessors; non-blocking with the above alternative.

## DB-OBS-004 — String builder lifetime is process-wide

- **Feature / classification / severity:** string builder resource management; runtime design observation; informational.
- **Expected:** short-lived token assembly should not accumulate per-token resources in the REPL.
- **Actual:** `runtime/opal_string.c` registers each builder for cleanup at process exit and has no public disposal API after `string_builder_finish`. The builder buffer is transferred on finish, but a builder and registry node remain.
- **Impact / alternative:** use an owned per-token scalar array and `string_join` for identifiers. This avoids keeping completed builder objects alive across queries, at the price of a transient array of pieces. Native regression cases pass. Neither approach addresses repeated scalar `.at` scans, which should be measured on larger inputs; `runtime/opal_string.c` finds indexed scalars starting from the beginning of the string.
- **Suggested improvement / status:** consider an explicit builder lifetime and a forward-only string cursor; non-blocking for this tokenizer increment.

## Investigation notes

- Reviewed the language specification, tutorial, standard-library reference, native testing docs, terminal editor, and Game of Life. Cross-file products, tag checks with `is … into`, string ownership, and fallible access have working fixture patterns. Typed payload assertions should check the tag and compare the extracted fields rather than require whole-payload equality.
- For persistence, the public atomic text operation lowers to `write_text_atomic_sync` (`src/codegen/functions_stdlib.rs`). The runtime writes and closes a same-directory temporary file before replacing the destination (`runtime/opal_fs.c`, `write_contents_atomic_bytes_sync`). It does not explicitly flush to stable storage. Persistence will not claim power-loss durability.
- `take_input` uses C-string operations after reading a line (`runtime/opal_io.c`); inspect the implications for input-boundary validation before choosing a REPL input API. `read_text_sync` validates UTF-8 over the byte count but internally returns a C string, so snapshot loading must not assume it can detect NUL through string indexing alone; `read_contents_sync` exposes the raw data.
