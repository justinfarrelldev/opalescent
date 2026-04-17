# Ownership Without Lifetimes (Move + Auto-Clone)

## Overview

A **move-by-default** ownership model where values are transferred on assignment, but the compiler **automatically inserts clones** when it detects a value is used after being moved. No lifetime annotations exist in the language — ever. Inspired by Val (formerly Val), Lobster, and Hylo.

The philosophy: if a developer uses a value after "moving" it, they almost certainly wanted a copy. The compiler should do the right thing silently, and developers who care about performance can use `move` to opt into explicit transfer semantics.

## Syntax Design

### Default Behavior: Implicit Clone on Reuse

```opal
let name = "Alice"
let greeting = name          # compiler auto-clones 'name'
print(name)                  # works fine — 'name' was cloned above
print(greeting)              # also fine
```

### Explicit Move for Performance

When you know a value won't be reused, `move` avoids the clone:

```opal
let name = "Alice"
let greeting = move name     # explicit transfer, no clone
# print(name)                # Compile error: 'name' was explicitly moved
```

### Function Parameters

By default, functions receive owned copies. Use `borrow` for zero-copy reads:

```opal
# Receives an owned copy (auto-cloned at call site if caller keeps using it)
let process = f(data: string): string =>
    return data.to_upper()

# Borrows — no copy, caller retains ownership
let measure = f(borrow data: string): int32 =>
    return data.len()

# Consumes — caller must give up ownership
let consume = f(move data: string): void =>
    print(data)
    return void
```

### No Lifetime Annotations — Period

Returning borrowed data is handled by **returning owned copies**:

```opal
# Instead of returning a reference, return an owned value
let first_word = f(borrow text: string): string =>
    let idx = text.index_of(' ')
    return text.slice(0, idx)   # returns a new owned string
```

### Mutable Borrowing

```opal
let append = f(borrow mutable list: int32[], value: int32): void =>
    list.push(value)
    return void

let mutable numbers = [1, 2, 3]
append(numbers, 4)
print(numbers)   # [1, 2, 3, 4]
```

## Example Applications

See companion `.op` files:

- `auto_clone_basics.op` — how implicit cloning works in practice
- `explicit_move_patterns.op` — performance-conscious ownership transfer
- `collections.op` — building and passing collections

## Strengths

1. **Near-zero learning curve**: Developers from any language background can immediately be productive — it "just works" like a GC language for simple cases
2. **No lifetime annotations**: The single biggest source of Rust frustration is eliminated entirely
3. **Progressive optimization**: Start with auto-clone (correct), then add `move` where profiling shows it matters
4. **Perfect fit for immutable-by-default**: Since most values are immutable, cloning is cheap (can use copy-on-write under the hood)
5. **Enterprise-friendly**: New team members don't need to learn ownership theory
6. **Self-referential structures work naturally**: No borrow checker means linked lists, trees with parent pointers, etc. are straightforward
7. **Fail-fast compatible**: Moves are checked at compile time; auto-clones are always safe

## Weaknesses

1. **Hidden performance costs**: Auto-cloning can cause unexpected allocations in hot loops — developers may not realize copies are happening
2. **Less safe than a borrow checker**: Cannot prevent all use-after-free at compile time if combined with unsafe FFI
3. **No data race prevention**: Without borrows, you can't prove exclusive access at compile time. Needs separate concurrency model (channels, actors, etc.)
4. **Clone requirement**: All types must implement Clone, which may be expensive for large structures
5. **Harder to reason about performance**: "Is this a move or a clone?" requires understanding the compiler's analysis
6. **Copy-on-write complexity**: To make auto-clone cheap, the runtime needs COW semantics, which adds implementation complexity

## Impact on Existing Syntax

- **Low impact**: Existing Opalescent code continues to work almost unchanged
- **New keywords**: `move`, `borrow` — both optional and additive
- **Array/string semantics**: Would use copy-on-write internally; `push` on a shared array triggers a copy
- **Current `mutable` keyword**: Works unchanged; `borrow mutable` adds mutable borrowing
- **`pure` functions**: Auto-clone is ideal for pure functions since they shouldn't mutate inputs anyway

## Implementation Difficulty

**Medium (6-10 months)**

- Move analysis is simpler than full borrow checking
- Auto-clone insertion is a straightforward compiler pass
- Copy-on-write for strings/arrays requires runtime support but is well-understood
- No lifetime inference engine needed
- The `move` keyword is a simple liveness check (already done in most compilers)
- Main complexity: making the clone insertion smart enough to avoid unnecessary copies
