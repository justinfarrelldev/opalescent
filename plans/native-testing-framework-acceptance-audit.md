# Native testing framework: conformance audit in progress

Specification: [original design](native-testing-framework-spec.md), Sections 1–22.
This audit refers to source files and executable tests at the revisions checked,
not earlier implementation summaries. It is not a completion sign-off: generic
application lowering and ordinary higher-order call paths still required
implementation or verification at those revisions.

## Verification scope

- `cargo test --lib --quiet -- --test-threads=1`: **1,832 passed, 8 existing ignored**.
- Strict `cargo make lint`, file-length and whitespace checks passed.
- Unmodified pre-commit gates passed through `f28eed5`, including production
  integration/error-family inventory, extension checks and all-feature builds.
- GCC/Clang C warning gates and Cppcheck passed. All changed testing C units passed
  direct Clang analysis. Standalone owned snapshot/double/attachment/PTY fixtures
  passed ASan/UBSan/leak verification; sanitizers are not integrated into the
  language feature or test runtime.
- Real CLI with fake-terminal overrides deliberately present: six editor cases
  passed, zero failed; 32 callback records, one transcript attachment, four double
  transitions and five World views were present in the JSON report.
- Real JUnit CLI run: six cases passed and the XML parsed with an XML parser.
- Known unrelated baseline: the aggregate Clang analyzer still diagnoses existing
  `unix.Malloc` paths in unchanged `runtime/opal_io.c` (1126/1145/1177 and related
  paths). This aggregate task is **not** claimed as passing. No checker or hook was
  disabled or weakened.

## Normative requirements and implementation evidence

