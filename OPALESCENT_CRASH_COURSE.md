# Opalescent Crash Course

This guide teaches Opalescent from the ground up. You do not need to know programming language theory. If you already write compilers or typed languages, skim the first sections and use the rest as a syntax-and-footguns reference.

Opalescent is early software. Prefer examples that already exist under `test-projects/`; those examples are used by the repository's tests and are more reliable than invented snippets.

## Fast path for experienced readers

```opal
##
  Description: Program entry point.
##
entry main = f(args: string[]): void =>
    let name = 'world'
    print('Hello {name}')
    return void
```

Key ideas:

- Files ending in `.op` contain imports, values, functions, and entries.
- Files ending in `.types.op` contain type declarations and type imports.
- `entry main` is the program entry point.
- `let` creates an immutable binding. `let mutable` creates a binding you can assign to.
- Blocks are indentation-sensitive. Match the style the formatter emits; do not rely on a fixed manual rule such as "always four spaces".
- Strings use single quotes and support interpolation with `{expression}`.
- Fallible functions declare `errors ...` (often with named error sets) and callers use `propagate` or `guard`.
- Standard-library functions are imported from `standard`.

A minimal working project is `test-projects/hello-world/`.

## 1. What is Opalescent?

Opalescent is a compiled programming language. You write `.op` files, the compiler checks them, lowers them to native code through LLVM, links a small runtime, and produces a binary.

The language is designed around a few habits:

- Make types explicit enough that mistakes are caught early.
- Make errors visible in function signatures.
- Prefer readable indentation over punctuation-heavy syntax.
- Keep examples small and practical.

## 2. Build the compiler and set up `opal`

Build the compiler from the repository root:

```bash
export LLVM_SYS_140_PREFIX=/usr/lib/llvm-14
cargo build --release
```

The built binary is named `opalescent`:

```bash
./target/release/opalescent run test-projects/hello-world/src/main.op
```

Most docs use `opal` as the short command name. For local development, create an alias after building:

```bash
alias opal="$PWD/target/release/opalescent"
```

If you are iterating on compiler code and using debug builds, use this instead:

```bash
cargo build
alias opal="$PWD/target/debug/opalescent"
```

Now the examples become shorter:

```bash
opal run test-projects/hello-world/src/main.op
opal check test-projects/hello-world/src/main.op
```

To avoid typing `src/main.op`, run from a project directory that has `opal.toml`:

```bash
cd test-projects/hello-world
opal run
```

`opal run` and `opal build` use the current project's `opal.toml` and `src/main.op`.

## 3. Running your first program

The hello-world source shape is:

```opal
entry main = f(args: string[]): void =>
    let world = 'world'
    print('Hello {world}')
    return void
```

Read it left to right:

- `entry main` means this function starts the program.
- `f(args: string[]): void` means the function accepts command-line arguments as an array of strings and returns `void`.
- `=>` starts the function body.
- Indented lines belong to the function.
- `return void` ends a function that has no meaningful value to return.

## 4. Comments and documentation comments

Single-line comments start with `#`.

```opal
# This is a normal comment.
let answer = 42
```

Public functions and examples usually use doc blocks:

```opal
##
  Description: Adds two numbers.
##
let add = f(a: int32, b: int32): int32 =>
    return a + b
```

The documentation generator reads these blocks:

```bash
opal doc test-projects/hello-world/src/main.op
```

## 5. Values and bindings

A binding gives a name to a value.

```opal
let name = 'Ada'
let age: int32 = 36
```

The type can often be inferred, but explicit types help beginners and are useful in docs.

Bindings are immutable by default. If the value should change, say so:

```opal
let mutable count: int32 = 0
count = count + 1
```

## 6. Primitive types

Common built-in types:

| Type | Meaning |
|---|---|
| `boolean` | `true` or `false` |
| `string` | text |
| `void` | no meaningful return value |
| `int8`, `int16`, `int32`, `int64` | signed integers |
| `uint8`, `uint16`, `uint32`, `uint64` | unsigned integers |
| `float32`, `float64` | floating-point numbers |

Use `int32` for most small examples unless a fixture needs another width.

## 7. Strings and interpolation

Strings use single quotes:

```opal
let language = 'Opalescent'
print('Hello from {language}')
```

Anything inside `{...}` is evaluated and inserted into the string.

You can ask for a string's length:

```opal
let text = 'hello'
print('length: {text.length}')
```

You can also read a string by zero-based Unicode scalar position with fallible `.at(...)`. The result is still a `string`, so there is no separate public `char` type:

