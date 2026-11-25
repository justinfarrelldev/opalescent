# Opalescent Native Testing Framework

## Language Specification and Implementation Plan

Status: original design and acceptance specification, retained for historical reference.
The requirements below describe the intended design, not the current API or a
claim of conformance. For supported syntax and runnable examples, see the
[Opalescent testing guide](../docs/native-testing.md). For implementation evidence,
see the [conformance audit](native-testing-framework-acceptance-audit.md).

This specification defines a compiled testing framework for Opalescent with:

- English-like test descriptions and assertions.
- Ordinary typed Opalescent functions as executable test bodies.
- Named, typed Worlds with explicit setup and optional cleanup.
- Compiler-validated references to functions under test.
- Independently reported, typed examples.
- Parallel execution by default.
- One framework for unit, integration, and end-to-end testing.
- Typed stubs, mocks, and pass-through spies suitable for a compiled language.

The design takes inspiration from Cucumber, Playwright, and Vitest, while
preserving Opalescent's syntax, compilation model, and emphasis on readability.

Normative terms:

- MUST identifies a required behavior.
- SHOULD identifies a strong implementation recommendation.
- MAY identifies an optional extension.

All syntax introduced here is proposed functionality. Examples are intended
to become executable conformance fixtures after implementation.

Repository paths and application APIs referenced below come from the preceding
design discussion. Verify them against the implementation checkout before
editing. This specification does not claim a fresh audit of the current
repository revision.

---

# 1. Goals and Scope

## 1.1 Readability

A product owner or SDET unfamiliar with Opalescent should be able to understand:

1. What behavior a scenario concerns.
2. What each test expects.
3. Which environments the test uses.
4. Which application function or functions the test targets.
5. Which application boundaries are replaced.
6. Why the test exists.

The framework MUST support paragraph-length documentation.

Descriptions are human-readable text. They are not instructions interpreted
by an AI or a natural-language execution engine.

Executable behavior MUST use a finite grammar and statically checked
Opalescent code.

## 1.2 Compilation

Tests MUST compile before execution.

The compiler MUST validate:

- Referenced functions under test.
- World declarations and dependency graphs.
- Ordered World injection into test functions.
- World parameter usage.
- Example schemas.
- Assertion operand types.
- Mock signatures, return values, and application error variants.
- Supported interception boundaries.

Production builds MUST remain separate from instrumented test builds.

## 1.3 Unified testing

The framework MUST support:

- Unit tests calling individual application functions.
- Integration tests composing real modules with selected boundaries replaced.
- Integration tests using real filesystem or process operations.
- End-to-end tests launching the normal compiled application.

The same declarations, assertion vocabulary, lifecycle model, runner, and
report formats MUST work across these categories.

A test's category describes its intended boundary. It is not proof that the
test provides a particular level of coverage or isolation.

## 1.4 Parallel execution

Every individual test case MUST be eligible for concurrent scheduling by
default, including:

- Tests in the same source file.
- Tests in the same scenario.
- Different examples of one parameterized test.

Grouping tests by scenario, function, or World MUST NOT serialize them.

Concurrency MUST be bounded and configurable.

## 1.5 Explicit exclusion: no leak-protection implementation

This project MUST NOT introduce:

- Resource-leak detection.
- Per-test resource ledgers.
- Compiler proofs that resources were released.
- New must-close or must-release obligations.
- New destructor or resource-owner protocols.
- Automatic registration of acquired external resources.
- Inferred rollback of partially completed setup.
- Synthesized cleanup for user-defined World state.
- Automatic deletion of resources left behind by failed tests.
- Orphan-resource scanners.
- Heap accounting or sanitizer integration as part of this feature.
- New async lifetime, cancellation, or cleanup machinery.

The runner WILL invoke explicitly declared cleanup callbacks during the normal
test lifecycle.

It WILL NOT determine whether cleanup was necessary, whether a callback
released everything, or whether external resources remain afterward.

Missing cleanup is legal. A cleanup callback that returns successfully is
reported as a successful callback, not proof that resources were released.

Existing Opalescent memory management and reference rules continue to apply
without being extended into a new leak-protection system.

---

# 2. Core Concepts

| Concept | Meaning |
| --- | --- |
| Scenario | A named group of independent behavioral tests and shared declarations |
| Test | One `it should ...` declaration with one typed callback |
| Example | A named, typed data record that creates an independent case |
| Case | The independently schedulable identity of a test or one of its examples |
| Attempt | One execution of a case, including setup, hooks, body, and cleanup |
| World declaration | Reusable environment definition with setup and optional cleanup |
| World instance | State initialized for one attempt |
| World handle | A generated nominal type exposing that instance through `.state` |
| Unit under test | A compiler-resolved function reference used for reporting |
| Double | A typed, precompiled interception boundary with attempt-local behavior |

A scenario is a grouping construct. It does not imply execution order or
shared mutable state.

A World definition is reusable. Its instances are not shared between test
attempts.

---

# 3. Canonical Surface Syntax

## 3.1 Basic test

```opal
import add from ../src/arithmetic

##
    Description:
        Addition combines both inputs, including when one input is negative.
##
scenario adding integers:
    unit under test: add
    kind: unit

    it should return 4 when 2 and 2 are passed: f(): void =>
        let result = add(2, 2)

        expect result to be 4

        return void

    it should return -1 when 2 and -3 are passed:
        f(): void =>
            let result = add(2, -3)

            expect result to be -1

            return void
```

Both callback layouts above MUST be accepted.

The canonical function arrow is `=>`.

Adding `⇒` as an alternative is outside this feature.

## 3.2 World declaration

```opal
world Workspace:
    setup: f(): WorkspaceState errors inferred =>
        return propagate create_workspace()

    cleanup:
        f(mutable ref state: WorkspaceState): void errors inferred =>
            propagate remove_workspace(mutable ref state)
            return void
```

Use exactly:

- `world`
- `worlds:`
- `setup:`
- `cleanup:`

Do not introduce `universe`, `set up universe`, or `tear down universe`
as aliases.

## 3.3 Test subscriptions

```opal
scenario saving documents:
    unit under test: save_lines
    kind: integration
    worlds: EditedDocument, Workspace

    it should persist the edited document:
        f(
            ref document: EditedDocument,
            ref workspace: Workspace
        ): void errors inferred =>
            # Ordinary Opalescent code.

            return void
```

The order of `worlds:` determines the order of the initial test parameters.

## 3.4 Hooks

```opal
before each test:
    f(): void =>
        # Technical setup.
        return void

after each test:
    f(): void =>
        # Verification or diagnostics.
        return void
```

Hooks MAY receive typed World references.

Hooks do not make their local variables visible to other functions.

## 3.5 Test declarations with metadata

A test MAY use an indented declaration body containing metadata followed by
exactly one callback:

