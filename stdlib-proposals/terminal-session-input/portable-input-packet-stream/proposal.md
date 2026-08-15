# Portable Input Packet Stream

## Historical status and selected-contract deference

This remains a historical expert transport alternative, not selected public v1 and not implemented support. Its complete recorded API, transport, parser, platform, lifecycle, example, and tradeoff body is preserved as historical design context rather than redesigned here. For any future design work, the selected [typed-event-session contract](../typed-event-session/proposal.md), [core prerequisites](../core-prerequisites.md), [active declarations](../typed-event-session/typed_event_session.types.op), [ABI history](../typed-event-session/abi-history.md), [chord contract](../CHORDS.md), and [test-only contract](../TESTING.md) control all cross-cutting rules.

Historical stream copies, output views, `StdoutTerminal`, direct byte writes, pause/close/retry prose, packet readiness, and parser fixtures grant no selected ownership, recovery, process-control, diagnostic, readiness, timer, test, trust, chord, or ABI authority. Any future transport must preserve the selected affine ownership/runtime state machine and legacy-I/O coordination; lifecycle-issued typed recovery tokens and rejection-before-mutation; separate process-control notifications and explicit resume; structured diagnostics/attachments; generic wait/timer ordering; chord correlation/lifecycle; explicit output declassification with no session-derived generic terminal; test-only sealed construction; and active-declaration/append-only-history authority. Canonical input bytes establish no rendering trust, packets never carry process-control authority, and application parser fixtures cannot forge terminal values or recovery tokens.

## Overview

This alternative exposes a compiler-registered, nonconstructible `TerminalInputStream` and delivers canonical terminal bytes in bounded `uint8[]` chunks. The stdlib owns mode setup, readiness, resize framing, Windows translation, output-mode setup, pause/resume, and restoration; the editor owns key parsing, ESC disambiguation, paste parsing, and protocol evolution.

Linux and VT/ConPTY bytes are preserved. Native Windows records are translated to a documented xterm-compatible vocabulary: UTF-8 text, CSI/SS3 keys, modifier parameters, SGR mouse, and focus sequences. Resize and unknown native records use structural packets because they have no original VT bytes.

## Assumes

- `TerminalInputStream` is an opaque runtime type that cannot be constructed from an integer or inspected; copies share validated ownership.
- `uint8[]` is the portable binary representation and supports NUL/partial UTF-8.
- The editor maintains an incremental parser across arbitrary packet boundaries.
- Existing `StdoutTerminal` rendering can receive a stream-owned output view.
- No deferred syntax is needed; potentially blocking transport functions end in `_sync`.

## Syntax Design

```opal
# terminal_input_stream_open_sync(options: TerminalInputStreamOptions): TerminalInputStream errors TerminalInputStreamOpenError
# terminal_input_stream_capabilities(stream: TerminalInputStream): TerminalCapabilities
# terminal_input_stream_output_terminal(stream: TerminalInputStream): StdoutTerminal errors TerminalInputStreamStateError
# terminal_input_stream_size_sync(stream: TerminalInputStream): TerminalSize errors TerminalInputStreamReadError, TerminalInputStreamStateError
# terminal_input_stream_read_sync(stream: TerminalInputStream, timeout_milliseconds: int32, maximum_bytes: int32): TerminalInputPacket errors TerminalInputStreamReadError, TerminalInputStreamStateError
# terminal_input_stream_write_sync(stream: TerminalInputStream, bytes: uint8[]): void errors TerminalInputStreamWriteError, TerminalInputStreamStateError
# terminal_input_stream_flush_sync(stream: TerminalInputStream): void errors TerminalInputStreamWriteError, TerminalInputStreamStateError
# terminal_input_stream_set_cursor_visible_sync(stream: TerminalInputStream, visible: boolean): void errors TerminalInputStreamWriteError, TerminalInputStreamStateError
# terminal_input_stream_set_cursor_shape_sync(stream: TerminalInputStream, shape: TerminalCursorShape): void errors TerminalInputStreamWriteError, TerminalInputStreamStateError
# terminal_input_stream_pause_sync(stream: TerminalInputStream): void errors TerminalInputStreamRestoreError, TerminalInputStreamStateError
# terminal_input_stream_resume_sync(stream: TerminalInputStream): void errors TerminalInputStreamOpenError, TerminalInputStreamStateError
# terminal_input_stream_close_sync(stream: TerminalInputStream): void errors TerminalInputStreamRestoreError
```

