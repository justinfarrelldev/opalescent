# Proposal: Named Error Sets

## Status

Draft language proposal.

This proposal is intentionally about the language/compiler surface, not about adding
new runtime error wrappers. It should be implemented with test-driven development,
red-green-refactor, and atomic commits. The implementation should stay on the
current branch while this proposal is being executed.

## Summary

Opalescent should add **named error sets**: compile-time aliases for exact sets of
existing leaf error types.

A named error set is not a new error value, not a wrapper, not an enum variant, and
not a dynamic supertype. It is a checked shorthand for a canonical list of leaf
errors. The type checker expands the set before checking `propagate`, `guard`,
function compatibility, documentation, LSP hovers, and exported module metadata.

```opal
# render_errors.types.op
public error set RenderErrors =
    WriteFailureError,
    FlushFailureError,
    TerminalWriteFailureError,
    SinkClosedError

# render.op
import type RenderErrors from ./render_errors.types

public let render_frame = f(writer: StdoutWriter, rows: string[]): void errors RenderErrors =>
    for row in rows:
        propagate writer_write_sync(writer, row)
    propagate writer_flush_sync(writer)
    return void
```

The public signature is short, but the compiler still expands it to concrete
members. If a caller guards the call, the guard binding is a precise union of the
expanded leaf errors, not a `RenderErrors` wrapper. The body-lint pass can also see
that this particular implementation only emits the writer leaves.

```opal
guard render_frame(writer, rows) else err =>
    if err is WriteFailureError:
        print('could not write the frame')
        return void
    if err is FlushFailureError:
        print('frame was written but flush failed')
        return void
    if err is SinkClosedError:
        print('output was closed')
        return void
    propagate err
```

Because `RenderErrors` above contains `TerminalWriteFailureError` but this
particular function only uses the writer APIs, the linter should produce a warning
and recommend the exact three used leaf errors or a narrower exact set.

## Problem

Explicit `errors` clauses are one of Opalescent's best safety features, but large
programs now have signatures like this:

```opal
entry main = f(args: string[]): void errors FilesystemReadError, FilesystemWriteError, IndexOutOfBoundsError, AllocationFailureError, StringRangeError, TerminalTextLayoutError, TerminalSessionOpenError, TerminalSessionReadError, TerminalSessionWriteError, TerminalSessionStateError, TerminalSessionRestoreError, InvalidCursorPositionError, InvalidEnvironmentVariableNameError =>
    # ...
```

This makes files longer, makes examples harder to scan, and encourages broad
existing aliases even when a function only uses part of a broad family.

The language needs a way to name repeated combinations while preserving the exact
leaf-level error model.

## Goals

- Shorten long `errors` clauses without hiding the actual possible failures.
- Keep guard bindings aware of every suberror inside a set.
- Keep leaf errors usable everywhere they are usable today.
- Let standard-library, package, and application authors publish stable aliases.
- Warn when a long explicit leaf list can be collapsed into a known set.
- Warn when a broad set is used but one or more members cannot escape the function.
- Emit all new diagnostics through Miette, with spans, warning codes, help text,
  and machine-actionable suggestions where possible.
- Make generated documentation and LSP hovers show both the concise alias and the
  expanded leaf list.
- Rewrite all `test-projects/` fixtures to their proper aliases as part of the
  implementation once the feature is available.

## Non-goals

- No exceptions.
- No hidden propagation.
- No `AnyError`.
- No wildcard `errors filesystem::*`.
- No runtime wrapper values for aliases.
- No loss of exhaustiveness for guards or future `match` forms.
- No ABI change for generated success/error return layouts.

## Syntax

### Declaration

Named error sets are type-level declarations and should live in `.types.op` files.
This mirrors product, sum, and enum declarations while making it clear that no
runtime value is introduced.

```opal
public error set FilesystemReadErrors =
    FileNotFoundError,
    PermissionDeniedError,
    ReadFailureError,
    IsADirectoryError,
    InvalidPathError,
    InvalidUtf8Error,
    OffsetOutOfRangeError
```

The grammar-level shape is:

```text
error_set_decl := visibility? "error" "set" PascalCaseIdentifier "=" error_set_member_list
error_set_member_list := error_set_member ("," error_set_member)* trailing_comma?
error_set_member := TypeIdentifier
```

