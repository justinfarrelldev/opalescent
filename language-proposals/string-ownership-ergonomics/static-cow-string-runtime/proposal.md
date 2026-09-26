# Static and Copy-on-Write String Runtime

## Overview

This alternative changes the runtime representation of strings so literals, shared strings, and copied strings are cheap and safe. A string value can point to static literal storage, heap-owned storage, or a shared copy-on-write buffer. Since Opalescent strings are immutable at the language level, many "copies" can be implemented as retains or static no-ops.

This can be a standalone runtime strategy or the implementation underpinning compiler-owned string semantics. The user goal is the same: returning `'ok'` and storing `text` in a payload should not require application-level cloning.

## Assumes

- Strings remain immutable from user code.
- Runtime string metadata can distinguish at least static vs heap-owned storage.
- Existing C runtime helpers can be updated without changing source syntax.
- The code generator can call retain/release operations that handle both static and heap strings safely.

## Syntax Design

No source syntax changes.

The difference is runtime behavior:

```opal
return 'ok'
```

The literal may lower to a static string descriptor with an immortal lifetime.

```opal
let copied = string_copy(text)
```

If `text` is immutable and shared, this may retain/share rather than deep-copy unless the API promises physical independence.

## Example Applications

```opal
import type EditorStatus from ./editor.types

##
  Description: Return status labels that may point to static runtime string storage.
##
public let editor_status_text = f(status: EditorStatus): string =>
    if status is EditorStatus.Ok:
        return 'ok'
    if status is EditorStatus.UnsavedChanges:
        return 'unsaved changes'
    return 'unknown status'
```

```opal
import type EditorStatus from ./editor.types

##
  Description: Store a string value in a status payload using runtime retain semantics.
##
public let invalid_key_status = f(text: string): EditorStatus =>
    return new EditorStatus.InvalidKey:
        key: text
```

## Strengths

- Makes string literals safe and cheap to return/store.
- Avoids unnecessary deep copies for immutable strings.
- Can preserve user-facing `string` type and syntax.
- Improves performance if many labels and payloads share text.
- Provides a concrete runtime mechanism for the recommended ownership semantics.

## Weaknesses

- Runtime representation becomes more complex.
- FFI and C runtime helpers must respect tagged/static string values.
- Bugs in retain/release paths can become memory safety issues.
- If `string_copy` promises physical independence, COW semantics must be carefully documented.
- May require ABI changes for string layout.

## Impact on Existing Syntax

No syntax changes. Existing programs should continue to compile.

Runtime ABI impact depends on current string representation. If the public ABI assumes strings are raw heap pointers, adding tags/descriptors may require a compatibility layer or a versioned ABI bump.

## Interactions with Other Concerns

- **Compiler-owned string semantics:** this is a likely implementation strategy.
- **Hot reload:** string ABI changes must participate in interface signature hashes.
- **FFI:** exported/imported string representation must be stable or marshaled.
- **Standard `string_copy`:** the helper must specify whether it requires physical copying or logical owned sharing.
- **Error handling:** retaining static strings should not introduce allocation errors.

## Implementation Difficulty

Medium to high depending on current ABI constraints.

Main tasks:

1. Define string runtime descriptor/header format.
2. Decide whether static literals use RC headers, tags, or separate descriptors.
3. Update retain/release helpers to handle static strings.
4. Update string_length, indexing, interpolation, and editing helpers.
5. Define FFI and hot-reload ABI consequences.
6. Add stress tests for literal returns, repeated labels, and payload storage.

## Must NOT Have

- Must not free static literal storage.
- Must not make string reads mutable or observable through aliases.
- Must not silently change public text indexing semantics.
- Must not break existing standard-library string functions without a migration path.
- Must not use COW as an excuse to skip ownership tests.
