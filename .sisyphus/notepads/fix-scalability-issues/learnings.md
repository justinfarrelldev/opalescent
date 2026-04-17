# Learnings

## [2026-04-17] Project Setup
- Language spec: `language-spec/*.op` — colon-block syntax, NO curly braces on guards/if/while
- Runtime: `runtime/opal_runtime.c` — 617 lines, 45 functions, 0 free() calls
- Codegen: `src/codegen/` — 16 files with `#[path]` attributes in `src/codegen.rs`
- Test projects: `test-projects/` — each has `opal.toml` + `src/main.op`
- Integration tests: `tests/integration_e2e.rs` — feature-gated with `--features integration`
- Trap mechanism: `opal_runtime_error("message")` → fprintf stderr + exit(1)
- LLVM upgrade: inkwell 0.8.0 → 0.9.0, LLVM 14 → 18 (llvm18-1 feature)
- `integer_literal_bits` signature: `fn integer_literal_bits(number: i64) -> Result<u64, CodegenError>` — two's complement encoding
- `is_signed_core_type` signature: `const fn is_signed_core_type(core_type: &CoreType) -> bool`
- Both functions duplicated: expressions.rs:741/748, adts.rs:368, expressions_numeric.rs:431

## [2026-04-17] Task 1: opal_runtime_error() Implementation
- **C Runtime**: Added `void opal_runtime_error(const char* message)` at line 27 in `runtime/opal_runtime.c`
  - Pattern: `fprintf(stderr, "%s\n", message); exit(1);` — matches `invalid_digit_error` style
  - Placed after `invalid_digit_error()` function
- **Codegen Registry**: Added to THREE locations (currently duplicated, fixed in Task 3):
  1. `declare_stdlib_function()` in `functions_stdlib.rs:174` — uses `void_fn!` macro with `i8_ptr` param
  2. `resolve_imported_runtime_name()` in `functions_stdlib.rs:227` — maps `(standard, opal_runtime_error)` to runtime name
  3. `is_stdlib_name()` in `functions.rs:391` — added to match block for stdlib name recognition
- **TDD Test**: Added `opal_runtime_error_is_stdlib_name()` test in `functions.rs:859`
  - Verifies: function resolves, returns void, accepts i8* parameter
  - Test passes GREEN after implementation
- **Build Status**: `cargo build` succeeds, `cargo test` passes 962/962 tests
- **Evidence Saved**:
  - `.sisyphus/evidence/task-1-runtime-trap-compiles.txt` — grep output showing all 3 locations
  - `.sisyphus/evidence/task-1-existing-tests-pass.txt` — test results (962 passed, 0 failed)

## Task 4: Format Specifier Portability (PRId64/PRIu64)

### Pattern: Portable Format Specifiers for Fixed-Width Integers
- **Problem**: `%ld` and `%lu` are non-portable across platforms (size varies by OS/arch)
- **Solution**: Use `<inttypes.h>` macros: `PRId64` for int64_t, `PRIu64` for uint64_t
- **Syntax**: `printf("%" PRId64 "\n", value)` — macro OUTSIDE string literal
- **Files affected**: `runtime/opal_runtime.c` (int64_to_string, uint64_to_string)
- **Verification**: C runtime compiles cleanly with gcc

### Key Learnings
1. Portable format specifiers are essential for cross-platform C code
2. The `<inttypes.h>` header provides platform-agnostic macros for all fixed-width types
3. Macro expansion happens at compile time, so `"%" PRId64` becomes the correct format string
4. No runtime overhead — purely a compile-time transformation
5. All 4 occurrences replaced successfully (2 in int64_to_string, 2 in uint64_to_string)

### Scalability Impact
- Enables compilation on Windows/macOS without format specifier warnings
- Reduces platform-specific code paths
- Improves binary portability across architectures

## [2026-04-17] Task 2: Extract Shared Codegen Helper Functions

### Deduplication Strategy
- **Functions extracted**: `integer_literal_bits(i64) -> Result<u64, CodegenError>` and `is_signed_core_type(&CoreType) -> bool`
- **Shared location**: `src/codegen/types.rs` — already a shared types module, ideal for utility functions
- **Rationale**: types.rs is the natural home for type-related helpers; avoids creating new helpers.rs module