| Specification | Current implementation/evidence |
| --- | --- |
| 1–3: finite compiled grammar, typed callbacks, readable documentation | Explicit lexer/parser test mode; structured AST; original bytes/spans; `native_project` emits real LLVM functions/descriptors. Parser tests accept inline/indented callbacks and reject invalid/recovered syntax. Paragraph documentation survives resolution and reports. |
| 4: discovery, explicit World/double support imports, ordinary application grammar, type files | `project`, `declaration_imports`, `module_loader`, explicit `LoadedModuleAst`; `.types.op` remains ordinary type-only syntax. `support_module_tests` proves test-library types work under tests without granting ordinary src imports test-only permission. |
| 5: apostrophes, hashes, colons, static quoted titles, original spans | `lexer/testing_tests.rs` covers every title delimiter/escape case and localized newline errors; parser uses tokens/expression boundaries, not textual replacement. Sibling names are checked without ordinal suffixes. |
| 6: resolved genuine subjects, aliases, overrides, categories, honest coverage | `subjects`, `cases/metadata`, `subject_tests`, `case_tests`; canonical defining identities and privacy checks. Multi-subject cases remain one total. Reports say `not_collected`; associations neither invoke subjects nor lock scheduling. |
| 7: nominal handles, setup/cleanup types, exact ordered references, mutability/usage, replacement/none, activation-only | `checker/native_testing`, `contracts`, `mutation`, `handles`, `cases/world_needs`; negative static tests cover missing/extra/reordered/duplicate/state-instead-of-handle/unused/shadowed injections, construction/copying and readonly mutation. Activation origins remain distinct. |
| 8: dependency resolution/cycles/diamonds/topology/cleanup subsets | `world_graph`, `world_resolution`, callback contracts and `lowering/planner`; graph tests verify closed cycle paths and source-order tie breaking; executable lifecycle descriptors initialize each canonical provider once and reverse cleanup. |
| 9: scopes/order/entered boundaries/failures/context | `opal_testing_lifecycle`, typed native callbacks and context; ten real C lifecycle scenarios verify traces and retained primary/secondary failures. World state is published only after setup succeeds; no failed setup state reaches cleanup. Context fields are runner-managed; attachments use explicit owned APIs. |
| 10: schema-checked constants, independent stable named cases, fresh values, one body | `examples`, `cases`, `native_project/cases`; schema failures are static. Reordering regression compares full identity sets. Real worker tests select independent examples from one executable. |
| 11: exact typed matchers and once-only operands/diagnostics/polling | `native_assertions`, value/string/array equality, snapshots, polling; executable matcher/product/polling fixtures exercise signedness, UTF-8, nested arrays/records, nominal tags, order/multiplicity and capture-before-mutation. Boolean C/source layout normalization preserves application ABI. Poll configuration runs once, only mismatches retry, errors escape immediately, latest diagnostics are owned. |
| 12: canonical pre-specialization import edges, attempt-local activation, exact/fallback/spy/restore/verify/history | `doubles`, `native_double_edges/rules/replacements/owned/history`, `native_standard_adapters`, C double engine. Managed/path history has explicit snapshot policy; unsupported history rejects at checking/listing while count-only spying works. Source and standard aliases/specialized calls preserve original bypasses, exact precedence, nominal errors and supported replacement ABIs. |
| 13: independent abort channel, no fabricated application values, existing lexical scope exits | `native_outcome/functions/abort`, explicit native ABI metadata, typed source/callback boundaries; linked guard regressions distinguish uncatchable test aborts from catchable deliberate application errors. Abort branches precede payload/error reads and use existing lexical RC/using cleanup. Managed callbacks infer only escaping application errors. |
| 14: fresh workers, bounded default parallelism, whole exclusion sets, retries/timeout/selection/reporting | `scheduler/processes`, dedicated validated transport and CLI options. Scheduler barriers/PIDs/key tests plus the real native two-sided example barrier prove overlap and isolation. Timeout records retain completed observations and mark unfinished callbacks interrupted with no invented duration. All required CLI selections/reporters are exercised. |
| 15: separate normal app, real PTY, input/screen/cursor/exit/transcript, explicit stop | E2E coordinator calls unchanged normal project compiler once per run in private application output. Opaque application descriptor is passed explicitly. Real PTY tests verify actual child cwd/window size, Unicode/cursor observations, input and exit/transcript. Fake backend variables are cleared. Normal binary symbol check rejects native-test ABI symbols. |
| 16: canonical case/attempt record and behavior/function/World views | Case/attempt schema retains categories, subjects, effective World requests, declared/activated doubles, source callback outcomes/monotonic durations, failures, attachments, wait times and crash state. Views derive from the same records; associated process durations are not function timings. World callback success is not a resource-release claim. Text/JSON/JUnit share attempt data. |
| 17: required exact simple-editor fixture | `terminal-simple-editor/tests/editor.test.op` and `editor_examples.types.op` implement the exact five specified cases without editing production sources; `terminal.test.op` adds real production-binary save/quit. All six pass. |
| 17.3: explicit temporary-directory contracts | Test-only standard interface and `opal_testing_workspace`; unique creation, child filenames, ordinary filesystem removal/errors, declared cleanup after abort. A regression proves omitted cleanup leaves the directory alone. |
| 18: typed integration and artifact separation/cache identity | Existing AST/checker/LLVM/ordinary ownership abstractions; separate native outcome/descriptor/worker protocol. Cache hashes original bytes/paths, cases, compiler release/running image, actual host/profile, ABI/features/interceptions and linker/toolchain inputs. Integrity/foreign/tampered/incomplete/concurrent publication tests pass; production products cannot substitute for test workers. |
| 19/Phase 7: usable documented feature | `docs/native-testing.md` and focused values/artifacts/reports/workspaces/terminal guides cover grammar, injection, mocking, failures, scheduling, responsibilities, platform/ABI limits and deferred functionality. Product-owner, SDET and language-developer examples link executable fixtures. README's empty-suite claim is removed. |
| 21: explicit deferrals and exclusions | No async/task scope, suite/shared mutable World, same-process reuse, runtime discovery, Gherkin engine, coverage/mutation or machine-code patching. Unsupported indirect/foreign callback mocking/abort paths diagnose explicitly. Host worker execution and fixed actual backend profile are documented rather than silently pretending to implement non-host execution. |

## Section 20 acceptance matrix

### 20.1 Parsing

All eight bullets map to `lexer/testing_tests.rs` and `parser/testing_tests.rs`:
`test_prose_preserves_apostrophes_hashes_and_escaped_colons`,
`quoted_titles_and_bare_scenario_are_contextual`,
`prose_requires_a_terminating_colon_on_the_same_line`,
`quoted_titles_cannot_consume_the_next_source_line`,
`parses_documented_scenarios_worlds_and_both_callback_layouts`,
`parses_token_bounded_assertions_without_splitting_string_contents`, and
`parses_double_configuration_and_polling_observations` (including the specified
multiline polling form). Original source spans are retained throughout.

### 20.2 Compiler checks

