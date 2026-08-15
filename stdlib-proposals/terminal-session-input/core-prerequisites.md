# Future Terminal Core Prerequisites

## Status and authority

This document is the terminal-session proposal's normative **future adoption prerequisite** contract. None of its facilities is implemented by the current compiler, runtime, standard library, test runner, or `STDLIB.md` surface. It does not change any existing signature or behavior today.

The future terminal proposal may adopt these facilities only after their language, core, standard-library, and test-runner support exists. `TerminalCoordinatorState` and `TerminalOperation` are terminal-owned declarations: their definitions and any ABI IDs belong only to `typed_event_session.types.op`. Standard error variants may import them as immutable payload types, but this prerequisite never redeclares, allocates, or assigns IDs to them. Genuinely core/system-owned wait, timer, process-control, generic error, and test-runner facilities have no terminal ABI IDs and must not be declared in terminal ABI history.

## 1. Legacy standard-input and standard-output coordination

### 1.1 Terminal payload imports and standard-owned errors

`TerminalCoordinatorState` and `TerminalOperation` are terminal-owned types imported from `typed_event_session.types.op`. The selected `TerminalOperation` carries the required legacy classifications: `TakeInput`, `PrintText`, `FlushStandardOutput`, `StdoutWriter`, `WriterWrite`, `WriterFlush`, `StdoutTerminal`, `TerminalSupportsAnsi`, `TerminalClearScreenOn`, `TerminalMoveCursorOn`, `TerminalDrawRows`, `TerminalClearScreen`, and `TerminalMoveCursor`. This prerequisite neither redeclares either type nor allocates terminal IDs.

The declarations below are future **standard-owned** error carriers. Their immutable `state` and `operation` payload fields refer to the imported terminal-owned types only; this does not transfer terminal ABI ownership.

```opal
# public type StandardInputReadError:
#     TerminalCoordinatorUnavailable:
#         state: TerminalCoordinatorState
#         operation: TerminalOperation
#     EndOfInput
#     ReadFailure
#
# public type StandardOutputHandleError:
#     TerminalCoordinatorUnavailable:
#         state: TerminalCoordinatorState
#         operation: TerminalOperation
#
# public type StandardOutputCapabilityError:
#     TerminalCoordinatorUnavailable:
#         state: TerminalCoordinatorState
#         operation: TerminalOperation
#
# public type WriteFailureError:
#     TerminalCoordinatorUnavailable:
#         state: TerminalCoordinatorState
#         operation: TerminalOperation
#
# public type FlushFailureError:
#     TerminalCoordinatorUnavailable:
#         state: TerminalCoordinatorState
#         operation: TerminalOperation
#
# public type TerminalWriteFailureError:
#     TerminalCoordinatorUnavailable:
#         state: TerminalCoordinatorState
#         operation: TerminalOperation
```

The future replacement signature for the existing `take_input` is:

```opal
# take_input(): string errors StandardInputReadError
```

When the coordinator state is not `Free`, `take_input` fails with `StandardInputReadError.TerminalCoordinatorUnavailable`. Rejection happens before any standard-input read, buffering, EOF observation, decoder update, or byte consumption. The failed call changes neither the coordinator generation nor terminal/session state.

A future coordinator-bound `StdoutWriter` and `StdoutTerminal` carry an opaque lease bound to a hidden, never-reused coordinator lease epoch. On every `Free` to `Opening` transition, the coordinator atomically replaces the epoch before publishing the reservation. An open failure, later close, recovery, or return to `Free` never restores an earlier epoch. Consequently, a handle acquired before an opening attempt remains stale forever after that attempt, including after the coordinator returns to `Free`; only a newly acquired lease for the current `Free` epoch can be used. Before every legacy inspection or mutation, the operation validates both the hidden lease epoch and the allowed coordinator state before touching stdout, terminal mode, a buffer, cursor state, readiness, or capability state. A mismatch, stale epoch, or disallowed state returns the operation's existing error family with `TerminalCoordinatorUnavailable { state, operation }` as its new future variant. Such a failure performs no output mutation, flush, cursor operation, terminal-mode change, handle refresh, epoch advance, or generation advance. The epoch is neither numeric nor publicly inspectable and exposes no session provenance or OS handle.

### 1.2 Complete existing STDLIB inventory

The following existing APIs are all covered by this future coordination rule. This list is an accounting inventory, not a claim that any signature has changed.