```opal
it should preserve the document after an unsuccessful write:
    worlds: EditedDocument, ControlledWrites

    f(
        ref document: EditedDocument,
        mutable ref writes: ControlledWrites
    ): void errors inferred =>
        # Test implementation.

        return void
```

This form also accommodates parameterized examples.

Do not require a separate `run:` or `run =` wrapper.

---

# 4. Source Files, Imports, and Documentation

## 4.1 Test roots

The initial discovery convention is:

```text
tests/**/*.test.op
```

Project configuration MAY override the test root directories.

Discovery MUST identify source declarations without executing application
functions, World setup, or test callbacks.

Compiler errors prevent execution of the selected test artifact.

## 4.2 Test-support modules

Worlds and doubles MAY be declared in the test file itself or imported from
test-support modules.

Introduce explicit imports:

```opal
import world Workspace, EditedDocument from ./worlds
import double AtomicWrite from ./doubles
```

These imports make the declaration identity and its generated handle type
available together.

World and double declarations are test-only. Production builds MUST reject
imports that introduce them.

The implementation MUST maintain an explicit test-module mode. Conversational
phrases MUST NOT become globally reserved words in ordinary application code.

A support module reached through a World or double import is a test-only
module. Its ordinary application dependencies continue to use the ordinary
language grammar.

## 4.3 Type files

Existing `.types.op` restrictions MUST remain intact.

User-defined example records and World state records belong in normal type
files:

```opal
public type FailedSaveExample:
    command_text: string
```

World declarations belong in executable test or test-support modules because
they contain functions.

A generated World handle type is not a user-defined record declaration and
does not require duplicating the World in a `.types.op` file.

## 4.4 Documentation

Existing documentation blocks immediately preceding scenarios, tests, Worlds,
and examples MUST be retained in test metadata.

Reports SHOULD expose this documentation.

Ordinary comments remain ordinary comments.

---

# 5. Parsing and Lexical Rules

## 5.1 Prose headers

Support unquoted prose in:

```opal
scenario preserving unsaved work:
it should preserve the user's changes:
example write without quitting:
```

The frontend MUST recognize these headers before applying ordinary string
and comment tokenization to the prose portion.

Within an unquoted title:

- Apostrophes are literal text.
- `#` is literal text.
- The first unescaped colon terminates the title.
- `\:` represents a literal colon.
- A newline before the terminating colon is an error.

After the colon, ordinary Opalescent tokenization resumes.

Quoted titles MUST also be supported:

```opal
it should 'accept the :wq command':
    f(): void =>
        return void
```

Quoted titles use normal string-literal escaping but MUST be static text.
Do not introduce runtime interpolation into declaration identities.

Use source spans from the original file. Do not implement this feature as
regex-based source rewriting.

## 5.2 Scenario names

A scenario SHOULD have an explicit title.

A bare `scenario:` MAY derive its display name from a single declared unit
under test.

If no unambiguous title can be derived, require an explicit name.

Sibling declarations MUST have distinct effective names. Do not silently
append ordinal suffixes.

## 5.3 Grammar outline

```text
scenario:
    "scenario" optional-title ":" scenario-body

test:
    "it should" title ":" callback
    or
    "it should" title ":" indented-test-body

world:
    "world" identifier ":" world-body

world-body:
    optional-world-dependencies
    exactly-one-setup
    optional-cleanup

hook:
    ("before each test" | "after each test") ":" callback

callback:
    ordinary-function-literal
    or
    indented-ordinary-function-literal

test-body:
    zero-or-more-test-metadata-declarations
    optional-examples
    exactly-one-callback

examples:
    "examples of" type-reference ":" example-declarations

example:
    "example" title ":" typed-field-initializers

subject:
    "unit under test:" function-reference
    or
    "units under test:" function-reference-list

subscriptions:
    "worlds:" world-reference-list
    or
    "worlds: none"

expectation:
    "expect" expression supported-matcher
```

The final grammar MUST be represented in the parser and AST.

Matcher parsing MUST operate on tokens and expression boundaries. It MUST NOT
split raw text at arbitrary occurrences of phrases such as `to be`.

---

# 6. Function and Test Metadata

## 6.1 `unit under test`

```opal
unit under test: add
```

The compiler MUST resolve `add` to a genuine callable declaration.

Reject:

```opal
unit under test: missing_function
unit under test: 42
unit under test: add(2, 2)
```

The final example is a call, not a reference.

Support qualified references and import aliases through the existing resolver.

The metadata MUST identify the canonical declaration, not:

- A display string alone.
- An import alias alone.
- A runtime function address.

Ambiguous references MUST produce a compiler error.

## 6.2 Multiple subjects

```opal
units under test: apply_named_key, execute_command, save_lines
```

A case may be associated with several functions.

Overall test totals MUST deduplicate case identities.

A declaration MUST NOT contain both singular and plural subject clauses.

Test-level subject metadata replaces inherited subject metadata.

## 6.3 Meaning of association

Subject metadata:

- Does not execute the function.
- Does not activate a double.
- Does not alter scheduling.
- Does not prove that the function was exercised.
- Does not prove that assertions meaningfully verify it.
- Does not make a failing test proof of a defect in that function.

Function-execution coverage is a separate feature and is not part of this
implementation.

Reports MUST show coverage as unavailable or not collected when appropriate.

## 6.4 Test categories

Supported categories:

```opal
kind: unit
kind: integration
kind: end_to_end
```

The category is inherited from the scenario and may be overridden by a test.

The initial default is `unit`.

Integration and end-to-end tests SHOULD declare their categories explicitly.

Subject metadata is optional, particularly for black-box end-to-end tests.

---

# 7. World Type and Subscription Semantics

## 7.1 Generated nominal handles

This declaration:

```opal
world Workspace:
    setup: f(): WorkspaceState =>
        return create_workspace_state()
```

introduces:

1. A World provider identity.
2. A generated nominal handle type named `Workspace`.
3. An associated state type, `WorkspaceState`.
4. A `.state` accessor.

Therefore:

```text
workspace                 Workspace
workspace.state           WorkspaceState
```

`Workspace` MUST NOT be a type alias for `WorkspaceState`.

Two Worlds returning the same state type remain different parameter types.

The runner constructs World handles. User code does not directly construct
or substitute them.

World parameters MUST be passed using `ref` or `mutable ref`, under existing
Opalescent reference rules.

The mutability of `.state` follows the handle's reference mode.

This feature does not introduce a new ownership or lifetime-checking system.

## 7.2 Setup contract

Each World MUST have exactly one `setup` callback.

Setup returns one concrete state value.

V1 does not support labeled multiple returns as World state. Use an ordinary
record when several values are needed.

A World with no dependencies uses:

```opal
setup: f(): StateType =>
    return ...
```

