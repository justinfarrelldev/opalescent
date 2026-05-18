
## 2026-05-18 21:xx:xxZ Task 14
- The codegen path did not have a direct alias resolver for local type aliases, so the minimal safe fix was to consult canonical expected/imported `CoreType` metadata before falling back to raw constructor names.
- Verification used a focused imported-alias regression to prove the canonical lookup path and avoided widening the compiler pipeline.
- Scope-fidelity cleanup removed the unrelated signature reflow in `standard_symbols_core_io_and_bytes_foundational_filesystem.rs` while leaving the feature logic untouched.
- The remaining F4 blocker was a single removed blank line in `src/codegen/statements.rs`; restoring it preserved semantics and satisfied the scope-fidelity gate.
