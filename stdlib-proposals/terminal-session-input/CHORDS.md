# Terminal Chords and Key Bindings

## Scope and authority

This companion consumes selected-session `TerminalInputEvent` values but owns no terminal mode, parser, trust, or platform behavior. `terminal_chords.types.op` is the authoritative chord ABI declaration file. The selected session proposal is normative for event correlation, TextInput phases, capability evidence, trust, error attachments, and ABI manifest generation.

Chords consume only runtime-origin `TerminalInputEvent.Key`. They never interpret TextInput, composition, paste, unknown bytes, or native metadata as commands. `TerminalChordModifiers` is application-constructible and intentionally distinct from sealed runtime `TerminalModifiers`; known `TerminalNamedKey` variants remain application-constructible while `TerminalLogicalKey`/event constructors remain sealed.

## Companion API

All signatures are proposal syntax and use canonical borrows.

```opal
# terminal_chord_modifiers(shift: boolean, control: boolean, alt: boolean, super_key: boolean): TerminalChordModifiers
# terminal_chord_new(key: TerminalChordKey, modifiers: TerminalChordModifiers, trigger: TerminalChordTrigger): TerminalChord
# terminal_chord_with_lock_modifier_mask(chord: TerminalChord, mask: TerminalLockModifierMask): TerminalChord
# terminal_chord_sequence_single(chord: TerminalChord): TerminalChordSequence errors AllocationFailureError
# terminal_chord_sequence_append(sequence: TerminalChordSequence, chord: TerminalChord): TerminalChordSequence errors TerminalChordValidationError, AllocationFailureError
# terminal_chord_router_new(capabilities: TerminalCapabilities, policy: TerminalChordRouterPolicy): TerminalChordRouter errors AllocationFailureError
# terminal_chord_router_register(mutable ref router: TerminalChordRouter, sequence: TerminalChordSequence, priority: TerminalChordPriority, text_policy: TerminalChordTextPolicy): TerminalChordBindingId errors TerminalChordValidationError, AllocationFailureError
# terminal_chord_router_process(mutable ref router: TerminalChordRouter, event: TerminalInputEvent): TerminalChordRouterOutput errors AllocationFailureError
# terminal_chord_router_expire_sync(mutable ref router: TerminalChordRouter): TerminalChordRouterOutput errors AllocationFailureError
# terminal_chord_router_reset(mutable ref router: TerminalChordRouter, reason: TerminalChordResetReason): TerminalChordReleasedInput errors AllocationFailureError
# terminal_chord_released_input_length(input: TerminalChordReleasedInput): int64
# terminal_chord_released_input_at(input: TerminalChordReleasedInput, index: int64): TerminalInputEvent errors IndexOutOfBoundsError
```

`terminal_chord_modifiers` sets both lock fields false. `terminal_chord_new` uses `IgnoreLocks`; the functional mask API opts into CapsLock/NumLock matching. Shift/Control/Alt/Super always participate. Sequences and released input are opaque immutable bounded carriers. Sequence append enforces the ABI-major absolute maximum of 256 chords and returns `SequenceTooLong` with that limit; registration separately enforces the router policy's possibly lower `maximum_sequence_length`. Neither operation exposes a mutable array.

`TerminalChordRouter` is an affine registry/state owner with compiler-registered deterministic infallible cleanup. Cleanup releases immutable registrations and buffered input; it performs no terminal I/O. One router stores the immutable capability snapshot supplied at construction and uses the selected centralized `terminal_capabilities_feature` inspector for enhanced identity and key-release validation.

## Registration and ambiguity

Registration validates atomically before modifying the router:

- Empty/over-limit sequences return structured `EmptySequence`/`SequenceTooLong`.
- Exact duplicate sequence+priority returns `DuplicateBinding` with the existing ID.
- `RejectAmbiguousPrefixes` rejects either direction of unresolved prefix overlap with `PrefixAmbiguity`.
- `HigherPriorityWins` permits overlap only when every competing prefix has a distinct priority; the highest priority wins as soon as it completes.
- `LongestThenPriority` waits through the bounded sequence deadline for a longer completion, then chooses longest, breaking equal length by distinct priority. Equal length/priority is `PrefixAmbiguity`.
- EnhancedText registration requires `terminal_capabilities_feature(..., EnhancedKeyIdentity)` to be Enabled or returns `EnhancedKeyIdentityRequired`.
- Release trigger requires KeyReleaseEvents Enabled or returns `ReleaseEventsRequired`.
- Binding-ID exhaustion is sticky for registration and returns `BindingIdentifierExhausted` without modifying the registry.

Priorities are signed nominal values; larger wins. Registration order never resolves ambiguity.

## Processing, correlation, and activation

The router accepts one normalized event at a time. It understands the selected proposal's atomic Key-linked TextInput group: after a candidate Key it buffers the complete adjacent Complete or Start/Continue/End group before releasing/suppressing it. No timeout can split that group.

`Pending` means the router retained bounded prefix/group input and released nothing. `ReleasedInput` contains immutable events in original order. `Activated` contains one binding ID, the exact original `TerminalKeyOccurrence`, and released input. The router emits at most one `Activated` output for one normalized Key event. A `Press(count)` or `Repeat(count)` therefore activates once carrying `count`; it never allocates one activation per repeat. Release activates once with `TerminalKeyOccurrence.Release`.

A completed binding suppresses every command Key in its matched sequence. `PreserveLinkedText` releases the linked text associated with every matched Key after activation; `SuppressLinkedText` suppresses those linked text groups as well. On mismatch, losing priority, timeout, FocusLost, Cancelled, EndOfInput, InputReset, or explicit reset, every buffered event not intentionally suppressed is released exactly once in original order.

A caller arms a generic monotonic timer for the policy timeout whenever processing returns Pending. When that timer wakes the shared wait set, `terminal_chord_router_expire_sync` checks the host monotonic clock: before the deadline it returns Pending; at/after the deadline it resolves according to prefix policy and returns Activated or ReleasedInput. The router does not create a scheduler or poll.

`terminal_chord_router_reset` explicitly returns all releasable buffered input and clears prefixes. Processing the selected mandatory PauseBoundary InputReset has the same effect. Paste, composition, UnknownBytes, and UnknownNative never satisfy or extend a chord and are released unchanged unless ordering requires temporary buffering behind an unresolved prefix.

## Verification

Fixtures compile-administer `terminal_chords.types.op` IDs/evolution and cover application construction for Control, Named, Function, and EnhancedText keys; modifier separation; immutable sequence limits; all stable validation variants; centralized capability checks; all three prefix policies; priority ties; timer-driven expiration without polling; timeout/reset release order; both text policies; Key-linked chunk buffering; exact occurrence propagation; one activation per normalized Key event; no per-repeat activation allocation; affine cleanup; and bounded allocation failure without partial registration.

See `configure_editor_chords.op` for end-to-end registration and processing call sites. Shared event/trust/ABI/platform guarantees are tested by the selected proposal and are not duplicated here.
