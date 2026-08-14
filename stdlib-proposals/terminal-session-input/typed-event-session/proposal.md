# Typed Event Session

## Overview and status

This is the recommended public v1 terminal-input API for a full-screen editor. A compiler-registered, nonconstructible `TerminalSession` exclusively owns the process's interactive standard-input/raw-output terminal pair. V1 does not open arbitrary terminal endpoints: each process has exactly one candidate pair, and at most one non-Closed `TerminalSession` may exist for it at a time. The runtime owns raw modes, VT parsing, Windows record translation, feature negotiation, bounded retention, cancellation wakeups, and restoration. Application code sees no file descriptor, Win32 handle, UTF-16 code unit, or escape sequence.

Opalescent already declares nominal sums with one `type` and nested payload-bearing or payloadless variants, as shown in `language-spec/types_example.types.op` and the Crash Course. Payload variants are constructed with `new Type.Variant:`. Parser and checker support for those established declarations and constructors exists. Code generation, payload refinement, ABI, payload-layout evolution, and runtime drop support remain incomplete, so this proposal does not claim its examples compile today. It extends existing sum types, it does not invent TypeScript-style structural `A | B` unions.

## Language prerequisites

### Narrow non-exhaustive refinement

Only these forms are required:

```opal
if event is TerminalInputEvent.Key into key_event:
    handle_key(key_event)

if event is TerminalInputEvent.Cancelled:
    stop_editor()
```

Use `Type.Variant` consistently. A payload binding is immutable and branch-local. Initially only a direct identifier scrutinee narrows; compound conditions do not narrow, and `is not` does not expose a payload. Untested future variants are ignored. This proposal requires neither match syntax nor exhaustive handling.

### Constrained types and cleanup

`public constrained type Name: BaseType where <predicate>` is required. Constants and literals may cast only when proved. Runtime values require a checked constructor or cast returning a constraint error. Distinct constrained nominal types are not implicitly interchangeable. `contains_no_nul(value)` and `utf8_byte_length(value)` are required constexpr/runtime predicates for string constraints. Every numeric range and string invariant is documented in `typed_event_session.types.op`.

### Explicit stable variant IDs and formatter safety

A payload variant uses `Variant = positive_integer:` and a payloadless variant uses `Variant = positive_integer`. `TerminalInputEvent` assigns stable variant IDs 1 through 16 inline in `typed_event_session.types.op`. A public ABI-stable non-exhaustive sum requires every variant to have a unique explicit uint64-range ID. Mixed implicit and explicit IDs are rejected, and declaration order has no ABI meaning. Private or closed sums may use implicit declaration order.

Generated ABI metadata and manifests retain active and retired IDs, payload layout hashes, representation version, and drop metadata. Removing an ID retires it permanently, it cannot be reused. Source declarations remain authoritative. A future `reserved 3` spelling may make an intentional retirement visible in source, but is not required now. The compiler validates inline-ID parsing, uniqueness, uint64 range, and no reuse against ABI history, then rejects incompatible declarations.

Unknown additive variants need boxed, runtime-described payloads with runtime-owned retain, drop, and forward operations. Without that representation, adding a variant is an ABI and hot-reload break even with a stable ID. Closed or private sums may use optimized inline layout.

The standard safe diagnostic formatter accepts a `TerminalDiagnostic` or bounded diagnostic array and produces bounded display text with every control character escaped. Runtime diagnostics and untrusted input are never directly written to either terminal stream. This does not constrain trusted application rendering strings: full-screen rendering legitimately contains terminal controls.

`using resource = fallible_expression:` is the only cleanup syntax needed. This proposal does not add or depend on `defer`. `TerminalSession` has compiler-registered fallible cleanup `terminal_session_close_sync`; every enclosing function declares cleanup errors. Cleanup runs on fallthrough, return, propagate, break, and continue that leave scope, in reverse nesting order. Body failure plus cleanup failure makes cleanup/restoration primary and body failure its error-value `cause`; all cleanups run and later failures become bounded error-value `suppressed_causes`.