Setup may return declared application errors.

## 7.3 Cleanup contract

Cleanup is optional.

If present, it MUST:

- Return `void`.
- Receive the initialized state as its first parameter.
- Use `ref` or `mutable ref` according to existing language rules.
- Optionally receive declared dependency Worlds after the state parameter.
- Declare or infer any ordinary application errors.

```opal
cleanup:
    f(mutable ref state: StateType): void errors inferred =>
        propagate cleanup_state(mutable ref state)
        return void
```

The compiler checks the callback's type contract.

It does not verify that the callback releases resources.

## 7.4 Ordered test injection

For:

```opal
worlds: Document, Workspace
```

the initial test parameters MUST be:

```opal
f(
    ref document: Document,
    ref workspace: Workspace
): void =>
```

Parameter names are arbitrary.

World identities and order are exact.

Reject:

- Missing World parameters.
- Extra World parameters.
- Reordered World parameters.
- Duplicate subscriptions, including aliases of the same declaration.
- Unresolved World names.
- Parameters using the state type where the World handle type is required.

## 7.5 Inheritance and narrowing

A test inherits its containing scenario's effective subscription list.

A test-level `worlds:` clause REPLACES that list.

It does not append implicitly.

```opal
worlds: none
```

explicitly removes inherited body injections.

Nested scenarios follow the same replacement rule.

## 7.6 Strict unused-World checking

Every injected World parameter MUST be used in the callback receiving it.

Uses include:

- Reading `.state`.
- Updating state through a permitted mutable reference.
- Passing the handle or state to a helper.
- Configuring a double.
- Inspecting call history.

These do not count:

- Mentioning the type in a signature.
- Automatic setup or cleanup.
- Another callback using the same instance.

Run this check on typed source before optimization.

Do not require every state field to be used.

Do not attempt to prove semantic relevance. A source-level usage check cannot
prove that an assertion is meaningful.

Apply this rule to injected World parameters in tests, hooks, and dependency
setup callbacks.

The raw state parameter of `cleanup` is not a World subscription. This rule
MUST NOT become a cleanup-obligation check.

## 7.7 Effect-only activation

Support an explicit clause for environments needed for their activation:

```opal
worlds: EditedDocument
activate worlds: NetworkForbidden
```

Activated Worlds participate in setup and cleanup but do not add test-body
parameters.

This avoids artificial reads solely to satisfy unused-parameter checking.

Reports MUST distinguish activation-only requirements from injected Worlds.

Reject the same World appearing in both effective lists.

Test-level activation metadata replaces inherited activation metadata.
Support `activate worlds: none`.

---

# 8. World Dependencies

A World may declare dependencies:

```opal
world RunningEditor:
    worlds: Workspace

    setup:
        f(ref workspace: Workspace): EditorSession errors inferred =>
            return propagate start_editor(ref workspace.state)

    cleanup:
        f(
            mutable ref session: EditorSession,
            ref workspace: Workspace
        ): void errors inferred =>
            propagate stop_editor(
                mutable ref session,
                ref workspace.state
            )

            return void
```

Rules:

1. Setup receives its declared dependencies in order.
2. Cleanup receives its own state first, followed by an ordered subset of
   dependency Worlds it needs.
3. Dependencies are resolved statically.
4. Dependency cycles are compilation errors.
5. One World declaration creates at most one instance per attempt.
6. Diamond dependency graphs share the same dependency instance within that
   attempt.
7. Transitive dependencies are not automatically injected into the test body.
8. Subscription order is not a substitute for declaring dependencies.

Initialize Worlds in a deterministic topological order.

For independent nodes, use a deterministic source/declaration ordering.

Cleanup runs in reverse initialization order.

Dependencies remain available while their dependents' cleanup callbacks run.

World setup within an attempt is sequential in v1. Parallelism occurs between
independent cases.

---

# 9. Hooks and Lifecycle

## 9.1 Hook scopes

Support:

- File-level hooks.
- Scenario-level hooks.
- Hooks in nested scenarios.

A hook may have no parameters or request World references and an optional
test context.

Hook World dependencies are explicit through their nominal parameter types.

A hook does not need to accept every World used by the test.

Hook-only Worlds are included in lifecycle setup but are not silently added
to the test body's parameter list.

Reports MUST identify hook-origin dependencies.

## 9.2 Test context

A callback MAY request a final parameter:

```opal
mutable ref test: TestContext
```

This provides attempt metadata and diagnostic attachment APIs.

It is not an untyped fixture dictionary.

If present, it follows World parameters and example data.

Its ordinary context fields are managed by the runner. A test may inspect
failure status in an after-hook but may not erase earlier failures.

## 9.3 Attempt lifecycle

For each attempt:

1. Construct the example value, if applicable.
2. Determine the union of body, activation, hook, and transitive World needs.
3. Initialize Worlds.
4. Enter hook scopes from outermost to innermost.
5. Run before-hooks in source order within each entered scope.
6. Run the test body.
7. Run after-hooks from innermost entered scope to outermost.
8. Invoke declared World cleanup in reverse initialization order.
9. Record the complete outcome.

After-hooks within one scope run in reverse declaration order.

Each after-hook and cleanup callback is a separate runner invocation boundary.

## 9.4 Failure behavior

| Failure | Required behavior |
| --- | --- |
| Example construction fails | Fail the attempt; do not start World setup |
| World setup fails | Stop setup; skip hooks and body; clean up already initialized Worlds |
| Before-hook fails | Skip remaining setup hooks and body; run after-hooks for entered scopes |
| Test body fails | Run applicable after-hooks and declared cleanup |
| After-hook fails | Record failure and continue remaining after-hooks and cleanup |
| Cleanup fails | Record failure and continue remaining cleanup callbacks |
| Worker crashes or is forcibly terminated | Report the crash/timeout; do not claim cleanup ran |

A World is initialized only after setup returns successfully.

If setup fails, its own cleanup MUST NOT receive nonexistent state.

Partial initialization inside the failed setup remains the author's
responsibility using existing language facilities.

Do not infer or synthesize rollback.

The first failure remains the primary failure. Later failures are retained as
secondary failures.

A successful body followed by a failing cleanup produces a failed case.

## 9.5 Suite-wide lifecycle

V1 supports `before each test` and `after each test`.

Defer:

- `before all` and `after all`.
- Suite-scoped World instances.
- Shared mutable World instances.
- Persistent in-process fixture caching.

These require a separate lifetime model and must not acquire ambiguous
semantics under fresh-process execution.

Build-once application artifacts are coordinator-managed build products,
not shared mutable Worlds.

---

# 10. Named, Typed Examples

