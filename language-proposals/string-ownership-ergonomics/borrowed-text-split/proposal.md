# Borrowed Text and Owned String Split

## Overview

This alternative makes ownership visible in the type system by distinguishing owned strings from borrowed text views. APIs that only inspect text can accept borrowed text; APIs that store or return text require owned strings.

Possible surface types include:

- `string` — owned, storable, returnable text.
- `TextView` or `text` — borrowed/non-owning view of text.
- `ref string` — second-class borrow for parameters only.

This is the most explicit long-term model, but it is a large language and stdlib migration. It should not be the first fix for the editor's `copy_text` workaround.

## Assumes

- Second-class references remain limited to parameters unless the language explicitly expands them.
- Borrowed text cannot be stored in products, arrays, or returned.
- Standard-library functions can be audited and migrated to accept borrowed text where appropriate.
- There is a clear conversion from borrowed text to owned `string` when storage is needed.

## Syntax Design

### Borrowing for read-only APIs

```opal
public let string_length = f(text: ref string): int64 => ...
```

or, if a first-class view type is chosen:

```opal
public let string_length = f(text: TextView): int64 => ...
```

### Owning when storing

```opal
import string_from_text from standard
import type EditorStatus from ./editor.types

##
  Description: Build an invalid-key status from borrowed text by explicitly owning it.
##
public let invalid_key_status = f(text: ref string): EditorStatus errors AllocationFailureError =>
    let owned_key = propagate string_from_text(text)
    return new EditorStatus.InvalidKey:
        key: owned_key
```

### Literal behavior

String literals may be usable as borrowed text or owned strings depending on expected type:

```opal
print_text(ref 'hello')
let label: string = 'hello'
```

The exact literal rule would need a separate design pass.

## Example Applications

```opal
import string_from_text from standard
import type EditorStatus from ./editor.types

##
  Description: Build an invalid-key status from borrowed input text.
##
public let invalid_key_status = f(text: ref string): EditorStatus errors AllocationFailureError =>
    let key = propagate string_from_text(text)
    return new EditorStatus.InvalidKey:
        key: key
```

```opal
import terminal_text_cell_width from standard

##
  Description: Measure text without taking ownership of the string.
##
public let label_width = f(label: ref string): int64 =>
    return terminal_text_cell_width(ref label)
```

## Strengths

- Very explicit ownership model.
- Read-only APIs can avoid RC churn.
- Storing/returning text clearly requires ownership.
- Aligns with second-class reference goals.
- Can improve performance for large strings passed to many readers.

## Weaknesses

- Large surface-area change for the standard library.
- More ceremony in everyday string code.
- First-class text views would add lifetime/escape questions.
- `ref` parameters everywhere may worsen readability.
- Does not automatically make simple label helpers nicer unless literals and owned returns are also fixed.

## Impact on Existing Syntax

Potentially high. Many functions that currently accept `string` by value may migrate to `ref string` or `TextView`. Call sites would need borrow syntax if the language does not infer it.

If `TextView` is first-class, the language must define where it can be stored, whether it can outlive the source, and how it interacts with arrays and products. That is a major design expansion.

## Interactions with Other Concerns

- **Second-class references:** this proposal depends on clear borrow escape rules.
- **String ownership semantics:** owned `string` still needs safe return/storage behavior.
- **Standard library:** almost every string-consuming function needs review.
- **Record update:** borrowed views must not be stored in product fields unless the type explicitly allows it.
- **LSP/docs:** hovers must explain ownership expectations clearly.

## Implementation Difficulty

High.

Main tasks:

1. Choose `ref string` only or add a first-class `TextView` type.
2. Define literal typing and conversion rules.
3. Audit stdlib signatures.
4. Implement borrow checking for text views if they are first-class.
5. Add conversion helpers such as `string_from_text`.
6. Update docs and examples across the repository.
7. Provide migration diagnostics.

## Must NOT Have

- Must not allow borrowed text to escape into long-lived product fields accidentally.
- Must not require ownership annotations for every trivial string use without clear benefit.
- Must not replace the simple `string` type with a confusing family of near-identical text types.
- Must not hide allocation in conversions that claim to be borrowing-only.
