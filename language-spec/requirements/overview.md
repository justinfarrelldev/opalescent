The Opalescent programming language will be compiled, statically and strongly typed, and is aiming for a fantastic developer experience. This means it should support hot module reloading and hot reloading, as well as have a reasonable build time. The build time should NOT come at the expense of safety - error checking is more important. 

# Hot Reloading

I want the following pattern for hot reloading: 

Versioned dynamic-library hot-swap + ABI guard + automatic fallback restart.

Host process: owns all long-lived state and threads.

Hot modules: compiled to .so/.dylib/.dll with a narrow C ABI (function table + POD structs).

ABI guard: each module exports a cheap, machine-checkable “interface signature” (hash) of its public ABI and data layouts. The host loads the new module, compares signatures; if compatible → swap; if not → trigger an orchestrated full rebuild/restart.

Versioned filenames: logic_v0123.so loaded by exact name to avoid Windows file locks; GC old files later.

Change classifier: a watcher decides “eligible for hot swap” vs “requires restart” from build graph + ABI hash.

There should be fast reloads for most edits, and an automatic “don’t crash—just restart” for unsafe ones.

# Style

If a function is going to return, it must include the return keyword in it. It is not like rust, where you can just put a value and it returns that - returns must be very explicit. 

# Naming

All files, variables and functions must be snake case:

example_file.(ending)
let example_variable = 5
let example_fn = f(): void => ...

All types must be in Pascal case:

type ExampleType

# Casts

Cast syntax: (expr as T)

- A cast converts the value of expr to type T, failing to compile if:
  - The conversion is lossy at compile-time (for literals/constants), or
  - The conversion cannot be proven safe for runtime values (use explicit checked APIs for fallible conversions).

- Provided casts:
  - Widening within signed or within unsigned families (e.g., int8→int32) require `as` and are well-defined.
  - Signed↔unsigned require `as` and follow two’s-complement reinterpretation rules only via explicit APIs (e.g., to_{int32,uint32}).
  - Float↔int and int↔float casts require `as`; out-of-range results are compile errors for constants and runtime traps unless the `checked_` or `saturating_` APIs are used.

# Compiler Output

Use miette for formatting of the output.

# String Access

Public string access uses `value.at(index)` with zero-based Unicode scalar positions.

- `value.at(index)` returns `string` on success
- `value.at(value.length - 1)` is valid when the string is non-empty
- Opalescent does not introduce a public `char` or `rune` type for indexed string access
- Out-of-bounds string access surfaces `IndexOutOfBoundsError`, so callers must use `guard` or `propagate`
- Unhandled out-of-bounds string access currently prints `IndexOutOfBoundsError` on stderr

# Whitespace

Source formatting whitespace is separate from text-processing whitespace semantics.

- Mixed indentation whitespace is not allowed. Only either spaces or tabs are allowed in a project, and all files must match. This will eventually be enforced with a formatter.
- `string_is_blank` and `string_trim_whitespace` use the Unicode `White_Space` property for the Unicode version bundled with the current Opalescent release.
- `string_split_lines` treats `\n`, `\r\n`, and bare `\r` as line terminators.