```opal
it should preserve unsaved work after a failed save:
    examples of FailedSaveExample:
        example write without quitting:
            command_text: 'w'

        example write before quitting:
            command_text: 'wq'

    f(
        ref document: EditedDocument,
        mutable ref writes: ControlledWrites,
        ref example: FailedSaveExample
    ): void errors inferred =>
        # Shared implementation using typed example data.

        return void
```

Rules:

- Example data MUST match the declared record type.
- Missing, extra, or incorrectly typed fields are compiler errors.
- V1 example initializers use compile-time-supported constant data.
- Runtime test generation is not supported.
- Example names must be unique within the test.
- Every example creates an independent case.
- Each attempt constructs fresh example values and World instances.
- Every example gets its own hooks, outcome, duration, filtering identity,
  diagnostics, and optional retry.
- A failure in one example does not prevent another from running.

Test parameters appear in this order:

1. Effective `worlds:` parameters.
2. `ref example: ExampleType`, when parameterized.
3. Optional test context.

Without examples, a test produces one case and has no example parameter.

The compiler SHOULD emit one test body plus multiple case descriptors.
It MUST NOT require a separate native build for each example.

World setup cannot implicitly access the current example through a hidden
global. Parameterized World construction is deferred until it has an
explicit contract.

---

# 11. English Assertion Syntax

## 11.1 Initial matcher set

```opal
expect actual to be expected
expect actual not to be expected

expect count to be greater than minimum
expect count to be less than maximum
expect count to be at least minimum
expect count to be at most maximum

expect text to contain fragment
expect text not to contain fragment
expect text to start with prefix
expect text to end with suffix
expect text not to end with suffix

expect value to be empty
expect value not to be empty

expect items to contain exactly expected_items in order

expect status to be the Written variant of EditorStatus

expect writer to have been called once
expect writer to have been called exactly count times
expect writer to have been called with (path, contents)
expect writer to have never been called
```

Use these phrases consistently.

Do not add numerous undocumented synonyms.

## 11.2 Semantics

Assertions MUST be typed operations.

- `to be` means supported value equality.
- It does not mean stringified equality or implicit pointer identity.
- Numeric comparisons follow the language's numeric compatibility rules.
- String operations use defined language string semantics.
- Exact ordered collection matching checks length, order, multiplicity,
  and supported element equality.
- Variant matching resolves a genuine variant of the named nominal type.
- Unsupported operand types are compilation errors.

Where test comparisons require generated helpers for arrays or records,
define their supported types explicitly. Do not silently stringify values.

Ordinary actual and expected expressions are evaluated exactly once,
in source order.

Failure reporting MUST NOT repeat the application operation.

Diagnostics SHOULD include:

- Source location.
- Matcher.
- Actual and expected types.
- Captured values where safely displayable.
- Scenario, case, example, and attempt.
- Current lifecycle phase.

## 11.3 Retrying observations

Support:

```opal
expect eventually terminal_row(ref terminal, 11)
    to contain '-- NORMAL --'
    within 2 seconds
```

This is a separate polling operation:

- The observation is reevaluated.
- The expected value and deadline are evaluated once.
- A monotonic clock controls the deadline.
- Only assertion mismatch is retried.
- Application errors from the observation fail immediately.
- The final failure includes the latest observed value.

Actions such as typing, submitting commands, or saving files MUST remain
outside the polling expression.

V1 does not include an effect system capable of proving observation purity.
Do not claim that side effects are statically prevented.

---

# 12. Typed Doubles, Stubs, and Spies

## 12.1 Declaration

```opal
double AtomicWrite replaces write_text_atomic_sync from standard
    as used by ../src/buffer
```

The compiler resolves the exact import edge:

```text
importing module → imported declaration
```

Interception is attached to that edge before optimization and special
standard-library lowering.

It is not global symbol replacement or runtime machine-code patching.

## 12.2 Activation

A double declaration introduces a generated nominal handle type.

In a test build:

```opal
let writer = new AtomicWrite
```

creates and activates a double for the current attempt.

Typical ownership is through a World:

```opal
world ControlledWrites:
    setup: f(): AtomicWrite =>
        return new AtomicWrite

    cleanup: f(mutable ref writer: AtomicWrite): void =>
        restore writer
        return void
```

A declared but inactive interception boundary calls the original operation.

An active double with no matching configuration fails the test by default.

`restore writer` deactivates that handle. It is explicit lifecycle behavior,
not inferred resource cleanup.

Reject conflicting active doubles targeting the same edge.

Detect conflicts statically when possible and at activation otherwise.

## 12.3 Configuration

```opal
stub writes.state:
    otherwise fail the test

stub writes.state(path_from('note.txt'), 'Draft')
    returns void

stub writes.state(path_from('note.txt'), 'Draft')
    fails with FilesystemFullError

stub writes.state using replacement_function

spy writes.state using original
```

The compiler MUST validate:

- Argument types.
- Reference modes.
- Result types.
- Allowed nominal application errors.
- Custom replacement-function signatures.

Exact argument expressions are evaluated when the rule is configured.

V1 exact-match rules use supported typed equality.

Duplicate or overlapping exact rules are rejected instead of silently using
registration order.

A custom replacement handler and a pass-through spy are explicit fallback
modes. Their interaction with exact-match rules MUST be deterministic:
exact rules take precedence; otherwise the selected fallback runs.

The original-call path MUST bypass interception to avoid recursive spying.

## 12.4 Verification

```opal
verify writes.state
```

means:

- Every configured non-fallback exact rule was exercised at least once.
- Fallbacks do not require execution.
- Repeated matches are allowed unless a count assertion forbids them.
- Unused configured rules fail verification.

Unexpected calls fail immediately.

Call-count assertions remain independent:

```opal
expect writes.state to have been called once
```

## 12.5 Call history

Call history belongs to one attempt.

Recording MUST obey existing borrowing rules.

Do not retain arbitrary borrowed addresses after the call.

V1 MUST define which value types support argument snapshots. Unsupported
argument-history assertions produce a compiler diagnostic.

Count-only recording may remain available when full snapshots are unsupported.

## 12.6 Boundaries and limitations

- Calls through other import edges are unaffected.
- Same-module calls that do not use the declared edge are unaffected.
- Normal production binaries are unaffected.
- Unsupported indirect or foreign callback interception MUST be rejected,
  not silently ignored.
- Import aliases resolving to a supported edge retain the same interception.

Stub values vary at runtime within an already compiled boundary.
Changing example arguments does not require recompiling the artifact.

---

# 13. Test Failures and Application Errors

This is a required compiler/runtime milestone.

A failed assertion or unexpected mock invocation is not an application error.

Application `guard` handling MUST NOT catch a test failure and convert the test
into a pass.

A deliberately stubbed `FilesystemFullError` remains an ordinary application
error that application code can handle normally.

## 13.1 Internal failure channel

For test artifacts, introduce an internal outcome equivalent to:

