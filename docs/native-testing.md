# Testing Opalescent programs

Write tests in Opalescent and run them with `opal test`. The runner discovers
`tests/**/*.test.op` under the project root, checks the source, and compiles a
native worker. Every case and retry runs in a fresh process. This is separate
from `cargo test`, which tests the Rust compiler and its tooling. Production
binaries are not instrumented.

## Write and run your first test

Put this in `tests/addition.test.op`, next to your project's `src/` directory.
Your test does not need an `entry main` or an import for `expect`.

```opal
scenario adding integers:
    it should sum two values:
        f(): void =>
            expect 2 + 3 to be 5
            return void
```

Run `opal test` from the project root, or pass `opal test --root path/to/project`
from elsewhere. Try changing `5` to `6` to see the assertion fail. For a test
that imports real application code, run the repository's [addition fixture][addition]:

```sh
opal test --root test-projects/native-testing-basic --list
opal test --root test-projects/native-testing-basic
```

## Scenarios, tests, and metadata

- `scenario title:` groups tests and can contain nested scenarios. A bare
  `scenario:` uses the sole resolved subject's name; otherwise supply a title.
- `it should title:` defines a test with one `f(...): void =>` callback. Put the
  signature on the same line as the title or on an indented line below it.
- Titles are prose, not executable instructions. Apostrophes and `#` can appear
  in a title. Use a quoted, non-interpolated title when it contains a colon, as in
  `it should 'accept a label: value':`. Put `##` documentation blocks immediately
  before the declaration they describe.

Scenarios pass metadata down to descendants. A nested scenario or test can
replace its inherited subjects, kind, World list, or activation list. Exclusion
keys are additive instead.

| Clause | Meaning |
|---|---|
| `unit under test: add` / `units under test: add, subtract` | Associate resolved functions with the test for selection and reporting. Use function references, not calls. This neither calls the functions nor measures coverage. |
| `kind: unit` / `kind: integration` / `kind: end_to_end` | Default is `unit`. An `end_to_end` selection also builds the normal application; see [terminal tests](native-testing-terminal.md). |
| `worlds: First, Second` / `worlds: none` | Set or clear the ordered World parameters injected into the test body. |
| `activate worlds: First` / `activate worlds: none` | Set or clear lifecycle-only Worlds; they are not body parameters. The effective body and activation lists must not overlap. |
| `requires exclusive access to: 'device', 'account'` | Acquire the whole set of constant keys before starting an attempt. Keys coordinate only within this runner invocation, not between invocations or machines. |

## Worlds and hooks

A World is a named fixture with state initialized for each attempt. Use
`world Name:` to declare it. `setup:` is required and returns one state value;
`cleanup:` is optional and takes that state by `ref` or `mutable ref`, not the
World handle. In a test body, take the declared Worlds in exactly the order in
`worlds:`. Use `ref` to read `.state` or `mutable ref` to change it. A body must
use every World it receives; use `activate worlds:` if you need only lifecycle.
Worlds with identical state types still have distinct nominal types. You cannot
construct, copy, or share a World handle between tests.

```opal
world Counter:
    setup: f(): int64 =>
        return 0

    cleanup: f(ref count: int64): void =>
        expect count to be 2
        return void

scenario counting:
    worlds: Counter

    before each test:
        f(mutable ref counter: Counter): void =>
            counter.state = counter.state + 1
            return void

    it should continue from the prepared state:
        f(mutable ref counter: Counter): void =>
            expect counter.state to be 1
            counter.state = counter.state + 1
            return void
```

Inside a World declaration, `worlds: Dependency, Other` declares dependencies,
not body subscriptions. Setup takes references to those Worlds in order. Cleanup
may take an ordered subset of those references after the state parameter.
Dependencies must be acyclic. A shared dependency is initialized once per
attempt; its dependents or the test body do not implicitly receive references to
its own dependencies. See the [diamond fixture](../test-projects/native-testing-lifecycle/tests/diamond.test.op).

