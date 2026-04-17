# Region-Based Memory Management

## Overview

Values are allocated into **regions** (also called arenas) — contiguous memory blocks that are freed all at once when the region's scope ends. Instead of tracking individual object lifetimes, the compiler assigns objects to regions and deallocates entire regions at scope boundaries.

Inspired by MLKit, Cyclone, and the arena allocation pattern used in game engines and high-performance servers. Regions give **deterministic, bulk deallocation** with **zero per-object overhead**.

The key insight for Opalescent: since most functions are `pure` and operate on immutable data, the compiler can automatically assign all allocations within a pure function to a single region that's freed when the function returns. No annotations needed for the common case.

## Syntax Design

### Automatic Regions (Default — No Syntax)

```opal
# The compiler assigns a region to each scope automatically
let process = f(items: string[]): string =>
    # All allocations in this function go into an implicit region
    let mutable result = ""
    for item in items:
        let formatted = '[{item}]'     # allocated in this function's region
        result = result + formatted
    return result
    # region freed here — all temporaries deallocated at once
```

### Explicit Regions for Control

```opal
# 'region' block creates a named arena
let process_batch = f(items: Request[]): Response[] =>
    let mutable results: Response[] = []

    for item in items:
        region per_item:                    # explicit region for each iteration
            let parsed = parse(item)        # allocated in 'per_item' region
            let validated = validate(parsed)
            let response = handle(validated)
            results.push(response)          # 'response' escapes → copied to outer region
        # 'per_item' freed here — parsed, validated freed in bulk
        # No accumulating garbage across loop iterations

    return results
```

### Region Parameters for Libraries

```opal
# Functions can accept a region parameter for allocation control
let build_index = f(data: string[], in region r): StringIndex =>
    let mutable index = StringIndex.new(in r)
    for item in data:
        index.insert(item, in r)            # allocates in caller's region
    return index
```

### `static` Region for Long-Lived Data

```opal
# Data that lives for the entire program
let config = static AppConfig{
    host: "0.0.0.0",
    port: 8080,
    timeout_ms: 5000
}
# Never freed — lives for program lifetime
```

## Example Applications

See companion `.op` files:

- `automatic_regions.op` — implicit region assignment, zero annotations
- `explicit_regions.op` — server request handling with explicit per-request regions

## Strengths

1. **Bulk deallocation**: Freeing thousands of objects is a single pointer reset — incredibly fast
2. **Zero per-object overhead**: No refcount, no GC metadata, no headers per object
3. **Cache-friendly**: Objects in the same region are contiguous in memory — great for performance
4. **Deterministic**: Regions are freed at scope exit — predictable and debuggable
5. **Ideal for request-response patterns**: Allocate everything per-request, free it all when the response is sent — the dominant enterprise pattern
6. **Pure function synergy**: Pure functions have clear allocation scopes — perfect for automatic region assignment
7. **No cycles problem**: Cycles within a region are freed when the region is freed — no leak
8. **Server-class performance**: Arena allocation is the secret weapon of high-performance servers (nginx, game engines)

## Weaknesses

1. **Memory waste**: A region can't free individual objects — if one object in a large region lives long, everything stays alive
2. **Escape analysis needed**: The compiler must detect when a value escapes its region (returned, stored in outer scope) and copy it
3. **Hard to share across scopes**: Data shared between unrelated scopes needs explicit region assignment or copying
4. **Region parameter complexity**: Library functions that allocate need to know which region to use — can clutter signatures
5. **Mutable shared data is tricky**: Long-lived mutable caches don't fit the "allocate, use, free-all" pattern
6. **Less familiar**: Most developers have never used region-based memory — learning curve for explicit regions
7. **Fragmentation**: If regions are long-lived, internal fragmentation can waste memory
8. **Debug complexity**: Use-after-region-free bugs are possible if an escaped reference is missed

## Impact on Existing Syntax

- **Low impact for basic code**: Automatic regions are invisible — existing code benefits without changes
- **New keywords**: `region` (explicit arena block), `in region` (allocation target), `static` (permanent allocation)
- **`pure` functions**: Automatically get single-region allocation — no syntax needed
- **`mutable` keyword**: Mutable values in a region work normally; mutation doesn't change allocation strategy
- **Return values**: Returning a value from a region triggers a copy to the outer region — this is implicit

## Implementation Difficulty

**High (10-14 months)**

- Region inference (automatically assigning objects to regions) is well-studied but complex to implement correctly
- Escape detection must be precise — missing an escape causes use-after-free
- The `region` keyword and `in region` parameter syntax require parser and type system changes
- Integration with C ABI requires region-aware allocation functions
- Hot reload interaction: regions need to be compatible with versioned module swapping
- Can reference MLKit's implementation for region inference algorithms
- Alternative: start with implicit regions only (per-function scope) and add explicit regions later
- Region-based allocation itself is trivial (bump allocator) — the hard part is the compiler analysis