```text
Completed(existing application return/error representation)
or
TestAborted(failure identity)
```

This is a compiler implementation detail.

It does not appear in source signatures and does not broaden application
error sets.

Affected call sites check for test abort before:

- Reading a return payload.
- Handling an application error.
- Entering an application `guard` branch.

An aborted call has no application return value.

Do not fabricate zeroes, empty arrays, success values, or undeclared errors.

Propagate the internal abort to the generated callback boundary.

The initial implementation MAY conservatively instrument the reachable
Opalescent call graph in the test artifact.

Preserve production ABIs and production artifact behavior.

## 13.2 Lifecycle integration

The runner invokes the body, each after-hook, and each cleanup callback through
separate boundaries.

An abort in the body does not prevent later lifecycle callbacks from starting.

A cleanup callback receives a fresh invocation boundary while the attempt's
earlier failures remain recorded.

Do not implement ordinary assertion failure with:

- Immediate process exit.
- `abort`.
- Unstructured `setjmp`/`longjmp`.
- A catchable application error.

Preserve existing language scope-exit behavior. Do not add new destructors
or resource obligations.

Unsupported test-abort propagation through foreign callback boundaries is
an explicit v1 limitation and MUST produce a diagnostic where statically known.

## 13.3 `errors inferred`

Allow `errors inferred` on framework-managed callbacks:

- Test bodies.
- Hooks.
- World setup.
- World cleanup.

It infers the existing nominal application error set.

It does not infer arbitrary effects or create exception semantics.

Ordinary helpers retain existing error-declaration rules.

Test failures use the separate internal channel.

---

# 14. Runner and Parallel Execution

## 14.1 Initial execution architecture

Use:

- A coordinator process.
- A bounded number of concurrently running native worker processes.
- One fresh worker process per case attempt.
- Reusable compiled test artifacts.

Do not compile one binary per case.

Do not reuse mutable process state between attempts in v1.

Fresh-process overhead is an acknowledged cost. Safe batching is a future
optimization that must preserve the public isolation contract.

## 14.2 Scheduling

The coordinator MUST schedule cases independently of file and scenario
boundaries.

Default concurrency SHOULD use available worker capacity, with an explicit
`--jobs` override.

Within one attempt, setup, hooks, body, and cleanup remain ordered.

The runner does not parallelize statements within a callback.

World or subject membership never implies a scheduling lock.

## 14.3 External sharing

Process isolation does not isolate:

- File paths.
- Ports.
- Database records.
- Remote accounts.
- Physical devices.

World implementations must choose independent environments when needed.

For deliberately shared resources, support scheduler-only exclusion metadata:

```opal
requires exclusive access to: 'laboratory_device'
```

Accept a list of constant string keys when necessary.

The coordinator acquires the complete key set before setup and holds it through
normal cleanup completion or worker termination.

Keys coordinate only within this runner invocation.

They do not acquire external service locks or protect against another CI job.

This feature is scheduling metadata, not resource tracking or leak protection.

## 14.4 Retries and timeouts

Default retries: zero.

An explicitly requested retry gets:

- A new worker process.
- New example values.
- New Worlds.
- New mock configuration and history.

Retain earlier failed attempts. A later pass is reported as a flaky pass,
not a clean first-attempt pass.

A hard timeout may require terminating the worker.

Report the last known phase and mark uncompleted callbacks accordingly.
Do not claim cleanup completed after forced termination.

## 14.5 CLI capabilities

Integrate with the repository's actual compiler CLI.

Required capabilities:

- Run discovered tests.
- List cases without running setup.
- Select an exact case identity.
- Filter by display text.
- Filter by category.
- Filter by declared function subject.
- Filter by World.
- Configure worker count.
- Configure timeout and retries.
- Emit text, JSON, and JUnit-compatible reports.

Suggested options:

```text
test
test --list
test --case <case-id>
test --filter <text>
test --kind integration
test --unit <qualified-function>
test --world <qualified-world>
test --jobs <count>
test --timeout <duration>
test --retries <count>
test --reporter json --output <path>
```

No matching cases SHOULD produce a nonzero result unless explicitly allowed.

Compilation or infrastructure failure MUST be distinguishable from test
assertion failure in machine-readable output.

---

# 15. Production-Binary End-to-End Testing

The framework MUST distinguish:

1. Instrumented native test artifacts.
2. Normal application executables.

End-to-end tests launch the second from the first.

The coordinator builds required application artifacts once using the normal
project build configuration.

Expose the built executable descriptor through the testing support library
so a World can launch it.

Production executables MUST NOT contain test replacements or the test-abort ABI.

A synchronous driver is sufficient for v1. Language-level async is not required.

## 15.1 Terminal editor driver

Provide a testing support driver capable of:

- Launching the executable through a real pseudoterminal.
- Setting working directory and terminal dimensions.
- Sending text and named keys.
- Decoding terminal output into a normalized screen.
- Reading rows and cursor position.
- Waiting for process exit.
- Capturing a transcript.
- Explicitly stopping and waiting for the child process from World cleanup.

The World author declares cleanup. Do not add general orphan-process recovery.

## 15.2 Editor acceptance flow

A required end-to-end test MUST:

1. Create a document in a private test workspace.
2. Launch the normal editor executable.
3. Wait for normal mode.
4. Enter insert mode and wait for the visible mode transition.
5. Type text.
6. Return to normal mode.
7. Open command entry.
8. Submit `wq`.
9. Observe successful process exit.
10. Read the actual file and verify its exact contents.

Clear fake-terminal backend overrides for this production-path test.

Do not assume the editor renders another complete frame after a successful
quit. Verify exit and file contents.

Polling assertions must observe state; they must not repeat input actions.

---

# 16. Reporting

Store one case record and expose multiple views.

## 16.1 Identity

Case identity is derived from the structured tuple:

```text
project-relative module
scenario ancestry
test title
optional example title
```

Use the complete structured identity or a deterministic encoding of it.

Do not use example position, source line alone, or process addresses.

Reordering examples does not change their identities.

Moving or renaming declarations may change identities; do not claim otherwise.

Attempts have separate identities beneath the case.

## 16.2 Required views

### By behavior

Show scenario, test, example, documentation, and outcome.

### By function

Show:

- Associated distinct cases.
- Outcomes.
- Failure phases.
- Associated test durations.

Do not label associated test duration as function execution time.

A case associated with several functions remains one case in overall totals.

### By World

Show:

- Associated cases.
- Direct, hook-only, activation-only, and transitive use.
- Setup and cleanup callback outcomes.
- Setup and cleanup durations.

Do not report cleanup success as evidence that external resources were released.

## 16.3 Attempt record

Include:

