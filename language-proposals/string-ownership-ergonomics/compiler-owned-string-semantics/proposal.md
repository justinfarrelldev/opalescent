# Compiler-Owned String Semantics

## Overview

This alternative fixes the language/runtime ownership rules for ordinary `string` values so application code does not need defensive cloning for normal returns, product fields, variant payloads, or arrays.

Under this proposal, a `string` in value position is a valid owned string value. The compiler is responsible for retaining, moving, or releasing RC-managed string storage at ownership boundaries. String literals are safe to return or store. Field access produces a valid value according to ordinary RC rules.

The goal is that editor label code can say:

```opal
return 'ok'
```

and status payload code can say:

```opal
return new EditorStatus.InvalidKey:
    key: text
```

without `copy_text` and without additional errors in signatures.

## Assumes

- `string` remains an RC-managed or RC-compatible heap/runtime type.
- The compiler already tracks owned values and can insert retain/release operations.
- String literals can be represented as immortal static strings, RC objects with static lifetime, or lowered into owned runtime strings safely.
- Product and sum constructors define ownership transfer for each field/payload.

## Syntax Design

No new source syntax is required.

This proposal defines semantics for existing syntax:

```opal
return 'ok'
```

```opal
let status: EditorStatus = new EditorStatus.InvalidKey:
    key: text
```

```opal
if status is EditorStatus.InvalidKey into invalid:
    return invalid.key
```

### Ownership rules

1. Function parameters of type `string` are owned values unless declared as `ref string`.
2. Returning a `string` transfers an owned value to the caller. If the returned value is still used locally on another path, the compiler retains as needed.
3. Constructing a product or payload with a `string` field stores an owned string reference. The constructor retains or moves the source according to last-use analysis.
4. Reading a `string` field as a value produces a valid owned string reference. The compiler retains if the containing object remains alive.
5. String literals are valid values with safe lifetime and must not be freed as ordinary heap allocations unless they were materialized as heap-owned strings.

## Example Applications

### Label helper without defensive copy

```opal
import type EditorStatus from ./editor.types

##
  Description: Return a user-visible status label without defensive string copying.
##
public let editor_status_text = f(status: EditorStatus): string =>
    if status is EditorStatus.Ok:
        return 'ok'
    if status is EditorStatus.UnsavedChanges:
        return 'unsaved changes'
    if status is EditorStatus.InvalidKey into invalid:
        return 'invalid key {invalid.key}'
    return 'unknown status'
```

### Payload construction from existing text

```opal
import type EditorStatus from ./editor.types

##
  Description: Build an invalid-key status from committed input text.
##
public let invalid_key_status = f(text: string): EditorStatus =>
    return new EditorStatus.InvalidKey:
        key: text
```

## Strengths

- Best user ergonomics: no new syntax and no routine copy helper.
- Removes artificial `AllocationFailureError` from simple label helpers.
- Aligns with ordinary expectations for immutable strings in an RC language.
- Preserves explicit fallibility for real allocating string operations.
- Improves products, sums, arrays, and returns consistently.
- Works well with Perceus reuse and last-use analysis.

## Weaknesses

- Requires careful compiler/runtime ownership auditing.
- Bugs can be memory safety bugs, not only type errors.
- Field access and payload binding need exact retain/release semantics.
- May require runtime support for static literals or a safe literal materialization strategy.
- Harder to validate than adding a simple `string_copy` function.

## Impact on Existing Syntax

No syntax change. Existing code becomes safer and can delete defensive clones.

Some codegen behavior changes:

- Returning string literals must lower safely.
- Product/sum constructors must own string fields reliably.
- Field access for string fields must produce safe values.
- Duplicate retains should be optimized where possible but correctness comes first.

## Interactions with Other Concerns

- **Record update:** copied string fields in record updates must follow the same retain/move rules.
- **Array value semantics:** storing strings in arrays must retain/move consistently.
- **Generated accessor ABI:** accessors returning string fields must document whether they return owned values; source-level field access should behave as owned value access.
- **String copy stdlib:** `string_copy` remains useful for forced deep copies but should not be required for ownership correctness.
- **Hot reload:** ABI metadata should distinguish static-lifetime and RC-managed string representations only if needed at module boundaries.

## Implementation Difficulty

Medium to high.

Main tasks:

1. Specify ownership transfer for string parameters, returns, constructors, field access, and arrays.
2. Audit current codegen retain/release behavior for string values.
3. Define safe string literal runtime representation.
4. Add tests for returning literals, returning parameters, storing payloads, and field access.
5. Add sanitizer-backed generated-program tests if available.
6. Remove defensive `copy_text` from editor only after tests are green.
7. Document the ownership model in the crash course and memory model docs.

## Must NOT Have

- Must not require users to call a copy helper before storing ordinary strings.
- Must not make string field access return dangling references.
- Must not free static string literals as heap allocations.
- Must not hide truly fallible string editing operations.
- Must not introduce a public `char` or `rune` type as part of this fix.
