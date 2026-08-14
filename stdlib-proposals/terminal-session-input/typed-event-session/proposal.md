# Typed Event Session

## Status, scope, authority, and adoption dependencies

This is the normative selected public v1 terminal-session/input design. It preserves Opalescent's explicit, nominal, errors-first identity: one compiler-registered affine `TerminalSession` owns restoration responsibility; lifecycle legality is validated at runtime; every fallible operation declares errors; events, options, diagnostics, trust boundaries, recovery authority, and errors are nominal; and no platform handle enters the public API. No compiler, runtime, standard-library, or test-runner implementation is claimed by this proposal package.

V1 owns only the process interactive standard-input/raw-output terminal pair. The process coordinator admits at most one session or process-owned recovery ledger. Chord routing is a companion concern declared in `../terminal_chords.types.op` and specified by `../CHORDS.md`; it does not enlarge the session API.

Adoption of this proposal requires, and MUST occur in the same compatible release as, the future language/core/standard-library contracts specified in `../core-prerequisites.md`. Those prerequisites normatively define legacy stdin/stdout coordination, one affine generic wait set, monotonic timers, the separate process-control source, `using` cleanup, immutable error propagation and attachments, and test-only availability. This proposal references those rules and adds only terminal-specific behavior. Until those prerequisites are adopted, no signature or behavior in the current standard library is changed by this proposal.

Adoption also requires reconciliation with the authoritative `abi-history.md`. Active `.types.op` declarations are the sole authority for current terminal-owned IDs, fields, constructor visibility, evolution, ownership, and current representation annotations. `abi-history.md` is the append-only authority for retired IDs, historical representations where evidenced, retirement reasons, and permanent never-reuse records. Active and retired sets MUST be disjoint. Core, standard-library, and test-only declarations have no terminal ABI IDs. The selected declarations and history MUST be aligned atomically in the applicable declaration revision before this proposal is adopted; prose allocates no independent IDs or manifest history.

## Narrow language and core/system prerequisites

### One `is` expression production

The grammar keeps one `is` expression production with optional `into identifier`:

```opal
if event is TerminalInputEvent.Key into key_event:
    handle_key(key_event)

if left is right:
    handle_equality()
```

Type checking treats the expression as nominal variant refinement only when the right-hand expression resolves to a nominal `Type.Variant` and the left side is a direct identifier. In that case optional `into` creates an immutable branch-local payload binding. In every other case `is` remains ordinary equality. `into` is legal only for a successful refinement classification; it cannot follow equality, `is not`, a compound left side, or a payloadless variant. No separate parser production, match, exhaustiveness, exceptions, or defer semantics are introduced.

Error values follow the same rule. Refining `error is TerminalSessionOpenError.InvalidOptions into invalid` proves stable family and variant identity. An `errors A, B` clause is a nominal family set, not a structural union; unknown additive variants remain propagatable members of their known family.

### Constrained construction

Constraint predicates such as `contains_no_nul(value)`, `utf8_byte_length(value)`, and `unicode_scalar_count(value)` are compiler intrinsics usable only in `where` expressions. They are not public functions. Runtime values use:

```opal
let timeout = propagate constrain TerminalInputSequenceTimeoutMilliseconds from runtime_timeout
```

The core-owned `ConstraintViolationError` v1 payload contains exactly the stable constrained type ID and one bounded `ConstraintObservedValue`; it has no terminal-owned ABI ID. Every constrained terminal/chord type explicitly declares one `where` clause and `@abi_evolution(closed_major_only)`. Constraint weakening or tightening is ABI-major. `as ConstrainedType` is allowed only for a literal or constexpr value statically proved to satisfy the clause.

### Generic facilities adopted by reference

`CancellationSource`, `CancellationToken`, `SystemReadinessSource`, `SystemWaitSet`, `SystemWaitRegistration`, `SystemWaitWake`, `MonotonicTimer`, process-control types, `ConstraintViolationError`, `ConstraintObservedValue`, `Error`, and `ErrorAttachmentTruncation` are core/system declarations imported from `standard`; they are not terminal types and receive no terminal ABI IDs.

The exact signatures and rules in `../core-prerequisites.md` are mandatory, including:

- fallible legacy input and generation-bound legacy stdout operations with the complete coordinator state matrix and rejection-before-consumption/mutation rules;
- sticky generation-scoped cancellation and source lifetime;
- one affine wait set, stable source identities, registrations, transition wakes, stale wake handling, and cancellation priority after already-published work;
- affine monotonic timers with stable source identity, never-reused arm generations, equality-as-expiration, and stale wake checks;
- a POSIX process-control source separate from terminal input;
- `using` cleanup on every lexical exit, explicit-close cleanup consumption, reverse acquisition order, and the complete body/cleanup error matrix;
- `propagate error_value` forwarding one already-evaluated immutable error value and `propagate call() cause prior_error` attachment semantics;
- immutable acyclic errors, cause/suppressed/truncation inspectors, cause depth 8, suppressed count 8, and 64 KiB total attachment storage; and
- test-only availability with sealed test-runner authority that cannot forge sessions, coordinator leases, or recovery tokens.

Terminal-specific cleanup and diagnostic ordering below refines, and MUST NOT contradict, those generic rules.

## Constructor visibility, ABI evolution, and unload

`@constructor_visibility(runtime)`, `(compiler)`, and `(standard_library)` are narrow opt-in declaration metadata. They suppress external construction while preserving public refinement and inspectors. Runtime events, capabilities, diagnostics, IDs, pause results, errors, constrained runtime payloads, trusted-paste evidence, and recovery tokens remain sealed. Standard immutable `Bytes` carries `UnknownBytes`; the sealed event constructor proves its configured per-event bound.

Every terminal-owned public type has an explicit stable uint64 type ID; every sum/error variant has an explicit positive stable uint64 ID. Every sum is either `non_exhaustive_additive` only where unknown variants are semantically ignorable, or `closed_major_only`. Removal requires a same-revision `abi-history.md` retirement entry, and retired IDs can never be reused.