- Case and attempt identities.
- Category.
- Canonical subject identities.
- Effective World graph and request origins.
- Declared and activated double boundaries.
- Lifecycle phase outcomes.
- Primary and secondary failures.
- Captured diagnostics and attachments.
- Setup, body, after-hook, and cleanup durations.
- Scheduler and exclusion-key waiting time.
- Worker timeout or crash status.

The runner SHOULD expose the cost of serialization and expensive setup.

## 16.4 Outcome distinctions

Distinguish at least:

- Passed.
- Assertion failed.
- Unhandled application error.
- World setup failed.
- Before-hook failed.
- After-hook failed.
- Cleanup callback failed.
- Timed out.
- Worker crashed.
- Skipped by selection or scheduling policy.
- Flaky pass after retry.

A World setup failure must not be presented as an assertion failure in the
declared function under test.

Coverage collection and mutation testing are deferred.

---

# 17. Simple-Editor Conformance Example

This example is a required implementation fixture.

Verify the imported application APIs against the checkout before adapting it.

The testing APIs and language constructs below are defined by this
specification, not assumed to exist already.

## 17.1 `tests/editor_examples.types.op`

```opal
public type FailedSaveExample:
    command_text: string
```

## 17.2 `tests/editor.test.op`

```opal
import backspace_at_cursor, buffer_cursor, move_down from ../src/buffer
import apply_text_input, apply_named_key from ../src/input
import initial_editor_state from ../src/state

import type EditorMode, EditorStatus, EditorTermination from ../src/editor.types
import type TerminalNamedKey from standard.terminal
import type FailedSaveExample from ./editor_examples.types

import path_from, write_text_sync, read_text_sync from standard
import temporary_directory_create, temporary_directory_file from standard.testing
import temporary_directory_remove from standard.testing
import type TemporaryDirectory from standard.testing

##
    Description:
        Two lines containing a non-ASCII character.

        The first line has four Unicode scalar values. Cursor coordinates
        must not count the bytes used to encode the accented character.
##
world TwoLineDocument:
    setup: f(): string[] =>
        return ['café', '!']

##
    Description:
        A private real directory for one test attempt.

        Cleanup is explicitly declared. The framework calls it but does
        not verify whether external resources remain afterward.
##
world Workspace:
    setup: f(): TemporaryDirectory errors inferred =>
        return propagate temporary_directory_create()

    cleanup:
        f(mutable ref directory: TemporaryDirectory): void errors inferred =>
            propagate temporary_directory_remove(mutable ref directory)
            return void

double AtomicWrite replaces write_text_atomic_sync from standard
    as used by ../src/buffer

##
    Description:
        Controls the final atomic write made by buffer.op.

        Input handling, command execution, and buffer serialization remain
        real. Tests that do not request this World use the real writer.
##
world ControlledWrites:
    setup: f(): AtomicWrite =>
        return new AtomicWrite

    cleanup:
        f(mutable ref writer: AtomicWrite): void =>
            restore writer
            return void

##
    Description:
        Removing a line boundary must preserve the text on both sides.

        The cursor should end at the join in scalar coordinates.
##
scenario joining adjacent lines:
    unit under test: backspace_at_cursor
    kind: unit
    worlds: TwoLineDocument

    it should join both lines and leave the cursor after the accented word:
        f(ref document: TwoLineDocument): void errors inferred =>
            let cursor = buffer_cursor(1, 0)

            let updated_lines, updated_cursor = propagate backspace_at_cursor(
                document.state,
                cursor
            )

            expect updated_lines to contain exactly ['café!'] in order
            expect updated_lines.length to be less than document.state.length
            expect updated_cursor.line to be 0
            expect updated_cursor.column to be 4

            return void

##
    Description:
        Moving down onto a shorter line must keep the cursor within it.

        This checks buffer coordinates, not terminal display-cell widths.
##
scenario moving onto a shorter line:
    unit under test: move_down
    kind: unit
    worlds: TwoLineDocument

    it should clamp the cursor to the end of the destination line:
        f(ref document: TwoLineDocument): void errors inferred =>
            let cursor = buffer_cursor(0, 4)
            let result = propagate move_down(document.state, cursor)

            expect result.line to be 1
            expect result.column to be 1
            expect result.column to be less than cursor.column

            return void

##
    Description:
        A real edit followed by a write command must persist the edited
        document, clear the dirty flag, and leave the editor open.

        No write double is active for this test.
##
scenario saving an edited document:
    unit under test: apply_named_key
    kind: integration
    worlds: TwoLineDocument, Workspace

    it should save the edited lines and remain open for further editing:
        f(
            ref document: TwoLineDocument,
            ref workspace: Workspace
        ): void errors inferred =>
            let path = temporary_directory_file(ref workspace.state, 'note.txt')
            propagate write_text_sync(path_from(path), 'café\n!')

            let initial = initial_editor_state(path, true, true)
            let editing = initial with:
                mode: new EditorMode.Insert
                cursor: buffer_cursor(1, 1)

            let updated_lines: edited_lines, transition: edit =
                propagate apply_text_input(
                    document.state,
                    editing,
                    ' Reviewed'
                )

            expect edited_lines to contain exactly ['café', '! Reviewed'] in order
            expect edit.state.dirty to be true

            let command = edit.state with:
                mode: new EditorMode.Command
                command_text: 'w'

            let updated_lines, transition = propagate apply_named_key(
                path,
                edited_lines,
                command,
                new TerminalNamedKey.Enter
            )

            let saved_text = propagate read_text_sync(path_from(path))

            expect saved_text to be 'café\n! Reviewed'
            expect saved_text not to end with '\n'
            expect updated_lines to contain exactly ['café', '! Reviewed'] in order
            expect transition.state.status to be the Written variant of EditorStatus
            expect transition.state.dirty to be false
            expect transition.state.saved to be true
            expect transition.state.running to be true
            expect transition.state.mode to be the Normal variant of EditorMode
            expect transition.state.command_text to be empty
            expect transition.should_bell to be false

            return void

##
    Description:
        Failed saves must leave the unsaved document available.

        Write-and-quit permits closing only after saving succeeds.
        A full filesystem must not cause either command to discard edits.

        Only the final atomic write is replaced. The previous file contents
        remain real and are checked after the command.
##
scenario preserving unsaved work after a write failure:
    unit under test: apply_named_key
    kind: integration
    worlds: TwoLineDocument, Workspace, ControlledWrites

    before each test:
        f(mutable ref writes: ControlledWrites): void =>
            stub writes.state:
                otherwise fail the test

            return void

    after each test:
        f(ref writes: ControlledWrites): void =>
            verify writes.state
            return void

    it should keep the document open when the filesystem is full:
        examples of FailedSaveExample:
            example write without quitting:
                command_text: 'w'

            example write before quitting:
                command_text: 'wq'

        f(
            ref document: TwoLineDocument,
            ref workspace: Workspace,
            mutable ref writes: ControlledWrites,
            ref example: FailedSaveExample
        ): void errors inferred =>
            let path = temporary_directory_file(ref workspace.state, 'note.txt')
            propagate write_text_sync(path_from(path), 'Previous saved contents')

            let initial = initial_editor_state(path, true, true)
            let command = initial with:
                dirty: true
                mode: new EditorMode.Command
                command_text: example.command_text

            stub writes.state(path_from(path), 'café\n!')
                fails with FilesystemFullError

            let updated_lines, transition = propagate apply_named_key(
                path,
                document.state,
                command,
                new TerminalNamedKey.Enter
            )

            expect updated_lines to contain exactly ['café', '!'] in order
            expect transition.state.running to be true
            expect transition.state.dirty to be true
            expect transition.state.saved to be false
            expect transition.state.status to be the WriteFailed variant of EditorStatus
            expect transition.state.termination to be the Running variant of EditorTermination
            expect transition.state.mode to be the Normal variant of EditorMode
            expect transition.state.command_text to be empty
            expect transition.should_bell to be true
            expect writes.state to have been called once

            let unchanged_text = propagate read_text_sync(path_from(path))
            expect unchanged_text to be 'Previous saved contents'

            return void
```