The preferred naming convention is plural `*Errors` for sets and singular
`*Error` for leaf errors. Existing singular compatibility aliases such as
`OutputError` may remain importable, but new canonical aliases should use the
plural spelling.

### Import

Use the existing type-import syntax. Error sets are compile-time type-level
aliases, so adding a separate `import error set` form would add user-facing
cognitive overhead without giving the compiler or LSP meaningful information that
name resolution does not already provide.

```opal
import type RenderErrors from ./render_errors.types
import type FilesystemReadErrors, FilesystemWriteErrors from standard.errors
```

Tooling should distinguish error sets after resolution through semantic tokens,
hovers, completions, and diagnostics. A hover for `RenderErrors`, for example,
should show that the imported type-level symbol is an error-set alias and display
its expanded leaves. Mistaken value construction is still rejected by the resolver
and type checker; no special import syntax is required for that safety property.

### Use in signatures

An `errors` clause can contain leaves, named sets, or a mix.

```opal
let load_and_render = f(path: FilesystemPath): void errors FilesystemReadErrors, RenderErrors =>
    let rows = propagate read_lines_sync(path)
    propagate render_frame(rows)
    return void
```

A mixed declaration is expanded, flattened, de-duplicated, and canonicalized.

```opal
errors RenderErrors, WriteFailureError
```

The extra `WriteFailureError` is redundant if `RenderErrors` already contains it;
the linter should warn and remove the duplicate in the suggested replacement.

### Nested sets

Sets may include other sets.

```opal
public error set StdoutWriterErrors =
    WriteFailureError,
    FlushFailureError,
    SinkClosedError

public error set TerminalRenderErrors =
    TerminalWriteFailureError,
    InvalidCursorPositionError,
    SinkClosedError

public error set RenderErrors =
    StdoutWriterErrors,
    TerminalRenderErrors,
    TerminalTextLayoutError,
    AllocationFailureError
```

The canonical expansion of `RenderErrors` is the sorted, duplicate-free leaf set:

```text
AllocationFailureError
FlushFailureError
InvalidCursorPositionError
SinkClosedError
TerminalTextLayoutError
TerminalWriteFailureError
WriteFailureError
```

## Semantics

### Expansion is compile-time only

The compiler stores every function's error contract in two equivalent forms:

1. the source spelling, for docs and precise suggestions;
2. the canonical expanded leaf set, for type checking and compatibility.

At runtime, an error value is still exactly one leaf error family and variant.
There is no `RenderErrors` object to allocate, inspect, compare, or print.

### Handling obligations are leaf-exhaustive

Using a named set never counts as handling the set's members. At every call site,
`guard`, `propagate`, callback boundary, and future `match` expression, the set is
expanded first and every expanded leaf error must be handled exactly as if the
leaf names had been written directly.

If a function declares `errors RenderErrors`, callers must handle or propagate all
leaves in `RenderErrors`. A caller cannot handle only `WriteFailureError` and
`FlushFailureError` while ignoring `TerminalWriteFailureError` just because the
contract was spelled with one alias. The only ways out are the existing language
mechanisms: recover locally, propagate to an enclosing function whose expanded
`errors` clause covers the leaf, return/terminate in a recognized handler, or use
an explicit future discard mechanism if the language adopts one.

A lint warning that a set is broader than a function's implementation does not
change the function's contract. Until the source is changed from the broad set to
a narrower set or explicit leaves, downstream callers must assume and handle the
full expanded set.

### Function compatibility

Two function signatures have compatible error contracts when their expanded leaf
sets are compatible. These are equivalent for checking purposes:

```opal
let a = f(): void errors WriteFailureError, FlushFailureError, SinkClosedError => ...
let b = f(): void errors StdoutWriterErrors => ...
```

Documentation should preserve the author-written spelling but display the
expansion.

### Propagation

`propagate` succeeds when every possible leaf error from the callee is contained
in the expanded leaf set of the enclosing function.

```opal
let write_all = f(writer: StdoutWriter, rows: string[]): void errors StdoutWriterErrors =>
    for row in rows:
        propagate writer_write_sync(writer, row) # WriteFailureError | SinkClosedError
    propagate writer_flush_sync(writer)          # FlushFailureError | SinkClosedError
    return void
```

### Guard bindings

