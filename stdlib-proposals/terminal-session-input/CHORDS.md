# Terminal Chords and Key Bindings

## Scope and authority

This companion consumes selected-session `TerminalInputEvent` values but owns no terminal mode, parser, trust, platform behavior, scheduler, or application command payload. `terminal_chords.types.op` is the authoritative chord ABI declaration file. The selected session proposal is normative for event correlation, TextInput phases, capability evidence, trust, error attachments, and ABI manifest generation.

Chords consume only runtime-origin `TerminalInputEvent.Key`. They never interpret `TextInput`, composition, paste, `UnknownBytes`, `UnknownNative`, reset events, or native metadata as commands. `TerminalChordModifiers` is application-constructible and distinct from sealed runtime `TerminalModifiers`. Known `TerminalNamedKey` variants remain application-constructible, while `TerminalLogicalKey` and event constructors remain sealed.

## Public API

All signatures are proposal syntax and use canonical borrows.

```opal
# terminal_chord_modifiers(shift: boolean, control: boolean, alt: boolean, super_key: boolean): TerminalChordModifiers
# terminal_chord_new(key: TerminalChordKey, modifiers: TerminalChordModifiers, trigger: TerminalChordTrigger): TerminalChord
# terminal_chord_with_lock_modifier_mask(chord: TerminalChord, mask: TerminalLockModifierMask): TerminalChord
# terminal_chord_sequence_single(chord: TerminalChord): TerminalChordSequence errors AllocationFailureError
# terminal_chord_sequence_append(sequence: TerminalChordSequence, chord: TerminalChord): TerminalChordSequence errors TerminalChordValidationError, AllocationFailureError
# terminal_chord_router_new(capabilities: TerminalCapabilities, policy: TerminalChordRouterPolicy): TerminalChordRouter errors AllocationFailureError
# terminal_chord_router_register(mutable ref router: TerminalChordRouter, sequence: TerminalChordSequence, priority: TerminalChordPriority, text_policy: TerminalChordTextPolicy): TerminalChordBindingId errors TerminalChordValidationError, AllocationFailureError
# terminal_chord_router_unregister(mutable ref router: TerminalChordRouter, binding_id: TerminalChordBindingId): TerminalChordMutationResult errors TerminalChordMutationError, AllocationFailureError
# terminal_chord_router_replace(mutable ref router: TerminalChordRouter, binding_id: TerminalChordBindingId, sequence: TerminalChordSequence, priority: TerminalChordPriority, text_policy: TerminalChordTextPolicy): TerminalChordMutationResult errors TerminalChordValidationError, TerminalChordMutationError, AllocationFailureError
# terminal_chord_binding_id_ordinal(binding_id: TerminalChordBindingId): uint64
# terminal_chord_router_process(mutable ref router: TerminalChordRouter, event: TerminalInputEvent): TerminalChordRouterOutput errors AllocationFailureError
# terminal_chord_router_expire_sync(mutable ref router: TerminalChordRouter): TerminalChordRouterOutput errors AllocationFailureError
# terminal_chord_router_reset(mutable ref router: TerminalChordRouter, reason: TerminalChordResetReason): TerminalChordReleasedInput errors AllocationFailureError
# terminal_chord_released_input_length(input: TerminalChordReleasedInput): int64
# terminal_chord_released_input_at(input: TerminalChordReleasedInput, index: int64): TerminalInputEvent errors IndexOutOfBoundsError
```

`terminal_chord_modifiers` sets both lock fields false. `terminal_chord_new` uses `IgnoreLocks`; the functional mask API opts into CapsLock and NumLock matching. Shift, Control, Alt, and Super always participate. Sequences and released input are opaque immutable bounded carriers. Sequence append enforces the ABI-major absolute maximum of 256 chords and returns `SequenceTooLong` with that limit. Registration separately enforces the router policy's possibly lower `maximum_sequence_length`. Neither operation exposes a mutable array.

`TerminalChordRouter` is an affine registry and state owner with compiler-registered deterministic infallible cleanup. Cleanup releases immutable registrations and buffered input but performs no terminal I/O. A router stores the immutable capability snapshot supplied at construction and uses the selected centralized `terminal_capabilities_feature` inspector for enhanced identity and key-release validation.

## Identity, registrations, and application ownership