This file produces five independent cases:

1. Joining lines.
2. Moving onto a shorter line.
3. Saving a real edited document.
4. Failed `w`.
5. Failed `wq`.

The two failed-save examples share compiled test code but have independent
World instances, mock behavior, hooks, results, and scheduling.

## 17.3 Minimum testing-library contracts

The temporary-directory helpers used above must have documented behavior:

- `temporary_directory_create` creates a unique directory.
- `temporary_directory_file` returns a child filename suitable for existing
  filesystem APIs.
- `temporary_directory_remove` explicitly removes that directory and its
  contents or returns an application error.

These helpers do not register resources with a leak detector or cause
automatic cleanup when the user omits the World cleanup callback.

---

# 18. Compiler and Runtime Integration

Confirm actual file locations before editing.

| Area | Implementation responsibility |
| --- | --- |
| `src/lexer.rs` | Test-mode prose headers, contextual phrases, source spans |
| `src/parser.rs` | Worlds, scenarios, tests, metadata, hooks, examples, assertions, doubles |
| `src/ast.rs` | Structured representations of test declarations |
| Type checker and resolver | Nominal World identities, injections, subjects, matchers, mocks, errors |
| Module resolver | Test roots, test-only imports, production-build rejection |
| Project builder | Separate artifacts, normal application builds for E2E |
| Native lowering/code generation | Case wrappers, World state slots, lifecycle, test-abort channel |
| Runtime support | Assertions, double dispatch, descriptors, result transport |
| Coordinator/runner | Discovery, scheduling, workers, filtering, reports |
| Testing library | Explicit workspace operations and terminal/process drivers |

Required internal representations include equivalents of:

```text
WorldDeclaration
WorldIdentity
WorldDependency
ScenarioDeclaration
TestDeclaration
ExampleDeclaration
HookDeclaration
SubjectReference
DoubleDeclaration
ResolvedImportEdge
Expectation
CaseDescriptor
AttemptResult
PhaseResult
```

Use existing compiler abstractions where suitable.

Do not store source-only strings where resolved declaration identities are
required.

## 18.1 Artifact separation

Test artifact cache keys MUST include relevant inputs such as:

- Source and compiler versions.
- Target and build profile.
- Test runtime/ABI version.
- Interception declarations.
- Testing feature configuration.

Production cache entries must not be reused as instrumented test modules,
or vice versa.

Construct interception before inlining and other transformations that can
erase the relevant source boundary.

Do not disable optimization globally as the mocking strategy.

---

# 19. Delivery Plan

## Phase 0 — Verify the repository baseline

Tasks:

- Inspect repository instructions.
- Confirm compiler CLI and project layout.
- Confirm lexer, parser, AST, checker, and code-generation entry points.
- Confirm current function, reference, error, import, and labeled-return syntax.
- Confirm editor APIs and behaviors used by conformance fixtures.
- Run the relevant existing baseline checks.
- Record any implementation adjustment needed to preserve current syntax.

Exit condition:

A concrete source map and compatible fixture API list exist.
No speculative source path is treated as verified without inspection.

## Phase 1 — Frontend and AST

Tasks:

- Add test-module mode.
- Add prose-header tokenization.
- Parse scenarios, Worlds, hooks, subscriptions, examples, and subjects.
- Parse assertion and double constructs.
- Preserve documentation and source spans.
- Add test-only import forms.

Exit condition:

Valid fixtures produce structured ASTs.
Malformed syntax produces localized diagnostics.
Ordinary application parsing remains unchanged.

## Phase 2 — Resolution and static validation

Tasks:

- Resolve function subjects.
- Generate distinct World handle types.
- Validate setup and cleanup signatures.
- Validate dependency graphs and ordered injections.
- Implement subscription replacement and activation-only requirements.
- Implement strict unused-World diagnostics.
- Check example schemas.
- Type-check matchers and mock configurations.
- Implement test-callback `errors inferred`.

Exit condition:

Invalid World orders, missing dependencies, unused injections, invalid
subjects, malformed examples, and invalid mock errors fail compilation.

## Phase 3 — Case lowering and lifecycle

Tasks:

- Generate case descriptors and typed callback wrappers.
- Allocate per-attempt World state.
- Implement deterministic initialization and declared cleanup.
- Implement hook scope entry and exit.
- Add the internal test-abort channel.
- Preserve primary and secondary failures.
- Support ordinary application errors independently.

Exit condition:

Failure at every supported lifecycle phase produces the specified remaining
callback behavior.

An application `guard` cannot swallow a test failure.

## Phase 4 — Assertions and doubles

Tasks:

- Implement the fixed matcher vocabulary.
- Capture operands once.
- Implement source-aware failure diagnostics.
- Implement import-edge dispatchers.
- Implement attempt-local activation and configuration.
- Implement exact stubs, custom replacements, spies, verification, and history.
- Validate inactive forwarding and original-call bypasses.

Exit condition:

A non-fallible application function making an unexpected mocked call fails
the test correctly and reaches declared lifecycle callbacks.

Expected application errors remain catchable by application code.

## Phase 5 — Coordinator, examples, and reports

Tasks:

- Discover and list independent cases.
- Implement fresh-process scheduling with bounded concurrency.
- Add selection, timeouts, retries, and exclusion keys.
- Add structured attempt-result transport.
- Implement behavior, function, and World views.
- Emit text, JSON, and JUnit-compatible reports.

Exit condition:

Cases in one file and one scenario can run concurrently with independent
Worlds and mock histories.

Parameterized examples are separately selectable and reportable.

## Phase 6 — Testing support and editor acceptance

Tasks:

