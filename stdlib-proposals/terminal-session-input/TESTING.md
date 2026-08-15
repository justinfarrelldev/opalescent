# Terminal Test Support

## Status, availability, and ABI boundary

This proposal-only test contract belongs to `standard.testing.terminal`. Every type in `terminal_testing.types.op` and every signature in the public API block below has `@availability(test_only)`. The compiler rejects a production import, exported signature, generic bound, field, error payload, metadata value, manifest entry, reflection record, generated export, or artifact that names, reaches, serializes, or erases a test-only symbol.

`TerminalTestAuthority` is opaque immutable and its construction is restricted to the test runner with `@constructor_visibility(test_runner)`. A test artifact receives it only from the test runner for that compilation. It cannot enter a production artifact or construct a session, recovery token, coordinator lease, host handle, or terminal mode.

Test-only declarations have no production `@abi_type_id`, production ABI export, or terminal ABI owner. Test artifact identity is ephemeral for one test build and is excluded from `typed_event_session.types.op`, `terminal_chords.types.op`, and `abi-history.md`.

## Public test-only API

The following are future test-only public signatures. They are specified here, not in `.types.op`, because the proposal's `.types.op` files contain type declarations only.

```opal
# import type AllocationFailureError, ConstraintViolationError, Bytes from standard
# import type TerminalCapabilities, TerminalCompositionId, TerminalDiagnostic, TerminalDiagnosticCollection, TerminalEventId, TerminalInputEvent, TerminalSessionOptionsError, TerminalTrustedPasteEvidence from ./typed-event-session/typed_event_session.types
#
# @availability(test_only)
# test_runner_terminal_authority(): TerminalTestAuthority
# @availability(test_only)
# terminal_test_scenario_new(authority: TerminalTestAuthority, limits: TerminalTestScenarioLimits): TerminalTestScenario errors TerminalSessionOptionsError, AllocationFailureError
# @availability(test_only)
# terminal_test_event_id_new(mutable ref scenario: TerminalTestScenario): TerminalEventId errors TerminalTestFactoryError
# @availability(test_only)
# terminal_test_composition_id_new(mutable ref scenario: TerminalTestScenario): TerminalCompositionId errors TerminalTestFactoryError
# @availability(test_only)
# terminal_test_trusted_paste_evidence(mutable ref scenario: TerminalTestScenario, boundary: TerminalTestTrustedPasteBoundary): TerminalTrustedPasteEvidence errors TerminalTestFactoryError

# @availability(test_only)
# terminal_test_key_event(mutable ref scenario: TerminalTestScenario, event_id: TerminalEventId, key: TerminalTestLogicalKey, occurrence: TerminalTestKeyOccurrence, modifiers: TerminalTestModifiers): TerminalInputEvent errors ConstraintViolationError, TerminalTestFactoryError
# @availability(test_only)
# terminal_test_text_input_event(mutable ref scenario: TerminalTestScenario, text: string, origin: TerminalTestTextInputOrigin, linked_phase: TerminalTestLinkedTextPhase): TerminalInputEvent errors ConstraintViolationError, TerminalTestFactoryError
# @availability(test_only)
# terminal_test_composition_started_event(mutable ref scenario: TerminalTestScenario, composition_id: TerminalCompositionId): TerminalInputEvent errors TerminalTestFactoryError
# @availability(test_only)
# terminal_test_composition_updated_event(mutable ref scenario: TerminalTestScenario, composition_id: TerminalCompositionId, preedit_text: string, cursor_scalar_index: int64): TerminalInputEvent errors ConstraintViolationError, TerminalTestFactoryError
# @availability(test_only)
# terminal_test_composition_ended_event(mutable ref scenario: TerminalTestScenario, composition_id: TerminalCompositionId, outcome: TerminalTestCompositionEnd): TerminalInputEvent errors TerminalTestFactoryError
# @availability(test_only)
# terminal_test_paste_event(mutable ref scenario: TerminalTestScenario, text: string, phase: TerminalTestPastePhase, evidence: TerminalTrustedPasteEvidence): TerminalInputEvent errors ConstraintViolationError, TerminalTestFactoryError
# @availability(test_only)
# terminal_test_mouse_event(mutable ref scenario: TerminalTestScenario, action: TerminalTestMouseAction, modifiers: TerminalTestModifiers, row: int32, column: int32): TerminalInputEvent errors ConstraintViolationError, TerminalTestFactoryError
# @availability(test_only)
# terminal_test_resize_event(mutable ref scenario: TerminalTestScenario, columns: int32, rows: int32): TerminalInputEvent errors ConstraintViolationError, TerminalTestFactoryError
# @availability(test_only)
# terminal_test_focus_gained_event(mutable ref scenario: TerminalTestScenario): TerminalInputEvent errors TerminalTestFactoryError
# @availability(test_only)
# terminal_test_focus_lost_event(mutable ref scenario: TerminalTestScenario): TerminalInputEvent errors TerminalTestFactoryError
# @availability(test_only)
# terminal_test_unknown_bytes_event(mutable ref scenario: TerminalTestScenario, raw_bytes: Bytes, reason: TerminalTestUnknownBytesReason): TerminalInputEvent errors ConstraintViolationError, TerminalTestFactoryError
# @availability(test_only)
# terminal_test_unknown_native_event(mutable ref scenario: TerminalTestScenario, metadata: TerminalTestNativeMetadata): TerminalInputEvent errors ConstraintViolationError, TerminalTestFactoryError
# @availability(test_only)
# terminal_test_input_reset_event(mutable ref scenario: TerminalTestScenario, reason: TerminalTestInputResetReason): TerminalInputEvent errors TerminalTestFactoryError

# @availability(test_only)
# terminal_test_capabilities(mutable ref scenario: TerminalTestScenario, ordinary: TerminalTestOrdinaryCapabilityEntry[], trusted_paste: TerminalTestTrustedPasteCapabilitySpec, color: TerminalTestColorCapabilitySpec): TerminalCapabilities errors TerminalTestFactoryError, AllocationFailureError
# @availability(test_only)
# terminal_test_diagnostic(mutable ref scenario: TerminalTestScenario, specification: TerminalTestDiagnosticSpec): TerminalDiagnostic errors ConstraintViolationError, TerminalTestFactoryError, AllocationFailureError
# @availability(test_only)
# terminal_test_diagnostic_collection(authority: TerminalTestAuthority, diagnostics: TerminalDiagnostic[], limits: TerminalTestDiagnosticCollectionLimits): TerminalDiagnosticCollection errors AllocationFailureError
# @availability(test_only)
# terminal_test_fake_backend(authority: TerminalTestAuthority): TerminalTestFakeBackend errors AllocationFailureError
# @availability(test_only)
# terminal_test_fake_backend_with_fault(backend: TerminalTestFakeBackend, fault: TerminalTestFakeBackendFault): TerminalTestFakeBackend errors TerminalTestFactoryError, AllocationFailureError
# @availability(test_only)
# terminal_test_bind_fake_backend(mutable ref scenario: TerminalTestScenario, backend: TerminalTestFakeBackend): void errors TerminalTestFactoryError
# @availability(test_only)
# terminal_test_activate_backend(mutable ref scenario: TerminalTestScenario): TerminalTestBackendActivation errors TerminalTestFactoryError
```