All thirteen bullets are exercised by subject/case/graph/resolution/checking tests
and `checker/native_testing_tests.rs`, `native_assertion_tests.rs`,
`native_double_tests.rs`, and error-inference tests. These explicitly reject bad
subjects/calls, wrong nominal/order/count/mode injections, duplicate aliases,
callback-local unused/shadowed usage, invalid example fields, cyclic graphs and
invalid mock values/errors. Positive tests establish canonical aliases,
`worlds: none`, activation-only origins and single-instance dependency diamonds.

### 20.3 Lifecycle

`native_lifecycle_obeys_failure_boundaries_and_retains_secondary_failures` compiles
and executes ten C-boundary scenarios covering every failure row and verifies exact
callback traces, state publication, entered scopes, drops and primary/secondary
identities. Real native source/CLI lifecycle, managed-history and workspace tests
add source abort/cleanup behavior. Omitted cleanup is both statically legal and
observably not synthesized.

### 20.4 Failure transport and mocking

Linked `native_guard_call_tests`/outcome/callback tests and executable double,
activation, values, replacement, managed, standard and filesystem fixtures prove
all nine bullets: guards distinguish channels; no unexpected-call fallback
application value; original forwarding/bypass; edge isolation/aliases; explicit
unsupported diagnostics; unused-rule verification; actual counts and owned
history. C double regressions also verify activation conflicts/duplicates and
failure preservation.

### 20.5 Parallel execution/reporting

All eleven bullets have authoritative scheduler and native evidence:

- `cases_in_one_scenario_overlap_in_distinct_worker_processes` uses a two-sided
  readiness barrier, not sleep-only inference.
- `real_examples_with_one_world_subject_and_double_overlap_with_independent_configurations`
  compiles one real source body and two named examples, checks distinct PIDs and
  peak occupancy two, and synchronizes **after** each installs a different rule
  on the same canonical edge. Both return their own configured value. The same
  scenario, World and subject do not serialize them.
- Exclusion tests compare shared/distinct keys; retries retain failures and new
  process IDs; example reorder and multi-subject tests preserve identities/totals.
- Static and real CLI setup failures remain setup failures, not subject assertions.
- Crash/timeout tests retain phase and no terminal completion; the real interrupted
  callback regression preserves completed setup and an attachment, marks body
  interrupted and does not invent cleanup or a completed duration.

### 20.6 Editor

The exact five required cases prove Unicode join `(1,0) -> (0,4)`, down-clamp to
`(1,1)`, real save without close, failed `w` preservation and failed `wq` no-quit.
The sixth case launches the normal binary through a real PTY, observes normal and
insert modes, sends input once, escapes, enters `wq`, observes exit/code, and reads
exact `Reviewed café\n!` contents. No final post-quit frame is assumed. Tests/real
CLI validate transcript attachments and production ABI separation.

## Section 22 definition of done

1. Proposed syntax reaches structured AST/checker/LLVM and executable fixtures.
2. Nominal Worlds, subscriptions, dependencies and callback-local usage are checked.
3. Stable named examples share one compiled body and independent descriptors/workers.
4. Typed assertions/doubles are executable; normal source/binary behavior is separate.
5. Harness abort and ordinary application errors have independent checked channels.
6. Explicit callback boundaries implement all normal failure-continuation rules.
7. Default capacity scheduling has no file/scenario/World/subject locks.
8. Resolved function/World views contain honest observed metrics, not coverage claims.
9. Exact editor unit/integration/mocked and production-PTY cases all pass.
10. No excluded resource-analysis/recovery, ownership protocol or async subsystem was
    introduced; ordinary memory drops and explicit library operations remain distinct.

## Overall maintainability/safety review

Reviewed original-source identity preservation, canonical alias/layout consistency,
abort-before-payload reads, source/standard import-edge ordering, original-call
bypasses, managed argument/history/attachment copying, lexical cleanup, successful
state publication, deterministic lifecycle order, transport integrity and incomplete
worker handling, scheduler key admission/retry freshness, cache atomic publication,
normal application ABI separation, PTY explicit cleanup and report deduplication.

The real barrier audit found and fixed one C/source boolean physical-layout mismatch
in `f28eed5`; the same regression now passes without altering application ABIs.
Unsupported foreign/indirect ABI paths and unsupported snapshot layouts remain
explicit v1 diagnostics, not silent fallback behavior. Full configured cross-target
execution/profile selection and generalized inline-field borrowing are not added
language features: cache identity records the actual backend settings, and existing
ordinary reference rules remain intact. No unrelated analyzer gate was weakened.