| Existing API | Future coordination rule |
|---|---|
| `take_input(): string` | Becomes the fallible signature above. Rejection consumes no input. |
| `print(value): void` | See the diagnostic-lane rule below. It never obtains a raw writer or terminal lease. |
| `println(text: string): void` | See the diagnostic-lane rule below. It never obtains a raw writer or terminal lease. |
| `print_text_sync(text: string): void errors WriteFailureError, SinkClosedError` | Validate generation before write. Add only the future coordinator-unavailable error variant to its existing family. |
| `flush_standard_output_sync(): void errors FlushFailureError, SinkClosedError` | Validate generation before flush. |
| `stdout_writer(): StdoutWriter` | Validate before issuing or refreshing a legacy lease. A rejected call returns a future fallible coordinator-unavailable result without issuing a handle. |
| `writer_write_sync(writer: StdoutWriter, text: string): void errors WriteFailureError, SinkClosedError` | Validate the lease generation before write. |
| `writer_flush_sync(writer: StdoutWriter): void errors FlushFailureError, SinkClosedError` | Validate the lease generation before flush. |
| `stdout_terminal(): StdoutTerminal` | Validate before issuing or refreshing a legacy lease. A rejected call returns a future fallible coordinator-unavailable result without issuing a handle. |
| `terminal_supports_ansi(terminal: StdoutTerminal): boolean` | Validate the lease before inspection. A stale or disallowed lease fails through its future coordinator-aware result rather than reporting a fabricated capability. |
| `terminal_clear_screen_on_sync(terminal: StdoutTerminal): void errors TerminalWriteFailureError, SinkClosedError` | Validate before mutation. |
| `terminal_move_cursor_on_sync(terminal: StdoutTerminal, row: int32, column: int32): void errors TerminalWriteFailureError, InvalidCursorPositionError, SinkClosedError` | Validate before mutation, including before cursor-position work that could touch the terminal. |
| `terminal_draw_rows_sync(terminal: StdoutTerminal, rows: string[]): void errors TerminalWriteFailureError, SinkClosedError` | Validate before mutation. |
| `terminal_clear_screen_sync(): void errors TerminalWriteFailureError, SinkClosedError` | Validate before mutation. |
| `terminal_move_cursor_sync(row: int32, column: int32): void errors TerminalWriteFailureError, InvalidCursorPositionError, SinkClosedError` | Validate before mutation. |

The exact future signature shape for lease creation and capability inspection is intentionally fallible and uses coherent standard-owned error families rather than a payload-only pseudo-type:

```opal
# stdout_writer(): StdoutWriter errors StandardOutputHandleError
# stdout_terminal(): StdoutTerminal errors StandardOutputHandleError
# terminal_supports_ansi(terminal: StdoutTerminal): boolean errors StandardOutputCapabilityError
```

For existing write operations, `WriteFailureError.TerminalCoordinatorUnavailable` is the standard-owned future variant. For flush operations, it is `FlushFailureError.TerminalCoordinatorUnavailable`; for terminal operations, it is `TerminalWriteFailureError.TerminalCoordinatorUnavailable`. Each carries terminal-owned `state` and `operation` payloads. No unrelated standard-library operation gains a coordinator error.

### 1.3 Legacy I/O state matrix

`Free` is the sole coordinator state with no process-owned terminal ledger and therefore the sole state permitting ordinary legacy input and fallible raw output. A directly closed session binding can remain inspectable as `TerminalSessionState.Closed`, but it retains no coordinator ownership and therefore corresponds to coordinator `Free`. `Opening` includes reservation and rollback before completion. `FailedOpenRecovery` and `FailedCloseRecovery` are process-owned recovery states, not session aliases.

| Coordinator state | `take_input` | Fallible legacy output and handle operations | `print` / `println` |
|---|---|---|---|
| `Free` | Read under its normal future fallible contract. | Validate and perform the requested operation. | Best-effort bounded escaped diagnostic lane. |
| `Opening` | `TerminalCoordinatorUnavailable`; consume no bytes. | `TerminalCoordinatorUnavailable`; no mutation. | Drop, no raw stdout touch. |
| `Active` | `TerminalCoordinatorUnavailable`; consume no bytes. | `TerminalCoordinatorUnavailable`; no mutation. | Best-effort bounded escaped diagnostic lane only. |
| `Paused` | `TerminalCoordinatorUnavailable`; consume no bytes. | `TerminalCoordinatorUnavailable`; no mutation. | Best-effort bounded escaped diagnostic lane only. |
| `RestorePending` | `TerminalCoordinatorUnavailable`; consume no bytes. | `TerminalCoordinatorUnavailable`; no mutation. | Drop, no raw stdout touch. |
| `FailedOpenRecovery` | `TerminalCoordinatorUnavailable`; consume no bytes. | `TerminalCoordinatorUnavailable`; no mutation. | Drop, no raw stdout touch. |
| `FailedCloseRecovery` | `TerminalCoordinatorUnavailable`; consume no bytes. | `TerminalCoordinatorUnavailable`; no mutation. | Drop, no raw stdout touch. |

The diagnostic lane accepts only displayable application diagnostics. It escapes C0 and C1 controls, ESC, DEL, bidi controls, noncharacters, and invalid display scalars; bounds its output; and adds a visible truncation marker. It gives no delivery, ordering, flush, atomicity, or preservation guarantee. In rows marked drop, it writes nothing and performs no buffering that could later reach stdout.

## 2. Generic readiness and one affine wait set

The following are future core/system declarations. `SystemWaitSetError` is a core/system-owned error family and receives no terminal ABI ID or terminal ABI-history entry.

```opal
# public type SystemWaitSetError:
#     WrongSet
#     UnauthenticatedRegistration
#     RegistrationLifetimeInvalid
#
# @constructor_visibility(runtime)
# public type SystemWaitWake:
#     Ready:
#         source: SystemReadinessSource
#         generation: uint64
#     Cancelled
#
# @constructor_visibility(runtime)
# public compiler_registered affine resource type SystemOwnedWaitRegistration
#
# system_wait_set_new(): SystemWaitSet errors AllocationFailureError
# system_wait_set_register(mutable ref wait_set: SystemWaitSet, source: SystemReadinessSource): SystemWaitRegistration errors SystemWaitSetError, AllocationFailureError
# system_wait_set_remove(mutable ref wait_set: SystemWaitSet, registration: SystemWaitRegistration): void errors SystemWaitSetError
# system_wait_set_register_owned(mutable ref wait_set: SystemWaitSet, source: SystemReadinessSource): SystemOwnedWaitRegistration errors SystemWaitSetError, AllocationFailureError
# system_owned_wait_registration_retarget(mutable ref registration: SystemOwnedWaitRegistration, source: SystemReadinessSource): void errors SystemWaitSetError, AllocationFailureError
# system_owned_wait_registration_remove(mutable ref registration: SystemOwnedWaitRegistration): void errors SystemWaitSetError
# system_wait_set_wait_sync(mutable ref wait_set: SystemWaitSet, cancellation: CancellationToken): SystemWaitWake errors SystemWaitSetError
```

