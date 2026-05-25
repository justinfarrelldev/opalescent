# Contributing to Opalescent

Thank you for helping with Opalescent. This repository contains a compiler, runtime, standard library, editor tooling, proposals, and integration fixtures. The project is early, so good contributions usually include code, tests, and documentation together.

## Quick start for contributors

### Debian/Ubuntu setup

On a fresh Debian or Ubuntu machine, install the native compiler toolchain, LLVM 14, and multilib support used by the cargo-make tasks and runtime/linker tests:

```bash
sudo apt-get update
sudo apt-get install -y \
    build-essential \
    clang \
    lld \
    llvm-14 \
    llvm-14-dev \
    libclang-14-dev \
    gcc-multilib \
    g++-multilib \
    pkg-config
```

Install Rust with `rustup` if it is not already available:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
. "$HOME/.cargo/env"
rustup default stable
```

Then build from the repository root:

```bash
export LLVM_SYS_140_PREFIX=/usr/lib/llvm-14
cargo build --release
./target/release/opalescent run test-projects/hello-world/src/main.op
cargo test
```

Optional contributor tools:

```bash
cargo install cargo-make
cargo install cargo-tarpaulin
```

`cargo-make` enables tasks from `Makefile.toml`, including `cargo make install-deps-debian`, `cargo make lint`, and `cargo make wine-tests`. `cargo-tarpaulin` is only needed for coverage tasks.

The Cargo binary is named `opalescent`. Most public examples use `opal` as the command name because that is the intended user-facing CLI. During local development, create an alias or shell function so examples stay short:

```bash
alias opal="$PWD/target/release/opalescent"
```

If you rebuild often and want the alias to pick up the debug binary instead:

```bash
alias opal="$PWD/target/debug/opalescent"
```

Run a file directly:

```bash
opal run test-projects/hello-world/src/main.op
```

Run a project without typing `src/main.op` each time:

```bash
cd test-projects/hello-world
opal run
```

`opal run` and `opal build` look for `opal.toml` in the current directory and use that project's `src/main.op`.

## Requirements

- Rust toolchain with edition 2024 support.
- LLVM 14 development libraries.
- `LLVM_SYS_140_PREFIX` pointing at the LLVM 14 installation prefix.
- A C toolchain and linker for your host platform.

Linux example:

```bash
export LLVM_SYS_140_PREFIX=/usr/lib/llvm-14
cargo build --release
```

macOS example, assuming Homebrew LLVM 14:

```bash
export LLVM_SYS_140_PREFIX="$(brew --prefix llvm@14)"
cargo build --release
```

Windows/MSVC cross-compilation from Linux is possible, but Windows development is not stable at the current time. It is expected to stabilize in the future. Today it needs extra setup through `xwin`, `clang-cl`, `lld-link`, and Wine, and the related tests are slower and more host-sensitive than the Linux path. Use `scripts/verify-wine-prereqs.sh`, `scripts/msvc_link_probe.sh`, and the `wine-tests` cargo-make task as references before changing that area.

## Note on GitHub Actions

Right now, the GitHub Actions are failing. This will be fixed when I upgrade LLVM.

## Repository architecture

Opalescent is implemented mostly in Rust, with a C runtime linked into generated programs.

| Area | Main files | What lives there |
|---|---|---|
| CLI | `src/main.rs`, `src/app.rs` | Command-line dispatch for `run`, `check`, `build`, `fmt`, `doc`, `test`, `bench`, `watch`, `lsp`, and package-manager help. |
| Lexer/parser/AST | `src/lexer/`, `src/parser/`, `src/ast/`, `src/token.rs` | Source text, tokens, parse tree, declarations, imports, expressions, and statements. |
| Type system | `src/type_system.rs`, `src/type_system/` | Type checking, module interfaces, generic solving, ADT validation, imports, purity checks, and error handling rules. |
| Codegen | `src/codegen.rs`, `src/codegen/` | LLVM lowering, runtime calls, array lowering, error ABI lowering, monomorphization, and cleanup logic. |
| Runtime | `runtime/*.c`, `runtime/*.h`, `src/runtime/` | C helpers linked into generated binaries and Rust-side runtime tests/helpers. |
| Standard library surface | `stdlib/prelude.op`, `src/type_system/module_resolver/`, `src/codegen/functions_stdlib.rs`, `runtime/` | User-visible symbols, type registration, codegen declarations, and runtime implementations. |
| Build system | `src/build_system/`, `Makefile.toml`, `scripts/` | Native linking, target triples, package/build config, cache helpers, and cross-platform scripts. |
| Tooling | `src/formatter/`, `src/doc_gen/`, `src/lsp/`, `vscode-extension/` | Formatter, documentation generation, language server, and editor integration. |
| Tests and fixtures | `src/**/tests.rs`, `tests/`, `test-projects/` | Rust unit tests, integration/e2e tests, and Opalescent programs used as examples and regression fixtures. |
| Design notes | `language-spec/`, `stdlib-proposals/`, `memory-model-proposals/`, `error-handler-proposals/`, `game-of-life-proposals/` | Current requirements and proposal material. Proposal directories are not finished API promises. |

## How the compiler pieces interface

The architecture table above tells you where code lives. This section explains how the pieces actually call each other during a normal compile/run.

### Compile pipeline diagram

```text
opal CLI
  |
  v
