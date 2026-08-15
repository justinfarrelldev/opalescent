# Terminal ABI History

## Scope and revision

This is the authoritative append-only history for terminal ABI ownership. The
committed Task 1 introduction baselines remain:

- selected declarations: commit `4278ed359e2186251f81dfbf280caecf69aa3678`;
- chord declarations: commit `384f502718e0a85f40862bfda0d02455b2344575`.

`6067a1d` introduced `typed_event_session.types.op` before it carried
`@abi_type_id` annotations or authoritative ABI IDs. Commit `4278ed3`
checkpointed 82 selected type IDs. The selected declaration revision added five
previously unused type IDs and retired the obsolete explicit variant IDs listed
below, producing exactly 87 active selected production type IDs. This chord
declaration revision adds only the next unused chord type IDs
`0x5400000000000114` and `0x5400000000000115`, producing exactly 22 active
chord production type IDs without changing the selected inventory.

No representation hash, representation version, or generated manifest is
recorded because none is evidenced by repository history.

## Authority and append-only rules

Active `.types.op` declarations own current type IDs, explicit variant IDs,
fields, constructor visibility, evolution, ownership, and current representation
annotations:

- `typed_event_session.types.op` owns selected terminal declarations.
- `terminal_chords.types.op` owns chord declarations.

This history owns only retired IDs, retirement reasons and commits, replacements,
evidenced prior representations, and permanent never-reuse records. It is not an
alternate active declaration source. Active and retired sets are disjoint.

Deleting an active declaration or explicit variant ID requires a retirement entry
in the same revision; a retired ID is permanently unavailable for reassignment.
Core, system, standard-library, process-control, wait/timer, and test-only types
are not terminal ABI declarations and have no entries here. No representation
hash, representation version, or generated manifest is recorded because none is
evidenced by the baseline history.

## Active selected inventory

The authoritative declaration file contains exactly 87 active selected
production type IDs. A range is used only where every hexadecimal value in it is
declared.

| Type IDs | Active declarations |
| --- | --- |
| `0x5400000000000010` to `0x5400000000000039` | `TerminalControlCode` through `TerminalOrdinaryFeature`, including new `TerminalRecoveryLedgerKind`, `TerminalRecoveryToken`, `TerminalCoordinatorState`, and `TerminalDiagnosticSessionState` |
| `0x5400000000000040` to `0x540000000000004a` | `TerminalMouseTracking` through `TerminalModifiers` |
| `0x540000000000004c` to `0x5400000000000059` | `TerminalNamedKey` through `TerminalMouseButton` |
| `0x5400000000000060` to `0x540000000000006d` | `TerminalInputEvent` through new `TerminalDiagnosticRetryability` |
| `0x5400000000000070` to `0x5400000000000075` | `TerminalSessionOptionsError`, `TerminalSessionOpenError`, `TerminalSessionReadError`, `TerminalSessionWriteError`, `TerminalSessionStateError`, `TerminalSessionRestoreError` |

Intentional selected type-ID gaps remain `0x540000000000004b`,
`0x540000000000005a` through `0x540000000000005f`, and
`0x540000000000006e` through `0x540000000000006f`. They are unallocated, not
retired, and are not included in a range above.

### Selected explicit active variant IDs

