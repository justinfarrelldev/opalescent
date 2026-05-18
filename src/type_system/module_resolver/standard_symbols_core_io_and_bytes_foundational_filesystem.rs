extern crate alloc;

use crate::type_system::symbol_table::SymbolType;
use crate::type_system::types::CoreType;
use alloc::{string::String, vec::Vec};

/// Macro-wrapped bytes symbol literals keep the provider helper concise for clippy.
macro_rules! bytes_symbols_vec {
    () => {
        vec![
            // `Bytes` stdlib surface. The opaque byte-buffer type is represented
            // as a nominal `Generic { name: "Bytes", type_args: [] }` which
            // lowers to `i8*` in codegen. The two fallible helpers surface
            // language-level error types so `guard`/`propagate` bind string
            // error messages for the user.
            (
                String::from("bytes_new"),
                CoreType::Function {
                    generic_params: Vec::new(),
                    parameters: Vec::new(),
                    return_types: vec![CoreType::Generic {
                        name: String::from("Bytes"),
                        type_args: Vec::new(),
                    }],
                    error_types: Vec::new(),
                },
                SymbolType::Function,
            ),
            (
                String::from("bytes_length"),
                CoreType::Function {
                    generic_params: Vec::new(),
                    parameters: vec![CoreType::Generic {
                        name: String::from("Bytes"),
                        type_args: Vec::new(),
                    }],
                    return_types: vec![CoreType::Int32],
                    error_types: Vec::new(),
                },
                SymbolType::Function,
            ),
            (
                String::from("bytes_to_hex"),
                CoreType::Function {
                    generic_params: Vec::new(),
                    parameters: vec![CoreType::Generic {
                        name: String::from("Bytes"),
                        type_args: Vec::new(),
                    }],
                    return_types: vec![CoreType::String],
                    error_types: Vec::new(),
                },
                SymbolType::Function,
            ),
            (
                String::from("bytes_concatenate"),
                CoreType::Function {
                    generic_params: Vec::new(),
                    parameters: vec![
                        CoreType::Generic {
                            name: String::from("Bytes"),
                            type_args: Vec::new(),
                        },
                        CoreType::Generic {
                            name: String::from("Bytes"),
                            type_args: Vec::new(),
                        },
                    ],
                    return_types: vec![CoreType::Generic {
                        name: String::from("Bytes"),
                        type_args: Vec::new(),
                    }],
                    error_types: Vec::new(),
                },
                SymbolType::Function,
            ),
            (
                String::from("bytes_from_hex"),
                CoreType::Function {
                    generic_params: Vec::new(),
                    parameters: vec![CoreType::String],
                    return_types: vec![CoreType::Generic {
                        name: String::from("Bytes"),
                        type_args: Vec::new(),
                    }],
                    error_types: vec![CoreType::Generic {
                        name: String::from("HexDecodeError"),
                        type_args: Vec::new(),
                    }],
                },
                SymbolType::Function,
            ),
            (
                String::from("bytes_slice"),
                CoreType::Function {
                    generic_params: Vec::new(),
                    parameters: vec![
                        CoreType::Generic {
                            name: String::from("Bytes"),
                            type_args: Vec::new(),
                        },
                        CoreType::Int32,
                        CoreType::Int32,
                    ],
                    return_types: vec![CoreType::Generic {
                        name: String::from("Bytes"),
                        type_args: Vec::new(),
                    }],
                    error_types: vec![CoreType::Generic {
                        name: String::from("SliceRangeError"),
                        type_args: Vec::new(),
                    }],
                },
                SymbolType::Function,
            ),
        ]
    };
}