These signatures are test-only declarations even though this Markdown block is their proposal authority. They cannot be imported, referenced, exported, or emitted by production compilation.

## Synthetic factory invariants

Every factory requires test-runner authority or an authority-derived scenario. Scenario creation requires `TerminalTestScenarioLimits`, whose `TerminalSessionResourceLimits` snapshot undergoes the same production cross-field validation as `terminal_session_options_validate` before any scenario, event-ID, composition-ID, evidence, backend-binding, activation state, stream identity, or delivery-order state is created. Invalid resource relationships return `TerminalSessionOptionsError` without mutation. Successful creation issues one hidden test-runner-only input-stream identity and initializes its next delivery ordinal to one. Every `TerminalCapabilities` snapshot and every sealed `TerminalInputEvent` produced from that scenario carries the same identity, so events from another scenario exercise authentic wrong-stream rejection without a forgeable production identity. The identity has no test or production inspector, cannot be supplied by test code, and never enters a production artifact. A successful immutable limits snapshot governs every synthetic text, preedit, paste, unknown-byte, size, correlation, retention, and diagnostic bound for that scenario. There is no undefined active-scenario limit, implicit fallback, or sentinel. A value rejected by that snapshot or by production construction is rejected here rather than normalized or clamped.

`terminal_test_event_id_new` and `terminal_test_composition_id_new` are the sole test-authorized construction paths for runtime-only IDs. Each scenario owns independent event-ID and composition-ID sequences. Each sequence starts at one, increases monotonically, and issues only nonzero, never-reused IDs; neither sequence wraps, resets, borrows capacity from the other, or reuses an ID after linked text or composition completion. The Key and composition factories reject an ID from another scenario, an unknown ID, an unstarted composition, a duplicate start, or an end/update after completion.