| Declaration | Variant IDs |
| --- | --- |
| `TerminalRecoveryLedgerKind` | `OpenRollback=1`, `CloseRestore=2` |
| `TerminalCoordinatorState` | `Free=1`, `Opening=2`, `Active=3`, `Paused=4`, `RestorePending=5`, `FailedOpenRecovery=6`, `FailedCloseRecovery=7` |
| `TerminalDiagnosticSessionState` | `Unavailable=1`, `Active=2`, `Paused=3`, `RestorePending=4`, `Closed=5` |
| `TerminalOrdinaryFeature` | `AlternateScreen=1`, `CursorShape=2`, `BracketedPaste=3`, `FocusEvents=4`, `MouseButtons=5`, `MouseMotion=6`, `KeyReleaseEvents=7`, `CompositionEvents=8`, `EnhancedKeyIdentity=9` |
| `TerminalMouseTracking` | `Disabled=1`, `Buttons=2`, `ButtonsAndDrag=3`, `AllMotion=4` |
| `TerminalCursorShape` | `Default=1`, `BlinkingBlock=2`, `SteadyBlock=3`, `BlinkingUnderline=4`, `SteadyUnderline=5`, `BlinkingBar=6`, `SteadyBar=7` |
| `TerminalCapabilitySupportedEvidence` | `NativeConfirmed=1`, `ProtocolQueried=2`, `EnvironmentInferred=3`, `ProtocolAssumed=4` |
| `TerminalTrustedPasteEvidence` | `NativeRecordBoundary=1`, `SanitizedProtocolBoundary=2` |
| `TerminalCapabilityUnsupportedEvidence` | `NativeUnavailable=1`, `ProtocolRejected=2`, `EnvironmentMissing=3` |
| `TerminalFeatureCapability` | `Unsupported=1`, `Available=2`, `Enabled=3` |
| `TerminalTrustedPasteCapability` | `Unsupported=1`, `Available=2`, `Enabled=3` |
| `TerminalColorCapability` | `Unsupported=1`, `Monochrome=2`, `Indexed=3`, `TrueColor=4` |
| `TerminalNamedKey` | `Enter=1`, `Escape=2`, `Backspace=3`, `Tab=4`, `BackTab=5`, `ArrowUp=6`, `ArrowDown=7`, `ArrowLeft=8`, `ArrowRight=9`, `Insert=10`, `Delete=11`, `Home=12`, `End=13`, `PageUp=14`, `PageDown=15` |
| `TerminalLogicalKey` | `Text=1`, `Control=2`, `Named=3`, `Function=4` |
| `TerminalKeyOccurrence` | `Press=1`, `Repeat=2`, `Release=3` |
| `TerminalTextInputOrigin` | `Direct=1`, `Key=2`, `Composition=3` |
| `TerminalLinkedTextPhase` | `Complete=1`, `Start=2`, `Continue=3`, `End=4` |
| `TerminalCompositionEnd` | `Committed=1`, `Cancelled=2`, `Interrupted=3` |
| `TerminalPastePhase` | `Complete=1`, `Start=2`, `Continue=3`, `End=4` |
| `TerminalUnknownBytesReason` | `UnrecognizedSequence=1`, `SequenceLimitExceeded=2`, `PasteContainsNul=3`, `PasteInvalidUtf8=4`, `BackendOverflow=5` |
| `TerminalNativeEventKind` | `WindowsMenu=1`, `WindowsUnknownRecord=2`, `VtPrivateSequence=3`, `BackendSpecific=4`, `Other=5` |
| `TerminalInputResetReason` | `PauseBoundary=1`, `PasteFallback=2`, `SequenceLimitExceeded=3`, `BackendReset=4`, `BackendOverflow=5`, `CompositionInterrupted=6` |
| `TerminalMouseAction` | `Press=1`, `Release=2`, `Move=3`, `Drag=4`, `Scroll=5` |
| `TerminalScrollDirection` | `Up=1`, `Down=2`, `Left=3`, `Right=4` |
| `TerminalMouseButton` | `Left=1`, `Middle=2`, `Right=3`, `AuxiliaryOne=4`, `AuxiliaryTwo=5` |
| `TerminalInputEvent` | `Key=1`, `TextInput=2`, `CompositionStarted=3`, `CompositionUpdated=4`, `CompositionEnded=5`, `Paste=6`, `Mouse=7`, `Resize=8`, `FocusGained=9`, `FocusLost=10`, `TimedOut=11`, `Cancelled=12`, `EndOfInput=13`, `UnknownBytes=14`, `UnknownNative=15`, `InputReset=16` |
| `TerminalWait` | `Poll=1`, `Forever=2`, `For=3` |
| `TerminalCloseOutcome` | `Clean=1`, `DiscardedInput=2` |
| `TerminalBackend` | `LinuxVt=1`, `WindowsConsole=2`, `WindowsConPty=3`, `VtStream=4`, `UnsupportedPlatform=5` |
| `TerminalOperation` | `Open=1`, `Read=2`, `Write=3`, `Flush=4`, `QuerySize=5`, `Pause=7`, `Resume=8`, `Close=9`, `RestorePendingOpen=10`, `RestorePendingClose=11`, `SetCursorVisibility=12`, `SetCursorShape=13`, `NegotiateCapability=14`, `Allocate=15`, `ValidateOptions=16`, `TakeInput=17`, `PrintText=18`, `FlushStandardOutput=19`, `StdoutWriter=20`, `WriterWrite=21`, `WriterFlush=22`, `StdoutTerminal=23`, `TerminalSupportsAnsi=24`, `TerminalClearScreenOn=25`, `TerminalMoveCursorOn=26`, `TerminalDrawRows=27`, `TerminalClearScreen=28`, `TerminalMoveCursor=29` |
| `TerminalDiagnosticStage` | `Snapshot=1`, `AcquireOwnership=2`, `ConfigureInput=3`, `ConfigureOutput=4`, `EnableProtocol=5`, `Wait=6`, `Decode=7`, `Allocate=8`, `ReverseProtocol=9`, `RestoreOperatingSystemState=10`, `ReleaseOwnership=11`, `ValidateState=12`, `ValidateOptions=13` |
| `TerminalSessionState` | `Active=2`, `Paused=3`, `RestorePending=4`, `Closed=5` |
| `TerminalOsCode` | `Unavailable=1`, `PosixErrno=2`, `WindowsError=3` |
| `TerminalFeature` | `AlternateScreen=1`, `CursorShape=2`, `BracketedPaste=3`, `FocusEvents=4`, `MouseButtons=5`, `MouseMotion=6`, `KeyReleaseEvents=7`, `CompositionEvents=8`, `EnhancedKeyIdentity=9`, `TrustedPasteFraming=10` |
| `TerminalSessionOptionField` | `RetainedEvents=3`, `RetainedBytes=4`, `CorrelatedEvents=5`, `CorrelatedBytes=6`, `CommittedTextBytes=7`, `CompositionPreeditBytes=8`, `PasteChunkBytes=9`, `UnknownChunkBytes=10`, `PendingSequenceBytes=11`, `DiagnosticCount=12`, `DiagnosticBytes=13` |
| `TerminalInvalidOptions` | `RetainedCapacityTooSmall=2`, `CorrelatedGroupTooLarge=3` |
| `TerminalDiagnosticRetryability` | `NonRetryable=1`, `SameLiveSession=2`, `RecoveryToken=3` |
| `TerminalSessionOptionsError` | `InvalidOptions=1` |
| `TerminalSessionOpenError` | `InputNotInteractive=1`, `OutputNotInteractive=2`, `TerminalAlreadyOwned=3`, `InvalidOptions=4`, `UnsupportedFeature=5`, `ModeReadFailed=6`, `ModeWriteFailed=7`, `AllocationFailed=8`, `RollbackFailed=9`, `RecoveryPending=10`, `ResumeFailed=11`, `GenerationExhausted=12` |
| `TerminalSessionReadError` | `ReadFailed=1`, `AllocationFailed=2`, `PauseDeliveryFailed=3`, `IdentifierExhausted=4` |
| `TerminalSessionWriteError` | `WriteFailed=1`, `FlushFailed=2`, `UnsupportedCursorShape=3` |
| `TerminalSessionStateError` | `SessionActive=2`, `SessionPaused=3`, `SessionRestorePending=4`, `SessionClosed=5` |
| `TerminalSessionRestoreError` | `RestoreInputModeFailed=1`, `RestoreOutputModeFailed=2`, `RestoreScreenStateFailed=3`, `MultipleRestoreStepsFailed=4`, `PendingOpenRollbackFailed=5`, `CloseRestorePending=6`, `PendingCloseRestoreFailed=7`, `WrongSession=9`, `WrongKind=10`, `Stale=11`, `Consumed=12`, `RecoveryInProgress=13`, `ResumeRestorePending=15` |