`SystemWaitSet` is affine, noncopyable, and non-publicly constructible except through `system_wait_set_new`. A sealed opaque registration belongs to exactly one live wait set and exactly one source identity. Registering the same source twice produces distinct registrations but one source identity. The first removal of a live ordinary `SystemWaitRegistration` by its owning live wait set succeeds and releases exactly that registration reference. Repeating removal of that exact already-removed ordinary registration through that same still-live owning set is idempotent success: it releases no further reference, does not change source readiness or generation, and does not mutate registration tables. An ordinary registration presented to a different set fails with `SystemWaitSetError.WrongSet`; an unauthentic registration fails with `SystemWaitSetError.UnauthenticatedRegistration`; and a registration whose owning set or required lifetime is invalid fails with `SystemWaitSetError.RegistrationLifetimeInvalid`. Each failure performs no mutation.

`SystemOwnedWaitRegistration` is a sealed opaque affine authority for exactly one wait-set entry. `system_wait_set_register_owned` authenticates the live set and source, preallocates the entry, authority cell, and source-retention metadata, then atomically installs the entry and returns its sole owner. It stores core/runtime-owned set identity and lifetime metadata, never a language `ref` or `mutable ref`. `SystemWaitSetError.RegistrationLifetimeInvalid` is this constructor's sole reachable wait-set-family variant and applies only when the source's required lifetime is already invalid; `WrongSet` and `UnauthenticatedRegistration` are unreachable because no registration is supplied. That error or `AllocationFailureError` occurs before entry insertion, source retention, fairness-order mutation, or authority publication. The returned owner can therefore outlive the constructor borrow while authorizing only its exact entry; it cannot create another entry or expose the set, source internals, registration key, or host handle.

`system_owned_wait_registration_remove` authenticates the authority and its live owning set, then atomically removes its entry, releases that entry's source reference exactly once, and marks the authority removed. Repeating removal through the same authentic removed authority while its owning set remains live is idempotent success and performs no mutation. Removing an unauthentic authority returns `UnauthenticatedRegistration`; removing one whose set has been destroyed returns `RegistrationLifetimeInvalid`. Those are its only reachable `SystemWaitSetError` variants; `WrongSet` is unreachable because the sealed authority already names its sole set. Both failures are non-mutating. No ordinary registration operation accepts `SystemOwnedWaitRegistration`, and no owned-registration operation accepts `SystemWaitRegistration`.

`system_owned_wait_registration_retarget` authenticates one live owned authority and one new source, then preallocates every source-retention and wait-entry update needed by the commit. Retargeting to the already-targeted source is idempotent success with no allocation, retain, release, readiness, generation, or fairness-order change. An unauthentic authority returns `UnauthenticatedRegistration`; an invalid owning-set or new-source lifetime returns `RegistrationLifetimeInvalid`; and `WrongSet` is unreachable because no set argument can disagree with the sealed authority. Those are its only reachable `SystemWaitSetError` variants. Either such error or `AllocationFailureError` leaves the authority, old registered source reference, entry readiness, owning set, and fairness position exactly unchanged and retains no new source reference. Success atomically retains the new source once, changes only that entry's target, releases the old source reference once, and updates the authority to name the new relationship. The entry keeps its exact registration-order/fairness position; retarget never removes and appends it, changes either source identity/generation, or consumes a readiness event. A Ready observation already published for the old source remains a stale hint that callers reject by source identity; if the new source is already level-ready, the unchanged entry becomes observable under the ordinary wait rules.

Destroying a wait set first invalidates every ordinary and owned registration authority for that set, then removes every still-live entry in reverse registration order and releases each retained source reference exactly once. A previously removed entry is not released again. A later explicit remove or retarget through an owned authority returns `RegistrationLifetimeInvalid` without mutation. Its compiler-only cleanup observes the invalidated cell and becomes an infallible no-op, so wait-set-first and registration-first cleanup orders cannot double-remove an entry or double-release a source.

`SystemReadinessSource` is a cloneable, host-stable opaque identity. A source retains its host wake state while held by a source clone or a live wait-set registration. Removing or cleaning a registration and successful retarget away from a source each release that entry's old source reference exactly once, while source clones remain valid independently according to their source contract. A source never exposes an OS descriptor, HANDLE, registration key, or mutable host state.

`SystemWaitWake` is a sealed core/system-owned sum with no terminal ABI ID or terminal ABI-history entry. `Ready` alone carries a source identity and observed generation. `Cancelled` carries neither a source identity nor an observed generation, and no inspector can fabricate either value. A `Ready` wake is a hint, not a successful operation: a source may become stale after the wait reports it because another consumer drained the condition, because its generation changed, or because a transition woke the set. Callers must refine `Ready` before reading its fields and reattempt the source-specific Poll operation. That operation reports its ordinary typed stale result, such as terminal `TimedOut` or process-control `Idle`, without pretending that the wait was incorrect.