The permanent host supplies generic retain, drop, and opaque-forward operations for host-described boxes, so ordinary unknown additive payloads survive producer unload without calling producer code. A producer module is pinned only while a payload truly requires module-specific destruction not expressible through permanent host metadata; such payloads cannot be forwarded into an old module until the pin is retained. Module unload waits only for those exceptional pins, not every terminal value.

## Affine ownership and runtime lifecycle

`TerminalSession` is one affine, noncopyable, non-publicly-constructible resource type. V1 has no state-indexed session types and makes no typestate claim. The compiler enforces ownership, moves, borrows, and exactly one cleanup obligation; the runtime validates the binding's current `TerminalSessionState` on every operation before host inspection, input consumption, output mutation, decoder mutation, ledger mutation, or state transition.

`using session = propagate terminal_session_open_sync(options):` owns the binding. Read-only operations use `ref session: TerminalSession`; state-changing operations use `mutable ref session: TerminalSession`. A borrow never transfers restoration responsibility. `Opening`, `FailedOpenRecovery`, and `FailedCloseRecovery` are coordinator states, not states of a returned session binding. A successful open first exposes a binding in `Active`. Public session state is exactly `Active`, `Paused`, `RestorePending`, or `Closed`.

A state rejection returns the matching `TerminalSessionStateError` with a structured diagnostic and changes no session state, coordinator generation, readiness generation, decoder, queue, ledger, capability snapshot, input, output, cursor, or terminal mode.

### Session operation/state matrix

`StateError(X)` means the error variant naming current state `X`, produced before any operation-specific effect. Capability and readiness inspectors are infallible for every still-owned binding. A successful explicit close consumes the binding's cleanup obligation and returns the coordinator to `Free`, but leaves the non-owning affine value inspectable in `TerminalSessionState.Closed` until its lexical binding ends.

| Public operation | Active | Paused | RestorePending | Closed |
|---|---|---|---|---|
| `terminal_session_capabilities` | Return current immutable snapshot. | Return last immutable snapshot. | Return last immutable snapshot. | Return last immutable snapshot. |
| `terminal_session_state` | `Active`. | `Paused`. | `RestorePending`. | `Closed`. |
| `terminal_session_readiness_source` | Return the session's stable source identity. | Same identity. | Same identity, continuously ready. | Same identity, continuously ready. |
| `terminal_session_size_sync` | Query and return current size. | Return the last authoritative size snapshot without host-mode mutation. | `StateError(RestorePending)`. | `StateError(Closed)`. |
| `terminal_session_read_event_sync` | Perform the selected wait/read contract. | `StateError(Paused)`; consume no input. | `StateError(RestorePending)`; consume no input. | `StateError(Closed)`; consume no input. |
| trusted write, diagnostic write, flush, cursor visibility, cursor shape | Validate then perform. | `StateError(Paused)`; no output/cursor mutation. | `StateError(RestorePending)`; no mutation. | `StateError(Closed)`; no mutation. |
| `terminal_session_pause_sync` | Build delivery, restore, transition to `Paused`, and return events; restoration failure enters `RestorePending`. | Return an empty `TerminalPauseEvents` non-transition. | `StateError(RestorePending)`. | `StateError(Closed)`. |
| `terminal_session_resume_sync` | `StateError(Active)`. | Reacquire/configure and transition to `Active` on success; a failure before mutation or after complete reverse-ledger compensation returns `TerminalSessionOpenError.ResumeFailed` and remains `Paused`; incomplete compensation commits completed inverses, enters live-binding `RestorePending`, and returns `TerminalSessionRestoreError.ResumeRestorePending`. | `StateError(RestorePending)`. | `StateError(Closed)`. |
| `terminal_session_close_sync` | Restore and close; failure enters `RestorePending`. | Close the retained ownership/ledger; failure enters `RestorePending`. | Retry the remaining inverse ledger; failure remains `RestorePending`. | Return `Clean` idempotently; no mutation. |
| lexical `using` cleanup | Attempt close, then consume or transfer as specified below. | Same. | Retry remaining ledger, then consume or transfer. | No-op because explicit close consumed the cleanup obligation. |

No public session operation accepts a binding in a process-owned recovery state because no such binding exists. Failed-open recovery exists before a session is returned. Failed-close recovery exists only after cleanup has consumed the session and transferred ownership to the coordinator.

### Coordinator operation/state matrix

The coordinator state is process-global for the standard terminal pair and has exactly `Free`, `Opening`, `Active`, `Paused`, `RestorePending`, `FailedOpenRecovery`, and `FailedCloseRecovery`. `Free` is the sole no-process-owned-ledger coordinator state and the sole state permitting ordinary acquisition and legacy I/O. A directly closed affine value may remain inspectable as `TerminalSessionState.Closed`, but it retains no coordinator ownership and therefore coexists with coordinator `Free`; session-value state and coordinator state are distinct. Legacy behavior is exactly the mandatory matrix in `../core-prerequisites.md`; this table states terminal-specific acquisition and recovery behavior.

| Coordinator state | `terminal_session_open_sync` | open-ledger recovery | close-ledger recovery | Session binding | Legacy stdin/fallible raw stdout | `print` / `println` |
|---|---|---|---|---|---|---|
| `Free` | Preflight and exclusively reserve capacity for one future process-recovery generation; only then replace the coordinator lease epoch and publish `Opening`. Capacity exhaustion returns `TerminalSessionOpenError.GenerationExhausted` while remaining `Free`. | Reject non-mutating by token validation; there is no current ledger. | Same. | No owning binding; a directly closed non-owning value may remain inspectable. | Permitted under prerequisite contracts. | Bounded escaped diagnostic lane. |
| `Opening` | `TerminalAlreadyOwned`; no inspection/mutation. | Reject non-mutating. | Reject non-mutating. | Not yet exposed. | Coordinator unavailable; no consumption/mutation. | Drop; no raw stdout touch. |
| `Active` | `TerminalAlreadyOwned`; no mutation. | Reject non-mutating. | Reject non-mutating. | One `Active` binding. | Coordinator unavailable; no consumption/mutation. | Bounded escaped diagnostic lane only. |
| `Paused` | `TerminalAlreadyOwned`; no mutation. | Reject non-mutating. | Reject non-mutating. | One `Paused` binding. | Coordinator unavailable; no consumption/mutation. | Bounded escaped diagnostic lane only. |
| `RestorePending` | `TerminalAlreadyOwned`; no mutation. | Reject non-mutating. | Reject non-mutating; the live binding owns retry authority. | One `RestorePending` binding. | Coordinator unavailable; no consumption/mutation. | Drop; no raw stdout touch. |
| `FailedOpenRecovery` | `RecoveryPending` carrying an alias of the current open token; no mutation. | Validate token and retry remaining open rollback. | `WrongKind`; no mutation. | None. | Coordinator unavailable; no consumption/mutation. | Drop; no raw stdout touch. |
| `FailedCloseRecovery` | `RecoveryPending` carrying an alias of the current close token; no mutation. | `WrongKind`; no mutation. | Validate token and retry remaining close restoration. | None. | Coordinator unavailable; no consumption/mutation. | Drop; no raw stdout touch. |

