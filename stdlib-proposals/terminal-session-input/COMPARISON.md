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
9. Serialize stdout globally and add Active-session raw endpoint lease ownership.
10. Keep public evolution additive through non-exhaustive sums, permanent discriminants, drop metadata, and ABI hashes.

## Summary matrix

| Axis | Typed Event Session | Batched Event Pump | Portable Input Packet Stream |
|---|---|---|---|
| Ergonomics | One typed event per read | Batch application loop | Editor-owned parser |
| Final event contract | Payload-bearing non-exhaustive sum | Alternative snapshot, not type-identical | Structural bytes/packets |
| Bounds | Total retained events/bytes, no-drop | Queue policy visible | Translation-byte bound |
| Malformed paste | Byte quarantine and ordered reset | Separate alternative policy | Original bytes |
| Cancellation | Token plus one OS wait set | Requires equivalent design | Requires equivalent design |
| Output ownership | Gate plus session lease | Same ownership goal | Same ownership goal |
| Implementation effort | High normalizer/parser/ledger | Higher queue ordering/coalescing | Medium-high transport/translation |

## Typed-event-session contract

Typed v1 returns one `TerminalInputEvent` from each read. Key is command identity only; ordinary text-only terminals emit TextInput without a duplicating Key. Enhanced logical-text Key identity requires `enhanced_key_identity` Enabled. TextInput is committed non-empty NUL-free UTF-8; composition commit emits TextInput before CompositionEnded.Committed. `Paste` has scalar-boundary Complete, Start, Continue, or End chunks only with trusted paste provenance. Chords consume Key only.

The public event sum uses explicit `@discriminant(1)` through `@discriminant(16)`, is non-exhaustive, has runtime-owned future-payload drop metadata, and stable ABI hashes. Tags are compiler-validated unique and never reused. Additive variants are minor-version changes; existing payload changes or removal are major ABI breaking. Refinement uses only `if value is Type.Variant into payload:` or payloadless `if value is Type.Variant:`. No match or exhaustive requirement is selected.

All listed numeric invariants use constrained nominal types; string constraints include bounded native metadata and diagnostic detail. TerminalDiagnostic uses backend, operation, stage, OS-code sum, session-state sum, retryability, and bounded detail. Error objects, not diagnostics, carry optional cause and bounded suppressed causes. Unsupported features use TerminalFeature rather than a string.

Total retained accounting covers queue records, variable payloads, parser bytes, paste fallback, native metadata, pause result memory, and pending native records. Capacity stops OS consumption until drain. The runtime reads conservatively and never silently drops. If an OS/backend overflow still occurs it surfaces bounded unknown data where possible and one BackendOverflow reset.

Pause constructs a bounded independent result before restoration. Failure to fit or allocate leaves Active and returns a structured pause-delivery error. Successful pause restores then returns events for caller application before child handoff. Close may discard unsurfaced consumed decoder data only because ownership ends, returning CloseDiscardedInput counts after restoration; a restoration failure remains primary with that diagnostic suppressed.

## Impact on Existing Syntax and Output

The proposal adds only narrow `if identifier is Type.Variant into payload`, constrained predicates, explicit `@discriminant(n)`, and fallible `using` cleanup. It does not require match, exhaustiveness, defer, slash variants, or a flat event kind.

While Active, `print`, global writers, and global terminal output route through the atomic stdout gate. During Paused, child handoff, RestorePending, FailedOpenRecovery, or FailedCloseRecovery, `stdout_writer` and `stdout_terminal` return `TerminalSessionStateError`; infallible `print` writes escaped, bounded diagnostics to independently serialized stderr. This is a fixture-backed migration behavior. Trusted rendering controls remain permitted in application output, while diagnostics and untrusted input are always escaped.

## Alternatives and effort

Batched-event-pump is deliberately an alternative snapshot: its flat batch fields do not match final typed event types. It offers potential burst throughput but makes capacity, ordering, and coalescing public policy. A future bounded `terminal_session_read_events_sync` can be added on the typed session handle after profiling without source break.

The packet stream makes the editor own escape timing, key parsing, and protocol evolution. It is useful internally or as a later expert mode but exposes more Windows translation and decoder responsibility.

