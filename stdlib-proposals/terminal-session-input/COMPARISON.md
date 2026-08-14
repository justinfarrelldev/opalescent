# Terminal Session and Input Comparison

## Scope and non-negotiable behavior

This concern owns exclusive interactive terminal sessions for a full-screen editor on Linux and Windows. Rendering helpers, subprocesses, pseudoterminals, watches, RPC, editor buffers, and generalized event loops remain separate.

1. Use nonconstructible runtime ownership; no public integer handle forges a session.
2. Snapshot and restore exact OS flags and modes changed by the runtime, while balancing unqueryable VT protocol changes without promising unknown prior protocol state.
3. Normalize command Key separately from committed TextInput, composition lifecycle, Paste, mouse, focus, resize, timeout, cancellation, EOF, UnknownBytes, UnknownNative, and InputReset.
4. Use constrained nominal ranges, total retained event/byte accounting, bounded paste/parser state, and no-drop backpressure.
5. Quarantine malformed/NUL bracketed paste as bytes through frame end; never turn fallback bytes into commands.
6. Use `using` with compiler-registered fallible close and cause-bearing structured errors with nominal diagnostics.
7. Pause returns bounded ordered events for caller application before child handoff; resume starts a fresh parser.
8. Use token cancellation plus OS wakeup in the same wait set; never polling cancellation.
9. Keep one host-owned stdout coordinator with operation-time checks, and permit at most one process-wide `TerminalSession` owner across Opening, Active, Paused, and RestorePending; release admission only after clean open abort, ownership-ending close, or matching recovery succeeds.
10. Keep public evolution additive through non-exhaustive sums, permanent explicit stable variant IDs, ABI history, drop metadata, and ABI hashes.

## Summary matrix

| Axis | Typed Event Session | Batched Event Pump | Portable Input Packet Stream |
|---|---|---|---|
| Ergonomics | One typed event per read | Batch application loop | Editor-owned parser |
| Final event contract | Payload-bearing non-exhaustive sum | Alternative snapshot, not type-identical | Structural bytes/packets |
| Bounds | Total retained events/bytes, no-drop | Queue policy visible | Translation-byte bound |
| Malformed paste | Byte quarantine and ordered reset | Separate alternative policy | Original bytes |
| Cancellation | Token plus one OS wait set | Requires equivalent design | Requires equivalent design |
| Output coordination | Host coordinator plus exclusive raw-session state | Same coordination goal | Same coordination goal |
| Implementation effort | High normalizer/parser/ledger | Higher queue ordering/coalescing | Medium-high transport/translation |

## Typed-event-session contract

Typed v1 returns one `TerminalInputEvent` from each read. Key is command identity only; ordinary text-only terminals emit TextInput without a duplicating Key. Enhanced logical-text Key identity requires `enhanced_key_identity` Enabled. TextInput is committed non-empty NUL-free UTF-8; composition commit emits TextInput before CompositionEnded.Committed. `Paste` has scalar-boundary Complete, Start, Continue, or End chunks only with trusted paste provenance. Chords consume Key only.

The public event sum uses inline explicit stable variant IDs 1 through 16, is non-exhaustive, has ABI history and runtime-owned future-payload drop metadata, and stable ABI hashes. IDs are compiler-validated unique, retained or retired in generated metadata, and never reused. Additive variants are minor-version changes only when boxed runtime-described payloads can retain, drop, and forward them. Existing payload changes or removal are major ABI breaking. Refinement uses only `if value is Type.Variant into payload:` or payloadless `if value is Type.Variant:`. No match or exhaustive requirement is selected.

All listed numeric invariants use constrained nominal types; string constraints include bounded native metadata and diagnostic detail. TerminalDiagnostic uses backend, operation, stage, OS-code sum, session-state sum, retryability, and bounded detail. Error objects, not diagnostics, carry optional cause and bounded suppressed causes. Unsupported features use TerminalFeature rather than a string.

Total retained accounting covers queue records, variable payloads, parser bytes, paste fallback, native metadata, pause result memory, and pending native records. Capacity stops OS consumption until drain. The runtime reads conservatively and never silently drops. If an OS/backend overflow still occurs it surfaces bounded unknown data where possible and one BackendOverflow reset.

Pause constructs a bounded independent result before restoration. Failure to fit or allocate leaves Active and returns a structured pause-delivery error. Successful pause restores then returns events for caller application before child handoff. Close may discard unsurfaced consumed decoder data only after restoration succeeds and ownership ends, returning CloseDiscardedInput counts from the Closed/Free state. Restoration failure retains the decoder data and never constructs or suppresses CloseDiscardedInput.

## Impact on Existing Syntax and Output

The proposal adds only narrow `if identifier is Type.Variant into payload`, constrained predicates, inline explicit stable variant IDs, and fallible `using` cleanup. It does not require match, exhaustiveness, defer, slash variants, or a flat event kind.

The hot-reload host owns one runtime stdout coordinator, while ordinary stdout remains shared and process-global. `stdout_writer()` and `stdout_terminal()` stay infallible singleton-view acquisition APIs. Their existing fallible operations resolve and check coordinator state at operation time and migrate to include `TerminalSessionStateError`. While Active, ordinary global output may continue but can disrupt full-screen layout, so full-screen applications should use session output. During Opening, Paused, child handoff, RestorePending, FailedOpenRecovery, or FailedCloseRecovery, application-visible fallible output operations return state errors; infallible `print` writes escaped, bounded diagnostics to stderr without touching stdout. Raw mode is active-only and general stdout is never leased, but the process-wide session reservation remains exclusive while Paused or restoration/recovery is pending.

## Alternatives and effort

