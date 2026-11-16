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
- [ ] Structured frontend increment commit and unchanged pre-commit gates.

## Remaining integration warning

The test AST and parsing entry point are not yet connected to a test compiler,
resolver, artifact builder or runner. Native execution, full static validation,
abort transport, lifecycle, scheduling/reporting and support libraries remain
required. Do not interpret parsing fixtures as full feature conformance.
