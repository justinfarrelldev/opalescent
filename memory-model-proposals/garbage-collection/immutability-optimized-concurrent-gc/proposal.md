# Immutability-Optimized Concurrent GC

## Overview

A **tracing garbage collector** purpose-built for Opalescent's immutable-by-default design. Since most objects are immutable after construction, the GC can exploit this for massive optimization:

- Immutable objects need no write barriers (no field updates to track)
- Immutable objects can be freely shared across threads with zero synchronization
- The GC only needs to track mutable objects closely
- Young-generation immutable objects can be promoted to "permanent" status quickly

This is Go's concurrent GC philosophy, but with Opalescent's immutability guarantees providing optimizations Go can never achieve.

## Syntax Design

### No Syntax Changes for Basic Use

```opal
# Developers never think about memory — it just works
let name = "Alice"
let numbers = [1, 2, 3, 4, 5]
let result = map<int32, int32>(numbers, f(n: int32): int32 => n * n)
# Everything is collected automatically when unreachable
```

### Mutable Objects Are the Exception

```opal
# Mutable objects are tracked more closely by the GC
let mutable cache: string[] = []
cache.push("entry-1")
cache.push("entry-2")
# The 'mutable' keyword already exists — the GC uses it as a hint
# to apply write barriers only to mutable bindings
```

### Optional: `@nogc` for Performance-Critical Sections

```opal
# For latency-sensitive code, suppress GC during a critical section
let process_frame = f(frame: FrameData): RenderResult =>
    @nogc                           # GC will not pause this function
    let vertices = transform(frame.mesh)
    let pixels = rasterize(vertices)
    return RenderResult{ data: pixels }
    # GC resumes after function returns
```

### Optional: `@prealloc` for Allocation Budgets

```opal
# Hint to the GC to pre-allocate a region for batch operations
let process_batch = f(items: Request[]): Response[] =>
    @prealloc(items.len() * 256)    # pre-allocate ~256 bytes per item
    let mutable results: Response[] = []
    for item in items:
        results.push(handle(item))
    return results
```

## Example Applications

See companion `.op` files:

- `basic_usage.op` — normal code, zero memory management
- `server_workload.op` — realistic server with many short-lived allocations

## Strengths

1. **Zero cognitive overhead**: Developers never think about memory — like Go, Java, C#
2. **Immutability optimization**: Opalescent's core feature becomes a performance advantage — immutable objects are cheaper to GC than in any other language
3. **Concurrent collection**: GC runs alongside application threads with minimal pauses
4. **Excellent throughput**: Tracing GCs handle high allocation rates better than refcounting
5. **Cycles are free**: No cycle problem — the tracer handles all reachability
6. **Enterprise developer familiarity**: Most enterprise devs come from GC'd languages
7. **Simpler compiler**: No ownership analysis, no borrow checking, no lifetime inference

## Weaknesses

1. **GC pauses**: Even concurrent GCs have stop-the-world phases — tail latencies suffer
2. **Memory overhead**: GC needs 2-3x the live data as headroom to operate efficiently
3. **Non-deterministic destruction**: Can't rely on objects being freed at scope exit — bad for file handles, network connections, etc.
4. **Slower than Rust**: GC overhead makes it fundamentally slower — conflicts with "faster than Go" goal
5. **Runtime complexity**: A concurrent, generational GC is a massive runtime component
6. **Harder to profile**: Memory issues manifest as GC pauses rather than leaks — different debugging skills needed
7. **Hot reload interaction**: GC heap state across hot reloads is complex — need to handle stale object graphs
8. **C ABI friction**: GC'd objects can't be passed directly to C code without pinning

## Impact on Existing Syntax

- **Zero impact on syntax**: No new keywords required
- **Runtime-only change**: Replace `Arc` with GC-managed heap
- **`mutable` keyword gains significance**: GC optimizes differently for mutable vs immutable objects
- **Optional annotations**: `@nogc`, `@prealloc` are hints, not requirements
- **Resource management**: Need to add `defer` or `using` blocks for deterministic cleanup of non-memory resources

## Implementation Difficulty

**Very High (12-18 months)**

- A concurrent, generational GC is one of the hardest runtime components to build correctly
- Write barrier implementation must be integrated into all mutable field assignments
- Safepoint insertion in generated code
- Stack map generation for precise GC (knowing which stack slots hold pointers)
- Interaction with C ABI requires object pinning and root registration
- Testing concurrent GC for correctness requires specialized tooling (stress tests, race detectors)
- Alternative: use an existing GC library (e.g., Boehm GC for conservative collection) to reduce implementation time to 4-6 months, at the cost of optimization potential

## Performance Comparison

| Metric | This Model | Current Arc | Rust Borrow Checker |
|--------|-----------|-------------|-------------------|
| Allocation speed | Fast (bump allocator) | Medium (malloc) | Medium (malloc) |
| Collection pauses | Yes (1-10ms) | None | None |
| Memory overhead | 2-3x live data | ~0% | ~0% |
| Cycle handling | Automatic | Leaks | N/A (prevented) |
| Throughput (alloc-heavy) | Excellent | Good | Good |
| Latency (p99) | Worse | Better | Best |