src/app.rs
  |  parses command-line intent: run/check/build/fmt/doc/test/bench/watch/lsp
  v
src/compiler.rs or tool-specific module
  |
  |-- run/check/build path -----------------------------------------------.
  |                                                                       |
  v                                                                       |
module_loader                                                             |
  |  resolves imports, validates .op vs .types.op file roles              |
  v                                                                       |
lexer -> parser -> AST                                                    |
  |      |        |                                                       |
  |      |        `-- src/ast/ defines declarations, expressions, types    |
  |      `----------- src/parser/ turns tokens into Program                |
  `------------------ src/lexer/ turns source into tokens                 |
  v                                                                       |
type_system                                                               |
  |  builds module interfaces, registers standard symbols, checks types    |
  |  and validates error propagation                                      |
  v                                                                       |
codegen                                                                   |
  |  lowers checked AST to LLVM IR, declares stdlib/runtime symbols,       |
  |  emits cleanup and error-ABI paths                                    |
  v                                                                       |
build_system + runtime                                                    |
  |  writes object files, materializes embedded C runtime, invokes linker  |
  v                                                                       |
native executable <-------------------------------------------------------'
```

### Module responsibilities and hand-offs

- `src/app.rs` is the CLI dispatcher. `run_with_args` decides whether to call `compile_program`, `compile_project`, `run_check_command`, `run_fmt_command`, `run_doc_command`, `run_test_command`, `run_bench_command`, `run_watch_command`, or the LSP path.
- `src/compiler.rs` owns the source-to-native pipeline. `compile_program` handles single-file builds; `compile_project` handles `opal.toml` projects; both delegate to run-policy variants so tests can use bounded subprocess timeouts.
- `compile_to_module_for_target` is the single-file front-end path: it normalizes tabs, calls `Lexer::new(...).tokenize()`, calls `Parser::new(...).parse()`, validates the file role, calls `TypeChecker::type_check_program`, creates `CodegenContext::for_triple`, and lowers declarations.
- `compile_project_with_run_policy` is the multi-file path: it reads `opal.toml`, asks `ModuleLoader::discover_all_modules` for dependency order, parses each module with `parse_source_to_program`, validates entries with `validate_entry_declarations_for_module`, registers module interfaces, codegens non-`.types.op` modules, emits one object per module, and links them.
- `src/module_loader.rs` resolves imports and enforces file-role rules. `resolve_import_path` handles local imports and the `standard` sentinel; `validate_module_file_role` is why `.types.op` files are treated differently from ordinary `.op` files.
- `src/lexer/` produces tokens and lexical diagnostics. It should not know about type rules or runtime behavior.
- `src/parser/` consumes tokens and builds `ast::Program`. It should validate syntax shape, not semantic correctness.
- `src/ast/` defines the shared data structures passed from parser to type checker and codegen.
- `src/type_system/` validates semantics. `TypeChecker` registers standard module symbols, resolves module interfaces, checks ADTs/generics/purity/errors, and rejects invalid programs before codegen.
- `src/type_system/module_resolver.rs` carries cross-module interfaces: exported symbols, private symbols, ADT fields, and standard-module symbols that imports can see.
- `src/codegen/` assumes the program has been type-checked. It lowers AST nodes to LLVM IR, manages runtime calls, handles arrays/strings/ADTs, emits cleanup logic, and uses monomorphization helpers for generic functions.
- `src/codegen/functions.rs` lowers function and import declarations. `src/codegen/functions_stdlib.rs` bridges language-level standard symbols to runtime function declarations. If you add a stdlib function, this file is usually involved.
- `emit_object_file` writes LLVM modules to `.o`/`.obj` files through Inkwell's target machine.
- `runtime/*.c` implements helpers used by generated programs: printing, parsing, bytes, strings, filesystem operations, reference counting, errors, and terminal/time helpers. `RuntimeTempFile` materializes the embedded runtime for native linking.
- `src/build_system/targets.rs` describes target triples and file naming; `src/build_system/linker.rs` chooses and configures platform linkers. `link_object_files_with_policy` ties those pieces to the emitted object files.
- `src/formatter/`, `src/doc_gen/`, `src/lsp/`, and `src/testing/` are sibling tooling paths. They reuse lexer/parser/type information where needed, but they do not all run the full native-code pipeline.

