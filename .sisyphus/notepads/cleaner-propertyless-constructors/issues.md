## 2026-05-17T21:56:00Z Task: propertyless-constructors-registry
- Initial file edit on `src/type_system.rs` missed the exact module block layout; the fix was to patch the `mod constraints;` section directly.
- `src/codegen/adts.rs` needed a cleanup pass after the first replacement left stale `bytes_new` call-site lines behind.

## 2026-05-17T22:03:40Z Task:1 verification
- Rejected first Task 1 implementation: subagent introduced out-of-scope behavioral edits in `src/parser/new_expression.rs`, `src/type_system/checker/constructors.rs`, and `src/codegen/adts.rs`.
- Plan requires Task 1 to establish registry + audit foundation only; parser/typechecker/codegen behavior changes belong to later tasks.

## 2026-05-17T21:56:00Z Corrective revert
- Removed the premature call-site migration from `src/parser/new_expression.rs`, `src/type_system/checker/constructors.rs`, and `src/codegen/adts.rs`.

## 2026-05-17T22:00:00Z Corrective revert
- Restored pre-task behavior in `src/parser/new_expression.rs`, `src/type_system/checker/constructors.rs`, and `src/codegen/adts.rs`; only registry/evidence artifacts remain in scope.

## 2026-05-17T23:00:00Z Task:2 parser generalization
- First generic rewrite accidentally caused `parse_new_expression` recursion because `new` was not consumed before parsing the callee.
- The initial field-block path treated the indent token as a field name until the parser was updated to consume the indentation sentinel explicitly.
- A separate newline/indent ordering issue appeared after `new Type:` and was fixed by skipping newlines before consuming the block indent.

## 2026-05-17T23:45:00Z Task:3 type-checker registry integration
- `propertyless_constructor_rejects_unknown_user_type` initially failed with `MissingDocComment` because the fixture used `entry`; switching that fixture to a local `let demo = f...` avoided unrelated doc-comment gating and exposed the intended constructor error path.

## 2026-05-17T00:00:00Z Task:4 codegen registry lowering
- Initial inferred-bytes codegen test shape (`let len: int32 = buffer.length`) failed in codegen with `unknown field `length` on receiver expression`; this crosses into broader inference/member-typing behavior and is out-of-scope for Task 4.
- Resolved by narrowing inferred test to constructor-lowering evidence only (`let buffer = new Bytes` + IR assertions), keeping task boundaries intact.

## 2026-05-17T22:19:36Z Task:5 constructor inference fix
- New regression test initially failed as expected (`codegen_inferred_new_bytes_member_access_uses_bytes_type`) because constructor inference fell through to the default `CoreType::Int64` branch.
- Fix was narrowly scoped to constructor inference only; non-constructor inference paths and member rules were kept unchanged.

## 2026-05-17T23:59:00Z Task:6 inferred-bytes e2e fixture
- Rust analyzer reports `unlinked-file` for `tests/integration_e2e/bytes_stdlib.rs` in this workspace; this is an IDE linkage hint only and not a compile/test failure.
- Compatibility verification required running legacy and explicit-new bytes integration tests separately after introducing the inferred fixture test to confirm no behavior regressions.


## 2026-05-17T22:31:35.869767+00:00 Task 8 issues
- No new blockers. The only notable runtime cost was cargo lock contention from parallel test invocations, but all targeted commands completed successfully.

## 2026-05-17T18:38:49-04:00 Task: final-wave remediation F2/F4
- No new blocker: parser drift root cause was localized to unbounded dot-chain parsing in `src/parser/new_expression.rs`.
- Needed to keep fix narrowly scoped: adding only one parser guard plus one targeted parser test avoided unnecessary type-checker or registry churn.
