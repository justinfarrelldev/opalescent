#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ActiveVariantIdTable {
    pub(super) declaration: &'static str,
    pub(super) variants: &'static [(&'static str, i64)],
}

impl ActiveVariantIdTable {
    const fn new(declaration: &'static str, variants: &'static [(&'static str, i64)]) -> Self {
        Self {
            declaration,
            variants,
        }
    }

    pub(super) fn expected_variant_id(self, variant_name: &str) -> Option<i64> {
        self.variants
            .iter()
            .find_map(|(name, id)| (*name == variant_name).then_some(*id))
    }
}

pub(super) const ACTIVE_SELECTED_VARIANT_IDS: &[ActiveVariantIdTable] = &[
    ActiveVariantIdTable::new(
        "TerminalRecoveryLedgerKind",
        &[("OpenRollback", 1), ("CloseRestore", 2)],
    ),
    ActiveVariantIdTable::new(
        "TerminalCoordinatorState",
        &[
            ("Free", 1),
            ("Opening", 2),
            ("Active", 3),
            ("Paused", 4),
            ("RestorePending", 5),
            ("FailedOpenRecovery", 6),
            ("FailedCloseRecovery", 7),
        ],
    ),
    ActiveVariantIdTable::new(
        "TerminalDiagnosticSessionState",
        &[
            ("Unavailable", 1),
            ("Active", 2),
            ("Paused", 3),
            ("RestorePending", 4),
            ("Closed", 5),
        ],
    ),
    ActiveVariantIdTable::new(
        "TerminalOrdinaryFeature",
        &[
            ("AlternateScreen", 1),
            ("CursorShape", 2),
            ("BracketedPaste", 3),
            ("FocusEvents", 4),
            ("MouseButtons", 5),
            ("MouseMotion", 6),
            ("KeyReleaseEvents", 7),
            ("CompositionEvents", 8),
            ("EnhancedKeyIdentity", 9),
        ],
    ),
    ActiveVariantIdTable::new(
        "TerminalMouseTracking",
        &[
            ("Disabled", 1),
            ("Buttons", 2),
            ("ButtonsAndDrag", 3),
            ("AllMotion", 4),
        ],
    ),
    ActiveVariantIdTable::new(
        "TerminalCursorShape",
        &[
            ("Default", 1),
            ("BlinkingBlock", 2),
            ("SteadyBlock", 3),
            ("BlinkingUnderline", 4),
            ("SteadyUnderline", 5),
            ("BlinkingBar", 6),
            ("SteadyBar", 7),
        ],
    ),
    ActiveVariantIdTable::new(
        "TerminalCapabilitySupportedEvidence",
        &[
            ("NativeConfirmed", 1),
            ("ProtocolQueried", 2),
            ("EnvironmentInferred", 3),
            ("ProtocolAssumed", 4),
        ],
    ),
    ActiveVariantIdTable::new(
        "TerminalTrustedPasteEvidence",
        &[
            ("NativeRecordBoundary", 1),
            ("SanitizedProtocolBoundary", 2),
        ],
    ),
    ActiveVariantIdTable::new(
        "TerminalCapabilityUnsupportedEvidence",
        &[
            ("NativeUnavailable", 1),
            ("ProtocolRejected", 2),
            ("EnvironmentMissing", 3),
        ],
    ),
    ActiveVariantIdTable::new(
        "TerminalFeatureCapability",
        &[("Unsupported", 1), ("Available", 2), ("Enabled", 3)],
    ),
    ActiveVariantIdTable::new(
        "TerminalTrustedPasteCapability",
        &[("Unsupported", 1), ("Available", 2), ("Enabled", 3)],
    ),
    ActiveVariantIdTable::new(
        "TerminalColorCapability",
        &[
            ("Unsupported", 1),
            ("Monochrome", 2),
            ("Indexed", 3),
            ("TrueColor", 4),
        ],
    ),
    ActiveVariantIdTable::new(
        "TerminalNamedKey",
        &[
            ("Enter", 1),
            ("Escape", 2),
            ("Backspace", 3),
            ("Tab", 4),
            ("BackTab", 5),
            ("ArrowUp", 6),
            ("ArrowDown", 7),
            ("ArrowLeft", 8),
            ("ArrowRight", 9),
            ("Insert", 10),
            ("Delete", 11),
            ("Home", 12),
            ("End", 13),
            ("PageUp", 14),
            ("PageDown", 15),
        ],
    ),
    ActiveVariantIdTable::new(
        "TerminalLogicalKey",
        &[("Text", 1), ("Control", 2), ("Named", 3), ("Function", 4)],
    ),
    ActiveVariantIdTable::new(
        "TerminalKeyOccurrence",
        &[("Press", 1), ("Repeat", 2), ("Release", 3)],
    ),
    ActiveVariantIdTable::new(
        "TerminalTextInputOrigin",
        &[("Direct", 1), ("Key", 2), ("Composition", 3)],
    ),
    ActiveVariantIdTable::new(
        "TerminalLinkedTextPhase",
        &[("Complete", 1), ("Start", 2), ("Continue", 3), ("End", 4)],
    ),
    ActiveVariantIdTable::new(
        "TerminalCompositionEnd",
        &[("Committed", 1), ("Cancelled", 2), ("Interrupted", 3)],
    ),
    ActiveVariantIdTable::new(
        "TerminalPastePhase",
        &[("Complete", 1), ("Start", 2), ("Continue", 3), ("End", 4)],
    ),
    ActiveVariantIdTable::new(
        "TerminalUnknownBytesReason",
        &[
            ("UnrecognizedSequence", 1),
            ("SequenceLimitExceeded", 2),
            ("PasteContainsNul", 3),
            ("PasteInvalidUtf8", 4),
            ("BackendOverflow", 5),
        ],
    ),
    ActiveVariantIdTable::new(
        "TerminalNativeEventKind",
        &[
            ("WindowsMenu", 1),
            ("WindowsUnknownRecord", 2),
            ("VtPrivateSequence", 3),
            ("BackendSpecific", 4),
            ("Other", 5),
        ],
    ),
    ActiveVariantIdTable::new(
        "TerminalInputResetReason",
        &[
            ("PauseBoundary", 1),
            ("PasteFallback", 2),
            ("SequenceLimitExceeded", 3),
            ("BackendReset", 4),
            ("BackendOverflow", 5),
            ("CompositionInterrupted", 6),
        ],
    ),
    ActiveVariantIdTable::new(
        "TerminalMouseAction",
        &[
            ("Press", 1),
            ("Release", 2),
            ("Move", 3),
            ("Drag", 4),
            ("Scroll", 5),
        ],
    ),
    ActiveVariantIdTable::new(
        "TerminalScrollDirection",
        &[("Up", 1), ("Down", 2), ("Left", 3), ("Right", 4)],
    ),
    ActiveVariantIdTable::new(
        "TerminalMouseButton",
        &[
            ("Left", 1),
            ("Middle", 2),
            ("Right", 3),
            ("AuxiliaryOne", 4),
            ("AuxiliaryTwo", 5),
        ],
    ),
    ActiveVariantIdTable::new(
        "TerminalInputEvent",
        &[
            ("Key", 1),
            ("TextInput", 2),
            ("CompositionStarted", 3),
            ("CompositionUpdated", 4),
            ("CompositionEnded", 5),
            ("Paste", 6),
            ("Mouse", 7),
            ("Resize", 8),
            ("FocusGained", 9),
            ("FocusLost", 10),
            ("TimedOut", 11),
            ("Cancelled", 12),
            ("EndOfInput", 13),
            ("UnknownBytes", 14),
            ("UnknownNative", 15),
            ("InputReset", 16),
        ],
    ),
    ActiveVariantIdTable::new("TerminalWait", &[("Poll", 1), ("Forever", 2), ("For", 3)]),
    ActiveVariantIdTable::new(
        "TerminalCloseOutcome",
        &[("Clean", 1), ("DiscardedInput", 2)],
    ),
    ActiveVariantIdTable::new(
        "TerminalBackend",
        &[
            ("LinuxVt", 1),
            ("WindowsConsole", 2),
            ("WindowsConPty", 3),
            ("VtStream", 4),
            ("UnsupportedPlatform", 5),
        ],
    ),
    ActiveVariantIdTable::new(
        "TerminalOperation",
        &[
            ("Open", 1),
            ("Read", 2),
            ("Write", 3),
            ("Flush", 4),
            ("QuerySize", 5),
            ("Pause", 7),
            ("Resume", 8),
            ("Close", 9),
            ("RestorePendingOpen", 10),
            ("RestorePendingClose", 11),
            ("SetCursorVisibility", 12),
            ("SetCursorShape", 13),
            ("NegotiateCapability", 14),
            ("Allocate", 15),
            ("ValidateOptions", 16),
            ("TakeInput", 17),
            ("PrintText", 18),
            ("FlushStandardOutput", 19),
            ("StdoutWriter", 20),
            ("WriterWrite", 21),
            ("WriterFlush", 22),
            ("StdoutTerminal", 23),
            ("TerminalSupportsAnsi", 24),
            ("TerminalClearScreenOn", 25),
            ("TerminalMoveCursorOn", 26),
            ("TerminalDrawRows", 27),
            ("TerminalClearScreen", 28),
            ("TerminalMoveCursor", 29),
        ],
    ),
    ActiveVariantIdTable::new(
        "TerminalDiagnosticStage",
        &[
            ("Snapshot", 1),
            ("AcquireOwnership", 2),
            ("ConfigureInput", 3),
            ("ConfigureOutput", 4),
            ("EnableProtocol", 5),
            ("Wait", 6),
            ("Decode", 7),
            ("Allocate", 8),
            ("ReverseProtocol", 9),
            ("RestoreOperatingSystemState", 10),
            ("ReleaseOwnership", 11),
            ("ValidateState", 12),
            ("ValidateOptions", 13),
        ],
    ),
    ActiveVariantIdTable::new(
        "TerminalSessionState",
        &[
            ("Active", 2),
            ("Paused", 3),
            ("RestorePending", 4),
            ("Closed", 5),
        ],
    ),
    ActiveVariantIdTable::new(
        "TerminalOsCode",
        &[("Unavailable", 1), ("PosixErrno", 2), ("WindowsError", 3)],
    ),
    ActiveVariantIdTable::new(
        "TerminalFeature",
        &[
            ("AlternateScreen", 1),
            ("CursorShape", 2),
            ("BracketedPaste", 3),
            ("FocusEvents", 4),
            ("MouseButtons", 5),
            ("MouseMotion", 6),
            ("KeyReleaseEvents", 7),
            ("CompositionEvents", 8),
            ("EnhancedKeyIdentity", 9),
            ("TrustedPasteFraming", 10),
        ],
    ),
    ActiveVariantIdTable::new(
        "TerminalSessionOptionField",
        &[
            ("RetainedEvents", 3),
            ("RetainedBytes", 4),
            ("CorrelatedEvents", 5),
            ("CorrelatedBytes", 6),
            ("CommittedTextBytes", 7),
            ("CompositionPreeditBytes", 8),
            ("PasteChunkBytes", 9),
            ("UnknownChunkBytes", 10),
            ("PendingSequenceBytes", 11),
            ("DiagnosticCount", 12),
            ("DiagnosticBytes", 13),
        ],
    ),
    ActiveVariantIdTable::new(
        "TerminalInvalidOptions",
        &[
            ("RetainedCapacityTooSmall", 2),
            ("CorrelatedGroupTooLarge", 3),
        ],
    ),
    ActiveVariantIdTable::new(
        "TerminalDiagnosticRetryability",
        &[
            ("NonRetryable", 1),
            ("SameLiveSession", 2),
            ("RecoveryToken", 3),
        ],
    ),
    ActiveVariantIdTable::new("TerminalSessionOptionsError", &[("InvalidOptions", 1)]),
    ActiveVariantIdTable::new(
        "TerminalSessionOpenError",
        &[
            ("InputNotInteractive", 1),
            ("OutputNotInteractive", 2),
            ("TerminalAlreadyOwned", 3),
            ("InvalidOptions", 4),
            ("UnsupportedFeature", 5),
            ("ModeReadFailed", 6),
            ("ModeWriteFailed", 7),
            ("AllocationFailed", 8),
            ("RollbackFailed", 9),
            ("RecoveryPending", 10),
            ("ResumeFailed", 11),
            ("GenerationExhausted", 12),
        ],
    ),
    ActiveVariantIdTable::new(
        "TerminalSessionReadError",
        &[
            ("ReadFailed", 1),
            ("AllocationFailed", 2),
            ("PauseDeliveryFailed", 3),
            ("IdentifierExhausted", 4),
        ],
    ),
    ActiveVariantIdTable::new(
        "TerminalSessionWriteError",
        &[
            ("WriteFailed", 1),
            ("FlushFailed", 2),
            ("UnsupportedCursorShape", 3),
        ],
    ),
    ActiveVariantIdTable::new(
        "TerminalSessionStateError",
        &[
            ("SessionActive", 2),
            ("SessionPaused", 3),
            ("SessionRestorePending", 4),
            ("SessionClosed", 5),
        ],
    ),
    ActiveVariantIdTable::new(
        "TerminalSessionRestoreError",
        &[
            ("RestoreInputModeFailed", 1),
            ("RestoreOutputModeFailed", 2),
            ("RestoreScreenStateFailed", 3),
            ("MultipleRestoreStepsFailed", 4),
            ("PendingOpenRollbackFailed", 5),
            ("CloseRestorePending", 6),
            ("PendingCloseRestoreFailed", 7),
            ("WrongSession", 9),
            ("WrongKind", 10),
            ("Stale", 11),
            ("Consumed", 12),
            ("RecoveryInProgress", 13),
            ("ResumeRestorePending", 15),
        ],
    ),
];

pub(super) const ACTIVE_CHORD_VARIANT_IDS: &[ActiveVariantIdTable] = &[
    ActiveVariantIdTable::new(
        "TerminalChordTrigger",
        &[("Press", 1), ("PressOrRepeat", 2), ("Release", 3)],
    ),
    ActiveVariantIdTable::new(
        "TerminalChordKey",
        &[
            ("Control", 1),
            ("Named", 2),
            ("Function", 3),
            ("EnhancedText", 4),
        ],
    ),
    ActiveVariantIdTable::new(
        "TerminalLockModifierMask",
        &[
            ("IgnoreLocks", 1),
            ("MatchCapsLock", 2),
            ("MatchNumLock", 3),
            ("MatchAllLocks", 4),
        ],
    ),
    ActiveVariantIdTable::new(
        "TerminalChordTextPolicy",
        &[("PreserveLinkedText", 1), ("SuppressLinkedText", 2)],
    ),
    ActiveVariantIdTable::new(
        "TerminalChordPrefixPolicy",
        &[
            ("RejectAmbiguousPrefixes", 1),
            ("HigherPriorityWins", 2),
            ("LongestThenPriority", 3),
        ],
    ),
    ActiveVariantIdTable::new(
        "TerminalChordResetReason",
        &[
            ("ApplicationRequested", 1),
            ("FocusLost", 2),
            ("InputReset", 3),
            ("Cancelled", 4),
            ("EndOfInput", 5),
            ("Pause", 6),
        ],
    ),
    ActiveVariantIdTable::new(
        "TerminalChordRouterOutput",
        &[
            ("Pending", 1),
            ("ReleasedInput", 2),
            ("Activated", 3),
            ("Idle", 4),
            ("AwaitingCorrelatedInput", 5),
        ],
    ),
    ActiveVariantIdTable::new(
        "TerminalChordValidationError",
        &[
            ("EmptySequence", 1),
            ("SequenceTooLong", 2),
            ("RegistrationLimitReached", 3),
            ("DuplicateBinding", 4),
            ("PrefixAmbiguity", 5),
            ("EnhancedKeyIdentityRequired", 6),
            ("ReleaseEventsRequired", 7),
            ("BindingIdentifierExhausted", 8),
            ("BufferCapacityBelowCorrelatedLimit", 9),
        ],
    ),
    ActiveVariantIdTable::new(
        "TerminalChordMutationResult",
        &[("Unregistered", 1), ("Replaced", 2)],
    ),
    ActiveVariantIdTable::new(
        "TerminalChordMutationError",
        &[
            ("BindingNotFound", 1),
            ("WrongRouter", 2),
            ("CorrelatedGroupPending", 3),
        ],
    ),
    ActiveVariantIdTable::new(
        "TerminalChordProcessError",
        &[
            ("BufferedCapacityExceeded", 1),
            ("WrongInputStream", 2),
            ("DeliveryAlreadyConsumed", 3),
            ("DeliveryOutOfOrder", 4),
        ],
    ),
];
