# Record Update Syntax — Comparison

## Status and scope

Draft language/compiler proposal package. This concern responds to the simple editor's repeated `EditorState` reconstruction helpers such as `with_status`, `with_command`, and `with_cursor_dirty_status`.

The goal is to let Opalescent users create a new product value from an existing product value while replacing a small set of fields, without listing every unchanged field by hand. This is a source-level ergonomics concern. It must preserve Opalescent's value semantics, explicit ownership model, and clear product construction rules.

## Reading order

1. This comparison.
2. [`block-with-expression`](./block-with-expression/proposal.md) — recommended v1: indentation-sensitive `state with:` blocks.
3. [`inline-with-expression`](./inline-with-expression/proposal.md) — compact single-expression updates for small changes.
4. [`generated-update-helpers`](./generated-update-helpers/proposal.md) — compiler-generated `with_<field>` helpers without new syntax.
5. [`mutable-copy-update`](./mutable-copy-update/proposal.md) — local mutable copy plus field assignment.

## Problem shape

The current editor state helpers are correct but noisy:

```opal
public let with_status = f(state: EditorState, status: EditorStatus): EditorState =>
    return editor_state(
        state.cursor,
        state.viewport_top,
        state.dirty,
        state.saved,
        state.mode,
        state.command,
        status,
        state.running,
        state.termination
    )
```

The interesting operation is only `status: status`; everything else is mechanical copying. This is error-prone when records become large, especially when several boolean fields sit near each other.

## Ground-up syntax target

The most direct user experience is:

```opal
let next = state with:
    status: new EditorStatus.UnsavedChanges
```

For multiple fields:

```opal
return state with:
    cursor: cursor
    dirty: true
    status: new EditorStatus.InsertedText
```

The source says exactly what changed while retaining the explicit product-field names Opalescent already uses in constructors.

## Comparison matrix

| Axis | Block `with:` expression — recommended | Inline `with` expression | Generated update helpers | Mutable copy update |
|---|---:|---:|---:|---:|
| **Editor-state fit** | ★★★★★ | ★★★★☆ | ★★★☆☆ | ★★★☆☆ |
| **Readability for multi-field updates** | ★★★★★ | ★★☆☆☆ | ★★★☆☆ | ★★★☆☆ |
| **Syntax footprint** | ★★★★☆ | ★★★☆☆ | ★★★★★ | ★★★☆☆ |
| **Parser/formatter effort** | Medium | Medium | Low to medium | Medium |
| **Type-checker effort** | Medium | Medium | Medium | Medium to high |
| **Ownership clarity** | ★★★★☆ | ★★★★☆ | ★★★★★ | ★★★☆☆ |
| **Hot-reload/package ABI fit** | ★★★★☆ | ★★★★☆ | ★★★★★ | ★★★☆☆ |
| **Risk of accidental mutation model confusion** | Low | Low | Low | Medium |

## Alternatives summary

### Block `with:` expression — recommended

Add an indentation-sensitive expression form:

```opal
let next = state with:
    status: status
    dirty: true
```

The expression evaluates `state`, copies every field, and replaces the listed fields. It is checked against the product type of `state`.

**Why it fits:** it follows Opalescent's colon-and-indent style, avoids braces, scales to multiple fields, and keeps field names obvious.

**Main cost:** it requires new parser, type checker, formatter, codegen, and LSP support.

### Inline `with` expression

Add compact syntax for small updates:

```opal
return state with status: status
```

Potentially allow comma-separated updates:

```opal
return state with cursor: cursor, dirty: true, status: status
```

**Why it fits:** it is terse and convenient for one-field changes.

**Main cost:** it becomes harder to format once updates contain constructors, fallible expressions, or long names. It also introduces more parser ambiguity around existing colon syntax.

### Generated update helpers

Generate or expose helpers such as:

```opal
return editor_state_with_status(state, status)
```

This avoids new source syntax and gives package/hot-reload tooling a clear helper ABI.

**Why it fits:** it is implementable as metadata/codegen work and can be a lowering target for future syntax.

**Main cost:** helpers multiply rapidly for large products and still force nested or multi-field changes into chains.

### Mutable copy update

Allow product field assignment on a local mutable copy:

```opal
let mutable next = state
next.status = status
return next
```

**Why it fits:** many programmers understand this pattern immediately.

**Main cost:** it blurs the line between value update and object mutation. The compiler must make it clear that `state` is not mutated unless the product itself is a mutable reference target.

## Recommendation

Select **block `with:` expression** as the source-level v1 design.

Keep **generated update helpers** as an internal lowering target and as a possible package ABI strategy. Avoid exposing helper names as the main user-facing answer unless syntax work is delayed.

Do not select **mutable copy update** as the primary ergonomic story. It is useful as a possible later feature, but it risks teaching users that product values are object references.

Use **inline `with`** only as optional sugar after block `with:` is stable. It should not be the first syntax because Opalescent's current style favors indentation for nontrivial expressions.

## Required fixture ladder if selected later

1. One-field update on a local product.
2. Multi-field update preserving unchanged fields.
3. Imported product type from `.types.op` across modules.
4. Product containing RC-managed fields such as `string` and `string[]`.
5. Nested product replacement by composing two `with:` expressions.
6. Error when updating an unknown field.
7. Error when update value has the wrong type.
8. Error when receiver is not a product.
9. Formatter round-trip for single-line, multiline, nested, and constructor-valued updates.
10. Codegen retention/drop tests proving unchanged RC fields remain valid.

## Must not regress

- Product construction with `new TypeName:` must continue to work.
- Type declarations must remain in `.types.op` files.
- Record update must not mutate the original value.
- Field order in source update blocks must not change product layout or ABI.
- Unknown or duplicate updated fields must be user-facing diagnostics, not codegen panics.
- The feature must not require brackets around update bodies.
