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

- 2026-08-23T06:42:55Z: Task 24 remains blocked at the generated-runtime boundary by two concrete compiler/runtime limitations discovered during the final retry. First, the current error ABI and codegen document and implement only string-like error pointers for guard/propagate, while Task 24 requires generated programs to receive and inspect structured payload-bearing TerminalSessionOptionsError.InvalidOptions metadata. Second, Expr::Constrain and refinement-based variant inspection still lack production codegen over imported terminal nominal types, so generated Opalescent programs cannot yet construct public constrained terminal values broadly or inspect structured sum payloads such as TerminalInvalidOptions without extending compiler lowering beyond the current Task 24 subset.

- 2026-08-23T06:47:41Z: Final Task 24 retry proved imported terminal ADT field layouts can be threaded into codegen and imported propertyless terminal enum variants can be type-checked, but the remaining generated-runtime implementation is still blocked by a concrete ABI limitation: imported pointer-backed terminal product values such as TerminalSessionFeaturePolicy and TerminalSessionResourceLimits are lowered as raw LLVM nominal payload pointers with no dedicated stable C-facing layout contract. That makes the remaining 23 setter/inspector runtime functions unsafe to complete in C without first introducing a compiler-lowered or explicitly specified ABI path for those imported nominal/product values.
