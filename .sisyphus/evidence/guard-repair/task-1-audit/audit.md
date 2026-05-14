# Task 1 Audit — Guard terminal coverage and permissive paths

Audit timestamp: 2026-05-13T18:19:51-04:00
Repo HEAD: `6a41a6009668d03b5b3cfc852c57920a120959c7`

## A. Permissive checker helpers and line refs

### Primary permissive path in `src/type_system/checker/expressions_guard.rs`
- `src/type_system/checker/expressions_guard.rs:397-420` — `type_check_guard_error_clause_statements` splits the clause into `prelude` + `terminal`, rejects only the exact `prelude.is_empty()` + `Stmt::PropagateGuardError` shorthand case at `:407-410`, then computes `allow_void_return_terminal` at `:413-415`.
- `src/type_system/checker/expressions_guard.rs:515-549` — `type_check_guard_error_clause_terminal_statement` accepts a non-wrapper `Stmt::Return` after `type_check_stmt_with_return(...)`; if `allow_void_return_terminal` is true, the clause is accepted at `:542-543`, otherwise it emits `TypeError::GuardErrorClauseMissingTerminal` at `:545-547`.
- `src/type_system/checker/expressions_guard.rs:781-786` — `guard_clause_allows_void_terminal_return` currently returns true whenever `expected_return` is `Some([CoreType::Unit])`, making `return void` an allowed named-guard terminal in unit-returning contexts.
- `src/type_system/checker/expressions_guard.rs:788-805` — `guard_clause_prelude_has_non_alias_handling` delegates to `guard_clause_statement_counts_as_handling`; any non-`let` statement, nested block containing such a statement, or non-literal/non-identifier expression statement counts as handling. This is the permissive checker helper the repair plan calls out as too broad.
- `src/type_system/checker/expressions_guard.rs:460-513` — `type_check_guard_error_clause_prelude_statement` still permits `return void`-capable preludes to pass through ordinary statement checking; it only rejects `PropagateGuardError` not in final position (`:489-491`) and `return err` forwarding the active binding (`:493-496`).
- `src/type_system/checker/expressions_guard.rs:807-825` — `guard_clause_is_error_alias_discard` exists nearby as discard detection, but it is not the predicate used by the permissive `allow_void_return_terminal` path above.

### Summary of what will matter for repair tasks
- Helper/function locations to change or preserve exactly:
  - `type_check_guard_error_clause_statements` — `src/type_system/checker/expressions_guard.rs:390-421`
  - `type_check_guard_error_clause_terminal_statement` — `src/type_system/checker/expressions_guard.rs:515-569`
  - `guard_clause_allows_void_terminal_return` — `src/type_system/checker/expressions_guard.rs:781-786`
  - `guard_clause_prelude_has_non_alias_handling` — `src/type_system/checker/expressions_guard.rs:788-792`
  - `guard_clause_statement_counts_as_handling` — `src/type_system/checker/expressions_guard.rs:794-805`
  - nearby discard helper intentionally relevant for comparison: `guard_clause_is_error_alias_discard` — `src/type_system/checker/expressions_guard.rs:807-825`

## B. Integration harness pass-vs-fail disposition for delete-download fixtures

### Guard harness helpers in `tests/integration_e2e/guard_stmt.rs`
- `tests/integration_e2e/guard_stmt.rs:14-50` — `run_guard_stmt_project(project_name)` compiles the fixture and runs the produced binary; this is the pass harness.
- `tests/integration_e2e/guard_stmt.rs:62-83` — `compile_guard_stmt_project_failure(project_name)` expects compilation to fail; this is the compile-fail harness already used for other strict-guard negative fixtures.
- Existing negative guard fixtures already wired to compile-fail harness:
  - `guard-stmt-success-binding-leak` — `tests/integration_e2e/guard_stmt.rs:235-254`
  - `guard-stmt-only-propagate` — `tests/integration_e2e/guard_stmt.rs:257-276`
  - `guard-stmt-print-only` — `tests/integration_e2e/guard_stmt.rs:278-297`
  - `guard-stmt-ignored-alias` — `tests/integration_e2e/guard_stmt.rs:299-318`
  - `guard-stmt-return-err-banned` — `tests/integration_e2e/guard_stmt.rs:320-342`
  - wrapper-invalid cases — `tests/integration_e2e/guard_stmt.rs:370-440`

### Current delete-download disposition
- `tests/integration_e2e/guard_stmt.rs:472-492` — `delete_downloads_project_compiles_and_runs_with_strict_terminal_handlers` currently calls `run_guard_stmt_project("delete-downloads")`, so `test-projects/delete-downloads` is still treated as a compile-and-run pass fixture.
- `tests/integration_e2e/guard_stmt.rs:494-514` — `delete_downloads_strict_project_compiles_and_runs_with_strict_terminal_handlers` currently calls `run_guard_stmt_project("delete-downloads-strict")`, so `test-projects/delete-downloads-strict` is also still treated as a compile-and-run pass fixture.
- `test-projects/delete-downloads/src/main.op:9-17` — the fixture contains two named guard error clauses ending in `return void`:
  - `:9-11` list failure branch prints `LIST_ERR={err}` then `return void`
  - `:15-17` delete failure branch prints `DELETE_ERR=...` then `return void`
