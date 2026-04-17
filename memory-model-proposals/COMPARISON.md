# Memory Model Proposals — Comparison & Recommendations

## Overview

Eleven memory model proposals organized into five categories, each evaluated against Opalescent's core goals:

1. **Maintainability at enterprise scale**
2. **Faster than Go, slower than Rust**
3. **Safer than Go, marginally less safe than Rust**
4. **Easy syntax (simpler than Rust)**
5. **Immutable-by-default / pure-function synergy**
6. **Fail-fast mentality**

---

## Quick Comparison

| Proposal | Safety | Performance | Syntax Simplicity | Learning Curve | Implementation | Cycle Handling |
|----------|--------|-------------|-------------------|----------------|----------------|----------------|
| [Simplified Borrow Checker](borrow-checker/simplified-borrow-checker/) | ★★★★★ | ★★★★★ | ★★☆☆☆ | Hard | Very High (12-18mo) | Prevented |
| [Ownership + Implicit Cloning](borrow-checker/ownership-with-implicit-cloning/) | ★★★★☆ | ★★★★☆ | ★★★★☆ | Easy | Medium (6-10mo) | Need backup |
| [Second-Class References](borrow-checker/second-class-references/) | ★★★★★ | ★★★☆☆ | ★★★★★ | Very Easy | Low (3-5mo) | Prevented |
| [Enhanced Arc + Cycle Collector](reference-counting/enhanced-arc-with-cycle-collector/) | ★★★☆☆ | ★★★☆☆ | ★★★★★ | None | Medium (4-8mo) | Automatic |
| [Swift-Style ARC + COW](reference-counting/swift-style-arc-with-cow/) | ★★★★☆ | ★★★★☆ | ★★★★☆ | Easy | Med-High (8-12mo) | Manual (weak) |
| [Immutability-Optimized GC](garbage-collection/immutability-optimized-concurrent-gc/) | ★★★☆☆ | ★★☆☆☆ | ★★★★★ | None | Very High (12-18mo) | Automatic |
| [Escape Analysis + Optional Borrows](hybrid/escape-analysis-with-optional-borrows/) | ★★★★☆ | ★★★★☆ | ★★★★☆ | Easy-Medium | Med-High (8-12mo) | Need backup |
| [Perceus Functional RC](hybrid/perceus-functional-reuse-analysis/) | ★★★★☆ | ★★★★★ | ★★★★★ | Easy | High (10-14mo) | Need backup |
| [Region-Based Memory](hybrid/region-based-memory/) | ★★★★☆ | ★★★★★ | ★★★★☆ | Medium | High (10-14mo) | Auto (region) |
| [Perceus + Second-Class Refs](combined/perceus-with-second-class-refs/) | ★★★★★ | ★★★★★ | ★★★★★ | Easy | High (12-16mo) | Prevented |
| [Swift ARC + Regions](combined/swift-arc-with-regions/) | ★★★★☆ | ★★★★★ | ★★★★☆ | Easy-Medium | High (12-16mo) | Manual (weak) |
| [Ownership + Cloning + Perceus](combined/ownership-cloning-perceus/) | ★★★★☆ | ★★★★★ | ★★★★★ | Easy | High (12-16mo) | Auto-clone breaks |

---

## Goal-by-Goal Analysis

### "Faster than Go, slower than Rust"

**Best fits:** Simplified Borrow Checker, Perceus, Region-Based Memory, Perceus + Second-Class Refs, Ownership + Cloning + Perceus

- A tracing GC almost certainly fails this goal — Go itself barely meets it
- Refcounting (Arc, Swift ARC) can meet it with good optimization
- Perceus and Regions can approach Rust's speed for functional/request-response patterns
- Perceus + Second-Class Refs combines zero-allocation transforms with zero-copy reads — best combined performance
- Ownership + Cloning + Perceus eliminates hidden auto-clone costs via reuse analysis
- Swift ARC + Regions gives ARC-level general performance with near-zero overhead for request-scoped work