## Recovery authority and lifecycle

### Sealed immutable recovery-token aliases

`TerminalRecoveryToken` is sealed, opaque, immutable, copyable, and non-publicly constructible. Copying a token creates an alias to one unforgeable host capability cell; it does not create independent recovery authority. The public inspectors reveal only `TerminalRecoveryLedgerKind` (`OpenRollback` or `CloseRestore`) and a nonzero uint64 generation. Hidden cell data binds the token to the host/process terminal coordinator, originating session/attempt identity, exact ledger identity, and ledger kind. No public inspector exposes host handles, session identity, ledger contents, or mutable recovery state.

Before any `Free`→`Opening` transition, the coordinator exclusively reserves capacity to issue exactly one future process-recovery generation for that ownership epoch. Reservation preflight occurs before `Opening` publication, coordinator lease-epoch replacement, terminal inspection or mutation, ledger insertion, allocation for ownership, or session exposure. If no next generation can ever be issued without wraparound, open returns `TerminalSessionOpenError.GenerationExhausted { last_issued_generation, diagnostic }`, leaves the coordinator `Free`, and performs none of those effects.

Reserved capacity is not an issued or published generation: it creates no token/capability cell, advances no public generation, and is not inspectable. Because the coordinator admits only one ownership or recovery epoch, the reservation is exclusive and guarantees allocation-independent issuance of one next nonzero generation if that epoch later transfers to process recovery. Releasing an unused reservation permits that never-issued candidate capacity to be reserved by a later open; this is not generation reuse. Once consumed, the issued generation advances the monotonic issued-generation record, is published in the token, and is never released, reused, wrapped, or issued a second time from the same ownership epoch. Corrupt or unauthentic reservation/counter state fails closed and never fabricates authority.

Recovery validates in this exact order before executing an inverse step:

1. Authenticate the sealed cell and hidden host/session/ledger provenance. A token from another host, coordinator, session attempt, or ledger fails `WrongSession`.
2. Compare the public/hidden ledger kind with the called recovery operation. A mismatch fails `WrongKind`.
3. Check the capability-cell consumed identity. If any alias already succeeded, every alias fails `Consumed`, even if a later ledger now exists.
4. Compare generation and exact ledger identity with the coordinator's current recovery ledger. Missing or superseded authority fails `Stale`.
5. Atomically claim recovery. A simultaneous caller that does not obtain the claim fails `RecoveryInProgress`.

`WrongSession`, `WrongKind`, `Consumed`, `Stale`, and `RecoveryInProgress` are structured recovery-call failures. They perform no terminal inspection, inverse step, input/output operation, state change, ledger change, generation advance, token consumption, or retry-count mutation. Recovery calls never issue a new generation, so v1 has no restore-family generation-exhaustion path. There is no free/no-ledger idempotent recovery success.

The claim is released after a retryable failure. Recovery executes only remaining inverse steps and never reserves or issues another generation. A partial recovery failure commits every inverse step already completed, records the shortened remaining ledger, keeps the same coordinator state and issued generation, and returns `PendingOpenRollbackFailed` or `PendingCloseRestoreFailed` carrying a usable alias to the same capability cell plus ordered diagnostics. That alias and every preexisting unconsumed alias remain retryable. A first complete recovery atomically releases terminal ownership, transitions the coordinator to `Free`, and marks the capability cell consumed before returning success; the issued generation remains permanently spent and all aliases then fail `Consumed` forever. Recovery never repeats a completed inverse step.

Recovery tokens are direct typed fields of `TerminalSessionOpenError`, `TerminalSessionRestoreError`, or retryable recovery errors. They are never causes, suppressed values, formatted diagnostic text, or truncatable attachments. Diagnostic truncation cannot remove, replace, or invalidate recovery authority.

### Open, direct close, cleanup transfer, and process recovery

After ordinary option validation while still `Free`, open atomically preflights and installs the exclusive unissued recovery-generation reservation. Only after that succeeds does it replace the hidden coordinator lease epoch, reserve the coordinator slot, and publish `Opening`. Before terminal inspection, mutation, or ledger insertion, open allocates and initializes the ledger, transfer cell, and potential recovery capability cell needed by that epoch; allocation failure at this point is a pre-mutation failure that releases the unused reservation and slot and returns the coordinator to `Free`. Any other pre-mutation failure, or a later open failure whose reverse-ledger rollback completes, likewise releases the unused reservation and slot and returns the original open error. Every applied mutation and exact inverse is appended to the preallocated ledger. If rollback remains incomplete, transfer to `FailedOpenRecovery` consumes that same attempt reservation and preallocated capability cell to issue its one authentic `OpenRollback` generation without allocation or exhaustion failure, then returns `RollbackFailed { diagnostics, recovery_token }`. No session binding is returned on any open failure, and no ownership epoch can issue both open- and close-recovery generations.

