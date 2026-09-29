# Numeric conversion functions implementation checklist

- [x] Update numeric conversion proposal to use plain checked names (`int64_to_int32`) and forbid unchecked/saturating/wrapping/fallback variants for this milestone.
- [x] Add red tests and fixture coverage for successful checked conversions and range failures.
- [x] Register numeric conversion functions in `standard.numeric` and root `standard` where appropriate.
- [x] Add numeric range error family/type registration for `IntegerRangeError` / `NumericConversionErrors`.
- [x] Add LLVM declarations and codegen runtime-name resolution for numeric conversions.
- [x] Implement C runtime conversion functions with value-preserving range checks.
- [x] Update docs and example fixtures to use the new conversion functions.
- [x] Run focused tests for numeric conversions.
- [x] Run relevant terminal-simple-editor/test-project coverage and final regression checks (full integration suite still has pre-existing stdlib error-family warning expectation failures unrelated to numeric conversions).
- [x] Review implementation for maintainability and safety.
