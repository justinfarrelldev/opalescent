# Enhanced Arc with Cycle Detection

## Overview

An evolution of Opalescent's **current `Arc<T>` reference counting model**, adding an automatic **backup cycle-detecting collector** that runs periodically to reclaim cyclic garbage. This addresses the primary weakness of the current model while preserving its simplicity.

The developer experience is unchanged — no new keywords, no ownership annotations. The cycle collector runs transparently in the background. This is similar to CPython's approach (reference counting + cycle detector) but implemented on top of Rust's `Arc` with atomic operations for thread safety.

## Syntax Design

### No Syntax Changes — Fully Transparent

```opal
# Developers write the same code as today
let name = "Alice"
let names = ["Alice", "Bob"]
print(name)
print(names)
```

### Cycles Are Automatically Handled

```opal
type TreeNode:
    value: int32
    mutable children: TreeNode[]
    mutable parent: TreeNode?        # nullable parent — creates cycle

let build_tree = f(): TreeNode =>
    let mutable root = TreeNode{ value: 1, children: [], parent: none }
    let mutable child = TreeNode{ value: 2, children: [], parent: none }
    child.parent = root              # cycle: child -> root
    root.children.push(child)        # cycle: root -> child
    return root
    # When 'root' goes out of scope and refcount hits zero,
    # the cycle collector eventually reclaims child.parent -> root cycle
```

### Optional: Weak References for Performance-Critical Paths

```opal
# For hot paths, developers can still use explicit weak refs to avoid
# waiting for the cycle collector
type CacheEntry:
    data: string
    mutable back_ref: weak TreeNode?   # weak ref — doesn't prevent collection

let resolve = f(entry: CacheEntry): TreeNode? =>
    return entry.back_ref.upgrade()    # Returns none if already collected
```

### Optional: Cycle Collection Hints

```opal
# In performance-critical sections, suppress the cycle collector
let process_batch = f(items: int32[]): int32 =>
    @suppress_gc    # hint: don't run cycle collector in this function
    let mutable total = 0
    for item in items:
        total = total + item
    return total
```

## Example Applications

See companion `.op` files:

- `transparent_usage.op` — showing that normal code is completely unchanged
- `cyclic_structures.op` — graphs, trees with parent pointers, observer patterns

## Strengths

1. **Zero syntax changes**: Existing Opalescent code works without modification
2. **Zero learning curve**: Developers never think about memory management
3. **Solves the cycle problem**: The biggest weakness of the current model is fixed
4. **Deterministic for non-cycles**: Acyclic structures still get immediate cleanup via refcount
5. **Thread-safe**: `Arc` provides atomic refcounting; cycle collector can run on a background thread
6. **Familiar model**: CPython, Swift, Nim all use similar approaches — well-proven in practice
7. **Immutability advantage**: Immutable objects can never form new cycles after construction, so the collector can skip them

## Weaknesses

1. **Non-deterministic cycle cleanup**: Cyclic structures are cleaned up "eventually," not immediately — bad for resource handles (files, sockets)
2. **GC pauses (small)**: The cycle collector needs to pause or scan; adds latency jitter
3. **No compile-time safety**: Memory bugs from FFI or unsafe code can't be caught at compile time
4. **Atomic refcounting overhead**: `Arc` uses atomic operations for every clone/drop — slower than non-atomic `Rc` for single-threaded code
5. **Throughput ceiling**: Refcounting's per-object overhead means it'll never match a tracing GC's throughput for allocation-heavy workloads
6. **Cycle collector complexity**: Implementing a correct, concurrent cycle detector is non-trivial
7. **`weak` keyword is still manual**: For optimal performance, developers must know when to use weak refs — partially defeats the "transparent" goal

## Impact on Existing Syntax

- **No impact**: Zero syntax changes required
- **Runtime-only change**: The cycle collector is a runtime addition
- **`weak` keyword**: Optional — only for developers who want deterministic cycle cleanup
- **`@suppress_gc` annotation**: Optional performance hint

## Implementation Difficulty

**Medium (4-8 months)**

- Cycle detection algorithm (trial deletion / synchronous cycle collection) is well-documented in literature
- Must integrate with `Arc`'s drop logic to trigger cycle scans
- Background collector thread needs careful synchronization
- Testing cycles across threads requires thorough integration tests
- The "immutable objects skip scan" optimization needs careful implementation
- Well-understood problem — can reference CPython, Swift, and Nim implementations
