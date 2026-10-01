const ERROR_LEAF_DOCUMENTATION: Readonly<Record<string, string>> = {
  AllocationFailureError: 'Reports that the runtime could not reserve or allocate memory needed to complete the operation. This keeps allocation-sensitive helpers fallible instead of trapping or returning partially built values.',
  BuilderFinishedError: 'Reports use of a `StringBuilder` after `string_builder_finish` has consumed its contents. It exists to make builder lifecycle mistakes explicit and recoverable.',
  ConstraintViolationError: 'Reports that a runtime value failed the bounds or invariant of a constrained standard-library type. It lets constructors reject invalid values without weakening the nominal type.',
  CopyFailureError: 'Reports that the operating system could not copy filesystem contents after path validation succeeded. Use it for device, metadata, or platform copy failures that are not more specific.',
  CreateFailureError: 'Reports that a file or directory could not be created for a reason other than existence, permissions, invalid path syntax, or full storage. It preserves a specific create-failure category for callers.',
  CurrentExecutablePathUnavailableError: 'Reports that the current executable path is not available from the host environment. It exists because some launch contexts cannot identify the running binary reliably.',
  CurrentWorkingDirectoryUnavailableError: 'Reports that the process current working directory cannot be read from the host. It exists because a process may lose access to, or outlive, its working directory.',
  DeleteFailureError: 'Reports that a filesystem delete operation failed after more specific path, permission, and type checks did not explain the failure. It keeps deletion failures distinct from read and write failures.',
  DirectoryNotEmptyError: 'Reports that a directory removal expected an empty directory but found entries inside it. It protects callers from accidentally deleting non-empty trees with a non-recursive operation.',
  DirectoryNotFoundError: 'Reports that a named directory path does not exist. It is separate from FileNotFoundError so directory-only APIs can communicate their expected path kind.',
  EnvironmentVariableNotFoundError: 'Reports that a requested environment variable is not present. It exists so callers can distinguish an absent variable from invalid names or undecodable values.',
  ErrorAttachmentAbsentError: 'Reports that the requested attachment is not present on an immutable error value, such as asking for a cause when no cause was retained.',
  FileAlreadyExistsError: 'Reports that creation or move requested a new path but the destination already exists. It lets callers choose between failing, overwriting, or selecting another path.',
  FileNotFoundError: 'Reports that the named path does not exist when a file-oriented operation needs an existing file. It exists so callers can create the file, skip it, or surface a clear missing-input message.',
  FilesystemFullError: 'Reports that storage space or filesystem quota prevented a write, append, copy, or create operation. It tells callers that retrying unchanged is unlikely to succeed until capacity changes.',
  FlushFailureError: 'Reports that buffered standard-output data could not be flushed to the host. It exists because a write may appear accepted while final delivery still fails.',
  HexDecodeError: 'Reports invalid hexadecimal text while decoding bytes. It exists to distinguish malformed test fixtures, hashes, IDs, or wire-format values from allocation or range failures.',
  IndexOutOfBoundsError: 'Reports that an indexed access requested an element outside the valid range. It exists so arrays, strings, diagnostics, pause events, and error attachments can be checked without panicking.',
  IntegerRangeError: 'Reports that a checked numeric conversion cannot represent the source value exactly in the destination type. Use it instead of unsafe narrowing when values are known only at runtime.',
  InvalidCursorPositionError: 'Reports a negative or otherwise invalid terminal cursor position before cursor movement is emitted. It prevents malformed terminal control sequences from being written.',
  InvalidDurationError: 'Reports a negative or unsupported duration for time operations such as sleep. It prevents time APIs from treating invalid values as platform-dependent sentinel values.',
  InvalidEnvironmentVariableNameError: 'Reports an invalid environment variable name, such as an empty name or one containing forbidden bytes. It exists so lookup and probe APIs reject names before consulting the host environment.',
  InvalidFrameRateError: 'Reports a zero, negative, or unsupported frame rate for fixed-rate frame clocks. It keeps animation and game loops from constructing clocks with nonsensical timing.',
  InvalidPathError: 'Reports that path text is malformed or cannot be represented as a host filesystem path. It exists to reject bad paths before a filesystem or process-path operation touches the host.',
  InvalidUtf8Error: 'Reports bytes that cannot be decoded as UTF-8 where text was requested. It keeps binary data from silently becoming replacement-character text.',
  IsADirectoryError: 'Reports that a file-oriented operation was given a directory. It exists so callers can branch to directory handling instead of treating the path as a failed file read or write.',
  IsNotADirectoryError: 'Reports that a directory-oriented operation was given a non-directory path. It exists so callers can separate wrong-kind paths from missing directories.',
  LineOutOfRangeError: 'Reports that a requested text line is outside the available line range. It exists for line-oriented readers that can distinguish EOF from I/O failure.',
  MetadataUnavailableError: 'Reports that filesystem metadata could not be obtained even though the path lookup reached the host. It preserves metadata failures separately from content reads.',
  MonotonicTimerError: 'Reports monotonic timer state changes that cannot be represented safely, currently generation exhaustion. It exists so stale wake detection never wraps or reuses a generation.',
  MonotonicTimerNotArmedError: 'Reports a request for a timer deadline while the monotonic timer is disarmed. It avoids an absent-deadline sentinel value.',
  MoveFailureError: 'Reports that a rename or move failed after more specific destination, permission, and path cases were ruled out. It captures platform and cross-device move failures.',
  OffsetOutOfRangeError: 'Reports that a byte offset is outside the valid range for a file operation. It prevents offset reads and writes from seeking to nonsensical positions.',
  ParseError: 'Reports that text could not be parsed into the requested primitive value. It exists so callers can distinguish bad input syntax from successful values.',
  PermissionDeniedError: 'Reports that filesystem or process permissions prevented an operation. It tells callers that credentials, ACLs, sandboxing, or access mode must change before retrying.',
  ProcessControlAcknowledgementError: 'Reports that a suspend acknowledgement could not be applied to the requested process-control generation. It exists to keep wrong, stale, and host-failed acknowledgements from mutating process-control state.',
  ProcessControlError: 'Reports failures while polling process-control notifications, such as host observation failure or generation exhaustion. It keeps suspend/continue notification handling separate from terminal input.',
  ProcessControlResumeError: 'Reports that application resume could not be recorded for the requested process-control generation. It exists so stale or wrong generations cannot accidentally resume application work.',
  ProcessControlUnavailableError: 'Reports that the host does not support process-control notifications for this runtime. It lets portable programs disable suspend/continue integration cleanly.',
  ReadFailureError: 'Reports an I/O failure while reading after the path was accepted. It separates host read errors from missing paths, invalid paths, and wrong path kinds.',
  SetPermissionsError: 'Reports that changing filesystem permissions failed after path validation. It exists because permission metadata updates can fail independently of file content operations.',
  SinkClosedError: 'Reports a write or flush attempted after the output sink was closed. It exists so output pipelines can stop writing without confusing closure with ordinary I/O failure.',
  SliceRangeError: 'Reports that a byte-slice range is invalid for the input buffer. It protects byte APIs from negative, reversed, or out-of-bounds slices.',
  StandardInputReadError: 'Reports a standard-input read failure, end-of-input, or terminal-coordinator rejection. It exists so interactive programs can distinguish input absence from parsed text.',
  StandardOutputCapabilityError: 'Reports inability to inspect standard-output terminal capabilities safely. It exists because a terminal lease can be unavailable, stale, or unsupported.',
  StandardOutputHandleError: 'Reports inability to acquire a standard-output writer or terminal handle. It exists to prevent callers from receiving a handle when the coordinator or host cannot provide one safely.',
  StringEmptySearchTextError: 'Reports a string search that used an empty pattern where a nonempty pattern is required. It avoids ambiguous answers for operations like contains, find, and replace.',
  StringNegativeCountError: 'Reports a negative count supplied to a string operation. It exists so repetition and range helpers reject invalid counts rather than interpreting them as zero or wrapping.',
  StringPatternNotFoundError: 'Reports that a required string pattern was not found. It lets callers distinguish unsuccessful searches from invalid search text.',
  StringRangeOrderError: 'Reports a string range whose start comes after its end. It exists so range helpers reject reversed spans before allocating or slicing.',
  StringRangeOutOfBoundsError: 'Reports a string range outside the valid Unicode-scalar positions for the string. It keeps text slicing Unicode-aware and bounds-checked.',
  SystemWaitSetError: 'Reports invalid wait-set registration state, such as using a registration with the wrong set, an unauthenticated registration, or an invalid registration lifetime. It protects wait sets from stale or forged readiness entries.',
  TerminalChordMutationError: 'Reports a chord-router mutation that cannot commit, such as replacing or unregistering an invalid binding. It exists so router state remains unchanged on failure.',
  TerminalChordProcessError: 'Reports a terminal input event the chord router cannot accept, such as wrong-stream, consumed, out-of-order, or capacity-exceeding input. It keeps router ordering and buffering invariants intact.',
  TerminalChordValidationError: 'Reports stable structured chord construction and registration failures. It exists to reject malformed chord sequences or policies before they enter a router.',
  TerminalSessionOpenError: 'Reports terminal-session open failures and resume failures that occur before ownership is safely published or after compensation. It exists so recovery authority and diagnostics remain structured.',
  TerminalSessionOptionsError: 'Reports invalid terminal-session option snapshots. It exists to validate resource limits and feature relationships before a session attempts to take terminal ownership.',
  TerminalSessionReadError: 'Reports terminal input or pause-delivery failures. It exists so readers can handle host read errors, identifier exhaustion, and end-of-input without losing session state.',
  TerminalSessionRestoreError: 'Reports terminal restoration, cleanup, or recovery-token rejection failures. It exists so applications can retry or surface recovery work without forging authority.',
  TerminalSessionStateError: 'Reports an operation that is invalid for the current terminal-session state. It prevents reads, writes, or transitions from mutating a closed, paused, or restore-pending session incorrectly.',
  TerminalSessionWriteError: 'Reports terminal output, cursor, or flush failures for an owned session. It exists so rendering code can distinguish session-write problems from state errors.',
  TerminalTestFactoryError: 'Reports rejection by a test-only factory for terminal scenarios, events, capabilities, diagnostics, or fake backends. It exists to keep deterministic tests within production-equivalent bounds.',
  TerminalTextLayoutError: 'Reports text layout input that cannot be rendered within terminal text-layout constraints. It exists so wrapping and truncation helpers reject impossible widths or ranges explicitly.',
  TerminalWriteFailureError: 'Reports failure to emit terminal control output to standard output or a legacy terminal handle. It separates terminal escape/control failures from ordinary text writes.',
  WriteFailureError: 'Reports failure to write text or bytes to a filesystem path or output sink after validation. It exists so callers can distinguish write I/O failures from flush, close, path, or capacity problems.'
};

