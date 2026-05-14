# Legal sibling coverage

- `test-projects/delete-downloads-legal/src/main.op` keeps runtime coverage with a helper that logs `LEGAL_LIST_ERR=handled-before-propagate` and then ends the named guard handler with final top-level `propagate err`.
- `test-projects/delete-downloads-strict-legal/src/main.op` mirrors the same legal pattern with strict-prefixed deterministic stdout markers.
- `tests/integration_e2e/guard_stmt.rs` wires both siblings through `run_guard_stmt_project(...)` and exact `expected/stdout.txt` fixtures, while the original `delete-downloads` and `delete-downloads-strict` projects are now asserted as compile-fail negatives.
