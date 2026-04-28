extern crate alloc;

use crate::type_system::symbol_table::SymbolType;
use crate::type_system::types::{CoreType, GenericTypeParameter};
use alloc::{string::String, vec::Vec};

/// Macro-wrapped symbol literal to keep the provider function concise for clippy.
macro_rules! standard_symbols_core_io_and_bytes_vec {
    () => {
        vec![
            (
                String::from("print"),
                CoreType::Function {
                    generic_params: Vec::new(),
                    parameters: vec![CoreType::Variable(crate::type_system::types::TypeVar::new(
                        0,
                        String::from("T"),
                    ))],
                    return_types: vec![CoreType::Unit],
                    error_types: Vec::new(),
                },
                SymbolType::Function,
            ),
            (
                String::from("println"),
                CoreType::Function {
                    generic_params: Vec::new(),
                    parameters: vec![CoreType::String],
                    return_types: vec![CoreType::Unit],
                    error_types: Vec::new(),
                },
                SymbolType::Function,
            ),
            (
                String::from("take_input"),
                CoreType::Function {
                    generic_params: Vec::new(),
                    parameters: Vec::new(),
                    return_types: vec![CoreType::String],
                    error_types: Vec::new(),
                },
                SymbolType::Function,
            ),
            (
                String::from("string_to_int32"),
                CoreType::Function {
                    generic_params: Vec::new(),
                    parameters: vec![CoreType::String],
                    return_types: vec![CoreType::Int32],
                    error_types: vec![CoreType::Generic {
                        name: "ParseError".to_owned(),
                        type_args: Vec::new(),
                    }],
                },
                SymbolType::Function,
            ),
            (
                String::from("string_to_int64"),
                CoreType::Function {
                    generic_params: Vec::new(),
                    parameters: vec![CoreType::String],
                    return_types: vec![CoreType::Int64],
                    error_types: vec![CoreType::Generic {
                        name: "ParseError".to_owned(),
                        type_args: Vec::new(),
                    }],
                },
                SymbolType::Function,
            ),
            (
                String::from("string_to_int8"),
                CoreType::Function {
                    generic_params: Vec::new(),
                    parameters: vec![CoreType::String],
                    return_types: vec![CoreType::Int8],
                    error_types: vec![CoreType::Generic {
                        name: "ParseError".to_owned(),
                        type_args: Vec::new(),
                    }],
                },
                SymbolType::Function,
            ),
            (
                String::from("string_to_int16"),
                CoreType::Function {
                    generic_params: Vec::new(),
                    parameters: vec![CoreType::String],
                    return_types: vec![CoreType::Int16],
                    error_types: vec![CoreType::Generic {
                        name: "ParseError".to_owned(),
                        type_args: Vec::new(),
                    }],
                },
                SymbolType::Function,
            ),
            (
                String::from("string_to_uint8"),
                CoreType::Function {
                    generic_params: Vec::new(),
                    parameters: vec![CoreType::String],
                    return_types: vec![CoreType::UInt8],
                    error_types: vec![CoreType::Generic {
                        name: "ParseError".to_owned(),
                        type_args: Vec::new(),
                    }],
                },
                SymbolType::Function,
            ),
            (
                String::from("string_to_uint16"),
                CoreType::Function {
                    generic_params: Vec::new(),
                    parameters: vec![CoreType::String],
                    return_types: vec![CoreType::UInt16],
                    error_types: vec![CoreType::Generic {
                        name: "ParseError".to_owned(),
                        type_args: Vec::new(),
                    }],
                },
                SymbolType::Function,
            ),
            (
                String::from("string_to_uint32"),
                CoreType::Function {
                    generic_params: Vec::new(),
                    parameters: vec![CoreType::String],
                    return_types: vec![CoreType::UInt32],
                    error_types: vec![CoreType::Generic {
                        name: "ParseError".to_owned(),
                        type_args: Vec::new(),
                    }],
                },
                SymbolType::Function,
            ),
            (
                String::from("string_to_uint64"),
                CoreType::Function {
                    generic_params: Vec::new(),
                    parameters: vec![CoreType::String],
                    return_types: vec![CoreType::UInt64],
                    error_types: vec![CoreType::Generic {
                        name: "ParseError".to_owned(),
                        type_args: Vec::new(),
                    }],
                },
                SymbolType::Function,
            ),
            (
                String::from("string_to_float32"),
                CoreType::Function {
                    generic_params: Vec::new(),
                    parameters: vec![CoreType::String],
                    return_types: vec![CoreType::Float32],
                    error_types: vec![CoreType::Generic {
                        name: "ParseError".to_owned(),
                        type_args: Vec::new(),
                    }],
                },
                SymbolType::Function,
            ),
            (
                String::from("string_to_float64"),
                CoreType::Function {
                    generic_params: Vec::new(),
                    parameters: vec![CoreType::String],
                    return_types: vec![CoreType::Float64],
                    error_types: vec![CoreType::Generic {
                        name: "ParseError".to_owned(),
                        type_args: Vec::new(),
                    }],
                },
                SymbolType::Function,
            ),
            (
                String::from("int8_to_string"),
                CoreType::Function {
                    generic_params: Vec::new(),
                    parameters: vec![CoreType::Int8],
                    return_types: vec![CoreType::String],
                    error_types: Vec::new(),
                },
                SymbolType::Function,
            ),
            (
                String::from("int16_to_string"),
                CoreType::Function {
                    generic_params: Vec::new(),
                    parameters: vec![CoreType::Int16],
                    return_types: vec![CoreType::String],
                    error_types: Vec::new(),
                },
                SymbolType::Function,
            ),
            (
                String::from("int32_to_string"),
                CoreType::Function {
                    generic_params: Vec::new(),
                    parameters: vec![CoreType::Int32],
                    return_types: vec![CoreType::String],
                    error_types: Vec::new(),
                },
                SymbolType::Function,
            ),
            (
                String::from("int64_to_string"),
                CoreType::Function {
                    generic_params: Vec::new(),
                    parameters: vec![CoreType::Int64],
                    return_types: vec![CoreType::String],
                    error_types: Vec::new(),
                },
                SymbolType::Function,
            ),
            (
                String::from("uint8_to_string"),
                CoreType::Function {
                    generic_params: Vec::new(),
                    parameters: vec![CoreType::UInt8],
                    return_types: vec![CoreType::String],
                    error_types: Vec::new(),
                },
                SymbolType::Function,
            ),
            (
                String::from("uint16_to_string"),
                CoreType::Function {
                    generic_params: Vec::new(),
                    parameters: vec![CoreType::UInt16],
                    return_types: vec![CoreType::String],
                    error_types: Vec::new(),
                },
                SymbolType::Function,
            ),
            (
                String::from("uint32_to_string"),
                CoreType::Function {
                    generic_params: Vec::new(),
                    parameters: vec![CoreType::UInt32],
                    return_types: vec![CoreType::String],
                    error_types: Vec::new(),
                },
                SymbolType::Function,
            ),
            (
                String::from("uint64_to_string"),
                CoreType::Function {
                    generic_params: Vec::new(),
                    parameters: vec![CoreType::UInt64],
                    return_types: vec![CoreType::String],
                    error_types: Vec::new(),
                },
                SymbolType::Function,
            ),
            (
                String::from("float32_to_string"),
                CoreType::Function {
                    generic_params: Vec::new(),
                    parameters: vec![CoreType::Float32],
                    return_types: vec![CoreType::String],
                    error_types: Vec::new(),
                },
                SymbolType::Function,
            ),
            (
                String::from("float64_to_string"),
                CoreType::Function {
                    generic_params: Vec::new(),
                    parameters: vec![CoreType::Float64],
                    return_types: vec![CoreType::String],
                    error_types: Vec::new(),
                },
                SymbolType::Function,
            ),
            (
                String::from("bool_to_string"),
                CoreType::Function {
                    generic_params: Vec::new(),
                    parameters: vec![CoreType::Boolean],
                    return_types: vec![CoreType::String],
                    error_types: Vec::new(),
                },
                SymbolType::Function,
            ),
            (
                String::from("string_length"),
                CoreType::Function {
                    generic_params: Vec::new(),
                    parameters: vec![CoreType::String],
                    return_types: vec![CoreType::Int64],
                    error_types: Vec::new(),
                },
                SymbolType::Function,
            ),
            (
                String::from("array_length"),
                CoreType::Function {
                    generic_params: vec![GenericTypeParameter {
                        name: "T".to_owned(),
                        type_var: crate::type_system::types::TypeVar::new(9_001, "T".to_owned()),
                        constraints: Vec::new(),
                    }],
                    parameters: vec![CoreType::Array(alloc::boxed::Box::new(CoreType::Variable(
                        crate::type_system::types::TypeVar::new(9_001, "T".to_owned()),
                    )))],
                    return_types: vec![CoreType::Int64],
                    error_types: Vec::new(),
                },
                SymbolType::Function,
            ),
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

/// Core runtime, conversion, bytes, and foundational file-I/O builtins.
pub(super) fn standard_symbols_core_io_and_bytes() -> Vec<(String, CoreType, SymbolType)> {
    standard_symbols_core_io_and_bytes_vec!()
}
