# Decisions — operator-tests plan

## [2026-04-13] Scope Decisions

- Do NOT fix `Equal`/`NotEqual` dead variants — flag in commit message only
- Do NOT modify existing tests — only ADD new tests
- Only file to modify: `src/parser/tests.rs`
- Do NOT add runtime/semantic evaluation tests — parser/AST level only
- Momus approved plan after fixing precedence pair count from 13→12
