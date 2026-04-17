# Perceus Functional Reference Counting (Koka-Inspired)

## Overview

A **compile-time optimized reference counting** model based on the Perceus algorithm from the Koka language. The compiler performs **reuse analysis** — when an object's reference count is 1 (unique owner), mutations and reconstructions happen **in-place** instead of allocating new memory.

This is the ideal model for an **immutable-by-default, functional-leaning language** like Opalescent. Instead of copying data on every transformation, the compiler detects when the old value is no longer needed and reuses its memory for the new value.

Key insight: In functional code like `let new_list = map(old_list, transform)`, if `old_list` is never used again, its memory can be reused for `new_list` — achieving the same performance as in-place mutation without any mutable syntax.

## Syntax Design

### No New Syntax — Compiler Optimization

```opal
# The developer writes pure functional code
let numbers = [1, 2, 3, 4, 5]
let doubled = map<int32, int32>(numbers, f(n: int32): int32 => n * 2)
# If 'numbers' is never used again, Perceus reuses its memory for 'doubled'
# If 'numbers' IS used again, a copy is made (standard refcount behavior)
print(doubled)
```

### Reuse in Action (Tree Transformations)

```opal
# Incrementing every node in a tree:
let increment_tree = f(tree: IntTree): IntTree =>
    match tree:
        Node(node):
            return IntTree.Node{
                value: node.value + 1,
                left: increment_tree(node.left),
                right: increment_tree(node.right)
            }
        Leaf:
            return IntTree.Leaf

# If the input tree is uniquely owned (refcount = 1),
# Perceus reuses each Node allocation in-place.
# A tree of 1 million nodes is updated with ZERO allocations.
```

### `drop` Keyword for Explicit Early Release

```opal
# Explicitly drop a value to enable reuse earlier
let process = f(data: string[]): string[] =>
    let intermediate = expensive_transform(data)
    drop data                        # explicitly release — enables reuse
    let result = finalize(intermediate)
    drop intermediate                # release before return
    return result
```

### `unique` Type Annotation (Optional, for Guarantees)

```opal
# Guarantee a value is uniquely owned — compiler enforces it
let consume = f(data: unique string[]): string =>
    # 'data' is guaranteed to have refcount = 1
    # All operations on it are guaranteed in-place
    let mutable result = ""
    for item in data:
        result = result + item + "\n"
    return result
```

## How Perceus Works

1. **Reference counting at compile time**: The compiler inserts increment/decrement operations, but then optimizes most of them away
2. **Reuse analysis**: When a value is deconstructed (pattern matched) and reconstructed (new value created with same type), the compiler checks if the original is uniquely owned
3. **In-place update**: If uniquely owned, the memory is mutated in-place instead of allocating new memory
4. **Drop specialization**: The compiler generates specialized drop code for each type, avoiding dynamic dispatch

## Example Applications

See companion `.op` files:

- `functional_transforms.op` — map, filter, tree transforms with reuse
- `real_world_pipeline.op` — data processing pipeline with Perceus optimization

## Strengths

1. **Perfect for immutable-by-default**: Functional transformations get in-place performance for free
2. **No GC pauses**: Deterministic reference counting — no stop-the-world
3. **No new syntax required**: Pure compiler optimization over standard functional code
4. **Competitive with mutation**: Uniquely-owned transformations are as fast as in-place mutation
5. **Proven in research**: Perceus has published benchmarks showing competitive performance with C
6. **Cycle-free by convention**: Immutable data structures are acyclic by construction
7. **Predictable memory usage**: Refcounting means memory is freed immediately when unused
8. **Aligns with `pure` keyword**: Pure functions are exactly where Perceus shines — functional transforms with automatic reuse
9. **`drop` is simple**: Unlike lifetimes, `drop` is just "I'm done with this" — easy to teach

## Weaknesses

1. **Cycles still leak**: Mutable cyclic structures (when they occur) aren't collected. Needs a cycle collector backup or `weak` refs
2. **Reuse analysis has limits**: The compiler can only reuse when types match exactly and ownership is unique
3. **Shared values can't be reused**: When multiple references exist, copies are still needed
4. **Reference counting overhead**: Increment/decrement operations, even optimized, add cost vs. a tracing GC's zero-cost allocation
5. **Novel technique**: Fewer real-world implementations to reference — Koka is the primary example
6. **Compiler complexity**: Reuse analysis and drop specialization are sophisticated compiler passes
7. **Less predictable for non-functional code**: Imperative-style code with lots of mutation doesn't benefit as much

## Impact on Existing Syntax

- **Zero required changes**: Existing Opalescent code benefits from Perceus optimization automatically
- **Optional keywords**: `drop` (explicit early release), `unique` (ownership guarantee) — both additive
- **`pure` functions**: Benefit most — the compiler can aggressively optimize pure functional transforms
- **Pattern matching**: Becomes the primary mechanism for reuse detection — matching + reconstructing = in-place reuse
- **Current `mutable` keyword**: Unchanged; mutable bindings work as before, Perceus optimizes the functional path

## Implementation Difficulty

**High (10-14 months)**

- Perceus reference counting insertion is well-described in the academic paper
- Reuse analysis requires tracking allocation sizes and types through the compiler
- Drop specialization generates type-specific drop code — needs integration with the type system
- The "frame-limited" reuse optimization (reusing across function calls) is particularly complex
- Must handle interaction with C ABI and hot reload
- Can be implemented incrementally: basic refcounting first, then add reuse analysis as an optimization pass
- Primary reference: "Perceus: Garbage Free Reference Counting with Reuse" (Reinking et al., 2021)

## Performance Characteristics

| Pattern | Perceus Behavior | vs. Naive Copy | vs. In-Place Mutation |
|---------|-----------------|----------------|----------------------|
| map over unique list | In-place reuse | O(1) vs O(n) alloc | Same performance |
| filter unique list | In-place reuse | O(1) vs O(n) alloc | Same performance |
| tree transform (unique) | Node-by-node reuse | Zero allocations | Same performance |
| shared list transform | Full copy | Same | Slower (copy needed) |
| append to unique list | In-place | No realloc | Same performance |
