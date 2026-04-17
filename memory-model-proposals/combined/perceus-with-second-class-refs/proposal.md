# Perceus + Second-Class References

## Overview

A combination of two complementary models:

- **Perceus reuse analysis** handles all allocation and deallocation automatically — the compiler inserts reference counting and optimizes away copies by reusing memory when values are uniquely owned
- **Second-class references** (`ref` / `mutable ref` in function parameters only) provide zero-copy reads without any lifetime complexity

This is the "compiler does the heavy lifting, developer gets simple opt-in tools" approach. Normal code has zero memory syntax. When a function wants to read data without copying it, it adds `ref` to the parameter — and that's the only memory-related keyword most developers ever see.

### Why These Two Together?

Perceus alone has one weakness: every function call receives an owned copy (or a refcount bump). For large data structures passed to read-only functions, that's wasted work. Second-class references solve exactly that — they let functions borrow data for the duration of the call without touching the refcount. Meanwhile, Perceus handles everything else: deallocation, reuse, and optimization of functional transforms.

## Syntax Design

### Normal Code — Zero Annotations (Perceus Handles It)

```opal
let numbers = [1, 2, 3, 4, 5]
let doubled = map<int32, int32>(numbers, f(n: int32): int32 => n * 2)
# 'numbers' not used again → Perceus reuses its memory for 'doubled'
print(doubled)
```

### Read-Only Functions — `ref` for Zero-Copy (Second-Class References)

```opal
# 'ref' borrows the array — no refcount bump, no copy
let sum = f(items: ref int32[]): int32 =>
    let mutable total = 0
    for item in items:
        total = total + item
    return total

# 'mutable ref' for in-place modification — still parameter-only
let normalize = f(scores: mutable ref float64[], factor: float64): void =>
    let mutable i = 0
    while i < scores.len():
        scores[i] = scores[i] / factor
        i = i + 1
    return void
```

### Combined in Practice

```opal
let process_data = f(raw: int32[]): int32[] =>
    # Step 1: read the data without copying (second-class ref)
    let total = sum(raw)
    let avg = total / raw.len()
    print('Average: {avg}')

    # Step 2: functional transform (Perceus reuse)
    # 'raw' is uniquely owned here → memory reused for 'normalized'
    let normalized = map<int32, int32>(raw, f(n: int32): int32 => n - avg)

    # Step 3: another transform (Perceus reuse again)
    let filtered = filter<int32>(normalized, f(n: int32): boolean => n > 0)

    return filtered
```

### The Rules

1. `ref` and `mutable ref` can only appear on function parameters — never in types, never as return types
2. Functions without `ref` receive owned values (Perceus manages the refcount)
3. Returning data always returns an owned value
4. Inside a `ref` function body, you can pass the ref further down to other `ref` parameters
5. Only one `mutable ref` to a value at a time (enforced at call site)

## Example Applications

See companion `.op` files:

- `functional_with_refs.op` — mixing Perceus transforms with zero-copy reads
- `enterprise_data_layer.op` — realistic service with both patterns

## Strengths

1. **Best of both worlds**: Zero-copy reads (refs) + zero-allocation transforms (Perceus) — covers essentially all performance-critical patterns
2. **Near-zero syntax burden**: `ref` is the only memory keyword most developers encounter, and it's optional
3. **Perfect for immutable-by-default**: Immutable data is read via `ref` (no copy), transformed via Perceus (memory reused)
4. **No lifetime annotations — ever**: Second-class refs are scoped to function calls; Perceus has no lifetime concept
5. **Progressive optimization**: Write simple code first, add `ref` where profiling shows unnecessary copies
6. **Enterprise-friendly**: New developers write zero-annotation code. Senior developers add `ref` for performance
7. **Deterministic destruction**: Refcounting means no GC pauses, no non-deterministic cleanup
8. **Pure function synergy**: `pure` functions get Perceus reuse + can accept `ref` parameters for zero-copy
9. **Fail-fast**: Second-class ref violations are compile-time errors. Perceus has no runtime failure modes
10. **Cycles prevented by construction**: Second-class refs can't be stored (no ref cycles). Perceus refcounting only needs a backup collector for mutable cyclic structures

## Weaknesses

1. **Forced copying on return**: Can't return a reference to borrowed data — must copy subsets (e.g., returning a substring copies it)
2. **No zero-copy views**: Slices/iterators that reference parent data are impossible without copies
3. **Implementation complexity**: Two separate compiler analyses (Perceus reuse + second-class ref checking) must work together correctly
4. **Perceus novelty**: Fewer real-world implementations to reference — primarily Koka
5. **Mutable cycles still leak**: If mutable objects form cycles (rare in immutable-by-default), they need a backup cycle collector or `weak` refs
6. **Reuse analysis limits**: Perceus only reuses when the original value is uniquely owned and types match
7. **Two concepts to learn**: Eventually developers need to understand both Perceus (automatic, invisible) and refs (explicit) — though refs are very simple

## Impact on Existing Syntax

- **Very low impact**: Only `ref` and `mutable ref` are added as parameter annotations — everything else is unchanged
- **Existing code benefits automatically**: Perceus optimization applies to all existing functional patterns without changes
- **`pure` keyword**: Unchanged — pure functions benefit from both Perceus reuse and ref parameter reads
- **`mutable` keyword**: Unchanged — mutable bindings trigger COW-like behavior; `mutable ref` is a separate concept for mutable borrows
- **Pattern matching**: Unchanged — match + reconstruct is Perceus's primary reuse trigger
- **Return types**: Always owned — consistent with current behavior

## Implementation Difficulty

**High (12-16 months total, but can be staged)**

### Phase 1: Second-Class References (3-5 months)

- Parameter-scope borrow tracking
- Aliasing check (no two `mutable ref` to same value)
- Simple — no lifetime inference needed

### Phase 2: Basic Reference Counting (2-3 months)

- Compile-time refcount insertion
- Drop ordering at scope boundaries
- Replaces current `Arc` with compiler-managed refcounting

### Phase 3: Perceus Reuse Analysis (5-8 months)

- Reuse detection for matching types
- Drop specialization per type
- Frame-limited reuse across function calls
- This is the most complex phase but can be added incrementally as an optimization

### Staging Advantage

The two systems are independent enough to implement in phases. Second-class references work immediately without Perceus (just with normal refcounting). Perceus can be added later as a pure compiler optimization — existing code gets faster without changes.
