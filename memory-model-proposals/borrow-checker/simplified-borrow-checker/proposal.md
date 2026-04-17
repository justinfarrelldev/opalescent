# Simplified Borrow Checker

## Overview

A Rust-inspired ownership and borrowing system with **aggressive lifetime elision** — the compiler infers lifetimes in 95%+ of cases. Uses readable keywords (`ref`, `mutable ref`, `own`) instead of Rust's sigil-heavy `&`, `&mut`, `'a` syntax.

The key insight: Rust's borrow checker is powerful but its syntax is the pain point, not the concept. Opalescent can keep the safety guarantees while making the syntax self-explanatory.

## Syntax Design

### Ownership

All values are owned by default. Assignment moves ownership (like Rust):

```opal
let name = "Alice"
let greeting = name    # 'name' moved into 'greeting'; 'name' is no longer valid
# print(name)          # Compile error: 'name' has been moved
```

### Borrowing with `ref` and `mutable ref`

```opal
let greet = f(name: ref string): void =>
    print('Hello {name}')
    return void

let append_exclaim = f(name: mutable ref string): void =>
    name.push('!')
    return void
```

### Automatic Elision (No Lifetime Annotations 95% of the Time)

The compiler infers lifetimes automatically when:

- A function takes one `ref` parameter → output lifetime matches it
- A function takes `self` → output lifetime matches `self`
- All references in a struct come from the same scope

```opal
# No lifetime annotations needed — compiler infers that the return
# borrows from 'items'
let first = f<T>(items: ref T[]): ref T =>
    return items[0]

# No annotation needed — single ref input
let longest_line = f(text: ref string): ref string =>
    let mutable best = ""
    for line in text.lines():
        if line.len() > best.len():
            best = line
    return best
```

### Explicit Lifetimes (Rare Edge Cases Only)

Only needed when the compiler genuinely can't infer which input a returned reference borrows from:

```opal
# Two ref inputs, returning one — compiler needs help
let pick_longer = f<lifetime a>(x: ref<a> string, y: ref<a> string): ref<a> string =>
    if x.len() >= y.len():
        return x
    return y
```

### Clone Escape Hatch

When you want to explicitly copy instead of move:

```opal
let name = "Alice"
let greeting = name.clone()   # Explicit copy; both remain valid
print(name)                   # Fine — 'name' was not moved
```

## Example Applications

See companion `.op` files:

- `ownership_basics.op` — move semantics, borrowing, returning refs
- `data_structures.op` — structs with borrowed data, builder patterns
- `concurrency.op` — ownership transfer across threads

## Strengths

1. **Maximum safety**: Prevents use-after-free, data races, and dangling references at compile time
2. **Zero-cost abstractions**: No runtime overhead (no GC, no refcounting)
3. **Deterministic destruction**: Resources cleaned up at predictable points
4. **Familiar to Rust developers**: Easier hiring for systems-level work
5. **Readable syntax**: `ref` and `mutable ref` are self-documenting compared to `&` and `&mut`
6. **Immutable-by-default synergy**: Opalescent's existing immutability makes borrow checking *easier* — most borrows are immutable (shared) by default

## Weaknesses

1. **Steep learning curve**: Even simplified, ownership/borrowing is a novel concept for most developers coming from GC languages
2. **Self-referential structs are hard**: Linked lists, graphs, and trees with parent pointers require `unsafe` blocks or arena allocators
3. **Lifetime annotations still leak through**: Even if rare, when they appear they confuse newcomers
4. **Refactoring friction**: Changing a function signature can cascade borrow-check errors through call chains
5. **Fighting the borrow checker**: Enterprise developers may waste time satisfying the checker instead of shipping features
6. **Complex error messages**: Borrow checker errors are notoriously hard to understand

## Impact on Existing Syntax

- **Moderate impact**: All function signatures need to consider ownership. `ref` and `mutable ref` become common keywords
- **Array/string APIs change**: Methods like `push` must take `mutable ref self`, immutable views return `ref` slices
- **`mutable` keyword**: `let mutable` creates a mutable binding; `mutable ref` creates a mutable borrow — consistent use of the same keyword for mutability
- **Pattern matching**: Match arms that bind by reference need `ref` keyword

## Implementation Difficulty

**Very High (12-18 months for a production-quality implementation)**

- Requires implementing a full borrow checker pass in the compiler
- Lifetime inference engine is complex (Rust's NLL took years to stabilize)
- Error message quality requires significant investment
- Must integrate with the existing type system, hot reload, and C ABI
- Testing the borrow checker itself requires extensive property-based testing
- The simplified elision rules need careful design to avoid unsoundness
