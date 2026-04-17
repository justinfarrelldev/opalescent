# Second-Class References (Austral-Inspired)

## Overview

References can **only exist as function parameters** — they can never be stored in structs, returned from functions, or placed in collections. This entirely eliminates the need for lifetime annotations while still providing zero-copy reads via borrowing.

Inspired by Austral and earlier academic work on "second-class values." The restriction sounds severe, but in practice it covers the vast majority of borrowing use cases (reading data without copying it) while making the ownership model trivially simple.

## Syntax Design

### Borrowing in Function Parameters Only

```opal
# 'ref' borrows — only valid as a parameter type
let count_words = f(text: ref string): int32 =>
    return text.split(' ').len()

# 'mutable ref' for mutable borrows — also parameter-only
let append = f(list: mutable ref int32[], value: int32): void =>
    list.push(value)
    return void
```

### Cannot Store References

```opal
# COMPILE ERROR: ref cannot appear in a type definition
# type BadView:
#     data: ref string[]    # Error: references are second-class

# Instead: types always own their data
type GoodView:
    data: string[]          # owned copy
    start: int32
    end: int32
```

### Cannot Return References

```opal
# COMPILE ERROR: cannot return a ref
# let first = f(items: ref int32[]): ref int32 =>
#     return items[0]

# Instead: return an owned copy
let first = f(items: ref int32[]): int32 =>
    return items[0]   # copies the int32 out (cheap for primitives)
```

### Multiple Borrows Are Fine (Immutable by Default)

```opal
# Multiple immutable refs are always safe
let compare = f(a: ref string, b: ref string): boolean =>
    return a.len() > b.len()

# But only one mutable ref at a time (enforced at call site)
let mutable data = [1, 2, 3]
append(data, 4)              # Fine — single mutable ref
# double_append(data, data)  # Compile error: two mutable refs to same value
```

### Working with Nested Function Calls

```opal
let process_items = f(items: ref int32[]): int32 =>
    # Can pass the borrow further down — still second-class
    let total = sum(items)
    let count = items.len()
    return total / count

let sum = f(items: ref int32[]): int32 =>
    return reduce<int32, int32>(items, 0, f(acc: int32, n: int32): int32 => acc + n)
```

## Example Applications

See companion `.op` files:

- `basic_borrowing.op` — parameter-level borrows, reads without copies
- `enterprise_service.op` — realistic service layer showing how second-class refs work at scale

## Strengths

1. **Trivially simple**: No lifetime annotations, no complex borrow rules, no "fighting the borrow checker"
2. **Zero-copy where it matters most**: Function parameters are the primary use case for borrowing
3. **Easy to teach**: "You can look at data without copying it, but you can't keep a reference to it" — one sentence explains the model
4. **Sound by construction**: The restriction makes use-after-free impossible without any complex analysis
5. **Enterprise-scale friendly**: New developers productive in minutes, not days
6. **Immutable-by-default synergy**: Most borrows are immutable reads, which is exactly what `ref` provides
7. **Simple implementation**: No lifetime inference, no NLL analysis — just parameter-scope tracking

## Weaknesses

1. **Forced copying**: When you need to return a subset of borrowed data, you must copy it. For large strings/arrays, this can be expensive
2. **No zero-copy views**: Slices, string views, and iterators that reference parent data are impossible without copies
3. **Graph/tree structures are expensive**: Traversals that need to "remember" references to nodes must copy node data
4. **Less optimization potential**: Can't hold borrows across multiple operations; must re-borrow each time
5. **Performance ceiling**: For hot-path code that needs to avoid allocations, the model forces copies that Rust wouldn't
6. **Verbose for some patterns**: Functions that operate on subsets of data need to pass indices rather than sub-slices
7. **May feel limiting to experienced systems programmers**: "I know this is safe, why won't it let me?"

## Impact on Existing Syntax

- **Very low impact**: Existing code works as-is; `ref` is purely additive
- **No new type-level concepts**: Types always own their data (current behavior)
- **Function signatures**: Add optional `ref` / `mutable ref` to parameters for zero-copy
- **Cannot change return types**: Functions always return owned values (current behavior)
- **`mutable` keyword**: Consistent — `let mutable` for mutable bindings, `mutable ref` for mutable borrows

## Implementation Difficulty

**Low (3-5 months)**

- No lifetime inference needed
- Borrow tracking is purely scope-based (function boundary = borrow boundary)
- Aliasing check (no two `mutable ref` to same value) is a simple call-site analysis
- Integrates cleanly with existing type system — just a parameter-position constraint
- Error messages are straightforward: "references cannot be stored / returned"
- Simpler than the current `Arc` implementation in some ways
