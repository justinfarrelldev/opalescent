//! Filesystem stdlib built-in registration.
//!
//! Registers the language-level `FilesystemPath`, `FileMetadata`, and
//! `FilePermissions` nominal types together with the twenty filesystem error
//! nominal types that the path-object-centric file-I/O surface exposes.
//!
//! The builtins are grouped here (rather than expanding
//! `register_standard_builtins` in [`super`]) so that the parent module stays
//! under the project's 500-line guideline while still exposing a single
//! authoritative registration site.

extern crate alloc;

use crate::type_system::checker::TypeChecker;
use crate::type_system::types::CoreType;
use alloc::borrow::ToOwned;

/// Opalescent-level name of the path nominal type.
const FILESYSTEM_PATH_TYPE_NAME: &str = "FilesystemPath";

/// Opalescent-level name of the file-metadata nominal type.
const FILE_METADATA_TYPE_NAME: &str = "FileMetadata";

/// Opalescent-level name of the file-permissions nominal type.
const FILE_PERMISSIONS_TYPE_NAME: &str = "FilePermissions";

/// All twenty filesystem error nominal type names.
const FS_ERROR_NAMES: &[&str] = &[
    "FileNotFoundError",
    "PermissionDeniedError",
    "FileAlreadyExistsError",
    "ReadFailureError",
    "WriteFailureError",
    "InvalidPathError",
    "FilesystemFullError",
    "IsADirectoryError",
    "IsNotADirectoryError",
    "DirectoryNotEmptyError",
    "DirectoryNotFoundError",
    "MetadataUnavailableError",
    "OffsetOutOfRangeError",
    "LineOutOfRangeError",
    "CopyFailureError",
    "MoveFailureError",
    "DeleteFailureError",
    "CreateFailureError",
    "SetPermissionsError",
    "InvalidUtf8Error",
];

impl TypeChecker {
    /// Register all filesystem nominal types and error types.
    ///
    /// Call this exactly once from
    /// [`TypeChecker::register_standard_builtins`](super::TypeChecker::register_standard_builtins).
    pub(super) fn register_fs_builtins(&mut self) {
        self.register_fs_nominal_types();
        self.register_fs_error_types();
    }

    /// Register `FilesystemPath`, `FileMetadata`, and `FilePermissions`.
    fn register_fs_nominal_types(&mut self) {
        self.environment.register_type(
            FILESYSTEM_PATH_TYPE_NAME.to_owned(),
            nominal_type(FILESYSTEM_PATH_TYPE_NAME),
        );
        self.environment.register_type(
            FILE_METADATA_TYPE_NAME.to_owned(),
            nominal_type(FILE_METADATA_TYPE_NAME),
        );
        self.environment.register_type(
            FILE_PERMISSIONS_TYPE_NAME.to_owned(),
            nominal_type(FILE_PERMISSIONS_TYPE_NAME),
        );
    }

    /// Register all twenty filesystem error nominal types.
    fn register_fs_error_types(&mut self) {
        for name in FS_ERROR_NAMES {
            self.environment
                .register_type((*name).to_owned(), nominal_type(name));
        }
    }
}

/// Construct a tag-only nominal [`CoreType`] with no type arguments.
fn nominal_type(name: &str) -> CoreType {
    CoreType::Generic {
        name: name.to_owned(),
        type_args: alloc::vec![],
    }
}
