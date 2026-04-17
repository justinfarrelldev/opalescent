# Swift ARC + Region-Based for Requests

## Overview

A combination of two proven strategies applied to different scopes:

- **Swift-style ARC with COW (copy-on-write)** handles general-purpose memory management — objects, services, shared state, long-lived data structures
- **Region-based allocation** handles request-response patterns — all memory for a request is allocated in a region and freed in one shot when the response is sent

This maps directly to how enterprise servers work: long-lived services use ARC (shared caches, connection pools, configuration), while per-request data (parsing, transformation, response building) uses regions for maximum throughput.

### Why These Two Together?

ARC alone has overhead per-allocation (refcount bumps on every share). Regions alone are too restrictive for long-lived data. Together: regions handle the high-volume short-lived allocations (requests), ARC handles the shared infrastructure. The fast path (request processing) has near-zero memory management overhead.

## Syntax Design

### Long-Lived Data — ARC (Automatic, Invisible)

```opal
# Normal ARC — refcounted, COW on mutation, no annotations needed
type AppConfig:
    db_host: string
    db_port: int32
    max_connections: int32

let load_config = f(path: string): AppConfig =>
    # ARC-managed — lives until last reference drops
    return AppConfig{
        db_host: "localhost",
        db_port: 5432,
        max_connections: 100
    }
```

### Request Processing — `region` Blocks

```opal
# Everything allocated inside 'region' is freed when the block exits
let handle_request = f(req: Request, config: AppConfig): Response =>
    region request_scope:
        let body = parse_json(req.body)
        let user_id = body["user_id"]
        let query = build_query(user_id, config.db_host)
        let result = execute_query(query)

        # Build response — still in the region
        let response_body = serialize_json(result)

        # Return copies the response out of the region
        return Response{ status: 200, body: response_body }
        # ← region freed here: body, user_id, query, result all gone instantly
```

### Shared Data Crossing Region Boundaries

```opal
# ARC data can be read inside regions (no copy)
# Region data escaping the region is automatically copied to ARC

let process_with_cache = f(req: Request, cache: Cache<string, UserProfile>): Response =>
    region:
        let user_id = extract_user_id(req)

        # Reading ARC data inside a region — fine, just a ref
        let cached = cache.get(user_id)
        match cached:
            some(profile):
                return Response{ status: 200, body: serialize(profile) }
            none:
                # Region-local computation
                let profile = fetch_profile(user_id)
                # Storing into ARC cache — copies from region to ARC heap
                cache.set(user_id, profile)
                return Response{ status: 200, body: serialize(profile) }
```

### Weak References for ARC (Cycle Prevention)

```opal
type TreeNode:
    value: string
    children: TreeNode[]
    parent: weak TreeNode?    # weak prevents ARC cycles

let build_tree = f(): TreeNode =>
    let root = TreeNode{ value: "root", children: [], parent: none }
    let child = TreeNode{ value: "child", children: [], parent: weak some(root) }
    root.children.push(child)
    return root
```

## Example Applications

See companion `.op` files:

- `server_with_regions.op` — HTTP server mixing ARC services with region-per-request
- `cache_and_regions.op` — shared ARC cache with region-scoped request processing

## Strengths

1. **Maps directly to server architecture**: Long-lived services (ARC) + short-lived requests (regions) — the two-tier model matches reality
2. **Region performance**: Per-request allocations are bump-allocated and freed in bulk — much faster than individual refcount operations
3. **No GC pauses**: ARC + regions are both deterministic — predictable latency for enterprise workloads
4. **Proven individually**: Swift ARC is battle-tested. Region-based allocation is well-studied (MLKit, Cyclone, web server allocators)
5. **COW for safety**: Shared mutable data uses copy-on-write — safe by default, only copies when needed
6. **Simple mental model**: "Long-lived → ARC. Short-lived → region. That's it."
7. **Gradual adoption**: Regions are opt-in — code without `region` blocks uses ARC everywhere (current behavior, improved)
8. **Fail-fast**: `weak` references trap on invalid access. Region escapes are caught at compile time
9. **Pure function synergy**: Pure functions inside regions benefit from bulk allocation. Pure functions with ARC data benefit from COW
10. **Immutable-by-default synergy**: Immutable ARC data never triggers COW copies. Immutable region data is allocated and freed cheaply

## Weaknesses

1. **Two mental models**: Developers need to understand when to use regions vs when to rely on ARC
2. **Region escape analysis**: The compiler must track what escapes a region (to copy it to ARC) — non-trivial
3. **Not great for non-server workloads**: CLI tools, scripts, and desktop apps don't benefit much from regions
4. **COW overhead**: ARC's copy-on-write has bookkeeping costs — not free even when copies are avoided
5. **Cycle prevention burden**: ARC cycles need manual `weak` annotations — a footgun for complex graphs
6. **Region nesting complexity**: Nested regions with cross-references become confusing
7. **Return copying**: Returning data from a region copies it — large return values are expensive
8. **Implementation complexity**: Two allocators (ARC heap + region bump allocator) with interaction rules

## Impact on Existing Syntax

- **Low-medium impact**: `region` blocks are new syntax. `weak` keyword is new. Everything else unchanged
- **Existing code**: Works as-is — defaults to ARC (improved version of current Arc behavior)
- **`mutable` keyword**: Unchanged — triggers COW on ARC data
- **`pure` keyword**: Unchanged — pure functions work in both ARC and region contexts
- **Pattern matching**: Unchanged
- **Return types**: Always owned — region data is copied out, ARC data is refcounted

## Implementation Difficulty

**High (12-16 months total, staged)**

### Phase 1: Improved ARC with COW (4-6 months)

- Replace current Arc with compiler-managed ARC
- COW semantics for mutable access
- `weak` reference support
- ARC optimizer to elide redundant refcount operations

### Phase 2: Region Allocator (3-4 months)

- Bump allocator for region blocks
- Bulk deallocation on region exit
- Arena-based allocation with alignment support

### Phase 3: Region Escape Analysis (3-4 months)

- Track which values escape a region
- Insert automatic copies from region to ARC at escape points
- Compile-time errors for storing region refs in ARC data

### Phase 4: Optimization (2-3 months)

- Region-aware code generation
- ARC elision inside regions (no refcount for region-only data)
- Cross-region optimization for nested regions

### Staging Advantage

Phase 1 improves the existing model immediately. Regions (Phase 2-3) are purely additive — no existing code changes.
