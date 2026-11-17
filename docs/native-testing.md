# Compiled native tests

`opal test --root <project>` discovers `tests/**/*.test.op`, checks original source
and compiles a reusable LLVM-native worker. Every case/attempt gets a **fresh
process**. Tests are not interpreted and production binaries are not instrumented.

## Product owners: readable behavior

```opal
import add from ../src/arithmetic

scenario adding prices:
    unit under test: add
    kind: unit
    it should add both prices:
        f(): void =>
            expect add(2, 3) to be 5
            return void
```

Titles are bounded prose, including apostrophes and `#`. Quote a title when it
needs a colon. Documentation blocks immediately before declarations are retained.
Inline callbacks and multiline signatures are supported. Indentation, not braces,
delimits bodies. Ordinary application sources keep their ordinary grammar.

`unit under test`/`units under test` name resolved functions, not call expressions.
Associations are documentation/reporting metadata, not coverage or implicit locks.
Categories are `unit`, `integration`, and `end_to_end`.

## SDETs: explicit Worlds and independent examples

A World declares `setup`, optional `cleanup`, and optional ordered dependency
`worlds`. Setup returns one state value. Cleanup takes a mutable reference to that
state, then any ordered subset of required dependency World references. Tests
subscribe with `worlds: Name, Other` and receive those nominal World references in
exactly that order. Access state through `.state`; mutate it only through a mutable
World reference. Same-state-type Worlds remain different nominal types.

World lists replace inherited body subscriptions; `worlds: none` removes them.
Hook requirements remain separate. `activate worlds: Name` requests lifecycle
without an unused body parameter. Unused body injections are errors; a hook's use
does not satisfy the body. Worlds cannot be constructed, copied as ordinary values,
or shared between cases. Dependencies must be explicit and acyclic; diamonds
initialize once per attempt. Bodies do not implicitly receive transitive Worlds.

`before each test` hooks run outer-to-inner in source order. Entered scopes run
`after each test` hooks inner-to-outer in reverse order. Setup is deterministic;
declared cleanup runs in reverse initialization order. Failed setup is never
published or passed to cleanup. Earlier successful Worlds still clean up. Failed
before-hooks skip the body but retain entered-scope after-hooks. Assertions and
later after/cleanup failures do not suppress remaining declared exit callbacks.
Primary failures remain primary; secondary diagnostics are retained.

Omitted cleanup is legal. No resource registry, leak detector, cleanup proof,
rollback, orphan recovery, new ownership protocol, or implicit serialization exists.
The ordinary compiler's lexical value-memory cleanup remains in force.

Examples use `examples of RecordType`, named `example title:` records, and a final
readonly example parameter after World parameters. Each title produces an
independently selectable identity; reordering examples does not change it. Example
values are checked against the record schema and constructed afresh per attempt.
See the exact working [editor conformance fixture](../test-projects/terminal-simple-editor/tests/editor.test.op).

## Language developers: fixed assertions and import-edge doubles

Supported phrases include equality/inequality, numeric `greater than`, `less than`,
`at least`/`at most`, emptiness, string containment/prefix/suffix, exact array order,
and `the Variant variant of Type`. Matching is typed value comparison, not rendered
text or pointer identity. Whole payload-bearing variant equality is rejected;
variant identity matching does not read the payload. Unsupported matcher/type
combinations fail checking.

`expect eventually observation to ... within seconds seconds` captures expected
values/deadline once, samples observations synchronously with a monotonic deadline,
and retries only mismatches. Application errors escape immediately. Inputs belong
outside polling assertions. Final diagnostics own the last observed values and
never repeat application operations.

```opal
double Write replaces operation from ../src/io
    as used by ../src/app

scenario controlling one imported boundary:
    it should use a typed exact rule:
        f(): void =>
            let writer = new Write
            stub writer(1) returns 9
            expect apply(1) to be 9
            expect writer to have been called once
            expect writer to have been called with (1)
            verify writer
            restore writer
            return void
```

Import `apply` from the importing module for the example above. The boundary is the
resolved source import edge, not all functions with that spelling. Same-module
calls, other importing edges, and explicit original bypasses are unaffected.
Interception precedes specialization/inlining; optimization is not globally
disabled. Inactive edges forward original functions. Active unconfigured calls
abort the test even through non-fallible functions. An application `guard` cannot
catch a harness abort, but can handle a deliberately stubbed application error.

Exact rules take precedence over fallbacks, preserve source argument evaluation
order, are repeatable, and must be exercised for `verify` to succeed. Duplicate or
overlapping exact rules fail. `stub writer using replacement` selects a checked
named native replacement; `spy writer using original` forwards while counting and
recording supported logical snapshots. `otherwise fail the test` is explicit.
`restore` disables the edge. Counts/history/activation are attempt-local; borrowed
addresses never escape into history. Unsupported snapshots retain count-only
spying. See [value support](native-testing-values.md) for the explicit policy.

Callbacks support `errors inferred` for escaping application errors. A final
`mutable ref context: TestContext` injection exposes `failed`, `case_id`, `attempt`
and the [owned attachment API](native-testing-reports.md).

## Scheduling, CLI and reports

```text
opal test --root project --list
opal test --root project --filter "saving" --jobs 4
opal test --root project --kind integration --timeout 10s --retries 1
opal test --root project --reporter json --output report.json
opal test --root project --reporter junit --output report.xml
```

`--case` takes the exact structured identity printed by listing. `--unit` and
`--world` select resolved associations. Empty selection fails unless `--allow-empty`.
Listing checks sources without compilation/setup execution. Default concurrency
uses available parallelism, not file/scenario/World/function grouping. Retries use
fresh workers and retain every failed attempt; a later pass is flaky, not erased.

`requires exclusive access to: 'external-service', 'other-key'` requests the whole
constant key set before starting an attempt. Keys are invocation-local scheduler
admission, not distributed locks or resource recovery. They remain held through
worker termination. Authors remain responsible for external services/files.

Hard timeout/crash cannot promise cleanup execution. Last flushed phase/callbacks,
attachments and double transitions are retained; unobserved callback completions
are not reported as successful. Compiler/infrastructure errors are distinct from
assertion/application/lifecycle failures. [Reports](native-testing-reports.md) keep
one case record with behavior, function and World views. Coverage is not collected.

## Support, artifacts and platform limits

- [Temporary workspaces](native-testing-workspaces.md): explicit create/file/remove.
- [Production PTY driver](native-testing-terminal.md): separate normal application,
  real terminal input/screen/exit/transcript and explicit stop/wait cleanup.
- [Artifact cache](native-testing-artifacts.md): instrumented namespace, full keys,
  verified immutable publication, actual host target/fixed native profile.

Native worker execution currently requires the host target. POSIX PTY support is
implemented; unsupported platforms return explicit launch errors. Unsupported
indirect/foreign callback mocking is diagnosed, not silently forwarded.

Deferred: all leak protection, async/awaited cleanup/task cancellation, shared or
suite-scoped mutable Worlds, before/after-all hooks, same-process batching/reuse,
runtime example discovery, natural-language/Gherkin interpretation, mandatory
BDD steps, coverage, mutation testing and arbitrary machine-code patching.