The language error ABI must provide stable family and variant identity, structured payload, optional error-value cause with bounded depth and count, and bounded suppressed causes. A pointer or string-only i8 ABI is insufficient. The explicit spelling `propagate fallible_call() cause prior_error` means that if the call fails, its error is propagated as primary with `prior_error` attached as its cause; on success execution continues normally. This cause form is required for failed-open recovery and is valid only when the propagated error and cause are both live error values accepted by the enclosing function's declared error set.

## API surface

```opal
# CancellationSource and CancellationToken are opaque language-level resources.
# cancellation_source_new(): CancellationSource
# cancellation_token(source: CancellationSource): CancellationToken
# cancellation_request(source: CancellationSource): void
#
# options contains constrained sequence, paste, retained-event, and retained-byte limits.
# terminal_session_open_sync(options: TerminalSessionOptions): TerminalSession errors TerminalSessionOpenError
# terminal_session_restore_pending_open_sync(): void errors TerminalSessionRestoreError
# terminal_session_restore_pending_close_sync(): void errors TerminalSessionRestoreError
# terminal_session_capabilities(session: TerminalSession): TerminalCapabilities
# terminal_session_output_terminal(session: TerminalSession): StdoutTerminal errors TerminalSessionStateError
# terminal_session_size_sync(session: TerminalSession): TerminalSize errors TerminalSessionReadError, TerminalSessionStateError
# wait is Poll, Forever, or For(milliseconds: TerminalWaitMilliseconds 1..2147483647).
# terminal_session_read_event_sync(session: TerminalSession, wait: TerminalWait, cancellation: CancellationToken): TerminalInputEvent errors TerminalSessionReadError, TerminalSessionStateError
# text is trusted application output; input payloads must never be passed directly.
# terminal_session_write_sync(session: TerminalSession, text: string): void errors TerminalSessionWriteError, TerminalSessionStateError
# terminal_session_flush_sync(session: TerminalSession): void errors TerminalSessionWriteError, TerminalSessionStateError
# terminal_session_set_cursor_visible_sync(session: TerminalSession, visible: boolean): void errors TerminalSessionWriteError, TerminalSessionStateError
# terminal_session_set_cursor_shape_sync(session: TerminalSession, shape: TerminalCursorShape): void errors TerminalSessionWriteError, TerminalSessionStateError
# returned events are independently retained and bounded by configured retained limits.
# terminal_session_pause_sync(session: TerminalSession): TerminalPauseResult errors TerminalSessionReadError, TerminalSessionRestoreError, TerminalSessionStateError
# terminal_session_resume_sync(session: TerminalSession): void errors TerminalSessionOpenError, TerminalSessionStateError
# terminal_session_close_sync(session: TerminalSession): void errors TerminalSessionRestoreError
# Reserved future API only, not v1: terminal_session_read_events_sync(session, wait, cancellation, maximum_events) returns a bounded ordered event list.
```

`TimedOut`, `Cancelled`, and `EndOfInput` are ordinary events, never read errors. `Poll` returns `TimedOut` only when no queued event or due parser deadline is available. A caller deadline never prematurely classifies an incomplete ESC sequence. Reads first return already-decoded queue entries. Once the queue is empty, cancellation and unread input becoming ready together linearize as cancellation first: `Cancelled` is returned and no new OS input is consumed. A cancelled token remains cancelled, so a reset or new token is required for a later read. Parser state remains for later reads unless pause or close resets it. EOF follows already decoded input and surfaced consumed bytes.

The future batching signature is reserved without source break. Implementation must preserve an explicit comment at the single-event/buffer boundary so backends and parser do not allocate intrinsically per returned event.

## Event, text, composition, and evolution contract

