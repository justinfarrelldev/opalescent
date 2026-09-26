# String Ownership Ergonomics — Comparison

## Status and scope

Draft language/runtime/stdlib proposal package. This concern responds to the editor workaround:

```opal
public let copy_text = f(text: string): string errors
    StringRangeOutOfBoundsError,
    AllocationFailureError
=>
    return propagate string_insert_at('', 0 as int64, text)
```

Application code should not need to call a string-editing function to obtain a safe owned string for labels or ADT payloads. If a helper returns `'ok'`, stores `text` in `EditorStatus.InvalidKey`, or copies `invalid.text` into another status, that should be safe under normal string ownership rules.

## Reading order

1. This comparison.
2. [`compiler-owned-string-semantics`](./compiler-owned-string-semantics/proposal.md) — recommended v1: fix ownership transfer/retain semantics so defensive copies are unnecessary.
3. [`standard-string-copy`](./standard-string-copy/proposal.md) — minimal stdlib escape hatch: `string_copy` / `string_clone`.
4. [`static-cow-string-runtime`](./static-cow-string-runtime/proposal.md) — runtime representation option for literal/static/COW strings.
5. [`borrowed-text-split`](./borrowed-text-split/proposal.md) — larger type-system split between owned strings and borrowed text views.

## Problem shape

The editor currently uses `copy_text` in label helpers:

```opal
if status is EditorStatus.Ok:
    return propagate copy_text('ok')
```

That has three costs:

1. It turns simple labels into fallible code.
2. It makes function signatures include allocation/string range errors that are not conceptually part of label selection.
3. It exposes memory ownership details to application authors.

## Desired end state

These should be safe without defensive cloning:

```opal
return 'ok'
```

```opal
let status: EditorStatus = new EditorStatus.InvalidKey:
    key: text
```

```opal
if command is EditorCommand.Invalid into invalid:
    return invalid.text
```

If a true deep copy is needed, there can still be an explicit `string_copy`, but normal ownership transfer and retention should not require it.

## Comparison matrix

| Axis | Compiler-owned string semantics — recommended | Standard `string_copy` | Static/COW string runtime | Borrowed text split |
|---|---:|---:|---:|---:|
| **Eliminates editor workaround** | ★★★★★ | ★★★☆☆ | ★★★★☆ | ★★★★★ |
| **Application ergonomics** | ★★★★★ | ★★☆☆☆ | ★★★★☆ | ★★★☆☆ |
| **Semantic clarity** | ★★★★☆ | ★★★☆☆ | ★★★★☆ | ★★★★★ |
| **Implementation effort** | Medium to high | Low | Medium | High |
| **Runtime risk** | Medium | Low | Medium | High |
| **Compatibility** | ★★★★★ | ★★★★★ | ★★★★☆ | ★★☆☆☆ |
| **Long-term type-system fit** | ★★★★☆ | ★★☆☆☆ | ★★★★☆ | ★★★★★ |

## Alternatives summary

### Compiler-owned string semantics — recommended

Define and implement clear ownership rules for `string` values:

- A `string` value in source is an owned value unless explicitly passed as `ref string`.
- Storing a string in a product/variant/array retains or moves it safely.
- Returning a string retains or moves it safely.
- Accessing a string field as a value produces a valid owned string reference according to RC rules.
- String literals are safe to return and store without heap-copying on every use.

This is the best user-facing result because most source code does not change.

### Standard `string_copy`

Add a standard helper:

```opal
string_copy(text: string): string errors AllocationFailureError
```

This replaces the `string_insert_at('', 0, text)` hack with an intentional API.

It is useful as a short-term mitigation and for rare true deep-copy needs, but it should not be the final answer for ordinary labels and ADT payloads.

### Static/COW string runtime

Represent strings with runtime metadata that distinguishes static literals, heap-owned strings, and possibly shared copy-on-write buffers. Cloning a string can be a retain or static no-op rather than a deep allocation.

This may be an implementation strategy for the recommended semantics, or a standalone runtime proposal.

### Borrowed text split

Introduce a larger type distinction between owned `string` and borrowed text views such as `TextView` or second-class `ref string` parameters. APIs that only read text accept borrowed text; APIs that store text require owned strings.

This is precise and expressive, but it is a much larger language and standard-library migration.

## Recommendation

Select **compiler-owned string semantics** as the primary v1 direction. The language should make ordinary `string` ownership safe and unsurprising before asking application authors to spell ownership conversions.

Add **standard `string_copy`** only as a pragmatic helper for explicit deep-copy needs or as a bridge while the runtime/compiler fix is underway. It should not be required for returning string literals or storing string parameters in ADTs.

Treat **static/COW runtime representation** as an implementation strategy that can support the recommended semantics.

Defer **borrowed text split** until the Perceus + second-class reference model is mature enough to expose a broader borrowed-text API without destabilizing ordinary code.

## Required fixture ladder if selected later

1. Return a string literal from a helper without allocation errors.
2. Return a string parameter from a helper and use the caller's original after the call.
3. Store a string parameter in a payload variant and render it later.
4. Store a string literal in a payload variant and render it later.
5. Store and read strings in products imported across modules.
6. Store and read strings in arrays without invalid frees.
7. Return a string field from a payload binding.
8. Repeat the above under generated project builds, not only type checking.
9. Run under sanitizers or equivalent memory diagnostics.
10. Remove `copy_text` from the simple editor fixture.

## Must not regress

- Existing string range/editing APIs remain explicitly fallible when they allocate or validate ranges.
- String `.at(...)` still returns a one-scalar `string` and can fail with `IndexOutOfBoundsError`.
- String literals must not be freed as if they were heap allocations.
- Field access must not return dangling references.
- A standard copy helper, if added, must not become required for ordinary ownership transfer.
