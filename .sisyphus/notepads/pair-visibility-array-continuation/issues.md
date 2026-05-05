# Issues

## 2026-05-05 Task: bootstrap
- Repository contains unrelated modified/untracked files from previous sessions; avoid including them in continuation scope.
- Serialized integration commands with `--test-threads=1` are required to avoid shared `target/program` race behavior.

## 2026-05-05 Task 2: Pair smoke coverage
- `.op` LSP diagnostics are unavailable in this environment because no language server is configured for `.op`; end-to-end cargo integration runs were used as the verification path for `test-projects/array-pair/src/main.op`.
- Workspace contains unrelated modified/untracked files from prior sessions, so commit staging must stay scoped to the Task 2 fixture, test, evidence, and notepad updates only.