`TerminalInputEvent` is a public non-exhaustive payload-bearing sum, never a flat discriminator product. Its permanent explicit stable variant IDs are declared inline in the type file. Unknown future payloads travel through old modules only through boxed runtime-described payloads with runtime-owned retain, drop, and forward operations, so known refinement tests remain safe. ABI metadata retains active and retired IDs, payload-layout hashes, representation version, and drop metadata. Minor releases may add variants only when that representation is available. Existing variants, stable variant IDs, and payload layouts change or disappear only in a major ABI-breaking release. Removed IDs are retired and never reused. No fake reserved variants exist.

`Key` is command identity, not inserted text. `TerminalLogicalKey.Text` exists only where `enable_enhanced_key_identity` negotiated `enhanced_key_identity` as Enabled and the backend reports layout-resolved logical identity independently. Ordinary terminal character input that cannot make that distinction emits `TextInput` only. `Key` has no physical-key identity in v1. A `TextInput.Key(event_id)` is emitted only where a Key and committed text came from the same gesture. Legacy control keys use `TerminalLogicalKey.Control`; EnhancedText chord registration requires `enhanced_key_identity` Enabled and validation rejects it otherwise.

`TextInput` contains committed non-empty NUL-free UTF-8. Applications insert it. `CompositionStarted`, `CompositionUpdated`, and `CompositionEnded` maintain preedit state; updated cursor is a constrained scalar index checked against scalar length. Commit emits `TextInput.Composition(composition_id)` before `CompositionEnded.Committed`. Without preedit support, backends emit only `TextInput` and capability reports `composition_events` Unsupported. `Paste` contains `TerminalPasteText` plus Complete, Start, Continue, or End phase. Valid paste is chunked on Unicode scalar boundaries within the configured byte limit. Paste, composition, and TextInput never match chords.

IDs are monotonically allocated per session, unique until Close. Allocation exhaustion is a structured allocation failure; no identifier is reused. IDs are assigned only to emitted Key and composition lifecycle records, never to raw fallback bytes.

`UnknownBytes` and `UnknownNative` are the only unknown-event forms. Native kind uses known structural variants or bounded `Other(name)`; its primitive code remains because OS code domains are open. Neither raw bytes nor native metadata are terminal commands.

## Total bounds and malformed paste

Options contain constrained paste chunk, incomplete-sequence, retained-event, and retained-byte limits. Retained accounting covers queued events, variable payloads, parser bytes, paste fallback bytes, native metadata, pause delivery result storage, and pending native records. The configured values are cross-validated so total retained capacity can hold required state. At capacity the runtime stops consuming OS records and bytes until callers drain. It reads conservatively according to remaining capacity. It never silently drops input or resets merely from capacity pressure. Backend or OS overflow despite backpressure emits bounded UnknownBytes or UnknownNative where possible, followed by exactly one `InputReset.BackendOverflow` for that discontinuity.

Bracketed-paste delimiters carried in-band are not intrinsically unforgeable. `Paste` and paste-to-command isolation are emitted only when `trusted_paste_framing` is Enabled, meaning the backend or terminal provides trusted framing or guarantees delimiter sanitization. With `require_trusted_paste_framing`, open and resume fail `UnsupportedFeature(TrustedPasteFraming, diagnostic)` when unavailable. In non-strict mode without it, the runtime emits no `Paste`, claims no isolation, and emits `TextInput` or `Key` according to the backend. Security-sensitive full-screen editors request trusted framing. Windows Console/native paste must provide trustworthy provenance or reports the feature Unsupported.

NUL or malformed UTF-8 in an already trusted paste frame enters bounded PasteFallback. Until closing delimiter, EOF, pause, or backend reset, it emits only bounded UnknownBytes with PasteContainsNul or PasteInvalidUtf8; it never reinterprets fallback bytes as Key, TextInput, composition, or commands. Capacity stops further OS consumption and waits for a drain, it is not a fallback termination or reset. Closing delimiter is framing, not payload. Split or embedded closing-delimiter-looking bytes and control/function bytes are fixture-tested: a trusted backend sanitizes the payload or preserves the frame so payload cannot become commands. After termination, exactly one `InputReset.PasteFallback` is emitted and parsing returns to ground. On EOF, pause, and backend reset, every already consumed byte is surfaced before the single reset. Sequence-limit recovery similarly surfaces bounded bytes then `InputReset.SequenceLimitExceeded`.