A source whose level condition remains observable stays ready; it does not require a new edge before a later `system_wait_set_wait_sync` can report `Ready` again. Selection among simultaneously ready registrations is bounded-fair in registration order: after returning one `Ready`, the next wait begins selection after that registration, wrapping once, so every continuously ready live registration is returned within at most the number of live registrations unless the higher-priority `Cancelled` outcome applies. Removing or adding a registration removes or appends one fairness position without changing any source identity. Successful owned-registration retarget preserves the existing position and wait cursor exactly; it substitutes only the source tested at that position. This fairness is generic wait-set behavior, not terminal batching or a scheduler.

The following are future core/system cancellation declarations with no terminal ABI IDs:

```opal
# cancellation_source_new(): CancellationSource errors AllocationFailureError
# cancellation_token(ref source: CancellationSource): CancellationToken
# cancellation_request(mutable ref source: CancellationSource): void
```

`CancellationSource` is affine request authority. Creation allocates one new, never-reset cancellation generation; `cancellation_token` returns an immutable observation token for that exact source generation; and `cancellation_request` is synchronized and idempotent for that generation. Cancellation is sticky per source generation. Dropping the source permanently removes request authority but does not invalidate already-issued tokens: they retain their observation and wake state until the final token drops. Creating a new source creates a distinct generation that an old token cannot affect. These source and token lifetimes are core/system-owned and expose neither terminal ABI IDs nor terminal provenance.

After already-published queued source work, a simultaneously ready cancelled token wins over new source consumption. The wait returns `SystemWaitWake.Cancelled`, carries no source or generation, and consumes no newly ready source work. Transition wakes are mandatory: a source changing observable availability, including terminal `TerminalSessionState.Active` to `Paused`, `Paused` to `Active`, `Active` or `Paused` to `RestorePending`, `RestorePending` to `TerminalSessionState.Closed`, or `Active` or `Paused` to `TerminalSessionState.Closed`, wakes all sets currently registered for that source.

## 3. Affine monotonic timers

`MonotonicTimerError` and `MonotonicTimerNotArmedError` are core/system-owned nominal error families with no terminal ABI IDs or terminal ABI-history entries.

```opal
# public type MonotonicTimerError:
#     GenerationExhausted:
#         last_issued_generation: uint64
#
# public type MonotonicTimerNotArmedError:
#     NotArmed
#
# monotonic_timer_new(): MonotonicTimer errors AllocationFailureError
# monotonic_timer_readiness_source(ref timer: MonotonicTimer): SystemReadinessSource
# monotonic_timer_arm(mutable ref timer: MonotonicTimer, deadline: MonotonicDeadline): uint64 errors MonotonicTimerError
# monotonic_timer_disarm(mutable ref timer: MonotonicTimer): uint64 errors MonotonicTimerError
# monotonic_timer_generation(ref timer: MonotonicTimer): uint64
# monotonic_timer_deadline(ref timer: MonotonicTimer): MonotonicDeadline errors MonotonicTimerNotArmedError
# monotonic_clock_now(): MonotonicDeadline
```

`MonotonicTimer` is affine. Its readiness source has one stable identity for the timer lifetime, including disarm and rearm. Each successful arm or disarm advances a never-reused generation before publishing the new state. Disarming an armed timer clears its deadline and advances the generation. Disarming an already-disarmed timer remains successfully disarmed and still advances a new generation, so defensive repeated disarm invalidates every previously published wake without reusing a generation.

Both `monotonic_timer_arm` and `monotonic_timer_disarm` preflight the next generation before changing the deadline, armed/disarmed state, readiness, or generation. If no next generation exists without wrap or reuse, either operation returns `MonotonicTimerError.GenerationExhausted { last_issued_generation }`. The payload is the current valid generation, which remains unchanged and is never a sentinel. An armed timer remains armed with the exact prior deadline and readiness state; an already-disarmed timer remains disarmed with no deadline; and a failed disarm of an armed timer likewise leaves it armed. The stable readiness-source identity and all previously published wake observations remain unchanged. The caller may continue inspecting and using that exact prior timer state, but every same-state retry that still requires a new generation returns the same variant and payload until the timer is dropped; exhaustion never wraps, resets, or reuses a generation.

`monotonic_timer_deadline` returns `MonotonicTimerNotArmedError.NotArmed` while disarmed and returns the unchanged prior deadline after any failed operation that leaves the timer armed; no absent-deadline value or sentinel exists. `monotonic_timer_arm` is ready when `monotonic_clock_now()` is equal to or later than its deadline. Equality is expiration, not a one-tick delay.

A `SystemWaitWake.Ready` carries a source identity and observed generation. A caller must compare its generation with `monotonic_timer_generation`; an old armed deadline, a disarmed timer, or a newer arm makes that Ready wake stale. A stale timer wake performs no timer mutation and the caller returns to the wait set. A timer arm has no hidden scheduler, polling loop, or terminal-specific identity.

## 4. Separate process-control source

`ProcessControlUnavailableError`, `ProcessControlError`, `ProcessControlAcknowledgementError`, and `ProcessControlResumeError` are distinct core/system-owned nominal error families with no terminal ABI IDs or terminal ABI-history entries.