Packet kinds are:

- `Bytes`: one or more canonical bytes; boundaries may split UTF-8 and control sequences.
- `Resize`: newest visible positive dimensions.
- `TimedOut`: caller deadline expired.
- `EndOfInput`: endpoint closed after preceding bytes were delivered.
- `NativeEvent`: an unknown Windows native record represented by `native_event_name` and `native_event_code` rather than fabricated bytes.
- `DecoderReset`: pause/resume discarded transport-level partial translation; the application must clear its pending parser bytes before consuming later byte packets.

`timeout_milliseconds` uses `-1`, `0`, and positive values. `maximum_bytes` must be at least 64 so one canonical native record can always be split/delivered safely. `maximum_pending_bytes` bounds runtime translation/transport state; output from one record may span packets but never exceeds this internal bound. Timeout and EOF are packets, not errors.

Optional features report supported/enabled/evidence state. Strict requested features fail open; non-strict requests use the portable subset.

## Canonical Byte Contract

- Printable input is UTF-8; control bytes including NUL/Escape are preserved.
- Arrow/edit/function keys use documented CSI/SS3 forms.
- Modifiers use xterm numeric parameters where canonical forms exist.
- Mouse uses SGR mouse bytes. Its wire coordinates remain one-based; the application decoder converts to zero-based Opalescent cells.
- Focus uses `CSI I`/`CSI O`.
- Bracketed paste preserves `CSI 200~`, payload, and `CSI 201~` when the backend identifies paste boundaries.
- Unknown VT sequences pass byte-for-byte.
- Packet boundaries convey no character or sequence boundary.

### Native Windows filtering/translation

- Key-down records become canonical key/text bytes. Repeat counts are emitted lazily from one retained fixed-size native record plus a remaining-repeat counter; the runtime never materializes the full expansion inside `maximum_pending_bytes` and never loses repetitions after consuming the record.
- Key-up and modifier-only records update translation state but produce no bytes unless an enabled enhanced protocol has a canonical release encoding.
- Menu records are deliberately filtered as console-internal noise.
- Focus and mouse records translate only when those modes are enabled.
- Unknown record types produce `NativeEvent`; they do not fail ordinary reads and are never silently discarded.
- Invalid UTF-16 produces `InvalidUnicodeInput`; surrogate pairs are handled before UTF-8 encoding.

Capabilities distinguish `supported`, `enabled`, and evidence such as native confirmation, protocol query, environment inference, or assumption. Native Windows may report bracketed paste unsupported and deliver pasted text as ordinary key bytes.

## Cross-Platform Contract

### Linux

The runtime saves complete `termios` and descriptor status flags, explicitly enables `O_NONBLOCK`, and restores the exact original flags. It clears canonical/echo/extended handling, input translations, software flow control, stripping/parity marker behavior, and output post-processing as required; uses eight-bit input; sets `VMIN=0`/`VTIME=0`; and clears `ISIG` only when control keys are captured. It uses `TCSANOW` without flushing pending input. `EAGAIN` means no data and retries only against the remaining monotonic deadline. It drains `POLLIN` before `POLLHUP`; a zero-byte read emits `EndOfInput` only after confirmed hangup/closure, otherwise it means no data under `VMIN=0`/`VTIME=0`.

`SIGWINCH` uses a nonblocking self-pipe/equivalent in the same poll set. The handler only notifies, and prior disposition/mask is restored. If input and resize are simultaneously ready, Linux drains current bytes first and then reports the newest `TIOCGWINSZ` snapshot. This is deterministic observation, not guaranteed chronology.

### Windows

