**CRITICAL: Always output the results of any commands you run to temp.log (in the root directory), then read that to get the results of the previous command. This ensures that you can read it - there is a bug at the moment that will prevent you from reading commands directly. Use tee when doing this so I can see the command running without having to open temp.log. You should NOT have to run "echo "Starting header test creation" | tee temp.log" or similar - only do it for test runs and other important commands. Echo should almost never be present.**

**CRITICAL: Prefer making changes to entire sections of each file - individual replacements take a lot of time. For example, if there are a lot of lint issues in the last 500 lines of a file, replace all of those lines at the same time rather than fixing each individual lint issue. Additionally, if you can mass-replace values with commands, do that rather than replacing them manually. For large documentation tasks or tasks that will span small changes across an entire file, tell me to invoke Claude Code and what prompt you will need, and then run an echo statement (which will pause the editor for me) and I will do it.**

See the "Steps" section at the bottom for your task.

# Common Commands

Uses cargo-make for build automation.

## Build Commands

- `cargo make build-all-windows` - Build all Windows targets (x86 and x64)
- `cargo make build-all-linux` - Build all Linux targets (x86 and x64)
- `cargo make build-all` - Build all targets for current platform
- `cargo make dev` - Build dev server with info logging

## Linting Commands

- `cargo make lint` - Run clippy with strict warnings
- `cargo make lint-fix` - Run clippy with automatic fixes

## Testing Commands

- `cargo make test` - Run standard test suite
- `cargo make test-verbose` - Run tests with verbose output (--nocapture)
- `cargo make test-release` - Run tests in release mode for performance testing

# LINTING

The linting rules are intentionally very strict. It is vital that you develop features with them in mind. Here are pointers to ensure you do not run into issues with them:

## Documentation Requirements

- **Document ALL items**: Every function, struct, enum, trait, module, and even private items must have documentation comments ( or )
- **Document unsafe blocks**: Every `unsafe` block requires a `// SAFETY:` comment explaining why the unsafe code is safe
- **Document safety requirements**: Functions with `unsafe` in their signature need `# Safety` sections in their documentation

## Error Handling & Panicking

- **Never use `panic!`**: Use `Result` types and proper error handling instead
- **Avoid `todo!()`, `unimplemented!()`, `unreachable!()`**: Complete all code paths before committing
- **No `unwrap()` or `expect()`**: Use pattern matching, `if let`, or proper error propagation
- **No `get().unwrap()`**: Use safe indexing alternatives or handle the `None` case
- **Handle `Result` types properly**: Don't ignore results from functions that can fail
- **No panicking in `Result`-returning functions**: Functions returning `Result` should never panic

## Memory Management & Safety

- **Avoid `mem::forget()`**: Use proper RAII patterns instead
- **Don't use `Arc<Mutex<T>>` when `Mutex<T>` suffices**: Prefer simpler synchronization primitives
- **Avoid `Rc<Vec<T>>` and similar**: Use more appropriate data structures
- **No raw pointer dereferencing without safety comments**: Document why dereferencing is safe

## String Handling

- **Use `String::push_str()` instead of `String::add()`**: More efficient for concatenation
- **Avoid `str.to_string()`**: Use `str.to_owned()` or `String::from(str)` for clarity
- **Don't convert `String` to `String`**: Avoid redundant conversions
- **Use appropriate string types**: Choose between `&str`, `String`, `Cow<str>` based on needs

## Numeric Operations

- **Handle arithmetic overflow**: Use checked arithmetic operations (`checked_add`, etc.)
- **Avoid integer division without overflow checks**: Use safe division methods
- **Be explicit with float comparisons**: Use `float_cmp` or epsilon comparisons for floats
- **Specify numeric literal types**: Don't rely on default numeric fallbacks
- **Separate literal suffixes**: Use `1_000_u32` instead of `1000u32`

## Type Conversions & Casts

- **Avoid `as` conversions**: Use `TryFrom`/`TryInto` or explicit conversion methods
- **Don't cast functions to numeric types**: Use proper function pointer handling
- **Be explicit about type annotations**: Avoid redundant but add necessary ones