The declaration file is the field and payload authority. In particular,
open-family `RollbackFailed` and `RecoveryPending`, restore-family
`PendingOpenRollbackFailed`, `CloseRestorePending`, and
`PendingCloseRestoreFailed` carry `TerminalRecoveryToken` aliases. The
live-binding `ResumeRestorePending=15` carries diagnostics and no token.
Open-family `GenerationExhausted=12` is the only reachable generation-capacity
failure; restore-family value `14` is permanently unavailable.

## Active chord inventory

Commit `384f502718e0a85f40862bfda0d02455b2344575` introduced the contiguous
baseline `0x5400000000000100` through `0x5400000000000113`. This chord
declaration revision, `docs(terminal): add atomic chord mutation`, adds only the
next unused IDs `0x5400000000000114` and `0x5400000000000115`. The active range
is contiguous and contains exactly 22 chord production type IDs.

| Type IDs | Active declarations |
| --- | --- |
| `0x5400000000000100` to `0x5400000000000113` | Baseline `TerminalChordTrigger` through `TerminalChordRouter` |
| `0x5400000000000114` | `TerminalChordMutationResult` |
| `0x5400000000000115` | `TerminalChordMutationError` |

### Chord explicit active variant IDs

| Declaration | Variant IDs |
| --- | --- |
| `TerminalChordTrigger` | `Press=1`, `PressOrRepeat=2`, `Release=3` |
| `TerminalChordKey` | `Control=1`, `Named=2`, `Function=3`, `EnhancedText=4` |
| `TerminalLockModifierMask` | `IgnoreLocks=1`, `MatchCapsLock=2`, `MatchNumLock=3`, `MatchAllLocks=4` |
| `TerminalChordTextPolicy` | `PreserveLinkedText=1`, `SuppressLinkedText=2` |
| `TerminalChordPrefixPolicy` | `RejectAmbiguousPrefixes=1`, `HigherPriorityWins=2`, `LongestThenPriority=3` |
| `TerminalChordResetReason` | `ApplicationRequested=1`, `FocusLost=2`, `InputReset=3`, `Cancelled=4`, `EndOfInput=5`, `Pause=6` |
| `TerminalChordRouterOutput` | `Pending=1 { deadline: MonotonicDeadline }`, `ReleasedInput=2`, `Activated=3`, `Idle=4`, `AwaitingCorrelatedInput=5` |
| `TerminalChordValidationError` | `EmptySequence=1`, `SequenceTooLong=2`, `RegistrationLimitReached=3`, `DuplicateBinding=4`, `PrefixAmbiguity=5`, `EnhancedKeyIdentityRequired=6`, `ReleaseEventsRequired=7`, `BindingIdentifierExhausted=8` |
| `TerminalChordMutationResult` | `Unregistered=1`, `Replaced=2` |
| `TerminalChordMutationError` | `BindingNotFound=1`, `WrongRouter=2`, `CorrelatedGroupPending=3` |

