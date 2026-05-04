# Task 2 Sandbox Guard Evidence

The test `fs_recursive_delete_from_op_source` in `tests/integration_e2e/fs_delete_directory_recursive.rs` includes an explicit safety guard before deletion:

1. It builds `fixture_root` under `std::env::temp_dir()` using `make_temp_path(...)`.
2. Before invoking `.op` deletion, it asserts:
   - `fixture_root.starts_with(std::env::temp_dir())`
   - `fixture_root.exists()`
3. If either condition fails, the test returns a descriptive error and does not execute recursive deletion.

This ensures destructive operations are scoped to sandboxed temp paths only.