## Pattern Matching & Control Flow

- **Use exhaustive pattern matching**: Don't use `..` in fully bound structs
- **Avoid underscore patterns for must-use types**: Handle important values explicitly
- **Don't use `_` for untyped bindings**: Specify types when needed
- **Handle all enum variants**: Make enums exhaustive or explicitly handle new variants

## Module Organization & Visibility

- **Use `mod.rs` consistently**: Don't mix module file naming conventions
- **Avoid `pub use`**: Re-exports should be minimized and well-justified
- **Don't expose partial public fields**: Make structs fully public or fully private
- **Use proper module hierarchy**: Avoid self-named module files

## Macro & Import Usage

- **Avoid `#[macro_use]`**: Use explicit macro imports instead
- **Don't use `use` statements in macros**: Be explicit about dependencies

## Testing & Debugging

- **Remove `dbg!()` macros**: Use proper logging instead
- **Keep tests in test modules**: Use `#[cfg(test)]` modules or separate test files
- **Add assertion messages**: All assertions should have descriptive messages
- **Use `std::fs::read_to_string()` instead of manual file reading**: More concise and safe

## File & I/O Operations

- **Don't use `Path::is_file()`**: Use more specific file type checking
- **Use appropriate file reading methods**: Choose based on expected file size and usage
- **Handle directory creation properly**: Don't use bare `create_dir()` without error handling

## Performance & Efficiency

- **Avoid cloning reference-counted pointers**: Use references when possible
- **Don't format strings just to push them**: Use direct concatenation or `write!` macros
- **Use appropriate data structures**: Choose `Vec`, `HashMap`, etc. based on access patterns

## Lifetimes & References

- **Use descriptive lifetime names**: Avoid single-character lifetimes like `'a`
- **Handle reference patterns appropriately**: Use `&` patterns correctly
- **Avoid shadowing unrelated variables**: Use different names for different purposes

## Attributes & Configuration

- **Provide reasons for `#[allow]` attributes**: Use `#[allow(lint, reason = "explanation")]`
- **Minimize allow attributes**: Only suppress lints when absolutely necessary
- **Use appropriate conditional compilation**: Platform-specific code should be properly gated

## Code Organization

- **Keep implementation blocks together**: Don't scatter `impl` blocks for the same type
- **Use proper endianness handling**: Be explicit about byte order in binary operations
- **Avoid infinite loops without explicit intent**: Use `loop` only when infinite iteration is intended

## Dependencies & Standard Library

- **Prefer `core` over `std` when possible**: For no-std compatibility (though this is allowed in your config)
- **Use `alloc` instead of `std` for allocation-only features**: When building for restricted environments

## General Code Quality

- **Write self-documenting code**: Variable and function names should be descriptive
- **Keep functions focused**: Single responsibility principle
- **Handle edge cases**: Consider boundary conditions and error states
- **Use type system for correctness**: Leverage Rust's type system to prevent errors at compile time

This strict linting setup will help maintain high code quality, safety, and maintainability, but requires careful attention to these details throughout development.

# Full linting rules

All of the linting rules and descriptions of how to avoid tripping over them are below.

- **warnings**: Treat all compiler warnings as errors; avoid committing code that builds with warnings.
- **clippy::all/pedantic/nursery**: Enable broad + strict Clippy checks; avoid style/maintainability footguns Clippy already knows about.
- **clippy::cargo** _(allowed)_: Skip Cargo-metadata style lints; no action.
- **clippy::undocumented_unsafe_blocks**: Every `unsafe` needs a justification comment; avoid unexplained `unsafe`.
- **clippy::multiple_unsafe_ops_per_block** _(allowed)_: Multiple ops per `unsafe` block are OK here.
- **clippy::missing_docs_in_private_items**: Document private items; avoid undocumented modules/types fns even if private.
- **clippy::missing_safety_doc**: Document why `unsafe fn` is safe to call; avoid `unsafe fn` without a “Safety:” section.
- **clippy::panic**: Don’t call `panic!` in library/critical code; prefer `Result`/errors.
- **clippy::todo / clippy::unimplemented**: Don’t leave `todo!()`/`unimplemented!()`; implement or gate behind cfg.
- **clippy::indexing_slicing** _(allowed)_: Unchecked `x[i]`/`&s[a..b]` are allowed here.
- **clippy::arithmetic_side_effects**: Avoid overflow/underflow/div-by-zero surprises; use checked/saturating ops where needed.

  ```rust
  // Bad: may overflow
  let x = a + b;
  // Good:
  let x = a.checked_add(b).ok_or(Error)?;
  ```