Before issuing an event ID, `terminal_test_event_id_new` preflights the next event ordinal. If no next value exists without wrap or reuse, it returns `TerminalTestFactoryError.EventIdentifierExhausted { last_issued_event_id }`. The payload is the last valid scenario-issued `TerminalEventId`, not zero or a sentinel. Failure occurs before ID issuance, event live-set insertion, linked-text bookkeeping, retained-event/byte accounting, or any other factory-visible scenario mutation. The event sequence remains exhausted at that same last ID; every later event-ID attempt returns the same variant and payload. The composition sequence and all already-issued IDs and legal factory operations remain unchanged and usable.

Before issuing a composition ID, `terminal_test_composition_id_new` performs the symmetric preflight and returns `TerminalTestFactoryError.CompositionIdentifierExhausted { last_issued_composition_id }` when no next value exists. Failure occurs before ID issuance, composition live-set insertion, lifecycle/preedit bookkeeping, retained-event/byte accounting, or any other factory-visible scenario mutation. The composition sequence remains exhausted at that same last ID; every later composition-ID attempt returns the same variant and payload. The event sequence and all already-issued IDs and legal factory operations remain unchanged and usable. Neither exhaustion path partially creates a value, advances a counter, emits zero, wraps, reuses an ID, or pollutes a production error family.

Every successful synthetic event factory stamps its result with the scenario identity and the current nonzero delivery ordinal, then atomically advances that ordinal. This single sequence covers Key, TextInput, every composition event, Paste, Mouse, Resize, both focus events, UnknownBytes, UnknownNative, and InputReset in successful factory call order; correlation IDs do not substitute for it. Constraint, relationship, lifecycle, evidence, allocation, and retained-accounting validation occurs before the stamp/advance commit. If no next ordinal exists without wrap or reuse, every event factory returns retry-stable payload-free `TerminalTestFactoryError.DeliveryOrdinalExhausted` before constructing an event or mutating correlation, lifecycle, retention, or delivery-order state. Hidden order state remains unchanged and no ordinal becomes inspectable. An event retained from an earlier successful call can therefore exercise replay, and two retained events can be supplied in reverse call order to exercise future/out-of-order rejection, without exposing or allowing mutation of either hidden stamp.

The explicit synthetic factories cover every payload-bearing or correlation-sensitive normalized event category:

- Key uses one scenario-issued ID, a bounded logical key, valid occurrence, and all six explicit modifier fields.
- TextInput rejects a Key origin that does not reference a recorded Key. Direct origin requires `Complete`; Key-linked input permits only start, continue, end ordering and remains incomplete until its end.
- Composition start, update, and end use one scenario-issued composition ID. Update verifies the cursor is within the preedit scalar count and every lifecycle is ordered.
- Paste accepts only scenario-issued `NativeRecordBoundary` or `SanitizedProtocolBoundary` evidence and enforces complete or start, continue, end phase order.
- Mouse, unknown bytes, unknown native metadata, and input reset accept only their corresponding test-only mirror types. The factory validates each mirror's bounded fields and authoritative discriminant mapping before creating the sealed runtime value. Resize and focus use primitive bounded input only. All remain non-command input; unknown input cannot become a chord or application command.

