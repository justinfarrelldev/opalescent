# Swift-Style ARC with Copy-on-Write

## Overview

Compile-time optimized **Automatic Reference Counting (ARC)** with **copy-on-write (COW) value semantics** for collections. Inspired by Swift's memory model, adapted for Opalescent's immutable-by-default philosophy.

The compiler analyzes retain/release patterns and eliminates redundant reference count operations at compile time. Combined with COW, most "copies" are free until mutation actually happens — and since Opalescent is immutable by default, mutation is rare, making COW extremely effective.

Adds `weak` and `unowned` keywords for explicit reference strength control.

## Syntax Design

### Value Semantics by Default (COW Under the Hood)

```opal
let names = ["Alice", "Bob", "Charlie"]
let also_names = names             # NOT a deep copy — shares storage via COW
# Both 'names' and 'also_names' point to the same underlying memory
# A copy only happens if one of them is mutated

let mutable editable = names       # still shares storage
editable.push("Diana")            # NOW a copy is triggered (COW)
# 'names' is unchanged; 'editable' has its own copy
```

### `weak` References

```opal
type Delegate:
    name: string
    mutable handler: weak Controller?   # weak — doesn't keep Controller alive

type Controller:
    label: string
    mutable delegate: Delegate?

let setup = f(): Controller =>
    let mutable ctrl = Controller{ label: "main", delegate: none }
    let mutable del = Delegate{ name: "handler", handler: none }

    del.handler = weak ctrl         # explicit weak assignment
    ctrl.delegate = del

    return ctrl
    # 'del.handler' becomes none when ctrl is deallocated
```

### `unowned` References (Non-Optional, Unsafe if Dangling)

```opal
type Child:
    name: string
    parent: unowned Parent          # guaranteed non-null; crashes if parent dies first

type Parent:
    name: string
    mutable children: Child[]

# Use 'unowned' when you KNOW the parent outlives the child
# Avoids the overhead of weak reference checking
# Traps at runtime if the referenced object is already freed (fail-fast)
```

### Compiler Optimizations (Invisible to Developer)

```opal
let process = f(data: string): string =>
    let upper = data.to_upper()    # compiler sees 'data' is never used again
    return upper                   # elides the retain on 'data', moves instead
    # No reference counting overhead for this function — compiler optimized it away
```

## Example Applications

See companion `.op` files:

- `value_semantics.op` — COW behavior, efficient sharing
- `reference_strength.op` — weak, unowned, delegate patterns

## Strengths

1. **Feels like value semantics**: Developers think in terms of values, not references — intuitive and safe
2. **Excellent for immutable-by-default**: COW almost never triggers copies since most values aren't mutated
3. **Deterministic destruction**: Objects freed immediately when last reference drops — great for resource management
4. **Compiler eliminates overhead**: Swift's ARC optimizer removes 60-90% of retain/release calls in practice
5. **`weak` and `unowned` are explicit and greppable**: Easy to audit reference cycles in code review
6. **Battle-tested**: Swift has proven this model works at enormous scale (iOS ecosystem)
7. **Thread-safe by default**: Atomic refcounting + COW means shared immutable data is inherently safe
8. **Fail-fast on `unowned`**: Accessing a freed `unowned` ref traps immediately — aligns with Opalescent's fail-fast philosophy

## Weaknesses

1. **Retain/release overhead**: Even with compiler optimization, hot loops with many small objects pay refcount costs
2. **COW can surprise**: A mutation deep in a call chain can trigger an unexpected O(n) copy
3. **`unowned` is unsafe**: If the referenced object dies first, you get a runtime crash — trading compile-time safety for ergonomics
4. **Cycle prevention is manual**: Developers must remember to use `weak` for back-references — compiler doesn't enforce it
5. **ARC optimizer complexity**: Building a good retain/release optimizer is significant compiler engineering
6. **Not as fast as borrow checker**: Runtime refcounting, even optimized, is slower than zero-cost borrows
7. **Atomic overhead for single-threaded code**: Paying for thread safety even when not needed

## Impact on Existing Syntax

- **Low-moderate impact**: Existing code works unchanged; COW replaces current `Arc` semantics transparently
- **New keywords**: `weak`, `unowned` — both optional and used only when breaking cycles
- **Collection semantics change**: Arrays/strings gain value semantics (COW) instead of shared-reference semantics
- **`mutable` keyword**: Becomes the trigger for COW copies — `let mutable x = shared_array` potentially copies on first mutation
- **Pattern matching**: Unchanged

## Implementation Difficulty

**Medium-High (8-12 months)**

- COW implementation for strings and arrays requires careful reference count checking before mutation
- ARC optimizer (eliding redundant retain/release) is a significant compiler pass — can reference Swift's SIL optimizer
- `weak` references need zeroing (automatically set to `none` when target is freed) — requires a side table or similar mechanism
- `unowned` is simpler (just a raw pointer with a debug-mode check)
- Must integrate with C ABI for hot reload — COW state needs to be compatible
- Testing COW edge cases (mutation during iteration, nested COW structures) requires extensive test suite
