# Opalescent

Opalescent is an experimental, statically typed programming language with explicit error handling, algebraic data types, native-code compilation through LLVM, and a small standard library focused on practical command-line programs.

If you already know what you are doing:

```bash
export LLVM_SYS_140_PREFIX=/usr/lib/llvm-14
cargo build --release
./target/release/opalescent run test-projects/hello-world/src/main.op
```

If you are new to language projects or to Opalescent, start with [OPALESCENT_CRASH_COURSE.md](OPALESCENT_CRASH_COURSE.md). It explains the language from the ground up and points at working programs in `test-projects/`. For standard library functions, see [STDLIB.md](STDLIB.md).

## Project status

Opalescent is early software. Many pieces work end-to-end, but the language and library are still changing. The table below separates features that are implemented from features that are only planned or proposed. This list is based on the parser, type checker, code generator, standard-library registration, and integration tests in this repository.

### Implemented and tested

- [x] `.op` source files and `.types.op` type-definition files
- [x] `entry main` program entry points
- [x] `let` bindings, `mutable` bindings, assignment, and explicit `return`
- [x] Functions declared with `f(...)` signatures
- [x] Primitive types: `boolean`, `string`, `void`, signed/unsigned integer widths, and floating-point widths
- [x] Algebraic data types: product types, sum types, enums, and recursive types
- [x] Generic type syntax and selected generic surfaces such as `Weak<T>` and standard-library array helpers
- [x] String interpolation with single-quoted strings such as `'Hello {name}'`
- [x] Arrays with `.length`, indexing, `push`, `pop`, `map`, `filter`, `reduce`, `zip`, and related helpers
- [x] Algebraic data type parsing/type work and `is`-based ADT/value checks in fixtures
- [x] `if`, `while`, `for`, `while true`, `continue`, and the fixture-backed `loop => ... break name: value` expression form
- [x] Fallible functions with `errors ...` clauses
- [x] `propagate`, `guard ... else`, and `guard ... into ... else` error handling
- [x] Importing functions and types from other files
- [x] Standard-library I/O, string, bytes, filesystem, terminal, stdout-writer, and time/frame-clock symbols
- [x] Dedicated `Bytes` values with hex conversion, concatenation, slicing, and `.length`
- [x] Native compilation, `opal run`, `opal check`, `opal build`, `opal fmt`, `opal doc`, `opal test`, `opal bench`, `opal watch`, and `opal lsp --stdio`
- [x] Linux native builds and Windows MSVC target support through the documented cross-compilation path
- [x] VS Code syntax highlighting and LSP wiring under `vscode-extension/`

### Implemented but still experimental

- [x] Package-manager help topics exist, but `opal pkg` command execution currently reports `not yet implemented`.
- [x] Hot-reload components exist in the codebase, but public production guarantees are still being defined.
- [x] Windows/Wine validation exists in tests and scripts, but host setup is more involved than the Linux quick start.

### Planned or proposed, not finished

- [ ] Regex standard-library module (`stdlib-proposals/regex/`)
- [ ] Crypto hashing standard-library module (`stdlib-proposals/crypto-hashing/`)
- [ ] HTTP/network standard-library layer (`stdlib-proposals/network-http-layer/`)
- [ ] Subprocess execution API (`stdlib-proposals/subprocess-exec/`)
- [ ] JSON/TOML serialization APIs (`stdlib-proposals/serialization/`)
- [ ] Compression APIs (`stdlib-proposals/compression/`)
- [ ] UUID APIs (`stdlib-proposals/uuid/`)
- [ ] Rich testing DSLs such as `describe`/`it`, snapshots, and property tests (`stdlib-proposals/testing-framework/`)
- [ ] Final memory-model design beyond the current reference-counted runtime (`memory-model-proposals/`)
- [ ] Final public package registry and publishing flow

## Repository layout

```text
src/                  Rust implementation of the compiler and tools
runtime/              C runtime used by generated programs
stdlib/               Documentation-oriented prelude signatures
tests/                Rust integration tests for the compiler
test-projects/        Small Opalescent projects used as examples and integration fixtures
language-spec/        Current language notes and sample programs
stdlib-proposals/     Draft standard-library design proposals
memory-model-proposals/ Draft memory-management design proposals
vscode-extension/     VS Code grammar and extension scaffolding
scripts/              Development and verification scripts
```

`test-projects/` is intentionally part of the public repo: each project is both a regression fixture and an example you can study.

## Requirements

- Rust toolchain with edition 2024 support.
- LLVM 14 development libraries. On Debian/Ubuntu this is usually `llvm-14` and related development packages.
- `LLVM_SYS_140_PREFIX` set to the LLVM 14 installation prefix.
- A C toolchain/linker for your host platform.

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

