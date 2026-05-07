# Draft: Guard Error Propagation

## Requirements (confirmed)
- Statement guards use the same typed path as expression guards.
- Do not let the success binding exist inside the else branch.
- Treat guard error bindings as must-handle values.
- Implement handlers in `error-handler-proposals/propagation-only`.
- Do not include direct return handling: `return err` should not be valid in guard error clauses.
- Add `propagate err` as the terminal statement for guard error clauses when the error should continue upward to the function surrounding the guard.
- Add a build error if a guard error clause only propagates upward and does nothing else; require shorthand `propagate fallible_function()` in those cases.
- Use extensive TDD with strict red-green-refactor.
- Create multiple test-projects for end-to-end verification.
- Existing broken test projects must be fixed, not skipped/commented/deleted.
- Use atomic commits and commit often during implementation.

## Technical Decisions
- Planning only in this session; implementation and commits will be delegated via `/start-work`.
- Test strategy requested by user: TDD / RED-GREEN-REFACTOR.

## Research Findings
- Pending: guard compiler implementation mapping (`bg_71f63051`).
- Pending: test infrastructure and test-project workflow (`bg_b8019ae0`).
- Pending: semantics sanity-check from Oracle (`bg_13395eb3`).

## Open Questions
- Whether the plan should require Sisyphus to actually create commits at each TDD slice, given implementation agents can commit only when explicitly directed by the plan/user.
- Exact diagnostic wording preferences if the repo does not already define matching diagnostic style.

## Scope Boundaries
- INCLUDE: compiler implementation, proposal handlers, tests/test-projects, migration/fixes for existing test projects broken by semantic changes, atomic commit strategy in execution plan.
- EXCLUDE: direct `return err` support in guard error clauses.
