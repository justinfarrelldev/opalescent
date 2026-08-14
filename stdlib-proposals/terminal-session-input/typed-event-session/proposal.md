# Typed Event Session

## Status, scope, and authority

This is the normative selected public v1 terminal-session/input design. It preserves Opalescent's explicit, nominal, errors-first identity: a compiler-registered affine `TerminalSession` owns restoration responsibility; every fallible operation declares errors; events, options, diagnostics, trust boundaries, and errors are nominal; and no platform handle enters the public API. No compiler/runtime implementation is part of this proposal package.

V1 owns only the process interactive standard-input/raw-output terminal pair. The host admits at most one session across Opening, Active, Paused, and RestorePending. Chord routing is a companion concern declared in `../terminal_chords.types.op` and specified by `../CHORDS.md`; it does not enlarge the session API.

`typed_event_session.types.op` and `../terminal_chords.types.op` are the authoritative source declarations for terminal-owned public ABI IDs. The compiler generates one terminal ABI manifest containing active/retired IDs, payload hashes, representation versions, and ownership metadata. Prose never allocates a second ID history.

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

The core-owned `ConstraintViolationError` v1 payload contains exactly the stable constrained type ID and one bounded `ConstraintObservedValue`; it has no terminal-owned ABI ID. Every constrained nominal declaration has exactly one `where` clause, so there is no separate `constraint_id` mechanism. `as ConstrainedType` is allowed only for a literal or constexpr value statically proved to satisfy the clause.

Every constrained terminal/chord type explicitly declares `@abi_evolution(closed_major_only)`. Constraint weakening or tightening is ABI-major because it changes accepted values.

### Generic cancellation, readiness, and error attachments

`CancellationSource`, `CancellationToken`, `SystemReadinessSource`, `ConstraintViolationError`, `ConstraintObservedValue`, and `ErrorAttachmentTruncation` are core/system declarations imported from `standard`; they are not terminal types and receive no terminal ABI IDs.

Selected cancellation signatures use canonical borrow syntax:

```opal
# cancellation_source_new(): CancellationSource errors AllocationFailureError
# cancellation_token(ref source: CancellationSource): CancellationToken
# cancellation_request(mutable ref source: CancellationSource): void
```

`CancellationSource` is affine request authority. Creation allocates host wake state and is fallible. Authority may be moved into a coordinator thread, but Opalescent second-class references never cross thread boundaries: the destination thread borrows its own local binding only after the move completes. Request is idempotent, synchronized, and requires mutable authority. Source destruction is deterministic and infallible: dropping it permanently removes request authority and releases its authority reference. Existing immutable tokens retain observation/wake state until the final token drops. There is no reset; uncancelled work creates a new source/token generation, and an old token never affects a new generation.

`SystemReadinessSource` is a host-stable, cloneable identity accepted by generic system wait sets for terminal, RPC, watch, process, and timer sources without exposing descriptors, HANDLEs, or registration keys. Wait-set removal releases that set's registration reference. General scheduling remains outside this concern.

Core errors are immutable nominal acyclic values. The ABI-major attachment limits remain cause depth 8, suppressed count 8, and 64 KiB total attachment storage. Core-owned truncation markers identify cause-depth, suppressed-count, and attachment-byte cuts. Cause precedes suppressed values; suppressed cleanup order is inner-to-outer and reverse-ledger within one restoration attempt. Preallocated attachment cells preserve the primary if attachment allocation fails. `propagate call() cause prior_error` evaluates once and, only when the call fails, propagates that failure as primary with the prior error attached.

## Constructor visibility, ABI evolution, and unload

`@constructor_visibility(runtime)`, `(compiler)`, and `(standard_library)` are narrow opt-in declaration metadata. They suppress external construction while preserving public refinement and inspectors. Runtime events, capabilities, diagnostics, IDs, pause results, errors, constrained runtime payloads, and trusted-paste evidence remain sealed. Standard immutable `Bytes` carries UnknownBytes; the sealed event constructor proves its configured per-event bound.