## Shared lifecycle and platform contract

A ledger is registered before mutation. Open reverses completed work in reverse order. Failed open and failed close transfer their ledgers and endpoint ownership process-wide until their distinct explicit recovery APIs succeed; later opens receive RecoveryPending. Active, Paused, RestorePending, Closed, FailedOpenRecovery, and FailedCloseRecovery have explicit operation rules. Close is idempotent, finalizers are defense in depth, and no recovery is promised after uncatchable termination.

Linux snapshots complete termios and descriptor flags; explicitly uses O_NONBLOCK, VMIN=0, VTIME=0, TCSANOW, poll or ppoll, proper EAGAIN/HUP ordering, and SIGWINCH self-pipe notification. Windows snapshots modes, cursor information, active screen buffer, and fallback state; uses ReadConsoleInputW, visible srWindow dimensions, viewport-relative mouse coordinates, surrogate handling, and ConPTY/VT incremental parsing. Both integrate cancellation wakeup into the same wait set.

Capabilities are Unsupported, Available, or Enabled with compatible evidence. Color is Unsupported, Monochrome, Indexed, or TrueColor. Strict requested features fail open/resume; otherwise the portable subset is used. `trusted_paste_framing` requires backend framing or delimiter sanitization, not merely in-band bracketed delimiters; without it, no `Paste` or command-isolation claim is made.

## External comparisons

### 4.6 Cancellation

Go Context is cooperative, while net.Conn deadlines and Close wake supported operations. .NET supplies CancellationToken to supporting async/wait APIs and remains cooperative. Rust standard blocking I/O has no universal token; async runtimes select or drop futures and Tokio provides CancellationToken. Opalescent combines explicit token with an OS wakeup in the readiness wait set.

### 4.7 Bounds

Go channel capacity blocks sender progress. .NET bounded Channels default to Wait and have explicit drop modes. Rust sync_channel blocks senders and capacity zero is rendezvous. Opalescent chooses no-drop backpressure and conservative OS reads.

### 6.1 Stdout

Go os.File methods are concurrent-safe but do not give full-screen ownership. .NET offers shared Console.Out and SetOut but no endpoint lease. Rust Stdout has shared synchronization and explicit locking. Opalescent adds an exclusive Active-session lease above one global serialization gate.

### 6.3 Evolution

Go additive APIs target source compatibility without sum exhaustiveness. C# enum values can be added but poorly written switches/default assumptions risk compatibility. Rust non_exhaustive forces wildcard handling outside the crate. Opalescent uses public non-exhaustive sums, permanent discriminants, runtime drop metadata, ABI hashes, additive minor variants, and major-only payload changes or removals.

## Verification and references

Mock backends inject every snapshot, allocation, rollback, restoration, pause delivery, close, and cleanup-precedence failure. Fuzz/golden tests cover split ESC, scalar paste chunking, fallback quarantine, limits, overflow, hostile metadata, and ordering. Linux PTY and Windows Console/ConPTY integrations verify modes, size, viewport offsets, surrogates, cancellation, output leases, and explicit cleanup.

- [termios(3)](https://man7.org/linux/man-pages/man3/termios.3.html), [poll(2)](https://man7.org/linux/man-pages/man2/poll.2.html), [TIOCGWINSZ](https://man7.org/linux/man-pages/man2/TIOCGWINSZ.2const.html)
- [ReadConsoleInput](https://learn.microsoft.com/windows/console/readconsoleinput), [Windows VT](https://learn.microsoft.com/windows/console/console-virtual-terminal-sequences)
- [Go Context](https://pkg.go.dev/context), [Go compatibility](https://go.dev/doc/go1compat), [.NET channels](https://learn.microsoft.com/dotnet/core/extensions/channels), [Rust sync_channel](https://doc.rust-lang.org/std/sync/mpsc/fn.sync_channel.html), [Rust non_exhaustive](https://doc.rust-lang.org/reference/attributes/type_system.html), [Tokio CancellationToken](https://docs.rs/tokio-util/latest/tokio_util/sync/struct.CancellationToken.html)