## Capability negotiation

Feature state is a sum: Unsupported, Available, or Enabled, each with only compatible evidence. Color is Unsupported, Monochrome, Indexed(count, evidence), or TrueColor(evidence). `require_requested_features` makes a requested unavailable feature fail open or resume with `UnsupportedFeature(feature, diagnostic)`; `require_trusted_paste_framing` independently makes trusted framing unavailable fail in the same way. Otherwise the runtime enables the portable subset. Cached capabilities are readable while paused or recovery-pending, but Active-only session operations fail in Paused, RestorePending, FailedOpenRecovery, FailedCloseRecovery, or Closed. `terminal_session_resume_sync` called while Active fails with `TerminalSessionStateError.SessionActive`; every other invalid transition reports its actual structured state through `TerminalDiagnostic` and the closest state-error variant. Session output uses `TerminalOperation.AcquireOutputTerminal` only for diagnostics, while existing fallible output operations validate coordinator state at operation time.

## Linux contract

The runtime snapshots complete `termios` and descriptor status flags, sets `O_NONBLOCK`, and restores exact prior flags. It clears `ICANON`, `ECHO`, `ECHONL`, `IEXTEN`, `ICRNL`, `INLCR`, `IGNCR`, `IXON`, `IXOFF`, `BRKINT`, `PARMRK`, and `ISTRIP`, and sets `IGNBRK` so break input is ignored rather than unexpectedly signaling or flushing; it preserves the original complete snapshot exactly on restore, including all parity and strip flags; uses eight-bit input; disables output post-processing when rendering requires it; and clears `ISIG` only when capture_control_keys is enabled. It sets `VMIN=0`, `VTIME=0`, and uses `poll` or `ppoll` with monotonic deadlines. `TCSANOW` does not flush pending input. `EAGAIN` means no data and retries only within remaining wait. Readable input is drained before `POLLHUP`; zero bytes become EndOfInput only after confirmed hangup or closure.

SIGWINCH uses a nonblocking self-pipe or equivalent in the same wait set; its handler performs only async-signal-safe notification and original disposition/mask are restored. If input and resize are both ready, currently readable bytes are drained before one newest `TIOCGWINSZ` snapshot. This is deterministic observation, not causal chronology. Cancellation joins this same wait set through eventfd or a nonblocking self-pipe, not polling.

## Windows contract

For Console handles, the runtime snapshots input and output modes, `CONSOLE_CURSOR_INFO`, original active screen-buffer handle and identity, and every queryable state changed by fallback output. Exact restoration reactivates original buffer before closing any temporary alternate buffer and restores cursor information. It clears line and echo input, clears `ENABLE_PROCESSED_INPUT` only when capturing control keys, enables window input, and enables mouse input plus extended flags while disabling Quick Edit when mouse is requested.

It waits on console input and cancellation manual-reset event with `WaitForMultipleObjects`, then reads bounded `INPUT_RECORD` batches using `ReadConsoleInputW`. Resize records wake size query rather than define authoritative size. The runtime derives visible dimensions from `GetConsoleScreenBufferInfo.srWindow`, subtracts `Left` and `Top` before validating zero-based mouse coordinates, and converts UTF-16 through surrogate-pair handling. An unpaired UTF-16 surrogate deterministically emits `UnknownNative` with `WindowsUnknownRecord` metadata, then exactly one `InputReset.BackendReset`; it is never replaced. Modifier-only and menu records are filtered where documented; remaining unknown records use bounded `UnknownNative` metadata. ConPTY and VT streams use the incremental VT parser, with overlapped I/O or a readiness event plus a cancellation wake event or wake pipe in one wait set.