A guard binding receives the exact expanded leaf union of the guarded expression.
The alias name does not erase member knowledge.

```opal
guard write_all(writer, rows) else err =>
    # err: WriteFailureError | FlushFailureError | SinkClosedError
    if err is SinkClosedError:
        print('the sink is closed')
        return void
    propagate err
```

If future typed `match` is adopted, matching against a set-expanded guard binding
must require cases for the expanded leaves, not one synthetic alias case.

### Public API and hot reload

Changing the expansion of a public error set is a public API change for any public
function whose exported contract includes that set. Hot-reload interface hashes
must include both the alias name and the canonical leaf expansion.

This avoids the bad behavior of wildcards: adding a new leaf to a public set is a
reviewable breaking or compatibility-managed change, not an invisible import-side
change.

### Cycles and invalid members

The resolver must reject cycles:

```opal
public error set AErrors = BErrors
public error set BErrors = AErrors
```

Miette error:

```text
error[E-error-set-cycle]: error set cycle detected
  --> errors.types.op:1:18
   |
 1 | public error set AErrors = BErrors
   |                  ^^^^^^^ participates in a cycle
help: break the cycle by replacing one set member with concrete leaf errors
```

The resolver must also reject non-error members, unknown names, generic
instantiations, value names, and empty sets.

## Linter behavior

The linter should run after type checking, because it needs both declared error
contracts and body-derived escaping error information.

### Warning: explicit leaves collapse to a known set

If a function declares a leaf list whose canonical set exactly matches a known
alias, emit a warning.

```opal
let render = f(): void errors WriteFailureError, FlushFailureError, SinkClosedError =>
    # ...
```

Miette warning:

```text
warning[W-error-set-collapsible]: error clause can use named error set `StdoutWriterErrors`
  --> render.op:1:32
   |
 1 | let render = f(): void errors WriteFailureError, FlushFailureError, SinkClosedError =>
   |                                ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
help: replace with `errors StdoutWriterErrors`
```

If multiple aliases have the exact same expansion, prefer the narrowest imported
alias, then the same-module alias, then the standard-library canonical alias. If
there is still a tie, do not warn; ambiguity should not create churn.

### Warning: set contains unused members for this function

If a function declares a set but body-derived escaping errors are a strict subset
of the set's expansion, emit a warning.

```opal
let writer_only_render = f(writer: StdoutWriter, rows: string[]): void errors RenderErrors =>
    for row in rows:
        propagate writer_write_sync(writer, row)
    propagate writer_flush_sync(writer)
    return void
```

Miette warning:

```text
warning[W-error-set-unused-member]: `RenderErrors` includes errors that cannot escape this function
  --> render.op:1:74
   |
 1 | let writer_only_render = f(writer: StdoutWriter, rows: string[]): void errors RenderErrors =>
   |                                                                          ^^^^^^^^^^^^
   |
   = unused through this set: TerminalWriteFailureError, InvalidCursorPositionError, TerminalTextLayoutError, AllocationFailureError
help: use `errors StdoutWriterErrors`
```

If no exact narrower set is in scope, the help should recommend the exact used
leaves instead:

```text
help: use `errors WriteFailureError, FlushFailureError, SinkClosedError`
```

This is a warning, not an error. Public APIs sometimes intentionally reserve a
stable broader contract. The project may later add an allow mechanism such as:

```opal
# opal lint allow unused_error_set_member: public compatibility promise
public let render_plugin_frame = f(plugin: Plugin): void errors RenderErrors => ...
```

The allow mechanism is not required for the first parser/type-checker feature, but
the diagnostic should be designed so adding suppression later is straightforward.

### Warning: redundant set and leaf

```opal
let f = f(): void errors RenderErrors, WriteFailureError => ...
```

Warn that `WriteFailureError` is already included by `RenderErrors`.

### Warning: overlapping sets

```opal
let f = f(): void errors StdoutWriterErrors, RenderErrors => ...
```

Warn if one set is wholly contained in another. If the function truly uses all of
`RenderErrors`, keep only `RenderErrors`; if it uses only the writer subset, use
`StdoutWriterErrors` or exact leaves.

### No warning for singleton sets by default

Singleton aliases such as `ParseErrors = ParseError` are useful for consistency,
but the linter should not force `errors ParseErrors` over `errors ParseError`.
They should be accepted without style churn.

