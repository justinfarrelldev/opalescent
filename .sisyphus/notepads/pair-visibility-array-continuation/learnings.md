# Learnings

## 2026-05-05 Task: bootstrap
- Initialized continuation notepad for pair-visibility-array-continuation.
- Prior completed Task 1 commit is `2989482 feat(array): expose Pair`.
- Key risk for upcoming `.zip` task: receiver-based Pair field access on expressions like `pairs[0].first` (carry forward for QA).

## 2026-05-05 Task 2: Pair smoke coverage
- Mirrored the existing array integration harness by adding `array_pair_runs` through `assert_stdout`, preserving the shared `target/program` line stripping behavior.
- Added `test-projects/array-pair` with the exact plan fixture and expected stdout so Pair field access is exercised without introducing new language surface.
- Serialized append/push/pop/map/filter/reduce sanity checks stayed green, so no fix-forward changes to prior array slices were needed.