/// Macro-wrapped foundational path symbol literals keep the provider helper concise for clippy.
macro_rules! foundational_path_symbols_vec {
    () => {
        vec![
            (
                String::from("path_from"),
                CoreType::Function {
                    generic_params: Vec::new(),
                    parameters: vec![CoreType::String],
                    return_types: vec![CoreType::Generic {
                        name: String::from("FilesystemPath"),
                        type_args: Vec::new(),
                    }],
                    error_types: Vec::new(),
                },
                SymbolType::Function,
            ),
            (
                String::from("join_path_components"),
                CoreType::Function {
                    generic_params: Vec::new(),
                    parameters: vec![
                        CoreType::Generic {
                            name: String::from("FilesystemPath"),
                            type_args: Vec::new(),
                        },
                        CoreType::Array(alloc::boxed::Box::new(CoreType::String)),
                    ],
                    return_types: vec![CoreType::Generic {
                        name: String::from("FilesystemPath"),
                        type_args: Vec::new(),
                    }],
                    error_types: Vec::new(),
                },
                SymbolType::Function,
            ),
            (
                String::from("path_parent_directory"),
                CoreType::Function {
                    generic_params: Vec::new(),
                    parameters: vec![CoreType::Generic {
                        name: String::from("FilesystemPath"),
                        type_args: Vec::new(),
                    }],
                    return_types: vec![CoreType::Generic {
                        name: String::from("FilesystemPath"),
                        type_args: Vec::new(),
                    }],
                    error_types: Vec::new(),
                },
                SymbolType::Function,
            ),
            (
                String::from("path_file_name"),
                CoreType::Function {
                    generic_params: Vec::new(),
                    parameters: vec![CoreType::Generic {
                        name: String::from("FilesystemPath"),
                        type_args: Vec::new(),
                    }],
                    return_types: vec![CoreType::String],
                    error_types: Vec::new(),
                },
                SymbolType::Function,
            ),
            (
                String::from("path_file_extension"),
                CoreType::Function {
                    generic_params: Vec::new(),
                    parameters: vec![CoreType::Generic {
                        name: String::from("FilesystemPath"),
                        type_args: Vec::new(),
                    }],
                    return_types: vec![CoreType::String],
                    error_types: Vec::new(),
                },
                SymbolType::Function,
            ),
            (
                String::from("normalize_path"),
                CoreType::Function {
                    generic_params: Vec::new(),
                    parameters: vec![CoreType::Generic {
                        name: String::from("FilesystemPath"),
                        type_args: Vec::new(),
                    }],
                    return_types: vec![CoreType::Generic {
                        name: String::from("FilesystemPath"),
                        type_args: Vec::new(),
                    }],
                    error_types: Vec::new(),
                },
                SymbolType::Function,
            ),
            (
                String::from("path_to_string"),
                CoreType::Function {
                    generic_params: Vec::new(),
                    parameters: vec![CoreType::Generic {
                        name: String::from("FilesystemPath"),
                        type_args: Vec::new(),
                    }],
                    return_types: vec![CoreType::String],
                    error_types: Vec::new(),
                },
                SymbolType::Function,
            ),
            (
                String::from("absolute_path_sync"),
                CoreType::Function {
                    generic_params: Vec::new(),
                    parameters: vec![CoreType::Generic {
                        name: String::from("FilesystemPath"),
                        type_args: Vec::new(),
                    }],
                    return_types: vec![CoreType::Generic {
                        name: String::from("FilesystemPath"),
                        type_args: Vec::new(),
                    }],
                    error_types: vec![
                        CoreType::Generic {
                            name: String::from("InvalidPathError"),
                            type_args: Vec::new(),
                        },
                        CoreType::Generic {
                            name: String::from("PermissionDeniedError"),
                            type_args: Vec::new(),
                        },
                    ],
                },
                SymbolType::Function,
            ),
        ]
    };
}

