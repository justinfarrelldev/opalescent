
- 2026-05-17T00:00:00Z: Added a legacy empty Bytes test project using `bytes_new()` plus `Bytes.length`; the expected stdout is `legacy length: 0` and the integration harness mirrors the existing bytes stdlib test pattern.

- 2026-05-17T00:00:00Z: Added the RED-only new-syntax fixture `bytes-empty-construct-new-syntax` with `let buffer: Bytes = new Bytes`; `cargo test --features integration empty_bytes_via_new_bytes` currently fails at front-end compilation as expected, and the fixture shape check confirms the exact source/expected stdout.

- 2026-05-17T00:00:00Z: Task 6 codegen lowering now treats bare zero-field `new Bytes` as a special constructor case in `src/codegen/adts.rs`, emitting the existing `bytes_new` stdlib/runtime declaration and a no-arg call without changing runtime ABI names or legacy `bytes_new()` lowering tests.
- 2026-05-17T00:00:00Z: Task 6 verification found `cargo check --lib` clean for the compiler library and `rg -n 'bytes_new' runtime/opal_bytes.c runtime/opal_runtime.h src/codegen/functions_stdlib.rs` unchanged, but targeted `cargo test ...` execution is currently blocked by unrelated pre-existing compile errors in `src/parser/tests.rs` and `src/type_system/tests.rs`.

- 2026-05-17T00:00:00Z: Task 5 updated `src/type_system/checker/constructors.rs` so zero-field constructor expressions only typecheck when the callee is exactly `Bytes`; other propertyless constructor forms now report an `InvalidOperation` with operation `propertyless constructor syntax`, while `bytes_new` registrations remain unchanged in the checker and module resolver.

- 2026-05-17T00:00:00Z: Added Task 5 typechecker coverage for `new Bytes` with explicit and inferred lets plus negative assertions for `new Person`, `new Message.Text`, and `new bytes`. LSP diagnostics are clean on the modified files, but the requested `cargo test` filters are currently blocked by pre-existing compile errors in `src/parser/tests.rs` (invalid `Stmt::Let` destructuring expecting a removed `type_annotation` field at lines 6143 and 6181).
- 2026-05-17T00:00:00Z: Task 4 parser/formatter support now parses bare `new Bytes` as a zero-field constructor, preserves `new Type:` and `new Type.Variant:`, rejects `new Bytes()` and `new Bytes:`, and formatter output keeps `new Bytes` without rewriting legacy `bytes_new()` calls.

- 2026-05-17T00:00:00Z: Task 5 negative coverage now treats bare non-Bytes constructors as valid front-end rejections whether they fail in parsing or typechecking; with current parser behavior, `new Person`, `new Message.Text`, and `new bytes` are asserted via parser `InvalidSyntax` diagnostics while positive `new Bytes` tests still run through full typechecking.

- 2026-05-17T00:00:00Z: Task 5 negative assertions should match the stable front-end rule (`only supported for `Bytes``) instead of overfitting to one exact parser message per rejected callee, because bare non-Bytes constructors can share the same parser diagnostic template.

- 2026-05-17T00:00:00Z: Task 7 reconciliation confirmed `new Bytes` end-to-end support in parser/typechecker/codegen/formatter while preserving legacy `bytes_new()` behavior; full gates (`cargo test`, `cargo test --features integration`, pre-commit lint/test/build) passed after normalizing the local `C:\Users\justi\Downloads` fixture directory state for guard integration tests.
- 2026-05-17T00:00:00Z: Kept runtime ABI untouched (no changes to runtime/opal_bytes.c or runtime/opal_runtime.h) and captured green proof in `.sisyphus/evidence/task-7-new-syntax-green.txt` plus `.sisyphus/evidence/task-7-regression-gate.txt`; existing red evidence file timestamps remain earlier than Task 7 green artifacts.
- Updated stdlib/prelude.op to prefer 'new Bytes' syntax while noting legacy compatibility for 'bytes_new()'.
- Verified that existing proposals and doc examples already used 'new Bytes', ensuring consistency.