const SYSTEM_TYPE_DOCUMENTATION: Readonly<Record<string, string>> = {
  CancellationSource: 'Affine authority used to request cancellation for one generation. Create it when several waits or reads need a shared, explicit stop signal.',
  CancellationToken: 'Immutable observation token for a `CancellationSource` generation. Pass it to waits or reads that should stop after `cancellation_request` is called.',
  Error: 'Opaque immutable view of a propagated error value for inspecting retained causes, suppressed errors, and truncation metadata without erasing the original nominal family for propagation.',
  ErrorAttachmentTruncation: 'Records whether an error value omitted cause, suppressed, or byte attachment data because bounded error-attachment limits were reached.',
  MonotonicDeadline: 'Monotonic clock instant used by timers and wait-set deadlines. It avoids wall-clock jumps when scheduling timeout-driven work.',
  MonotonicTimer: 'Affine timer resource with a stable readiness source and monotonically increasing generations. Use it to wake a wait set at deadlines without polling.',
  ProcessControlPollResult: 'Result of polling a process-control source. It is either a queued suspend/continue notification or `Idle` when no notification is observable.',
  ProcessControlSource: 'Affine source for host process-control notifications such as POSIX suspend and continue. It stays separate from terminal input so applications can coordinate pause and resume explicitly.',
  SystemOwnedWaitRegistration: 'Affine authority for one wait-set registration. It can outlive the registration call borrow and remove or retarget its exact entry safely.',
  SystemReadinessSource: 'Cloneable opaque identity for something a `SystemWaitSet` can observe, such as a terminal session, timer, cancellation token, or process-control source.',
  SystemWaitRegistration: 'Opaque token for an ordinary wait-set entry. It identifies one registered source so the owning wait set can remove it later.',
  SystemWaitSet: 'Affine collection of readiness sources. Register sources, then block on the set to learn which source may have work or whether cancellation won.',
  SystemWaitWake: 'Wake result from a wait set. `Ready` is a source/generation hint that callers must refine with the source-specific operation; `Cancelled` reports that cancellation won.'
};

