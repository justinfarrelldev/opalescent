# Batched Event Pump

## Historical status and selected-contract deference

This remains a historical batching alternative, not selected public v1 and not implemented support. Its recorded API body and batching-specific migration constraints are preserved as historical design context rather than redesigned here. For any future design work, the selected [typed-event-session contract](../typed-event-session/proposal.md), [core prerequisites](../core-prerequisites.md), [active declarations](../typed-event-session/typed_event_session.types.op), [ABI history](../typed-event-session/abi-history.md), [chord contract](../CHORDS.md), and [test-only contract](../TESTING.md) control all cross-cutting rules.

In particular, this alternative creates no independent terminal or legacy-I/O owner; no recovery, process-control, diagnostic, readiness, timer, test, trust, or ABI authority; and no session-derived raw-output escape. Any future batch operation must preserve the selected affine ownership/runtime state machine, coordinator rejection-before-consumption/mutation, typed recovery-token validation and retry, separate process-control protocol, structured diagnostics and attachments, generic wait/timer ordering, chord correlation/lifecycle, explicit output declassification, test-only sealed construction, and active/retired ABI authority. Batch contents and diagnostics are never recovery or rendering authority, and signals are never terminal events or batch members.

## Status

This is a historical batching alternative, not a type-compatible public API. The normative v1 contract is [`../typed-event-session/proposal.md`](../typed-event-session/proposal.md), with authoritative declarations in its `.types.op` file and companion chord declarations in [`../terminal_chords.types.op`](../terminal_chords.types.op).

A future batch read may be added only after profiling demonstrates runtime-crossing cost. It must be an additive operation on the selected affine `TerminalSession`; it cannot revive this snapshot's mutable/public batch records.

## Batching-specific migration constraints

- Return an opaque immutable bounded event sequence with length/index inspectors; never a mutable array or flat parallel fields.
- Charge the batch carrier header, index/offset table, boxes, alignment, payload capacity, and retained references to selected checked session accounting until independent caller retention completes.
- Admit and publish a Key plus all linked Complete or Start/Continue/End TextInput chunks atomically in one batch. A batch boundary may occur before or after that group, never inside it.
- Preserve composition commit chunks contiguously immediately before matching `CompositionEnded.Committed`; never split their ordering contract across independently visible partial batches.
- Preserve the selected malformed-frame rule: once fallback begins, a batch contains only ordered bounded UnknownBytes for consumed payload followed eventually by exactly one PasteFallback reset; it cannot reinterpret a valid prefix/suffix.
- A successful Active→Paused result still ends with exactly one PauseBoundary reset. Batch capacity must reserve that final record before restoration.
- Readiness remains a hint from the same host-stable source. A batch read after shared wake uses Poll semantics and may return an empty/stale-readiness outcome defined without sentinel arrays.
- Cancellation, sticky EOF, sticky identifier exhaustion, transition wakes, RestorePending/Closed terminal readiness, and close-discard accounting retain selected ordering across the whole batch.
- Implementations must retain the selected no-allocation fast path: reserved queue/slab storage may move several fixed events into one immutable view without allocating one object per event.
- Batch APIs reuse selected event/trust/error/capability types and the generated terminal ABI manifest. They introduce only batch-owned stable declarations and tests.

## Why retain this snapshot

Batching may improve burst throughput for mouse, paste, or resize traffic, but exposes allocation and boundary policy prematurely. One-event v1 keeps those choices internal while requiring a batch-capable parser/buffer boundary. This document records only the extra constraints a future measured design must satisfy.