## Diagnostics must use Miette

All parser, resolver, type checker, linter, and documentation diagnostics added
for this feature must use Miette. Each diagnostic should include:

- severity (`Error` for invalid declarations, `Warning` for lints);
- stable code (`E-error-set-cycle`, `W-error-set-collapsible`, etc.);
- source span labels;
- help text with an exact replacement when possible;
- related labels for alias declarations and set members when useful.

Example with related alias declaration:

```text
warning[W-error-set-unused-member]: `RenderErrors` is broader than this function needs
  --> src/render.op:12:87
   |
12 | public let draw_status = f(status: string): void errors RenderErrors =>
   |                                                       ----------- declared here
   |
  ::: src/render_errors.types.op:2:18
   |
 2 | public error set RenderErrors = WriteFailureError, FlushFailureError, TerminalWriteFailureError, SinkClosedError
   |                  ^^^^^^^^^^^^ includes `TerminalWriteFailureError`, which cannot escape `draw_status`
help: use `errors WriteFailureError, FlushFailureError, SinkClosedError`
```

## Standard-library named error sets

The implementation should ship a canonical `standard.errors` surface. The tables
below are intentionally exhaustive for current standard-library and terminal
proposal errors. Names may be refined during implementation, but every current
leaf should either appear in a useful set or be documented as intentionally
standalone.

### Core and allocation

| Set | Members | Notes |
|---|---|---|
| `ParseErrors` | `ParseError` | Singleton; no style warning. |
| `AllocationErrors` | `AllocationFailureError` | Singleton; useful for nested sets. |
| `IndexAccessErrors` | `IndexOutOfBoundsError` | Singleton; arrays, strings, chord released input, suppressed errors. |
| `ConstraintErrors` | `ConstraintViolationError` | Singleton for test/fake backend builders and future contracts. |

### Bytes

| Set | Members |
|---|---|
| `BytesDecodeErrors` | `HexDecodeError` |
| `BytesSliceErrors` | `SliceRangeError` |
| `BytesErrors` | `HexDecodeError`, `SliceRangeError` |

### Strings and terminal text layout

| Set | Members |
|---|---|
| `StringSearchErrors` | `StringEmptySearchTextError`, `StringPatternNotFoundError` |
| `StringRangeErrors` | `StringNegativeCountError`, `StringRangeOutOfBoundsError`, `StringRangeOrderError` |
| `StringMutationErrors` | `StringRangeOutOfBoundsError`, `StringRangeOrderError`, `AllocationFailureError` |
| `StringSliceErrors` | `StringNegativeCountError`, `StringRangeOutOfBoundsError`, `StringRangeOrderError`, `AllocationFailureError` |
| `StringBuilderErrors` | `BuilderFinishedError`, `AllocationFailureError` |
| `StringJoinErrors` | `AllocationFailureError` |
| `TerminalTextLayoutErrors` | `TerminalTextLayoutError`, `AllocationFailureError` |
| `StringProcessingErrors` | `StringSearchErrors`, `StringRangeErrors`, `StringBuilderErrors`, `TerminalTextLayoutErrors`, `AllocationFailureError` |

### Console, stdout writer, and rendering

| Set | Members |
|---|---|
| `StandardInputErrors` | `StandardInputReadError` |
| `StandardOutputAcquireErrors` | `StandardOutputHandleError` |
| `StandardOutputCapabilityErrors` | `StandardOutputCapabilityError` |
| `StdoutWriteErrors` | `WriteFailureError`, `SinkClosedError` |
| `StdoutFlushErrors` | `FlushFailureError`, `SinkClosedError` |
| `StdoutWriterErrors` | `WriteFailureError`, `FlushFailureError`, `SinkClosedError` |
| `TerminalControlErrors` | `TerminalWriteFailureError`, `InvalidCursorPositionError`, `SinkClosedError` |
| `TerminalDrawErrors` | `TerminalWriteFailureError`, `SinkClosedError` |
| `RenderErrors` | `WriteFailureError`, `FlushFailureError`, `TerminalWriteFailureError`, `InvalidCursorPositionError`, `SinkClosedError` |
| `RenderSetupErrors` | `StandardOutputHandleError`, `StandardOutputCapabilityError` |
| `AnsiRenderErrors` | `RenderSetupErrors`, `RenderErrors` |
| `ConsoleIoErrors` | `StandardInputReadError`, `StandardOutputHandleError`, `StandardOutputCapabilityError`, `WriteFailureError`, `FlushFailureError`, `TerminalWriteFailureError`, `InvalidCursorPositionError`, `SinkClosedError` |