- **clippy::integer_division**: Avoid `iN/jN` when you meant float; cast or document intent.

  ```rust
  // Bad:
  let r = 1/2; // 0
  // Good:
  let r = 1.0f32/2.0;
  ```

- **clippy::lossy_float_literal**: Don’t write high-precision literals into low-precision types; suffix or cast correctly.
- **clippy::dbg_macro**: Don’t ship `dbg!(...)`; remove or gate with debug cfg.
- **clippy::empty_drop**: Don’t call `drop(&x)` (no effect); use `drop(x)` or `_ = x`.
- **clippy::exit**: Avoid `std::process::exit` in libs; return errors instead.
- **clippy::filetype_is_file**: Don’t use `Path::is_file()` to check types portably; prefer metadata checks where appropriate.
- **clippy::float_cmp_const**: Avoid `==`/`!=` to a float const; use epsilon comparisons.
- **clippy::get_unwrap**: Don’t chain `.get(...).unwrap()`; use indexing or `ok_or`/`copied().ok_or`.
- **clippy::shadow_unrelated**: Don’t re-use variable names for unrelated values; pick distinct names.
- **clippy::string_slice**: Avoid slicing `String` by byte indices; use `&str` APIs/iterators.

  ```rust
  // Bad:
  let s = "éé".to_string(); let _ = &s[0..1];
  // Good:
  let ch = s.chars().next();
  ```

- **clippy::try_err**: Don’t write `Err(e)?`; return `Err(e)` or use `?` on a `Result` expression directly.
- **clippy::clone_on_ref_ptr**: Don’t `clone()` `Rc/Arc` inner `T`; clone the smart pointer instead.
- **clippy::create_dir**: Don’t assume directory absent; prefer `create_dir_all` or handle `AlreadyExists`.
- **clippy::decimal_literal_representation**: Be consistent/clear with decimal literals; avoid ambiguous underscores/cases.
- **clippy::deref_by_slicing**: Don’t use `&*v` via slicing tricks; use `&*v` or `&v[..]` idiomatically, not redundant forms.

  ```rust
  // Bad:
  let _ = &v[..][..];
  // Good:
  let _ = &v[..];
  ```

- **clippy::float_arithmetic**: Avoid unchecked float math where determinism matters; prefer integers/fixed-point or justify.
- **clippy::if_then_some_else_none**: Replace `if { Some(..) } else { None }` with `.then(|| ..)`.
- **clippy::let_underscore_must_use**: Don’t discard `#[must_use]` results with `_`; handle or explicitly `let _ = ...;` only if justified.
- **clippy::map_err_ignore**: Don’t map to an ignored error; propagate or log meaningful context.
- **clippy::mem_forget**: Don’t leak memory with `mem::forget`; use `ManuallyDrop`/ownership patterns if needed.
- **clippy::missing_assert_message**: Provide messages on `assert!` to aid failures.
- **clippy::mod_module_files**: Don’t use `mod.rs`; use self-named files/dirs for modules.
- **clippy::partial_pub_fields**: Avoid structs with mixed pub/private fields; prefer ctor methods or all-private with getters.
- **clippy::pub_use**: Don’t re-export with `pub use` in confusing ways; keep clear module boundaries or `pub(crate) use`.
- **clippy::rc_buffer**: Avoid `Rc<Vec/Box<[u8]>>` when `Vec`/`Arc<[u8]>` is better; pick ownership wisely.
- **clippy::rc_mutex**: Don’t use `Rc<Mutex<_>>`; use `Arc<Mutex/_>` for threads or plain `RefCell` for single-thread.
- **clippy::ref_patterns**: Avoid needless `ref` in patterns; bind by reference with `&` or by move as idiomatic.
- **clippy::rest_pat_in_fully_bound_structs**: Don’t use `..` when all fields are bound; remove redundant rest.

  ```rust
  let S { a, b, .. } = s; // Bad
  let S { a, b } = s;     // Good
  ```