## Build and run

Build the compiler:

```bash
cargo build --release
```

Run a single source file:

```bash
./target/release/opalescent run test-projects/hello-world/src/main.op
```

Type-check without generating a binary:

```bash
./target/release/opalescent check test-projects/hello-world/src/main.op
```

Build a project from a directory containing `opal.toml` and `src/main.op`:

```bash
cd test-projects/hello-world
../../target/release/opalescent build
```

Format a source file:

```bash
./target/release/opalescent fmt --check test-projects/hello-world/src/main.op
```

Generate Markdown docs from a source file:

```bash
./target/release/opalescent doc test-projects/hello-world/src/main.op
```

## CLI reference

| Command | Status | Purpose |
|---|---:|---|
| `opal <file.op>` | working | Compile one source file and print the generated binary path. |
| `opal <file.op> --run` | working | Compile and run one source file. |
| `opal run <file.op> [-- args...]` | working | Compile and run one source file, forwarding args after `--`. |
| `opal run` | working | Compile and run the current `opal.toml` project. |
| `opal check <file.op>` | working | Lex, parse, validate file role, and type-check. |
| `opal build` | working | Build the current `opal.toml` project. |
| `opal fmt [--check] [--config <path>] [--output <path>] <file>` | working | Format Opalescent source. |
| `opal doc <file.op>` | working | Generate Markdown documentation from doc comments. |
| `opal test [--filter <pattern>] [--target <triple>]` | working | Run the project test command. |
| `opal bench` | working | Run built-in compiler benchmark probes. |
| `opal watch <file.op>` | working | Recompile and run when a source file changes. |
| `opal lsp --stdio` | working | Start the language server transport. |
| `opal pkg <command>` | planned | Help exists; command execution is not implemented yet. |

## Windows targets

Opalescent can target Windows MSVC from Linux when the Windows SDK is available through `xwin` and `clang-cl`/`lld-link` are configured. The short version is:

```bash
cargo install xwin --locked
xwin --accept-license splat --output ~/.xwin
export LLVM_SYS_140_PREFIX=/usr/lib/llvm-14
export XWIN_CACHE=$HOME/.xwin
export OPAL_XWIN_SYSROOT=$HOME/.xwin
export OPAL_MSVC_CC=$LLVM_SYS_140_PREFIX/bin/clang-cl
```

Then build an Opalescent project with:

```bash
opal build --target x86_64-pc-windows-msvc
```

Use the scripts and Windows integration tests as the source of truth when changing this area.

## Documentation

Top-level Markdown files are intended to answer different questions:

| File | What it is for |
|---|---|
| [README.md](README.md) | Project overview, status, requirements, build/run commands, and the feature checklist. |
| [OPALESCENT_CRASH_COURSE.md](OPALESCENT_CRASH_COURSE.md) | Beginner-friendly language tutorial, local `opal` alias setup, and common language footguns. |
| [STDLIB.md](STDLIB.md) | User-facing standard-library reference with function behavior, signatures, and error notes. |
| [CONTRIBUTING.md](CONTRIBUTING.md) | Contributor setup, architecture map, testing strategy, documentation rules, and PR checklist. |
| [ARRAY_FEATURES.md](ARRAY_FEATURES.md) | Notes on implemented array behavior and array-related language/runtime support. |
| [ERROR_HANDLING_STANDARDS.md](ERROR_HANDLING_STANDARDS.md) | Error-handling design expectations and implementation standards. |
| [HOT_RELOAD_ARCHITECTURE.md](HOT_RELOAD_ARCHITECTURE.md) | Hot-reload subsystem architecture and design notes. |
| [REFACTORING_GUIDE.md](REFACTORING_GUIDE.md) | Guidance for safe refactors in the compiler codebase. |
| [SECURITY.md](SECURITY.md) | How to report security-sensitive issues. |
| [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md) | Community behavior expectations and enforcement. |
| [GAME_OF_LIFE_POST_MORTEM.md](GAME_OF_LIFE_POST_MORTEM.md) | Historical Game of Life debugging notes, if retained in the branch. |

Design proposal directories such as `stdlib-proposals/`, `memory-model-proposals/`, `error-handler-proposals/`, and `game-of-life-proposals/` contain draft options. They are useful context, but they are not finished API guarantees.

## Contributing

Contributions are welcome. Please read [CONTRIBUTING.md](CONTRIBUTING.md) before opening issues or pull requests.

## License

A final open-source license has not been selected in this branch. Add a real `LICENSE` file before publishing the repository publicly.