`before each test:` and `after each test:` can appear at file or scenario scope.
Hooks declare their own World needs through their reference parameters, independent
of `worlds:` on a scenario or test. Clearing body subscriptions does not clear
hook requirements, and use in a hook does not count as use in the body.

The runner sets up Worlds dependency-first, runs before-hooks outer-to-inner
in source order, then runs the body. It runs after-hooks from entered scopes
inner-to-outer in reverse order, then cleans up successfully initialized Worlds
in reverse initialization order. Failed setup does not publish state or call its
cleanup; earlier successful Worlds still clean up. Failed before-hooks skip the
body but retain entered-scope after-hooks. Assertion failures and later cleanup
failures do not suppress the remaining exit callbacks. The first failure remains
primary and later failures are retained as secondary diagnostics.

Use `import world Name from ./fixtures` or `import double Name from ./doubles`
(with optional `as Alias`) to bring declarations into another test module. Keep
shared test support under `tests/` and use `.test.op` for files to be discovered
as test roots. See [temporary workspaces](native-testing-workspaces.md) for an
example of explicit resource cleanup. A crash or hard timeout can interrupt
callbacks; there is no automatic cleanup, leak detection, or orphan recovery.

## Named examples

Use `examples of RecordType:` inside an `it should` block to run one body against
multiple named records. Define the product type in a `.types.op` file and import
it with `import type`. Each `example title:` supplies fields checked against that
type. Each row becomes a separately selectable case; reordering rows does not
change their identities. Data is constructed afresh per attempt.

This excerpt from the [addition fixture][addition] assumes `add` and
[`AdditionExample`](../test-projects/native-testing-basic/tests/addition_examples.types.op)
are imported. A body receives Worlds first, then the example as a read-only
`ref`, then an optional test context.

```opal
scenario adding named inputs:
    it should handle a negative second input:
        examples of AdditionExample:
            example a negative second input:
                left: 2
                right: -3
                expected: -1

        f(ref example: AdditionExample): void =>
            expect add(example.left, example.right) to be example.expected
            return void
```

## Assertion keywords

Start with `expect actual` and use one of these endings. The compiler checks
operand types; comparisons are typed, not based on printed text or pointers.

| Ending | Meaning |
|---|---|
| `to be expected` / `not to be expected` | Equality / inequality for [supported values](native-testing-values.md). |
| `to be greater than n`, `to be less than n`, `to be at least n`, `to be at most n` | Numeric ordering. |
| `to be empty` / `not to be empty` | Empty or nonempty string or array. |
| `to contain text` / `not to contain text` | String substring check. |
| `to start with text` | String prefix check. |
| `to end with text` / `not to end with text` | String suffix check. |
| `to contain exactly items in order` | Array contents in exact order, including multiplicity. |
| `to be the Variant variant of Type` | Check the nominal variant without comparing its payload. |

`expect eventually observation to ... within duration seconds` samples an
observation synchronously against a monotonic deadline. The deadline and expected
operands are captured once; only mismatches retry. Application errors escape
immediately. Keep actions such as sending input outside the polling assertion so
only the observation is repeated. Diagnostics use the last captured observation
without re-running it.

Use ordinary `guard` or `propagate` for application errors. Managed callbacks can
use `errors inferred` to declare the application errors that escape them. An
assertion failure aborts through the harness, not as an application error that a
`guard` can catch.

Test bodies and hooks can request a final `mutable ref context: TestContext`
parameter. The runner supplies `context.failed`, `context.case_id`, and
`context.attempt`. Import `attach_text` from `standard.testing` and call
`attach_text(mutable ref context, name, text)` to keep diagnostic text in the
[report](native-testing-reports.md).

## Typed doubles

