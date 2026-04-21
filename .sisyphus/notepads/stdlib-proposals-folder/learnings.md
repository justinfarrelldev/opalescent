# Learnings

## [2026-04-21] Session ses_251dc2dfeffej3OqV1HXfx3obc — Initial Setup

### Opalescent Style Rules (18 binding rules)
1. Signatures: `let name = f(params): return_type errors E1, E2 =>`
2. Arrays: `T[]` never `[T]`
3. Verbose names: `greatest_common_divisor` not `gcd`
4. Doc block: `##\nDescription: …\n##` ≥30 chars on every public fn
5. Explicit returns: `return <expr>` or `return void`
6. No semicolons
7. snake_case fns/vars, PascalCase types, types in `*.types.op`
8. Error handling: `guard <expr> into <bind> else <err_bind> => <block>` and `propagate`
9. Imports: bare=stdlib, `./`=local, `@scope/name`=pkg; type imports: `import T from ./foo.types` OR `import type T from ./foo.types`; multi-import, aliasing all valid
10. Operators: `is`/`is not`, `band`/`bor`/`bxor`/`bnot`/`bshl`/`bshr`/`bushr`, `and`/`or`/`xor`/`not`
11. Memory model fixed (Perceus+SCR) — no proposals to change it
12. Constructors: `new TypeName:` with indented fields
13. Bare modules: `standard`, `math`
14. `_sync` suffix on any fn with plausible async future (file-io, network, subprocess, logging-flush, time-sleep, stream-hash, stream-compress, streaming-serialization)
15. Comments: `#` single-line, `##…##` doc-blocks only
16. Full usage examples mandatory — every proposed method needs a realistic call site
17. Exhaustive `errors` clauses — no placeholders
18. Every fallible call handled via `guard` or `propagate`

### Key File Locations
- Language spec: `/home/justi/Projects/opalescent/language-spec/`
- Memory model proposals (structural reference): `/home/justi/Projects/opalescent/memory-model-proposals/`
- Stdlib: `/home/justi/Projects/opalescent/stdlib/prelude.op`
- Error handling samples: `/home/justi/Projects/opalescent/language-spec/error_handling_samples.op`
- Types example: `/home/justi/Projects/opalescent/language-spec/types_example.types.op`
- Modules spec: `/home/justi/Projects/opalescent/language-spec/requirements/modules.md`

### Proposal Template (10 sections in order)
Overview, Assumes, Syntax Design, Example Applications, Strengths, Weaknesses, Impact on Existing Syntax, Interactions with Other Concerns, Implementation Difficulty, Must NOT Have

### COMPARISON.md Axes (6 fixed)
Ergonomics, Error-model fit, Opalescent-idiom fit, Implementation effort, Extensibility, Async readiness

### Target: 58 alternatives across 19 concerns

## [2026-04-21] Testing framework concern authoring
- The `testing-framework` concern now has five alternatives with proposal docs using the 10-section template in order.
- `COMPARISON.md` was authored with exactly the fixed six axes: Ergonomics, Error-model fit, Opalescent-idiom fit, Implementation effort, Extensibility, Async readiness.
- Vitest-style alternative is most complete when examples explicitly show suite hooks, `describe` + `it`/`test`, matcher calls, and mock/stub/spy workflows.
- Keeping all domain types in `*.types.op` files improved consistency; additional `testing.types.op` files were added where necessary.

## [2026-04-21] Syntax polish pass across stdlib-proposals .op files
- Canonical syntax inferred from language-spec + real project usage:
  - Prefer single-quoted strings with `{}` interpolation.
  - Prefer `is` / `is not` over `==` / `!=`.
  - Guard pattern is `guard call(...) into value else err => ...`; `propagate` is explicit.
  - Canonical imports are `import X, Y from module` (not bare quoted path imports).
- Applied syntax-only normalization in stdlib-proposals:
  - Converted remaining double-quoted string literals to single-quoted literals in edited files.
  - Replaced remaining `==` occurrences with `is` in proposal code.
  - Replaced `import './x.op'` style with explicit `from` imports in regex proposal files.
- Verified `bash stdlib-proposals/.style-gate.sh` passes after edits.

## [2026-04-21] Momus high-accuracy review findings
- Structural coverage check confirms 19 concerns exist and each concern has a `COMPARISON.md` with all six required axes.
- Alternative-level artifact check confirms every alternative has `proposal.md` and at least two `.op` files.
- README concern index is stale for `testing-framework`: README lists 1 alternative (`vitest-style-describe-it`) while filesystem currently contains 5 alternatives.
- Syntax drift still present in proposal examples despite style-gate passing: several files use `bool` in `.op` signatures/fields instead of language-spec `boolean`.
- `testing-framework/vitest-style-describe-it/mock_stub_spy.op` includes the expected full mock/stub/spy primitives and matcher coverage in one scenario.
