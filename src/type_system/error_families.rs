//! Declaration-only families for standard-library error leaf types.

use super::environment::TypeEnvironment;
use super::types::CoreType;

/// A named collection of stdlib error leaves available in an `errors` declaration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StdlibErrorFamily {
    /// Declaration-only family name.
    pub name: &'static str,
    /// Ordered leaf names covered by the family.
    pub members: &'static [&'static str],
    /// Whether a complete leaf declaration may be shortened to this family.
    pub warning_eligible: bool,
    /// Lower ranks are preferred when later warning selection has overlapping families.
    pub specificity_rank: usize,
}

const STDLIB_ERROR_FAMILIES: &[StdlibErrorFamily] = &[
    StdlibErrorFamily {
        name: "ParseError",
        members: &["ParseError"],
        warning_eligible: false,
        specificity_rank: 0,
    },
    StdlibErrorFamily {
        name: "BytesError",
        members: &["HexDecodeError", "SliceRangeError"],
        warning_eligible: true,
        specificity_rank: 0,
    },
    StdlibErrorFamily {
        name: "StringSearchError",
        members: &["StringEmptySearchTextError", "StringPatternNotFoundError"],
        warning_eligible: true,
        specificity_rank: 0,
    },
    StdlibErrorFamily {
        name: "StringRangeError",
        members: &[
            "StringNegativeCountError",
            "StringRangeOutOfBoundsError",
            "StringRangeOrderError",
        ],
        warning_eligible: true,
        specificity_rank: 0,
    },
    StdlibErrorFamily {
        name: "StringBuilderError",
        members: &["BuilderFinishedError", "AllocationFailureError"],
        warning_eligible: true,
        specificity_rank: 0,
    },
    StdlibErrorFamily {
        name: "OutputError",
        members: &["WriteFailureError", "FlushFailureError", "SinkClosedError"],
        warning_eligible: true,
        specificity_rank: 0,
    },
    StdlibErrorFamily {
        name: "StandardInputReadError",
        members: &["StandardInputReadError"],
        warning_eligible: false,
        specificity_rank: 0,
    },
    StdlibErrorFamily {
        name: "StandardOutputHandleError",
        members: &["StandardOutputHandleError"],
        warning_eligible: false,
        specificity_rank: 0,
    },
    StdlibErrorFamily {
        name: "StandardOutputCapabilityError",
        members: &["StandardOutputCapabilityError"],
        warning_eligible: false,
        specificity_rank: 0,
    },
    StdlibErrorFamily {
        name: "TerminalError",
        members: &[
            "TerminalWriteFailureError",
            "InvalidCursorPositionError",
            "SinkClosedError",
        ],
        warning_eligible: true,
        specificity_rank: 0,
    },
    StdlibErrorFamily {
        name: "TimeError",
        members: &["InvalidDurationError", "InvalidFrameRateError"],
        warning_eligible: true,
        specificity_rank: 0,
    },
    StdlibErrorFamily {
        name: "ProcessPathError",
        members: &[
            "PermissionDeniedError",
            "InvalidPathError",
            "CurrentWorkingDirectoryUnavailableError",
            "CurrentExecutablePathUnavailableError",
            "FileNotFoundError",
            "IsNotADirectoryError",
        ],
        warning_eligible: true,
        specificity_rank: 0,
    },
    StdlibErrorFamily {
        name: "ProcessEnvError",
        members: &[
            "EnvironmentVariableNotFoundError",
            "InvalidEnvironmentVariableNameError",
            "InvalidUtf8Error",
        ],
        warning_eligible: true,
        specificity_rank: 0,
    },
    StdlibErrorFamily {
        name: "FilesystemPathError",
        members: &["InvalidPathError", "PermissionDeniedError"],
        warning_eligible: true,
        specificity_rank: 0,
    },
    StdlibErrorFamily {
        name: "FilesystemReadError",
        members: &[
            "FileNotFoundError",
            "PermissionDeniedError",
            "ReadFailureError",
            "IsADirectoryError",
            "InvalidPathError",
            "InvalidUtf8Error",
            "OffsetOutOfRangeError",
        ],
        warning_eligible: true,
        specificity_rank: 0,
    },
    StdlibErrorFamily {
        name: "FilesystemWriteError",
        members: &[
            "FileNotFoundError",
            "PermissionDeniedError",
            "WriteFailureError",
            "IsADirectoryError",
            "InvalidPathError",
            "FilesystemFullError",
            "OffsetOutOfRangeError",
        ],
        warning_eligible: true,
        specificity_rank: 0,
    },
    StdlibErrorFamily {
        name: "FilesystemCreateError",
        members: &[
            "FileAlreadyExistsError",
            "PermissionDeniedError",
            "CreateFailureError",
            "InvalidPathError",
            "FilesystemFullError",
        ],
        warning_eligible: true,
        specificity_rank: 0,
    },
    StdlibErrorFamily {
        name: "FilesystemDeleteError",
        members: &[
            "FileNotFoundError",
            "PermissionDeniedError",
            "DeleteFailureError",
            "IsADirectoryError",
            "InvalidPathError",
        ],
        warning_eligible: true,
        specificity_rank: 0,
    },
    StdlibErrorFamily {
        name: "FilesystemDirectoryDeleteError",
        members: &[
            "DirectoryNotFoundError",
            "PermissionDeniedError",
            "DeleteFailureError",
            "DirectoryNotEmptyError",
            "IsNotADirectoryError",
            "InvalidPathError",
        ],
        warning_eligible: true,
        specificity_rank: 0,
    },
    StdlibErrorFamily {
        name: "FilesystemCopyMoveError",
        members: &[
            "FileNotFoundError",
            "PermissionDeniedError",
            "CopyFailureError",
            "MoveFailureError",
            "IsADirectoryError",
            "FileAlreadyExistsError",
            "InvalidPathError",
            "FilesystemFullError",
        ],
        warning_eligible: true,
        specificity_rank: 0,
    },
    StdlibErrorFamily {
        name: "FilesystemMetadataError",
        members: &[
            "FileNotFoundError",
            "PermissionDeniedError",
            "MetadataUnavailableError",
            "InvalidPathError",
        ],
        warning_eligible: true,
        specificity_rank: 0,
    },
    StdlibErrorFamily {
        name: "FilesystemListError",
        members: &[
            "DirectoryNotFoundError",
            "PermissionDeniedError",
            "ReadFailureError",
            "IsNotADirectoryError",
            "InvalidPathError",
        ],
        warning_eligible: true,
        specificity_rank: 0,
    },
    StdlibErrorFamily {
        name: "FilesystemError",
        members: &[
            "FileNotFoundError",
            "PermissionDeniedError",
            "ReadFailureError",
            "IsADirectoryError",
            "InvalidPathError",
            "InvalidUtf8Error",
            "OffsetOutOfRangeError",
            "WriteFailureError",
            "FilesystemFullError",
            "FileAlreadyExistsError",
            "CreateFailureError",
            "DeleteFailureError",
            "DirectoryNotFoundError",
            "DirectoryNotEmptyError",
            "IsNotADirectoryError",
            "CopyFailureError",
            "MoveFailureError",
            "MetadataUnavailableError",
            "LineOutOfRangeError",
            "SetPermissionsError",
        ],
        warning_eligible: false,
        specificity_rank: 1,
    },
    StdlibErrorFamily {
        name: "IndexAccessError",
        members: &["IndexOutOfBoundsError"],
        warning_eligible: false,
        specificity_rank: 0,
    },
];