### "Safer than Go, marginally less safe than Rust"

**Best fits:** Second-Class References, Simplified Borrow Checker, Ownership + Implicit Cloning, Perceus + Second-Class Refs

- A full borrow checker matches Rust's safety but is the hardest to implement
- Second-class references provide strong safety with extreme simplicity
- Auto-cloning trades some safety (no compile-time data-race prevention) for usability
- Perceus + Second-Class Refs inherits second-class ref safety (ref violations are compile-time errors) plus Perceus's deterministic drops
- Ownership + Cloning + Perceus is always correct (auto-clone prevents all use-after-move), with optional `move` for strict paths

### "Easy syntax (simpler than Rust)"

**Best fits:** Perceus, Second-Class References, Enhanced Arc + Cycle Collector, Perceus + Second-Class Refs, Ownership + Cloning + Perceus

- Perceus is invisible — no syntax changes, the compiler optimizes silently
- Second-class references add only `ref` and `mutable ref` in parameters — nothing else
- Enhanced Arc requires zero changes from the current model
- Perceus + Second-Class Refs adds only `ref`/`mutable ref` — the Perceus half is fully invisible
- Ownership + Cloning + Perceus requires zero mandatory keywords — optional `move` for advanced users

### "Immutable-by-default / pure synergy"

**Best fits:** Perceus, Region-Based Memory, Immutability-Optimized GC, Perceus + Second-Class Refs, Ownership + Cloning + Perceus

- Perceus thrives on immutable functional transforms — its reuse analysis is designed for them
- Regions work naturally with pure functions (allocate in function region, free on return)
- The GC can optimize heavily when it knows most objects are immutable
- Perceus + Second-Class Refs combines Perceus's functional reuse with zero-copy `ref` reads of immutable data — ideal pairing
- Ownership + Cloning + Perceus: pure functions with single-use params get full Perceus reuse; auto-clone ensures correctness
- Swift ARC + Regions: immutable ARC data never triggers COW; pure functions inside regions benefit from bulk allocation

### "Fail-fast mentality"

**Best fits:** Simplified Borrow Checker, Swift ARC (unowned), Second-Class References, Perceus + Second-Class Refs

- Borrow checker catches errors at compile time — the ultimate fail-fast
- Swift's `unowned` traps immediately on invalid access
- Second-class refs prevent invalid references by construction
- Perceus + Second-Class Refs: ref violations are compile-time errors, Perceus has no runtime failure modes
- Ownership + Cloning + Perceus: `move` annotated params fail at compile time; auto-clone is always safe at runtime
- Swift ARC + Regions: `weak` refs trap on invalid access; region escapes are compile-time errors

---

## Recommendation Tiers

### Tier 1: Best Fit for Opalescent's Goals

**Perceus + Second-Class References** — The strongest combined proposal. Perceus's invisible reuse analysis for functional transforms + second-class refs for zero-copy reads. Near-zero syntax burden (`ref` is the only keyword). Covers virtually all performance patterns. Main risk is implementation complexity of two compiler analyses.

**Perceus Functional RC** — Maximum performance + zero syntax burden + perfect fit for immutable-by-default. The compiler does all the work. The main risk is implementation complexity and the novelty of the approach.

**Second-Class References** — Maximum simplicity + strong safety + fast implementation. The "good enough" borrow checker that covers 90% of cases. The main trade-off is forced copying when returning subsets of borrowed data.

### Tier 2: Strong Contenders

**Ownership + Implicit Cloning + Perceus** — Auto-clone makes every program valid (zero learning curve), Perceus eliminates unnecessary clones (zero performance penalty for common cases). Optional `move` for strict hot paths. Main risk: three interacting compiler analyses + hidden clone costs when Perceus can't optimize.