### TDD Approach (RED → GREEN → REFACTOR)
1. **RED Phase**: Added unit tests to types.rs (6 tests total)
   - `test_integer_literal_bits_positive()` — verifies 42 → 42u64
   - `test_integer_literal_bits_zero()` — verifies 0 → 0u64
   - `test_integer_literal_bits_negative_one()` — verifies -1 → u64::MAX (two's complement)
   - `test_is_signed_core_type_signed()` — verifies Int8/16/32/64 return true
   - `test_is_signed_core_type_unsigned()` — verifies UInt8/16/32/64 return false
   - `test_is_signed_core_type_float()` — verifies Float32/64 return false
2. **GREEN Phase**: Moved functions to types.rs as `pub fn` / `pub const fn`
   - Added import: `use crate::codegen::expressions::CodegenError;`
   - All 6 tests pass immediately
3. **REFACTOR Phase**: Updated all call sites and deleted duplicates
   - expressions_numeric.rs: Added import, removed duplicate, 7 call sites now use shared version
   - expressions.rs: Added import, removed duplicate, 2 call sites now use shared version
   - adts.rs: Added import, removed duplicate, 1 call site now uses shared version

### Verification Results
- **No duplicates**: `grep -rn "fn integer_literal_bits|fn is_signed_core_type" src/codegen/` returns exactly 2 results (both in types.rs)
- **Unit tests**: All 6 tests pass (3 for integer_literal_bits, 3 for is_signed_core_type)
- **Build**: `cargo build` succeeds with 0 errors
- **Full test suite**: `cargo test` passes 969/969 tests (0 failures)
- **Evidence saved**:
  - `.sisyphus/evidence/task-2-no-duplicates.txt` — grep output confirming single location
  - `.sisyphus/evidence/task-2-unit-tests.txt` — test results (6 passed, 0 failed)
  - `.sisyphus/evidence/task-2-build-tests-pass.txt` — full build and test output

### Key Learnings
1. Shared types.rs module is the right home for type-related utility functions
2. TDD approach (RED → GREEN → REFACTOR) ensures correctness during refactoring
3. Two's complement encoding: `(!magnitude).wrapping_add(1)` for negative integers
4. `const fn` for pure functions enables compile-time evaluation
5. Deduplication reduces maintenance burden and ensures consistent behavior across call sites

## [2026-04-17] Task 3: Stdlib Function Name Registry Deduplication
- **Problem**: Three separate lists of 44 stdlib names each (132 total occurrences)
- **Solution**: Created single authoritative registry `STDLIB_NAMES: &[&str]` constant
- **Build Status**: `cargo build` succeeds, `cargo test` passes 969/969 tests (0 failures)

## [2026-04-17] Task 5: Integer overflow trapping in debug + release
- `codegen_numeric_binop` now always routes integer `add/sub/mul` through `codegen_checked_overflow_intrinsic`.
- Overflow trap path calls `opal_runtime_error("integer overflow")`.

## [2026-04-17] Task 6: Float→Int Cast Range Guard + NaN Trap
- Added float→int guard path in `codegen_cast` before every `fptosi`/`fptoui`.
- Full `cargo test` at that time had one unrelated pre-existing failure in lambda runtime test.

## [2026-04-17] Task 7: Lambda body codegen emits real body + parameter binding
- Fixed `resolve_callee_function` (`src/codegen/functions.rs`, `Expr::Lambda` arm) to follow regular function codegen flow instead of unconditional `emit_default_return`.
- Lambda lowering now:
  1. Computes param/return/capture core types
  2. Creates LLVM function + entry block
  3. Binds lambda parameters via alloca+store into `env.variables` (same model as `codegen_function_declaration`)
  4. Binds captured variables from trailing function params via alloca+store
  5. Recursively lowers `LambdaBody::Block` statements via `codegen_statement`
  6. Lowers `LambdaBody::Expression` and emits explicit return
  7. Emits default return only when insertion block exists and has no terminator
- Important detail: restore caller insertion block after lambda emission so call-site codegen continues in caller function.
- Added return-type-aware lowering for single-value lambda returns so lambda body arithmetic keeps declared width (`int32` remains i32 instead of widening to i64 in return path).
- Added tests in `src/codegen/functions.rs`:
  - `lambda_body_codegen_emits_body_and_returns_incremented_value`
  - `lambda_runtime_call_returns_incremented_result` (validates `f(x: int32): int32 => x + 1` with `5` yields `6`).
- Full regression run after change: `cargo test` passed with `973 passed; 0 failed; 5 ignored`.


## [2026-04-17] Task 8: Array bounds trap and length tracking in codegen
- Changed `CoreType::Array` LLVM lowering from zero-length array payload (`[0 x T]`) to internal fat struct `{ i64, T* }` in `src/codegen/types.rs`.
- `codegen_array_literal` now materializes runtime array struct value with:
  - field 0: element count (`i64`)
  - field 1: pointer to contiguous element payload (`T*`)
- Added reusable bounds-check helper in `src/codegen/expressions.rs`:
  - `emit_array_bounds_check(...)` emits:
    - signed negative-index check (`icmp slt`)
    - unsigned upper-bound check (`icmp uge index, length`)
    - conditional trap path calling `opal_runtime_error("array index out of bounds")`
    - `unreachable` in trap block
- Added integer normalization helper `normalize_int_to_i64(...)` so bounds checks and GEP index use a consistent `i64` index regardless of source integer width.
- `codegen_array_access` now:
  1. extracts `{len, ptr}` from array struct
  2. infers signedness from index expression type where possible
  3. emits bounds checks before GEP
  4. performs GEP with `[index]` on `T*` payload pointer (correct indexing pattern for element pointers)
- Function parameter behavior follows automatically because array-typed values are now represented as `{len, ptr}` and passed/stored using existing value flow.
- TDD flow executed:
  - RED: added `test_codegen_array_access_emits_bounds_trap_check_before_gep` and observed failure (no bounds checks in IR)
  - GREEN: implemented fat-array + bounds check helper + access updates
  - REFACTOR: extracted check logic into helper function and added index normalization helper
- Validation:
  - LSP diagnostics clean for changed files (`expressions.rs`, `types.rs`, `tests.rs`)
  - Full `cargo test` passed: `974 passed; 0 failed; 5 ignored`
  - Evidence written to `.sisyphus/evidence/task-8-bounds-check.txt`

## [2026-04-17] Tasks 12/13/14/16 completion recovery
- Parser now accepts declaration modifiers in any order before function declarations (`public`, `entry`, `pure`, `untested`).
- `entry` functions are normalized at parse-time to include `FunctionModifier::Untested` implicitly.
- Type checker now tracks active function modifiers with a stack and enforces pure-function restrictions at call sites.
- Pure-function enforcement is direct-call only to known impure stdlib names (`print`, `take_input`, `random_int32`) per task constraints.
- Added parser tests for `pure`, `untested`, and implicit `entry -> untested` behavior.
- Added type-system tests for rejecting impure stdlib calls inside pure functions while allowing same calls in non-pure functions.
- Added codegen test ensuring unsigned int→float casts emit `uitofp` and immutability assignment failures return `CodegenError` (no panic).
- Full verification after fixes: `cargo build` succeeded and `cargo test` passed (`976 passed; 0 failed; 5 ignored`).

## [2026-04-17] Task 15: Cast safety matching spec (4c)
- Implemented integer-cast range guards in `codegen_cast` for:
  - narrowing integer casts (`in_bits > out_bits`), and
  - same-width signedness-changing casts (`intN <-> uintN`).
- Guard behavior uses the same trap pattern as float→int checks:
  - conditional branch to trap block,
  - `opal_runtime_error("cast out of range: {source} to {target}")`,
  - `unreachable`, then continue via `ok` block.
- Added compile-time constant detection for integer literal casts:
  - if cast range fit can be decided at compile-time, the generated condition is a constant i1
  - still keeps unified branch/trap structure for consistency and easy IR auditing.
- Widening integer casts remain unchanged (no range checks).
- Added codegen tests:
  - `test_codegen_narrowing_signed_int_cast_emits_runtime_range_trap`
  - `test_codegen_widening_signed_int_cast_emits_no_range_trap`
  - `test_codegen_same_width_signed_to_unsigned_cast_emits_runtime_range_trap`
  - `test_codegen_same_width_unsigned_to_signed_cast_emits_runtime_range_trap`
- Verification for this task:
  - LSP diagnostics clean for changed files
  - full `cargo test` passed: `980 passed; 0 failed; 5 ignored`