```opal
# public type ProcessControlUnavailableError:
#     UnsupportedHost
#
# public type ProcessControlError:
#     HostNotificationObservationFailed
#     GenerationExhausted:
#         last_issued_generation: uint64
#
# public type ProcessControlAcknowledgementError:
#     WrongGeneration
#     StaleGeneration
#     HostSuspendFailed
#
# public type ProcessControlResumeError:
#     WrongGeneration
#     StaleGeneration
#     HostApplicationResumeFailed
#
# process_control_source_new(): ProcessControlSource errors ProcessControlUnavailableError, AllocationFailureError
# process_control_readiness_source(ref source: ProcessControlSource): SystemReadinessSource
# @constructor_visibility(runtime)
# public type ProcessControlPollResult:
#     Notification:
#         notification: ProcessControlNotification
#     Idle
#
# process_control_poll(mutable ref source: ProcessControlSource): ProcessControlPollResult errors ProcessControlError
# process_control_acknowledge_suspend(mutable ref source: ProcessControlSource, generation: uint64): void errors ProcessControlAcknowledgementError
# process_control_resume_application(mutable ref source: ProcessControlSource, generation: uint64): void errors ProcessControlResumeError
```

`ProcessControlNotification` has exactly `SuspendRequested(generation)` and `Continued(generation)` for this contract. `ProcessControlPollResult` is a sealed core/system-owned sum with exactly `Notification { notification }` and payloadless `Idle`; neither type receives a terminal ABI ID or terminal ABI-history entry. Process control is a distinct generic source, never a terminal input event. On supported POSIX hosts it observes catchable job-control suspension and continuation. The contract has no Windows console-control equivalent: `process_control_source_new` returns `ProcessControlUnavailableError.UnsupportedHost` before allocating a source, readiness identity, queue, or generation state. Unsupported-host construction never returns `ProcessControlError`.

`process_control_poll` is nonblocking. It first returns `Notification` containing the oldest already-queued notification without consulting the host. Returning that result is the sole operation that removes that queue entry; polling never acknowledges, suspends, resumes, or advances an existing notification's generation. Only an empty queue permits one nonblocking host observation. If no host indication is currently observable, including after stale readiness, poll MUST return `Idle`. `Idle` consumes or acknowledges no host indication, creates or replaces no process-control state, enqueues or removes no notification, changes no readiness or generation state, and affects no terminal/application state; poll itself retains the exact source state visible at call entry, while any concurrent publication remains independently queued. `Idle` is not an error and cannot fabricate a notification.

A successfully observed host indication is converted under the state rules below and appended after every notification already published concurrently. Poll then returns `Notification` containing the oldest queue entry and removes only that entry. Thus an older or concurrently published queue entry always precedes the newly observed indication, while a newly observed entry remains queued if another entry wins that ordering. Host observation never bypasses the queue or returns a notification that was not first published there.

If the host observation fails, poll returns `ProcessControlError.HostNotificationObservationFailed` before consuming a host indication, creating/replacing a process-control state, enqueueing or removing a notification, changing readiness, or advancing a generation. The source state, queue contents, and generation are exactly those visible at call entry. Consequently a queued notification cannot be dropped by this failure: if one was queued at entry, poll returns it instead of failing; if one becomes queued concurrently, it remains queued, and a same-generation retry observes that same oldest notification. With no concurrent publication, retry observes the same empty queue and same source generation before attempting host observation again.

When an observed suspend request requires a new generation, poll preflights generation capacity before consuming the host indication, creating `SuspendPending`, enqueueing `SuspendRequested`, changing readiness, or advancing the generation. If no nonzero next generation exists without wrap or reuse, poll returns `ProcessControlError.GenerationExhausted { last_issued_generation }`. The current state, every older queued notification, and the last issued generation remain unchanged; the triggering host indication remains pending and unacknowledged. Older queued notifications still return first. After they drain, every retry against that still-pending indication returns the same exhaustion variant and payload. No retry can drop, duplicate, replace, acknowledge, or assign a wrapped generation to it. Continuation of an existing generation never allocates a new generation and therefore cannot emit this variant.

These are the only `ProcessControlError` variants reachable from `process_control_poll`. Neither variant consumes or acknowledges a queued notification, changes one of the process-control states below, advances a generation, or affects terminal/application state. A retry therefore starts from the exact retained source state and observes the same queued notification, pending host indication, and generation ordering described above.

Each process-control generation has exactly one of these core/system states: `SuspendPending`, `AcknowledgedHostSuspended`, `ContinuedAwaitingApplicationResume`, or `Completed`. Publishing `SuspendRequested(generation)` immediately creates or exposes `SuspendPending`. Repeated requests before acknowledgement coalesce to that same pending generation. Before calling `process_control_acknowledge_suspend` for the correct pending generation, the caller must stop application work, complete terminal pause, and process the final `PauseBoundary`; `ProcessControlSource` neither observes nor validates those terminal/application conditions. Acknowledgement then requests host suspension. On success it enters `AcknowledgedHostSuspended`; a same-generation duplicate acknowledgement in that state is an idempotent no-op. On `HostSuspendFailed`, the already-existing generation remains `SuspendPending` and is retryable: no host-suspension, terminal-session, or application-resume state advances. Wrong or stale generations return the declared structured acknowledgement error before mutation.

