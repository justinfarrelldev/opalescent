# Opalescent Language Proposals

This folder is for language/compiler proposals that are not standard-library API proposals.

Current concerns:

- [`user-defined-adts-cross-module`](./user-defined-adts-cross-module/) — options for making project-defined product, sum, and enum ADTs reliable in generated code across module boundaries, motivated by the simple Neovim-like editor post-mortem.
- [`named-error-sets`](./named-error-sets/proposal.md) — compile-time aliases for exact leaf error sets, with Miette diagnostics and lints for collapsible or overly broad `errors` clauses.
- [`record-update-syntax`](./record-update-syntax/) — options for reducing product-record reconstruction boilerplate such as editor-state `with_status` helpers.
- [`string-ownership-ergonomics`](./string-ownership-ergonomics/) — options for removing application-level defensive string-copy workarounds such as `copy_text`.
- [`numeric-conversion-ergonomics`](./numeric-conversion-ergonomics/) — options for reducing noisy numeric casts while preserving explicit, safe narrowing.
