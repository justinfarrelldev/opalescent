# Issues

## [2026-04-16] Known issues at session start
- `module_resolver.rs:316` — `string_to_int32` incorrectly returns `Int64` type (should be `Int32`) — must fix in Task 7
- `codegen_guard_statement` in `statements.rs` does not handle else_body or error_binding — Task 6 must implement full branching
- `codegen_propagate_expression` assumes struct field 1 is an int flag — must update for pointer-based error in Task 6
