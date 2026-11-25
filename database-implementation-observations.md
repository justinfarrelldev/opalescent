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

## DB-OBS-005 — Read-only parsing can borrow the token stream

- **Feature / classification / severity:** owned vs. `ref` array parameters in native code; compiler/runtime ownership defect observed; moderate in isolation.
- **Expected:** lexical token data containing structured fields can be passed to a parser without invalidating live storage.
- **Actual:** the first `parse_create(tokens: Token[])` implementation crashed the native worker in `opal_rc_dec`/`opal_rc_drop_iterative` with `free(): double free detected in tcache 2`. An ordinary build of the malformed-type case printed the expected diagnostic. Changing the parameter and call to `ref tokens` eliminated the native crash and kept the parser's four behavior tests green.
- **Evidence:** `test-projects/small-database/src/parser.op`, `tests/parser.test.op` in that project, and `language-spec/requirements/memory-model.md`, §§2–3. The only semantic change required at that call edge was borrowing the same tokens.
- **Impact / alternative:** the parser needs only read access, so the borrow is idiomatic and does not harm correctness or runtime cost. It does not solve ownership when the database must retain a value after its producer returns.
- **Suggested improvement / status:** regression-test owned transfers as well as read-only borrows for arrays of products containing sum values. Non-blocking for the parser.

## BLOCKER DB-001 — Returned records can lose ownership of their arrays

- **Discovered:** 2026-11-25, during the Milestone 1 `CREATE TABLE` state-integrity increment at starting compiler revision `9e7ee13822d0d3de1afcb5398f9fb10f2d1af37a` (Linux/LLVM 14).
- **Affected requirement:** §§9.2–9.4, 10.4, 11.3–11.4. A database must hold independent schemas and rows across calls, publish only validated mutations, and preserve earlier values. Insertion, selection, mutation, and eventually snapshot loading depend on that guarantee.
- **Classification / severity:** confirmed compiler/runtime ownership defect; blocking. The isolated failure occurs in normal executables as well as native workers. Exact root cause is not confirmed.
- **Expected:** a returned record still owns its empty array. Allocating a separate one-element array must not make the first array nonempty. A valid `CREATE TABLE` must leave an accessible table in the database for subsequent queries.
- **Actual:** the minimal ordinary executable printed `stored=1, other=1` and exited 0 (expected `stored=0, other=1`). The native counterpart failed with `equality failed: actual (int64) 1; expected (int64) 0`. In the database prototype, one caller-owned replacement-state route segfaulted in `execute_create` (exit 139); attempts using field publication also produced native worker crashes, including `free(): double free detected in tcache 2` in `opal_rc_drop_iterative`. Ordinary codegen rejects the direct field-publication route with `assignment target must be an identifier or identifier-backed index expression`.
- **Minimal evidence:** [`repros/array_ownership`](test-projects/small-database/repros/array_ownership) is a separate Opalescent project. `src/holder.op` returns `new ArrayHolder: items: items` for an empty local `int64[]`. The caller allocates an unrelated `[7]` and reads the returned field. To reproduce from the repository root:

  ```sh
  ./target/debug/opalescent test --root test-projects/small-database/repros/array_ownership --filter 'stay empty'
  (cd test-projects/small-database/repros/array_ownership && ../../../../target/debug/opalescent run)
  ```

  A broader native probe returned three items after two inserts (reproduced three times). Control cases in `repros/array_ownership/tests/ownership.test.op` passed when mutating a simple store built directly at the call site or using a standalone owned array. The production `empty_holder` disassembly shows allocation of the array, allocation of the enclosing record, then `opal_rc_dec` before returning from the factory, with no intervening `opal_rc_inc` in that function. This is consistent with premature release, but does not identify the responsible compiler analysis pass.
- **Relevant source and contract:** `language-spec/requirements/memory-model.md`, §§3 and 8; `src/codegen/adts.rs` (`codegen_product_constructor` emits a field-boundary retain for `string`, not for array fields at that site); `src/codegen/statements.rs` (assignment targets); `src/codegen/functions_call/array/intrinsics.rs` (`codegen_array_push_call`); `runtime/opal_rc.c`. Investigate ownership analysis and emission together before selecting a fix; adding a retain indiscriminately could leak newly allocated fields.
- **Impact:** malformed input isolation and even publication of valid table definitions cannot be guaranteed. Silent data corruption is possible, not just an obvious crash. The implementation has stopped before adding dependent query operations or starting persistence.

### Resolution options

| Option | Scope, tradeoffs, and requirements |
| --- | --- |
| Approve a narrowly scoped ownership investigation and fix | Medium effort. Start with product construction/return of array fields, pin down the missing retain or invalid transfer, and add separate production and native regression tests. Verify ordinary and native ownership paths; the fix touches shared code generation, so compiler/RC regressions must pass. This preserves the original requirements. |
| Approve a database-only representation investigation | Medium to high effort. A standalone array passed and returned safely in an isolated control. Try carrying table/state data as top-level arrays rather than through returned products. This is **not** a verified solution for schema/row/result lifetimes. It would require a broader redesign and fresh tests; continue only if a small slice proves it can preserve all requirements without fragile manual copying or parallel-array invariants. |
| Defer database mutations and persistence | Low immediate effort. Retains the working front end and reproductions but does not satisfy either milestone. Requires an explicit scope decision. |