/// Macro-wrapped foundational file-reading symbol literals keep the provider helper concise for clippy.
macro_rules! foundational_file_read_symbols_vec {
    () => {
        vec![
            (
                String::from("read_contents_sync"),
                CoreType::Function {
                    generic_params: Vec::new(),
                    parameters: vec![CoreType::Generic {
                        name: String::from("FilesystemPath"),
                        type_args: Vec::new(),
                    }],
                    return_types: vec![CoreType::Generic {
                        name: String::from("Bytes"),
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
                            name: String::from("ReadFailureError"),
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
                String::from("read_text_sync"),
                CoreType::Function {
                    generic_params: Vec::new(),
                    parameters: vec![CoreType::Generic {
                        name: String::from("FilesystemPath"),
                        type_args: Vec::new(),
                    }],
                    return_types: vec![CoreType::String],
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
                            name: String::from("ReadFailureError"),
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
                            name: String::from("InvalidUtf8Error"),
                            type_args: Vec::new(),
                        },
                    ],
                },
                SymbolType::Function,
            ),
            (
                String::from("read_first_line_sync"),
                CoreType::Function {
                    generic_params: Vec::new(),
                    parameters: vec![CoreType::Generic {
                        name: String::from("FilesystemPath"),
                        type_args: Vec::new(),
                    }],
                    return_types: vec![CoreType::String],
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
                            name: String::from("IsADirectoryError"),
                            type_args: Vec::new(),
                        },
                        CoreType::Generic {
                            name: String::from("InvalidUtf8Error"),
                            type_args: Vec::new(),
                        },
                        CoreType::Generic {
                            name: String::from("OffsetOutOfRangeError"),
                            type_args: Vec::new(),
                        },
                        CoreType::Generic {
                            name: String::from("ReadFailureError"),
                            type_args: Vec::new(),
                        },
                    ],
                },
                SymbolType::Function,
            ),
            (
                String::from("read_lines_sync"),
                CoreType::Function {
                    generic_params: Vec::new(),
                    parameters: vec![CoreType::Generic {
                        name: String::from("FilesystemPath"),
                        type_args: Vec::new(),
                    }],
                    return_types: vec![CoreType::Array(alloc::boxed::Box::new(CoreType::String))],
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
                            name: String::from("ReadFailureError"),
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
                            name: String::from("InvalidUtf8Error"),
                            type_args: Vec::new(),
                        },
                    ],
                },
                SymbolType::Function,
            ),
            (
                String::from("read_bytes_at_offset_sync"),
                CoreType::Function {
                    generic_params: Vec::new(),
                    parameters: vec![
                        CoreType::Generic {
                            name: String::from("FilesystemPath"),
                            type_args: Vec::new(),
                        },
                        CoreType::Int64,
                        CoreType::Int64,
                    ],
                    return_types: vec![CoreType::Generic {
                        name: String::from("Bytes"),
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
                            name: String::from("ReadFailureError"),
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
                    ],
                },
                SymbolType::Function,
            ),
        ]
    };
}

/// Macro-wrapped foundational file-writing symbol literals keep the provider helper concise for clippy.
macro_rules! foundational_file_write_symbols_vec {
    () => {
        vec![
            (
                String::from("write_contents_sync"),
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
                String::from("write_text_sync"),
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
                String::from("write_contents_atomic_sync"),
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
                String::from("write_text_atomic_sync"),
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
        ]
    };
}

/// Bytes plus foundational filesystem/path symbols extracted from the standard resolver table.
pub(super) fn standard_symbols_bytes_and_foundational_filesystem(
) -> Vec<(String, CoreType, SymbolType)> {
    let mut symbols = bytes_symbols();
    symbols.extend(foundational_path_symbols());
    symbols.extend(foundational_file_read_symbols());
    symbols.extend(foundational_file_write_symbols());
    symbols
}

/// Build the extracted `Bytes` stdlib symbol registrations.
fn bytes_symbols() -> Vec<(String, CoreType, SymbolType)> {
    bytes_symbols_vec!()
}

/// Build the extracted foundational path manipulation symbol registrations.
fn foundational_path_symbols() -> Vec<(String, CoreType, SymbolType)> {
    foundational_path_symbols_vec!()
}

/// Build the extracted foundational file-reading symbol registrations.
fn foundational_file_read_symbols() -> Vec<(String, CoreType, SymbolType)> {
    foundational_file_read_symbols_vec!()
}

/// Build the extracted foundational file-writing symbol registrations.
fn foundational_file_write_symbols() -> Vec<(String, CoreType, SymbolType)> {
    foundational_file_write_symbols_vec!()
}