A successful open moves the still-unissued reservation into the affine `TerminalSession` binding. The binding retains it unchanged through `Active`, `Paused`, direct-close `RestorePending`, resume `RestorePending`, and every retry. A direct `terminal_session_close_sync` keeps both restoration authority and the reservation in the live binding. Complete restoration transitions the affine value to `TerminalSessionState.Closed`, releases the unused reservation and coordinator ownership, returns the coordinator to `Free`, consumes the binding's cleanup obligation, and returns `Clean` or `DiscardedInput`. The closed value remains inspectable but owns no coordinator slot, terminal ledger, or generation reservation. Partial restoration commits completed inverses, leaves the shortened ledger, retained decoder data, and unissued reservation in the same `RestorePending` binding, and returns the corresponding restoration error. It does not issue a process recovery token because the caller still owns the session. A successful direct-close retry releases the reservation unused.

Resume from `Paused` records every applied reacquisition/configuration step and exact inverse in the live binding's preallocated ledger. A failure before the first mutation, or a later failure whose compensation completes in strict reverse-ledger order, returns `TerminalSessionOpenError.ResumeFailed` and leaves the binding and coordinator `Paused`. `TerminalSessionOpenError.RollbackFailed` is open-only: resume never transfers its live binding to process-owned failed-open recovery and never issues an open-recovery token. If resume compensation is incomplete, completed inverse steps remain committed, the binding retains the shortened ledger and retry authority, both binding and coordinator enter `RestorePending`, and `TerminalSessionRestoreError.ResumeRestorePending { diagnostics }` is returned. That error carries ordered diagnostics and no recovery token because the live binding remains the sole restoration owner. Subsequent direct close or lexical cleanup handles the remaining ledger under the existing live-binding rules.

When `using` exits with a binding in `Active`, `Paused`, or `RestorePending`, cleanup follows the exact acquisition/exit/error-precedence contract in `../core-prerequisites.md`. It attempts every safely executable remaining inverse despite earlier inverse failures. On complete restoration it consumes the binding, releases the unused reservation, and releases the coordinator. If restoration remains incomplete, it atomically transfers the remaining ledger, retained decoder/input accounting, readiness ownership, coordinator slot, reservation, and preallocated capability cell into `FailedCloseRecovery`; consumes that reservation to issue the epoch's one authentic `CloseRestore` generation without allocation or exhaustion failure; consumes the session binding only after transfer and issuance are complete; and reports `CloseRestorePending { diagnostics, recovery_token }`. The preallocated transfer/capability cells and pre-reserved generation capacity make issuance allocation-independent. No lexical exit can strand a live affine binding solely because generation capacity is unavailable. If reservation/counter state is corrupt, the process enters fail-stop while retaining terminal ownership internally and fabricates no authority; execution cannot continue as a successful scope exit. Corruption is outside the ordinary v1 error surface rather than an unreachable restore-family `GenerationExhausted` result.

Terminal inverses execute in strict reverse order of successful ledger insertion. For the selected v1 operations this means, when those steps were applied, reverse protocol enables first; then cursor/alternate-screen/active-buffer changes in reverse application order; then output-mode changes; then input-mode, descriptor/status, and signal-notification changes in reverse application order; and coordinator ownership release last. A step absent from the ledger is never synthesized. Diagnostics are recorded in attempted inverse order. Within one resource, the first failing inverse is primary and later failing inverses are suppressed in encounter order. Across nested `using` resources, the prerequisite's inner-to-outer cleanup order and cause/suppression rules apply. The body error is attached as cause when cleanup becomes primary; typed recovery-token payloads remain on their primary terminal error and are not moved into attachments.

`terminal_session_recover_open_sync` accepts only an authenticated current `OpenRollback` token. `terminal_session_recover_close_sync` accepts only an authenticated current `CloseRestore` token. Neither function discovers or recovers a ledger without a token. A new open observing process recovery returns `RecoveryPending` with an alias of the existing token and does not retry implicitly.

## Readiness, wait sets, timers, and one-event draining

`terminal_session_readiness_source(ref session: TerminalSession)` always returns the same `SystemReadinessSource` identity for one session across `Active`, pause, resume, restore pending, and close. Active input, resize notification, parser deadlines, and queued events make it ready. Every `Active`→`Paused`, `Paused`→`Active`, `Active`/`Paused`→`RestorePending`, `RestorePending`→`Closed`, and `Active`/`Paused`→`Closed` transition wakes all current wait-set registrations so callers can observe runtime state changes.

`RestorePending` and `Closed` are continuously ready terminal states rather than edge-only notifications. Clones obtained before close remain terminally ready after close until individually dropped. They never expose or resurrect the session. Removing a registration and dropping final source references follow the generic lifetime rules in `../core-prerequisites.md`.

A wake is a hint. The caller compares the wake source identity/generation as required by the generic wait-set contract and reattempts the source-specific operation. After a terminal wake, the caller invokes `terminal_session_read_event_sync(..., TerminalWait.Poll, ...)`. One successful call returns exactly one normalized event. To drain already-published work, the caller repeats `Poll` until `TimedOut`, `Cancelled`, `EndOfInput`, or a state error; `TimedOut` after any number of events is the required stale/drained-readiness result and returns control to the shared wait set. The API never returns a hidden batch.

`TerminalWait.For` uses a host monotonic deadline with equality treated as expiration. Internal parser deadlines participate in the same host wait without exposing a terminal-specific timer identity. Application/chord deadlines use the generic affine `MonotonicTimer` and `SystemWaitSet` declarations from the prerequisite. There is no polling scheduler. Already-published queued terminal events precede cancellation; once the queue is empty, a simultaneously ready cancelled token wins before new terminal input is consumed.

## Public API

All selected terminal public signatures are collected here in one coherent proposal block and use canonical Opalescent borrows. Core prerequisite signatures are not redeclared here.

