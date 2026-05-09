## 2026-05-09 Task 1 follow-up
- Statement guards still diverge from expression guards in the checker: `src/type_system/checker/statements.rs::type_check_guard_statement` registers the success binding before and inside the else scope.
- The current baseline behavior is that a statement-guard success binding is visible inside the error clause, so the Task 1 RED evidence can be created by temporarily asserting the future opposite expectation and running that real test with `cargo test success_binding_currently_leaks_into_else_clause --lib`.
- The current guard error binding remains string-typed in both checker and codegen, which shows up in baseline tests as `let err_message: string = err` succeeding and `return err` failing as a `unit` vs `string` mismatch.
- Reliable cargo filters for these Task 1 unit tests are substring matches against the lib test binary, not the earlier zero-test exact invocation.
