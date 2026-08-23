# Problems



## Task 20 coordination constraint - 2026-08-21
- User/Atlas coordination constraint: use at most 2 agents/models at one time. Prefer one synchronous implementation/fix agent, and do not start parallel background agents unless explicitly necessary for a future task.

## Task 21 preferred session timeout - 2026-08-21
- Preferred reuse session `ses_fda3030e1ffeelHnhv86kvRP9r` timed out twice at the 30-minute poll limit and reported no file changes both times. Treat that session as ineffective/unavailable for Task 21; continue with a single fresh implementation agent rather than running parallel agents.
- Fresh Task 21 session `ses_fd9f61ceaffe8siHnloi56QApG` also timed out at the 30-minute poll limit without implementation changes. Task 21 remains unchecked and blocked by delegation timeouts; per continuation policy, move to next independent task while preserving this blocker.

## Task 22 full-task timeout - 2026-08-21
- Full Task 22 delegation `ses_fd9d90dd6ffesBPmKoBQNPVyNa` timed out at the 30-minute poll limit without code changes. Continue Task 22 as smaller sequential implementation slices with one synchronous agent at a time; do not run parallel agents.

## Task 22 coordinator slice - 2026-08-21
- Lease invalidation is tested by acquiring a lease in `Free`, then attempting `Free -> Opening`; the reservation advances the hidden epoch, so that pre-reservation lease remains stale even after returning to `Free`.

## Task 22 ABI migration finding - 2026-08-21
- Existing generated I/O fixtures and the direct C harness still encoded the pre-Task-22 infallible ABI. They had to be migrated to explicit `propagate` handling and `{ value, error }` assertions; leaving any one fixture unchanged causes either a type-check failure or a C amalgamation compile failure.

## Task 22 remaining live fixture fallout - 2026-08-21
- The remaining migration surface included terminal/stdout Game of Life fixtures, writer integration fixtures, `saferm` confirmation branches, formatter/spec examples, and public standard-library documentation. All now propagate the new acquisition/input/capability errors while preserving Free-state output behavior.

## Task 22 acceptance wording cleanup - 2026-08-21
- Coordinator family constants now pass bare family names to the shared formatter, producing one `TerminalCoordinatorUnavailable { state, operation }` variant; `STDLIB.md` documents EOF as `StandardInputReadError: EndOfInput` while preserving successful partial final-line reads.

## Task 23 inherited branch problem - 2026-08-23
- Broad `cargo test --features integration` remains blocked by the unrelated proposal-traceability test `tests::terminal_aggregate::terminal_aggregate_proposal_traceability_mentions_private_runtime_contract`; follow-up terminal work should not treat that failure as a generated process-control regression.
