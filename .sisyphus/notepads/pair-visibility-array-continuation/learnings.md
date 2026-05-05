# Learnings

## 2026-05-05 Task: bootstrap
- Initialized continuation notepad for pair-visibility-array-continuation.
- Prior completed Task 1 commit is `2989482 feat(array): expose Pair`.
- Key risk for upcoming `.zip` task: receiver-based Pair field access on expressions like `pairs[0].first` (carry forward for QA).

## 2026-05-05 Task 2: Pair smoke coverage
- Mirrored the existing array integration harness by adding `array_pair_runs` through `assert_stdout`, preserving the shared `target/program` line stripping behavior.
- Added `test-projects/array-pair` with the exact plan fixture and expected stdout so Pair field access is exercised without introducing new language surface.
- Serialized append/push/pop/map/filter/reduce sanity checks stayed green, so no fix-forward changes to prior array slices were needed.

## 2026-05-05T18:48:47-04:00 Task 3: array zip
- Implemented `.zip` as compiler-lowered array codegen in `src/codegen/functions_call/array.rs`, using `min(left.length, right.length)` for allocation and publishing pending array metadata so `let pairs = left.zip(right)` tracks dynamic length correctly.
- Added concrete LLVM lowering for `Pair<T, U>` in `src/codegen/types.rs`, which lets zipped results store inline `first`/`second` fields instead of degrading to opaque pointers during code generation.
- Extended product field access fallback in `src/codegen/adts.rs` so expression receivers like `pairs[0].first` and `pairs[0].second` extract fields directly from the indexed Pair struct value instead of requiring an identifier-backed constructor binding.
- Added zip integration coverage in `tests/array_integration.rs` for unequal lengths, equal lengths, and empty-left/empty-right cases; the RED evidence now fails specifically on missing zip codegen and the GREEN evidence confirms truncation and Pair field reads.