const ERROR_SET_PURPOSES: Readonly<Record<string, string>> = {
  AllocationErrors: 'Groups allocation failures from standard-library operations that need runtime storage.',
  AnsiRenderErrors: 'Groups failures from setting up and using ANSI-style terminal rendering on standard output.',
  BytesDecodeErrors: 'Groups failures from decoding textual representations into bytes.',
  BytesErrors: 'Groups byte-buffer failures from decoding and slicing helpers.',
  BytesSliceErrors: 'Groups failures from extracting ranges from byte buffers.',
  ClockErrors: 'Groups sleep, frame-clock, and monotonic-timer failures for time-driven programs.',
  ConsoleIoErrors: 'Groups standard input, standard output, and legacy terminal-control failures for console programs.',
  ConstraintErrors: 'Groups constrained-type construction failures when runtime values violate declared bounds or invariants.',
  EnvironmentLookupErrors: 'Groups required environment-variable lookup failures, including absent names and invalid host data.',
  EnvironmentOptionalLookupErrors: 'Groups optional environment-variable lookup failures where absence is allowed but invalid names or values are not.',
  EnvironmentProbeErrors: 'Groups environment-variable probe failures caused by invalid variable names.',
  ErrorCauseInspectionErrors: 'Groups failures from inspecting an error cause when no cause attachment exists.',
  ErrorInspectionErrors: 'Groups failures from immutable error attachment inspection, including absent causes and out-of-range suppressed errors.',
  ErrorSuppressedInspectionErrors: 'Groups failures from indexing suppressed error attachments.',
  FilesystemAppendErrors: 'Groups failures from appending data to an existing file.',
  FilesystemCopyErrors: 'Groups failures from copying one filesystem path to another.',
  FilesystemCopyMoveErrors: 'Groups failures from copy and move operations that transfer data between paths.',
  FilesystemCreateErrors: 'Groups failures from creating new filesystem entries.',
  FilesystemDeleteErrors: 'Groups failures from deleting file-oriented paths.',
  FilesystemDirectoryCreateErrors: 'Groups failures from creating a single directory.',
  FilesystemDirectoryDeleteErrors: 'Groups failures from deleting directories with directory-specific checks.',
  FilesystemErrors: 'Groups broad filesystem failures for boundary code that intentionally handles any file, directory, metadata, copy, move, or permission problem.',
  FilesystemExistenceErrors: 'Groups failures from probing whether a filesystem path exists when path syntax or permissions prevent a reliable answer.',
  FilesystemLineReadErrors: 'Groups failures from line-oriented filesystem reads.',
  FilesystemListErrors: 'Groups failures from listing directory contents.',
  FilesystemMetadataErrors: 'Groups failures from reading filesystem metadata.',
  FilesystemMoveErrors: 'Groups failures from moving or renaming filesystem paths.',
  FilesystemMutationErrors: 'Groups failures from filesystem operations that create, modify, move, copy, delete, or change permissions.',
  FilesystemOffsetReadErrors: 'Groups failures from reading filesystem data at an explicit byte offset.',
  FilesystemOffsetWriteErrors: 'Groups failures from writing filesystem data at an explicit byte offset.',
  FilesystemOverwriteErrors: 'Groups failures from overwriting file contents.',
  FilesystemPathErrors: 'Groups failures from converting or inspecting filesystem paths before file I/O begins.',
  FilesystemPermissionErrors: 'Groups failures from reading or changing filesystem permissions.',
  FilesystemRawReadErrors: 'Groups failures from reading filesystem bytes without UTF-8 decoding.',
  FilesystemReadErrors: 'Groups failures from reading filesystem data, including missing files, permissions, host read errors, wrong path kinds, invalid paths, UTF-8 decoding, and offset bounds.',
  FilesystemRecursiveDirectoryCreateErrors: 'Groups failures from creating a directory tree and its missing ancestors.',
  FilesystemTextReadErrors: 'Groups failures from reading filesystem data as UTF-8 text.',
  FilesystemWriteErrors: 'Groups failures from writing filesystem data, including missing files, permissions, host write errors, wrong path kinds, invalid paths, full storage, and offset bounds.',
  FrameClockErrors: 'Groups failures from constructing or using fixed-rate frame clocks.',
  IndexAccessErrors: 'Groups out-of-bounds indexed access failures across collection-like standard-library values.',
  MonotonicTimerErrors: 'Groups failures from arming, disarming, or inspecting monotonic timers.',
  NumericConversionErrors: 'Groups checked numeric conversion failures where the destination type cannot represent the source value.',
  ParseErrors: 'Groups text parsing failures for primitive numeric parsers.',
  ProcessControlAckErrors: 'Groups failures from acknowledging a process suspend generation.',
  ProcessControlErrors: 'Groups failures from opening, polling, acknowledging, and resuming process-control notifications.',
  ProcessControlOpenErrors: 'Groups failures from creating a process-control source.',
  ProcessControlPollErrors: 'Groups failures from polling process-control notifications.',
  ProcessControlResumeErrors: 'Groups failures from recording application resume for a process-control generation.',
  ProcessCurrentDirectoryErrors: 'Groups failures from reading the process current working directory.',
  ProcessEnvErrors: 'Groups failures from reading process environment variables, including missing variables, invalid names, and invalid UTF-8 values.',
  ProcessErrors: 'Groups process path and environment failures for application boundary code.',
  ProcessExecutablePathErrors: 'Groups failures from reading the running executable path.',
  ProcessPathErrors: 'Groups failures from reading or changing process-related filesystem paths.',
  ProcessSetDirectoryErrors: 'Groups failures from changing the process current working directory.',
  RenderErrors: 'Groups text and terminal-output failures that can occur while rendering to standard output.',
  RenderSetupErrors: 'Groups failures from acquiring output handles or terminal capabilities before rendering.',
  SleepErrors: 'Groups failures from sleeping for an invalid duration.',
  StandardInputErrors: 'Groups failures from reading standard input.',
  StandardOutputAcquireErrors: 'Groups failures from acquiring standard-output handles.',
  StandardOutputCapabilityErrors: 'Groups failures from inspecting standard-output terminal capabilities.',
  StdoutFlushErrors: 'Groups failures from flushing standard output.',
  StdoutWriteErrors: 'Groups failures from writing text to standard output.',
  StdoutWriterErrors: 'Groups failures from writing and flushing with `StdoutWriter`.',
  StringBuilderErrors: 'Groups failures from incremental string-builder operations.',
  StringJoinErrors: 'Groups allocation failures from joining strings.',
  StringMutationErrors: 'Groups failures from string replacement or mutation-style helpers that validate ranges and allocate replacement text.',
  StringProcessingErrors: 'Groups broad string search, range, builder, allocation, and terminal text-layout failures for text-processing boundaries.',
  StringRangeErrors: 'Groups failures from invalid string ranges or counts.',
  StringSearchErrors: 'Groups failures from string search operations that require nonempty, present patterns.',
  StringSliceErrors: 'Groups failures from string slicing helpers that validate counts, ranges, order, and allocation.',
  SystemWaitRegistrationErrors: 'Groups failures from creating or retargeting wait-set registrations.',
  SystemWaitSetErrors: 'Groups failures from using wait-set registrations or registration authorities.',
  TerminalChordMutationErrors: 'Groups failures from unregistering or replacing terminal chord bindings.',
  TerminalChordProcessErrors: 'Groups failures from processing terminal input through a chord router.',
  TerminalChordReleasedInputErrors: 'Groups failures from indexing input released by a chord router.',
  TerminalChordReplaceErrors: 'Groups validation, mutation, and allocation failures from replacing a chord binding atomically.',
  TerminalChordRouterErrors: 'Groups all chord-router construction, mutation, processing, indexing, constraint, and allocation failures.',
  TerminalChordValidationErrors: 'Groups failures from constructing or registering terminal chord sequences.',
  TerminalControlErrors: 'Groups legacy terminal-control output failures such as clear-screen and cursor movement.',
  TerminalDrawErrors: 'Groups failures from drawing rows or writing terminal display output.',
  TerminalSessionCursorErrors: 'Groups terminal-session cursor movement failures, including invalid positions and session write/state failures.',
  TerminalSessionErrors: 'Groups broad terminal-session lifecycle, input, output, state, restoration, cursor, and allocation failures.',
  TerminalSessionLifecycleErrors: 'Groups failures from validating, opening, resuming, restoring, and closing terminal sessions.',
  TerminalSessionOpenErrors: 'Groups failures from validating options and opening terminal sessions.',
  TerminalSessionPauseResumeErrors: 'Groups failures from pausing, resuming, and restoring terminal sessions.',
  TerminalSessionReadErrors: 'Groups failures from reading terminal session input or querying read-side state.',
  TerminalSessionRenderErrors: 'Groups failures from rendering through an owned terminal session.',
  TerminalSessionRestoreErrors: 'Groups failures from terminal restoration and recovery-token handling.',
  TerminalSessionWriteErrors: 'Groups failures from writing or flushing through an owned terminal session.',
  TerminalTestingErrors: 'Groups failures from test-only terminal factories and constrained synthetic values.',
  TerminalTextLayoutErrors: 'Groups failures from terminal text layout and allocation.',
  TimeErrors: 'Groups sleep and frame-clock failures from ordinary time helpers.'
};

