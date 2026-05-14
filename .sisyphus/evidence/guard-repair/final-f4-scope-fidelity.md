VERDICT: REJECT

# F4 Scope Fidelity Check — deep

## Scope baseline
Normative scope comes from `.sisyphus/plans/guard-error-propagation-repair.md`:
- Named long-form guard error clauses are legal only when they perform real handling and end with final top-level `propagate <active_error_binding>` (`:66-73`).
- Ordinary `propagate <fallible_call>()` must remain unchanged (`:80`).
- `return err` must remain banned (`:81`).
- No strict-mode/config flag (`:25`, `:86`).
- Wrapper-return drift must be follow-up only; do not change wrapper-return support/tests in this repair (`:36`, `:87`).
- Evidence must be sealed and final gate must have clean git status (`:62-64`, `:448-454`).

## Findings

### 1) Named guard requires real handling + final `propagate err` — PASS
`src/type_system/checker/expressions_guard.rs:385-419` now computes `clause_has_real_handling` via `guard_clause_prelude_has_real_handling(prelude)` and passes that into terminal validation. `type_check_guard_error_propagate_terminal` at `:605-652` rejects `Stmt::PropagateGuardError` unless it is for the active binding and `allow_terminal_propagate` is true, which only happens when real handling was found. This matches the repair plan’s normative predicate.

### 2) Ordinary `propagate <fallible_call>()` unchanged — PASS
`tests/integration_e2e/guard_stmt.rs:551-583` contains `guard_stmt_propagate_call_valid_project_compiles_links_and_runs`, explicitly asserting shorthand propagate-call support still compiles and runs. The Task 8 targeted gate output in `.sisyphus/evidence/guard-repair/task-8-final-gate/guard-stmt.txt:49` shows that test passed.

### 3) `return err` remains banned — PASS
`src/type_system/checker/expressions_guard.rs:529-534` still rejects direct forwarding of the active guard error binding with `TypeError::GuardReturnErrInvalid`. Coverage exists in `tests/integration_e2e/guard_stmt.rs:433-453` and `src/type_system/tests.rs:3534-3545`; the targeted guard gate shows `guard_stmt_return_err_banned_project_emits_return_err_diagnostic ... ok` at `.sisyphus/evidence/guard-repair/task-8-final-gate/guard-stmt.txt:39`.

### 4) No strict-mode flag introduced — PASS
Repo scans found no strict-mode/config toggle in the relevant checker/test scope, matching the repair-plan guardrail.

### 5) Wrapper-return drift handled as follow-up only — FAIL
The repair plan explicitly narrowed scope away from wrapper-return changes (`.sisyphus/plans/guard-error-propagation-repair.md:36,87`). However, the current audited state includes active wrapper-return checker logic in `src/type_system/checker/expressions_guard.rs:529-759`, wrapper integration tests in `tests/integration_e2e/guard_stmt.rs:455-549`, and history showing wrapper coverage landed in this repair line (`GIT_MASTER=1 git log -S "guard-stmt-wrapper-valid" ...` => commit `457e3d7 feat(guard): finalize strict terminal handler validation and wrapper coverage`). Task 7 follow-up evidence says wrapper drift was recorded only as follow-up (`.sisyphus/evidence/guard-repair/task-7-sweep/follow-up-wrapper-return-drift.md:5-8`), but the code/test/history state shows wrapper support/coverage was not merely left untouched drift; it was part of the implemented repair slice. That is scope expansion relative to this repair plan.

### 6) Evidence sealed — FAIL
The plan requires sealed evidence with clean final git status (`.sisyphus/plans/guard-error-propagation-repair.md:62-64, 448-454`). But `.sisyphus/evidence/guard-repair/task-8-final-gate/git-status.txt:1-30` shows a non-clean tree with modified tracked files and untracked guard-repair evidence, plan, notepads, and legal sibling fixtures. Because the final-gate artifact itself records a dirty tree, the evidence cannot be considered sealed under this plan’s own acceptance criteria.

## Explicit check summary
- Ordinary `propagate <fallible_call>()` unchanged: PASS
- `return err` banned: PASS
- Named guard requires real handling + final `propagate err`: PASS
- Wrapper-return drift follow-up only / no scope expansion: FAIL
- Evidence sealed: FAIL

## Final reasoning
The semantic repair for named guard propagation is correct and the ordinary propagate / `return err` invariants are preserved. But F4 is a scope-fidelity gate, not just a semantic gate. Under this repair plan, wrapper-return behavior was supposed to remain follow-up-only, and final evidence was supposed to be sealed with clean git status. The current audited state violates both conditions.

## Verdict basis
REJECT because the repaired semantics do not stay within the exact repair-plan scope, and the final-gate evidence does not satisfy the plan’s sealing requirement.