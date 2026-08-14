# Terminal ABI History

## Scope and baseline

This is the authoritative append-only history for terminal ABI ownership. Its
active inventory is the committed Task 1 baseline:

- selected declarations: commit `4278ed359e2186251f81dfbf280caecf69aa3678`;
- chord declarations: commit `384f502718e0a85f40862bfda0d02455b2344575`.

`6067a1d` introduced `typed_event_session.types.op` before it carried
`@abi_type_id` annotations or authoritative ABI IDs. Commit `4278ed3` checkpointed
the current selected declaration bytes and is the supported introduction baseline
for the selected active inventory below. Commit `384f502` introduced
`terminal_chords.types.op` and is the supported introduction baseline for the
chord active inventory below.

The selected baseline has 82 active type IDs and the chord baseline has 20 active
type IDs. Later uncommitted proposal revisions are not active history.

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

Every row below is active at `4278ed359e2186251f81dfbf280caecf69aa3678`. A range
is used only where every hexadecimal value in it is declared.

| Type IDs | Active declarations |
| --- | --- |
| `0x5400000000000010` to `0x540000000000002c` | `TerminalControlCode` through `TerminalCompositionPreeditText` |
| `0x540000000000002e` | `TerminalSession` |
| `0x5400000000000032` to `0x5400000000000039` | `TrustedTerminalOutput`, `SafeTerminalDiagnosticOutput`, `TerminalPauseEvents`, `TerminalDiagnosticCollection`, `TerminalSessionOptions`, `TerminalSessionFeaturePolicy`, `TerminalSessionResourceLimits`, `TerminalOrdinaryFeature` |
| `0x5400000000000040` to `0x540000000000004a` | `TerminalMouseTracking` through `TerminalModifiers` |
| `0x540000000000004c` to `0x5400000000000059` | `TerminalNamedKey` through `TerminalMouseButton` |
| `0x5400000000000060` to `0x540000000000006c` | `TerminalInputEvent` through `TerminalInvalidOptions` |
| `0x5400000000000070` to `0x5400000000000075` | `TerminalSessionOptionsError`, `TerminalSessionOpenError`, `TerminalSessionReadError`, `TerminalSessionWriteError`, `TerminalSessionStateError`, `TerminalSessionRestoreError` |

### Selected explicit active variant IDs

| Declaration | Variant IDs |
| --- | --- |
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
| `TerminalOperation` | `Open=1`, `Read=2`, `Write=3`, `Flush=4`, `QuerySize=5`, `AcquireOutputTerminal=6`, `Pause=7`, `Resume=8`, `Close=9`, `RestorePendingOpen=10`, `RestorePendingClose=11`, `SetCursorVisibility=12`, `SetCursorShape=13`, `NegotiateCapability=14`, `Allocate=15`, `ValidateOptions=16` |
| `TerminalDiagnosticStage` | `Snapshot=1`, `AcquireOwnership=2`, `ConfigureInput=3`, `ConfigureOutput=4`, `EnableProtocol=5`, `Wait=6`, `Decode=7`, `Allocate=8`, `ReverseProtocol=9`, `RestoreOperatingSystemState=10`, `ReleaseOwnership=11`, `ValidateState=12`, `ValidateOptions=13` |
| `TerminalSessionState` | `Opening=1`, `Active=2`, `Paused=3`, `RestorePending=4`, `Closed=5`, `FailedOpenRecovery=6`, `FailedCloseRecovery=7` |
| `TerminalOsCode` | `Unavailable=1`, `PosixErrno=2`, `WindowsError=3` |
| `TerminalFeature` | `AlternateScreen=1`, `CursorShape=2`, `BracketedPaste=3`, `FocusEvents=4`, `MouseButtons=5`, `MouseMotion=6`, `KeyReleaseEvents=7`, `CompositionEvents=8`, `EnhancedKeyIdentity=9`, `TrustedPasteFraming=10` |
| `TerminalSessionOptionField` | `RetainedEvents=3`, `RetainedBytes=4`, `CorrelatedEvents=5`, `CorrelatedBytes=6`, `CommittedTextBytes=7`, `CompositionPreeditBytes=8`, `PasteChunkBytes=9`, `UnknownChunkBytes=10`, `PendingSequenceBytes=11`, `DiagnosticCount=12`, `DiagnosticBytes=13` |
| `TerminalInvalidOptions` | `RetainedCapacityTooSmall=2`, `CorrelatedGroupTooLarge=3` |
| `TerminalSessionOptionsError` | `InvalidOptions=1` |
| `TerminalSessionOpenError` | `InputNotInteractive=1`, `OutputNotInteractive=2`, `TerminalAlreadyOwned=3`, `InvalidOptions=4`, `UnsupportedFeature=5`, `ModeReadFailed=6`, `ModeWriteFailed=7`, `AllocationFailed=8`, `RollbackFailed=9`, `RecoveryPending=10`, `ResumeFailed=11` |
| `TerminalSessionReadError` | `ReadFailed=1`, `AllocationFailed=2`, `PauseDeliveryFailed=3`, `IdentifierExhausted=4` |
| `TerminalSessionWriteError` | `WriteFailed=1`, `FlushFailed=2`, `UnsupportedCursorShape=3` |
| `TerminalSessionStateError` | `SessionOpening=1`, `SessionActive=2`, `SessionPaused=3`, `SessionRestorePending=4`, `SessionClosed=5`, `SessionFailedRecovery=6` |
| `TerminalSessionRestoreError` | `RestoreInputModeFailed=1`, `RestoreOutputModeFailed=2`, `RestoreScreenStateFailed=3`, `MultipleRestoreStepsFailed=4`, `PendingOpenRollbackFailed=5`, `CloseRestorePending=6`, `PendingCloseRestoreFailed=7`, `RecoveryOwnerMismatch=8` |

