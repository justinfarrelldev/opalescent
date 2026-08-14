# Terminal Session and Input Comparison

## Scope and decision

This concern owns process-interactive terminal session/input on Linux and Windows. `typed-event-session` is selected for public v1 because one normalized event per read is the smallest safe surface while the runtime retains parser, restoration, provenance, bounds, and platform responsibility.

Normative selected contract: [`typed-event-session/proposal.md`](./typed-event-session/proposal.md)

Authoritative terminal declarations: [`typed-event-session/typed_event_session.types.op`](./typed-event-session/typed_event_session.types.op)

Companion chord behavior/declarations: [`CHORDS.md`](./CHORDS.md) and [`terminal_chords.types.op`](./terminal_chords.types.op)

RPC, subprocesses, watches, timers, generalized scheduling, editor buffers, and rendering policy remain outside scope. The only shared integration is core-owned `SystemReadinessSource`; no proposal exposes platform handles.

## Summary matrix

| Axis | Typed Event Session | Batched Event Pump | Portable Input Packet Stream |
|---|---|---|---|
| Decision | Selected public v1 | Historical batching alternative | Historical expert transport alternative |
| Read shape | One normalized event | Bounded event batch | Bounded canonical bytes/packets |
| Parser owner | Runtime | Runtime | Application |
| Ownership | Affine session with explicit borrows | Must adopt selected affine session | Separate stream ownership is insufficient for selected guarantees |
| Correlation | Atomic Key/Text group | Batch boundaries must preserve whole groups | Caller must reconstruct groups |
| Bounds | Per-value/group plus total accounting | Adds batch carrier/accounting constraints | Transport bound does not bound caller parser |
| Readiness | Core host-stable source | Same source; batching adds no handles | Requires composition with selected source model |
| Trust | Sealed evidence and nominal output | Must reuse selected trust types | Raw packets establish no command isolation |
| Migration cost | Baseline | High: replace public batch shapes | High: move parser/provenance back into runtime |

## Why alternatives remain

The batched snapshot records potential throughput benefits and batch-specific migration constraints; it is not type-compatible with v1. The packet stream records a lower-level transport tradeoff but intentionally fails the selected goal of centralized portable normalization.

All ownership, ABI, options, error, platform, signal, trust, cancellation, readiness, event, and verification requirements live in the selected proposal and are not repeated here.
