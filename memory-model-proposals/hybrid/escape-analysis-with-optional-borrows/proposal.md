# Escape Analysis with Optional Borrow Annotations

## Overview

A **hybrid model** combining Go-style automatic escape analysis with optional Rust-like borrow annotations for performance-critical paths. The compiler automatically decides whether values live on the stack or heap — developers only intervene when profiling shows it matters.

This is the "easy by default, optimizable when needed" approach. 90% of code reads like a GC language. The remaining 10% (hot loops, latency-sensitive paths) can opt into borrow annotations for zero-copy performance.

The key innovation: combine escape analysis (automatic) with **opt-in `borrow` annotations** (manual) and **scope-based deterministic destruction** (no GC).

## Syntax Design

### Default: Automatic Stack/Heap Decision

```opal
# Compiler analyzes whether values escape their scope
let process = f(n: int32): string =>
    let label = 'item-{n}'        # does this escape? Compiler checks:
    return label                   # yes — placed on heap (Arc/COW)

let compute = f(n: int32): int32 =>
    let temp = n * n + 1          # does this escape? No — stays on stack
    let result = temp + 5
    return result                  # primitive — always stack
```

### Opt-in Borrow Annotations (For Performance)

```opal
# When profiling shows a hot path, add borrow annotations
let sum_lengths = f(items: borrow string[]): int32 =>
    let mutable total = 0
    for item in items:             # 'item' borrows from 'items' — no copy
        total = total + item.len()
    return total

# Without 'borrow', the compiler might auto-clone items into the function
# With 'borrow', it's guaranteed zero-copy
```

### Scope-Based Destruction (No GC)

```opal
let handle_file = f(path: string): string =>
    let file = open_file(path)     # resource allocated
    let content = file.read_all()
    return content
    # 'file' deterministically closed here — scope exit triggers cleanup
    # No GC needed — stack unwinding handles it
```

### `inline` Hint for Hot Paths

```opal
# Suggest the compiler inline and stack-allocate aggressively
inline let dot_product = f(a: borrow float64[], b: borrow float64[]): float64 =>
    let mutable sum = 0.0
    let mutable i = 0
    while i < a.len():
        sum = sum + a[i] * b[i]
        i = i + 1
    return sum
```

### `heap` Annotation for Explicit Heap Allocation

```opal
# Force heap allocation when you know a value will be shared
let create_shared_config = f(): heap AppConfig =>
    return AppConfig{ host: "0.0.0.0", port: 8080, timeout_ms: 5000 }
    # Explicitly on heap — can be shared across threads/scopes
```

## Example Applications

See companion `.op` files:

- `automatic_mode.op` — normal code with zero annotations
- `optimized_hot_path.op` — performance-critical code with borrow annotations

## Strengths

1. **Progressive complexity**: Start simple, add annotations only where profiling demands it
2. **Familiar to Go developers**: Escape analysis is invisible — code reads like a GC language
3. **No GC pauses**: Scope-based destruction is deterministic — no runtime collector
4. **Borrow annotations for power users**: When you need zero-copy, you can have it without switching languages
5. **Compiler does the work**: 90%+ of stack-vs-heap decisions are automatic
6. **Enterprise-friendly ramp**: New developers write normal code; performance tuning is additive
7. **Immutability synergy**: Immutable values that don't escape are trivially stack-allocated
8. **Best balance of goals**: Easier than Rust, potentially faster than Go, safer than C

## Weaknesses

1. **Escape analysis is imperfect**: Compiler may heap-allocate conservatively when stack would suffice
2. **Two mental models**: Developers eventually need to understand both automatic mode and borrow mode
3. **Non-obvious performance**: Whether something is stack or heap allocated isn't visible in the code
4. **Borrow annotations can cascade**: Adding `borrow` to one function may require changes in callers/callees
5. **Scope-based destruction has limits**: Shared ownership (multiple owners) still needs refcounting
6. **Escape analysis quality varies**: The optimization ceiling depends heavily on compiler sophistication
7. **Harder to guarantee performance**: Without explicit ownership, you can't guarantee zero allocations in a code path

## Impact on Existing Syntax

- **Low impact for basic code**: Existing Opalescent code works unchanged
- **New keywords**: `borrow` (opt-in parameter annotation), `inline` (hint), `heap` (explicit allocation)
- **Destruction semantics**: Values are freed at scope exit (like current `Arc` drop, but generalized)
- **`mutable` keyword**: Unchanged
- **Function signatures**: Optional `borrow` in parameters; return types unchanged

## Implementation Difficulty

**Medium-High (8-12 months)**

- Escape analysis is a well-understood compiler optimization (Go has a production implementation)
- Scope-based destruction requires careful ordering of drops (similar to Rust's drop order)
- Borrow annotations need a simplified borrow checker (function-boundary only, no lifetime inference)
- Must handle the interaction between automatic mode and annotated mode
- Shared ownership (values that escape to multiple scopes) still needs `Arc` or similar
- The `inline` and `heap` annotations are hints — implementation is optional for MVP
- Can be implemented incrementally: escape analysis first, borrow annotations later