## Active chord inventory

Every row below is active at `384f502718e0a85f40862bfda0d02455b2344575`.
The range is contiguous and contains all 20 chord type IDs.

| Type IDs | Active declarations |
| --- | --- |
| `0x5400000000000100` to `0x5400000000000113` | `TerminalChordTrigger` through `TerminalChordRouter` |

### Chord explicit active variant IDs

| Declaration | Variant IDs |
| --- | --- |
| `TerminalChordTrigger` | `Press=1`, `PressOrRepeat=2`, `Release=3` |
| `TerminalChordKey` | `Control=1`, `Named=2`, `Function=3`, `EnhancedText=4` |
| `TerminalLockModifierMask` | `IgnoreLocks=1`, `MatchCapsLock=2`, `MatchNumLock=3`, `MatchAllLocks=4` |
| `TerminalChordTextPolicy` | `PreserveLinkedText=1`, `SuppressLinkedText=2` |
| `TerminalChordPrefixPolicy` | `RejectAmbiguousPrefixes=1`, `HigherPriorityWins=2`, `LongestThenPriority=3` |
| `TerminalChordResetReason` | `ApplicationRequested=1`, `FocusLost=2`, `InputReset=3`, `Cancelled=4`, `EndOfInput=5`, `Pause=6` |
| `TerminalChordRouterOutput` | `Pending=1`, `ReleasedInput=2`, `Activated=3` |
| `TerminalChordValidationError` | `EmptySequence=1`, `SequenceTooLong=2`, `RegistrationLimitReached=3`, `DuplicateBinding=4`, `PrefixAmbiguity=5`, `EnhancedKeyIdentityRequired=6`, `ReleaseEventsRequired=7`, `BindingIdentifierExhausted=8` |

## Retired IDs

The retired set is evidenced empty for the inspected range: `6067a1d` through
`4278ed359e2186251f81dfbf280caecf69aa3678` for selected declarations and
`384f502718e0a85f40862bfda0d02455b2344575` for chord declarations. Read-only
history inspection found no prior `abi-history.md` and no deletion commit for
these declaration paths.

| Retired ID | Former declaration | Retirement reason | Retirement commit | Replacement | Prior representation evidence |
| --- | --- | --- | --- | --- | --- |
| None | None | No committed retirement, removal, or replacement was evidenced in the inspected baseline range. | None | None | None |