Every terminal-owned public type has an explicit stable uint64 type ID; every sum/error variant has an explicit positive stable uint64 ID. Every sum is either `non_exhaustive_additive` only where unknown variants are semantically ignorable, or `closed_major_only`. Removed IDs remain retired and cannot be reused.

The permanent host supplies generic retain, drop, and opaque-forward operations for host-described boxes, so ordinary unknown additive payloads survive producer unload without calling producer code. A producer module is pinned only while a payload truly requires module-specific destruction not expressible through permanent host metadata; such payloads cannot be forwarded into an old module until the pin is retained. Module unload waits only for those exceptional pins, not every terminal value.

## Affine ownership and lifecycle

`TerminalSession` cannot be copied, implicitly dropped, or publicly constructed. `using session = propagate terminal_session_open_sync(options):` owns the binding. Read-only operations use canonical `ref session: TerminalSession`; state-changing operations use `mutable ref session: TerminalSession`. A borrow never transfers restoration responsibility.

Direct close returns `TerminalCloseOutcome`. Success mutates the still-owned binding to Closed before returning; restoration failure retains the same binding in RestorePending for retry. Closing Closed is idempotent `Clean`. Scope cleanup attempts restoration, transfers a failed ledger, retained decoder state, and coordinator slot into host-owned FailedCloseRecovery, and only then consumes the binding. A preallocated ledger cell makes transfer allocation-independent.

Opening reserves the process slot before terminal inspection/mutation. Pre-mutation failure releases it. Open records every applied step and inverse in a preallocated ledger. Successful rollback releases ownership and returns the original open error. Failed rollback transfers to FailedOpenRecovery and returns `RollbackFailed`. Recovery APIs are ledger-kind-specific; Free/no-ledger is idempotent success, mismatch performs no mutation, and matching success releases ownership.

Capability retrieval is valid while the binding is explicitly Active, Paused, RestorePending, or Closed and returns the session's last immutable snapshot. Closed is still an owned affine binding until explicit scope consumption. A `using` binding that has been consumed is unavailable by affine typing. Resume refreshes the current snapshot without mutating snapshots already returned.

## Readiness lifetime

`terminal_session_readiness_source(ref session: TerminalSession)` always returns the same source identity for one session across Active, Pause, and Resume. Active input/resize/parser deadlines make it ready. Every transition Active→Paused, Paused→Active, Active/Paused→RestorePending, RestorePending→Closed, and Active/Paused→Closed wakes current waiters so they can observe state changes.

RestorePending and Closed are terminal readiness states: the source is continuously ready rather than edge-only. Clones obtained before close remain terminally ready after close until individually dropped. They never expose or resurrect the session. Removing a source from a generic wait set releases registration; dropping the final source/registration after session consumption releases host readiness state.

Readiness is a hint. Input may be drained by another consumer or readiness may represent resize, transition, cancellation, or a parser deadline. After a generic wait reports the terminal source, the caller invokes `terminal_session_read_event_sync(..., TerminalWait.Poll, ...)`; `TimedOut` is the required stale-readiness result and the caller returns to the shared wait.

## Public API

All signatures below are proposal syntax and use canonical Opalescent borrows.