/// Return stdlib error families in taxonomy order.
pub const fn stdlib_error_families() -> &'static [StdlibErrorFamily] {
    STDLIB_ERROR_FAMILIES
}

/// Look up a declaration-only stdlib error family by name.
pub fn stdlib_error_family(name: &str) -> Option<&'static StdlibErrorFamily> {
    STDLIB_ERROR_FAMILIES
        .iter()
        .find(|family| family.name == name)
}

/// Return whether an emitted leaf error is covered by a declared leaf or family.
pub fn error_type_is_covered_by_declared_type(emitted_leaf: &str, declared_type: &str) -> bool {
    emitted_leaf == declared_type
        || stdlib_error_family(declared_type)
            .is_some_and(|family| family.members.contains(&emitted_leaf))
}

/// Construct the payload-free nominal type used by every stdlib error leaf and family.
pub fn stdlib_error_core_type(name: &str) -> CoreType {
    CoreType::Generic {
        name: name.to_owned(),
        type_args: Vec::new(),
    }
}

/// Register declaration-only stdlib family names without changing precise leaf signatures.
pub fn register_stdlib_error_family_types(environment: &mut TypeEnvironment) {
    for family in STDLIB_ERROR_FAMILIES {
        environment.register_type(family.name.to_owned(), stdlib_error_core_type(family.name));
    }
}