```opal
# terminal_session_options_default(): TerminalSessionOptions
# terminal_session_options_with_feature_policy(options: TerminalSessionOptions, policy: TerminalSessionFeaturePolicy): TerminalSessionOptions errors AllocationFailureError
# terminal_session_options_with_resource_limits(options: TerminalSessionOptions, limits: TerminalSessionResourceLimits): TerminalSessionOptions errors AllocationFailureError
# terminal_session_options_validate(options: TerminalSessionOptions): TerminalSessionOptions errors TerminalSessionOptionsError

# terminal_session_open_sync(options: TerminalSessionOptions): TerminalSession errors TerminalSessionOpenError
# terminal_session_recover_open_sync(recovery_token: TerminalRecoveryToken): void errors TerminalSessionRestoreError
# terminal_session_recover_close_sync(recovery_token: TerminalRecoveryToken): void errors TerminalSessionRestoreError
# terminal_recovery_token_kind(recovery_token: TerminalRecoveryToken): TerminalRecoveryLedgerKind
# terminal_recovery_token_generation(recovery_token: TerminalRecoveryToken): uint64

# terminal_session_state(ref session: TerminalSession): TerminalSessionState
# terminal_session_capabilities(ref session: TerminalSession): TerminalCapabilities
# terminal_capabilities_feature(capabilities: TerminalCapabilities, feature: TerminalOrdinaryFeature): TerminalFeatureCapability
# terminal_capabilities_trusted_paste_framing(capabilities: TerminalCapabilities): TerminalTrustedPasteCapability
# terminal_capabilities_color(capabilities: TerminalCapabilities): TerminalColorCapability
# terminal_session_readiness_source(ref session: TerminalSession): SystemReadinessSource
# terminal_session_size_sync(ref session: TerminalSession): TerminalSize errors TerminalSessionReadError, TerminalSessionStateError
# terminal_session_read_event_sync(mutable ref session: TerminalSession, wait: TerminalWait, cancellation: CancellationToken): TerminalInputEvent errors TerminalSessionReadError, TerminalSessionStateError
# terminal_session_write_sync(ref session: TerminalSession, output: TrustedTerminalOutput): void errors TerminalSessionWriteError, TerminalSessionStateError
# terminal_session_write_diagnostic_sync(ref session: TerminalSession, output: SafeTerminalDiagnosticOutput): void errors TerminalSessionWriteError, TerminalSessionStateError
# terminal_session_flush_sync(ref session: TerminalSession): void errors TerminalSessionWriteError, TerminalSessionStateError
# terminal_session_set_cursor_visible_sync(ref session: TerminalSession, visible: boolean): void errors TerminalSessionWriteError, TerminalSessionStateError
# terminal_session_set_cursor_shape_sync(ref session: TerminalSession, shape: TerminalCursorShape): void errors TerminalSessionWriteError, TerminalSessionStateError
# terminal_session_pause_sync(mutable ref session: TerminalSession): TerminalPauseResult errors TerminalSessionReadError, TerminalSessionRestoreError, TerminalSessionStateError
# terminal_session_resume_sync(mutable ref session: TerminalSession): void errors TerminalSessionOpenError, TerminalSessionRestoreError, TerminalSessionStateError
# terminal_session_close_sync(mutable ref session: TerminalSession): TerminalCloseOutcome errors TerminalSessionRestoreError

# trusted_terminal_output_from_application_text(text: string): TrustedTerminalOutput errors AllocationFailureError
# safe_terminal_diagnostic_format(diagnostic: TerminalDiagnostic): SafeTerminalDiagnosticOutput errors AllocationFailureError
# safe_terminal_diagnostic_collection_format(diagnostics: TerminalDiagnosticCollection): SafeTerminalDiagnosticOutput errors AllocationFailureError

# terminal_pause_events_length(events: TerminalPauseEvents): int64
# terminal_pause_events_at(events: TerminalPauseEvents, index: int64): TerminalInputEvent errors IndexOutOfBoundsError

# terminal_diagnostic_backend(diagnostic: TerminalDiagnostic): TerminalBackend
# terminal_diagnostic_operation(diagnostic: TerminalDiagnostic): TerminalOperation
# terminal_diagnostic_stage(diagnostic: TerminalDiagnostic): TerminalDiagnosticStage
# terminal_diagnostic_coordinator_state(diagnostic: TerminalDiagnostic): TerminalCoordinatorState
# terminal_diagnostic_session_state(diagnostic: TerminalDiagnostic): TerminalDiagnosticSessionState
# terminal_diagnostic_os_code(diagnostic: TerminalDiagnostic): TerminalOsCode
# terminal_diagnostic_detail(diagnostic: TerminalDiagnostic): TerminalDiagnosticDetail
# terminal_diagnostic_retryability(diagnostic: TerminalDiagnostic): TerminalDiagnosticRetryability
# terminal_diagnostic_was_truncated(diagnostic: TerminalDiagnostic): boolean

# terminal_diagnostics_length(diagnostics: TerminalDiagnosticCollection): int64
# terminal_diagnostics_at(diagnostics: TerminalDiagnosticCollection, index: int64): TerminalDiagnostic errors IndexOutOfBoundsError
# terminal_diagnostics_retained_count(diagnostics: TerminalDiagnosticCollection): uint64
# terminal_diagnostics_omitted_count(diagnostics: TerminalDiagnosticCollection): uint64
# terminal_diagnostics_retained_bytes(diagnostics: TerminalDiagnosticCollection): uint64
# terminal_diagnostics_omitted_bytes(diagnostics: TerminalDiagnosticCollection): uint64
# terminal_diagnostics_was_truncated(diagnostics: TerminalDiagnosticCollection): boolean
```

There is no `terminal_session_output_terminal`, no `AcquireOutputTerminal` terminal operation, and no session-derived `StdoutTerminal` or writer. Session rendering accepts only `TrustedTerminalOutput` or `SafeTerminalDiagnosticOutput`. Legacy writer/terminal handles remain generation-bound under `../core-prerequisites.md` and cannot bypass session ownership or the selected trust boundary.

The authoritative declarations and `abi-history.md` MUST define and account for all types and error payloads named here. In particular, `RollbackFailed` and `RecoveryPending` carry `recovery_token`; `CloseRestorePending`, `PendingOpenRollbackFailed`, and `PendingCloseRestoreFailed` carry `recovery_token`; token-rejection variants carry the structured expected/observed kind or generation data applicable to that rejection plus a bounded diagnostic. No rejection uses a sentinel or diagnostic text as authority.

## Options and capabilities