```opal
# terminal_session_options_default(): TerminalSessionOptions
# terminal_session_options_with_feature_policy(options: TerminalSessionOptions, policy: TerminalSessionFeaturePolicy): TerminalSessionOptions errors AllocationFailureError
# terminal_session_options_with_resource_limits(options: TerminalSessionOptions, limits: TerminalSessionResourceLimits): TerminalSessionOptions errors AllocationFailureError
# terminal_session_options_validate(options: TerminalSessionOptions): TerminalSessionOptions errors TerminalSessionOptionsError

# terminal_session_open_sync(options: TerminalSessionOptions): TerminalSession errors TerminalSessionOpenError
# terminal_session_restore_pending_open_sync(): void errors TerminalSessionRestoreError
# terminal_session_restore_pending_close_sync(): void errors TerminalSessionRestoreError
# terminal_session_capabilities(ref session: TerminalSession): TerminalCapabilities
# terminal_capabilities_feature(capabilities: TerminalCapabilities, feature: TerminalOrdinaryFeature): TerminalFeatureCapability
# terminal_capabilities_trusted_paste_framing(capabilities: TerminalCapabilities): TerminalTrustedPasteCapability
# terminal_capabilities_color(capabilities: TerminalCapabilities): TerminalColorCapability
# terminal_session_readiness_source(ref session: TerminalSession): SystemReadinessSource
# terminal_session_output_terminal(ref session: TerminalSession): StdoutTerminal errors TerminalSessionStateError
# terminal_session_size_sync(ref session: TerminalSession): TerminalSize errors TerminalSessionReadError, TerminalSessionStateError
# terminal_session_read_event_sync(mutable ref session: TerminalSession, wait: TerminalWait, cancellation: CancellationToken): TerminalInputEvent errors TerminalSessionReadError, TerminalSessionStateError
# terminal_session_write_sync(ref session: TerminalSession, output: TrustedTerminalOutput): void errors TerminalSessionWriteError, TerminalSessionStateError
# terminal_session_write_diagnostic_sync(ref session: TerminalSession, output: SafeTerminalDiagnosticOutput): void errors TerminalSessionWriteError, TerminalSessionStateError
# terminal_session_flush_sync(ref session: TerminalSession): void errors TerminalSessionWriteError, TerminalSessionStateError
# terminal_session_set_cursor_visible_sync(ref session: TerminalSession, visible: boolean): void errors TerminalSessionWriteError, TerminalSessionStateError
# terminal_session_set_cursor_shape_sync(ref session: TerminalSession, shape: TerminalCursorShape): void errors TerminalSessionWriteError, TerminalSessionStateError
# terminal_session_pause_sync(mutable ref session: TerminalSession): TerminalPauseResult errors TerminalSessionReadError, TerminalSessionRestoreError, TerminalSessionStateError
# terminal_session_resume_sync(mutable ref session: TerminalSession): void errors TerminalSessionOpenError, TerminalSessionStateError
# terminal_session_close_sync(mutable ref session: TerminalSession): TerminalCloseOutcome errors TerminalSessionRestoreError

# trusted_terminal_output_from_application_text(text: string): TrustedTerminalOutput errors AllocationFailureError
# safe_terminal_diagnostic_format(diagnostic: TerminalDiagnostic): SafeTerminalDiagnosticOutput errors AllocationFailureError
# safe_terminal_diagnostic_collection_format(diagnostics: TerminalDiagnosticCollection): SafeTerminalDiagnosticOutput errors AllocationFailureError
# terminal_pause_events_length(events: TerminalPauseEvents): int64
# terminal_pause_events_at(events: TerminalPauseEvents, index: int64): TerminalInputEvent errors IndexOutOfBoundsError
# terminal_diagnostics_length(diagnostics: TerminalDiagnosticCollection): int64
# terminal_diagnostics_at(diagnostics: TerminalDiagnosticCollection, index: int64): TerminalDiagnostic errors IndexOutOfBoundsError
# terminal_diagnostic_state(diagnostic: TerminalDiagnostic): TerminalSessionState
```

The opaque default is stable for an ABI major: alternate screen off, cursor visible, bracketed-paste transport off, trusted-paste requirement off, enhanced/focus/mouse/control capture off, requested features non-strict, 25 ms sequence timeout, 4096-byte committed/preedit/paste chunks, 1024-byte unknown/pending chunks, 1024 retained events, 1 MiB retained bytes, 64 correlated events, 64 KiB correlated bytes, 16 diagnostics, and 64 KiB diagnostic collections.

`TerminalSessionFeaturePolicy` and `TerminalSessionResourceLimits` are application-constructible closed/major-only records. Each allocation-fallible functional API replaces its entire category in an immutable options snapshot; the two calls commute and never cross-validate, so setter order is irrelevant. Final validation returns the snapshot unchanged or structured `TerminalSessionOptionsError.InvalidOptions`. Open repeats validation. Future categories use new additive APIs rather than adding required fields to either v1 record.

