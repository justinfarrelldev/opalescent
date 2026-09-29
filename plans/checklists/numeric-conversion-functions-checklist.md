# Numeric conversion functions implementation checklist

- [x] Update numeric conversion proposal to use plain checked names (`int64_to_int32`) and forbid unchecked/saturating/wrapping/fallback variants for this milestone.
- [ ] Add red tests and fixture coverage for successful checked conversions and range failures.
- [ ] Register numeric conversion functions in `standard.numeric` and root `standard` where appropriate.
- [ ] Add numeric range error family/type registration for `IntegerRangeError` / `NumericConversionErrors`.
- [ ] Add LLVM declarations and codegen runtime-name resolution for numeric conversions.
- [ ] Implement C runtime conversion functions with value-preserving range checks.
- [ ] Update docs and example fixtures to use the new conversion functions.
- [ ] Run focused tests for numeric conversions.
- [ ] Run relevant terminal-simple-editor/test-project coverage and final regression checks.
- [ ] Review implementation for maintainability and safety.