const COMPATIBILITY_ERROR_SET_ALIASES: Readonly<Record<string, string>> = {
  BytesError: 'BytesErrors',
  FilesystemCopyMoveError: 'FilesystemCopyMoveErrors',
  FilesystemCreateError: 'FilesystemCreateErrors',
  FilesystemDeleteError: 'FilesystemDeleteErrors',
  FilesystemDirectoryDeleteError: 'FilesystemDirectoryDeleteErrors',
  FilesystemError: 'FilesystemErrors',
  FilesystemListError: 'FilesystemListErrors',
  FilesystemMetadataError: 'FilesystemMetadataErrors',
  FilesystemPathError: 'FilesystemPathErrors',
  FilesystemReadError: 'FilesystemReadErrors',
  FilesystemWriteError: 'FilesystemWriteErrors',
  IndexAccessError: 'IndexAccessErrors',
  IntegerRangeError: 'NumericConversionErrors',
  OutputError: 'StdoutWriterErrors',
  ParseError: 'ParseErrors',
  ProcessEnvError: 'ProcessEnvErrors',
  ProcessPathError: 'ProcessPathErrors',
  StandardInputReadError: 'StandardInputErrors',
  StandardOutputCapabilityError: 'StandardOutputCapabilityErrors',
  StandardOutputHandleError: 'StandardOutputAcquireErrors',
  StringBuilderError: 'StringBuilderErrors',
  StringRangeError: 'StringRangeErrors',
  StringSearchError: 'StringSearchErrors',
  TerminalError: 'TerminalControlErrors',
  TimeError: 'TimeErrors'
};