### Chord representation and additive-variant record

`TerminalChordBindingId` retains type ID `0x540000000000010e` and changes from
the baseline constrained numeric surface to an opaque immutable,
standard-library-constructed identity with hidden router provenance and a hidden
monotonic never-reused ordinal. Only the ordinal inspector is public. This is a
current representation change, not an ID retirement or reassignment; repository
history evidences no representation hash or representation version to record.

`TerminalChordRouterOutput.Pending=1` retains discriminator 1 and gains the
exact `MonotonicDeadline` payload required to arm the caller-owned affine timer.
`Idle=4` and `AwaitingCorrelatedInput=5` use previously unused discriminators.
No existing chord discriminator is reassigned or retired.

## Retired selected IDs

The Task 1 selected baseline introduced the active explicit variant IDs retired
below. Their retirement occurs in the same selected declaration revision as this
history update. Every value is permanently unavailable and is absent from the
active declarations.

| Retired ID | Former declaration | Retirement reason | Introduction evidence | Retirement evidence | Replacement | Prior representation evidence |
| --- | --- | --- | --- | --- | --- | --- |
| `TerminalOperation.AcquireOutputTerminal=6` | Output-terminal acquisition operation | The selected API removes session-derived `StdoutTerminal` authority. | Selected baseline `4278ed359e2186251f81dfbf280caecf69aa3678` | This selected declaration revision, `docs(terminal): define recovery and diagnostic ABI` | None | Unavailable |
| `TerminalSessionState.Opening=1` | Binding state | Opening is coordinator-only before a session binding exists. | Selected baseline `4278ed359e2186251f81dfbf280caecf69aa3678` | This selected declaration revision, `docs(terminal): define recovery and diagnostic ABI` | `TerminalCoordinatorState.Opening=2` | Unavailable |
| `TerminalSessionState.FailedOpenRecovery=6` | Binding state | Failed-open recovery is process-owned and has no session binding. | Selected baseline `4278ed359e2186251f81dfbf280caecf69aa3678` | This selected declaration revision, `docs(terminal): define recovery and diagnostic ABI` | `TerminalCoordinatorState.FailedOpenRecovery=6` | Unavailable |
| `TerminalSessionState.FailedCloseRecovery=7` | Binding state | Failed-close recovery is process-owned after cleanup consumes the binding. | Selected baseline `4278ed359e2186251f81dfbf280caecf69aa3678` | This selected declaration revision, `docs(terminal): define recovery and diagnostic ABI` | `TerminalCoordinatorState.FailedCloseRecovery=7` | Unavailable |
| `TerminalSessionStateError.SessionOpening=1` | Session state error variant | Opening cannot be returned as a rejection for a binding that does not yet exist. | Selected baseline `4278ed359e2186251f81dfbf280caecf69aa3678` | This selected declaration revision, `docs(terminal): define recovery and diagnostic ABI` | `TerminalCoordinatorState.Opening=2` | Unavailable |
| `TerminalSessionStateError.SessionFailedRecovery=6` | Session state error variant | Process-owned recovery has no live binding and therefore no binding-state rejection variant. | Selected baseline `4278ed359e2186251f81dfbf280caecf69aa3678` | This selected declaration revision, `docs(terminal): define recovery and diagnostic ABI` | None | Unavailable |
| `TerminalSessionRestoreError.RecoveryOwnerMismatch=8` | Recovery error variant | Authenticated token validation now distinguishes provenance, kind, stale, consumed, and concurrent-claim failures. | Selected baseline `4278ed359e2186251f81dfbf280caecf69aa3678` | This selected declaration revision, `docs(terminal): define recovery and diagnostic ABI` | `WrongSession=9`, `WrongKind=10`, `Stale=11`, `Consumed=12`, `RecoveryInProgress=13` | Unavailable |

## Permanently unavailable reviewed candidate ID

`TerminalSessionRestoreError.GenerationExhausted=14` appeared only in an
uncommitted reviewed candidate and has no introduction commit. This selected
revision permanently records value `14` as retired and never reusable because
every ownership epoch reserves recovery-generation capacity before `Opening`;
process-owned transfer cannot exhaust, and retries reuse the same issued
generation. The sole reachable capacity failure is
`TerminalSessionOpenError.GenerationExhausted=12` during open-family preflight.

## Chord retirement status

The chord retired set is evidenced empty from the baseline
`384f502718e0a85f40862bfda0d02455b2344575` through this chord declaration
revision, `docs(terminal): add atomic chord mutation`. No chord type ID or
explicit variant ID is removed, reassigned, or retired. In particular,
`TerminalChordBindingId` retains `0x540000000000010e`, and
`TerminalChordRouterOutput.Pending` retains discriminator 1. Every future chord
retirement remains append-only and permanently never reusable under the
same-revision rule above.