`require_trusted_paste_framing` is backend-independent policy. Native trusted record boundaries satisfy it even when bracketed-paste transport is disabled. Enabling bracketed paste requests one possible transport; it neither proves nor is required for trusted framing. Validation must not declare those fields conflicting.

The centralized `terminal_capabilities_feature` accepts every `TerminalOrdinaryFeature`: alternate screen, cursor shape, bracketed paste, focus, mouse buttons, mouse motion, key release, composition, and enhanced key identity. Trusted paste uses its dedicated unforgeable evidence inspector; color uses its specialized cardinality inspector. These three functions make every opaque capability field inspectable.

## Events, text phases, and identifiers

`Key` is command identity; `TextInput` is insertion. Ordinary terminals unable to separate them emit only `TextInput.Direct`. Enhanced backends may emit a Key followed by linked text. The complete Key/text group is admitted atomically against correlated and total capacity before publication. The Key is immediately followed by either one Complete chunk or adjacent Start, zero or more Continue, and End chunks with one unchanged event ID. Nothing interleaves.

TextInput phase legality is origin-specific:

- `Direct` is always `Complete`. V1 has no direct-group ID. If direct committed text exceeds the chunk limit, emit multiple independent `Direct + Complete` events in order, each split only on scalar boundaries.
- `Key(event_id)` is `Complete` or Start/Continue/End. Every chunk retains the same Key event ID and remains adjacent to that Key and its sibling chunks.
- `Composition(composition_id)` is `Complete` or Start/Continue/End. Every chunk retains one composition ID, is contiguous, and appears immediately before the matching `CompositionEnded.Committed`.

Any other origin/phase combination is an internal invariant violation and cannot be runtime-constructed. `Press(count)`/`Repeat(count)` contain the exact occurrences represented by one Key. Linked text is already expanded and is never multiplied implicitly by count. Companion chord routing emits at most one activation for that Key and carries the exact `TerminalKeyOccurrence`.

Composition order is Started, zero or more Updated, then exactly one Ended. Cursor is <= preedit scalar count. Focus loss, pause, EOF, observed cancellation, backend reset, or a new start interrupts the active composition with `CompositionEnded.Interrupted` then exactly one `InputReset.CompositionInterrupted`.

Event/composition IDs increase monotonically and are never reused within one session. Before publishing an event/group that needs a new ID, the runtime detects exhaustion and enters a sticky identifier-exhausted read state. Every later read returns `TerminalSessionReadError.IdentifierExhausted`. Detection occurs before event publication and ID assignment, not necessarily before backend consumption. Already-consumed bytes/native records remain retained under session bounds; pause surfaces them as UnknownBytes/UnknownNative where representable, and a successful close reports any remainder through `TerminalCloseOutcome.DiscardedInput`. No consumed input silently becomes a forged or reused ID.

Mouse coordinates are zero-based against the visible viewport snapshot used at decode time. A Resize carrying that snapshot is ordered before Mouse decoded against it. Later resize does not reinterpret retained coordinates.

## Trusted paste and malformed fallback

`TerminalTrustedPasteEvidence` is runtime-only and cannot be EnvironmentInferred or ProtocolAssumed. `NativeRecordBoundary` proves payload and key records are distinct. `SanitizedProtocolBoundary` proves the host removed/escaped every delimiter or control capable of ending framing or producing commands before parsing. The guarantee is only parser command isolation; it does not make text safe for a shell, editor language, renderer, or remote protocol.

After the first malformed UTF-8 byte or NUL byte in a trusted frame, the entire frame enters PasteFallback. Every payload byte already consumed, including any valid prefix, and every subsequently consumed payload byte through frame termination, EOF, pause, or backend reset is emitted only as bounded immutable `UnknownBytes`. Every fallback chunk uses the first defect's reason. No prefix/suffix becomes Paste, TextInput, Key, composition, or commands. The framing delimiter is not payload. Termination emits exactly one `InputReset.PasteFallback`, after all consumed payload bytes. Capacity pauses backend consumption; it never terminates fallback or drops bytes.

