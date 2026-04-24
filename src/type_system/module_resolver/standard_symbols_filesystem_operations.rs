extern crate alloc;

use crate::type_system::symbol_table::SymbolType;
use crate::type_system::types::CoreType;
use alloc::{string::String, vec::Vec};

/// Macro-wrapped symbol literal to keep the provider function concise for clippy.
macro_rules! standard_symbols_filesystem_operations_vec {
    () => {
        vec![
            (
                String::from("append_contents_sync"),
                CoreType::Function {
                    generic_params: Vec::new(),
                    parameters: vec![
                        CoreType::Generic {
                            name: String::from("FilesystemPath"),
                            type_args: Vec::new(),
                        },
                        CoreType::Generic {
                            name: String::from("Bytes"),
                            type_args: Vec::new(),
                        },
                    ],
                    return_types: vec![CoreType::Unit],
                    error_types: vec![
                        CoreType::Generic {
                            name: String::from("FileNotFoundError"),
                            type_args: Vec::new(),
                        },
                        CoreType::Generic {
                            name: String::from("PermissionDeniedError"),
                            type_args: Vec::new(),
                        },
                        CoreType::Generic {
                            name: String::from("WriteFailureError"),
                            type_args: Vec::new(),
                        },
                        CoreType::Generic {
                            name: String::from("IsADirectoryError"),
                            type_args: Vec::new(),
                        },
                        CoreType::Generic {
                            name: String::from("InvalidPathError"),
                            type_args: Vec::new(),
                        },
                        CoreType::Generic {
                            name: String::from("FilesystemFullError"),
                            type_args: Vec::new(),
                        },
                    ],
                },
                SymbolType::Function,
            ),
            (
                String::from("append_text_sync"),
                CoreType::Function {
                    generic_params: Vec::new(),
                    parameters: vec![
                        CoreType::Generic {
                            name: String::from("FilesystemPath"),
                            type_args: Vec::new(),
                        },
                        CoreType::String,
                    ],
                    return_types: vec![CoreType::Unit],
                    error_types: vec![
                        CoreType::Generic {
                            name: String::from("FileNotFoundError"),
                            type_args: Vec::new(),
                        },
                        CoreType::Generic {
                            name: String::from("PermissionDeniedError"),
                            type_args: Vec::new(),
                        },
                        CoreType::Generic {
                            name: String::from("WriteFailureError"),
                            type_args: Vec::new(),
                        },
                        CoreType::Generic {
                            name: String::from("IsADirectoryError"),
                            type_args: Vec::new(),
                        },
                        CoreType::Generic {
                            name: String::from("InvalidPathError"),
                            type_args: Vec::new(),
                        },
                        CoreType::Generic {
                            name: String::from("FilesystemFullError"),
                            type_args: Vec::new(),
                        },
                    ],
                },
                SymbolType::Function,
            ),
            (
                String::from("write_bytes_at_offset_sync"),
                CoreType::Function {
                    generic_params: Vec::new(),
                    parameters: vec![
                        CoreType::Generic {
                            name: String::from("FilesystemPath"),
                            type_args: Vec::new(),
                        },
                        CoreType::Int64,
                        CoreType::Generic {
                            name: String::from("Bytes"),
                            type_args: Vec::new(),
                        },
                    ],
                    return_types: vec![CoreType::Unit],
                    error_types: vec![
                        CoreType::Generic {
                            name: String::from("FileNotFoundError"),
                            type_args: Vec::new(),
                        },
                        CoreType::Generic {
                            name: String::from("PermissionDeniedError"),
                            type_args: Vec::new(),
                        },
                        CoreType::Generic {
                            name: String::from("WriteFailureError"),
                            type_args: Vec::new(),
                        },
                        CoreType::Generic {
                            name: String::from("OffsetOutOfRangeError"),
                            type_args: Vec::new(),
                        },
                        CoreType::Generic {
                            name: String::from("InvalidPathError"),
                            type_args: Vec::new(),
                        },
                        CoreType::Generic {
                            name: String::from("FilesystemFullError"),
                            type_args: Vec::new(),
                        },
                    ],
                },
                SymbolType::Function,
            ),
            (
                String::from("create_file_sync"),
                CoreType::Function {
                    generic_params: Vec::new(),
                    parameters: vec![CoreType::Generic {
                        name: String::from("FilesystemPath"),
                        type_args: Vec::new(),
                    }],
                    return_types: vec![CoreType::Unit],
                    error_types: vec![
                        CoreType::Generic {
                            name: String::from("FileAlreadyExistsError"),
                            type_args: Vec::new(),
                        },
                        CoreType::Generic {
                            name: String::from("PermissionDeniedError"),
                            type_args: Vec::new(),
                        },
                        CoreType::Generic {
                            name: String::from("CreateFailureError"),
                            type_args: Vec::new(),
                        },
                        CoreType::Generic {
                            name: String::from("InvalidPathError"),
                            type_args: Vec::new(),
                        },
                        CoreType::Generic {
                            name: String::from("FilesystemFullError"),
                            type_args: Vec::new(),
                        },
                    ],
                },
                SymbolType::Function,
            ),
            (
                String::from("delete_file_sync"),
                CoreType::Function {
                    generic_params: Vec::new(),
                    parameters: vec![CoreType::Generic {
                        name: String::from("FilesystemPath"),
                        type_args: Vec::new(),
                    }],
                    return_types: vec![CoreType::Unit],
                    error_types: vec![
                        CoreType::Generic {
                            name: String::from("FileNotFoundError"),
                            type_args: Vec::new(),
                        },
                        CoreType::Generic {
                            name: String::from("PermissionDeniedError"),
                            type_args: Vec::new(),
                        },
                        CoreType::Generic {
                            name: String::from("DeleteFailureError"),
                            type_args: Vec::new(),
                        },
                        CoreType::Generic {
                            name: String::from("IsADirectoryError"),
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
                String::from("copy_file_sync"),
                CoreType::Function {
                    generic_params: Vec::new(),
                    parameters: vec![
                        CoreType::Generic {
                            name: String::from("FilesystemPath"),
                            type_args: Vec::new(),
                        },
                        CoreType::Generic {
                            name: String::from("FilesystemPath"),
                            type_args: Vec::new(),
                        },
                    ],
                    return_types: vec![CoreType::Unit],
                    error_types: vec![
                        CoreType::Generic {
                            name: String::from("FileNotFoundError"),
                            type_args: Vec::new(),
                        },
                        CoreType::Generic {
                            name: String::from("PermissionDeniedError"),
                            type_args: Vec::new(),
                        },
                        CoreType::Generic {
                            name: String::from("CopyFailureError"),
                            type_args: Vec::new(),
                        },
                        CoreType::Generic {
                            name: String::from("IsADirectoryError"),
                            type_args: Vec::new(),
                        },
                        CoreType::Generic {
                            name: String::from("InvalidPathError"),
                            type_args: Vec::new(),
                        },
                        CoreType::Generic {
                            name: String::from("FilesystemFullError"),
                            type_args: Vec::new(),
                        },
                    ],
                },
                SymbolType::Function,
            ),
            (
                String::from("move_path_sync"),
                CoreType::Function {
                    generic_params: Vec::new(),
                    parameters: vec![
                        CoreType::Generic {
                            name: String::from("FilesystemPath"),
                            type_args: Vec::new(),
                        },
                        CoreType::Generic {
                            name: String::from("FilesystemPath"),
                            type_args: Vec::new(),
                        },
                    ],
                    return_types: vec![CoreType::Unit],
                    error_types: vec![
                        CoreType::Generic {
                            name: String::from("FileNotFoundError"),
                            type_args: Vec::new(),
                        },
                        CoreType::Generic {
                            name: String::from("PermissionDeniedError"),
                            type_args: Vec::new(),
                        },
                        CoreType::Generic {
                            name: String::from("MoveFailureError"),
                            type_args: Vec::new(),
                        },
                        CoreType::Generic {
                            name: String::from("FileAlreadyExistsError"),
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
                String::from("path_exists_sync"),
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
                String::from("read_metadata_sync"),
                CoreType::Function {
                    generic_params: Vec::new(),
                    parameters: vec![CoreType::Generic {
                        name: String::from("FilesystemPath"),
                        type_args: Vec::new(),
                    }],
                    return_types: vec![CoreType::Generic {
                        name: String::from("FileMetadata"),
                        type_args: Vec::new(),
                    }],
                    error_types: vec![
                        CoreType::Generic {
                            name: String::from("FileNotFoundError"),
                            type_args: Vec::new(),
                        },
                        CoreType::Generic {
                            name: String::from("PermissionDeniedError"),
                            type_args: Vec::new(),
                        },
                        CoreType::Generic {
                            name: String::from("MetadataUnavailableError"),
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
                String::from("read_metadata_nofollow_sync"),
                CoreType::Function {
                    generic_params: Vec::new(),
                    parameters: vec![CoreType::Generic {
                        name: String::from("FilesystemPath"),
                        type_args: Vec::new(),
                    }],
                    return_types: vec![CoreType::Generic {
                        name: String::from("FileMetadata"),
                        type_args: Vec::new(),
                    }],
                    error_types: vec![
                        CoreType::Generic {
                            name: String::from("FileNotFoundError"),
                            type_args: Vec::new(),
                        },
                        CoreType::Generic {
                            name: String::from("PermissionDeniedError"),
                            type_args: Vec::new(),
                        },
                        CoreType::Generic {
                            name: String::from("MetadataUnavailableError"),
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
                String::from("create_directory_sync"),
                CoreType::Function {
                    generic_params: Vec::new(),
                    parameters: vec![CoreType::Generic {
                        name: String::from("FilesystemPath"),
                        type_args: Vec::new(),
                    }],
                    return_types: vec![CoreType::Unit],
                    error_types: vec![
                        CoreType::Generic {
                            name: String::from("FileAlreadyExistsError"),
                            type_args: Vec::new(),
                        },
                        CoreType::Generic {
                            name: String::from("PermissionDeniedError"),
                            type_args: Vec::new(),
                        },
                        CoreType::Generic {
                            name: String::from("CreateFailureError"),
                            type_args: Vec::new(),
                        },
                        CoreType::Generic {
                            name: String::from("InvalidPathError"),
                            type_args: Vec::new(),
                        },
                        CoreType::Generic {
                            name: String::from("FilesystemFullError"),
                            type_args: Vec::new(),
                        },
                    ],
                },
                SymbolType::Function,
            ),
            (
                String::from("create_directory_recursive_sync"),
                CoreType::Function {
                    generic_params: Vec::new(),
                    parameters: vec![CoreType::Generic {
                        name: String::from("FilesystemPath"),
                        type_args: Vec::new(),
                    }],
                    return_types: vec![CoreType::Unit],
                    error_types: vec![
                        CoreType::Generic {
                            name: String::from("PermissionDeniedError"),
                            type_args: Vec::new(),
                        },
                        CoreType::Generic {
                            name: String::from("CreateFailureError"),
                            type_args: Vec::new(),
                        },
                        CoreType::Generic {
                            name: String::from("InvalidPathError"),
                            type_args: Vec::new(),
                        },
                        CoreType::Generic {
                            name: String::from("FilesystemFullError"),
                            type_args: Vec::new(),
                        },
                    ],
                },
                SymbolType::Function,
            ),
            (
                String::from("delete_directory_sync"),
                CoreType::Function {
                    generic_params: Vec::new(),
                    parameters: vec![CoreType::Generic {
                        name: String::from("FilesystemPath"),
                        type_args: Vec::new(),
                    }],
                    return_types: vec![CoreType::Unit],
                    error_types: vec![
                        CoreType::Generic {
                            name: String::from("DirectoryNotFoundError"),
                            type_args: Vec::new(),
                        },
                        CoreType::Generic {
                            name: String::from("PermissionDeniedError"),
                            type_args: Vec::new(),
                        },
                        CoreType::Generic {
                            name: String::from("DeleteFailureError"),
                            type_args: Vec::new(),
                        },
                        CoreType::Generic {
                            name: String::from("DirectoryNotEmptyError"),
                            type_args: Vec::new(),
                        },
                        CoreType::Generic {
                            name: String::from("IsNotADirectoryError"),
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
                String::from("delete_directory_recursive_sync"),
                CoreType::Function {
                    generic_params: Vec::new(),
                    parameters: vec![CoreType::Generic {
                        name: String::from("FilesystemPath"),
                        type_args: Vec::new(),
                    }],
                    return_types: vec![CoreType::Unit],
                    error_types: vec![
                        CoreType::Generic {
                            name: String::from("DirectoryNotFoundError"),
                            type_args: Vec::new(),
                        },
                        CoreType::Generic {
                            name: String::from("PermissionDeniedError"),
                            type_args: Vec::new(),
                        },
                        CoreType::Generic {
                            name: String::from("DeleteFailureError"),
                            type_args: Vec::new(),
                        },
                        CoreType::Generic {
                            name: String::from("IsNotADirectoryError"),
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
                String::from("list_directory_sync"),
                CoreType::Function {
                    generic_params: Vec::new(),
                    parameters: vec![CoreType::Generic {
                        name: String::from("FilesystemPath"),
                        type_args: Vec::new(),
                    }],
                    return_types: vec![CoreType::Array(alloc::boxed::Box::new(
                        CoreType::Generic {
                            name: String::from("FilesystemPath"),
                            type_args: Vec::new(),
                        },
                    ))],
                    error_types: vec![
                        CoreType::Generic {
                            name: String::from("DirectoryNotFoundError"),
                            type_args: Vec::new(),
                        },
                        CoreType::Generic {
                            name: String::from("PermissionDeniedError"),
                            type_args: Vec::new(),
                        },
                        CoreType::Generic {
                            name: String::from("ReadFailureError"),
                            type_args: Vec::new(),
                        },
                        CoreType::Generic {
                            name: String::from("IsNotADirectoryError"),
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
                String::from("is_file_sync"),
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
                String::from("is_file_nofollow_sync"),
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
        ]
    };
}

/// Filesystem mutation and query builtins.
pub(super) fn standard_symbols_filesystem_operations() -> Vec<(String, CoreType, SymbolType)> {
    standard_symbols_filesystem_operations_vec!()
}