The opaque default is stable for an ABI major: alternate screen off, cursor visible, bracketed-paste transport off, trusted-paste requirement off, enhanced/focus/mouse/control capture off, requested features non-strict, 25 ms sequence timeout, 4096-byte committed/preedit/paste chunks, 1024-byte unknown/pending chunks, 1024 retained events, 1 MiB retained bytes, 64 correlated events, 64 KiB correlated bytes, 16 diagnostics, and 64 KiB diagnostic collections.

`TerminalSessionFeaturePolicy` and `TerminalSessionResourceLimits` are application-constructible closed/major-only records. Each allocation-fallible functional API replaces its entire category in an immutable options snapshot; the two calls commute and never cross-validate, so setter order is irrelevant. Final validation returns the snapshot unchanged or structured `TerminalSessionOptionsError.InvalidOptions`. Open repeats validation. Future categories use new additive APIs rather than adding required fields to either v1 record.

`require_trusted_paste_framing` is backend-independent policy. Native trusted record boundaries satisfy it even when bracketed-paste transport is disabled. Enabling bracketed paste requests one possible transport; it neither proves nor is required for trusted framing. Validation must not declare those fields conflicting.

The centralized `terminal_capabilities_feature` accepts every `TerminalOrdinaryFeature`: alternate screen, cursor shape, bracketed paste, focus, mouse buttons, mouse motion, key release, composition, and enhanced key identity. Trusted paste uses its dedicated unforgeable evidence inspector; color uses its specialized cardinality inspector. These functions make every opaque capability field inspectable. Resume replaces the session's current immutable snapshot without mutating snapshots already returned.

## Structured diagnostics and collection accounting

`TerminalDiagnostic` is sealed, immutable, runtime-origin structured data. Every diagnostic contains a backend, operation, stage, coordinator state, nominal session-state presence (`Unavailable` or one public `TerminalSessionState`), OS code, bounded detail, retryability classification, and per-diagnostic truncation flag. Each inspector is infallible and returns the stored field without consulting the host or session. Unknown additive backend/operation/stage/code values remain inspectable and safely formattable.

Retryability distinguishes at least non-retryable, retryable with the same live session, and retryable only with a recovery token. It is advisory structured metadata, never authority. `TerminalDiagnosticDetail` is diagnostic-only and cannot carry a recovery token, OS handle, unbounded host message, or active terminal control sequence. The truncation flag records sanitization or bounded-detail truncation; it does not imply collection truncation.

`TerminalDiagnosticCollection` preserves diagnostic encounter order and retains the longest prefix of complete diagnostic values whose count and production collection-byte accounting fit the configured limits. Before retaining each complete diagnostic, it uses checked prospective arithmetic for both retained count and retained bytes. A prospective limit excess or arithmetic overflow is limit exhaustion before insertion: that diagnostic and every later complete diagnostic are omitted without partial retention. `length`, `retained_count`, and indexed access describe only retained diagnostics; retained count equals indexed length, and retained bytes remain the exact checked production-accounting total within the configured limit. Retained accounting includes collection records, immutable boxes, bounded detail capacity, alignment, and truncation metadata. Collection metadata is preallocated and cannot be displaced by a diagnostic.

For each excluded complete diagnostic, omitted count and omitted bytes are derived rather than caller-authored. Omitted count adds one with checked uint64 arithmetic; omitted bytes adds that diagnostic's complete checked production-accounting byte contribution. Before overflow, each stored total is exact. If either omitted accumulator addition overflows, that accumulator stores exactly `18446744073709551615` (uint64 maximum), remains at that value for every later omission, and never wraps or decreases. After saturation, the stored maximum is a lower bound on the true omitted count or true omitted bytes, respectively; it is neither an upper bound nor an unknown-value sentinel. Saturation and every omission set `was_truncated=true`; equivalently, `was_truncated` is true exactly when at least one complete diagnostic was omitted. Retained totals and configured limit/allocation behavior are unchanged by omitted-total saturation.

Safe formatting is independent from inspection. `safe_terminal_diagnostic_format` and collection formatting escape C0/C1 controls, ESC, DEL, bidi controls, noncharacters, and invalid display scalars; bound output; and emit visible truncation markers. They never recreate token authority from text. Error causes, suppressed values, and `ErrorAttachmentTruncation` remain available only through the generic inspectors in `../core-prerequisites.md` and are not flattened into terminal collection accounting.

## Events, text phases, and identifiers

`Key` is command identity; `TextInput` is insertion. Ordinary terminals unable to separate them emit only `TextInput.Direct`. Enhanced backends may emit a Key followed by linked text. The complete Key/text group is admitted atomically against correlated and total capacity before publication. The Key is immediately followed by either one Complete chunk or adjacent Start, zero or more Continue, and End chunks with one unchanged event ID. Nothing interleaves.

TextInput phase legality is origin-specific:

- `Direct` is always `Complete`. V1 has no direct-group ID. If direct committed text exceeds the chunk limit, emit multiple independent `Direct + Complete` events in order, each split only on scalar boundaries.
- `Key(event_id)` is `Complete` or Start/Continue/End. Every chunk retains the same Key event ID and remains adjacent to that Key and its sibling chunks.
- `Composition(composition_id)` is `Complete` or Start/Continue/End. Every chunk retains one composition ID, is contiguous, and appears immediately before the matching `CompositionEnded.Committed`.

These are sealed runtime-validated cross-field invariants; v1 does not claim every invalid combination is structurally unrepresentable. `Press(count)`/`Repeat(count)` contain the exact occurrences represented by one Key. Linked text is already expanded and is never multiplied implicitly by count. Companion chord routing emits at most one activation for that Key and carries the exact `TerminalKeyOccurrence`.

Composition order is Started, zero or more Updated, then exactly one Ended. Cursor is <= preedit scalar count. Focus loss, pause, EOF, observed cancellation, backend reset, or a new start interrupts the active composition with `CompositionEnded.Interrupted` then exactly one `InputReset.CompositionInterrupted`.

Event/composition IDs increase monotonically and are never reused within one session. Before publishing an event/group that needs a new ID, the runtime detects exhaustion and enters a sticky identifier-exhausted read state. Every later read returns `TerminalSessionReadError.IdentifierExhausted`. Detection occurs before event publication and ID assignment, not necessarily before backend consumption. Already-consumed bytes/native records remain retained under session bounds; pause surfaces them as `UnknownBytes`/`UnknownNative` where representable, and a successful close reports any remainder through `TerminalCloseOutcome.DiscardedInput`. No consumed input silently becomes a forged or reused ID.