A `double ... replaces ... from ... as used by ...` declaration names a **single
import edge**, not every call to a function. Same-module calls, other importers,
and calls made directly through the provider from the test are unaffected. Inactive
edges run the real function. Run this example with the [double fixture's source files](../test-projects/native-testing-doubles/src/):

```opal
import apply from ../src/app

double Write replaces operation from ../src/io
    as used by ../src/app

scenario controlling one imported boundary:
    it should use a typed exact rule:
        f(): void errors inferred =>
            let writer = new Write
            stub writer(1) returns 9
            expect propagate apply(1) to be 9
            expect writer to have been called once
            expect writer to have been called with (1)
            verify writer
            restore writer
            return void
```

| Form | Effect |
|---|---|
| `new Write` | Construct an attempt-local handle for the declared double. |
| `stub writer(args) returns value` | Configure a repeatable exact argument rule with a typed result. |
| `stub writer(args) fails with ErrorType.Variant` | Configure a declared application error; an application `guard` can catch it. |
| `stub writer using replacement` | Use a checked named native function as fallback. |
| `spy writer using original` | Forward unmatched calls to the real function and track calls. |
| `stub writer:` with indented `otherwise fail the test` | Explicit fail-on-unmatched fallback. |
| `verify writer` | Check that all exact rules have been exercised. |
| `restore writer` | Disable interception on the edge. |

Exact rules take precedence over fallbacks. Duplicate or overlapping exact rules
are rejected. An active unmatched call without a forwarding fallback aborts the
attempt, even through a non-fallible application function. Use these endings with
`expect writer`: `to have been called once`, `to have been called exactly n times`,
`to have never been called`, or `to have been called with (args)`. The last form
checks whether any recorded call had those arguments. Some values support only
counting, not exact rules or argument history; see [value contracts](native-testing-values.md).
Unsupported indirect or foreign interception paths are not general-purpose mocks.

## CLI, scheduling, and reports

| Option | Behavior |
|---|---|
| `--root <directory>` | Project root; defaults to the working directory. |
| `--list` | Check sources and list selected cases without native builds or callback execution. |
| `--case <id>` | Match the exact case ID printed by `--list`; quote it in your shell. |
| `--filter <text>` | Match a case-sensitive substring of the displayed behavior. |
| `--kind <category>` | Select `unit`, `integration`, or `end_to_end`. |
| `--unit <function>` / `--world <world>` | Select by resolved function association or World requirement (body, hook, activation, or dependency). |
| `--allow-empty` | Succeed when no cases match; otherwise empty selection fails. |
| `--jobs <count>` | Limit concurrency to a positive worker count; defaults to available parallelism. |
| `--timeout <duration>` | Per-attempt hard deadline, default `30s`; accepts seconds or `ms`, `s`, `m` suffixes. |
| `--retries <count>` | Additional fresh attempts after failure; default zero. |
| `--reporter <format>` | `text` (default), `json`, or `junit`. |
| `--output <path>` | Write the report to a file instead of standard output. |
| `--target <triple>` | Explicit native build target; execution requires the host target. |

Example for CI: `opal test --kind integration --reporter junit --output report.xml`.
Cases in the same file, scenario, or World are eligible to run concurrently.
Exclusion keys remain held until the attempt's worker terminates. Retries retain
all failed attempts; a later pass is reported as flaky. [Reports](native-testing-reports.md)
keep one record per case, with function and World views and the last observed
callback events. These views do not measure coverage or count extra tests.

## Standard-library helpers and limits

The test-only `standard.testing` module provides [temporary workspaces](native-testing-workspaces.md),
[real terminal-process tests](native-testing-terminal.md), and
[diagnostic attachments](native-testing-reports.md). Imports from production
modules are rejected. See [native test artifacts](native-testing-artifacts.md) for
worker caching and target limits. For a larger example, see the
[editor suite](../test-projects/terminal-simple-editor/tests/).

There are no before/after-all hooks, shared mutable Worlds, same-process worker
reuse, snapshot/property-testing DSLs, coverage collection, or natural-language
execution. World cleanup is explicit; the ordinary compiler's lexical
value-memory cleanup still applies.

[addition]: ../test-projects/native-testing-basic/tests/addition.test.op