**Ownership + Implicit Cloning** — Best balance of usability and safety. Feels like a GC language but has ownership semantics. The main risk is hidden performance costs from auto-cloning.

**Swift-Style ARC + COW** — Battle-tested, enterprise-proven, good performance. The main risk is manual cycle prevention and ARC optimizer complexity.

**Swift ARC + Region-Based for Requests** — Maps directly to server architecture (ARC for long-lived services, regions for per-request data). Excellent throughput for request-heavy workloads. Main risk: two mental models (ARC vs region) and region escape analysis complexity.

### Tier 3: Worth Considering

**Escape Analysis + Optional Borrows** — "Easy by default, optimizable when needed." Good philosophy but two mental models can be confusing.

**Region-Based Memory** — Exceptional for server workloads. Less intuitive for general-purpose programming.

### Tier 4: Likely Mismatches

**Simplified Borrow Checker** — Maximum safety and performance, but the syntax complexity conflicts with "easier than Rust."

**Immutability-Optimized GC** — Maximum ease of use, but GC pauses and memory overhead likely conflict with "faster than Go."

**Enhanced Arc + Cycle Collector** — Simplest to implement, but performance ceiling may not meet the "faster than Go" goal.

---

## Combined Approaches

Three combined proposals have been developed into full proposals with example code:

1. **[Perceus + Second-Class References](combined/perceus-with-second-class-refs/)**: Perceus for automatic reuse in functional code + second-class refs for zero-copy reads in function parameters. Tier 1 recommendation.

2. **[Swift ARC + Region-Based for Requests](combined/swift-arc-with-regions/)**: ARC for general memory management + explicit regions for request-response server patterns. Tier 2 recommendation.

3. **[Ownership + Implicit Cloning + Perceus](combined/ownership-cloning-perceus/)**: Auto-clone for correctness + Perceus reuse analysis to eliminate unnecessary clones. Tier 2 recommendation.

---

## Directory Structure

```
memory-model-proposals/
├── COMPARISON.md                          (this file)
├── borrow-checker/
│   ├── simplified-borrow-checker/
│   │   ├── proposal.md
│   │   ├── ownership_basics.op
│   │   └── data_structures.op
│   ├── ownership-with-implicit-cloning/
│   │   ├── proposal.md
│   │   ├── auto_clone_basics.op
│   │   └── collections.op
│   └── second-class-references/
│       ├── proposal.md
│       ├── basic_borrowing.op
│       └── enterprise_service.op
├── reference-counting/
│   ├── enhanced-arc-with-cycle-collector/
│   │   ├── proposal.md
│   │   ├── transparent_usage.op
│   │   └── cyclic_structures.op
│   └── swift-style-arc-with-cow/
│       ├── proposal.md
│       ├── value_semantics.op
│       └── reference_strength.op
├── garbage-collection/
│   └── immutability-optimized-concurrent-gc/
│       ├── proposal.md
│       ├── basic_usage.op
│       └── server_workload.op
├── hybrid/
│   ├── escape-analysis-with-optional-borrows/
│   │   ├── proposal.md
│   │   ├── automatic_mode.op
│   │   └── optimized_hot_path.op
│   ├── perceus-functional-reuse-analysis/
│   │   ├── proposal.md
│   │   ├── functional_transforms.op
│   │   └── real_world_pipeline.op
│   └── region-based-memory/
│       ├── proposal.md
│       ├── automatic_regions.op
│       └── explicit_regions.op
└── combined/
    ├── perceus-with-second-class-refs/
    │   ├── proposal.md
    │   ├── functional_with_refs.op
    │   └── enterprise_data_layer.op
    ├── swift-arc-with-regions/
    │   ├── proposal.md
    │   ├── server_with_regions.op
    │   └── cache_and_regions.op
    └── ownership-cloning-perceus/
        ├── proposal.md
        ├── auto_clone_with_reuse.op
        └── enterprise_pipeline.op
```
