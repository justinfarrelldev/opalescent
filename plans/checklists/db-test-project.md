# Small database checklist

Contract: [`db-test-project.md`](../../db-test-project.md). This checklist tracks progress; it does not change the contract.

- [x] Investigate the starting revision, language/runtime surfaces, and reference projects.
- [x] Start the application README and root observations log.
- [x] Commit the first native red/green/refactor increment for all three typed values.
- [x] Verify basic tokens, ASCII name normalization, initial keywords, and source positions.
- [ ] Complete tokenization, with literal and comparison-operator tests.
- [ ] Deliver a tested `CREATE TABLE` / `INSERT` / wildcard `SELECT` vertical slice.
- [ ] Add projections, resolved predicates, precedence, limits, `UPDATE`, and `DELETE`.
- [ ] Add REPL, introspection, formatting, and native terminal integration.
- [ ] Verify invalid mutations and independent ownership of results; complete the Milestone 1 gate.
- [ ] Implement and test deterministic, versioned snapshot encoding and validated candidate decoding.
- [ ] Implement and test filesystem save/load, startup, dirty state, and failure recovery.
- [ ] Verify native filesystem doubles, real filesystem fixtures, and cross-session terminal lifecycle.
- [ ] Run the workloads, review all changes, verify regressions, and complete docs and acceptance review.
