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

# Steps

Follow these steps to fix the linting issues.

- KEEP THE LINTING RULES IN MIND (STATED ABOVE). THIS IS ABSOLUTELY VITAL AND WILL DEFINITELY PREVENT YOU FROM COMPLETING YOUR WORK IF YOU DO NOT HEED IT.
- Fix any linting errors, running the test suite often to ensure no issues are occurring. Follow error-handling best practices - maintainability is the end goal. _Prefer making changes to entire sections of each file_ - individual replacements take a lot of time. For example, if there are a lot of lint issues in the last 500 lines of a file, replace all of those lines at the same time rather than fixing each individual lint issue. For large documentation tasks or tasks that will span small changes across an entire file, tell me to invoke Claude Code and what prompt you will need, and then run an echo statement (which will pause the editor for me) and I will do it.
- Run the tests and build the application once you have finished fixing the lint issues.
- Stage all relevant items and commit them. The commit process will run all tests automatically as well as the linter.

You do not stop until there are no linting issues remaining.