## Lifecycle, pause, recovery, close, and signals

States are Opening, Active, Paused, RestorePending, Closed, and process-owned FailedOpenRecovery or FailedCloseRecovery. The host coordinator exposes one process-wide terminal-session ownership slot. Each open attempt atomically reserves it before inspecting or mutating terminal state; nested, concurrent, or later opens while an Opening, Active, Paused, or RestorePending session owns the slot fail `TerminalAlreadyOwned` without inspecting or mutating terminal state. FailedOpenRecovery or FailedCloseRecovery owns the slot without a usable session, so opens instead fail `RecoveryPending`. Pausing never releases the slot. A failure detected before the first terminal mutation releases the provisional Opening reservation before returning its original open error. Before mutation, open creates a ledger recording each successfully applied change, inverse operation, order, and completion. Open snapshots then mutates transactionally. A partial open failure reverses successful steps in reverse order. Successful rollback releases the slot before returning the original open error. Only rollback failure returns `RollbackFailed` and transfers the ledger plus slot to process-owned FailedOpenRecovery; later opens receive `RecoveryPending`.

The two explicit recovery APIs are ledger-kind-specific. With no recovery ledger and a Free slot, either is idempotent success. A matching API retains the slot until every restoration step succeeds, then releases it; failure leaves the matching recovery state and returns `PendingOpenRollbackFailed` or `PendingCloseRestoreFailed`. While the slot is occupied, a recovery call without its matching process-owned ledger returns `RecoveryOwnerMismatch` with the actual Opening, Active, Paused, RestorePending, FailedOpenRecovery, or FailedCloseRecovery state and performs no mutation or release. This includes calling the opposite recovery API and calling either recovery API while a live session owns the slot. Recovery failure is primary and the original open or close failure is its structured cause.

Pause from Active first drains and constructs the independently retained bounded result. If partial parser state existed, the result ends with exactly one `InputReset.PauseBoundary`; otherwise it contains no pause-boundary reset. If every already-consumed parser byte and the required PauseBoundary reset fit, it restores OS modes and protocol reversals, transitions Paused, then returns the result. If result allocation or fit fails before restoration starts, pause returns PauseDeliveryFailed and remains Active. If restoration begins and then fails, state is RestorePending with its ledger; no success result is claimed. The caller applies returned events before child handoff. Pause from Paused returns an empty bounded result. Resume only succeeds from Paused. Resume from Active, Closed, or RestorePending returns `TerminalSessionStateError` with the structured current state. From Paused it reapplies options transactionally, refreshes capabilities and size, starts a fresh parser, and never replays pause events. Failed resume remains Paused unless rollback fails, then RestorePending.

Close from Active performs pause-like restoration and releases ownership only after every required restoration step succeeds. A failed direct `terminal_session_close_sync` leaves the usable binding in RestorePending with its ledger and slot still session-owned; retrying close resumes remaining restoration, and competing opens fail `TerminalAlreadyOwned`. If compiler-registered `using` cleanup cannot restore before the binding leaves scope, it transfers the ledger and slot to process-owned FailedCloseRecovery and returns `CloseRestorePending`; later opens fail `RecoveryPending` until close recovery succeeds. Close from Paused releases the slot, close from RestorePending retries remaining restoration, and close from Closed is idempotent.

Close may discard still-unsurfaced consumed decoder bytes only on an otherwise successful ownership-ending close. After restoration, it transitions Closed and releases the slot before returning `CloseDiscardedInput` with structured discarded byte/event counts and no raw bytes. `CloseDiscardedInput` is therefore an error-valued notice but never creates recovery state and never blocks a later open. If restoration fails, close retains the decoder data with the restoration ledger, reports only the restoration failure, and does not construct or suppress `CloseDiscardedInput`; discard accounting is produced only if a later retry completes restoration and closes ownership. Explicit close through `using` is normal cleanup; finalizers and process-exit cleanup are defense in depth only. No restoration is promised after SIGKILL, TerminateProcess, power loss, or corruption.