`RenderErrors` is intentionally broad enough for frame-style rendering that mixes
writer output, flushes, and terminal cursor/screen commands. A writer-only helper
should receive an unused-member warning and use `StdoutWriterErrors` instead.

### Time and clocks

| Set | Members |
|---|---|
| `SleepErrors` | `InvalidDurationError` |
| `FrameClockErrors` | `InvalidFrameRateError` |
| `TimeErrors` | `InvalidDurationError`, `InvalidFrameRateError` |
| `MonotonicTimerErrors` | `MonotonicTimerError`, `MonotonicTimerNotArmedError` |
| `ClockErrors` | `TimeErrors`, `MonotonicTimerErrors` |

### Filesystem paths, reads, writes, and metadata

| Set | Members |
|---|---|
| `FilesystemPathErrors` | `InvalidPathError`, `PermissionDeniedError` |
| `FilesystemExistenceErrors` | `PermissionDeniedError`, `InvalidPathError` |
| `FilesystemRawReadErrors` | `FileNotFoundError`, `PermissionDeniedError`, `ReadFailureError`, `IsADirectoryError`, `InvalidPathError` |
| `FilesystemTextReadErrors` | `FileNotFoundError`, `PermissionDeniedError`, `ReadFailureError`, `IsADirectoryError`, `InvalidPathError`, `InvalidUtf8Error` |
| `FilesystemOffsetReadErrors` | `FileNotFoundError`, `PermissionDeniedError`, `ReadFailureError`, `OffsetOutOfRangeError`, `InvalidPathError` |
| `FilesystemLineReadErrors` | `FileNotFoundError`, `PermissionDeniedError`, `ReadFailureError`, `IsADirectoryError`, `InvalidUtf8Error`, `OffsetOutOfRangeError` |
| `FilesystemReadErrors` | `FileNotFoundError`, `PermissionDeniedError`, `ReadFailureError`, `IsADirectoryError`, `InvalidPathError`, `InvalidUtf8Error`, `OffsetOutOfRangeError` |
| `FilesystemOverwriteErrors` | `PermissionDeniedError`, `WriteFailureError`, `IsADirectoryError`, `InvalidPathError`, `FilesystemFullError` |
| `FilesystemAppendErrors` | `FileNotFoundError`, `PermissionDeniedError`, `WriteFailureError`, `IsADirectoryError`, `InvalidPathError`, `FilesystemFullError` |
| `FilesystemOffsetWriteErrors` | `FileNotFoundError`, `PermissionDeniedError`, `WriteFailureError`, `OffsetOutOfRangeError`, `InvalidPathError`, `FilesystemFullError` |
| `FilesystemWriteErrors` | `FileNotFoundError`, `PermissionDeniedError`, `WriteFailureError`, `IsADirectoryError`, `InvalidPathError`, `FilesystemFullError`, `OffsetOutOfRangeError` |
| `FilesystemCreateErrors` | `FileAlreadyExistsError`, `PermissionDeniedError`, `CreateFailureError`, `InvalidPathError`, `FilesystemFullError` |
| `FilesystemDeleteErrors` | `FileNotFoundError`, `PermissionDeniedError`, `DeleteFailureError`, `IsADirectoryError`, `InvalidPathError` |
| `FilesystemDirectoryDeleteErrors` | `DirectoryNotFoundError`, `PermissionDeniedError`, `DeleteFailureError`, `DirectoryNotEmptyError`, `IsNotADirectoryError`, `InvalidPathError` |
| `FilesystemCopyErrors` | `FileNotFoundError`, `PermissionDeniedError`, `CopyFailureError`, `IsADirectoryError`, `InvalidPathError`, `FilesystemFullError` |
| `FilesystemMoveErrors` | `FileNotFoundError`, `PermissionDeniedError`, `MoveFailureError`, `FileAlreadyExistsError`, `InvalidPathError` |
| `FilesystemCopyMoveErrors` | `FilesystemCopyErrors`, `FilesystemMoveErrors` |
| `FilesystemMetadataErrors` | `FileNotFoundError`, `PermissionDeniedError`, `MetadataUnavailableError`, `InvalidPathError` |
| `FilesystemListErrors` | `DirectoryNotFoundError`, `PermissionDeniedError`, `ReadFailureError`, `IsNotADirectoryError`, `InvalidPathError` |
| `FilesystemDirectoryCreateErrors` | `FileAlreadyExistsError`, `PermissionDeniedError`, `CreateFailureError`, `InvalidPathError`, `FilesystemFullError` |
| `FilesystemRecursiveDirectoryCreateErrors` | `PermissionDeniedError`, `CreateFailureError`, `InvalidPathError`, `FilesystemFullError` |
| `FilesystemPermissionErrors` | `PermissionDeniedError`, `SetPermissionsError`, `InvalidPathError` |
| `FilesystemMutationErrors` | `FilesystemWriteErrors`, `FilesystemCreateErrors`, `FilesystemDeleteErrors`, `FilesystemDirectoryDeleteErrors`, `FilesystemCopyMoveErrors`, `FilesystemPermissionErrors` |
| `FilesystemErrors` | all filesystem leaves, including `LineOutOfRangeError` and `SetPermissionsError` for registered compatibility |