- `test-projects/delete-downloads-strict/src/main.op:9-17` — the strict variant currently has the same two named guard error clauses ending in `return void`.

## C. All currently passing named-guard `return void` occurrences discovered

The items below were found as currently passing test/project surfaces where a named guard error clause ends in `return void` and the surrounding harness expects successful compile/run behavior.

### Direct task blockers: delete-download fixtures
1. `test-projects/delete-downloads/src/main.op:9-11` and `:15-17`
   - Harness: `tests/integration_e2e/guard_stmt.rs:472-492`
   - Current disposition: compile and run pass via `run_guard_stmt_project("delete-downloads")`
2. `test-projects/delete-downloads-strict/src/main.op:9-11` and `:15-17`
   - Harness: `tests/integration_e2e/guard_stmt.rs:494-514`
   - Current disposition: compile and run pass via `run_guard_stmt_project("delete-downloads-strict")`

### Additional currently passing integration fixtures using the same permissive path
3. `test-projects/fs-path-manipulation/src/main.op:88-91`, `:95-98`, `:102-105`, `:109-112`, `:116-119`, `:123-126`, `:130-133`, `:137-140`
   - Each named guard clause aliases the error (`let _unused_error = err`), prints a progress line, then terminates with `return void`.
   - Harness: `tests/integration_e2e/fs_path_manipulation.rs:15-80`
   - Current disposition: compile and run pass; test expects exact stdout `passed 50/50`.
   - Also included in rerunnability suite: `tests/integration_e2e/fs_rerunnability.rs:14-35` (`"fs-path-manipulation"`).
4. `test-projects/fs-directory-operations/src/main.op:13-15`, `:17-19`, `:78-80`, `:82-84`, `:86-88`, `:90-92`, `:94-96`, `:100-102`, `:111-113`, `:115-117`, `:119-121`, `:123-125`, `:127-129`, `:131-133`, `:135-137`, `:139-141`, `:143-145`, `:147-149`
   - Named guard clauses print the error and terminate with `return void` during seed read, file creation, directory listing, and cleanup/removal paths.
   - Harness: `tests/integration_e2e/fs_directory_operations.rs:15-83`
   - Current disposition: compile and run pass; test asserts success markers in stdout.
   - Also included in rerunnability suite: `tests/integration_e2e/fs_rerunnability.rs:14-35` (`"fs-directory-operations"`).
5. `test-projects/_fs_read_text_lines/src/main.op:7-9`, `:13-15`, `:19-21`
   - Named guard clauses print the error then `return void`.
   - Harness: `tests/integration_e2e/fs_read_text_lines.rs:15-89`
   - Current disposition: compile and run pass; test expects `lines=4`, `first=alpha`, and `match=true`.
6. `test-projects/fs-markdown-roundtrip/src/main.op:27-29`, `:41-43`, `:45-47`, `:49-51`
   - Named guard clauses print the error then `return void`.
   - Harness: `tests/integration_e2e/fs_markdown_roundtrip.rs:12-95`
   - Current disposition: compile and run pass; test expects exact success stdout and output-file match.
   - Also included in rerunnability suite: `tests/integration_e2e/fs_rerunnability.rs:14-35` (`"fs-markdown-roundtrip"`).
7. `test-projects/_absolute_path_sync/src/main.op:8-10`, `:18-20`, `:28-30`, `:38-40`
   - Named guard clauses print a deterministic `<input>: error` marker then `return void`.
   - Harness: `tests/integration_e2e/fs_absolute_path_sync.rs:181-275`
   - Current disposition: compile and run pass; test asserts four printed lines and exit success.
   - Also included in rerunnability suite: `tests/integration_e2e/fs_rerunnability.rs:14-35` (`"_absolute_path_sync"`).
8. `test-projects/_fs_append_log/src/main.op:15-17`, `:19-21`
   - Named guard clauses print the error then `return void`.
   - Harness: `tests/integration_e2e/fs_append_log.rs:35-107`
   - Current disposition: compile and run pass; test expects `appended 5 lines; readback confirmed`.

### Not currently discovered as passing named-guard `return void` unit tests in `src/type_system/tests.rs`
- The positive named-guard unit tests I inspected (`src/type_system/tests.rs:2198-2299`) use final `propagate err`, not `return void`.
- The `return void` occurrence at `src/type_system/tests.rs:2179-2182` is part of a negative scope test (`test_guard_statement_success_binding_is_hidden_inside_else_clause`) and is not a currently passing semantic acceptance case for strict named-guard terminals.

## Notes
- The task-required concrete references were inspected directly:
  - `src/type_system/checker/expressions_guard.rs`
  - `tests/integration_e2e/guard_stmt.rs`
  - `test-projects/delete-downloads/src/main.op`
  - `test-projects/delete-downloads-strict/src/main.op`
- The current permissive behavior is broader than the two delete-download fixtures; several filesystem integration fixtures also rely on the same `return void` escape hatch for named guard error clauses.
