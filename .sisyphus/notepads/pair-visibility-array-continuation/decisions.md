# Decisions

## 2026-05-05 Task: bootstrap
- Continue from existing plan `.sisyphus/plans/pair-visibility-array-continuation.md`.
- Treat Task 1 implementation as already completed in code (pending plan checkbox closeout + evidence gate).
- Next execution target after closeout is Task 2 (Pair smoke project + prior-slice sanity checks).

## 2026-05-05 Task 4: double arrays
- Kept the existing pointer-plus-side-metadata representation for normal arrays and introduced explicit `{ptr,len,cap}` row values only for nested array elements, minimizing surface area while satisfying jagged row `.length` and nested read requirements.
- Added `opal_array_bounds_error(uint64_t, uint64_t)` to the runtime instead of synthesizing bounds strings in LLVM so nested bounds diagnostics stay precise without expanding the generated-program string ABI.
