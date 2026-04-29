# Array Features: append and Multi-Dimensional Arrays

This document specifies two planned array features for Opalescent: the `append` standard
library function for adding elements to arrays, and first-class multi-dimensional array
syntax using `T[][]`.

---

## 1. `append` — Adding Elements to Arrays

### Overview

`append` is a free function imported from `standard` that returns a new array containing
all elements of the original plus one new element at the end. It is pure: the original
array is not modified.

`append` **does not require a generic type argument**. The element type is inferred from
the array and the value being appended.

### Signature

```op
import append from standard

let append = f(xs: T[], value: T): T[] => ...
```

### Rules

- `xs` and `value` must have compatible types; passing a mismatched type is a compile error.
- The original array is never mutated; `append` always returns a new array.
- Works with any element type: primitives, strings, custom types.
- For building arrays incrementally, declare the accumulator `mutable` and reassign it.

### Efficiency and Value Semantics

`append` is *logically* pure — callers see a new array and the original binding is
unchanged. The critical implementation question is whether it physically copies all
elements on every call.

A naïve full-copy makes "build a list in a loop" **O(n²)** in time and allocations.
Appending `n` items one by one produces arrays of size 1, 2, 3, …, n — `O(n²)` total
element moves across `n` allocations. This will surprise users coming from languages
where the equivalent growth is amortised O(1).

The two viable runtime strategies to avoid this:

- **Copy-on-write (COW).** Arrays share a reference-counted backing buffer. `append`
  only copies the buffer when the original has more than one live owner. When the
  accumulator is the sole reference throughout a loop, no copy is ever made.

- **Uniqueness reuse (Perceus-style).** The compiler proves at the call site that `xs`
  has refcount = 1 and mutates the buffer in-place, returning the same pointer — the
  same performance as `.push` with a purely functional signature. See
  `memory-model-proposals/hybrid/perceus-functional-reuse-analysis/proposal.md`.

Until one of these strategies is in place, every `append` call in a loop allocates.
For performance-critical array construction today, use the `.push` method described
after the examples below.

### Examples

**Example 1 — Building a list of scores**

```op
import append from standard

entry main = f(args: string[]): void =>
    let mutable scores: int32[] = []
    scores = append(scores, 10)
    scores = append(scores, 25)
    scores = append(scores, 8)
    scores = append(scores, 17)

    let mutable i = 0
    while i < scores.length:
        print('Score: {scores[i]}')
        i = i + 1

    return void
```

**Example 2 — Collecting lines that pass a filter**

```op
import append, read_lines_sync, path_from from standard

let collect_non_empty = f(path: string): string[] errors FileNotFoundError, ReadFailureError, IsADirectoryError, InvalidPathError, InvalidUtf8Error, PermissionDeniedError =>
    let lines = propagate read_lines_sync(path_from(path))
    let mutable result: string[] = []
    let mutable i = 0
    while i < lines.length:
        if lines[i].length > 0:
            result = append(result, lines[i])
        i = i + 1
    return result

entry main = f(args: string[]): void =>
    guard collect_non_empty('input.txt') into lines else _e =>
        print('Could not read file')
        return void
    print('Non-empty line count: {lines.length}')
    return void
```

**Example 3 — Flattening two arrays**

```op
import append from standard

let concat_arrays = f(left: int32[], right: int32[]): int32[] =>
    let mutable out = left
    let mutable i = 0
    while i < right.length:
        out = append(out, right[i])
        i = i + 1
    return out

entry main = f(args: string[]): void =>
    let a: int32[] = [1, 2, 3]
    let b: int32[] = [4, 5, 6]
    let combined = concat_arrays(a, b)

    let mutable i = 0
    while i < combined.length:
        print('{combined[i]}')
        i = i + 1

    return void
```

**Example 4 — Accumulating parsed values from strings**

```op
import append from standard

let parse_valid_numbers = f(inputs: string[]): int32[] =>
    let mutable valid: int32[] = []
    let mutable i = 0
    while i < inputs.length:
        guard string_to_int32(inputs[i]) into n else _e =>
            i = i + 1
            continue
        valid = append(valid, n)
        i = i + 1
    return valid

entry main = f(args: string[]): void =>
    let raw: string[] = ['42', 'hello', '7', 'world', '100']
    let numbers = parse_valid_numbers(raw)
    print('Parsed {numbers.length} valid numbers')
    return void
```

### Mutable Alternative: `.push`

The type system already registers `[t].push` as a built-in method that appends a value
in-place and returns `void`. It is the preferred choice when constructing an array
inside a single function body with a locally owned `mutable` binding, because it avoids
any allocation overhead:

```op
let mutable xs: int32[] = []
xs.push(10)
xs.push(25)
xs.push(8)
```

**When to use `append` vs `.push`**

| Situation | Prefer |
|---|---|
| Building a result inside one function body | `.push` |
| Returning a new array without modifying the original | `append` |
| Pure/functional context, no `mutable` binding available | `append` |
| Growing an array across function call boundaries | `append` + return |

