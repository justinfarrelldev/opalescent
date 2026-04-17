# F3 Manual QA Results (fail-fast-conversions)

## Scenario 1: All tests pass
Command:
```bash
cargo test 2>&1 | grep "test result"
```
Output (summary line):
- `test result: ok. 960 passed; 0 failed; 5 ignored; 0 measured; 0 filtered out; finished in 0.12s`

## Scenario 2: Guard pattern type-checks
Command:
```bash
cargo test --lib test_guard_with_string_to_int32 2>&1 | tail -5
```
Result:
- `test ...test_guard_with_string_to_int32_type_checks ... ok`

## Scenario 3: Bare call produces compile error
Command:
```bash
cargo test --lib test_bare_call_to_string_to_uint32 2>&1 | tail -5
```
Result:
- `test ...test_bare_call_to_string_to_uint32_produces_unhandled_call_error ... ok`

## Scenario 4: Propagate pattern type-checks
Command:
```bash
cargo test --lib test_propagate_string_to_int32 2>&1 | tail -5
```
Result:
- `test ...test_propagate_string_to_int32_in_error_function_type_checks ... ok`

## Scenario 5: int32_to_string type-checks (infallible)
Command:
```bash
cargo test --lib test_int32_to_string 2>&1 | tail -5
```
Result:
- `test ...test_int32_to_string_type_checks ... ok`
- `test ...test_int32_to_string_does_not_require_error_handling ... ok`

## Scenario 6: Struct-return LLVM IR assertion
Command:
```bash
cargo test --lib test_guard_statement_compiles_to_valid_llvm_ir 2>&1 | tail -5
```
Result:
- `test codegen::tests::test_guard_statement_compiles_to_valid_llvm_ir ... ok`

## Scenario 7: simple-quiz pre-existing failure (NOT a regression)
Command:
```bash
cargo run -- check test-projects/simple-quiz/src/main.op 2>&1 | head -5
```
Output:
- `error: lex errors in source`

## Edge cases (runtime/opal_runtime.c)
- Confirmed `string_to_int32` returns `ParseResultI32` struct: `typedef struct { int32_t value; const char* error; } ParseResultI32;` and `ParseResultI32 string_to_int32(const char* s)`.
- Confirmed `int32_to_string` exists and returns `char*`: `char* int32_to_string(int32_t value)`.
