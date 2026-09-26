# Standard `string_copy` Helper

## Overview

This alternative adds a first-class standard-library helper for making an explicit owned string copy:

```opal
string_copy(text: string): string errors AllocationFailureError
```

It replaces the current user-defined workaround based on `string_insert_at('', 0, text)` with a readable, intentional API. This is the smallest proposal and can be useful even if deeper ownership semantics are fixed later.

It is not recommended as the final answer for ordinary labels and ADT payloads because it still forces users to expose allocation fallibility in code that should only select a label.

## Assumes

- Strings are immutable values and copying produces an independent string value or an owned retained value according to runtime representation.
- Allocation failure remains explicit when a real deep copy is needed.
- The helper belongs in `standard` near other string functions.

## Syntax Design

No language syntax changes.

### Function

```opal
import string_copy from standard

string_copy(text: string): string errors AllocationFailureError
```

The name should be chosen deliberately:

- `string_copy` emphasizes a new value.
- `string_clone` aligns with RC/clone terminology but may imply shallow sharing to some users.
- `string_to_owned` is explicit but longer and introduces ownership jargon.

Recommendation for this alternative: use `string_copy` in the stdlib docs, with `string_clone` only if the language adopts broader clone naming.

## Example Applications

```opal
import string_copy from standard
import type EditorStatus from ./editor.types

##
  Description: Return a copied user-visible status label using the standard copy helper.
##
public let editor_status_text = f(status: EditorStatus): string errors AllocationFailureError =>
    if status is EditorStatus.Ok:
        return propagate string_copy('ok')
    if status is EditorStatus.InvalidKey into invalid:
        return propagate string_copy(invalid.key)
    return propagate string_copy('unknown status')
```

```opal
import string_copy from standard
import type EditorStatus from ./editor.types

##
  Description: Build a status payload with an explicit copied key string.
##
public let invalid_key_status = f(text: string): EditorStatus errors AllocationFailureError =>
    let key = propagate string_copy(text)
    return new EditorStatus.InvalidKey:
        key: key
```

## Strengths

- Very small surface area.
- Replaces a surprising hack with an obvious API.
- Easy to document and test.
- Useful for callers that truly require an independent deep copy.
- Does not require parser/type-system changes.

## Weaknesses

- Does not solve the underlying ownership leak if ordinary storage still requires copying.
- Adds `AllocationFailureError` to simple label helpers.
- Users may cargo-cult `string_copy` everywhere.
- Deep copying immutable strings may be unnecessary overhead if RC retain would suffice.
- It can make Opalescent look more ownership-heavy than intended.

## Impact on Existing Syntax

No syntax changes. Existing `copy_text` helpers can migrate to `string_copy`.

The standard library and type checker need to register the new function and `AllocationFailureError` contract.

## Interactions with Other Concerns

- **Compiler-owned string semantics:** once ownership is fixed, `string_copy` becomes a rare explicit deep-copy tool rather than a workaround.
- **Error annotation noise:** this alternative worsens signatures when used in label helpers.
- **String editing primitives:** implementation may share runtime internals with `string_insert_at` but should not validate a range or expose range errors.
- **Performance:** copy may be optimized into retain/COW if the runtime representation permits, but the semantic contract should be clear.

## Implementation Difficulty

Low.

Main tasks:

1. Add stdlib signature for `string_copy`.
2. Add runtime implementation.
3. Add type-system registration with `AllocationFailureError`.
4. Add docs and tests.
5. Migrate local helpers only if the deeper ownership fix is not selected first.

## Must NOT Have

- Must not use `string_insert_at` semantics that can fail with range errors.
- Must not be required before every string return or payload construction.
- Must not hide allocation failure if a deep copy allocates.
- Must not introduce mutable string semantics.
