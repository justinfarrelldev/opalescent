# Batched Event Pump

## Status and relationship to typed event session

This is a separate alternative snapshot. Its batch record, fields, ordering choices, and retention model do not have type identity with the selected `typed-event-session` API. It does not claim that its types are final typed-session types. Typed event session is the public v1 direction: one payload-bearing non-exhaustive event per read.

A future bounded `terminal_session_read_events_sync` may be added to the typed session handle after profiling, without changing v1 source shape. That possibility is not adoption of this snapshot. Any future migration must replace this proposal's public shapes and behavior with the typed-event-session contracts below.

## Mandatory migration requirements

- Use only `if value is Type.Variant into payload:` and payloadless `if value is Type.Variant:`. Do not require match syntax, exhaustiveness, or deferred cleanup syntax. A migrated pump would use `using pump = propagate terminal_event_pump_open_sync(options):` with compiler-registered fallible close.
- Use the typed public non-exhaustive event sum with permanent tags, ABI hashes, runtime-owned unknown-payload drop metadata, structured causes, and bounded suppressed causes. Do not claim type identity before the migration is complete.
- Preserve Key-only chords, `TextInput` as the only insertion event, enhanced-only logical Text keys gated by `enhanced_key_identity` Enabled, event-id links, explicit composition lifecycle, and paste Complete, Start, Continue, and End phases split on scalar boundaries only with `trusted_paste_framing` Enabled. In-band bracketed-paste delimiters alone are not a security boundary; without trusted provenance, emit no Paste and claim no command isolation.
- Use constrained nominal limits and UTF-8 text values, including `utf8_byte_length`; diagnostics must use structured backend, operation, stage, state, OS-code, retryability, and bounded detail. `UnsupportedFeature` must carry `TerminalFeature`.
- Account for total retained events and all bytes, including batch records, payloads, parser state, fallback, preedit, native metadata, pause delivery, and pending records. Capacity must mean no-drop backpressure, never reset or silent drop.
- Quarantine malformed or NUL paste as bounded `UnknownBytes` until delimiter, EOF, pause, or backend reset. Exclude delimiter payload, emit exactly one fallback reset after termination, and never parse fallback as commands.
- Put cancellation token wakeup beside terminal, resize, and deadline readiness in one wait set. Deliver queued events first; when cancellation and unread input become ready together, cancellation wins and no new input is consumed. A reset or new token is required after observed cancellation. Retain parser state until pause or close, and do not poll.
- Preserve Linux termios, exact snapshot restoration, `IGNBRK`, `IXOFF`, `O_NONBLOCK`, `VMIN=0`, `VTIME=0`, `TCSANOW`, poll, and SIGWINCH contracts; Windows Console and ConPTY mode, screen-buffer, UTF-16, viewport-offset, and wakeable overlapped-read contracts; restoration ledger plus failed-open and failed-close recovery; pause result, resume freshness, close discard precedence; and process-global stdout gate with atomic endpoint-lease cutovers.

## Why keep this snapshot

Batching can reduce runtime crossings during bursty mouse or paste traffic, but exposes queue ordering, capacity, allocation, and ownership choices. One-event v1 leaves those choices internal and requires its parser/buffer boundary not to assume per-event allocation, so a later bounded batch can be designed from evidence. This document remains a record of that tradeoff, not a compatibility promise.