Mouse coordinates are zero-based against the visible viewport snapshot used at decode time. A Resize carrying that snapshot is ordered before Mouse decoded against it. Later resize does not reinterpret retained coordinates.

## Trusted paste and malformed fallback

`TerminalTrustedPasteEvidence` is runtime-only and cannot be `EnvironmentInferred` or `ProtocolAssumed`. `NativeRecordBoundary` proves payload and key records are distinct. `SanitizedProtocolBoundary` proves the host removed or escaped every delimiter/control capable of ending framing or producing commands before parsing. The guarantee is only parser command isolation; it does not make text safe for a shell, editor language, renderer, or remote protocol.

After the first malformed UTF-8 byte or NUL byte in a trusted frame, the entire frame enters PasteFallback. Every payload byte already consumed, including any valid prefix, and every subsequently consumed payload byte through frame termination, EOF, pause, or backend reset is emitted only as bounded immutable `UnknownBytes`. Every fallback chunk uses the first defect's reason. No prefix/suffix becomes Paste, TextInput, Key, composition, or commands. The framing delimiter is not payload. Termination emits exactly one `InputReset.PasteFallback`, after all consumed payload bytes. Capacity pauses backend consumption; it never terminates fallback or drops bytes.

## EOF, cancellation, pause, close, and accounting

Queued decoded events precede EOF/cancellation. EOF is sticky for the Active parser generation after queued consumed input and composition interruption; later reads immediately return `EndOfInput` until resume or close. If cancellation and unread input become ready together after the queue empties, cancellation wins and consumes no new OS input. A cancelled token is sticky; a new source/token generation is required. Parser bytes remain for the new token unless composition interruption rules apply.

Every successful `Active`→`Paused` transition returns immutable `TerminalPauseEvents` ending in exactly one `InputReset.PauseBoundary`, even with no parser bytes. `Paused`→`Paused` returns an empty non-transition. Pause constructs complete delivery before restoration; fit/allocation failure leaves `Active`, and restoration failure leaves `RestorePending` with no success result. Session accounting ends only after independent result retention and release of session references. Aggregate caller retention is outside the session bound.

Successful close returns `Clean` or `DiscardedInput`; discarded input is a successful notice, never an error. Direct restoration failure retains decoder data and live `RestorePending` ownership. Cleanup restoration failure outranks a body error according to the prerequisite and transfers retained data/accounting into process recovery before consuming the binding.

Retained bytes use checked uint64 accounting of allocation headers, variant boxes, alignment, queue records/links, payload capacity, parser/fallback/native buffers, ledger records, pause storage before transfer, and session readiness/cancellation registrations. Overflow is capacity exhaustion before allocation/consumption. Events count queue, pending native, unpublished correlated, and pre-transfer pause records. Capacity causes no-drop backpressure. Backend overflow emits bounded unknown data where possible then exactly one reset. Fixed events have a reserved-queue no-allocation fast path; the single-event API shape never requires one allocation per event.

## Separate process-control workflow

Process control uses only the generic `ProcessControlSource` and signatures in `../core-prerequisites.md`. `SuspendRequested` and `Continued` are never `TerminalInputEvent` variants, never consumed from terminal stdin, and never synthesized from a control-key event.

On supported POSIX hosts the application registers both terminal readiness and process-control readiness in the same generic wait set. For `SuspendRequested(generation)`, ordinary application code:

1. stops child/application work and new session output, then calls `terminal_session_pause_sync`;
2. processes every returned pause event through the mandatory final `PauseBoundary` and clears incremental application/chord state;
3. only after the pause boundary is delivered calls `process_control_acknowledge_suspend(source, generation)`, allowing host suspension;
4. after execution continues, polls/waits for matching `Continued(generation)` without reading terminal input while the session is `Paused`;
5. explicitly calls `terminal_session_resume_sync`; and
6. only after successful session resume calls `process_control_resume_application(source, generation)`; after that succeeds, prepares/redraws application view state and only then resumes child/application work.

Repeated suspend requests before acknowledgement coalesce to the same pending generation. Requests after acknowledgement and before continuation do not create a second suspension. A stale, duplicate, mismatched, or out-of-order notification is an observable no-op under the prerequisite and does not pause, acknowledge, restore, resume, or consume terminal input. The application retries only `ProcessControlAcknowledgementError.HostSuspendFailed` and `ProcessControlResumeError.HostApplicationResumeFailed` with the same generation; wrong/stale generation errors propagate unchanged as immutable errors. If pause fails and leaves `Active`, the application does not acknowledge and keeps application/child work stopped. If pause or terminal resume enters `RestorePending`, the application remains fail-closed, propagates the structured restore error, and does not resume process application or ordinary application work. While awaiting matching `Continued`, the event loop handles only process-control notification and cancellation: it performs no terminal read, timer/chord expiry, rendering, or ordinary application/child work. If continuation arrives before the application observes it, readiness remains sufficient for the explicit sequence; `Continued` never resumes the terminal or application by itself. On matching continuation the order is terminal resume, successful process application resume, application view preparation/redraw, then application/child work resume. A pre-mutation or completely compensated terminal-resume failure leaves `Paused`; an incompletely compensated failure leaves `RestorePending`.

The process-control source is unavailable on Windows under this contract; there is no invented console-control equivalent and no terminal process-control event. Catchable termination/interrupt notifications that an application maps to generic cancellation only request ordinary cancellation and never restore terminal state in a signal handler. Original POSIX dispositions/masks changed by the terminal ledger are restored through that ledger. No restoration is promised after SIGKILL, `TerminateProcess`, power loss, or corruption.

## Output trust and legacy standard I/O

`trusted_terminal_output_from_application_text` is explicit application declassification. Calling it states that the application has reviewed the complete string as intended terminal control/rendering output. The type system enforces only the nominal conversion and write boundary; v1 performs no provenance, taint, origin, data-flow, or semantic safety enforcement. Input, environment, path, remote, and diagnostic text may reach this conversion only by an explicit application decision, not by an implicit conversion. The function does not make content safe for a shell, editor language, remote protocol, or later concatenation.