### Standard-library hand-off

A user-visible standard-library function normally flows through four layers:

```text
stdlib/prelude.op documentation
  -> src/type_system/module_resolver/ signature registration
  -> src/codegen/functions_stdlib.rs runtime declaration/import resolution
  -> runtime/*.c or codegen lowering implementation
```

If any layer is missing, users may see confusing behavior: an import may type-check but fail at codegen, or a runtime helper may exist but be impossible to import. Keep these layers in sync.

### Tooling paths that do not produce a binary

```text
opal check: source -> lexer -> parser -> module role validation -> type_system
opal fmt:   source -> formatter parser/printer path -> formatted source
opal doc:   source -> lexer -> parser -> doc_gen Markdown renderer
opal lsp:   editor transport -> lsp server modules -> parser/type information over time
opal test:  project test runner, not the Rust `cargo test` harness
```

These commands share pieces with compilation, but they intentionally stop before codegen or linking.

## Where to make changes

Start by finding the layer that owns the behavior.

- Syntax change: update the lexer/parser/AST, formatter, VS Code grammar, parser tests, and crash course examples.
- Type rule change: update `src/type_system/`, type-system tests, and at least one small user-visible fixture when appropriate.
- Codegen change: update `src/codegen/`, runtime declarations, e2e tests, and any cleanup/ownership tests the change affects.
- Runtime change: update `runtime/`, Rust runtime tests, codegen declarations if symbols change, and e2e tests that compile and run `.op` programs.
- Standard-library change: update type registration, codegen declarations, runtime implementation, `stdlib/prelude.op`, `STDLIB.md`, and tests.
- CLI/tooling change: update `src/app.rs`, relevant tool modules, README command tables, and CLI tests.
- Documentation-only change: prefer examples that already exist under `test-projects/` or mark examples as illustrative.

When a feature crosses layers, update the layers together. A new standard-library function commonly requires all of these:

1. Type registration under `src/type_system/module_resolver/`.
2. Codegen declaration/import resolution in `src/codegen/functions_stdlib.rs`.
3. Runtime implementation in `runtime/` or lowering support in `src/codegen/`.
4. Documentation in `stdlib/prelude.op` and `STDLIB.md`.
5. Unit tests plus an e2e fixture when user-visible.

## Test strategy

Run the narrowest useful test first, then widen the gate.

```bash
cargo test
cargo test -- --nocapture
cargo test --features integration
```

Useful targeted examples:

```bash
cargo test parser::tests::test_simple_function_parsing
cargo test type_system::tests::test_builtin_bytes_from_hex_type_checks_under_propagate
cargo test --test integration_e2e -- --nocapture fs_markdown_roundtrip
```

Some integration tests compile and execute Opalescent programs. They are slower and can depend on platform tooling. Use temporary directories in tests rather than writing into fixture directories.

### When to add which test

- Lexer/parser behavior: Rust unit tests near the parser plus formatter tests if syntax printing changes.
- Type checking: type-system unit tests and a small `.op` fixture for user-facing rules.
- Codegen/runtime behavior: e2e tests that compile and run an Opalescent program.
- Standard-library APIs: type-registration tests, runtime tests, and at least one integration fixture for the public surface.
- CLI behavior: tests around `src/app.rs` or integration tests that invoke the compiled binary.