- **clippy::same_name_method**: Don’t define methods with same name but unrelated semantics; avoid confusion/overlaps.
- **clippy::single_char_lifetime_names**: Avoid lifetimes like `'a` when clearer names help; prefer descriptive lifetimes in pub APIs.
- **clippy::str_to_string**: Avoid `"x".to_string()` when `"x".to_owned()`/`String::from` is clearer or unnecessary.
- **clippy::string_add**: Avoid `String` `+ &str` chains; use `push_str`, `format!`, or `write!`.
- **clippy::string_to_string**: Don’t call `to_string()` on a `String`; it’s a no-op—use `.clone()` or move it.
- **clippy::suspicious_xor_used_as_pow**: Don’t use `^` as exponent; it’s XOR—use `pow`.

  ```rust
  // Bad:
  let x = 2 ^ 4;
  // Good:
  let x = 2u32.pow(4);
  ```

- **clippy::tests_outside_test_module**: Keep tests in `#[cfg(test)] mod tests` or test files; avoid stray test fns.
- **clippy::unseparated_literal_suffix**: Separate numeric value and suffix clearly (e.g., `1i32`); avoid confusing forms.
- **clippy::use_debug**: Don’t format for user output with `{:?}`; use `{}` or implement `Display`.
- **clippy::verbose_file_reads**: Don’t hand-roll file-to-string; use `fs::read_to_string`.
- **clippy::absolute_paths** _(allowed)_: Absolute paths like `crate::`/`::std` are OK here.
- **clippy::allow_attributes**: Don’t blanket `#[allow(...)]`; scope narrowly.
- **clippy::allow_attributes_without_reason**: Every `#[allow]` needs a brief reason; avoid unexplained allows.
- **clippy::as_conversions**: Avoid lossy `as` casts; use `TryFrom`/`from` or checked conversions.

  ```rust
  // Bad:
  let y = big as u8;
  // Good:
  let y = u8::try_from(big)?;
  ```

- **clippy::assertions_on_result_states** _(allowed)_: Asserting on `Result` states isn’t restricted here.
- **clippy::big_endian_bytes**: Don’t assume BE order; use `to_be_bytes`/`from_be_bytes`.
- **clippy::default_numeric_fallback**: Avoid relying on type fallback for numbers; annotate types/suffixes.
- **clippy::empty_enum_variants_with_brackets / empty_structs_with_brackets**: Don’t write `Variant()`/`Struct()` for unit items; use `Variant`/`Struct`.
- **clippy::error_impl_error**: Don’t implement `std::error::Error` incorrectly; include `source`/`Display` as needed.
- **clippy::exhaustive_enums/structs** _(allowed)_: Public exhaustiveness is allowed here.
- **clippy::fn_to_numeric_cast_any**: Don’t cast fn pointers to integers; use `fn as *const _` carefully or avoid entirely.
- **clippy::format_push_string**: Don’t build strings via `format!` then `push_str`; either `format!` once or push pieces.
- **clippy::host_endian_bytes / little_endian_bytes**: Don’t assume platform endianness; use explicit `to_*_bytes`.
- **clippy::infinite_loop**: Avoid `loop {}` without `break`/sleep; ensure a termination or wait.
- **clippy::large_include_file**: Don’t `include_str!` huge files; load at runtime or compress.
- **clippy::let_underscore_untyped**: Don’t write `let _ = expr` where type matters; bind or annotate.
- **clippy::macro_use_imports**: Prefer path macros (`crate::m!`) over `#[macro_use]`.
- **clippy::min_ident_chars** _(allowed)_: Short identifiers are permitted by policy.
- **clippy::missing_trait_methods** _(allowed)_: Not requiring all trait methods is OK here.
- **clippy::mixed_read_write_in_expression**: Don’t read and mutate the same value in one expr; split steps.

  ```rust
  // Bad:
  v[i] = v[i] + 1;
  // Good:
  let val = v[i]; v[i] = val + 1;
  ```

