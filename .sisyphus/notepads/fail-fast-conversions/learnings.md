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

## [2026-04-16] T4: ParseError registration verified
- ParseError is registered in `src/type_system/environment.rs:53-59` as `CoreType::Generic { name: "ParseError", type_args: Vec::new() }`
- All builtin functions use `error_types: Vec::new()` pattern (confirmed in size_specific_builtins.rs lines 50, 72, 94)
- `string_to_int32` in checker.rs:218-237 uses same `error_types: Vec::new()` pattern
- **COMPATIBILITY CONFIRMED**: The Generic variant with empty type_args is the correct representation for error_types vectors
- T7 can safely populate error_types with `vec![CoreType::Generic { name: "ParseError", type_args: Vec::new() }]`
- Evidence saved to `.sisyphus/evidence/task-4-parse-error.txt`

## [2026-04-16] T5: LLVM parse declarations switched to struct returns
- `src/codegen/functions.rs` now declares all 10 `string_to_*` parse functions with LLVM struct returns using `context.struct_type(&[value_ty, i8_ptr], false)`.
- Added parse result struct LLVM types for widths/float families: `{ i8, ptr }`, `{ i16, ptr }`, `{ i32, ptr }`, `{ i64, ptr }`, `{ float, ptr }`, `{ double, ptr }`; unsigned parse calls reuse signed-width struct layouts.
- Removed `ptr_to_int_fn!` usage for parse declarations and replaced with explicit per-function declarations; also added `string_to_float32`/`string_to_float64` stdlib declaration/import resolution support.
- Updated `known_runtime_return_type` in `src/codegen/statements.rs` so parse functions map to parse-result `CoreType::Generic` markers (`ParseResultI8`/`...`/`ParseResultF64`) rather than raw numeric scalars.
- Verification passed: `cargo build` succeeded and evidence was written to `.sisyphus/evidence/task-5-build.txt`; LSP diagnostics are clean for changed files.

## [2026-04-16] T6: guard/propgate parse-result pointer branching
-  now handles full  for struct-return parse results: extracts field 1 as error ptr, checks  for success, branches to /, and rejoins at .
- Success path now binds extracted field 0 value to success binding; else path binds extracted error pointer to  as string and lowers else body before merge.
- Guard fallback for non-struct/non-pointer-error values remains intact to preserve backward compatibility.
-  now treats struct field 1 as pointer error ( => early return), while retaining int-flag fallback for non-pointer legacy layouts.
- Verification: 
running 1 test
test codegen::tests::test_guard_statement_compiles_to_valid_llvm_ir ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 957 filtered out; finished in 0.00s


running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s passed;  passed; build evidence logged at .

## [2026-04-16] T6 correction: guard/propagate pointer-error flow
- `src/codegen/statements.rs::codegen_guard_statement` now lowers statement-guard control flow for struct parse results: extract error field (index 1), compare against null, branch to success/else blocks, and merge.
- Success path extracts value field (index 0) and stores into success binding allocation.
- Else path binds extracted error pointer to `error_binding` as `CoreType::String`, emits else body, and restores any shadowed binding.
- Guard retains fallback behavior for non-struct/non-pointer layouts to avoid breaking non-parse guards.
- `src/codegen/functions.rs::codegen_propagate_expression` now treats struct field 1 as pointer error (`is_not_null` => early return); legacy int-flag handling remains as fallback.
- Verification rerun: target test `codegen::tests::test_guard_statement_compiles_to_valid_llvm_ir` passed; `cargo build` passed; evidence file `.sisyphus/evidence/task-6-guard-ir.txt` contains successful build output.

## T7: Parse Builtins with ParseError Registration

**Completed:** All 5 mechanical edits applied successfully.

**Changes:**
1. `register_string_to_int` helper now includes `ParseError` in error_types
2. Added `string_to_float32` and `string_to_float64` registrations (reuse same helper)
3. Added new `register_to_string` helper for infallible T→string conversions
4. Registered all 11 *_to_string variants (int8-64, uint8-64, float32-64, bool)
5. Fixed `string_to_int32` return type from Int64 → Int32 in module_resolver
6. Added `ParseError` to both `string_to_int32` and `string_to_int64` in module_resolver

**Test Results:** 950 tests pass (up from 949), 3 expected codegen failures remain.

**Key Pattern:** Parse functions (string→T) are fallible with ParseError; conversion functions (T→string) are infallible.

## [2026-04-16] T8: Fixed 3 failing codegen tests

**Task:** Fix 3 failing codegen tests that used bare calls to `string_to_int32`/`string_to_int64` without guard/propagate syntax.

**Changes Made:**
1. `test_import_string_to_int64_emits_correct_declaration` (line 959-985):
   - Changed bare call `let n = string_to_int64('42')` to guard syntax: `guard string_to_int64('42') into n else _e => return void`
   - Updated IR assertion from `"declare i64 @string_to_int64(i8*)"` to `"@string_to_int64"` (signature changed)

2. `test_import_standard_multiple_symbols_emit_all_runtime_declarations` (line 988-1019):
   - Changed bare call `let value = string_to_int32(text)` to guard syntax: `guard string_to_int32(text) into value else _e => return void`
   - Updated IR assertion from `"declare i32 @string_to_int32(i8*)"` to `"@string_to_int32"`

3. `test_builtin_calls_emit_runtime_declarations_without_imports` (line 1050-1084):
   - Changed bare call `let parsed = string_to_int32(raw)` to guard syntax: `guard string_to_int32(raw) into parsed else _e => return void`
   - Updated IR assertion from `"declare i32 @string_to_int32(i8*)"` to `"@string_to_int32"`

**Verification:**
- All 3 tests now pass individually
- All 47 codegen tests pass (no regressions)
- Evidence saved to `.sisyphus/evidence/task-8-codegen-tests.txt`

**Key Pattern:** Guard syntax is required for fallible parse functions; bare calls now produce type errors because parse functions return struct types with error fields, not raw values.
