# User-defined ADTs Across Project Modules

This language proposal package compares ways to make project-defined algebraic data types reliable in generated code across module boundaries.

It was motivated by `SIMPLE_NEOVIM_POST_MORTEM.md`: a modal editor wants small nominal state types (`EditorMode`, `EditorStatus`, `EditorCommand`) and one product state record (`EditorState`) instead of integer constants and wide labeled returns.

Read [`COMPARISON.md`](./COMPARISON.md) first.

The `.op` files under alternatives are proposal examples. They show the desired source shape and may exercise compiler behavior that this proposal is asking to implement.

## Alternatives

- [`status-quo-integer-constants`](./status-quo-integer-constants/proposal.md) — no compiler change; keep integer constants.
- [`enum-first-nominal-states`](./enum-first-nominal-states/proposal.md) — make propertyless enums work first.
- [`whole-project-layout-merge`](./whole-project-layout-merge/proposal.md) — collect one project-wide ADT layout map before codegen.
- [`module-interface-layout-manifests`](./module-interface-layout-manifests/proposal.md) — recommended v1; export module-qualified ADT layout manifests through module interfaces.
- [`generated-accessor-abi`](./generated-accessor-abi/proposal.md) — lower field/variant syntax to generated constructors and accessors.
- [`opaque-state-module-boundary`](./opaque-state-module-boundary/proposal.md) — avoid cross-module ADT values by architecture.

## Recommended direction

Choose `module-interface-layout-manifests` as the real language/compiler direction. Use `whole-project-layout-merge` only as a timeboxed bridge if it is implemented with module-qualified IDs and migrates toward manifests.
