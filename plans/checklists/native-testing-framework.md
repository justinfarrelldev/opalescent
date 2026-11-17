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
- [x] Minimal fixture added at `test-projects/native-testing-basic`; parsed by regression tests.
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
- [x] Current verification: 1,703 library tests passed (8 existing ignored); strict lint and file-length gates passed.
- [x] Support-module loading fix committed as `089a9e8`; all unchanged pre-commit gates passed.
- [x] Body/context contracts committed as `385b63c`; all unchanged pre-commit gates passed.
- [x] Source-to-checker integration RED/GREEN: load ordinary dependency interfaces, register nominal aliases, check setup/cleanup/hooks/bodies and validate example records/constants before execution.
- [x] Minimal project fixture passes the integrated static pipeline.
- [x] Lifecycle contract tests cover dependency setup, activation-only use, ordered cleanup subsets and legal omitted cleanup.
- [x] State-reference RED/GREEN: readonly World mutation rejected; mutable `.state` updates permitted; `.state` borrows remain projections of the original handle; overlapping mutable borrows and by-value World helper parameters rejected.
- [x] Compiler callback constraints are solved at each managed boundary, preserving ordinary source diagnostics.
- [x] Integrated checking committed as `75d7575`; 1,711 library tests passed (8 existing ignored), and all unchanged pre-commit gates passed.

- [x] Double edge resolution RED/GREEN: load referenced application sources, resolve canonical direct import edges, preserve importing-module aliases, and reject same-module/global-symbol replacement.
- [x] Double declaration aliases retain one provider identity through explicit support imports.
- [x] Refactor: Worlds and doubles share fixed-point declaration-import resolution.
- [x] Double resolution verification: 26 native-source regression tests passed; strict lint passed.
- [x] Double edge resolution committed as `96dd86a`; all unchanged pre-commit gates passed.
- [x] Typed double RED/GREEN: canonical handles, ordered reference/result/error contracts, exact rules, replacements, spying, verify/restore, typed counts and owned-history operand validation.
- [x] Unsupported snapshot operands retain count-only spying; exact rules/history comparisons produce explicit compiler diagnostics.
- [x] Borrowed configuration RED/GREEN: conditional exact-rule arguments are immediate value snapshots, not captured borrow addresses.
- [x] World/double integration RED/GREEN: declare nominal identities before state layouts/signatures, resolve original application interfaces, then check helpers and managed callbacks.
- [x] World safety RED/GREEN: reject handle reconstruction through record updates and copying through generic owned parameters.
- [x] Stored affine result RED/GREEN: reject duplicating existing affine owners in reusable exact returns; replacements/spies can produce results normally.
- [x] Generic helper RED/GREEN: provisional interfaces preserve generic parameters; ordinary substitution preserves quantified callable schemes and avoids imported/local numeric type-variable collisions.
- [x] Refactor: share generic signature/body binding resolution and isolate lexical reference-capture traversal, retaining parameter shadowing through control flow.
- [x] Current verification: 1,728 library tests passed (8 existing ignored); strict all-target/all-feature lint passed.
- [x] Typed double checking committed as `8db96e7`; all unchanged pre-commit gates passed.

- [x] Nominal source typing RED/GREEN: same-spelled declarations in separate modules remain distinct through aliases, variant matchers and double error contracts.
- [x] Source error alias RED/GREEN: a genuine error alias is not reinterpreted as an unrelated standard error family with the same spelling.
- [x] Ordinary record annotation/layout RED/GREEN: source constructors, typed local bindings and public field manifests retain defining type identity.
- [x] Context binding RED/GREEN: later foreign imports cannot replace the implicit runner context; callback parameter names do not shadow its generated type.
- [x] Native application typing retains ordinary grammar and test-only import availability checks.
- [x] Minimal typed-double project added at `test-projects/native-testing-doubles`, covering World-owned activation, exact returns, nominal application errors, ordinary guards and verification.
- [x] Canonical source typing verification: 1,737 library tests passed (8 existing ignored); strict all-target/all-feature lint and file-length gates passed.
- [x] Canonical source typing committed as `cd0ae13`; all unchanged pre-commit gates passed.

- [x] Invalid mock error value RED/GREEN: reject a bare source sum family or a variant requiring unsupplied payload fields; replacements retain ordinary typed payload construction.
- [x] Error-value validation verification: 1,738 library tests passed (8 existing ignored); strict lint and file-length gates passed.
- [x] Mock error value validation committed as `3436b95`; all unchanged pre-commit gates passed.

- [x] Internal LLVM outcome primitive RED/GREEN: completed results retain the original application ABI, aborted results retain only failure identity and leave application slots undefined; void has no dummy slot.
- [x] Native call branching RED/GREEN: explicit abort/completed edges precede application-result extraction and leave abort scope exits to existing cleanup emission.
- [x] Host-executable LLVM regression: an integer-producing callee's abort crosses a string-producing caller unchanged, linked as a real native executable.
- [x] Refactor native call regression into emission/linking helpers; strict lint passes.
- [x] Internal LLVM outcome primitive committed as `ecaa1bc`; 1,742 library tests passed (8 existing ignored), and all unchanged pre-commit gates passed.
- [ ] Integrate the outcome primitive into all native source function declarations, ordinary/higher-order calls, returns, guard/propagate paths, lexical scope exits and compiled callback wrappers.

- [x] Registered native function/source-return RED/GREEN: ordinary integer, void, application-error, implicit void, propagation and guard-error forwarding emit completed outcomes using explicit original ABI metadata.
- [x] Production declarations/returns remain unchanged in the same LLVM context; conflicting declarations and variadic native ABI paths are rejected.
- [x] Source-return integration verification: 1,749 library tests passed (8 existing ignored); strict lint and file-length gates passed.
- [ ] Native function/source-return increment commit and unchanged pre-commit gates.

## Remaining integration work

- [x] Resolve and type-check double declarations and configurations in the integrated native source pipeline.
- [ ] Implement supported test-build interception/instrumentation for checked double operations.
- [x] Resolve cross-module nominal application type/error collisions for same-spelled declarations and imported aliases.
- [ ] Complete the nominal type audit through generic ADT imports/examples, remaining annotation paths and executable lowering.
- [ ] Preserve local source aliases in diagnostics/suggestions and source-facing type names in reflection when lowering canonical identities.
- [x] Reject source mock error references that omit a variant or its required payload.
- [ ] Audit built-in mocked error constructors against their supported executable ABI; reject unsupported foreign/resource-bearing payloads explicitly.
- [ ] Complete all remaining static validation and compiler acceptance tests.
- [ ] Implement native artifact lowering and executable test artifact builds.
- [ ] Implement the internal test-abort channel separately from application errors.
- [ ] Implement and verify runtime World, hook, body and cleanup lifecycle behavior.
- [ ] Connect discovery, selection and execution to the compiler CLI.
- [ ] Implement bounded fresh-process parallel scheduling, retries, timeouts and exclusion keys.
- [ ] Implement structured result transport and text, JSON and JUnit reports.
- [ ] Implement testing support libraries and editor conformance fixtures.
- [ ] Verify executable end-to-end conformance, not just parsing or static checking.