A successful registration creates an opaque immutable `TerminalChordBindingId`. Construction is sealed to the standard library through `@constructor_visibility(standard_library)`. Its hidden router provenance authenticates the producing router. Its hidden monotonically increasing ordinal is allocated once, never decreases, and is never reused, including after unregister, cleanup, or ID exhaustion. The router fails registration with `BindingIdentifierExhausted` before registry mutation when it cannot advance the ordinal. `terminal_chord_binding_id_ordinal` exposes only the ordinal for application logs and side maps. It exposes no router provenance, host identity, registry state, or ability to construct, compare across routers, or revive an ID.

The router intentionally stores no application command, callback, closure, payload, module object, or application allocation. Applications maintain their own side map from binding ID or ordinal to application behavior. They add a map entry only after successful registration, retain it through a successful replace because the ID is preserved, and remove it only after a successful unregister. Router cleanup retires registrations but cannot clean application state, so the application must clear its side map when it drops its router owner. A failed mutation leaves both router and application map unchanged.

## Registration and mutation validation

Registration validates atomically before modifying the router:

- Empty or over-limit sequences return structured `EmptySequence` or `SequenceTooLong`.
- An exact duplicate sequence and priority returns `DuplicateBinding` with the existing ID.
- `RejectAmbiguousPrefixes` rejects either direction of unresolved prefix overlap with `PrefixAmbiguity`.
- `HigherPriorityWins` permits overlap only when every competing prefix has a distinct priority. The highest priority wins as soon as it completes.
- `LongestThenPriority` waits through the bounded sequence deadline for a longer completion, then chooses longest, breaking equal length by distinct priority. Equal length and priority is `PrefixAmbiguity`.
- EnhancedText registration requires `terminal_capabilities_feature(..., EnhancedKeyIdentity)` to be Enabled or returns `EnhancedKeyIdentityRequired`.
- Release trigger requires KeyReleaseEvents Enabled or returns `ReleaseEventsRequired`.
- Registration count and sequence-length limits report the policy value that rejected the request.

Unregister and replace are atomic call-level mutations. Between affine mutable-router calls, one successful commit changes the registry, buffered prefix, router-owned prefix deadline, binding-ID state, and immutable result together; it is otherwise all-or-nothing. This contract makes no concurrent, lock-free, or cross-object atomicity claim.

Before either mutation, the router authenticates the sealed ID. An ID produced by another router fails `WrongRouter`. A same-router ordinal that has never been registered, was permanently retired, or no longer names a live binding fails `BindingNotFound`. These failures, malformed or unavailable sealed values, and every validation or allocation failure leave registry contents, buffered input, router-owned prefix deadline, ordinal state, and application-visible result unchanged. They do not change the caller-owned generic timer.

A mutation rejects with `CorrelatedGroupPending` when the buffer ends in an incomplete Key-linked `TextInput.Start` or `TextInput.Continue` group. It releases nothing and does not mutate. A complete Key-linked TextInput group is a mutation boundary, so it can be cleared and released as one whole group.

Replace validates its proposed sequence, priority, and text policy as if the target binding were temporarily absent. It applies every ordinary registration validation, including capability, sequence, duplicate, prefix ambiguity, and structured policy-limit rules, against the remaining live bindings. It does not reserve a second registration slot and does not advance the binding-ID ordinal. The implementation completes that validation and allocates every replacement registry node, buffered-release carrier, and result carrier before the single mutable-router commit. Allocation failure and validation failure therefore cannot partially replace a binding.

Unregister authenticates the live target and allocates its immutable buffered-release carrier and result carrier before commit. It removes that binding and permanently retires its ordinal in the same commit. Replace removes and inserts the target atomically while preserving the exact same `TerminalChordBindingId` and ordinal. No success result can expose a newly allocated or provenance-bearing identity.

## Buffered prefixes, results, and timer invalidation

`TerminalChordMutationResult.Unregistered` carries the retired binding ID and immutable `released_input`. `Replaced` carries the preserved binding ID and immutable `released_input`. In both cases, `released_input` contains every buffered event that was not already intentionally suppressed, exactly once and in original normalized-event order. After router success, the caller applies that input exactly once, then immediately disarms or supersedes its affine generic timer before any further router operation or wait handling.

If a successful mutation encounters a complete buffered prefix, its atomic router commit clears the buffered prefix and router-owned prefix deadline, and its result releases the unsuppressed input. It does not activate the old binding or the replacement binding. It preserves the linked-text policy that had already determined suppression for any completed matched key. A mutation never splits a complete correlated group, never turns text into a command, and never reinterprets buffered paste, composition, unknown, or reset input.

