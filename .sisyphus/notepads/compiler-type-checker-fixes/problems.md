# Problems — compiler-type-checker-fixes

## [2026-04-17] Unresolved: required commit blocked by pre-commit lint in inherited file
- Requirement asks commit to include both:
  - `src/type_system/checker/expressions.rs` (Task 2)
  - `src/codegen/statements.rs` (Task 3)
- Current state: commit hook fails due to clippy violations in Task 2 file (`unwrap_in_result`, `match_same_arms`, `missing_panics_doc`).
- Constraint conflict:
  - Must include Task 2 file in commit
  - Must not modify type-checker code in Task 3
- Outcome: commit remains blocked unless orchestrator permits a follow-up fix in Task 2 file or adjusts commit-scope constraints.