`TimedOut`, `Cancelled`, and `EndOfInput` deliberately have no synthetic factories. They are no-payload outcomes produced only by the normal deterministic fake-backend read lifecycle, preserving production readiness, cancellation, and EOF ordering. When that lifecycle publishes one, it uses the bound scenario's same hidden stream identity and consumes the next delivery ordinal in publication order; exhaustion follows the normal production sticky read-error path rather than adding a public factory. The factory list above does not claim to construct every `TerminalInputEvent` variant.

## Capability and diagnostic invariants

`ordinary` contains exactly one keyed `TerminalTestOrdinaryCapabilityEntry` for each known `TerminalOrdinaryFeature`. The factory identifies features from each entry's `feature` field, never from array position, and rejects `DuplicateOrdinaryFeature`, `MissingOrdinaryFeature`, or `UnknownOrdinaryFeature` before producing a snapshot. Each entry's specification carries its own compatible evidence: `Unsupported` carries unsupported evidence and `Available` or `Enabled` carry supported evidence. No evidence is supplied separately, omitted, or represented by a sentinel.

The trusted-paste specification likewise carries its own data. `Unsupported` has only unsupported evidence. `Available` and `Enabled` carry a `TerminalTrustedPasteEvidence` issued by the same scenario. `TerminalTestColorCapabilitySpec.Indexed` carries a nonzero production-bounded `TerminalColorCount` and supported evidence; its other variants carry only their compatible evidence. No zero count, boolean flag, or dummy evidence encodes another variant.

`TerminalTestDiagnosticSpec` is the sole test input for diagnostic construction. Its backend, operation, stage, coordinator/session state, OS-code, retryability, and truncation codes are authoritatively mapped to the identically discriminated sealed production values: only their published production discriminants are valid, while any unrecognized, retired, or incompatible code is rejected. The factory also rejects an illegal coordinator/session pairing. Its detail is checked against the scenario's production diagnostic limit before mapping to the sealed bounded detail value. Diagnostics never contain a session, recovery token, lease, host handle, or unbounded host message; retryability remains advisory metadata.

`terminal_test_diagnostic_collection` derives every collection-accounting field from ordered complete diagnostics and `TerminalTestDiagnosticCollectionLimits`; it accepts no retained count, omitted count, retained bytes, omitted bytes, truncation flag, optional value, or sentinel. It iterates supplied diagnostics in encounter order and retains the longest prefix of complete values whose count and production collection-byte accounting fit the supplied production-valid limits. Before retaining each complete diagnostic, it uses checked prospective arithmetic for both retained count and retained bytes. A prospective limit excess or arithmetic overflow is limit exhaustion before insertion: that diagnostic and every later complete diagnostic are omitted without partial retention. Retained count equals indexed length, indexed access remains bounded to retained values, and retained bytes remain the exact checked production-accounting total within the supplied limit. Byte accounting includes collection records, immutable boxes, bounded detail capacity, alignment, and truncation metadata exactly as production does. Collection metadata is preallocated before iteration and cannot be displaced by a diagnostic.

For each excluded complete diagnostic, the factory adds one to omitted count and adds that diagnostic's complete checked production-accounting byte contribution to omitted bytes, using checked uint64 arithmetic exactly as production does. Before overflow, each stored total is exact. If either omitted accumulator addition overflows, that accumulator stores exactly `18446744073709551615` (uint64 maximum), remains at that value for every later omission, and never wraps or decreases. After saturation, the stored maximum is a lower bound on the true omitted count or true omitted bytes, respectively; it is neither an upper bound nor an unknown-value sentinel. Saturation and every omission set `was_truncated=true`; equivalently, `was_truncated` is true exactly when at least one complete diagnostic was omitted. The factory continues visiting later complete diagnostics to preserve deterministic derived metadata, but a saturated accumulator remains saturated. Configured limits, allocation behavior, and longest-prefix retained accounting are unchanged.

Because both fields of `TerminalTestDiagnosticCollectionLimits` use production-valid constrained types, a low valid count or byte limit with multiple supplied complete diagnostics deterministically produces nonzero omitted count and bytes. This exercises the real accounting path rather than fabricating collection metadata.

## Deterministic fake backend activation and recovery