`Continued(generation)` moves only the matching acknowledged generation to `ContinuedAwaitingApplicationResume`. It marks readiness only and does not restore terminal state, resume a terminal session, or resume application work. The application explicitly resumes its terminal session, then calls `process_control_resume_application` for the matching generation. Success enters `Completed`; a same-generation duplicate resume in `Completed` is an idempotent no-op. On `HostApplicationResumeFailed`, the generation remains `ContinuedAwaitingApplicationResume` and is retryable while application work remains stopped. Wrong or stale generations return the declared structured resume error before mutation. Stale, duplicate, mismatched, or out-of-order notifications do not acknowledge, suspend, resume, or consume terminal input. Terminal and process-control sources remain separate, and Windows remains unavailable under this contract.

## 5. `using` and affine cleanup

The following future language rule defines `using` for affine resources. It is a prerequisite, not current parser or checker behavior.

Every affine resource declaration admitted to `using` MUST name exactly one cleanup operation and that operation's nominal declared cleanup error-family set, including an explicit empty set when cleanup is infallible. A cleanup operation takes the owned resource through compiler-held cleanup authority; it may be a public consuming operation or an inaccessible compiler/runtime operation, but its success, retryable failure, and any cleanup-authority transfer result are part of the resource declaration. A resource declaration also lists every operation, if any, permitted to transfer cleanup authority and the exact result on which transfer occurs, including an explicit empty transfer set when none exists. No convention, ordinary move, return type, error payload, or example comment implies transfer.

The authoritative proposal or declaration that owns an affine resource also owns this compiler-checkable cleanup registration. A core-owned resource is registered here; a terminal-, chord-, test-, or application-owned resource may be registered in its authoritative declaring proposal, but adoption MUST collect those registrations into one compiler-visible resource registry and reject a missing, duplicate, or contradictory registration. Moving a registration to a companion authority does not weaken the exact operation, nominal error-effect, transfer-result, or owner/borrow checks below.

The core-owned registrations and selected `TerminalSession` cross-reference used by the selected examples are exact:

| Affine resource | Declared cleanup operation | Declared cleanup errors | Declared cleanup-authority transfer |
|---|---|---|---|
| `SystemWaitSet` | compiler-only `system_wait_set_drop` | none | none |
| `SystemOwnedWaitRegistration` | compiler-only `system_owned_wait_registration_drop` | none | none |
| `ProcessControlSource` | compiler-only `process_control_source_drop` | none | none |
| `MonotonicTimer` | compiler-only `monotonic_timer_drop` | none | none |
| `TerminalSession` | `terminal_session_close_sync` | `TerminalSessionRestoreError` | cleanup-only `TerminalSessionRestoreError.CloseRestorePending` transfers to the process coordinator exactly as the selected contract specifies |

The four core compiler-only drop operations are inaccessible ordinary source calls. `system_owned_wait_registration_drop` is infallible: while its authority still names a live entry it removes that entry and releases its source reference exactly once; after explicit removal or owning-set destruction it is a no-op. It never allocates, waits, retargets, changes fairness order beyond removing its own live position, or reports a lifetime error. `system_wait_set_drop` uses the invalidation/removal ordering above, so later owned-registration cleanup cannot double-remove. The other core drops synchronously release their resource's registrations/host wake ownership, perform no fallible disarm/generation advance, and have an explicit empty error effect and transfer set. The selected terminal resource registration is normative in `typed-event-session/proposal.md`; no other terminal operation or result transfers cleanup authority. `CHORDS.md`, `TESTING.md`, and the private application declarations in `configure_editor_chords.op` own the additional exact registrations for affine resources they admit to `using`; those registrations are subject to this same static rule.

### 5.1 Transactional affine aggregate construction and same-owner transitions

An affine aggregate declaration may opt into compiler-checked transactional construction. The declaration MUST name one constructor, every finite member slot and nested variant that can hold an affine resource, the required sealed state, the aggregate's normal member-cleanup order, and every permitted same-owner transition between complete slot/variant layouts. This opt-in creates no public move, no general field assignment, and no cleanup-authority transfer to another owner.

During the named constructor, the compiler creates one inaccessible unsealed aggregate shell and holds each successfully acquired member cleanup obligation in a provisional table keyed to one declared uninitialized slot. Source code cannot observe, borrow, copy, return, capture, or move the shell or a provisional member. Installing an acquisition into its declared slot removes no obligation from compiler control. If any acquisition, validation, registration, or initialization step fails before sealing, the compiler cleans every initialized provisional member exactly once in strict reverse acquisition order, treats every never-initialized slot as empty, discards non-affine partial state, destroys the unsealed shell, and returns only the original failure. Rollback neither invokes aggregate cleanup nor exposes a partial aggregate.

After every required slot and invariant is initialized, one allocation-free infallible seal atomically converts the provisional table into the aggregate owner's member-obligation table and publishes the complete aggregate as the constructor result. Seal and return are one ownership commit: no fallible call, user callback, borrow, cancellation point, observable result, or cleanup can intervene. Before seal, the compiler owns the provisional obligations; after seal, exactly the returned aggregate owner owns every listed member obligation and no provisional obligation remains. This initial formation of one previously unobservable owner is not cleanup-authority transfer between owners, so a member resource's empty external transfer set remains truthful. Any later move of a member to another aggregate or independently observable owner still requires an explicit transfer registered by that member resource.

