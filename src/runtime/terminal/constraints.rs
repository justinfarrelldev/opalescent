//! Selected terminal constrained value types and exact boundary checks.

extern crate alloc;

use alloc::string::String;
use core::fmt;

/// Structured error returned when a terminal constrained value rejects an input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerminalConstraintError {
    /// Terminal constrained type name.
    pub type_name: &'static str,
    /// Human-readable rejection reason.
    pub reason: String,
}

impl TerminalConstraintError {
    /// Build a new structured constraint rejection.
    #[must_use]
    pub fn new(type_name: &'static str, reason: impl Into<String>) -> Self {
        Self {
            type_name,
            reason: reason.into(),
        }
    }
}

impl fmt::Display for TerminalConstraintError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} rejected value: {}", self.type_name, self.reason)
    }
}

impl std::error::Error for TerminalConstraintError {}

/// Declare one exact-bounds terminal constrained integer wrapper.
macro_rules! constrained_int {
    ($name:ident, $inner:ty, $min:expr, $max:expr) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name($inner);

        impl $name {
            #[must_use]
            pub const fn get(self) -> $inner {
                self.0
            }

            pub fn new(value: $inner) -> Result<Self, TerminalConstraintError> {
                if !($min..=$max).contains(&value) {
                    return Err(TerminalConstraintError::new(
                        stringify!($name),
                        alloc::format!("expected {}..={}, found {value}", $min, $max),
                    ));
                }
                Ok(Self(value))
            }
        }
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TerminalControlCode(u8);

impl TerminalControlCode {
    #[must_use]
    pub const fn get(self) -> u8 {
        self.0
    }

    pub fn new(value: u8) -> Result<Self, TerminalConstraintError> {
        if value <= 31 || value == 127 {
            return Ok(Self(value));
        }
        Err(TerminalConstraintError::new(
            "TerminalControlCode",
            alloc::format!("expected C0 control or DEL, found {value}"),
        ))
    }
}

constrained_int!(TerminalFunctionKeyNumber, i32, 1_i32, 0x7FFF_i32);
constrained_int!(TerminalColumnCount, i32, 1_i32, i32::MAX);
constrained_int!(TerminalRowCount, i32, 1_i32, i32::MAX);
constrained_int!(TerminalColumnIndex, i32, 0_i32, i32::MAX);
constrained_int!(TerminalRowIndex, i32, 0_i32, i32::MAX);
constrained_int!(TerminalKeyRepeatCount, i32, 1_i32, i32::MAX);
constrained_int!(TerminalWaitMilliseconds, i32, 1_i32, i32::MAX);
constrained_int!(
    TerminalInputSequenceTimeoutMilliseconds,
    i32,
    1_i32,
    60_000_i32
);
constrained_int!(TerminalCommittedTextByteLimit, i32, 4_i32, 0x0010_0000_i32);
constrained_int!(
    TerminalCompositionPreeditByteLimit,
    i32,
    4_i32,
    0x0010_0000_i32
);
constrained_int!(TerminalPasteChunkByteLimit, i32, 4_i32, 0x0100_0000_i32);
constrained_int!(TerminalUnknownByteChunkLimit, i32, 1_i32, 0x0010_0000_i32);
constrained_int!(
    TerminalPendingSequenceByteLimit,
    i32,
    4_i32,
    0x0010_0000_i32
);
constrained_int!(TerminalRetainedEventLimit, i32, 8_i32, 0x0010_0000_i32);
constrained_int!(TerminalRetainedByteLimit, i32, 4_096_i32, 0x4000_0000_i32);
constrained_int!(TerminalCorrelatedEventLimit, i32, 2_i32, 0x0001_0000_i32);
constrained_int!(TerminalCorrelatedByteLimit, i32, 64_i32, 0x0100_0000_i32);
constrained_int!(TerminalDiagnosticCountLimit, i32, 1_i32, 256_i32);
constrained_int!(
    TerminalDiagnosticCollectionByteLimit,
    i32,
    256_i32,
    0x0010_0000_i32
);
constrained_int!(TerminalColorCount, i32, 1_i32, 0x0100_0000_i32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TerminalEventId(u64);

impl TerminalEventId {
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }

    /// Construct a runtime-issued nonzero terminal event identifier.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "production terminal backends construct runtime-issued event identifiers later"
        )
    )]
    pub(crate) fn new_runtime(value: u64) -> Result<Self, TerminalConstraintError> {
        if value == 0 {
            return Err(TerminalConstraintError::new(
                "TerminalEventId",
                "expected value >= 1",
            ));
        }
        Ok(Self(value))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TerminalCompositionId(u64);

impl TerminalCompositionId {
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }

    /// Construct a runtime-issued nonzero composition identifier.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "production terminal backends construct runtime-issued composition identifiers later"
        )
    )]
    pub(crate) fn new_runtime(value: u64) -> Result<Self, TerminalConstraintError> {
        if value == 0 {
            return Err(TerminalConstraintError::new(
                "TerminalCompositionId",
                "expected value >= 1",
            ));
        }
        Ok(Self(value))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TerminalCompositionScalarIndex(i64);

impl TerminalCompositionScalarIndex {
    #[must_use]
    pub const fn get(self) -> i64 {
        self.0
    }

    /// Construct a composition cursor index validated against a preedit scalar count.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "production composition updates validate scalar cursor positions later"
        )
    )]
    pub(crate) fn new_runtime(
        value: i64,
        scalar_count: usize,
    ) -> Result<Self, TerminalConstraintError> {
        if value < 0 {
            return Err(TerminalConstraintError::new(
                "TerminalCompositionScalarIndex",
                alloc::format!("expected value >= 0, found {value}"),
            ));
        }
        let index = usize::try_from(value).map_err(|_conversion_error| {
            TerminalConstraintError::new(
                "TerminalCompositionScalarIndex",
                alloc::format!("value {value} does not fit usize"),
            )
        })?;
        if index > scalar_count {
            return Err(TerminalConstraintError::new(
                "TerminalCompositionScalarIndex",
                alloc::format!(
                    "expected scalar index <= scalar count {scalar_count}, found {index}"
                ),
            ));
        }
        Ok(Self(value))
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TerminalNativeEventName(String);

impl fmt::Debug for TerminalNativeEventName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("TerminalNativeEventName")
            .field(&self.0)
            .finish()
    }
}