Do not delete failing tests just to make a branch green. If a test is wrong because the language changed, update it and explain why in the PR.

## Formatting and linting

Rust formatting:

```bash
cargo fmt --check
cargo fmt
```

Opalescent source formatting:

```bash
opal fmt test-projects/hello-world/src/main.op
opal fmt --check test-projects/hello-world/src/main.op
```

The formatter is config-driven; do not document a universal indentation width unless you are quoting the active formatter configuration or generated output.

Clippy is intentionally strict in `Makefile.toml`:

```bash
cargo make lint
```

If `cargo make` is unavailable, run a narrower local check first:

```bash
cargo clippy --all-targets --all-features -- -D warnings
```

## Common footguns

- The local binary is `opalescent`, not `opal`. Use an alias while developing.
- `LLVM_SYS_140_PREFIX` must point to LLVM 14. A different LLVM version can fail at build time or link time.
- `.types.op` files are type-definition modules. They may contain type declarations and type imports, not `let` values or functions. Violations are reported as `TypeError::NonTypeDeclarationInTypesFile`.
- Regular `.op` files should not declare types. Violations are reported as `TypeError::TypeDeclarationOutsideTypesFile`.
- Type imports are still maturing. `import type ... from ./name.types` works for current fixture patterns, but imported `.types.op` names are not yet reliable in every function-signature position. Keep public signatures close to their defining type module or use existing fixture patterns until this is fully supported.
- Relative module imports omit the `.op` extension for regular modules and include `.types` for type modules, for example `import create_seed_board from ./patterns` and `import type LifeConfig from ./life.types`.
- Guard/error handling is strict. Returning aliases or wrapper values from guard handlers can be rejected when the checker cannot prove the original error is propagated safely.
- `opal pkg` has help text, but command execution currently returns `not yet implemented`.
- Proposal directories contain design options, not finished guarantees. Do not move an item from planned to implemented in the README unless parser/typechecker/codegen/runtime/tests support it.
- `test-projects/` is both examples and regression input. Do not edit it casually, and never commit generated binaries or local target artifacts.
- Windows/Wine tests need local prerequisites and are slower than ordinary `cargo test`.

## Documentation expectations

Update public docs when you change:

- Language syntax or examples: `OPALESCENT_CRASH_COURSE.md`, `README.md`, and sometimes `language-spec/`.
- CLI commands or flags: `README.md`, `CONTRIBUTING.md`, and command help tests.
- Standard-library names, types, errors, or behavior: `STDLIB.md`, `stdlib/prelude.op`, type registration, and runtime tests.
- Runtime or memory behavior: `README.md`, `memory-model-proposals/`, and relevant architecture notes.
- Hot reload: `HOT_RELOAD_ARCHITECTURE.md`.
- Error handling conventions: `ERROR_HANDLING_STANDARDS.md`.

`README.md` should remain a map and quick start. Put tutorial material in `OPALESCENT_CRASH_COURSE.md`, standard-library behavior in `STDLIB.md`, and contributor workflow detail here.

## Pull request checklist

Before opening a PR, check:

- [ ] The branch has a focused purpose.
- [ ] Public docs are updated if public behavior changed.
- [ ] Tests cover the behavior or the PR explains why tests are not appropriate.
- [ ] `cargo fmt --check` passes, or the PR clearly explains pre-existing formatting drift.
- [ ] Relevant `cargo test ...` commands pass.
- [ ] `cargo test --features integration` was run for user-visible compiler/runtime changes, or the PR explains why not.
- [ ] No generated logs, local binaries, private workflow artifacts, or accidental fixture build outputs are included.
- [ ] Any `test-projects/` changes are intentional and explained.

## Reporting bugs

A useful bug report includes:

- The smallest Opalescent source that triggers the problem.
- The command you ran.
- Full compiler output.
- Your operating system, Rust version, LLVM version, and Opalescent commit.
- Whether the problem is a parse error, type error, codegen failure, runtime crash, wrong output, formatter issue, or documentation mismatch.

If the issue may be security-sensitive, do not open a public issue. Follow `SECURITY.md` instead.