- **clippy::multiple_inherent_impl**: Avoid many inherent `impl` blocks per type; group logically.
- **clippy::mutex_atomic**: Don’t wrap atomics in a mutex; use the atomic type alone or a plain mutex.
- **clippy::needless_raw_strings**: Don’t use raw string literals when normal strings suffice.
- **clippy::panic_in_result_fn**: Functions returning `Result` shouldn’t panic; return `Err` instead.

  ```rust
  // Bad:
  fn f()->Result<(),E>{ panic!("oops") }
  // Good:
  fn f()->Result<(),E>{ Err(E::Oops) }
  ```

- **clippy::pattern_type_mismatch**: Match pattern types correctly; avoid `if let Some(x): Option<u8> = y`.
- **clippy::print_literal**: Don’t use `println!("{:?}", "x")` literal prints; remove debug prints.
- **clippy::redundant_type_annotations**: Don’t annotate types the compiler can infer; keep code lean.
- **clippy::renamed_function_params**: Don’t rename params inconsistently in docs/sigs breaking expectations.
- **clippy::semicolon_inside_block**: Don’t put stray semicolons that change block expr values.
- **clippy::semicolon_outside_block** _(allowed)_: Style leniency on outer semicolons is fine here.
- **clippy::std_instead_of_alloc**: Don’t use `std` types in `no_std+alloc` contexts; use `alloc`.
- **clippy::std_instead_of_core** _(allowed)_: Using `std` over `core` is OK here.
- **clippy::tuple_array_conversions**: Avoid manual tuple↔array conversions; use `From`/`into` helpers.

  ```rust
  let a: [u8;3] = (1,2,3).into(); // Good
  ```

- **clippy::unneeded_field_pattern**: Don’t bind fields you don’t use; use `..` or `_`.

  ```rust
  // Bad:
  let S { a, b } = s; let _ = a;
  // Good:
  let S { a, .. } = s;
  ```

- **clippy::unwrap_in_result**: Don’t `unwrap()` inside fns that return `Result`; propagate errors with `?`.

  ```rust
  // Bad:
  fn f()->Result<u8,E>{ Ok(opt.unwrap()) }
  // Good:
  fn f()->Result<u8,E>{ opt.ok_or(E::Missing)? }
  ```

# Steps

Follow these steps to fix the linting issues.

- KEEP THE LINTING RULES IN MIND (STATED ABOVE). THIS IS ABSOLUTELY VITAL AND WILL DEFINITELY PREVENT YOU FROM COMPLETING YOUR WORK IF YOU DO NOT HEED IT. LOOK AT THE LINTING RULES IN Makefile.toml BEFORE CONTINUING SO YOU KNOW WHAT TO AVOID.
- Fix any linting errors, running the test suite often to ensure no issues are occurring. Follow error-handling best practices - maintainability is the end goal. _Prefer making changes to entire sections of each file_ - individual replacements take a lot of time. For example, if there are a lot of lint issues in the last 500 lines of a file, replace all of those lines at the same time rather than fixing each individual lint issue. For large documentation tasks or tasks that will span small changes across an entire file, tell me to invoke Claude Code and what prompt you will need, and then run an echo statement (which will pause the editor for me) and I will do it. DO NOT LISTEN TO CLIPPY'S HELP SECTIONS - THEY WILL MISLEAD YOU FREQUENTLY. SINCE YOU ARE AN EXPERT IN CLIPPY, YOU KNOW WHAT TO DO
- Run the tests and build the application once you have finished fixing the lint issues.
- Stage all relevant items and commit them. The commit process will run all tests automatically as well as the linter and will reject your commit if either do not pass. **YOU ARE NOT ALLOWED TO USE --no-verify!**

You do not stop until there are no linting issues remaining.