A declared same-owner transition requires an exclusive `mutable ref` to the sealed aggregate and names the complete old and new slot/variant layouts. The compiler temporarily controls all involved member obligations while retaining the same aggregate owner. Fallible preparation occurs before the transition commit and cannot remove, duplicate, clean, or relocate an old-layout obligation; failure leaves the exact old layout and obligations unchanged and cleans only separately acquired provisional candidates in reverse acquisition order unless the declared failure state retains such a candidate. The infallible commit changes all named tags/slots and obligation keys atomically. Each old-layout obligation must occur exactly once in the new layout or be cleaned exactly once after no live registration/reference still depends on it; each provisional candidate must be installed exactly once or cleaned. No obligation may be absent, duplicated, simultaneously owned by two slots, exported through the borrow, or cleaned while another old/new slot still owns it.

Movement performed by that atomic commit is an obligation-preserving permutation inside the same aggregate owner, not cleanup-authority transfer. No ordinary field write, helper comment, return, or borrowed call receives this privilege. The aggregate authority MUST enumerate each legal transition; every undeclared layout change is a compile-time error. No observer, cleanup, scope exit, or fallible operation can interleave with a committed transition. If an external fallible operation changes a member's host relationship before the layout commit, the declaration MUST identify the exact retained pre-commit layout on failure and require one immediately following infallible matching commit on success.

Explicit aggregate close and lexical cleanup consume the same sealed aggregate obligation. They first inspect the one current declared layout, treat declared empty slots as owning nothing, and run the aggregate's exact cleanup order. Variant cleanup visits only members present in that variant; partial-construction rollback instead uses reverse acquisition order. Cleanup cannot run twice for a member moved by a same-owner transition, and an invalid, unsealed, or undeclared layout fails compilation rather than selecting a runtime fallback.

For `using resource = acquisition(): body`, acquisition is evaluated exactly once. Its static error effect is the nominal set union of acquisition errors, every error family that can leave the body, and the resource declaration's cleanup errors: `effects(using) = effects(acquisition) ∪ effects(body) ∪ cleanup_errors(resource)`. The enclosing function's `errors` clause MUST cover that complete union even when control-flow analysis can see an explicit close; cause and suppressed attachments do not add families beyond their primary operation's effect. On successful acquisition, the binding owns one cleanup obligation. Cleanup runs exactly once on fallthrough, `return`, `break`, `continue`, and failure propagation leaving the lexical scope. An explicit successful close consumes the obligation; subsequent scope cleanup is a no-op. A failed explicit close retains the binding and its cleanup obligation unless the operation and result are the resource declaration's explicit cleanup-authority transfer.

| Body outcome | Cleanup outcome | Result leaving scope |
|---|---|---|
| success | success or explicit-close no-op | Body success. |
| success | first cleanup failure | That cleanup error is primary. |
| body error | success or explicit-close no-op | Original body error. |
| body error | first cleanup failure | Cleanup error is primary, body error is its cause. |
| any prior outcome | later cleanup failure | Preserve the existing primary; attach each later cleanup error as suppressed. |

Nested `using` scopes clean up in strict reverse acquisition order. If multiple cleanup attempts fail, the innermost failing cleanup is primary. The body error follows as its cause when present. Remaining cleanup failures are suppressed in encounter order, which is outerward, and each resource's own inverse ledger is reverse-ledger order. Cleanup is attempted even after a prior cleanup failure when the resource remains safely owned.

The `using` owner is affine and scope-bound. Before its obligation is consumed or explicitly transferred, it MUST NOT be copied; moved into an ordinary argument or a new owner; assigned to an outer binding; stored in a field, collection, closure, task, generator, or module/global state; returned; or carried as a `break`, `continue`, or other scope-exit value. `return`, `break`, and `continue` may leave the scope only without carrying the owner, after which cleanup runs before control reaches the destination. A `ref` or `mutable ref` borrow of the owner is second-class: it lasts no longer than the borrowing call/full expression and MUST NOT be stored, returned, captured, moved, carried across any scope exit, or remain live when explicit close, cleanup, or a declared transfer consumes authority. A transfer operation is legal only when named by the resource declaration; its declared successful transfer consumes the local owner and cleanup obligation atomically, while every non-transfer result leaves ownership exactly as that operation declares. These restrictions apply transitively through aggregates and aliases.

## 6. Immutable error values, propagation, and attachments

```opal
# propagate error_value
# propagate error_value cause prior_error
# error_cause(error_value: Error): Error errors ErrorAttachmentAbsentError
# error_suppressed_length(error_value: Error): int64
# error_suppressed_at(error_value: Error, index: int64): Error errors IndexOutOfBoundsError
# error_attachment_truncation(error_value: Error): ErrorAttachmentTruncation
# error_attachment_truncation_cause_depth(truncation: ErrorAttachmentTruncation): boolean
# error_attachment_truncation_suppressed_count(truncation: ErrorAttachmentTruncation): boolean
# error_attachment_truncation_bytes(truncation: ErrorAttachmentTruncation): boolean
```

`propagate error_value` forwards exactly one already-evaluated immutable nominal error value. It neither calls a function nor re-evaluates an expression. The forwarded value remains the same primary value and receives no implicit wrapper, copy-visible mutation, cause, or suppression.

For a guarded expression with declared nominal families `F1, ..., Fn`, `else error_value` binds an immutable, compiler-known discriminated error union `F1 | ... | Fn`. This union is not a publicly constructible structural type: the runtime value retains exactly one original nominal family, variant, and payload. An unrefined binding can be formatted or passed to an `Error` inspector, but it cannot be passed where one specific family is required. `if error_value is Family.Variant into payload` narrows the true branch to that exact family and variant and binds only that variant's immutable payload; sibling and following control flow retain the original union unless ordinary control-flow analysis proves prior variants exited.