```opal
let message = 'aéf'

guard message.at(0) into first else err =>
    print('string access failed: {err}')
    return void

guard message.at(message.length - 1) into last else err =>
    print('string access failed: {err}')
    return void

guard message.at(1) into unicode else err =>
    print('string access failed: {err}')
    return void

print('first={first}')
print('last={last}')
print('unicode={unicode}')
```

If the index is outside `0 <= index < message.length`, `.at(...)` returns `IndexOutOfBoundsError`, so the read must be handled with `guard` or `propagate`. If it is left unhandled, the runtime currently prints `IndexOutOfBoundsError` on stderr.

## 8. Functions

A function is declared with `let name = f(...): ReturnType =>`.

```opal
let double = f(value: int32): int32 =>
    return value * 2
```

Call it with parentheses:

```opal
let result = double(21)
print('result: {result}')
```

Functions can have multiple parameters:

```opal
let add = f(a: int32, b: int32): int32 =>
    return a + b
```

### Labeled multiple returns

A function can return more than one value, but every multi-return function must declare labels in its signature.

```opal
let pair = f(): first: int32, second: int32 =>
    return first: 11 as int32, second: 22 as int32
```

Those labels are required metadata. Callers, generated docs, diagnostics, and imported modules can see them. The returned values still stay in declaration order.

Unlabeled destructuring only works when the local names exactly match the returned labels in the same order:

```opal
let first, second = pair()
```

If you want different local names, write the returned label on the left and the local name on the right:

```opal
let first: renamed_first, second: renamed_second = pair()
```

That syntax means `returned_label: local_name`. It verifies intent, but it never reorders values. If a function returns `first, second`, then `let second, first = pair()` is wrong, and `let second: b, first: a = pair()` is wrong too.

Return pass-through is also tested:

```opal
let forwarded_pair = f(): first: int32, second: int32 =>
    return pair()
```

Important limits:

- There is no tuple type for these results.
- There is no first-class multi-value storage. You destructure at the call site or pass the whole result straight through with `return other_multi()`.
- Labels are not function type identity.
- Labels are not ABI identity.
- Labels do not let callers reorder returned values.

The working ordinary fixture is `test-projects/multiple-returns-basic/src/main.op`. Cross-module label metadata is covered by `test-projects/multiple-returns-cross-module/src/`.

## 9. `entry main`

Every runnable program has one entry point:

```opal
entry main = f(args: string[]): void =>
    print('program started')
    return void
```

`args: string[]` is an array of command-line arguments. The `[]` means “array of”.

## 10. Blocks and indentation

Most control flow uses a colon followed by indented statements:

```opal
if ready:
    print('ready')
else:
    print('not ready')
```

The indentation is part of the structure. The formatter is intended to become the source of truth for normalized style, but it is still under development:

```bash
opal fmt path/to/file.op
opal fmt --check path/to/file.op
```

Do not assume the language requires exactly four spaces in every context. Follow existing fixtures and formatter output, and be cautious when using formatter output as a final style authority until the formatter stabilizes.

## 11. `if`, comparisons, and boolean logic

```opal
let describe = f(n: int32): string =>
    if n > 0:
        return 'positive'
    if n < 0:
        return 'negative'
    return 'zero'
```

Equality is written with `is`:

```opal
if answer is 42:
    print('correct')
```

> **`is` means equality, not type identity.** If you come from Python, `is` checks whether two values are equal, the same role that `==` plays in most languages. It does not check whether two variables refer to the same object in memory, and it has nothing to do with type testing. Use `is` anywhere you would write `==` in another language.

Boolean combinations use words:

```opal
if index is 2 or index is 3:
    return true
```

You can see this style in `test-projects/fs-markdown-roundtrip/src/main.op`.

## 12. Loops

`while` repeats while a condition is true:

```opal
let mutable i: int32 = 0
while i < 3:
    print('i = {i}')
    i = i + 1
```

`for` iterates over arrays:

```opal
let names: string[] = ['Ada', 'Grace']
for name in names:
    print(name)
```

Use `while true:` for a simple unconditional statement loop. The tested `loop` form is an expression form written with `loop =>`; it can `continue` and can `break` with named values.

```opal
import take_input, string_to_int32 from standard

let user_input, user_number =
    loop =>
        let s = propagate take_input()
        guard string_to_int32(s) into n else e =>
            print('Error: {e}')
            continue
        break user_input: s, user_number: n
```

That shape is used by `test-projects/simple-quiz/src/main.op`. Do not use the old colon-style spelling for `loop`; the fixture-backed syntax is `loop =>`.

## 13. Arrays

An array stores many values of the same type.

```opal
let mutable scores: int32[] = []
scores.push(10)
scores.push(20)
print('count: {scores.length}')
```