Catchable SIGINT, SIGTERM, SIGHUP, SIGQUIT, and Windows console control events only notify the coordinator or cancellation path and reach ordinary `using` cleanup. SIGTSTP reaches ordinary pause and restoration before suspension. SIGCONT requires explicit resume. No unsafe restoration occurs inside a signal handler. Uncatchable termination has no restoration guarantee. Thread-affinity enforcement is deferred until concurrent user execution exists; cancellation request remains safe for the signal coordinator.

## Impact on Existing Output APIs

The hot-reload host owns one long-lived runtime terminal/stdout coordinator because it owns process state across reloads. Ordinary stdout remains a shared process-global sink. `stdout_writer()` and `stdout_terminal()` keep their existing infallible acquisition signatures: their singleton handles are views, not independently owned terminal endpoints. Each existing fallible writer or terminal operation resolves and checks current coordinator state at operation time, so its error set migrates to include `TerminalSessionStateError`. A view acquired before session open is neither permanently valid nor permanently invalid, it follows current state and can be reused after resume.

`TerminalSession` alone owns raw-mode and full-screen endpoint state. The process-wide ownership slot permits at most one owning session across Opening, Active, Paused, and RestorePending, because mode changes, parser state, handoff, and restoration cannot be partitioned safely between sessions. Paused retains ownership so another session cannot mutate the terminal before the original session resumes or closes. Ordinary global output may continue while Active through the operation-time coordinator checks, although it can disrupt layout. Full-screen applications should use session output. Provenance enforcement between `print` and session output is not a v1 prerequisite.

During Opening, Paused, RestorePending, FailedOpenRecovery, FailedCloseRecovery, and child handoff, application-visible fallible writer and terminal operations return state errors; only coordinator-internal open/restoration operations may touch stdout. Infallible `print` must not touch stdout in those states and instead retains the escaped bounded fallback to stderr. Child handoff is explicit: flush, pause and restore, transition to suppress parent output, let the child inherit descriptors, wait, then explicitly resume. A parent mutex cannot serialize an inherited child descriptor, so no lock is held across child execution.

A global mutex is deferred until concurrent user execution exists. At that point all output calls route through one host-owned mutex that locks one logical runtime call, never individual bytes. General async output and batching remain deferred until evidence or profiling warrants them. The v1 requirements are raw-session exclusivity, restoration and recovery, operation-time coordinator checks, explicit handoff, and host-owned coordination.

## External precedent and research rationale

### 4.6 Cancellation

Go Context is cooperative; supported `net.Conn` operations can wake through deadlines or Close. .NET passes CancellationToken to supporting asynchronous and wait APIs and remains cooperative. Rust standard blocking I/O has no universal token; async runtimes select or drop futures and Tokio offers CancellationToken. Opalescent chooses an explicit token plus an OS wakeup in the same wait set.

### 4.7 Bounds

Go channel capacity blocks a sender. .NET bounded Channels default to Wait and offer explicit drop modes. Rust `sync_channel` blocks senders and capacity zero is rendezvous. Opalescent chooses no-drop backpressure and conservative OS reads.

### 6.1 Stdout

Go os.File methods are concurrent-safe but do not provide raw-session coordination. .NET exposes shared Console.Out and SetOut without full-screen ownership. Rust Stdout has shared synchronization and explicit locking. Opalescent keeps general stdout shared, makes raw mode active-only, and separately retains one process-wide session reservation until a clean open abort, ownership-ending close, or matching recovery succeeds.

### 6.3 Evolution

Go prioritizes additive source-compatible APIs without sum exhaustiveness. C# enum values can be added but switches and default assumptions may be unsafe. Rust non_exhaustive requires wildcard handling outside the defining crate. Opalescent uses public non-exhaustive sums, permanent explicit stable variant IDs, ABI history, runtime drop metadata, ABI hashes, minor-only additive variants when boxed runtime-described payloads are available, and major-version payload changes or removal.