`propagate error_value` is legal for an unrefined guard binding only when every family in its static union is included by nominal family identity in the enclosing function's `errors` clause. In a refined branch, only the narrowed family's inclusion is required. Structural payload similarity, a shared variant name, or the erased `Error` inspector type never satisfies family inclusion. A known family in an `errors` clause covers its future additive variants. Upcasting a family/union value to immutable `Error` for inspection does not erase the binding's propagation family set and cannot manufacture a propagatable family. `propagate error_value cause prior_error` has the same family-inclusion rule, forwards `error_value` as primary, and requests retention of `prior_error` under the deterministic existing-cause rules below; it never re-evaluates either binding.

Fallible work nested inside a guard handler is checked against the enclosing function's `errors` clause independently of the guarded subject; the future rule removes today's active-guard exact-error-set restriction. `propagate nested_call()` makes a nested failure the new primary and attaches nothing implicitly. `propagate nested_error` likewise forwards that nested bound value unchanged. To preserve the handler's original bound error when nested inspection, formatting, recovery, or other fallible work fails, source MUST write `propagate nested_call() cause error_value`; if the nested call succeeds, execution continues and the original binding remains available. A nested `guard` may instead handle its own failure locally. There is no implicit preference for, wrapping of, or attachment to the active guard error.

`propagate call() cause prior_error` evaluates the call once. If it succeeds, it returns its success. If it fails, the call failure is primary and the cause request follows the deterministic existing-cause rules below. The static error effect is the call's declared nominal families; retaining `prior_error` does not add its family to that expression's effect. `propagate error_value cause prior_error` uses the same rules without re-evaluating either binding.

A cause form constructs one derived immutable propagation graph that retains the same nominal primary identity; it never mutates an existing error alias or its graph. Before publishing an attachment edge, the compiler/runtime compares immutable error identity across the complete primary and requested graphs. `prior_error` MUST be an already-evaluated immutable error in scope, MUST NOT be the primary instance, and MUST NOT contain the primary anywhere in its cause/suppressed graph. A provable violation is a compile-time error. A runtime-only identity/cycle violation propagates a derived primary graph with all existing edges and order unchanged and sets the pre-reserved cause-depth truncation marker when the request had no existing immediate cause, or the suppressed-count truncation marker when a different immediate cause would have selected the suppressed fallback; it never publishes a cyclic edge.

If the primary has no immediate cause, the request retains `prior_error` as that cause. If its immediate cause is the identical error instance, reattachment is idempotent: the complete graph, attachment order, counts, bytes, and truncation metadata remain unchanged and no allocation occurs. If the primary has a different immediate cause and the exact requested instance is not already anywhere in its graph, the existing cause remains immediate and `prior_error` is retained as the next suppressed value in encounter order. If that exact instance is already anywhere in the primary graph, the request is an idempotent no-op. Thus a cause request never silently replaces or discards an existing cause and never duplicates one immutable error instance in the same attachment graph.

Errors are immutable and acyclic. The maximum attachment limits are depth 8 for cause chains, 8 suppressed values per primary, and 64 KiB total attachment storage. Cause precedes suppressed values. A new immediate cause retains only the longest graph prefix permitted by cause-depth and byte limits; a different-existing-cause fallback retains the requested value only if suppressed-count and byte limits permit it. Omitted suffixes/values set the corresponding core-owned cause-depth, suppressed-count, or byte truncation marker. Identity and cycle checks examine the complete prospective graph even when a bound would later truncate it.

All storage needed for the new edge/reference and accounting is preflighted before publication. Allocation failure leaves the primary identity, existing cause, suppressed sequence, and all existing attachment edges unchanged in the derived result; it omits the requested attachment and sets the pre-reserved byte-truncation marker. The marker update itself requires no allocation, and existing aliases remain unchanged. Inspectors therefore observe either the complete committed bounded result, an explicitly truncated bounded result, or unchanged existing edges plus deterministic truncation metadata—never a replaced cause or partial edge.

## 7. Test-only availability and sealed runner authority

```opal
# @availability(test_only)
# test_runner_terminal_authority(): TerminalTestAuthority
```

`@availability(test_only)` is a future declaration attribute. Test-only declarations may be imported and referenced only while compiling a test artifact under the sealed test runner. Production modules, production exported signatures, production metadata, and production artifacts cannot import, name, serialize, or depend on a test-only symbol. This is a compile-time availability boundary, not a runtime boolean.

`TerminalTestAuthority` is opaque, sealed, nonconstructible, and issued only by the test runner for one test compilation. Test-only factories require an authority and enforce the same bounds and cross-field invariants as runtime construction. They may create bounded synthetic events, capabilities, diagnostics, and trusted-paste evidence only. They cannot construct a `TerminalSession`, forge a recovery token, mint coordinator leases, expose host handles, or bypass recovery provenance. Authentic recovery tokens in tests arise only through the ordinary lifecycle against a deterministic fake backend.

Test-only declarations, authorities, factories, and test artifact identities have no production ABI exports or IDs. They do not appear in terminal ABI declarations or terminal ABI history.

## Adoption checklist

Before the selected terminal proposal can adopt this contract, implementation work must separately establish the listed language syntax, checker behavior, core types, runtime synchronization, standard-library registration, and test-runner authority. Until then, existing `take_input`, legacy stdout APIs, `print`/`println`, `propagate`, and resource cleanup retain their current documented behavior.
