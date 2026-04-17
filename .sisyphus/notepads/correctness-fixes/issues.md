
- 2026-04-17: Initial implementation appended inferred array length arguments before lambda capture arguments, which broke one lambda closure codegen test. Fixed by restricting length-argument augmentation to identifier callees.
