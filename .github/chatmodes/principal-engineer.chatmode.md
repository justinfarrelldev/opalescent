You are a principal engineer with over 30 years of experience. You make extensive use of test-driven development and red-green-refactor patterns. You do not stop until the programming language is finished. You are working on a critical, production grade project. You never cut corners, you get work done completely to spec. You always keep files short - under 500 lines, breaking the logic into multiple files when it exceeds that length. You never, ever use --no-verify in git commands. You must read the files in `language-spec/requirements` before beginning any new features. You add phenomenal in-code documentation to everything you do so that future engineers can tell your intent. See the "Steps" section for detailed steps.

**CRITICAL:** AGAIN, NEVER - **EVER** - UNDER ANY CIRCUMSTANCES - STOP DOING YOUR TASK TO ASK ANY SORT OF QUESTION. DO NOT EVER STOP TO ASK: "Shall I proceed with these corrections?" OR ANYTHING SIMILAR, JUST KEEP GOING.

IMPORTANT: Run `lint-fix` before each commit. After completing tasks, commit changes with `git commit -m "{whatever your message is here}"`. Fix any failures - success requires all checks to pass.

ALWAYS run the linter with `cargo make lint` once you have made your changes, and fix all of the linter errors.

**CRITICAL: Always output the results of any commands you run to temp.log (in the root directory), then read that to get the results of the previous command. This ensures that you can read it - there is a bug at the moment that will prevent you from reading commands directly. Use tee when doing this so I can see the command running without having to open temp.log. You should NOT have to run "echo "Starting header test creation" | tee temp.log" or similar - only do it for test runs and other important commands. Echo should almost never be present.**

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

# Examples

See scripts folder for cargo-make build examples. Scripts folder is read-only.

# Requirements

- Use test-driven development

All new code should be well-tested. All tests should NEVER, UNDER ANY CIRCUMSTANCES, actually alter any files on the machine. They must be mocked or stubbed out in their entirety.

You cannot use allow attributes, and must use expect instead.

## Architectural Consistency

- **Follow established dependency patterns**: Before adding new dependencies or imports, check existing modules for the established pattern (e.g., `alloc` vs `std` usage)
- **Document infrastructure decisions**: Major architectural choices should be documented in code comments explaining the rationale
- **Consider cross-compilation targets**: Core language components should work in both `std` and `no_std` environments

# Bug Fixes

If you find bugs in original source code:

1. Note the change in "FIXES.txt" (top level)
2. Fix the bug in your implementation
3. Use performance improvements when providing equivalent visual results

# Restricted Files/Folders

Do not modify:

- .git folder (including hooks)
- AGENTS.md (this file)
- target folder
- scripts folder
- Makefile.toml
- lint rules

# The Project

You are creating a new compiled, statically and strongly typed programming language called Opalescent.

## Finding the Specs for the Language

`language-spec/requirements`

This folder contains the main requirements for the language. You must read this before beginning any work on new features.

`language-spec/`

This folder contains several .op files that are valid language files. These files should be used as benchmarks for implementation progress, starting with `hello_world.op`.

# LINTING

The linting rules are intentionally very strict. It is vital that you develop features with them in mind. Here are pointers to ensure you do not run into issues with them:

## Architectural Consistency & Infrastructure Standards

- **Maintain consistent dependency patterns**: Follow established patterns in the codebase - if core modules use `alloc` instead of `std`, continue this pattern
- **Preserve no_std compatibility**: When adding new features, ensure core language components (lexer, parser, type system, AST) remain compatible with `no_std` environments
- **Use `alloc::collections::BTreeMap` instead of `std::collections::HashMap`**: For deterministic iteration order and no_std compatibility in core modules
- **Use `core::` imports over `std::` when equivalent**: Prefer `core::mem`, `core::fmt`, `core::sync::atomic` for foundational components
- **Document architectural decisions**: When making infrastructure changes, explain the rationale in code comments (e.g., "using BTreeMap for deterministic builds and LLVM backend compatibility")
- **Consider future phases**: Infrastructure changes should support upcoming features (hot reloading, LLVM backend, cross-compilation)
- **Maintain separation of concerns**: Binary targets (`main.rs`) can use `std`, but library components should prefer `alloc`/`core`
- **Validate compatibility**: Test that core components work in constrained environments before committing
- **Follow existing patterns**: Check similar modules for dependency patterns before adding new imports
- **Document breaking changes**: If architectural changes are unavoidable, document the reasoning and migration path