Untrusted diagnostics use `safe_terminal_diagnostic_format` or another API that already returns `SafeTerminalDiagnosticOutput`. Nominal session write parameters make direct string/input rendering a type error. There is no implicit conversion between string, `TrustedTerminalOutput`, and `SafeTerminalDiagnosticOutput`.

All existing standard input/output APIs follow the mandatory inventory, fallible signatures, generation leases, and state matrix in `../core-prerequisites.md`. In particular, rejected `take_input` consumes no bytes; rejected legacy writer, terminal, flush, capability, or cursor operations perform no mutation; stale handles cannot be refreshed while ownership is active; and `print`/`println` are only the bounded escaped diagnostic lane in states where that lane is permitted. No existing operation gains a generic session capability, and unrelated APIs do not gain `TerminalSessionStateError`.

## Linux normative contract

The runtime snapshots the complete `termios` value and descriptor status flags before mutation and restores those exact snapshots. It sets descriptor `O_NONBLOCK`; clears `ICANON`, `ECHO`, `ECHONL`, `IEXTEN`, `ICRNL`, `INLCR`, `IGNCR`, `IXON`, `IXOFF`, `BRKINT`, `PARMRK`, and `ISTRIP`; sets `IGNBRK`; clears `CSIZE` then sets `CS8`; preserves parity and every unrelated control/local/input flag; clears output `OPOST` only when rendering requires raw output; and clears `ISIG` only when control-key capture is requested. It sets `VMIN=0`, `VTIME=0`, and applies changes with `TCSANOW`, which does not flush pending input.

Wait uses `poll`/`ppoll` with monotonic deadlines and terminal input, SIGWINCH notification, cancellation wakeup, and parser deadline in one host wait compatible with the generic wait-set contract. `EAGAIN` means no current data and retries only within the remaining wait. Readable bytes are drained before processing `POLLHUP`; zero bytes become sticky EOF only after confirmed hangup/closure. When input and resize are both ready, currently readable input is drained first, then one newest `TIOCGWINSZ` snapshot is queued; this is deterministic observation ordering, not causal chronology. Cancellation tied with unread input wins only after already-decoded queue entries and consumes no new bytes.

SIGWINCH uses a nonblocking self-pipe or equivalent; the handler performs only async-signal-safe notification. Original signal disposition/mask and every descriptor flag are restored exactly. Job-control notifications use the separate process-control source, not the terminal parser.

## Windows Console and ConPTY normative contract

For Console handles, the runtime snapshots input/output modes, `CONSOLE_CURSOR_INFO`, the original active screen-buffer handle/identity, and every queryable state changed by fallback output. Restoration reactivates the original buffer before closing a temporary alternate buffer, then restores cursor information and modes. It clears `ENABLE_LINE_INPUT` and `ENABLE_ECHO_INPUT`, clears `ENABLE_PROCESSED_INPUT` only for control-key capture, sets `ENABLE_WINDOW_INPUT`, and, only when mouse is requested, sets `ENABLE_MOUSE_INPUT` plus `ENABLE_EXTENDED_FLAGS` while clearing `ENABLE_QUICK_EDIT_MODE`.

Console wait combines console input and cancellation manual-reset event with `WaitForMultipleObjects`, then consumes bounded `INPUT_RECORD` batches using `ReadConsoleInputW`. Resize records trigger authoritative visible-size query. Coordinates use `srWindow`; the runtime subtracts `Left`/`Top` before validation. UTF-16 conversion retains a pending high surrogate only within bounds. An unpaired surrogate emits bounded `UnknownNative.WindowsUnknownRecord` followed by exactly one `BackendReset`; it is never replacement text. Modifier-only/menu records are filtered only where documented; all other unsupported records become bounded `UnknownNative`.

ConPTY/VT uses the same incremental parser and bounds as Linux streams. Overlapped input or a host readiness event, cancellation wake event/pipe, resize notification, and parser deadline participate in one wait. No polling loop is permitted. Handoff/restoration retains the same ledger and active-buffer guarantees as Console where applicable. The separate POSIX process-control contract is unavailable on Windows.

## Verification obligations and exclusions

Future proposal-only fixtures cover canonical borrow parsing and reject reversed forms; one-`is` classification and `into` restrictions; core/terminal ABI ownership and append-only retirement history; one-where constrained errors; constructor sealing; affine ownership without typestate claims; runtime state rejection before effects; direct close, cleanup transfer, and tokenized recovery; immutable-error cause/suppression ordering; and generic host forwarding/module pins.

Behavior fixtures cover fallible cancellation lifetime; wait-set registration/removal, readiness identity, transition wakes, stale Poll, timer generations, and cancellation ordering; complete session/coordinator matrices; legacy I/O rejection without consumption/mutation; option validation; capability inspection; explicit output declassification; pre-Opening generation-capacity exhaustion, reservation release/transfer/one-shot issuance, token provenance/kind/generation/consumption/concurrency, and retryable partial recovery; complete diagnostic inspection/accounting; trusted-paste independence; atomic event correlation; identifier exhaustion; malformed-frame quarantine; sticky EOF; unconditional pause reset; process-control acknowledgement/continuation/explicit resume; and all accounting paths.

Chord fixtures remain specified in `../CHORDS.md`. Platform tests inject every Linux snapshot/restore/wait ordering, Windows Console/ConPTY mode/buffer/surrogate/viewport/wake path, process-control generation, and restoration failure. Security tests reject forged evidence, wrong-session recovery tokens, replayed aliases, direct string writes, implicit trust conversion, stale legacy leases, and public host handles.

RPC, subprocess execution, watches, generalized scheduling, editor buffers, rendering policy, and implementation are outside scope. Public OS handles, flat events, polling cancellation, terminal process-control events, state-indexed v1 session types, mutable bounded arrays, stringly diagnostics, implicit trust conversion, provenance-enforcement claims, authority-free recovery, generic session-derived output terminals, and unbounded retention are forbidden.
