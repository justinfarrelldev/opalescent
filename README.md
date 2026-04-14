# Opalescent

A statically-typed, expression-oriented programming language with first-class error handling, algebraic data types, and an integrated toolchain.

---

## Table of Contents

- [Quick Start](#quick-start)
- [Installation](#installation)
- [CLI Reference](#cli-reference)
- [Language Basics](#language-basics)
  - [Functions](#functions)
  - [Variables](#variables)
  - [Types](#types)
  - [Pattern Matching](#pattern-matching)
  - [Error Handling](#error-handling)
  - [Generics](#generics)
- [Project Architecture](#project-architecture)
  - [Directory Layout](#directory-layout)
  - [Project Configuration (`opal.toml`)](#project-configuration-opaltoml)
  - [Package Manifest (`opal.pkg.toml`)](#package-manifest-opalpkgtoml)
  - [Source File Conventions](#source-file-conventions)
- [Compiler](#compiler)
  - [Compiler Pipeline](#compiler-pipeline)
  - [Escape Hatches](#escape-hatches)
- [Testing](#testing)
  - [Test Projects](#test-projects)
  - [Integration Tests](#integration-tests)
  - [Test Project Conventions](#test-project-conventions)
- [Package Manager](#package-manager)
- [Formatter](#formatter)
- [Language Server (LSP)](#language-server-lsp)
- [Hot Reload](#hot-reload)
- [Build Targets](#build-targets)
- [IDE Integration](#ide-integration)

---

## Quick Start

```bash
# Build the compiler
cargo build --release

# Run a source file
opal run hello_world.op

# Get help
opal help
opal --help

# Get help for a specific subcommand
./target/release/opalescent help pkg
./target/release/opalescent help fmt
```

---

## Installation

### Prerequisites

- Rust 2024 edition toolchain (`rustup` recommended)
- LLVM 14 (`llvm-14` package on Debian/Ubuntu, or Homebrew `llvm@14`)
- Set the LLVM prefix before building:

```bash
export LLVM_SYS_140_PREFIX=/usr/lib/llvm-14   # Linux
export LLVM_SYS_140_PREFIX=$(brew --prefix llvm@14)  # macOS
```

### Build

```bash
git clone https://github.com/your-org/opalescent
cd opalescent
cargo build --release
```

The binary will be at `target/release/opalescent`. You can alias it as `opal`:

```bash
alias opal="$(pwd)/target/release/opalescent"
```

---

## CLI Reference

The compiler binary is invoked as `opal`. All commands follow the pattern:

```
opal <command> [arguments]
```

### Top-level commands

| Command | Description |
|---------|-------------|
| `opal <file.op>` | Compile an Opalescent source file |
| `opal <file.op> --run` | Compile and execute an Opalescent source file |
| `opal run <file.op>` | Alias for `opal <file.op> --run` |
| `opal check <file.op>` | Typecheck source without code generation |
| `opal build` | Build the project from `opal.toml` |
| `opal watch <file.op>` | Watch a file and recompile on changes |
| `opal help` | Show the top-level help message |
| `opal --help` | Alias for `opal help` |
| `opal help <topic>` | Show help for a specific command |
| `opal pkg <command>` | Package manager commands |
| `opal fmt [options] <file>` | Format an Opalescent source file |
| `opal lsp [options]` | Start the language server |
| `opal test [options]` | Run project tests |
| `opal doc [options]` | Generate documentation |
| `opal bench` | Run benchmarks |

### `opal <file.op>` — Compile

Pass any `.op` source file as the first argument to compile it:

```bash
opal hello_world.op
opal src/main.op
```

The compiler performs lexing, parsing, and type-checking. Errors are printed with source-location context and actionable suggestions.

Pass `--run` to execute the compiled binary immediately after compilation:

```bash
opal src/main.op --run
```

### `opal fmt` — Formatter

```
opal fmt [--check] [--config <path>] <file>
```

| Flag | Description |
|------|-------------|
| `--check` | Exit with a non-zero code if the file would change (useful in CI) |
| `--config <path>` | Path to an `opal-fmt.toml` configuration file |

**Examples:**

```bash
# Format a file in-place
opal fmt src/main.op

# Check formatting without modifying (CI mode)
opal fmt --check src/main.op

# Use a custom config
opal fmt --config .opal-fmt.toml src/main.op
```

### `opal pkg` — Package Manager

```
opal pkg <command>
```

| Command | Description |
|---------|-------------|
| `opal pkg init <name>` | Initialise a new project manifest |
| `opal pkg add <pkg> <version>` | Add a dependency |
| `opal pkg remove <pkg>` | Remove a dependency |
| `opal pkg install` | Install all declared dependencies |
| `opal pkg publish` | Publish the package to the registry |

**Examples:**

```bash
opal pkg init my-project
opal pkg add opal-json >=1.0.0
opal pkg remove opal-json
opal pkg install
opal pkg publish
```

### `opal lsp` — Language Server

```
opal lsp [options]
```

Start the Opalescent language server.

| Flag | Description |
|------|-------------|
| `--stdio` | Communicate over stdin/stdout (required for editor integration) |

**Examples:**

```bash
opal lsp --stdio
```

### `opal test` — Test Runner

```
opal test [options]
```

Run tests in the current project.

| Flag | Description |
|------|-------------|
| `--target <triple>` | Run tests for a specific build target |
| `--filter <pattern>` | Only run tests whose names contain `<pattern>` |

**Examples:**

```bash
opal test
opal test --filter my_test
opal test --target x86_64-linux
```

### `opal doc` — Documentation Generator

```
opal doc [options]
```

Generate documentation for the current project.

| Flag | Description |
|------|-------------|
| `--format <md\|html>` | Output format (default: `md`) |

**Examples:**

```bash
opal doc
opal doc --format html
```

### `opal bench` — Benchmarks

```
opal bench
```

Run benchmarks in the current project.

**Examples:**

```bash
opal bench
```

### `opal run` — Compile and Execute

```
opal run <file.op> [-- args...]
```

Compile and execute an Opalescent source file.

| Flag | Description |
|------|-------------|
| `-- args...` | Arguments forwarded to the compiled binary |

**Alias:** `opal <file.op> --run`

**Examples:**

```bash
# Compile and run a file
opal run src/main.op

# Pass arguments to the program
opal run src/main.op -- --verbose --input data.json
```

### `opal check` — Typecheck

```
opal check <file.op>
```

Run lex, parse, and typecheck pipeline without code generation.

**Examples:**

```bash
opal check src/main.op
```

### `opal build` — Build Project

```
opal build
```

Build the project by reading `opal.toml` and compiling `src/main.op`.

**Examples:**

```bash
opal build
```

### `opal watch` — Watch and Recompile

```
opal watch <file.op>
```

Watch a source file and recompile on each detected change. Press Ctrl-C to stop watching.

**Examples:**

```bash
opal watch src/main.op
```

### `opal help` — Help

```bash
opal help              # Top-level usage summary
opal --help            # Alias for opal help
opal help pkg          # Package manager subcommands
opal help fmt          # Formatter flags
opal help lsp          # Language server flags
opal help test         # Test runner flags
opal help doc          # Documentation generator flags
opal help bench        # Benchmark runner usage
```


---

## Language Basics

Opalescent source files use the `.op` extension. Type definition files use the `.types.op` extension.

### Functions

Functions are first-class values declared with `let` (or `entry` for the program entry point):

```opal
##
  Description: Adds two integers together
##
let add = f(a: int32, b: int32): int32 =>
    return a + b
```

The `entry` keyword marks the single program entry point. It implies `public`, `impure`, and `untested`:

```opal
##
  Description: starting point of the application
##
entry main = f(args: string[]): void =>
    let world = 'world'
    print('Hello {world}')
    return void
```

**Function signatures:**

```
let <name> = f(<params>): <return_type> =>
    <body>

let <name> = f(<params>): <return_type> errors <ErrorType1>, <ErrorType2> =>
    <body>
```

**Doc comments** are required on all public functions. Use `##` for multi-line doc blocks:

```opal
##
  Description: Computes the nth Fibonacci number
  Returns: int32
##
let fib = f(n: int32): int32 =>
    if n <= 1:
        return n
    return fib(n - 1) + fib(n - 2)
```

### Variables

Variables are immutable by default. Use `mutable` to opt into mutability:

```opal
# Immutable — type is inferred
let message = 'hello'

# Immutable with explicit type
let count: int32 = 42

# Mutable
let mutable buffer: string[] = []
```

### Types

Algebraic data types (sum and product types) are declared in `.types.op` files:

Built-in numeric primitives are explicitly sized:

- Signed integers: `int8`, `int16`, `int32`, `int64`
- Unsigned integers: `uint8`, `uint16`, `uint32`, `uint64`
- Floating-point: `float32`, `float64`

**Product type (record):**

```opal
##
  Description: A person with a name and age
##
type Person:
    name: string
    age: int32
```

**Sum type (tagged union / enum with data):**

```opal
##
  Description: A message in a chat system
##
type Message:
    Text:
        sender: string
        body: string
    Join:
        user: string
    Leave:
        user: string
    Error:
        code: int32
        description: string
```

**Enum (no payload variants):**

```opal
##
  Description: Cardinal directions
##
type Direction:
    North
    East
    South
    West
```

**Generic types:**

```opal
##
  Description: A generic result type
##
type Result<T, E>:
    Ok:
        value: T
    Error:
        error: E
```

**Recursive types** are supported natively:

```opal
##
  Description: A binary expression tree
##
type Expr:
    Lit:
        value: int32
    Add:
        left: Expr
        right: Expr
    Neg:
        inner: Expr
```

### Pattern Matching

Use `match` to destructure sum types:

```opal
let describe = f(msg: Message): string =>
    match msg:
        Text { sender, body } => return 'Message from {sender}: {body}'
        Join { user }         => return '{user} joined'
        Leave { user }        => return '{user} left'
        Error { code, _ }     => return 'Error {code}'
```

Pattern matching is exhaustive — the compiler rejects non-exhaustive matches at compile time.

**Guard clauses** refine matches:

```opal
match value:
    n if n > 0 => return 'positive'
    n if n < 0 => return 'negative'
    _          => return 'zero'
```

### Error Handling

Opalescent has structured error handling without exceptions. Functions declare their errors in the signature:

```opal
let parse_number = f(text: string): int32 errors ParseError =>
    ...
```

**`propagate`** — propagate an error up to the caller (like `?` in Rust):

```opal
let load_number = f(path: string): int32 errors IoError, ParseError =>
    let content = propagate read_line_from_disk(path)
    ...
```

**`guard ... into ... else`** — handle an error locally without propagating:

```opal
let parse_number = f(text: string): int32 errors ParseError =>
    guard parse_number_core(text) into parsed else {
        log_parse_failure(text)
        return 0
    }
    return parsed
```

Combined example:

```opal
let load_number = f(path: string): int32 errors IoError, ParseError =>
    let content = propagate read_line_from_disk(path)
    guard parse_number(content) into value else {
        return 0
    }
    return value
```

### Generics

Generic functions declare type parameters after `f`:

```opal
let map = f<A, B, Err>(items: A[], mapper: f(A): B errors Err): B[] errors Err =>
    let mutable out: B[] = []
    for x in items:
        let y = propagate mapper(x)
        out.push(y)
    return out
```

Generic constraints limit which types are accepted:

```opal
let max = f<T: Comparable>(a: T, b: T): T =>
    if a > b: return a
    return b
```

String interpolation uses `{expr}` inside single-quoted strings:

```opal
let n = 10
print('The answer is {n}')
print('Sum: {a + b}')
```

---

## Project Architecture

### Directory Layout

A typical Opalescent project looks like:

```
my-project/
├── opal.toml            # Project configuration (build system)
├── opal.pkg.toml        # Package manifest (for publishing)
├── opal-fmt.toml        # Formatter config (optional)
├── src/
│   ├── main.op          # Entry point (contains `entry main`)
│   ├── models.types.op  # Type definitions
│   ├── utils.op         # Utility functions
│   └── ...
├── tests/
│   └── ...
└── language-spec/       # (optional) Language spec / example files
```

**Key conventions:**

- All source files use `.op` extension
- Type definitions **must** live in files ending in `.types.op`
- Each project has exactly **one** `entry main` function
- All public functions require doc comments (≥ 30 characters)

### Project Configuration (`opal.toml`)

The build system reads `opal.toml` at the project root:

```toml
name = "my-project"
version = "1.0.0"

[dependencies]
opal-json = ">=1.0.0"
opal-http = ">=0.5.0, <1.0.0"

[build]
targets = ["x86_64-linux", "aarch64-darwin", "x86_64-windows"]
```

**Root fields:**

| Field | Required | Description |
|-------|----------|-------------|
| `name` | ✅ | Project package name |
| `version` | ✅ | Semantic version (`major.minor.patch`) |

**`[dependencies]` section:**

Each line is `package-name = "version-constraint"`. Supported constraint operators:

| Operator | Example | Meaning |
|----------|---------|---------|
| `=` | `=1.2.0` | Exactly version 1.2.0 |
| `>=` | `>=1.0.0` | At least 1.0.0 |
| `>` | `>1.0.0` | Strictly newer than 1.0.0 |
| `<=` | `<=2.0.0` | At most 2.0.0 |
| `<` | `<2.0.0` | Strictly older than 2.0.0 |

Combine constraints with a space or comma for range constraints:

```toml
[dependencies]
opal-http = ">=0.5.0 <1.0.0"
```

**`[build]` section:**

| Field | Description |
|-------|-------------|
| `targets` | Array of target triples to build for |

**Supported target triples:**

| Triple | Platform |
|--------|----------|
| `x86_64-linux` | 64-bit Linux |
| `aarch64-linux` | ARM64 Linux |
| `x86_64-darwin` | 64-bit macOS |
| `aarch64-darwin` | Apple Silicon macOS |
| `x86_64-windows` | 64-bit Windows |

### Package Manifest (`opal.pkg.toml`)

For publishing to the package registry, create `opal.pkg.toml`:

```toml
name = "my-library"
version = "0.1.0"
author = "Your Name"
description = "A brief description of the package"

[dependencies]
opal-json = ">=1.0.0"
```

**Fields:**

| Field | Required | Description |
|-------|----------|-------------|
| `name` | ✅ | Package name (used in `opal pkg add`) |
| `version` | ✅ | Semantic version string |
| `author` | ❌ | Author name |
| `description` | ❌ | Short human-readable description |

### Source File Conventions

| Convention | Rule |
|-----------|------|
| Regular source | `*.op` |
| Type definitions | `*.types.op` |
| Only one `entry` | Exactly one `entry main` per project |
| Doc comments | Required on all public functions (≥ 30 chars) |
| No semicolons | Statement termination uses newlines |
| Indentation | 4 spaces (configurable via `opal-fmt.toml`) |

---

## Compiler

### Compiler Pipeline

The Opalescent compiler pipeline processes source code through several stages to produce a native binary:

1. **Lexing & Parsing** — Source text is tokenized and parsed into an Abstract Syntax Tree (AST)
2. **Type Checking** — The AST is analyzed for type correctness and symbol resolution
3. **Code Generation** — Type-checked AST is lowered to LLVM IR via Inkwell
4. **Object Emission** — LLVM IR is compiled to native object code (`.o` file)
5. **Linking** — The object file is linked with the C runtime (embedded in the compiler binary) to produce the final binary. No `runtime/` folder is needed at runtime.

The entry point is `compile_program(source: &str, output_dir: &Path) -> Result<PathBuf, CompileError>`, which orchestrates all stages:

```rust
// Compiles source code and produces a binary at output_dir/program
let binary_path = compile_program(source, Path::new("target"))?;
std::process::Command::new(binary_path).spawn()?.wait()?;
```

The compiler creates two artifacts inside `output_dir`:
- `program.o` — intermediate object file
- `program` — final executable binary

### Escape Hatches

The following escape hatches are used to work around platform-specific constraints and linker incompatibilities:

| Platform | Escape Hatch | Reason |
|----------|-------------|--------|
| Linux (x86_64) | `-no-pie` flag to linker | Avoids Position-Independent Executable (PIE) relocation failure (`R_X86_64_32S`) on x86_64; required for compatibility with emitted object files |

No other escape hatches were needed for current targets.

---

## Testing

### Test Projects

Test projects are located in the `test-projects/` directory. Each is a minimal Opalescent program demonstrating specific language features or use cases. They serve as integration tests and examples:

**Available test projects:**

| Project | Location | Purpose |
|---------|----------|---------|
| `hello-world` | `test-projects/hello-world/` | Basic I/O and program entry |
| `fib-recursive` | `test-projects/fib-recursive/` | Recursive function calls and integer arithmetic |
| `fib-iterative` | `test-projects/fib-iterative/` | Iterative loops and mutable state |
| `simple-quiz` | `test-projects/simple-quiz/` | User input, conditionals, and randomness |

Each test project follows a standard structure (see [Test Project Conventions](#test-project-conventions) below).

### Integration Tests

Integration tests compile and execute test projects end-to-end. They verify that the compiler pipeline (lexing, parsing, type checking, code generation, linking, execution) works correctly.

**Running integration tests:**

```bash
# Run all integration tests
cargo test --features integration

# Run a specific integration test
cargo test --features integration simple_quiz_compiles_links_and_runs
```

Integration tests are gated behind the `integration` feature flag in `Cargo.toml` because they:
- Write files to disk (`test-projects/<name>/target/`)
- Spawn external processes
- Take longer to execute than unit tests

Test artifacts are automatically cleaned up after each test execution.

### Test Project Conventions

All `.op` files in test projects must follow these conventions:

**Syntax:**
- Colon-block syntax is the standard for control flow (`if`, `while`, `for`, `loop`)
- Brace syntax `{ }` is also supported for block expressions
- Statements use newlines for termination (no semicolons)

**Types:**
- Use **`int32` for all numeric types**
- String types use the built-in `string` type

**Entry Function:**
- Entry function must be named `f(args: string[]): void` (legacy signatures without parameters are also supported):

```opal
entry main = f(args: string[]): void =>
    print('Hello world')
    return void
```

**Project Structure:**
- `opal.toml` — Project metadata and dependencies
- `.gitignore` — Typical `target/` and artifact entries
- `README.md` — Brief description of the test project
- `src/main.op` — Opalescent source code with `entry main`

**Example directory layout:**

```
test-projects/hello-world/
├── opal.toml
├── .gitignore
├── README.md
└── src/
    └── main.op
```

---

## Package Manager

The package manager is invoked via `opal pkg`. It reads from and writes to `opal.pkg.toml`.

Status: package-manager execution is not wired through `src/app.rs` yet; current CLI behavior exposes package manager help topics.

### Initialise a project

```bash
opal pkg init my-library
```

Creates a starter `opal.pkg.toml`:

```toml
name = "my-library"
version = "0.1.0"
```

### Add a dependency

```bash
opal pkg add opal-json >=1.0.0
```

Appends to the `[dependencies]` section of `opal.pkg.toml`.

### Remove a dependency

```bash
opal pkg remove opal-json
```

### Install dependencies

```bash
opal pkg install
```

Resolves the dependency graph (highest compatible versions win), downloads, and installs all packages.

### Publish a package

```bash
opal pkg publish
```

Validates the manifest and uploads the package to the registry. Requires a valid `opal.pkg.toml` with both `name` and `version`.

---

## Formatter

The formatter ensures consistent code style across all `.op` files.

### Usage

```bash
# Format in-place
opal fmt src/main.op

# Check only (CI-safe, no writes)
opal fmt --check src/main.op

# Custom config
opal fmt --config opal-fmt.toml src/main.op
```

### Configuration (`opal-fmt.toml`)

```toml
indent_size = 4
max_line_width = 100
use_tabs = false
```

| Key | Default | Description |
|-----|---------|-------------|
| `indent_size` | `4` | Spaces per indent level (ignored when `use_tabs = true`) |
| `max_line_width` | `100` | Soft line-length hint for wrapping decisions |
| `use_tabs` | `false` | Use tab characters instead of spaces |

Unknown keys are silently ignored for forward-compatibility.

### What the formatter does

- Normalises line endings to `\n`
- Removes trailing whitespace
- Collapses multiple consecutive blank lines to one
- Ensures a single trailing newline
- Enforces naming conventions (snake_case for functions/variables, PascalCase for types)
- Applies consistent indentation

---

## Language Server (LSP)

The Opalescent LSP server provides editor integration with full language intelligence.

### Capabilities

| Feature | Description |
|---------|-------------|
| **Diagnostics** | Real-time type errors and parse errors as you type |
| **Completion** | Keywords, local variables, and function names |
| **Hover** | Type information for symbols under the cursor |
| **Go to Definition** | Jump to where a function or variable is declared |
| **Rename** | Rename a symbol and all its references project-wide |
| **Semantic Tokens** | Syntax highlighting beyond basic tokenization |

### VS Code

Install the bundled VS Code extension from the `vscode-extension/` directory:

```bash
cd vscode-extension
npm install
npx vsce package
code --install-extension opalescent-*.vsix
```

Or open the `vscode-extension/` folder in VS Code and press **F5** to launch the extension in development mode.

The extension activates automatically for any file with the `.op` or `.types.op` extension and connects to the LSP server.

### Other editors

Any editor that supports the Language Server Protocol can connect to `opal-lsp`. Start the server:

```bash
opal lsp --stdio
```

Configure your editor to launch this command for `.op` files.

---

## Hot Reload

Opalescent supports hot reloading of modules during development — swap out compiled modules without restarting the host process.

### How it works

1. The **change detector** watches for file modifications and classifies each change:
   - **Hot-swappable** — function body change only, ABI is compatible → swap in-place
   - **Requires restart** — function signature changed → full process restart needed
   - **Full restart** — type layout changed → cannot swap safely

2. The **ABI guard** verifies that the new module's exported function signatures are binary-compatible with the currently loaded module before swapping.

3. The **hot-swap loader** atomically replaces the active module:
   - Loads the new module
   - Validates ABI compatibility
   - Unloads the old module
   - Sets the new module as active

4. If loading fails or ABI is incompatible, the **recovery system** rolls back to the previous module without stopping the host process.

### Change classification

| Change type | Action |
|-------------|--------|
| Function body only | Hot swap (no restart) |
| Function signature | Full process restart |
| Type layout | Full process restart |

---

## Build Targets

Cross-compilation targets are declared in `opal.toml`:

```toml
[build]
targets = ["x86_64-linux", "aarch64-darwin"]
```

**Supported platforms and their native library extensions:**

| Target triple | OS | Extension |
|--------------|-----|-----------|
| `x86_64-linux` | Linux (64-bit) | `.so` |
| `aarch64-linux` | Linux (ARM64) | `.so` |
| `x86_64-darwin` | macOS (Intel) | `.dylib` |
| `aarch64-darwin` | macOS (Apple Silicon) | `.dylib` |
| `x86_64-windows` | Windows (64-bit) | `.dll` |

---

## IDE Integration

### VS Code extension

The `vscode-extension/` directory contains a complete VS Code extension with:

- Syntax highlighting for `.op` and `.types.op` files
- Bracket matching and auto-closing pairs
- LSP integration (diagnostics, completion, hover, go-to-definition, rename)
- Snippet support

**Syntax highlighting covers:**
- Keywords (`let`, `entry`, `type`, `match`, `if`, `for`, `return`, `guard`, `propagate`, `mutable`)
- Built-in types (`int32`, `int64`, `float64`, `string`, `boolean`, `void`)
- String literals with interpolation highlighting
- Comments (`#` single-line, `## ... ##` multi-line doc blocks)
- Function declarations

### Language configuration

The extension ships `language-configuration.json` with:

- Comment toggling (`#` for line comments, `## ... ##` for block comments)
- Auto-closing pairs for `()`, `[]`, `{}`, `''`
- Bracket definitions for indentation rules
