# Opalescent: Exhaustive Scalability & Correctness Issues

This document is an exhaustive, engineer-oriented catalog of every known issue
identified in the current Opalescent implementation. Each entry specifies the
exact files, functions, and line numbers affected, explains the impact, and
notes the language-spec deviation where applicable.

---

## Table of Contents

1. [Silent Data Corruption & Undefined Behavior](#1-silent-data-corruption--undefined-behavior)
2. [C Runtime Portability & Safety Issues](#2-c-runtime-portability--safety-issues)
3. [Memory Management (Leaks & Missing Bounds)](#3-memory-management-leaks--missing-bounds)
4. [Language-Spec Deviations](#4-language-spec-deviations)
5. [Code Duplication & Maintainability Debt](#5-code-duplication--maintainability-debt)
6. [Architectural / Scalability Concerns](#6-architectural--scalability-concerns)

---

## 1. Silent Data Corruption & Undefined Behavior

### 1a. Integer overflow silently wraps in release mode

**File:** `src/codegen/expressions_numeric.rs`  
**Function:** `codegen_numeric_binop` (lines 17–82)  
**Root cause:** The function checks `env.debug_mode` (line 58). When `false`,
it emits plain LLVM `build_int_add` / `build_int_sub` / `build_int_mul` (lines
65–79), which silently wrap on overflow. The checked-overflow intrinsic path
(`codegen_checked_overflow_intrinsic`, lines 84+) is only taken in debug mode.

**Functions affected:**

- `codegen_numeric_binop` — the sole entry point for all integer `+`, `-`, `*`

**Impact:** Every integer add, subtract, and multiply in a release-compiled
Opalescent program wraps silently. This directly contradicts the spec goal of
"safety — error checking is more important."

**Spec reference:** `language-spec/requirements/overview.md`:
_"The build time should NOT come at the expense of safety - error checking is
more important."_

---

### 1b. Float-to-integer cast has no range guard (LLVM undefined behavior)

**File:** `src/codegen/expressions.rs`  
**Function:** `codegen_cast` (lines 419–431)  
**Affected code path:** When a float value is cast to an integer type, the code
emits either `build_float_to_signed_int` (fptosi) or
`build_float_to_unsigned_int` (fptoui) with no range check.

Per the LLVM Language Reference, `fptosi` / `fptoui` produce **poison** if the
float value is out of the destination integer's representable range. This is
LLVM-level undefined behavior.

**All affected cast pairs (source → target):**

- `float32 → int8`, `float32 → int16`, `float32 → int32`, `float32 → int64`
- `float32 → uint8`, `float32 → uint16`, `float32 → uint32`, `float32 → uint64`
- `float64 → int8`, `float64 → int16`, `float64 → int32`, `float64 → int64`
- `float64 → uint8`, `float64 → uint16`, `float64 → uint32`, `float64 → uint64`

**Spec reference:** `language-spec/requirements/overview.md`:
_"Float↔int and int↔float casts require `as`; out-of-range results are compile
errors for constants and runtime traps unless the `checked_` or `saturating_`
APIs are used."_

The spec explicitly demands runtime traps; the implementation silently produces
LLVM poison.

---

### 1c. Unsigned int-to-float cast uses wrong LLVM instruction

**File:** `src/codegen/expressions.rs`  
**Function:** `codegen_cast` (lines 387–391)  
**Affected code path:** When an integer is cast to a float, the code
unconditionally calls `build_signed_int_to_float` (`sitofp`), regardless of
whether the source type is signed or unsigned.

For unsigned types (`uint32`, `uint64`, etc.), `sitofp` reinterprets the high
bit as a sign bit, producing incorrect negative float results for values
≥ 2^(n-1).

**All affected cast pairs (source → target):**

- `uint8 → float32`, `uint8 → float64`
- `uint16 → float32`, `uint16 → float64`
- `uint32 → float32`, `uint32 → float64`
- `uint64 → float32`, `uint64 → float64`

**Fix:** Branch on `is_signed_core_type(&source_type)` and call
`build_unsigned_int_to_float` (`uitofp`) for unsigned source types.

---

### 1d. Lambda bodies are never code-generated

**File:** `src/codegen/functions.rs`  
**Function:** `resolve_callee_function`, `Expr::Lambda` arm (lines 452–472)  
**Affected code:** The lambda branch creates a new LLVM function with
`module.add_function`, appends an `entry` basic block, positions the builder at
it, calls `emit_default_return` — and then **returns**. There is no codegen of
the lambda body. Every lambda call executes only the default-return path,
returning a zero value for its declared return type.

**Functions affected:**

- `resolve_callee_function` — the `Expr::Lambda { .. }` match arm
- `emit_default_return` (lines 510–538) — called from lambda path

**Impact:** All lambda expressions silently return zero/false/null regardless of
their body. Any Opalescent program using lambdas produces incorrect results
without warning.

---

### 1e. Missing captured variables silently become zero

**File:** `src/codegen/functions.rs`  
**Function:** `codegen_call_expression`, captured-variable lowering (lines 198–210)  
**Affected code:** When lowering captured variables for a lambda call, if a
capture name is not found in `env.variables`, the code pushes
`i64_type().const_zero()` as the argument instead of raising an error.

```rust
} else {
    lowered_args.push(
        codegen_context
            .context
            .i64_type()
            .const_zero()
            .as_basic_value_enum()
            .into(),
    );
}
```

**Impact:** Silent data corruption — a captured variable that was expected to
hold a meaningful value (string, struct, float, etc.) is replaced by integer 0.

---

### 1f. `emit_default_return` silently returns zero for non-void functions

**File:** `src/codegen/functions.rs`  
**Function:** `emit_default_return` (lines 510–538)  
**Affected code:** For non-void functions, this function returns
`const_zero()` of the return type. This is called from:

1. Lambda codegen (issue 1d above)
2. `emit_function_default_return` (lines 540–556) — used in the propagate
   early-return path

**Impact:** Non-void functions that hit the default-return path silently return
`0` / `0.0` / `false` / `null_ptr` instead of producing a compile error or
runtime trap.

---

### 1g. No array bounds checking

**File:** `src/codegen/expressions.rs`  
**Function:** `codegen_array_access` (lines 493–523)  
**Affected code:** The function uses `build_in_bounds_gep` with no bounds
guard. The LLVM `inbounds` flag is an optimization hint to the optimizer, NOT a
safety check — it is UB if the index is out of bounds.

There is no:

- Length tracking on arrays
- Runtime bounds comparison before the GEP
- Trap/abort on out-of-range indices

**Impact:** Out-of-bounds array access silently reads/writes arbitrary memory.

---

## 2. C Runtime Portability & Safety Issues

All issues in this section are in **`runtime/opal_runtime.c`** (617 lines).

### 2a. `invalid_digit_error` uses a non-thread-safe static buffer

**Function:** `invalid_digit_error` (lines 22–25)  
**Code:**

```c
static char msg[64];
snprintf(msg, sizeof(msg), "invalid digit '%c' in input", ch);
return msg;
```

This function is called from every `string_to_*` parser. If two parsing calls
race (e.g. in a multi-threaded host), the shared buffer is overwritten.

**All callers (every string-to-numeric parser):**

- `string_to_int8` (line 173)
- `string_to_int16` (line 210)
- `string_to_int32` (line 240)
- `string_to_int64` (line 275)
- `string_to_uint8` (line 317)
- `string_to_uint16` (line 354)
- `string_to_uint32` (line 391)
- `string_to_uint64` (line 429)
- `string_to_float32` (line 466)
- `string_to_float64` (line 503)

---

### 2b. `%ld` / `%lu` format specifiers are non-portable (wrong on Windows)

On Windows, `long` is 32-bit even on 64-bit platforms. The `%ld` and `%lu`
format specifiers do not match `int64_t` / `uint64_t`. The portable specifiers
are `PRId64` / `PRIu64` from `<inttypes.h>`.

**Affected functions and their format strings:**

| Function | Line | Current format | Correct format |
|---|---|---|---|
| `int64_to_string` | 569–571 | `"%ld"` | `"%" PRId64` |
| `uint64_to_string` | 597–599 | `"%lu"` | `"%" PRIu64` |

**Functions that are correctly portable (for reference):**

| Function | Line | Format | Status |
|---|---|---|---|
| `int8_to_string` | 548–550 | `"%d"` (cast to `int`) | OK |
| `int16_to_string` | 555–557 | `"%d"` (cast to `int`) | OK |
| `int32_to_string` | 562–564 | `"%d"` | OK |
| `uint8_to_string` | 576–578 | `"%u"` (cast to `unsigned`) | OK |
| `uint16_to_string` | 583–585 | `"%u"` (cast to `unsigned`) | OK |
| `uint32_to_string` | 590–592 | `"%u"` | OK |
| `float32_to_string` | 604–606 | `"%g"` (cast to `double`) | OK |
| `float64_to_string` | 611–613 | `"%g"` | OK |
| `bool_to_string` | 615 | N/A (uses `strdup`) | OK |

**Print functions with the same portability issue:**

| Function | Line | Current format | Correct format |
|---|---|---|---|
| `print_int64` | 67 | `"%lld\n"` | `"%" PRId64 "\n"` |
| `print_uint64` | 79 | `"%llu\n"` | `"%" PRIu64 "\n"` |

Note: `print_int64` and `print_uint64` use `%lld`/`%llu` with casts to
`long long`/`unsigned long long`. This is technically portable since C99
guarantees `long long` ≥ 64 bits, but is inconsistent with the `*_to_string`
functions' approach. For consistency, all should use `<inttypes.h>` macros.

---

### 2c. `malloc` return values never checked for `NULL`

Every `*_to_string` function calls `malloc` and uses the result without
checking for `NULL`.

**All affected functions:**

- `int8_to_string` (line 549)
- `int16_to_string` (line 556)
- `int32_to_string` (line 563)
- `int64_to_string` (line 570)
- `uint8_to_string` (line 577)
- `uint16_to_string` (line 584)
- `uint32_to_string` (line 591)
- `uint64_to_string` (line 598)
- `float32_to_string` (line 605)
- `float64_to_string` (line 612)

Additionally, `take_input` (line 50) calls `strdup` which internally calls
`malloc` — the return is not checked.

`bool_to_string` (line 615) calls `strdup` — same issue.

---

### 2d. Low-quality RNG via `srand(time(NULL))` + `rand()`

**Function:** `seed_rand_once` (lines 95–100)  
**All affected RNG functions:**

- `random_int8` (lines 102–106)
- `random_int16` (lines 108–112)
- `random_int32` (lines 114–118)
- `random_int64` (lines 120–124)
- `random_uint8` (lines 126–130)
- `random_uint16` (lines 132–136)
- `random_uint32` (lines 138–142)
- `random_uint64` (lines 144–148)

**Issues:**

1. `srand(time(NULL))` seeds with 1-second granularity — programs started in the
   same second get identical sequences.
2. `rand()` typically returns only 15–31 bits on many platforms (RAND_MAX is
   often 2^31-1 or even 2^15-1). For `random_int64` / `random_uint64`, this
   means only a tiny fraction of the 64-bit range is reachable.
3. `rand() % range` introduces modulo bias when `range` does not evenly divide
   `RAND_MAX + 1`.
4. `seed_rand_once` uses a non-thread-safe static flag.

---

### 2e. `take_input` uses a static buffer

**Function:** `take_input` (lines 46–53)  
**Code:**

```c
static char buf[1024];
```

Input longer than 1023 characters is silently truncated. The static buffer is
not thread-safe.

---

## 3. Memory Management (Leaks & Missing Bounds)

### 3a. Zero `free()` calls in the entire runtime

**File:** `runtime/opal_runtime.c`  
**Verification:** `grep -c 'free(' runtime/opal_runtime.c` returns **0**.

Every heap allocation in the runtime leaks:

**Allocation sites that leak:**

| Function | Allocator | Line |
|---|---|---|
| `int8_to_string` | `malloc` | 549 |
| `int16_to_string` | `malloc` | 556 |
| `int32_to_string` | `malloc` | 563 |
| `int64_to_string` | `malloc` | 570 |
| `uint8_to_string` | `malloc` | 577 |
| `uint16_to_string` | `malloc` | 584 |
| `uint32_to_string` | `malloc` | 591 |
| `uint64_to_string` | `malloc` | 598 |
| `float32_to_string` | `malloc` | 605 |
| `float64_to_string` | `malloc` | 612 |
| `bool_to_string` | `strdup` | 615 |
| `take_input` | `strdup` | 50 |

### 3b. Zero `free()` calls in the LLVM codegen

**File:** `src/codegen/expressions_string.rs`  
**Verification:** `grep 'free\|dealloc' src/codegen/**` returns only a comment
at line 30 acknowledging the leak:

```
// freed (consistent with how string constants are handled in the runtime).
```

The LLVM codegen never emits calls to `free()` for any heap-allocated value.
This means string interpolation buffers, `take_input` results, and all
`*_to_string` return values are leaked.

---

### 3c. String interpolation uses a fixed 256-byte heap buffer

**File:** `src/codegen/expressions_string.rs`  
**Function:** `codegen_string_interpolation` (lines 14–60)  
**Code (line 37):**

```rust
let buf_size = codegen_context.context.i64_type().const_int(256_u64, false);
```

The function `malloc`s 256 bytes and calls `sprintf` into it. If the
interpolated result exceeds 256 bytes, `sprintf` writes past the end of the
buffer — a classic heap buffer overflow.

**All code that triggers this path:** Any string interpolation expression in
Opalescent source code (e.g., `'Hello {name}, your score is {score}'`).

---

## 4. Language-Spec Deviations

### 4a. `pure` keyword: not lexed, not parsed, not enforced

**Spec reference:** The language was designed with purity as a core goal.

**Verification:**

- `grep -r '"pure"\|"untested"' src/token.rs src/lexer/` — **0 results**
- The keywords `pure` and `untested` do not appear in the token/lexer
  definitions.
- No purity analysis or side-effect tracking exists anywhere in the type checker
  or codegen.

**Affected areas:** The entire compiler pipeline — lexer, parser, type checker,
codegen — has no concept of function purity.

---

### 4b. `untested` keyword: not lexed, not parsed, not enforced

Same as 4a. The `untested` keyword described in the language goals for marking
functions lacking test coverage is entirely unimplemented.

---

### 4c. Cast safety does not match spec requirements

**Spec (`language-spec/requirements/overview.md`):**
> _"The conversion is lossy at compile-time (for literals/constants)"_ → should
> fail to compile  
> _"Float↔int casts — out-of-range results are compile errors for constants and
> runtime traps unless the `checked_` or `saturating_` APIs are used."_

**Current implementation (`src/codegen/expressions.rs`, `codegen_cast`):**

- No compile-time constant evaluation for cast safety
- No runtime trap emission for out-of-range casts
- No `checked_*` or `saturating_*` cast APIs exist
- Float→int produces LLVM poison (UB) on out-of-range (see issue 1b)
- Unsigned int→float uses wrong instruction (see issue 1c)

---

### 4d. Immutability not enforced in codegen (defense-in-depth gap)

**Type checker enforcement:**
`src/type_system/checker/statements.rs` line 535 — correctly raises
`TypeError::ImmutableAssignment`.

**Codegen gap:**
`src/codegen/statements.rs`, function `codegen_assignment` (lines 200–225) —
performs `build_store` to any variable without checking `is_mutable`. A codegen
bug that bypasses the type checker would silently mutate immutable variables.

**Verification:** `grep -r 'is_mutable' src/codegen/` returns hits only in
`src/codegen/tests.rs` (test data), never in production codegen code.

---

## 5. Code Duplication & Maintainability Debt

### 5a. Stdlib function name list duplicated in 3 places

The complete list of stdlib function names appears independently in **three**
locations. Adding a new stdlib function requires updating all three, and
forgetting one causes silent failures.

**Location 1:** `src/codegen/functions.rs`, `resolve_callee_function` (lines
371–408)  
An inline `matches!()` expression checking `is_stdlib_name` for 32 function
names.

**Location 2:** `src/codegen/functions_stdlib.rs`, `declare_stdlib_function`
(lines 22–183)  
A `match` block with per-function LLVM type declarations for 36 function names.

**Location 3:** `src/codegen/functions_stdlib.rs`, `resolve_imported_runtime_name`
(lines 186–262)  
A `match` block mapping `(module, symbol)` pairs to runtime names for 40
function names.

**Complete list of duplicated names:**

| # | Name | Location 1 | Location 2 | Location 3 |
|---|---|---|---|---|
| 1 | `print` | ✓ | ✓ | ✓ |
| 2 | `printf` | ✓ | ✓ | — |
| 3 | `print_string` | ✓ | ✓ | ✓ |
| 4 | `print_int8` | ✓ | ✓ | ✓ |
| 5 | `print_int16` | ✓ | ✓ | ✓ |
| 6 | `print_int32` | ✓ | ✓ | ✓ |
| 7 | `print_int64` | ✓ | ✓ | ✓ |
| 8 | `print_uint8` | ✓ | ✓ | ✓ |
| 9 | `print_uint16` | ✓ | ✓ | ✓ |
| 10 | `print_uint32` | ✓ | ✓ | ✓ |
| 11 | `print_uint64` | ✓ | ✓ | ✓ |
| 12 | `print_float32` | ✓ | ✓ | ✓ |
| 13 | `print_float64` | ✓ | ✓ | ✓ |
| 14 | `take_input` | ✓ | ✓ | ✓ |
| 15 | `random_int8` | ✓ | ✓ | ✓ |
| 16 | `random_int16` | ✓ | ✓ | ✓ |
| 17 | `random_int32` | ✓ | ✓ | ✓ |
| 18 | `random_int64` | ✓ | ✓ | ✓ |
| 19 | `random_uint8` | ✓ | ✓ | ✓ |
| 20 | `random_uint16` | ✓ | ✓ | ✓ |
| 21 | `random_uint32` | ✓ | ✓ | ✓ |
| 22 | `random_uint64` | ✓ | ✓ | ✓ |
| 23 | `string_to_int8` | ✓ | ✓ | ✓ |
| 24 | `string_to_int16` | ✓ | ✓ | ✓ |
| 25 | `string_to_int32` | ✓ | ✓ | ✓ |
| 26 | `string_to_int64` | ✓ | ✓ | ✓ |
| 27 | `string_to_uint8` | ✓ | ✓ | ✓ |
| 28 | `string_to_uint16` | ✓ | ✓ | ✓ |
| 29 | `string_to_uint32` | ✓ | ✓ | ✓ |
| 30 | `string_to_uint64` | ✓ | ✓ | ✓ |
| 31 | `string_to_float32` | ✓ | ✓ | ✓ |
| 32 | `string_to_float64` | ✓ | ✓ | ✓ |
| 33 | `int8_to_string` | — | ✓ | ✓ |
| 34 | `int16_to_string` | — | ✓ | ✓ |
| 35 | `int32_to_string` | — | ✓ | ✓ |
| 36 | `int64_to_string` | — | ✓ | ✓ |
| 37 | `uint8_to_string` | — | ✓ | ✓ |
| 38 | `uint16_to_string` | — | ✓ | ✓ |
| 39 | `uint32_to_string` | — | ✓ | ✓ |
| 40 | `uint64_to_string` | — | ✓ | ✓ |
| 41 | `float32_to_string` | — | ✓ | ✓ |
| 42 | `float64_to_string` | — | ✓ | ✓ |
| 43 | `bool_to_string` | — | ✓ | ✓ |

Note: Location 1 is missing `*_to_string` and `bool_to_string` names — these
names won't match the `is_stdlib_name` guard, potentially causing different
codegen behavior (e.g. attempting monomorphization on stdlib functions).

---

### 5b. `integer_literal_bits` function duplicated identically

**Copy 1:** `src/codegen/expressions.rs` line 748  
**Copy 2:** `src/codegen/adts.rs` line 368

Both are identical implementations. The function in `expressions.rs` is used at
line 200; the one in `adts.rs` is used at line 133.

---

### 5c. `is_signed_core_type` function duplicated

**Copy 1:** `src/codegen/expressions.rs` line 741  
**Copy 2:** (used across multiple codegen files via re-import or inline redefinition)

This helper is small enough that duplication is a pattern risk — any time a new
file needs it, it gets copied.

---

## 6. Architectural / Scalability Concerns

### 6a. 106 `#[path = ...]` module attributes across 13 files

The codebase uses non-idiomatic `#[path = "..."]` attributes instead of
standard Rust module directory conventions (`mod.rs` or `dirname.rs` +
`dirname/`).

**Files and their `#[path]` counts:**

| File | `#[path]` count |
|---|---|
| `src/codegen.rs` | 15 |
| `src/lib.rs` | 12 |
| `src/hot_reload.rs` | 11 |
| `src/lsp.rs` | 10 |
| `src/runtime.rs` | 8 |
| `src/stdlib.rs` | 8 |
| `src/testing.rs` | 7 |
| `src/formatter.rs` | 7 |
| `src/package_manager.rs` | 7 |
| `src/build_system.rs` | 6 |
| `src/benchmarks.rs` | 6 |
| `src/doc_gen.rs` | 5 |
| `src/errors.rs` | 4 |
| **Total** | **106** |

**Impact:** Every new submodule requires adding a `#[path]` attribute. IDE
navigation, `rust-analyzer`, and standard Rust tooling expect the conventional
layout. This pattern does not scale and makes the module hierarchy opaque to
new contributors.

---

### 6b. `NEXT_NODE_ID` is a global atomic that never resets

**File:** `src/parser.rs` (lines 47–54)  
**Code:**

```rust
static NEXT_NODE_ID: AtomicUsize = AtomicUsize::new(1);

fn next_node_id() -> NodeId {
    NodeId(NEXT_NODE_ID.fetch_add(1, Ordering::Relaxed))
}
```

**All callers:** `next_node_id()` is called from `src/parser/statements.rs`
(at least 20+ call sites confirmed) and `src/parser/expressions.rs` for every
AST node construction.

**Issues:**

1. **Non-determinism:** In multi-threaded or concurrent compilation scenarios
   (LSP, build system), node IDs are non-deterministic.
2. **No reset:** Across multiple compilations in the same process (e.g., hot
   reload, LSP re-parsing), IDs grow monotonically and never reset, making
   cross-compilation ID comparisons meaningless.
3. **`Ordering::Relaxed`:** Provides no happens-before guarantees across
   threads.

---

### 6c. `TypeChecker` struct has 20 fields with ad-hoc state stacks

**File:** `src/type_system/checker.rs` (lines 63–101)

**All fields:**

1. `environment: TypeEnvironment`
2. `next_var_id: usize`
3. `symbol_table: SymbolTable`
4. `constraints: Vec<TypeConstraint>`
5. `guard_else_depth: usize`
6. `guard_error_stack: Vec<Vec<CoreType>>`
7. `in_propagate_context: bool`
8. `in_guard_subject_context: bool`
9. `return_label_modes: Vec<ReturnLabelMode>`
10. `warnings: Vec<Warning>`
11. `function_hot_reload_metadata: BTreeMap<String, FunctionHotReloadMetadata>`
12. `arithmetic_modes: BTreeMap<usize, ArithmeticMode>`
13. `constant_integer_values: BTreeMap<usize, i128>`
14. `adt_variants: BTreeMap<String, Vec<String>>`
15. `loop_break_type_stack: Vec<Option<Vec<CoreType>>>`
16. `adt_fields: BTreeMap<String, BTreeMap<String, CoreType>>`
17. `adt_generic_params: BTreeMap<String, Vec<GenericTypeParameter>>`
18. `generic_instantiations: BTreeMap<String, Vec<Vec<CoreType>>>`
19. `module_resolver: ModuleResolver`
20. `current_module_path: String`

Fields 5–9 and 15 are ad-hoc context stacks (depth counters, boolean flags,
Vec-based stacks) that are manually pushed/popped throughout the checker code.
This pattern is error-prone: forgetting to pop a stack or resetting a flag
produces subtle type-checking bugs.

**Scaling concern:** Every new language feature that requires context tracking
(e.g., async/await, pattern matching guards, effect handlers) will add more
fields to this struct.

---

### 6d. Linker invocation assumes Unix-like environment

**File:** `src/compiler.rs`  
**Function:** `link_object_file` (lines 261–283)  
**Code:**

```rust
let mut command = Command::new("cc");
command.arg(object_path);
command.arg(runtime_temp_file.path());
if cfg!(target_os = "linux") {
    command.arg("-no-pie");
}
command.arg("-o").arg(output_path);
```

**Issues:**

1. `Command::new("cc")` — Windows does not have `cc` in PATH by default
2. `-o` flag syntax is Unix-specific
3. `-no-pie` is only conditionally added for Linux; macOS behavior is unhandled
4. No MSVC toolchain support
5. No cross-compilation support

---

### 6e. LLVM version hard-pinned to 14

**File:** `Cargo.toml`  
**Dependency:**

```toml
inkwell = { version = "0.8.0", features = ["llvm14-0", "llvm14-0-prefer-dynamic"] }
```

LLVM 14 was released in March 2022. As of this writing (April 2026), LLVM 22.1.3+
is current. The hard pin means:

- No access to newer LLVM optimizations and bug fixes
- Increasing difficulty finding LLVM 14 packages on newer OS releases
- `inkwell 0.8.0` may not receive updates for newer Rust editions

Please update this to the most recent reasonable update.

---

### 6f. C runtime is a single 617-line monolithic file

**File:** `runtime/opal_runtime.c` (617 lines)

All runtime functions — I/O, string parsing, numeric printing, string
conversion, and RNG — are in a single file. This makes it difficult to:

- Test individual functions in isolation
- Conditionally link only needed functions
- Add new runtime features without increasing compile time for all users
- Maintain clear ownership boundaries

**Function count by category in the single file:**

| Category | Functions | Count |
|---|---|---|
| I/O | `take_input`, `print_string` | 2 |
| Numeric printing | `print_int8` through `print_float64` | 10 |
| String→numeric parsing | `string_to_int8` through `string_to_float64` | 10 |
| Numeric→string conversion | `int8_to_string` through `bool_to_string` | 11 |
| RNG | `random_int8` through `random_uint64` | 8 |
| Internal helpers | `invalid_digit_error`, `skip_leading_whitespace`, `skip_trailing_whitespace`, `seed_rand_once` | 4 |
| **Total** | | **45** |

---

## Priority Summary

| Priority | Issue | Risk |
|---|---|---|
| **P0 — Correctness** | 1a (overflow wraps silently) | Silent wrong results in release |
| **P0 — Correctness** | 1b (float→int UB) | LLVM undefined behavior |
| **P0 — Correctness** | 1d (lambda bodies never emitted) | All lambdas return zero |
| **P0 — Correctness** | 1g (no array bounds check) | Memory corruption |
| **P0 — Correctness** | 3c (256-byte interp buffer) | Heap buffer overflow |
| **P0 — Correctness** | 1e (missing captures → zero) | Silent data corruption |
| **P0 — Correctness** | 1f (default return → zero) | Silent wrong results |
| **P1 — Correctness** | 1c (uint→float wrong insn) | Wrong results for large unsigned values |
| **P1 — Spec** | 4a (pure keyword missing) | Core language feature unimplemented |
| **P1 — Spec** | 4b (untested keyword missing) | Core language feature unimplemented |
| **P1 — Spec** | 4c (cast safety) | Spec promises traps; impl has UB |
| **P1 — Spec** | 4d (immutability not in codegen) | Defense-in-depth gap |
| **P1 — Memory** | 3a (zero frees in runtime) | Unbounded memory growth |
| **P1 — Memory** | 3b (zero frees in codegen) | Unbounded memory growth |
| **P1 — Portability** | 2b (%ld/%lu on Windows) | Wrong output on Windows |
| **P1 — Portability** | 6d (linker assumes Unix) | Won't compile on Windows |
| **P2 — Safety** | 2a (static buffer thread safety) | Race condition |
| **P2 — Safety** | 2c (malloc NULL unchecked) | Null deref on OOM |
| **P2 — Safety** | 2e (take_input static buf) | Truncation + thread safety |
| **P2 — Quality** | 2d (low-quality RNG) | Biased/predictable output |
| **P2 — Maintainability** | 5a (stdlib names ×3) | Must update 3 places per function |
| **P2 — Maintainability** | 5b (integer_literal_bits ×2) | Divergence risk |
| **P2 — Maintainability** | 6a (106 #[path] attrs) | Non-idiomatic, confuses tooling |
| **P2 — Maintainability** | 6b (global NEXT_NODE_ID) | Non-deterministic in LSP/hot-reload |
| **P2 — Maintainability** | 6c (TypeChecker 20 fields) | Hard to extend safely |
| **P2 — Maintainability** | 6e (LLVM 14 pinned) | Increasingly stale dependency |
| **P2 — Maintainability** | 6f (monolithic runtime.c) | Hard to test/extend |
