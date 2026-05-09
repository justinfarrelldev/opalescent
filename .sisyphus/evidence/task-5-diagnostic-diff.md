# Task 5 diagnostic diff (2026-05-09T04:49:53Z)

## Intentional drift from Task 1/3 baseline
- `type_system::tests::test_guard_statement_success_binding_currently_leaks_into_else_clause` now fails because statement guards no longer pre-register the success binding in the error clause. This is the intended Task 5 shared-path change.

## Preserved baseline / unchanged behavior for later tasks
- `type_system::tests::test_guard_statement_return_err_currently_fails_as_string_to_unit_mismatch` still passes, confirming Task 5 did not implement the later `return err` diagnostic semantics.
- `type_system::tests::test_propagate_call_remains_valid_unmodified` still passes, confirming ordinary `propagate <call>` behavior stayed unchanged.
- Full-suite failures remain concentrated in later Task 6/7 semantics: success-binding leak diagnostic wording test, `return err` rejection, `propagate err` terminal-only rule, only-propagate rejection, and missing handler rejection.

## Full-suite result summary
- `cargo test --all-features` => FAIL (expected later-slice REDs remain)
- Additional changed-semantics failure: the old baseline leak test now fails with `SymbolNotFound` for the success binding in the error clause, reflecting the successful shared-path refactor.
