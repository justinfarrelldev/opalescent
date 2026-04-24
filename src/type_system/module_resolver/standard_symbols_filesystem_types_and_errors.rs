extern crate alloc;

use crate::type_system::symbol_table::SymbolType;
use crate::type_system::types::CoreType;
use alloc::{string::String, vec::Vec};

/// Macro-wrapped symbol literal to keep the provider function concise for clippy.
macro_rules! standard_symbols_filesystem_types_and_errors_vec {
    () => {
        vec![
            (
                String::from("is_directory_sync"),
                CoreType::Function {
                    generic_params: Vec::new(),
                    parameters: vec![CoreType::Generic {
                        name: String::from("FilesystemPath"),
                        type_args: Vec::new(),
                    }],
                    return_types: vec![CoreType::Boolean],
                    error_types: vec![
                        CoreType::Generic {
                            name: String::from("PermissionDeniedError"),
                            type_args: Vec::new(),
                        },
                        CoreType::Generic {
                            name: String::from("InvalidPathError"),
                            type_args: Vec::new(),
                        },
                    ],
                },
                SymbolType::Function,
            ),
            (
                String::from("is_directory_nofollow_sync"),
                CoreType::Function {
                    generic_params: Vec::new(),
                    parameters: vec![CoreType::Generic {
                        name: String::from("FilesystemPath"),
                        type_args: Vec::new(),
                    }],
                    return_types: vec![CoreType::Boolean],
                    error_types: vec![
                        CoreType::Generic {
                            name: String::from("PermissionDeniedError"),
                            type_args: Vec::new(),
                        },
                        CoreType::Generic {
                            name: String::from("InvalidPathError"),
                            type_args: Vec::new(),
                        },
                    ],
                },
                SymbolType::Function,
            ),
            // `FilesystemPath`, `FileMetadata`, `FilePermissions` nominal types
            // and the twenty filesystem error nominal types.  These are tag-only
            // or field-bearing product types registered by `fs_builtins.rs`.
            (
                String::from("FilesystemPath"),
                CoreType::Generic {
                    name: String::from("FilesystemPath"),
                    type_args: Vec::new(),
                },
                SymbolType::Type,
            ),
            (
                String::from("FileMetadata"),
                CoreType::Generic {
                    name: String::from("FileMetadata"),
                    type_args: Vec::new(),
                },
                SymbolType::Type,
            ),
            (
                String::from("FilePermissions"),
                CoreType::Generic {
                    name: String::from("FilePermissions"),
                    type_args: Vec::new(),
                },
                SymbolType::Type,
            ),
            (
                String::from("FileNotFoundError"),
                CoreType::Generic {
                    name: String::from("FileNotFoundError"),
                    type_args: Vec::new(),
                },
                SymbolType::Type,
            ),
            (
                String::from("PermissionDeniedError"),
                CoreType::Generic {
                    name: String::from("PermissionDeniedError"),
                    type_args: Vec::new(),
                },
                SymbolType::Type,
            ),
            (
                String::from("FileAlreadyExistsError"),
                CoreType::Generic {
                    name: String::from("FileAlreadyExistsError"),
                    type_args: Vec::new(),
                },
                SymbolType::Type,
            ),
            (
                String::from("ReadFailureError"),
                CoreType::Generic {
                    name: String::from("ReadFailureError"),
                    type_args: Vec::new(),
                },
                SymbolType::Type,
            ),
            (
                String::from("WriteFailureError"),
                CoreType::Generic {
                    name: String::from("WriteFailureError"),
                    type_args: Vec::new(),
                },
                SymbolType::Type,
            ),
            (
                String::from("InvalidPathError"),
                CoreType::Generic {
                    name: String::from("InvalidPathError"),
                    type_args: Vec::new(),
                },
                SymbolType::Type,
            ),
            (
                String::from("FilesystemFullError"),
                CoreType::Generic {
                    name: String::from("FilesystemFullError"),
                    type_args: Vec::new(),
                },
                SymbolType::Type,
            ),
            (
                String::from("IsADirectoryError"),
                CoreType::Generic {
                    name: String::from("IsADirectoryError"),
                    type_args: Vec::new(),
                },
                SymbolType::Type,
            ),
            (
                String::from("IsNotADirectoryError"),
                CoreType::Generic {
                    name: String::from("IsNotADirectoryError"),
                    type_args: Vec::new(),
                },
                SymbolType::Type,
            ),
            (
                String::from("DirectoryNotEmptyError"),
                CoreType::Generic {
                    name: String::from("DirectoryNotEmptyError"),
                    type_args: Vec::new(),
                },
                SymbolType::Type,
            ),
            (
                String::from("DirectoryNotFoundError"),
                CoreType::Generic {
                    name: String::from("DirectoryNotFoundError"),
                    type_args: Vec::new(),
                },
                SymbolType::Type,
            ),
            (
                String::from("MetadataUnavailableError"),
                CoreType::Generic {
                    name: String::from("MetadataUnavailableError"),
                    type_args: Vec::new(),
                },
                SymbolType::Type,
            ),
            (
                String::from("OffsetOutOfRangeError"),
                CoreType::Generic {
                    name: String::from("OffsetOutOfRangeError"),
                    type_args: Vec::new(),
                },
                SymbolType::Type,
            ),
            (
                String::from("LineOutOfRangeError"),
                CoreType::Generic {
                    name: String::from("LineOutOfRangeError"),
                    type_args: Vec::new(),
                },
                SymbolType::Type,
            ),
            (
                String::from("CopyFailureError"),
                CoreType::Generic {
                    name: String::from("CopyFailureError"),
                    type_args: Vec::new(),
                },
                SymbolType::Type,
            ),
            (
                String::from("MoveFailureError"),
                CoreType::Generic {
                    name: String::from("MoveFailureError"),
                    type_args: Vec::new(),
                },
                SymbolType::Type,
            ),
            (
                String::from("DeleteFailureError"),
                CoreType::Generic {
                    name: String::from("DeleteFailureError"),
                    type_args: Vec::new(),
                },
                SymbolType::Type,
            ),
            (
                String::from("CreateFailureError"),
                CoreType::Generic {
                    name: String::from("CreateFailureError"),
                    type_args: Vec::new(),
                },
                SymbolType::Type,
            ),
            (
                String::from("SetPermissionsError"),
                CoreType::Generic {
                    name: String::from("SetPermissionsError"),
                    type_args: Vec::new(),
                },
                SymbolType::Type,
            ),
            (
                String::from("InvalidUtf8Error"),
                CoreType::Generic {
                    name: String::from("InvalidUtf8Error"),
                    type_args: Vec::new(),
                },
                SymbolType::Type,
            ),
        ]
    };
}

/// Filesystem type and error nominal declarations plus directory predicates.
pub(super) fn standard_symbols_filesystem_types_and_errors() -> Vec<(String, CoreType, SymbolType)>
{
    standard_symbols_filesystem_types_and_errors_vec!()
}