### Process environment and process paths

| Set | Members |
|---|---|
| `ProcessCurrentDirectoryErrors` | `PermissionDeniedError`, `InvalidPathError`, `CurrentWorkingDirectoryUnavailableError` |
| `ProcessExecutablePathErrors` | `PermissionDeniedError`, `InvalidPathError`, `CurrentExecutablePathUnavailableError` |
| `ProcessSetDirectoryErrors` | `FileNotFoundError`, `PermissionDeniedError`, `IsNotADirectoryError`, `InvalidPathError` |
| `ProcessPathErrors` | `PermissionDeniedError`, `InvalidPathError`, `CurrentWorkingDirectoryUnavailableError`, `CurrentExecutablePathUnavailableError`, `FileNotFoundError`, `IsNotADirectoryError` |
| `EnvironmentLookupErrors` | `EnvironmentVariableNotFoundError`, `InvalidEnvironmentVariableNameError`, `InvalidUtf8Error` |
| `EnvironmentOptionalLookupErrors` | `InvalidEnvironmentVariableNameError`, `InvalidUtf8Error` |
| `EnvironmentProbeErrors` | `InvalidEnvironmentVariableNameError` |
| `ProcessEnvErrors` | `EnvironmentVariableNotFoundError`, `InvalidEnvironmentVariableNameError`, `InvalidUtf8Error` |
| `ProcessErrors` | `ProcessPathErrors`, `ProcessEnvErrors` |

### Core system wait sets and process control

| Set | Members |
|---|---|
| `SystemWaitSetErrors` | `SystemWaitSetError` |
| `SystemWaitRegistrationErrors` | `SystemWaitSetError`, `AllocationFailureError` |
| `ProcessControlOpenErrors` | `ProcessControlUnavailableError`, `AllocationFailureError` |
| `ProcessControlPollErrors` | `ProcessControlError` |
| `ProcessControlAckErrors` | `ProcessControlAcknowledgementError` |
| `ProcessControlResumeErrors` | `ProcessControlResumeError` |
| `ProcessControlErrors` | `ProcessControlUnavailableError`, `ProcessControlError`, `ProcessControlAcknowledgementError`, `ProcessControlResumeError`, `AllocationFailureError` |

### Terminal session and typed input