Use fallible `.at(...)` to read an element:

```opal
guard scores.at(0) into first else err =>
    print('array access failed: {err}')
    return void
```

Arrays support helpers such as `push`, `pop`, `map`, `filter`, `reduce`, and `zip`. Fixture directories under `test-projects/array-*` show concrete examples.

## 14. Type definition files

A `.types.op` file defines shapes of data. Regular `.op` files should not contain type declarations.

Product type, like a record:

```opal
##
  Description: A person record.
##
public type Person:
    name: string
    age: int32
```

Construct a product value with `new`:

```opal
import type Person from ./person.types

let person: Person = new Person:
    name: 'Ada'
    age: 36
```

The Game of Life fixture uses this style in `test-projects/game-of-life-full/src/life.types.op` and constructs `LifeConfig` in `test-projects/game-of-life-full/src/main.op`.

Important caveats:

- `.types.op` files may contain type declarations and type imports, not values or functions. Putting `let` in a `.types.op` file triggers `TypeError::NonTypeDeclarationInTypesFile`.
- Regular `.op` files should not declare types. Doing so triggers `TypeError::TypeDeclarationOutsideTypesFile`.
- Imported `.types.op` names are not yet reliable in every function-signature position. This is expected to be supported eventually, but for now follow working fixture patterns and keep signatures near the defining type module when possible.

## 15. Sum types and enums

A sum type is a value that can be one of several variants.

```opal
##
  Description: Result of a small lookup.
##
public type LookupResult:
    Found:
        value: string
    Missing
```

An enum is a sum type with variants that have no fields:

```opal
##
  Description: A direction on a grid.
##
public type Direction:
    North
    East
    South
    West
```

## 16. ADT checks and `match` caution

The tested fixtures use `is` checks when branching on simple values and constructor results:

```opal
if cell_at(board, width, height, x, y) is alive_cell():
    return 1
```

Top-level proposal documents discuss richer `match` forms, and the compiler has parser/type-checker work in this area, but the beginner fixtures audited for this guide do not provide a copy-pasteable `match` control-flow example. Until a passing fixture is added, do not treat proposal `match` snippets as public tutorial syntax.

## 17. Imports

Import standard-library functions from `standard`:

```opal
import path_from, read_text_sync from standard
```

Import local functions from another file, omitting `.op`:

```opal
import create_seed_board from ./patterns
```

Import a type from a `.types.op` file with `import type` and include `.types` in the module path:

```opal
import type LifeConfig from ./life.types
```

Working examples live in `test-projects/import-types-basic/`, `test-projects/import-types-multiple/`, and `test-projects/import-types-aliased/`.

## 18. Errors: why they are different

Opalescent does not hide errors as exceptions. A function that can fail says so in its signature:

```opal
import read_text_sync from standard
import type FilesystemTextReadErrors from standard.errors

let load_text = f(path: FilesystemPath): string errors FilesystemTextReadErrors =>
    return propagate read_text_sync(path)
```

That `errors ...` clause is part of the function type. Callers must decide what to do with possible failure.

Named error sets such as `FilesystemTextReadErrors` are compile-time aliases imported with `import type`. They shorten signatures, but `guard` and `propagate` still check the expanded leaf errors.

## 19. `propagate`

Use `propagate` when you cannot handle an error locally and want to return it to your caller.

```opal
import read_text_sync from standard

let text = propagate read_text_sync(path)
```

Read this as: “try to read the text; if it fails, stop this function and send the error upward.”

The function containing that line must declare compatible error types.

This also works with tested fallible multiple returns. The success values keep their declared order, and the labels still act as metadata only:

```opal
let parse_pair = f(left_text: string, right_text: string): left: int32, right: int32 errors ParseError =>
    let left = propagate string_to_int32(left_text)
    let right = propagate string_to_int32(right_text)
    return left: left, right: right

let sum_pair = f(left_text: string, right_text: string): int32 errors ParseError =>
    let left, right = propagate parse_pair(left_text, right_text)
    return left + right
```

The tested success ABI shape for this form is ordered success fields plus trailing error state. That is why `propagate` can destructure the success side positionally while labels still stay outside ABI identity.

## 20. `guard ... else`

Use `guard` when you want to handle an error right here.

```opal
import read_text_sync from standard

guard read_text_sync(path) into text else err =>
    print(err)
    propagate err

print(text)
```

The `into text` part names the successful result. The `else err =>` part runs only on failure.

Statement `guard` also supports the tested multiple-return success forms:

```opal
guard parse_pair('5', '6') into left, right else err =>
    print('UNEXPECTED_GUARD_ERROR={err}')
    propagate err

print('GUARD_SUCCESS={left},{right}')

guard parse_pair('oops', '9') into left: failed_left, right: failed_right else err =>
    print('GUARD_HANDLED={err}')
    propagate err
```

The unlabeled form still requires exact local names in order. The explicit form still means `returned_label: local_name`. Labels verify the success binding shape, but they do not change value order.

Guard footguns:

- A handler must either produce the expected fallback value or transfer control safely with `return`/`propagate`.
- A long-form handler that contains only `propagate err` is intentionally rejected by `test-projects/guard-stmt-only-propagate/`; use `return propagate err` where an expression is expected, or add real local handling such as logging before propagating.
- Do not put an `if ... else ...` expression directly after `guard`; `test-projects/ambiguous-guard-if/` documents that ambiguity.
- Aliasing an error and then wrapping/returning the alias can be rejected by the strict guard rules. Prefer propagating the original error binding unless you are following an existing tested pattern.
- `_ignored_*` aliases in guard error handlers are not a way to bypass strict propagation checks.

`test-projects/fs-markdown-roundtrip/src/main.op` shows ordinary filesystem guards. Guard edge cases are covered by dedicated guard fixtures under `test-projects/guard-*`.

## 21. Bytes

`Bytes` is an opaque immutable byte buffer.

```opal
let buffer: Bytes = new Bytes
let length: int32 = buffer.length
print('new syntax length: {length}')
```

That exact shape is used by `test-projects/bytes-empty-construct-new-syntax/src/main.op`.

Useful functions include:

```opal
import bytes_from_hex, bytes_to_hex, bytes_concatenate, bytes_slice from standard
```

Because decoding and slicing can fail, use `guard` or `propagate` with those operations.

## 22. Filesystem example

This simplified version of the markdown roundtrip fixture reads lines, joins them, writes output, and verifies the result:

```opal
import path_from, read_lines_sync, read_text_sync, write_text_sync, string_join from standard
import type AllocationErrors, FilesystemOverwriteErrors, FilesystemTextReadErrors from standard.errors

entry main = f(args: string[]): void errors FilesystemTextReadErrors, FilesystemOverwriteErrors, AllocationErrors =>
    let input_path = path_from('input.md')
    let output_path = path_from('output.md')

    guard read_lines_sync(input_path) into lines else err =>
        print(err)
        propagate err

    let rendered = propagate string_join(lines, '\n')

    guard write_text_sync(output_path, rendered) else err =>
        print(err)
        propagate err

    let actual = propagate read_text_sync(output_path)
    print(actual)
    return void
```

See the complete working version in `test-projects/fs-markdown-roundtrip/src/main.op`.

## 23. Terminal and time APIs

The Game of Life fixture uses terminal and frame-clock APIs:

```opal
import frame_clock_wait_next_sync from standard

let clock = propagate new FrameClock:
    frames_per_second: config.frames_per_second

propagate frame_clock_wait_next_sync(clock)
```

It also uses terminal rendering helpers through its `render.op` module. Read `test-projects/game-of-life-full/` for a larger real example.

## 24. Formatting and checking

Check one file:

```bash
opal check src/main.op
```

Format one file:

```bash
opal fmt src/main.op
```

Check formatting without rewriting:

```bash
opal fmt --check src/main.op
```

## 25. Common footguns

- Use an `opal` alias locally; the built binary is `opalescent`.
- Set `LLVM_SYS_140_PREFIX` before building.
- Run from a directory containing `opal.toml` if you want `opal run` without a file path.
- Keep type declarations in `.types.op` files and values/functions in `.op` files.
- Type imports from `.types.op` modules work in common fixture patterns, but using imported type names in every signature position is still incomplete.
- Do not rely on `opal pkg`; command help exists, but execution is not implemented yet.
- Proposal directories are not finished APIs.
- The formatter is still under development. Use it, but review its output and prefer tested fixtures when behavior is unclear.
- When examples disagree, trust `test-projects/` and the integration tests over old proposal docs.

## 26. How to keep learning

Suggested path:

1. Run `test-projects/hello-world`.
2. Read `test-projects/fib-iterative` for loops and mutable state.
3. Read `test-projects/array-push`, `array-map`, and `array-filter` for arrays.
4. Read `test-projects/bytes-empty-construct-new-syntax` for `Bytes`.
5. Read `test-projects/fs-markdown-roundtrip` for imports, guards, filesystem operations, arrays, and strings.
6. Read `test-projects/game-of-life-full` for a multi-file project.

When in doubt, prefer examples that already exist under `test-projects/`; those examples are used by the repository's integration tests.