- Implement explicit temporary-directory helpers.
- Implement the synchronous terminal driver.
- Build the normal editor artifact for E2E tests.
- Compile and execute the conformance tests in this specification.
- Add the production-binary terminal save-and-quit test.

Exit condition:

Unit, real-filesystem integration, mocked-failure integration, and
production-binary E2E tests run through the same framework.

## Phase 7 — Documentation and release readiness

Tasks:

- Document language grammar and lifecycle semantics.
- Document mock interception boundaries.
- Document reporting association versus coverage.
- Document parallel defaults and external-resource responsibilities.
- Document failure, timeout, and crash limitations.
- Document all deferred features.
- Add examples for product owners, SDETs, and language developers.

Exit condition:

The feature can be used without relying on undocumented injection,
mocking, cleanup, or scheduling behavior.

---

# 20. Required Acceptance Tests

## 20.1 Parsing

- Apostrophes in unquoted titles.
- `#` inside unquoted titles.
- Escaped and quoted colons.
- Inline and indented callbacks.
- Multiline function signatures.
- Supported matcher phrases inside expressions and string literals.
- Accurate source spans and useful malformed-title errors.

## 20.2 Compiler checks

- A missing function under test is rejected.
- A call expression used as subject metadata is rejected.
- An alias resolves to the same canonical function.
- Two Worlds returning the same state type remain distinct.
- Reordered, missing, duplicate, and extra World parameters are rejected.
- An unused injected World parameter is rejected.
- A hook's World use does not satisfy the body's unused check.
- `worlds: none` removes inherited body injections.
- Example fields are checked against their record type.
- Dependency cycles report the cycle path.
- Diamond dependencies initialize once per attempt.
- Invalid mock return types and error variants are rejected.

## 20.3 Lifecycle

- A setup failure skips the body.
- Failed setup does not call cleanup with uninitialized state.
- Previously initialized Worlds still receive declared cleanup.
- A before-hook failure runs after-hooks for entered scopes.
- An assertion failure runs applicable after-hooks and cleanup.
- A failing after-hook does not skip later cleanup.
- A failing cleanup does not skip remaining cleanups.
- Secondary failures do not erase the primary failure.
- A successful body with failing cleanup is a failed case.
- Omitted cleanup remains legal without a resource-analysis diagnostic.

## 20.4 Failure transport and mocking

- Application `guard` cannot catch a harness failure.
- Application `guard` can handle a deliberately stubbed application error.
- Unexpected calls from non-fallible functions do not fabricate values.
- An inactive double calls the original function.
- A spy's original call does not recursively invoke its own dispatcher.
- Interception affects only the declared import edge.
- Unsupported interception paths produce explicit diagnostics.
- Verification detects an unused configured rule.
- Call-count assertions observe actual counts.

## 20.5 Parallel execution and reporting

- Two cases in one scenario can overlap.
- Different examples of one body can overlap.
- Each attempt gets fresh Worlds.
- Concurrent configurations of one declared double remain independent.
- World and function groupings do not serialize cases.
- Exclusion keys serialize only cases sharing those keys.
- Example reordering preserves case identities.
- Multi-subject cases are not double-counted in overall totals.
- Setup failures are reported as setup failures.
- A retry retains the failed attempt and gets fresh state.
- A hard-terminated worker does not report unexecuted cleanup as successful.

Use synchronization or runner instrumentation to verify overlap and isolation.
Do not rely solely on timing-sensitive sleeps.

## 20.6 Editor behavior

- Backspace joins `['café', '!']` at `(1, 0)` into `['café!']` at `(0, 4)`.
- Moving down from `(0, 4)` clamps to `(1, 1)`.
- A real write persists the edited document without closing the editor.
- Failed `w` preserves unsaved state and the previous file.
- Failed `wq` preserves unsaved state and does not quit.
- Production-binary E2E verifies real terminal interaction and persisted data.

---

# 21. Deferred Features

The following are outside this implementation:

- Every form of leak protection listed in Section 1.5.
- Async syntax and execution semantics.
- Awaited cleanup.
- Structured task scopes and cancellation.
- Shared mutable or suite-scoped Worlds.
- Before-all and after-all hooks.
- Same-process concurrent case execution.
- Safe worker batching or process-state reuse.
- Runtime-generated example discovery.
- Arbitrary natural-language interpretation.
- Separate Gherkin feature files and step binding.
- Mandatory `given` / `when` / `then` blocks.
- Coverage collection.
- Mutation testing.
- Arbitrary native machine-code patching.
- Unsupported indirect or foreign callback mocking.

Future async support must receive its own specification.

This implementation should keep lifecycle boundaries explicit so they can be
extended later, but it must not select an async model or claim to solve async
resource lifetimes now.

---

# 22. Definition of Done

The implementation is complete when:

1. The proposed syntax compiles through the normal compiler pipeline.
2. World identities, subscriptions, dependencies, and usage are checked.
3. Named examples are independently compiled into case descriptors,
   scheduled, filtered, and reported.
4. Assertions and typed doubles work without altering production behavior.
5. Harness failures remain separate from application errors.
6. Declared lifecycle callbacks run according to the failure rules.
7. Tests execute in parallel by default across file and scenario boundaries.
8. Function and World reporting uses resolved identities and honest metrics.
9. The simple-editor unit, integration, and production-binary E2E tests pass.
10. No leak detector, cleanup proof system, automatic resource recovery,
    new ownership protocol, or async subsystem has been introduced.

---

# 23. Repository Reference Targets

Verify these against the implementation checkout:

- Repository:
  <https://github.com/justinfarrelldev/opalescent>

- Language overview:
  <https://github.com/justinfarrelldev/opalescent/blob/main/OPALESCENT_CRASH_COURSE.md>

- Lexer:
  <https://github.com/justinfarrelldev/opalescent/blob/main/src/lexer.rs>

- Parser:
  <https://github.com/justinfarrelldev/opalescent/blob/main/src/parser.rs>

- AST:
  <https://github.com/justinfarrelldev/opalescent/blob/main/src/ast.rs>

- Editor buffer operations:
  <https://github.com/justinfarrelldev/opalescent/blob/main/test-projects/terminal-simple-editor/src/buffer.op>

- Editor input and commands:
  <https://github.com/justinfarrelldev/opalescent/blob/main/test-projects/terminal-simple-editor/src/input.op>

- Editor state:
  <https://github.com/justinfarrelldev/opalescent/blob/main/test-projects/terminal-simple-editor/src/state.op>

- Editor executable:
  <https://github.com/justinfarrelldev/opalescent/blob/main/test-projects/terminal-simple-editor/src/main.op>

- Terminal testing proposal:
  <https://github.com/justinfarrelldev/opalescent/blob/main/stdlib-proposals/terminal-session-input/TESTING.md>