| Set | Members |
|---|---|
| `TerminalSessionOpenErrors` | `TerminalSessionOptionsError`, `TerminalSessionOpenError`, `AllocationFailureError` |
| `TerminalSessionReadErrors` | `TerminalSessionReadError`, `TerminalSessionStateError` |
| `TerminalSessionWriteErrors` | `TerminalSessionWriteError`, `TerminalSessionStateError` |
| `TerminalSessionCursorErrors` | `TerminalSessionWriteError`, `TerminalSessionStateError`, `InvalidCursorPositionError` |
| `TerminalSessionRestoreErrors` | `TerminalSessionRestoreError` |
| `TerminalSessionPauseResumeErrors` | `TerminalSessionOpenError`, `TerminalSessionRestoreError`, `TerminalSessionStateError` |
| `TerminalSessionLifecycleErrors` | `TerminalSessionOptionsError`, `TerminalSessionOpenError`, `TerminalSessionRestoreError`, `TerminalSessionStateError`, `AllocationFailureError` |
| `TerminalSessionRenderErrors` | `TerminalSessionWriteError`, `TerminalSessionStateError`, `InvalidCursorPositionError`, `TerminalTextLayoutError`, `AllocationFailureError` |
| `TerminalSessionErrors` | `TerminalSessionOptionsError`, `TerminalSessionOpenError`, `TerminalSessionReadError`, `TerminalSessionWriteError`, `TerminalSessionStateError`, `TerminalSessionRestoreError`, `InvalidCursorPositionError`, `AllocationFailureError` |
| `TerminalTestingErrors` | `TerminalTestFactoryError`, `ConstraintViolationError`, `AllocationFailureError` |

Most existing terminal test projects that repeat
`TerminalSessionOpenError, TerminalSessionReadError, TerminalSessionWriteError,
TerminalSessionStateError, TerminalSessionRestoreError, AllocationFailureError`
should become `errors TerminalSessionErrors`, unless a narrower lifecycle/read/write
set applies.

### Terminal chords

| Set | Members |
|---|---|
| `TerminalChordValidationErrors` | `TerminalChordValidationError`, `AllocationFailureError` |
| `TerminalChordMutationErrors` | `TerminalChordMutationError`, `AllocationFailureError` |
| `TerminalChordReplaceErrors` | `TerminalChordValidationError`, `TerminalChordMutationError`, `AllocationFailureError` |
| `TerminalChordProcessErrors` | `TerminalChordProcessError`, `AllocationFailureError` |
| `TerminalChordReleasedInputErrors` | `IndexOutOfBoundsError` |
| `TerminalChordRouterErrors` | `TerminalChordValidationError`, `TerminalChordMutationError`, `TerminalChordProcessError`, `IndexOutOfBoundsError`, `ConstraintViolationError`, `AllocationFailureError` |

### Error attachments and inspection

| Set | Members |
|---|---|
| `ErrorCauseInspectionErrors` | `ErrorAttachmentAbsentError` |
| `ErrorSuppressedInspectionErrors` | `IndexOutOfBoundsError` |
| `ErrorInspectionErrors` | `ErrorAttachmentAbsentError`, `IndexOutOfBoundsError` |

`Error` itself is an erased inspection value, not a propagatable named error set.
`WindowsError` appears in terminal proposal ABI records as a variant/payload name,
not as a top-level leaf error family for `errors` clauses.

## Examples

### Filesystem markdown roundtrip

Before:

```opal
entry main = f(args: string[]): void errors FileNotFoundError, FilesystemPathError, ReadFailureError, IsADirectoryError, InvalidUtf8Error, WriteFailureError, FilesystemFullError, IndexOutOfBoundsError, AllocationFailureError =>
    # ...
```

After:

```opal
import type FilesystemReadErrors, FilesystemOverwriteErrors, IndexAccessErrors, StringJoinErrors from standard.errors

entry main = f(args: string[]): void errors FilesystemReadErrors, FilesystemOverwriteErrors, IndexAccessErrors, StringJoinErrors =>
    # ...
```

If the body never calls `read_bytes_at_offset_sync`, the broader
`FilesystemReadErrors` may include `OffsetOutOfRangeError` unnecessarily. The
linter should either suggest `FilesystemTextReadErrors` or the exact leaves.

### Game of Life rendering

```opal
import type RenderSetupErrors, RenderErrors, TimeErrors, StringBuilderErrors from standard.errors

entry main = f(args: string[]): void errors RenderSetupErrors, RenderErrors, TimeErrors, StringBuilderErrors =>
    propagate prepare_display()
    propagate write_frame(width, height, board, generation)
    propagate frame_clock_wait_next_sync(clock)
    return void
```

A render helper that only builds strings should use `StringBuilderErrors`, not
`RenderErrors`.

### Simple terminal editor