#### Does `.push` Clash with `guard`?

**No.** `push` returns `void` with no declared error types, so it is used as a plain
statement — `guard` and `propagate` cannot and do not need to be applied to it.
`guard`/`propagate` wrap *failable expressions*; `push` is an *infallible statement*.
They occupy different layers of the language and compose without conflict:

```op
entry main = f(args: string[]): void =>
    let mutable results: int32[] = []

    guard parse_something() into value else _e =>
        print('parse failed')
        return void

    results.push(value)   # plain statement following a guard — no conflict
    return void
```

**What about out-of-memory?** Most runtime environments treat heap exhaustion as a
fatal condition (abort or panic) because programs typically cannot recover meaningfully.
Keeping `push` infallible follows that convention and avoids forcing `guard` onto every
append in a loop.

If OOM must eventually be recoverable (e.g. for embedded or safety-critical targets),
the right design is a separate `try_push` rather than making `push` itself failable:

```op
xs.push(value)                 # infallible — panics on OOM (the common case)
propagate xs.try_push(value)   # errors OutOfMemoryError — for when OOM must be handled
```

This keeps the common path uncluttered while remaining consistent with Opalescent's
explicit error-handling philosophy.

#### `.push` Does Not Propagate Through Function Parameters

Because arrays have value semantics, passing an array to a function passes a copy (or
COW-shared reference). A `.push` inside the callee affects only its local view; the
caller's binding is unchanged:

```op
let bad_fill = f(xs: int32[]): void =>
    xs.push(99)   # mutates a local copy only — caller sees nothing

entry main = f(args: string[]): void =>
    let mutable data: int32[] = []
    bad_fill(data)
    print(data.length)   # prints 0, not 1
    return void
```

When a helper function must grow an array, have it accept and return the array:

```op
let add_defaults = f(xs: int32[]): int32[] =>
    let mutable out = xs
    out.push(0)
    out.push(-1)
    return out
```

If Opalescent later adds `mutable` parameter annotations (reference semantics for
individual parameters), `.push` will work naturally across call boundaries at that point.

---

## 2. Multi-Dimensional Arrays (`T[][]`)

### Overview

Opalescent supports multi-dimensional arrays using the postfix `[][]` type syntax. A
`T[][]` is an array of arrays of `T`. Each inner array is an independent array and may
have a different length (jagged arrays are valid).

### Syntax

```op
# Type annotation
let grid: boolean[][] = ...

# Literal
let matrix: int32[][] = [[1, 2, 3], [4, 5, 6], [7, 8, 9]]

# Access — each [] consumes one dimension
let value = matrix[row][col]
```

### Rules

- `T[][]` means "array of `T[]`". `T[][][]` would be "array of `T[][]`", and so on.
- Index access is left-to-right: `grid[r][c]` first resolves row `r`, then column `c`.
- Each inner array has its own `.length`. `grid.length` is the number of rows; `grid[r].length` is the width of row `r`.
- Jagged arrays (rows of different lengths) are permitted. There is no built-in rectangular enforcement.

### Value Semantics and Aliasing

Because every array has value semantics, each row in a `T[][]` is an independent array
value. Constructing a grid from a shared row binding does not create aliases:

```op
let row: int32[] = [1, 2]
let grid: int32[][] = [row, row]
# grid[0] and grid[1] are independent copies (or COW-shared until either is mutated).
# Writing to grid[0] will NOT affect grid[1].
```

This is intentional and safe but differs from languages with reference semantics, where
`grid[0]` and `grid[1]` would alias the same underlying storage. Be aware of this when
porting algorithms that rely on aliased rows.

### Length Tracking

`grid.length` is the number of rows and is straightforward. `grid[r].length` requires
per-row length tracking at runtime — a single compile-time constant is not sufficient
for jagged arrays. The codegen must maintain a separate length binding for each
extracted row value, not just for the outer array. This is one of the concrete open
implementation items.

### Rectangular Arrays and `Matrix<T>`

`T[][]` is explicitly jagged: rows can differ in length. For performance-sensitive
rectangular data (image pixels, numerical matrices, game grids), jagged arrays impose
real costs: one heap allocation per row, poor cache locality, and an extra bounds-check
per row access.

A future `Matrix<T>` type backed by a flat `T[]` with explicit `width` and `height`
fields is the right solution for rectangular data. It gives a single contiguous
allocation, O(1) `[row][col]` indexing via a single multiply-add, and enforced
rectangular invariants. The flat manual-index encoding shown in Example 2
(`cell_index` helper) is the practical workaround until then.

### Examples

**Example 1 — A 3×3 integer matrix**

```op
entry main = f(args: string[]): void =>
    let matrix: int32[][] = [
        [1, 2, 3],
        [4, 5, 6],
        [7, 8, 9]
    ]

    let mutable row = 0
    while row < matrix.length:
        let mutable col = 0
        while col < matrix[row].length:
            print('{matrix[row][col]} ')
            col = col + 1
        print('')
        row = row + 1

    return void
```