impl TerminalNativeEventName {
    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    /// Construct a bounded native event name for opaque backend records.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "production backends validate native event names later"
        )
    )]
    pub(crate) fn new_runtime(value: impl Into<String>) -> Result<Self, TerminalConstraintError> {
        let value = value.into();
        require_nonempty_nul_free_max_bytes("TerminalNativeEventName", &value, 256)?;
        Ok(Self(value))
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TerminalDiagnosticDetail(String);

impl fmt::Debug for TerminalDiagnosticDetail {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("TerminalDiagnosticDetail")
            .field(&self.0)
            .finish()
    }
}

impl TerminalDiagnosticDetail {
    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    /// Construct bounded diagnostic detail text without NUL bytes.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "production diagnostics validate bounded detail later"
        )
    )]
    pub(crate) fn new_runtime(value: impl Into<String>) -> Result<Self, TerminalConstraintError> {
        let value = value.into();
        require_nul_free_max_bytes("TerminalDiagnosticDetail", &value, 4_096)?;
        Ok(Self(value))
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TerminalCommittedText(String);

impl fmt::Debug for TerminalCommittedText {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("TerminalCommittedText")
            .field(&self.0)
            .finish()
    }
}

impl TerminalCommittedText {
    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    /// Construct committed text emitted by the runtime decoder.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "production input decoding validates committed text later"
        )
    )]
    pub(crate) fn new_runtime(value: impl Into<String>) -> Result<Self, TerminalConstraintError> {
        let value = value.into();
        require_nonempty_nul_free("TerminalCommittedText", &value)?;
        Ok(Self(value))
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TerminalPasteText(String);

impl fmt::Debug for TerminalPasteText {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("TerminalPasteText").field(&self.0).finish()
    }
}

impl TerminalPasteText {
    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    #[cfg(test)]
    pub(crate) fn new_runtime(value: impl Into<String>) -> Result<Self, TerminalConstraintError> {
        let value = value.into();
        require_nonempty_nul_free("TerminalPasteText", &value)?;
        Ok(Self(value))
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TerminalCompositionPreeditText(String);

impl fmt::Debug for TerminalCompositionPreeditText {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("TerminalCompositionPreeditText")
            .field(&self.0)
            .finish()
    }
}

impl TerminalCompositionPreeditText {
    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    #[cfg(test)]
    pub(crate) fn new_runtime(value: impl Into<String>) -> Result<Self, TerminalConstraintError> {
        let value = value.into();
        require_nul_free("TerminalCompositionPreeditText", &value)?;
        Ok(Self(value))
    }
}

/// Require nonempty text with no NUL byte.
fn require_nonempty_nul_free(
    type_name: &'static str,
    value: &str,
) -> Result<(), TerminalConstraintError> {
    if value.is_empty() {
        return Err(TerminalConstraintError::new(
            type_name,
            "expected nonempty UTF-8 text",
        ));
    }
    require_nul_free(type_name, value)
}

/// Require nonempty text with no NUL byte and bounded UTF-8 byte length.
fn require_nonempty_nul_free_max_bytes(
    type_name: &'static str,
    value: &str,
    max_bytes: usize,
) -> Result<(), TerminalConstraintError> {
    require_nonempty_nul_free(type_name, value)?;
    require_max_bytes(type_name, value, max_bytes)
}

/// Require text with no NUL byte.
fn require_nul_free(type_name: &'static str, value: &str) -> Result<(), TerminalConstraintError> {
    if value.chars().any(|character| character == '\0') {
        return Err(TerminalConstraintError::new(
            type_name,
            "NUL byte is not allowed",
        ));
    }
    Ok(())
}

/// Require text with no NUL byte and bounded UTF-8 byte length.
fn require_nul_free_max_bytes(
    type_name: &'static str,
    value: &str,
    max_bytes: usize,
) -> Result<(), TerminalConstraintError> {
    require_nul_free(type_name, value)?;
    require_max_bytes(type_name, value, max_bytes)
}

/// Require a maximum UTF-8 byte length.
fn require_max_bytes(
    type_name: &'static str,
    value: &str,
    max_bytes: usize,
) -> Result<(), TerminalConstraintError> {
    let utf8_bytes = value.len();
    if utf8_bytes > max_bytes {
        return Err(TerminalConstraintError::new(
            type_name,
            alloc::format!("expected UTF-8 byte length <= {max_bytes}, found {utf8_bytes}"),
        ));
    }
    Ok(())
}