## EOF, cancellation, pause, close, and accounting

Queued decoded events precede EOF/cancellation. EOF is sticky for the Active parser generation after queued consumed input and composition interruption; later reads immediately return EndOfInput until resume/close. If cancellation and unread input become ready together after the queue empties, cancellation wins and consumes no new OS input. A cancelled token is sticky; a new source/token generation is required. Parser bytes remain for the new token unless composition interruption rules apply.

Every successful Active→Paused transition returns immutable `TerminalPauseEvents` ending in exactly one `InputReset.PauseBoundary`, even with no parser bytes. Paused→Paused is an empty non-transition. Pause constructs delivery before restoration; fit/allocation failure leaves Active, and restoration failure leaves RestorePending with no success result. Session accounting ends only after independent result retention and release of session references. Aggregate caller retention is outside the session bound.

Successful close returns `Clean` or successful `DiscardedInput`; it is never an error. Only restoration/ledger-transfer failure outranks a body error. Restoration failure retains decoder data and RestorePending ownership.

Retained bytes use checked uint64 accounting of allocation headers, variant boxes, alignment, queue records/links, payload capacity, parser/fallback/native buffers, ledger records, pause storage before transfer, and session readiness/cancellation registrations. Overflow is capacity exhaustion before allocation/consumption. Events count queue, pending native, unpublished correlated, and pre-transfer pause records. Capacity causes no-drop backpressure. Backend overflow emits bounded unknown data where possible then exactly one reset. Fixed events have a reserved-queue no-allocation fast path; single-event API shape never requires one allocation per event.

## Linux normative contract

The runtime snapshots the complete `termios` value and descriptor status flags before mutation and restores those exact snapshots. It sets descriptor `O_NONBLOCK`; clears `ICANON`, `ECHO`, `ECHONL`, `IEXTEN`, `ICRNL`, `INLCR`, `IGNCR`, `IXON`, `IXOFF`, `BRKINT`, `PARMRK`, and `ISTRIP`; sets `IGNBRK`; clears `CSIZE` then sets `CS8`; preserves parity and every unrelated control/local/input flag; clears output `OPOST` only when rendering requires raw output; and clears `ISIG` only when control-key capture is requested. It sets `VMIN=0`, `VTIME=0`, and applies changes with `TCSANOW`, which does not flush pending input.

Wait uses `poll`/`ppoll` with monotonic deadlines and terminal input, SIGWINCH notification, cancellation wakeup, and parser deadline in one host wait. `EAGAIN` means no current data and retries only within the remaining wait. Readable bytes are drained before processing `POLLHUP`; zero bytes become sticky EOF only after confirmed hangup/closure. When input and resize are both ready, currently readable input is drained first, then one newest `TIOCGWINSZ` snapshot is queued; this is deterministic observation ordering, not causal chronology. Cancellation tied with unread input wins only after already-decoded queue entries, and consumes no new bytes.

SIGWINCH uses a nonblocking self-pipe or equivalent; the handler performs only async-signal-safe notification. Original signal disposition/mask and every descriptor flag are restored exactly.

## Windows Console and ConPTY normative contract

For Console handles, the runtime snapshots input/output modes, `CONSOLE_CURSOR_INFO`, the original active screen-buffer handle/identity, and every queryable state changed by fallback output. Restoration reactivates the original buffer before closing a temporary alternate buffer, then restores cursor information and modes. It clears `ENABLE_LINE_INPUT` and `ENABLE_ECHO_INPUT`, clears `ENABLE_PROCESSED_INPUT` only for control-key capture, sets `ENABLE_WINDOW_INPUT`, and, only when mouse is requested, sets `ENABLE_MOUSE_INPUT` plus `ENABLE_EXTENDED_FLAGS` while clearing `ENABLE_QUICK_EDIT_MODE`.