## Strengths

- One portable event at a time with typed text, command identity, composition, paste, diagnostics, and discontinuities.
- Bounded no-drop retention and lossless malformed-paste quarantine.
- Explicit ownership, cancellation wakeup, recovery ledger, and structured cleanup causes.
- Centralized Linux, Console, ConPTY, Unicode, and viewport normalization.

## Weaknesses

- The standard library must maintain VT and Windows normalizers, restoration ledgers, and cross-version payload drop metadata.
- One-event v1 can add runtime crossings during bursts; profiling alone may justify future bounded batching.
- The language prerequisites are substantial and must be fixture-backed before public release.

## Interactions and implementation difficulty

Subprocesses, pseudoterminals, jobs, filesystem watches, RPC, generalized event loops, and editor buffers remain separate concerns. A future event-loop API may wrap the same wakeable wait primitive. Implementation difficulty is high: deterministic mock backends inject failure at each snapshot, allocation, rollback, restoration, and cleanup-precedence step; parser fuzz/golden tests cover split sequences, ESC deadline races, scalar chunking, malformed paste, unknown native records, overflow, and hostile metadata; integrations require Linux PTYs plus Windows Console and ConPTY.

## Verification and Must NOT Have

Fixtures must verify exact refinement restrictions, constrained construction, `contains_no_nul`, `utf8_byte_length`, established sum construction/drop, inline stable variant-ID parsing, uint64 uniqueness, ABI-history no-reuse rejection, safe diagnostic escaping/bounds, cause depth/count and suppressed-cause bounds, pause-result transactionality and exactly-one partial-parser boundary reset, process-wide single-session admission including nested/concurrent open rejection, clean failed-open release, Paused ownership retention, direct-close RestorePending ownership, `using` close-recovery transfer, Free-state recovery idempotence, recovery-owner mismatch from every live and opposite-recovery state, ownership-ending CloseDiscardedInput only after successful restoration, no-drop backpressure, cancellation linearization/wakeups, lifecycle recovery, operation-time stdout coordination including Opening suppression, explicit child handoff, and infallible `print` stderr migration. Trusted-paste fixtures cover embedded and split closing delimiters plus control/function bytes. Integration verifies exact termios flags including IGNBRK/IXOFF, SIGWINCH and catchable signal coordination, Windows modes/screen buffers/unpaired surrogates/viewport offsets, ConPTY cancellation waits, and explicit close on every post-open path.

Must NOT have public platform handles, flat event kinds, generic unknown events, numeric absence sentinels, unbounded paste/parser retention, replacement or direct rendering of untrusted input, polling cancellation, a claim of exact restoration for unqueryable VT protocol state, abandoned failed-open ledger, finalization as normal cleanup, or restoration guarantees after uncatchable termination.

## Research references

- [termios(3)](https://man7.org/linux/man-pages/man3/termios.3.html)
- [poll(2)](https://man7.org/linux/man-pages/man2/poll.2.html)
- [TIOCGWINSZ](https://man7.org/linux/man-pages/man2/TIOCGWINSZ.2const.html)
- [Windows ReadConsoleInput](https://learn.microsoft.com/windows/console/readconsoleinput)
- [Windows virtual terminal sequences](https://learn.microsoft.com/windows/console/console-virtual-terminal-sequences)
- [Go Context](https://pkg.go.dev/context) and [Go compatibility](https://go.dev/doc/go1compat)
- [.NET CancellationToken](https://learn.microsoft.com/dotnet/api/system.threading.cancellationtoken) and [bounded channels](https://learn.microsoft.com/dotnet/core/extensions/channels)
- [Rust sync_channel](https://doc.rust-lang.org/std/sync/mpsc/fn.sync_channel.html), [Rust non_exhaustive](https://doc.rust-lang.org/reference/attributes/type_system.html), and [Tokio CancellationToken](https://docs.rs/tokio-util/latest/tokio_util/sync/struct.CancellationToken.html)