**Recommendation:** Approve the narrowly scoped compiler ownership investigation and fix. Start from the primitive `int64[]` reproduction, so a patch does not depend on the database's particular data model. Treat any distinct defect uncovered by that work as a separate escalation.

**Resolution:** The project owner explicitly approved this compiler investigation and fix, plus separate regression tests. Nominal product construction now gives the product its own reference to a borrowed array; an owned temporary is transferred rather than retained again. A provisional field scope releases array references if a later initializer propagates an error or aborts the native case. The production regression and native fixture in `test-projects/record-array-ownership/` pass; both reproduced the wrong length before the fix. A codegen check also verifies that constructing a record from a fresh array literal does not emit an extra retain.

## BLOCKER DB-002 — Leaving scope after reading an array field can invalidate the record

- **Discovered:** 2026-11-25 while rechecking `CREATE TABLE` after the approved DB-001 fix, on `06bf49c` plus the DB-001 compiler patch (Linux/LLVM 14). The owner was notified immediately after the isolated production and native failures were confirmed. No change has been authorized for DB-002.
- **Affected requirement:** §§9.2–9.4, 10.4, 11.3–11.4. Core operations must be able to read and retain tables, schemas, and rows without destroying the existing state.
- **Classification:** confirmed compiler ownership defect, blocking. This is on the field-read/binding side; the record can be constructed directly in the caller with a fresh literal, so the DB-001 factory bug is not involved.
- **Expected and actual:** `let alias: int64[] = holder.items` should not make `holder.items` lose ownership when `alias` leaves scope. The native check instead reports `actual (int64) 1; expected (int64) 0` after an unrelated one-element array allocation. The ordinary probe prints `stored after alias scope exit=1, independent=1`; the expected stored length is `0`. The DB-001 probe in the same binary now correctly prints `stored=0, other=1`.
- **Reproduction:** use the DB-001 commands above after applying the DB-001 patch, substituting `--filter 'extracted binding'` for the native command. The isolated case does this:

  ```opal
  let holder: ArrayHolder = new ArrayHolder:
      items: []
  if true:
      let alias: int64[] = holder.items
      print(alias.length)
  let other: int64[] = [7]
  print(holder.items.length)
  print(other.length)
  ```

- **Likely source:** `src/codegen/statements.rs` (`codegen_let_statement`) retains borrowed member initializers for `string` but retains only identifier initializers for other managed bindings. Member lowering itself loads the field without making an owned copy (`src/codegen/adts.rs`); lexical cleanup can therefore release the record's only array reference. This evidence does not yet establish that the same defect explains every failure in the larger prototype.
- **Impact:** one native `CREATE TABLE` mutation case reports the wrong column count and the other crashes; the ordinary prototype still segfaults. Field reads are pervasive in the intended design. Direct length observation without making an owned binding works in the DB-001 regression but does not substitute for owning schema or row arrays across calls.
- **Options:** (1) approve a focused ownership fix for assigning an array field to a new `let` binding, with regressions proving correct alias lifetime and no extra retain for owned expressions (small to medium effort; shared lowering needs regression coverage; preserves requirements); (2) investigate a database-only design that avoids owned field reads (higher effort; not yet validated, and risks unmaintainable storage conventions); (3) defer nested state operations (low effort; does not meet the milestones).
- **Recommendation / status:** approve option 1 as a separate patch and recheck the database slice after it lands. Blocking — awaiting decision. No DB-002 compiler change has been made.

## Verification at the pause

- Completed increments: `caaa6d0` (initial project and observations), `be9fe3f` (scalar values), `ba1b54a` (basic tokenizer), and `06bf49c` (`CREATE TABLE` parser). The blocker report, reproductions, and in-progress executor are left in the working tree; the failing executor tests were not committed.
- The front-end native cases pass: 4 scalar-value cases, 3 lexer cases, and 4 parser cases. Running the full current worktree suite adds two `CREATE TABLE` mutation cases; both crash. The minimal reproduction is intentionally failing and is discovered only when its own project root is selected.
- The completed commits ran the unchanged pre-commit hook. The latest completed verification reported 1,841 passing library tests (8 ignored), 257 passing E2E regression tests (3 ignored), and 54 passing VS Code extension tests. The reference editor's separate native run passed all six cases, including its terminal test. No database REPL, persistence, performance, or cross-platform verification has been performed.

## Investigation notes

- Reviewed the language specification, tutorial, standard-library reference, native testing docs, terminal editor, and Game of Life. Cross-file products, tag checks with `is … into`, string ownership, and fallible access have working fixture patterns. Typed payload assertions should check the tag and compare the extracted fields rather than require whole-payload equality.
- For persistence, the public atomic text operation lowers to `write_text_atomic_sync` (`src/codegen/functions_stdlib.rs`). The runtime writes and closes a same-directory temporary file before replacing the destination (`runtime/opal_fs.c`, `write_contents_atomic_bytes_sync`). It does not explicitly flush to stable storage. Persistence will not claim power-loss durability.
- `take_input` uses C-string operations after reading a line (`runtime/opal_io.c`); inspect the implications for input-boundary validation before choosing a REPL input API. `read_text_sync` validates UTF-8 over the byte count but internally returns a C string, so snapshot loading must not assume it can detect NUL through string indexing alone; `read_contents_sync` exposes the raw data.