Console wait combines console input and cancellation manual-reset event with `WaitForMultipleObjects`, then consumes bounded `INPUT_RECORD` batches using `ReadConsoleInputW`. Resize records trigger authoritative visible-size query. Coordinates use `srWindow`; the runtime subtracts `Left`/`Top` before validation. UTF-16 conversion retains a pending high surrogate only within bounds. An unpaired surrogate emits bounded `UnknownNative.WindowsUnknownRecord` followed by exactly one BackendReset; it is never replacement text. Modifier-only/menu records are filtered only where documented; all other unsupported records become bounded UnknownNative.

ConPTY/VT uses the same incremental parser and bounds as Linux streams. Overlapped input or a host readiness event, cancellation wake event/pipe, resize notification, and parser deadline participate in one wait. No polling loop is permitted. Handoff/restoration retains the same ledger and active-buffer guarantees as Console where applicable.

## Signals and process control

Catchable SIGINT, SIGTERM, SIGHUP, SIGQUIT, and Windows console-control events only notify the coordinator/cancellation path and return to ordinary code; they never restore terminal state in a handler. SIGTSTP notification causes ordinary code to pause, restore, and deliver its mandatory PauseBoundary before the host performs suspension. SIGCONT only marks readiness; the application must explicitly resume. Original dispositions/masks are restored. No restoration is promised after SIGKILL, TerminateProcess, power loss, or corruption.

## Output trust and existing stdout APIs

`trusted_terminal_output_from_application_text` is the explicit trust boundary for complete application-generated rendering, not input/environment/path/remote/diagnostic text. `safe_terminal_diagnostic_format` escapes C0/C1, ESC, DEL, bidi controls, noncharacters, and invalid display scalars, bounds output, and emits a visible truncation marker. Nominal write parameters make direct input rendering a type error.

Existing stdout signatures and family lists remain unchanged. Coordinator failures use `WriteFailureError.TerminalCoordinatorUnavailable`, `FlushFailureError.TerminalCoordinatorUnavailable`, and `TerminalWriteFailureError.TerminalCoordinatorUnavailable`, each with exact `state: TerminalSessionState` and `operation: TerminalOperation` payload. Their stable IDs belong to the authoritative existing standard-family declarations and the one generated ABI manifest; this prose allocates no parallel IDs. Unrelated APIs never add `TerminalSessionStateError`. Infallible `print` is best-effort escaped diagnostics only, with no delivery/ordering guarantee and possible truncation/loss.

## Verification and exclusions

Language fixtures cover canonical borrow parsing (`ref name: Type`, `mutable ref name: Type`) and reject reversed forms; one-`is` classification and `into` restrictions; core/terminal ABI ownership; one-where constrained errors; closed evolution on every constrained terminal/chord type; constructor sealing; affine close/cleanup transfer; generated-manifest ID uniqueness/history; and generic host forwarding/module pins.

Behavior fixtures cover fallible cancellation creation/destruction/token lifetime/thread moves without cross-thread refs; readiness identity/wakes/terminal states/stale Poll/removal; two-record order-independent options and validation; complete ordinary/trusted/color capability inspection including Closed bindings; trusted-paste transport independence; atomic correlation and every origin/phase legality; identifier exhaustion retention/close counts; exact malformed-frame quarantine; sticky EOF/token generations; unconditional pause reset; and all accounting/no-allocation paths.

Chord fixtures are specified in `../CHORDS.md`. Platform tests inject every Linux snapshot/restore/poll ordering, Windows Console/ConPTY mode/buffer/surrogate/viewport/wake path, catchable signal, SIGTSTP, SIGCONT, and restoration failure. Security tests reject forged evidence and direct input writes.

RPC, subprocess, watches, timers, generalized scheduling, editor buffers, and rendering policy remain outside scope. Public OS handles, flat events, polling cancellation, mutable bounded arrays, stringly diagnostics, implicit trust conversion, and unbounded retention are forbidden.