A complete candidate Key and linked complete TextInput group that leaves a resolvable prefix waiting for longer input creates the exact router-owned deadline `monotonic_clock_now() + sequence_timeout` and returns `Pending { deadline }`. Extending or changing that unresolved prefix with another complete candidate group replaces it with a new exact deadline. Repeated `Pending` results and early expiration preserve the same current deadline. Unrelated buffered data does not refresh it. An incomplete Key-linked TextInput group suspends prefix expiration and returns `AwaitingCorrelatedInput`; it has no deadline, and no timeout may split the group. When the group completes, the router re-evaluates it and returns activation, release, awaiting-correlated-input, or the appropriate new or preserved `Pending { deadline }`.

A caller arms its separate generic affine monotonic timer to the exact `Pending.deadline`. `AwaitingCorrelatedInput` disarms that timer. A successful unregister or replace atomically clears only the router's buffered prefix and router-owned prefix deadline. After receiving success and applying its released input once, the affine caller immediately disarms or supersedes its separate generic timer before any further router operation or wait handling. Explicit reset likewise returns released input for one application, then the caller disarms before further router or wait work. That core timer operation, not the router mutation, advances the caller-owned timer generation. After the wait set reports a wake, `SystemWaitWake.Cancelled` is an immediate non-expiring no-op: it carries neither source nor generation and leaves router, timer, map, and input unchanged. Only after refining `SystemWaitWake.Ready into ready` does the application verify `ready.source` equals `monotonic_timer_readiness_source(timer)`, then compare `ready.generation` with `monotonic_timer_generation(timer)`, before calling `terminal_chord_router_expire_sync`. A Ready source or generation mismatch, disarmed timer, or superseding arm is stale: the application performs an idle no-op and returns to the wait set without invoking expiration. A stale old Ready wake therefore cannot expire a newer prefix deadline. If expiration is called after mutation when no router prefix deadline exists, it returns `TerminalChordRouterOutput.Idle` and performs no mutation. The router creates no scheduler, owns no generic timer, and does not poll.

## Processing, correlation, and activation

The router accepts one normalized event at a time. After a candidate Key it buffers the selected proposal's complete adjacent Complete or Start/Continue/End Key-linked TextInput group before releasing or suppressing it. No timeout or mutation can split that group.

`Pending { deadline }` means the router retained a resolvable bounded prefix and released nothing; the deadline is exact and core-owned. `AwaitingCorrelatedInput` means an incomplete Key-linked TextInput group is retained with prefix expiration suspended and no active timeout. `Idle` means expiration was called with no current router deadline and changed nothing. Stale timer wakes are filtered by the application before it invokes expiration. `ReleasedInput` contains immutable events in original order. `Activated` contains one binding ID, the exact original `TerminalKeyOccurrence`, and released input. The router emits at most one `Activated` output for one normalized Key event. A `Press(count)` or `Repeat(count)` activates once carrying `count`; it never allocates one activation per repeat. Release activates once with `TerminalKeyOccurrence.Release`.

A completed binding suppresses every command Key in its matched sequence. `PreserveLinkedText` releases linked text associated with every matched Key after activation. `SuppressLinkedText` suppresses those linked text groups as well. On mismatch, losing priority, timeout, FocusLost, Cancelled, EndOfInput, InputReset, or explicit reset, every buffered event not intentionally suppressed is released exactly once in original order.

`terminal_chord_router_expire_sync` checks the host monotonic clock. Before its current deadline it returns `Pending` with that unchanged deadline; at or after it resolves according to prefix policy and returns `Activated` or `ReleasedInput`. It never expires while the router returns `AwaitingCorrelatedInput`. `terminal_chord_router_reset` returns all releasable buffered input and clears prefixes and the router-owned prefix deadline. The selected mandatory PauseBoundary InputReset has the same effect. Paste, composition, UnknownBytes, and UnknownNative never satisfy or extend a chord and are released unchanged unless ordering requires temporary buffering behind an unresolved prefix.

## Verification scope

Future fixtures must administer `terminal_chords.types.op` IDs and evolution and cover application construction for Control, Named, Function, and EnhancedText keys; modifier separation; immutable sequence limits; every stable validation and mutation failure; centralized capability checks; all prefix policies; priority ties; timer-driven expiration without polling; stale `Idle`; timeout, reset, unregister, and replace release order; both text policies; Key-linked chunk buffering; exact occurrence propagation; one activation per normalized Key event; no per-repeat activation allocation; identity preservation and permanent retirement; application side-map lifecycle; affine cleanup; and bounded allocation failure without partial registration.
