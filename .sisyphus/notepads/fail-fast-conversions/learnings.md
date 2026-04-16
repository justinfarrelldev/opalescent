# Learnings

## [2026-04-16] Session start baseline
- `cargo test`: 949 passed, 4 failed, 5 ignored
- 4 failing tests are all caused by invalid guard syntax in `language-spec/error_handling_samples.op`
- Failing tests:
  - `parser::tests::test_error_handling_sample_parses_successfully`
  - `parser::tests::test_error_handling_sample_contains_guard_and_propagate`
  - `type_system::test_integration_ecosystem::ecosystem_tests::test_error_handling_samples_spec_file_parses`
  - `type_system::tests::test_type_check_error_handling_sample_program`

## Key Code Locations
- `runtime/opal_runtime.c:119-181` — current `string_to_*` functions (return raw values, silent 0 on failure)
- `src/codegen/functions.rs:535-542` — `ptr_to_int_fn!` macro for parse function declarations
- `src/type_system/checker.rs:218-237` — direct registration of `string_to_int32` with `error_types: Vec::new()`
- `src/type_system/checker/size_specific_builtins.rs:15-20` — registers int8/16/uint8/16/32/64 (NOT int32/int64)
- `src/type_system/module_resolver.rs:315-334` — `standard` module exports string_to_int32 (incorrectly returns Int64!) and string_to_int64, both with `error_types: Vec::new()`
- `src/type_system/environment.rs:53-59` — ParseError already registered as built-in type

## Guard Syntax
- Statement form: `guard <expr> into <binding> else <err_binding> =>` followed by indented body
- Expression form: `guard <expr> into <binding> else { block }` (parser supports but language design says invalid)
- `parse_guard_statement()` in `src/parser/statements.rs:671+`
- `parse_guard_expression()` in `src/parser/expressions.rs:208-300`

## CLI
- `cargo run -- check <file.op>` outputs "error: type check failed with N error(s)" — does NOT print specific error details
- `run_check_command` in `src/app.rs:454-488`

## [2026-04-16] Task 2 runtime parse refactor
- `runtime/opal_runtime.c` now defines 10 `ParseResult*` structs directly after includes and adds `<errno.h>`, `<limits.h>`, and `<float.h>` for robust range checks.
- All `string_to_*` integer parse functions now return `{ value, error }` with fail-fast errors (`null input`, `empty input`, invalid digit, overflow) and allow trailing whitespace only.
- Added `string_to_float32`/`string_to_float64` returning `ParseResultF32`/`ParseResultF64` using `strtof`/`strtod` with `ERANGE` overflow detection.
- Compile evidence captured at `.sisyphus/evidence/task-2-compile.txt`; `gcc -Wall -Wextra` completed cleanly and struct typedef count is 10.