The runtime saves modes, `CONSOLE_CURSOR_INFO`, original active screen-buffer identity/handle, and every queryable Console API state changed by fallback output. It clears line/echo input, conditionally clears processed input, enables window input, and manages mouse/extended/Quick Edit flags together. Restore reactivates the original buffer before closing a temporary alternate buffer and restores cursor information exactly. It waits on the console handle and reads records conservatively enough to honor pending-byte capacity.

Resize records trigger a visible-window query with `GetConsoleScreenBufferInfo`; size derives from `srWindow`. Mouse positions are adjusted by `srWindow.Left`/`Top`. VT/ConPTY streams pass bytes through. Native records follow the canonical policy above.

Output uses VT processing where supported and Console API fallback for required controls where practical. Unavailable strict features return `UnsupportedFeature`.

## Lifecycle and Restoration

States are `Active`, `Paused`, `RestorePending`, and `Closed`.

- Open applies OS/protocol modes transactionally and reports failed rollback.
- Pause balances enabled protocol modes and restores exact OS flags. Runtime translation state is discarded. The next successful resume causes one `DecoderReset` packet before later bytes, forcing application parser state to reset.
- Pause is idempotent; partial restoration enters recovery, where only pause retry or close is valid.
- Resume reapplies options, refreshes size/capabilities, and starts fresh translation. Failure rolls back and remains paused; failed rollback enters recovery.
- Read/write/size/cursor/output-view operations outside active state return `TerminalInputStreamStateError`; cached capabilities remain readable.
- Close active pauses then releases; close paused releases without duplicate reversal; close recovery retries; close closed is idempotent.

OS bitsets are restored exactly. VT protocol state cannot generally be queried, so only balanced reversal of modes enabled by this stream is guaranteed. Finalization is best effort. Uncatchable termination has no restoration guarantee.

The output view shares the stream lease and follows active/paused/closed validity. The application must not retain undecoded bytes across `DecoderReset`.

## Example Applications

- `run_editor_packet_stream.op`: reads bounded packets, uses the rendering bridge, handles resize/native/reset packets, writes, pauses/resumes, and closes.
- `recognize_escape_prefix.op`: pure checked `uint8[]` parsing for an Escape/CSI prefix.

## Strengths

- Maximum protocol control and immediate access to new VT extensions.
- Unknown VT bytes and native records are both preserved meaningfully.
- Smaller stdlib parser policy than typed alternatives.
- Bounded runtime state and NUL-safe `uint8[]` transport.
- Natural deferred byte-stream future.

## Weaknesses

- Every editor implements a difficult stateful parser.
- Windows native translation still embeds canonicalization policy.
- Applications own malformed UTF-8, ESC timing, paste framing, and mouse decoding.
- Duplicate parsers can drift in compatibility.

## Impact on Existing Syntax

No parser/formatter changes. This additive handle-and-array API leaves `Bytes` unchanged because current `Bytes` lacks indexed reads. Cached capability/output-view operations are unsuffixed; OS-I/O and waiting operations end in `_sync`.

## Interactions with Other Concerns

- The stream can underlie a future typed decoder.
- A future event loop can await packet readiness beside jobs, timers, watches, and RPC.
- The output bridge coordinates existing terminal rendering with ownership.
- Hot reload must preserve or explicitly reset application parser state.
- Subprocess/pseudoterminal APIs remain separate.

## Implementation Difficulty

Medium-High. Tests need split UTF-8/CSI, NUL, lone Escape, Alt/Ctrl, native repeat expansion, key-up/modifier/menu filtering, native unknown packets, SGR mouse viewport translation, resize observation, pending-byte limits, `DecoderReset`, rollback, pause/recovery/resume, and retained output views.

## Must NOT Have

- No constructible integer-backed public handle.
- No `string` raw-input return type.
- No active-code-page encoding or lossy UTF-16.
- No boundary-alignment promise.
- No ordinary error for documented key-up/modifier/menu filtering.
- No silent unknown-native discard.
- No unbounded runtime translation buffer.
- No exact-restoration claim for unqueryable VT protocol state.
