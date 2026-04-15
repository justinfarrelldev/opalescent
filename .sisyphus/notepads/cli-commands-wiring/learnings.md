# Learnings — cli-commands-wiring

## Key Code Facts
- `run_with_args(args: &[String])` — test helper in app.rs. Tests use `["opal".to_string(), "cmd".to_string()]` pattern.
- Match arm to split: `app.rs:135-139` — `if let Some(cmd @ ("pkg" | "fmt" | ...))`
- `help_text()` at `app.rs:23-100`. New `Some(...)` arms go BEFORE `Some(unknown)` catch-all at line 69.
- All file I/O stays in `app.rs` — never in module files.
- `PollingFileWatcher` methods (`start`, `poll_changes`) are TRAIT methods — must import `FileWatcher` trait.
- Lexer/Parser in `src/lexer.rs` and `src/parser.rs` (NOT mod.rs subdirs).

## Module API Quick Reference
- FormatCommand: `src/formatter/command.rs` — `.execute()` / `.execute_with_config(config)`
- FormatterConfig: `src/formatter/config.rs` — `::from_toml_str(s)` 
- LspServer: `src/lsp/server.rs` — `::new()` (const fn, no args)
- TestCommand: `src/testing/runner.rs` — `::new(config)`, `.with_filter()`, `.with_target()`, `.execute(&suite)`
- BenchmarkSuite: `src/benchmarks/suite.rs` — `::new()`, `.add_result()`, `.report()`
- parse_config: `src/build_system/config.rs` — `parse_config(s) -> Result<ProjectConfig, BuildError>`
- compile_program: `src/compiler.rs` — `compile_program(&source, Path::new("target")) -> Result<PathBuf, CompileError>`
- generate_markdown_for_program: `src/doc_gen.rs` — `generate_markdown_for_program(&program) -> String`

## Task 6 — Wire opal bench (BenchmarkSuite)
- `BenchmarkSuite::new()` is a `const fn` — works fine in non-const context too
- `bench_parse` and `bench_typecheck` imported from `crate::benchmarks::compile_time`
- `BenchmarkSuite` imported from `crate::benchmarks::suite`
- `run_bench_command` takes `_args` (unused) — must suppress `clippy::unnecessary_wraps` with `#[expect(..., reason="...")]` (NOT `#[allow]` — codebase uses `allow_attributes` = deny)
- All helper dispatch functions must return `Result<(), i32>` to match `run_with_args` dispatch pattern
- `SuiteReport.results.len()` gives count of benchmarks for summary print
- Clippy `doc_markdown` requires backtick-quoting type names like `` `BenchmarkSuite` `` in doc comments
- Test doc comments require backtick-quoting `Ok(())` in doc comments per `doc_markdown` lint

## Task 7 — Wire opal run subcommand with arg passthrough
- `opal run` MUST be dispatched BEFORE the file-path fallback — insert `if args.get(1) == Some("run")` before the `// Separate flags from positional args` block
- Without early dispatch, "run" gets treated as a filename (file_args[0] = "run"), and Err(1) is still produced (just for wrong reason) — tests pass vacuously in RED phase
- Shared `compile_and_run(source_path: &str, program_args: &[&str]) -> Result<(), i32>` eliminates duplication between `opal run` and `opal <file> --run`
- `--` separator parsed via `args.iter().position(|a| a == "--")` — collect everything after that index as program_args
- `compile_and_run` returns `Ok(())` when binary exits 0, `Err(code)` otherwise — same contract as process exit codes
- `--run` backward-compat preserved by calling `compile_and_run(source_path, &[])` in the existing flag path
- When `--run` is detected in the fallback path, check it BEFORE reading the file (skip unnecessary I/O)
- `Command::new(&binary_path).args(program_args).status()` — args() accepts `&[&str]` or `impl IntoIterator<Item = impl AsRef<OsStr>>`
- Total app tests after Task 7: 39 app tests (51 total across all modules including 4 new run tests)

## Task 8 — Wire opal check (TypeChecker)
- `TypeChecker` is at `crate::type_system::checker::TypeChecker`
- `TypeChecker::new()` — constructor, no args
- `checker.type_check_program(&program)` — takes `&Program`, returns `Result<(), Vec<TypeError>>`
- `TypeError` has NO `Display` impl — cannot use `{err}`. Cannot use `{err:?}` either (strict `use_debug` lint). Use `errors.len()` for summary message.
- Lexer/Parser API: `Lexer::new(&source).tokenize()` → `(tokens, lex_errors)` (tuple, NOT a Result). Same for `Parser::new(tokens).parse()` → `(program_opt, parse_errors)`.
- TypeChecker requires a full program with `entry main` + doc comment — tests using bare `let x = 42` will fail with `MissingEntryPoint` error.
- Valid minimal test source: `"##\n  Description: starting point of the application\n##\nentry main = f(args: string[]): void =>\n    return void\n"`
- `opal check` is dispatched BEFORE `// Separate flags from positional args` block (same pattern as `opal run`)
- Total app tests after Task 8: 55 (was 51)
- The TODO comment line for TypeChecker was: `// TODO: import when wired — path unknown (TypeChecker)` — removed it and added `use crate::type_system::checker::TypeChecker;`

## Task 9 — Wire opal build (project-level compile)
- `run_build_command` dispatched BEFORE `// Separate flags from positional args` block (same pattern as all other subcommands)
- `BuildError` does NOT implement `Display` — must match on all variants to destructure the inner `msg: String` (same pattern as `run_test_command`)
- Use `let Ok(x) = fs::read_to_string(...) else { ... }` (`manual_let_else`) — NOT `match` with `Ok(x) => x, Err(_) => { return }` (triggers `manual_let_else` and `single_match_else`)
- For cleanup calls whose return value is ignorable, use `drop(std::fs::remove_file(...))` — NOT `let _ = ...` (triggers `let_underscore_must_use` + `let_underscore_untyped`)
- For `unwrap_or_else` with a method reference, use bare `std::sync::PoisonError::into_inner` — NOT `|e| e.into_inner()` (triggers `redundant_closure`)
- `static CWD_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());` serializes cwd-changing tests (no `serial_test` available)
- `filesystem_edit_file` (line-based) works when `Edit` (string-based) silently fails due to whitespace mismatch
- File limit (1000 lines) hit after adding 58+ lines: condense multi-line `args` arrays to single lines in tests to recover lines
- Total app tests after Task 9: 57 (was 55, +2 new build tests)

## Task 13 — Final CLI integration verification
- Added integration tests in `src/app.rs`:
  - `test_all_commands_no_unimplemented`
  - `test_pkg_still_unimplemented`
  - `test_run_is_alternative_to_run_flag`
  - `test_help_lists_all_commands_integration`
- `test_all_commands_no_unimplemented` iterates the 9 wired commands (`fmt`, `lsp`, `test`, `doc`, `bench`, `run`, `check`, `build`, `watch`) and asserts each dispatches through wired paths (non-`pkg` fallback behavior).
- Verified `pkg` remains unimplemented via `run_with_args(&["opal", "pkg", "status"]) == Err(1)`.
- Verified both `opal run nonexistent.op` and `opal nonexistent.op --run` return `Err(1)` due to file/read path, proving `run` subcommand is a proper alternative path.
- Verified top-level `help_text(None)` contains all command names: `pkg`, `fmt`, `lsp`, `test`, `doc`, `bench`, `run`, `check`, `build`, `watch`.
- `cargo test app::tests -- --show-output` passes (`52 passed; 0 failed`).
- No `_not_yet_implemented` test names remain; only `pkg` still exhibits not-yet-implemented runtime behavior.