**Example 2 — A Game of Life grid (flat encoding via `int32` 0/1)**

Because indexed assignment (`grid[i][j] = value`) requires multi-dimensional indexed
assignment support in codegen, the idiomatic approach today is to represent a 2D grid as
a flat `int32[]` and compute the index manually. This example shows both the flat
encoding and how `T[][]` would express the same intent once full support lands.

```op
# Flat encoding (works today):
let cell_index = f(row: int32, col: int32, width: int32): int32 =>
    return row * width + col

entry main = f(args: string[]): void =>
    let width = 5
    let height = 5
    let mutable grid: int32[] = [
        0, 1, 0, 0, 0,
        0, 0, 1, 0, 0,
        1, 1, 1, 0, 0,
        0, 0, 0, 0, 0,
        0, 0, 0, 0, 0
    ]

    let mutable row = 0
    while row < height:
        let mutable col = 0
        while col < width:
            let idx = cell_index(row, col, width)
            if grid[idx] is 1:
                print('#')
            else:
                print('.')
            col = col + 1
        print('')
        row = row + 1

    return void

# T[][] encoding (once full 2D codegen support lands):
# let grid: int32[][] = [
#     [0, 1, 0, 0, 0],
#     [0, 0, 1, 0, 0],
#     [1, 1, 1, 0, 0],
#     [0, 0, 0, 0, 0],
#     [0, 0, 0, 0, 0]
# ]
# let alive = grid[row][col] is 1
```

**Example 3 — Adjacency list (jagged 2D array)**

```op
entry main = f(args: string[]): void =>
    # Node 0 connects to 1, 2
    # Node 1 connects to 0, 3
    # Node 2 connects to 0
    # Node 3 connects to 1
    let adjacency: int32[][] = [
        [1, 2],
        [0, 3],
        [0],
        [1]
    ]

    let mutable node = 0
    while node < adjacency.length:
        print('Node {node} neighbours: ')
        let mutable n = 0
        while n < adjacency[node].length:
            print('{adjacency[node][n]} ')
            n = n + 1
        print('')
        node = node + 1

    return void
```

**Example 4 — Passing and returning `T[][]` from functions**

```op
let transpose = f(matrix: int32[][]): int32[][] =>
    let rows = matrix.length
    let cols = matrix[0].length
    let mutable result: int32[][] = []
    let mutable c = 0
    while c < cols:
        let mutable row_out: int32[] = []
        let mutable r = 0
        while r < rows:
            row_out = append(row_out, matrix[r][c])
            r = r + 1
        result = append(result, row_out)
        c = c + 1
    return result

entry main = f(args: string[]): void =>
    let m: int32[][] = [[1, 2, 3], [4, 5, 6]]
    let t = transpose(m)

    let mutable r = 0
    while r < t.length:
        let mutable c = 0
        while c < t[r].length:
            print('{t[r][c]} ')
            c = c + 1
        print('')
        r = r + 1

    return void
```

### Implementation Status

| Feature | Parser | Type System | Codegen |
|---|---|---|---|
| `T[][]` type annotation | ✅ | ✅ | ⚠️ Partial |
| `[[...], [...]]` literal | ✅ | ✅ | ❌ Not yet |
| `arr[r][c]` read access | ✅ | ✅ | ❌ Not yet |
| `arr[r][c] = val` write | ✅ | ✅ | ❌ Not yet |

The parser and type system handle `T[][]` correctly throughout. The codegen gap is that
`codegen_array_literal` only peels one level of `CoreType::Array` when emitting LLVM IR,
and `codegen_array_access` does not yet chain pointer loads across dimensions. Until those
are addressed, use the flat `T[]` encoding with manual index arithmetic as shown in
Example 2.

### Design Recommendations

1. **Implement COW or Perceus reuse for `append` before it is documented as the
   idiomatic growth primitive.** Without it, every incremental accumulation loop is
   O(n²). A naïve full-copy is acceptable as a bootstrap implementation but must not
   be the permanent runtime behaviour.

2. **Expose both `append` and `.push`.** `append` for functional/return-style code;
   `.push` for local mutable builders. They are complementary, not redundant. The
   type checker already registers `[t].push`; it needs codegen backing.

3. **Keep `push` infallible; add `try_push` only if OOM must be recoverable.**
   Forcing `guard` on every append in a loop would make everyday array construction
   noisy without meaningful benefit in most programs.

4. **Allow jagged `T[][]`; add `Matrix<T>` later for rectangular data.** Jagged arrays
   are compositionally correct and cover common use cases (adjacency lists,
   variable-width rows). A separate `Matrix<T>` backed by a flat buffer should be
   introduced when performance-critical rectangular algorithms become a priority.

5. **Codegen priorities for `T[][]`:** The two concrete tasks are (a) make
   `codegen_array_literal` recurse into `CoreType::Array` elements to emit nested heap
   allocations, and (b) make `codegen_array_access` chain GEP/load sequences for each
   additional `[]` in an index expression. Per-row length binding is a prerequisite
   for (b).