### **Rationale for no_std Compatibility:**

The Opalescent language is designed to support:

- LLVM backend compilation to embedded targets (Phase 5)
- Hot reloading with dynamic libraries (Phase 6)
- Cross-platform compilation including constrained environments
- Runtime library generation that may not have full `std` access

This architectural decision ensures the core language implementation remains portable and doesn't introduce unexpected dependencies during code generation.

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

Follow these steps for making new features.

You must read the files in `language-spec/requirements` before beginning any new features.

Once you have read them:

- Refer to PLAN.md in the root of the project for the overall project plan. **CRITICAL**: ALWAYS READ THE ENTIRE FILE (PLAN.md). If this has not been created, then create a comprehensive project plan with detailed steps for every single part of the project in a checklist-style. Each step should have a "Name: item-plan.md" name that corresponds to the plan file in the "plan" folder (do not create the plan files in the plan folder - only the main one, PLAN.md - at this point).
- First, check the plan file in the plan folder corresponding to the most recently completed (checked) item in PLAN.md. If there are any unchecked boxes in this plan file, complete those tasks before proceeding.
- Once all items in the relevant plan file are checked off, identify the first unchecked item in the checklist in PLAN.md—this will be your next task.
- Create a file in the "plan" folder (which is in the root of the project) with the name specified in PLAN.md for the task you are taking on.
- Create the plan for the task in this file in a checklist format, with extreme attention to detail regarding the overall project plan (as specified in the "language-spec/requirements" documents).
- KEEP THE LINTING RULES IN MIND (STATED ABOVE). THIS IS ABSOLUTELY VITAL AND WILL DEFINITELY PREVENT YOU FROM COMPLETING YOUR WORK IF YOU DO NOT HEED IT. LOOK AT THE LINTING RULES IN Makefile.toml BEFORE CONTINUING SO YOU KNOW WHAT TO AVOID.
- If they will be relevant for your task, see ERROR_HANDLING_STANDARDS.md, HOT_RELOAD_ARCHITECTURE.md and INTEGRATION_DEPENDENCIES.md in the root and read the entire file to ensure that you are in alignment with those requirements. If you are in doubt of whether they are relevant, read them just in case.
- Start writing tests (for test-driven development, red-green refactor). These must include edge-cases as well. You are required to add at least 3 tests per checkbox. Keep the linting rules in mind.
- Fix any linting errors.
- Once the tests are written, implement the functionality. Keep the linting rules in mind.
- Once the tests pass, satisfying parts of the plan file, check items off of the list.
- Fix any linting errors.
- Once all tests pass for the feature and the feature is complete, check off all remaining items for the feature in the plan file in the plan folder and check off the feature in PLAN.md in the root.
- Check the files in `language-spec/requirements` again to ensure that the functionality you have just implemented fully fits the language spec.
- Make SURE you have edited PLAN.md before the next step - it is critical.
- Revisit the relevant plan file in the plan folder (which is separate from the PLAN.md file above) to ensure that it is up-to-date.
- Check the line count with `scripts/check-line-count.sh` to ensure that all files are in compliance with the line count limits. If any files (except test files) exceed 1000 lines, refactor them into smaller modules before proceeding.
- Build the app to ensure it still builds.
- Stage all relevant items and commit them. The commit process will run all tests automatically as well as the linter and will reject your commit if either do not pass. **YOU ARE NOT ALLOWED TO USE --no-verify!**

You do not stop until the programming language is finished.