```opal
import type FilesystemReadErrors, FilesystemWriteErrors, IndexAccessErrors, AllocationErrors, StringRangeErrors, TerminalTextLayoutErrors, TerminalSessionErrors, TerminalSessionRenderErrors, EnvironmentProbeErrors from standard.errors

entry main = f(args: string[]): void errors FilesystemReadErrors, FilesystemWriteErrors, IndexAccessErrors, AllocationErrors, StringRangeErrors, TerminalTextLayoutErrors, TerminalSessionErrors, TerminalSessionRenderErrors, EnvironmentProbeErrors =>
    # ...
```

During implementation, this project and every other project under
`test-projects/` should be rewritten to use the proper aliases. These rewrites are
not cosmetic only; they become fixture coverage for imports, docs, lints, and
cross-module exported metadata.

### Application-specific alias

```opal
# app_errors.types.op
import type FilesystemReadErrors, FilesystemWriteErrors, RenderErrors from standard.errors

public error set AppStartupErrors =
    FilesystemReadErrors,
    FilesystemWriteErrors,
    ParseError,
    RenderErrors
```

```opal
# main.op
import type AppStartupErrors from ./app_errors.types

entry main = f(args: string[]): void errors AppStartupErrors =>
    let config = propagate load_config()
    propagate render_intro(config)
    return void
```

The guard at a caller still sees leaves such as `FileNotFoundError`, `ParseError`,
and `WriteFailureError`.

## Implementation plan

Use TDD with red-green-refactor for every compiler slice. Do not batch large
compiler changes into one commit; each commit should leave the repository green.

1. Add parser tests for `error set` declarations. Start red with a minimal
   `test-projects/error-set-declaration-basic` fixture.
2. Add AST nodes and formatter/doc-generator round trips.
3. Add resolver tests for public/private aliases, existing `import type` imports,
   unknown members, duplicate members, nested aliases, and cycles.
4. Add canonical expansion in the type checker. Red tests should cover
   propagation through aliases, leaf/set equivalence, function compatibility, and
   guard binding leaf knowledge.
5. Add Miette diagnostics for invalid declarations.
6. Add lint passes and Miette warnings for collapsible leaf lists, broad sets with
   unused members, redundant leaf+set combinations, and overlapping sets.
7. Add `standard.errors` declarations for the catalog above.
8. Rewrite all `test-projects/` error clauses to their proper aliases. Each
   rewrite group should be an atomic commit by area: filesystem, terminal session,
   string/bytes, process, and application-specific combinations.
9. Update `STDLIB.md`, `OPALESCENT_CRASH_COURSE.md`, LSP hover tests, generated
   docs tests, and formatter tests.
10. Run the full pre-commit suite without modifying or bypassing the hook.
11. Perform a code review focused on canonical-set correctness, maintainability,
    public API compatibility, and diagnostic clarity.

## Required tests

- Parsing: declaration, multiline member list, trailing comma, empty list.
- Resolution: existing `import type` of error sets, unknown alias, unknown leaf,
  non-error member, duplicate member, private alias export leak, same-name
  conflicts, cyclic aliases.
- Type checking: propagation with set, propagation with leaf, nested set expansion,
  mixed leaf/set clauses, redundant members, guard binding narrowing.
- Lints: exact explicit list collapses to set, broad set unused member, no warning
  for singleton aliases, no warning on ambiguous same-expansion aliases, exact
  narrower-set suggestion, exact leaf-list fallback suggestion.
- Docs: generated signatures include source spelling and expanded leaves.
- LSP: hover shows `RenderErrors = ...`; quick fix can replace a leaf list.
- Formatter: stable formatting for `.types.op` declarations.
- Test projects: every rewritten fixture still checks/builds/runs as appropriate.

## Open questions

- Should v1 permit `type Name = errors ...` as a compatibility spelling for the
  older error-declaration proposal, or should it only accept `error set Name = ...`?
- Should broad public compatibility contracts have a first-class lint suppression
  comment in v1, or wait until the general lint configuration exists?
- Should standard-library aliases use plural names exclusively while retaining
  singular legacy aliases as deprecated compatibility names?
- Should alias expansion order be alphabetical, declaration-order stable, or a
  compiler-internal deterministic order with source-order docs?

## Recommendation

Adopt named error sets with the dedicated `error set` declaration syntax, ship the
standard alias catalog, and make the linter actively guide projects toward exact
sets. This preserves Opalescent's explicit error model while making real files
shorter and easier to maintain.