A test first creates `TerminalTestFakeBackend`, derives ordered fault rules, and binds it to exactly one `TerminalTestScenario` through `terminal_test_bind_fake_backend`. A second binding fails `TerminalTestFactoryError.FakeBackendAlreadyBound`. Activation of a scenario with no binding fails `TerminalTestFactoryError.FakeBackendNotBound`. Both failures occur before activation-stack, lifecycle, coordinator, backend-plan, session, or recovery mutation. The runner then enters `using activation = propagate terminal_test_activate_backend(scenario):`. `TerminalTestBackendActivation` is an affine test-runner-only dynamic scope. While live, unchanged ordinary `terminal_session_open_sync(options)`, direct `terminal_session_close_sync`, and lexical `using` cleanup consult the activation for the current test runner task and use that scenario's bound deterministic backend. Production signatures receive no scenario parameter and non-test calls have no activation record, so they always use the normal production backend.

Activation records are stack-scoped and task-local. Nested activation is permitted only for a distinct nested scenario; it shadows the outer record. Re-activating an already-live scenario fails `TerminalTestFactoryError.ScenarioAlreadyActive`; attempting to activate a scenario currently owned by another task fails `TerminalTestFactoryError.ScenarioBoundToAnotherTask`. The affine activation is non-transferable and accepted by no operation, so wrong-task use is unreachable rather than an error path. Each reachable rejection occurs before activation-stack, lifecycle, coordinator, backend-plan, session, or recovery mutation. Child tasks receive no activation implicitly; the runner must enter their own activation scope.

Activation cleanup is infallible. The affine activation is non-transferable, so wrong-task cleanup is prevented rather than silently losing the previous record. On every lexical exit it removes exactly its own current-task top record and restores the immediately preceding record in strict LIFO order, even if the body or terminal cleanup fails. It never closes a session, changes coordinator ownership, mutates a backend plan, or converts a fake backend into production state.

Each `TerminalTestFaultPoint` matches exactly one lifecycle invocation kind and one-based occurrence. A forward rule additionally matches the production stage code and exact before/after mutation boundary; an inverse rule matches one strict reverse-ledger inverse-step ordinal and boundary. Rules are evaluated in declared order only at their exact match point; unmatched rules do nothing, and a matched rule is consumed once. A duplicate rule for one exact point fails `TerminalTestFactoryError.DuplicateFaultPoint`. A zero occurrence or inverse step, or an unknown forward-stage code, fails `TerminalTestFactoryError.FakeBackendPlanInvalid` before activation-stack, lifecycle, coordinator, backend-plan, session, or recovery mutation. An inverse rule whose step is not reached by the future invocation is an unmatched no-op, not a pre-lifecycle validation failure. This gives deterministic ordering without inspecting host handles or exposing ledger contents.

For the required recovery path, exactly one matched `Open(1)` `AfterMutation` forward fault first commits its ledger-appending mutation and then fails that same forward invocation, beginning ordinary rollback. The next configured rule matches Open(1) inverse-step one and fails that first rollback inverse. Ordinary open therefore transfers the shortened real ledger to `FailedOpenRecovery` and returns its normal `RollbackFailed { diagnostics, recovery_token }` payload. The backend never returns or constructs the token: it is observable only through that ordinary error, and retains all production provenance, kind, generation, stale, consumption, and retry semantics. The same matching model applies to direct close and lexical cleanup, including their ordinary recovery paths.

## Chord coverage note

The production chord example remains independent from `standard.testing.terminal`; see this same-directory `TESTING.md` for synthetic-event coverage. Chord fixtures create a router from one scenario's capability snapshot, then cover an event from a second scenario as `WrongInputStream`, replay of an accepted retained event as `DeliveryAlreadyConsumed`, and reverse factory-call order as `DeliveryOutOfOrder`. They cover every normalized event category in the one expected-ordinal stream, successful atomic ordinal advance, and unchanged expected ordinal after ordering, capacity, or allocation rejection. Side-map fixtures inject preparation allocation failure before registration, registration failure with preparation cleanup, infallible prepared-entry insertion after success, unregister removal ordering, and map-before-router cleanup on fallthrough plus every propagated error exit. The example delivers released input once, then disarms or supersedes its caller-owned timer before later router or wait work. It filters stale timer wakes before expiration and treats `TerminalChordRouterOutput.Idle` as a no-op.