/**
 * Looks up documentation for a standard-library error leaf.
 * @param name Error leaf name.
 * @returns Purpose-oriented documentation, or undefined for non-error names.
 */
export function stdlibErrorDocumentation(name: string): string | undefined {
  return ERROR_LEAF_DOCUMENTATION[name];
}

/**
 * Looks up documentation for standard.system nominal types, including its error leaves.
 * @param name System prerequisite type or error name.
 * @returns Purpose-oriented documentation, or undefined for unknown names.
 */
export function stdlibSystemTypeDocumentation(name: string): string | undefined {
  return SYSTEM_TYPE_DOCUMENTATION[name] ?? stdlibErrorDocumentation(name);
}

/**
 * Builds documentation for a named standard.errors error set.
 * @param name Error-set name.
 * @param members Expanded member error leaves.
 * @returns Purpose, usage guidance, and member summaries.
 */
export function stdlibErrorSetDocumentation(name: string, members: readonly string[]): string {
  const aliasTarget = COMPATIBILITY_ERROR_SET_ALIASES[name];
  const purpose = aliasTarget
    ? `Compatibility alias for \`${aliasTarget}\`. ${ERROR_SET_PURPOSES[aliasTarget] ?? fallbackErrorSetPurpose(aliasTarget)}`
    : ERROR_SET_PURPOSES[name] ?? fallbackErrorSetPurpose(name);
  const memberSummaries = members.map((member) => `- \`${member}\`: ${memberSummary(member)}`).join('\n');
  return `${purpose}\n\nUse this named error set in an \`errors\` clause when callers should be able to handle or propagate the whole category together.\n\nMembers:\n${memberSummaries}`;
}

/**
 * Provides a safe fallback for future named error sets until explicit prose is added.
 * @param name Error-set name.
 * @returns Fallback purpose text.
 */
function fallbackErrorSetPurpose(name: string): string {
  return `Groups related standard-library failures represented by \`${name}\`.`;
}

/**
 * Summarizes one member for an error-set documentation block.
 * @param member Error leaf name.
 * @returns Single-sentence member summary.
 */
function memberSummary(member: string): string {
  const documentation = stdlibErrorDocumentation(member);
  if (!documentation) {
    return 'Standard-library error family member used by this named error set.';
  }
  return firstSentence(documentation);
}

/**
 * Extracts the first sentence from documentation prose.
 * @param text Full documentation.
 * @returns First sentence, or the original text when no sentence boundary is found.
 */
function firstSentence(text: string): string {
  const match = /.*?\.(?:\s|$)/.exec(text);
  return match ? match[0].trim() : text;
}
