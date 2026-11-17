# Native testing framework delivery checklist

Specification: `TESTING_FRAMEWORK.md` (all normative sections and acceptance tests).

- [x] Read repository instructions, specification, README, standard-library reference, crash course, and language-spec files.
- [x] Phase 0: verify compiler pipeline, CLI, editor APIs, relevant frontend baseline tests; record source map and syntax adaptations (`plans/native-testing-framework-source-map.md`).
- [ ] Phase 1: explicit test-module mode, prose lexing and spans, structured AST/parser, documentation, test-only imports, assertions and doubles.
- [ ] Phase 2: canonical subjects, nominal Worlds, graph and injection validation, unused parameters, examples, typed matchers/doubles, callback error inference.
- [ ] Phase 3: compiled descriptors/wrappers, per-attempt state, deterministic lifecycle, independent internal failure transport, primary/secondary failures.
- [ ] Phase 4: matcher vocabulary, single operand evaluation, diagnostics, import-edge interception, stubs/replacements/spies/history/verification.
- [ ] Phase 5: discovery, independent fresh-process bounded scheduling, stable examples, selection, exclusion, retries/timeouts, text/JSON/JUnit and association views.
- [ ] Phase 6: explicit directory helpers, PTY driver, normal application artifact, editor unit/integration/mocked/E2E fixtures.
- [ ] Phase 7: grammar, lifecycle, mocking, reporting, parallel responsibility and limitations documentation.
- [ ] Run every Section 20 acceptance category and audit all Section 22 completion requirements.
- [ ] Thorough maintainability/safety code review; fix findings and rerun verification.
- [ ] Atomic commits using the unchanged pre-commit hook.

## Red–green–refactor evidence

Unchecked phases are not implemented or verified.

- [x] Prose lexer RED: new regression tests fail because explicit test mode is absent.
- [x] Prose lexer GREEN: explicit mode and source-preserving contextual titles; 28 lexer tests pass.
- [x] Malformed quoted title RED: consumes the following declaration.
- [x] Malformed quoted title GREEN: bounded string scanner; all 29 lexer tests pass.
- [x] Refactor: isolate prose scanning; use explicit `ModuleMode` rather than another state boolean; ordinary multiline strings unchanged.
- [x] `cargo make lint`: all-target/all-feature strict Clippy passes.
- [x] First increment committed as `7be4b90`; unchanged pre-commit gates passed (Rust tests/lint/build and extension checks).
- [x] Structured test-module parser RED/GREEN: ordinary callbacks, Worlds, scenarios, hooks and metadata.
- [x] Named examples/test-support imports RED/GREEN; explicit provider/handle import AST.
- [x] Assertions RED/GREEN: token-bounded fixed vocabulary, polling and call-history matchers; no raw-text splitting.
- [x] Import-edge declaration and double configuration RED/GREEN; parent-relative import grammar preserves original paths.
- [x] Scheduler exclusion metadata RED/GREEN; only constant strings accepted.
- [x] Callback error recovery RED/GREEN: any recovered parser error invalidates the test module.
- [x] Minimal fixture documentation RED/GREEN: retain documentation between indented test declarations.
- [x] Structured statement visitors preserve operand traversal for capture/reference/naming analysis; production checker/lowering reject unlowered test operations.
- [x] Extract existing diagnostic/expression/store-mode helpers without changing production semantics to keep file-length gates intact.
- [x] Minimal fixture added at `test-projects/native-testing-basic`; parsed by regression tests (not yet executable through CLI).
- [x] Frontend verification: 11 new parser tests; 268 existing/new parser tests passed with 1 existing ignored; strict lint passes.
- [x] Structured frontend committed as `0096a3c`; all unchanged pre-commit gates passed.
- [x] Native loader RED/GREEN: explicit test/support mode, ordinary dependency grammar, original-source diagnostics and `.types.op` restrictions.
- [x] Resolved World graph RED/GREEN: canonical alias identity, missing/duplicate dependencies, closed cycle paths, deterministic diamond initialization order.
- [x] Function subjects RED/GREEN: genuine local/imported callables, privacy, aliases and qualified standard-module references use canonical defining identities.
- [x] Native foundation regression tests: 12 passed; strict all-target/all-feature lint passed.
- [x] Native loading/resolution committed as `3b26b4c`; all unchanged pre-commit gates passed.
- [x] Case metadata RED/GREEN: stable named-case expansion, one shared source body, inherited/replaced lists, subject/kind overrides, duplicate names and conflicting clauses.
- [x] Hook/transitive requirements retain separate origins and do not become body injections.
- [x] Example reordering regression proves the complete structured identity set remains unchanged.
- [x] Native resolution verification: 17 tests passed; strict all-target/all-feature lint passed.
- [x] Case metadata committed as `9d385cb`; all unchanged pre-commit gates passed.
- [x] Generated handle typing RED/GREEN: two providers with the same state have distinct canonical nominal types and typed `.state` fields.
- [x] Typed callback use checking RED/GREEN: binding-span tracking rejects unused/shadowed injections; another callback's use does not satisfy the contract.
- [x] World construction diagnostic RED/GREEN: only the runner may construct generated handles.
- [x] Refactor: lambda and declaration signatures share one nominal/generic type mapping implementation; existing library tests remain green.
- [x] Typing increment verification: 1,693 library tests passed (8 existing ignored); strict lint and file-length gates passed.
- [x] Initial handle/callback typing committed as `39f1f7a`; all unchanged pre-commit gates passed.
- [x] Matcher typing RED/GREEN: scalar equality, numeric/string/empty/ordered-array operations and genuine nominal variants; unsupported operand types fail checking.
- [x] `errors inferred` RED/GREEN on managed callbacks: collect only escaping nominal application errors; handled errors and assertion failures do not widen the set; nested ordinary helpers retain explicit rules.
- [x] Inference uses the existing guard/propagate and explicit `using` error paths without introducing cleanup or ownership machinery.
- [x] Matcher/error-inference committed as `830b869`; all unchanged pre-commit gates passed.
- [x] Test-support helper import RED/GREEN: establish explicit test-support closure before ordinary dependencies, permitting normal helper imports from tests without enabling test grammar in application dependencies.
- [x] Body signature contract RED/GREEN: nominal order/reference modes, missing/extra parameters and a final mutable context slot.
- [x] Generated test context has typed runner metadata fields; ordinary application grammar/type mapping remains unchanged.
- [x] Current verification: 1,703 library tests passed (8 existing ignored); strict lint/file-length gates being rechecked before commits.
- [ ] Support-module loading fix commit and unchanged pre-commit gates.
- [ ] Body/context contract increment commit and unchanged pre-commit gates.

## Remaining integration warning

The test AST/parser now feed an explicit source loader and canonical World/function
resolution APIs. These are not yet connected to an executable test artifact builder
or CLI runner; full callback/type checking is still pending. Native execution, full static validation,
abort transport, lifecycle, scheduling/reporting and support libraries remain
required. Do not interpret parsing fixtures as full feature conformance.