Batched-event-pump is deliberately an alternative snapshot: its flat batch fields do not match final typed event types. It offers potential burst throughput but makes capacity, ordering, and coalescing public policy. A future bounded `terminal_session_read_events_sync` can be added on the typed session handle after profiling without source break.

The packet stream makes the editor own escape timing, key parsing, and protocol evolution. It is useful internally or as a later expert mode but exposes more Windows translation and decoder responsibility.

## Shared lifecycle and platform contract

The host coordinator has one process-wide terminal-session ownership slot because v1 exposes only the process's interactive standard-input/raw-output pair, not arbitrary endpoints. Each open attempt atomically reserves the slot before inspecting or mutating terminal state. Nested, concurrent, and later opens while an Opening, Active, Paused, or RestorePending session owns it fail TerminalAlreadyOwned without touching terminal state; pause retains it. A pre-mutation failure or successful in-call rollback releases the slot before returning the original open error. Only rollback failure transfers an open ledger and slot to FailedOpenRecovery. A direct close restoration failure remains session-owned RestorePending, while failed `using` cleanup transfers the close ledger and slot to FailedCloseRecovery. Later opens then receive RecoveryPending. Matching recovery success releases the slot. When the slot is Free with no recovery ledger, either recovery API is idempotent success. While the slot is occupied, a recovery API without its matching process-owned ledger—including calls while a live session owns the slot—returns RecoveryOwnerMismatch without mutation or release. An otherwise successful close releases the slot before returning ownership-ending CloseDiscardedInput, so that error never blocks another open. Active, Paused, RestorePending, Closed, FailedOpenRecovery, and FailedCloseRecovery have explicit operation rules. Close is idempotent, finalizers are defense in depth, and no recovery is promised after uncatchable termination.

Linux snapshots complete termios and descriptor flags; explicitly uses O_NONBLOCK, VMIN=0, VTIME=0, TCSANOW, poll or ppoll, proper EAGAIN/HUP ordering, and SIGWINCH self-pipe notification. Windows snapshots modes, cursor information, active screen buffer, and fallback state; uses ReadConsoleInputW, visible srWindow dimensions, viewport-relative mouse coordinates, surrogate handling, and ConPTY/VT incremental parsing. Both integrate cancellation wakeup into the same wait set.

Capabilities are Unsupported, Available, or Enabled with compatible evidence. Color is Unsupported, Monochrome, Indexed, or TrueColor. Strict requested features fail open/resume; otherwise the portable subset is used. `trusted_paste_framing` requires backend framing or delimiter sanitization, not merely in-band bracketed delimiters; without it, no `Paste` or command-isolation claim is made.

## External comparisons

### 4.6 Cancellation

Go Context is cooperative, while net.Conn deadlines and Close wake supported operations. .NET supplies CancellationToken to supporting async/wait APIs and remains cooperative. Rust standard blocking I/O has no universal token; async runtimes select or drop futures and Tokio provides CancellationToken. Opalescent combines explicit token with an OS wakeup in the readiness wait set.

### 4.7 Bounds

Go channel capacity blocks sender progress. .NET bounded Channels default to Wait and have explicit drop modes. Rust sync_channel blocks senders and capacity zero is rendezvous. Opalescent chooses no-drop backpressure and conservative OS reads.

### 6.1 Stdout

Go os.File methods are concurrent-safe but do not give raw-session coordination. .NET offers shared Console.Out and SetOut without full-screen ownership. Rust Stdout has shared synchronization and explicit locking. Opalescent keeps stdout shared, makes raw mode active-only, and separately retains one process-wide session reservation while Paused or restoration/recovery is pending. A host-owned mutex is deferred until concurrent user execution exists, and never spans child execution.

### 6.3 Evolution

Go additive APIs target source compatibility without sum exhaustiveness. C# enum values can be added but poorly written switches/default assumptions risk compatibility. Rust non_exhaustive forces wildcard handling outside the crate. Opalescent uses public non-exhaustive sums, permanent inline stable variant IDs, ABI history, runtime drop metadata, ABI hashes, additive minor variants when boxed runtime-described payloads are available, and major-only payload changes or removals.

## Verification and references

Mock backends inject every snapshot, allocation, rollback, restoration, pause delivery, close, and cleanup-precedence failure. They also verify atomic single-session admission, nested/concurrent `TerminalAlreadyOwned`, Paused reservation retention, and `RecoveryPending` until successful recovery. Fuzz/golden tests cover split ESC, scalar paste chunking, fallback quarantine, limits, overflow, hostile metadata, inline-ID parsing and ABI-history no-reuse, and ordering. Linux PTY and Windows Console/ConPTY integrations verify modes, size, viewport offsets, surrogates, cancellation, operation-time output coordination, explicit child handoff, and cleanup.

- [termios(3)](https://man7.org/linux/man-pages/man3/termios.3.html), [poll(2)](https://man7.org/linux/man-pages/man2/poll.2.html), [TIOCGWINSZ](https://man7.org/linux/man-pages/man2/TIOCGWINSZ.2const.html)
- [ReadConsoleInput](https://learn.microsoft.com/windows/console/readconsoleinput), [Windows VT](https://learn.microsoft.com/windows/console/console-virtual-terminal-sequences)
- [Go Context](https://pkg.go.dev/context), [Go compatibility](https://go.dev/doc/go1compat), [.NET channels](https://learn.microsoft.com/dotnet/core/extensions/channels), [Rust sync_channel](https://doc.rust-lang.org/std/sync/mpsc/fn.sync_channel.html), [Rust non_exhaustive](https://doc.rust-lang.org/reference/attributes/type_system.html), [Tokio CancellationToken](https://docs.rs/tokio-util/latest/tokio_util/sync/struct.CancellationToken.html)
