## 2026-05-17T21:56:00Z Task: propertyless-constructors-registry
- Shared propertyless constructor support now lives in `src/type_system/propertyless_constructors.rs` with a single canonical entry: `Bytes -> bytes_new`.
- The same registry can be reused by parser, type checker, and codegen without duplicating type-name checks.
- Keeping the registry in `type_system` makes it easy to expose through `src/type_system.rs` without adding a new top-level crate module.

## 2026-05-17T22:03:40Z Task:1 verification
- Registry module `src/type_system/propertyless_constructors.rs` and its tests are valid and passing.
- Critical sequencing rule: do not migrate parser/typechecker/codegen to registry in Task 1; keep those refactors for Tasks 2-5 to preserve plan boundaries and verification clarity.

## 2026-05-17T21:56:00Z Corrective revert
- Reverted out-of-scope behavior changes in parser, type checker, and codegen; the shared registry artifact remains in place for Task 1 scope only.

## 2026-05-17T22:00:00Z Corrective revert
- Restored pre-task behavior in `src/parser/new_expression.rs`, `src/type_system/checker/constructors.rs`, and `src/codegen/adts.rs`; only registry/evidence artifacts remain in scope.

## 2026-05-17T23:00:00Z Task:2 parser generalization
- `parse_new_expression` must consume `new` before parsing the callee; otherwise the parser can recurse back into itself and overflow the stack.
- Bare constructor syntax can be handled syntactically by parsing only identifier/member callees and rejecting call postfixes explicitly.
- Field-block parsing for `new Type:` needs to skip trivia, then consume the indentation sentinel before reading fields, matching the rest of the parser's block handling.

## 2026-05-17T23:45:00Z Task:3 type-checker registry integration
- `src/type_system/checker/constructors.rs` now resolves propertyless constructor eligibility via `lookup_propertyless_constructor(...)`, keeping registry data in one canonical module.
- The type checker still returns nominal `CoreType::Generic { name: "Bytes", type_args: [] }` for `new Bytes`, but no longer hard-codes the decision in constructor checking logic.
- Negative semantic coverage now asserts explicit diagnostics for unregistered constructors (`StringBuilder`, `MyEmptyType`) instead of parser-layer rejection behavior.

## 2026-05-17T00:00:00Z Task:4 codegen registry lowering
- Added explicit + inferred codegen tests for `new Bytes` lowering (`codegen_explicit_new_bytes_lowers_to_bytes_new`, `codegen_inferred_new_bytes_lowers_to_bytes_new`) to enforce runtime `bytes_new` declaration/call emission.
- Constructor lowering in `src/codegen/adts.rs` now uses `lookup_propertyless_constructor(...)` for empty-field identifier constructors, removing hard-coded `name == "Bytes" && fields.is_empty()` from that branch.
- Product and sum constructor lowering branches were preserved unchanged.

## 2026-05-17T22:19:36Z Task:5 constructor inference fix
- `infer_core_type_from_expr` in `src/codegen/statements/inference.rs` now has a dedicated `Expr::Constructor` branch for empty-field constructors.
- The branch consults `lookup_propertyless_constructor(...)` and infers `CoreType::Generic { name: "Bytes", type_args: [] }` for `new Bytes`, aligning inference with parser/type-checker/codegen registry decisions.
- This prevents inferred `let buffer = new Bytes` from falling back to `Int64`, unblocking `buffer.length` member lowering to `bytes_length` (`i32`).

## 2026-05-17T23:59:00Z Task:6 inferred-bytes e2e fixture
- Added `test-projects/bytes-empty-construct-inferred-new-syntax` following the same fixture contract as legacy and explicit-new fixtures (`opal.toml`, `README.md`, `src/main.op`, `expected/stdout.txt`).
- The fixture proves constructor inference end-to-end with `let buffer = new Bytes` (no type annotation) and inferred member access via `buffer.length`.
- Integration harness parity was preserved by reusing the existing compile/run/assert pattern in `tests/integration_e2e/bytes_stdlib.rs`.
- 2026-05-17T00:00:00Z: Task 7 completed wording/documentation updates.
  - Updated `src/type_system/propertyless_constructors.rs` to include future-extension notes for opaque handles while explicitly stating only `Bytes` is registered.
  - Neutralized `Bytes`-specific parser error expectations in `src/parser/tests.rs` to align with the registry-driven design where propertyless syntax is accepted generically by the parser and restricted by the type-checker.
  - Verified no claims of support for `new StringBuilder` exist in current documentation or code (only test-suite negative assertions and future-work proposals).
  - Cleaned up stale wording in `src/parser/tests.rs` that implied `new Bytes` was a parser special case.


## 2026-05-17T22:31:35.869750+00:00 Task 8 learnings
- Final non-test Bytes audit found only intentional runtime, registration, and member-lowering hits; no propertyless-constructor logic still checks Bytes directly.
- `cargo test --all` passed after the targeted gates, including the inferred `new Bytes` integration fixture.

## 2026-05-17T18:38:49-04:00 Task: final-wave remediation F2/F4
- For `new` parser callee scope, replacing the unbounded `while` dot-chain with a single optional dot segment enforces plan scope cleanly (`Type` or `Module.Type` only).
- Explicitly erroring on a second dot in `parse_new_expression` preserves accepted forms while preventing accidental grammar broadening (`new A.B.C`).
- Existing type-checker boundary tests already correctly enforce registry-only propertyless support (Bytes accepted; StringBuilder and unknown user type rejected), so no checker change was needed.

## 2026-05-18T00:00:00Z Final-wave closeout
- Final Verification Wave reached full approval after remediation: F1 APPROVE, F2 APPROVE, F3 APPROVE, F4 APPROVE.
- Plan top-level implementation tasks (1-8) and final-wave tasks (F1-F4) are now all checked.
- Remediation remained scope-faithful: parser limited to `Type` or `Module.Type`, with explicit rejection test for deep chains.
