# Error Propagation Diagnostics and VS Code Quick Fix Checklist

- [x] Red: add a non-ignored integration regression test proving `test-projects/game-of-life-full` compiles in normal test runs.
- [x] Green: fix the Game of Life Full project errors clause so the regression test passes.
- [x] Refactor/review the regression test for maintainability and minimal runtime cost.
- [x] Commit the Game of Life Full regression fix atomically.
- [x] Red: add compiler diagnostic test coverage for a propagated-error mismatch suggested fix that lists missing errors.
- [x] Green: implement the propagated-error mismatch suggestion in compiler diagnostics.
- [x] Refactor/review the compiler diagnostic implementation for clarity and stable output.
- [x] Commit the compiler diagnostic suggestion atomically.
- [x] Red: add VS Code extension test coverage for a quick fix on propagated-error mismatch diagnostics.
- [x] Green: implement the VS Code quick fix for adding missing errors to any function errors list.
- [x] Refactor/review the VS Code quick-fix implementation for generality and maintainability.
- [x] Run targeted Rust and VS Code extension tests.
- [x] Commit the VS Code quick fix atomically.
- [x] Run pre-commit without modifying or bypassing hooks.
- [x] Perform final code review of all changes.
