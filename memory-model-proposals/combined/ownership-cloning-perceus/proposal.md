# Ownership + Implicit Cloning + Perceus Reuse Analysis

## Overview

A three-layer combination that gives developers the simplicity of a GC language with the performance characteristics of an ownership-based system:

1. **Ownership semantics** — every value has one owner. When ownership transfers, the source is invalidated
2. **Implicit cloning** — when the compiler detects a value is used after being moved, it automatically inserts a clone instead of erroring
3. **Perceus reuse analysis** — the compiler then analyzes the clone/move graph and eliminates unnecessary clones by reusing memory when the original is no longer needed

The key insight: auto-cloning makes the language *correct by default* (no use-after-move errors ever), while Perceus makes it *fast by default* (most auto-clones are optimized away). The developer writes simple code and gets good performance without thinking about memory.

### Why These Three Together?

- Ownership alone (Rust-style) rejects valid programs with confusing errors
- Ownership + auto-cloning alone is correct but potentially slow (hidden copies everywhere)
- Adding Perceus closes the loop: the auto-clones that would have been free moves in Rust are detected and eliminated by Perceus

The result: Rust's performance ceiling with Python's ergonomics.

## Syntax Design

### Simple Case — Ownership is Invisible

```opal
let data = [1, 2, 3, 4, 5]
let doubled = map<int32, int32>(data, f(n: int32): int32 => n * 2)
# 'data' was moved into map → Perceus reuses its memory for 'doubled'
print(doubled)
```

### Use-After-Move — Auto-Clone Kicks In

```opal
let data = [1, 2, 3, 4, 5]

# First use: 'data' is moved into sum()
let total = sum(data)

# Second use: compiler sees 'data' was moved, inserts clone automatically
# BUT — Perceus detects this clone is unnecessary if sum() consumed data
let avg = average(data)

print('Total: {total}, Average: {avg}')
```

### Pipeline — Perceus Eliminates Clones

```opal
let transform = f(input: int32[]): int32[] =>
    # Step 1: input moved into filter
    let filtered = filter<int32>(input, f(n: int32): boolean => n > 0)
    # Perceus: input's memory reused for filtered (same type, uniquely owned)

    # Step 2: filtered moved into map
    let doubled = map<int32, int32>(filtered, f(n: int32): int32 => n * 2)
    # Perceus: filtered's memory reused for doubled

    # Step 3: doubled moved into sort
    let sorted = sort<int32>(doubled)
    # Perceus: doubled's memory reused for sorted

    return sorted
    # Zero total allocations beyond the initial input — all reuse
```

### When Auto-Clone Actually Clones

```opal
let data = [1, 2, 3, 4, 5]

# 'data' used in two different branches — auto-clone makes a real copy
let evens = filter<int32>(data, f(n: int32): boolean => n % 2 is 0)
let odds = filter<int32>(data, f(n: int32): boolean => n % 2 is 1)
# Here Perceus can't optimize — both 'evens' and 'odds' coexist
# The compiler inserts one clone (for the second use of 'data')
# This is the correct behavior — you DO need two copies of the data

print(evens)
print(odds)
```

### Explicit `move` for Performance-Conscious Code

```opal
# Optional: developers can write 'move' to assert single-use
# If the value IS used again, compile error (no auto-clone)
let process = f(data: move int32[]): int32[] =>
    let result = map<int32, int32>(data, f(n: int32): int32 => n * 2)
    # Using 'data' here would be a compile error (explicit move)
    return result
```

## Example Applications

See companion `.op` files:

- `auto_clone_with_reuse.op` — demonstrates the interplay between auto-cloning and Perceus
- `enterprise_pipeline.op` — realistic data processing showing optimization in practice

## Strengths

1. **Zero learning curve for correctness**: No use-after-move errors — auto-clone makes every program valid
2. **Zero learning curve for performance**: Perceus optimizes away most clones — developers get good performance without trying
3. **Progressive performance opt-in**: `move` annotation available for hot paths where developers want compile-time guarantees
4. **Perfect for immutable-by-default**: Immutable values are the ideal case for Perceus reuse analysis
5. **No lifetime annotations — ever**: Ownership is structural, auto-clone handles sharing, no lifetimes needed
6. **Enterprise-friendly progression**: Juniors write simple code (auto-clone). Seniors add `move` for critical paths
7. **Deterministic destruction**: Ownership + refcounting (for shared auto-cloned values) — no GC pauses
8. **Pure function synergy**: Pure functions with single-use parameters get full Perceus reuse
9. **Fail-fast**: `move` annotated parameters fail at compile time on reuse. Auto-clone is always safe at runtime
10. **Transparent cost model**: Compiler can report where auto-clones happen (warnings or diagnostics) — no hidden surprises

## Weaknesses

1. **Hidden performance costs**: Auto-clone can silently insert expensive copies — developers may not realize until profiling
2. **Perceus doesn't catch everything**: Multi-use values, different types, and non-uniquely-owned values still clone
3. **Implementation complexity**: Three interacting analyses (ownership tracking, auto-clone insertion, Perceus reuse elimination) are complex
4. **Diagnostic challenge**: "Did Perceus optimize this?" is hard to answer without tooling — invisible optimizations are hard to reason about
5. **Perceus novelty**: Still a relatively new technique (Koka) — less reference material than ARC or borrow checking
6. **Mutable cycle risk**: Auto-cloning reduces the risk (clones break sharing), but mutable cyclic structures can still theoretically leak
7. **Optimization variability**: Small code changes can flip a Perceus reuse into a clone — performance can be fragile
8. **Two competing concepts**: "Ownership" suggests Rust-like strictness, but "auto-clone" undermines it — the messaging is tricky

## Impact on Existing Syntax

- **Zero mandatory impact**: No new keywords required for basic usage
- **Optional `move` keyword**: Added as a parameter annotation for explicit ownership transfer (opt-in)
- **Existing code**: All existing code works as-is — auto-clone handles any move conflicts
- **`mutable` keyword**: Unchanged — mutable values trigger COW when auto-cloned
- **`pure` keyword**: Unchanged — pure functions get the strongest Perceus optimization
- **Pattern matching**: Unchanged — destructuring triggers Perceus reuse naturally
- **Compiler diagnostics**: New: optional warnings for "auto-clone inserted here" and "Perceus reuse applied here"

## Implementation Difficulty

**High (12-16 months total, staged)**

### Phase 1: Ownership Tracking (3-4 months)

- Move semantics in the compiler
- Detect use-after-move (but don't error — mark for auto-clone)
- Ownership transfer in function calls, assignments, pattern matching

### Phase 2: Auto-Clone Insertion (2-3 months)

- Insert clone calls at use-after-move points
- Handle nested structures (deep clone vs shallow clone decisions)
- COW integration for mutable data
- Optional compiler warning: "auto-clone inserted at line X"

### Phase 3: Perceus Reuse Analysis (5-8 months)

- Drop-point analysis for uniquely-owned values
- Reuse credit passing (Koka-style)
- Type-matching for in-place reuse
- Frame-limited reuse across function boundaries

### Phase 4: `move` Annotation + Diagnostics (1-2 months)

- `move` parameter annotation with compile-time enforcement
- Diagnostic output: clone/reuse report per function
- IDE integration: inline hints for clone vs reuse

### Staging Advantage

Phase 1+2 deliver a working language immediately (ownership + auto-clone = always correct, just not optimized). Phase 3 (Perceus) is a pure optimization pass — existing code gets faster without changes. Phase 4 is tooling polish.
