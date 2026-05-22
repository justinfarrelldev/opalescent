#![allow(
    clippy::all,
    clippy::too_many_lines,
    reason = "runtime return mapping is intentionally explicit and grouped by API surface"
)]
extern crate alloc;

use crate::ast::Expr;
use crate::codegen::expressions::CodegenEnv;
use crate::type_system::types::CoreType;
use alloc::string::String;
use alloc::vec::Vec;
use inkwell::types::BasicTypeEnum;

/// Infer semantic success type for values bound by `guard ... into`.
pub(super) fn infer_guard_success_core_type<'context>(
    env: &CodegenEnv<'context>,
    expression: &Expr,
    success_value_type: BasicTypeEnum<'context>,
) -> CoreType {
    let Expr::Call { ref callee, .. } = *expression else {
        return llvm_return_type_to_core_type(Some(success_value_type)).unwrap_or(CoreType::Int64);
    };
    let Expr::Identifier { ref name, .. } = *callee.as_ref() else {
        return llvm_return_type_to_core_type(Some(success_value_type)).unwrap_or(CoreType::Int64);
    };

    if let Some(runtime_name) = env.imported_functions.get(name) {
        if let Some(core_type) = known_guard_success_type(runtime_name.as_str()) {
            return core_type;
        }
    }
    if let Some(core_type) = known_guard_success_type(name) {
        return core_type;
    }

    llvm_return_type_to_core_type(Some(success_value_type)).unwrap_or(CoreType::Int64)
}

/// Map known runtime functions to language-level return `CoreType`.
pub(super) fn known_runtime_return_type(name: &str) -> Option<CoreType> {
    match name {
        "take_input"
        | "bytes_to_hex"
        | "path_file_name"
        | "path_file_extension"
        | "read_text_sync"
        | "read_first_line_sync"
        | "string_builder_finish" => Some(CoreType::String),
        "random_int8" => Some(CoreType::Int8),
        "random_int16" => Some(CoreType::Int16),
        "random_int32" | "bytes_length" => Some(CoreType::Int32),
        "random_int64" => Some(CoreType::Int64),
        "random_uint8" => Some(CoreType::UInt8),
        "random_uint16" => Some(CoreType::UInt16),
        "random_uint32" => Some(CoreType::UInt32),
        "random_uint64" => Some(CoreType::UInt64),
        "string_to_int8" => Some(CoreType::Generic {
            name: String::from("ParseResultI8"),
            type_args: Vec::new(),
        }),
        "string_to_int16" => Some(CoreType::Generic {
            name: String::from("ParseResultI16"),
            type_args: Vec::new(),
        }),
        "string_to_int32" => Some(CoreType::Generic {
            name: String::from("ParseResultI32"),
            type_args: Vec::new(),
        }),
        "string_to_int64" => Some(CoreType::Generic {
            name: String::from("ParseResultI64"),
            type_args: Vec::new(),
        }),
        "string_to_uint8" => Some(CoreType::Generic {
            name: String::from("ParseResultU8"),
            type_args: Vec::new(),
        }),
        "string_to_uint16" => Some(CoreType::Generic {
            name: String::from("ParseResultU16"),
            type_args: Vec::new(),
        }),
        "string_to_uint32" => Some(CoreType::Generic {
            name: String::from("ParseResultU32"),
            type_args: Vec::new(),
        }),
        "string_to_uint64" => Some(CoreType::Generic {
            name: String::from("ParseResultU64"),
            type_args: Vec::new(),
        }),
        "string_to_float32" => Some(CoreType::Generic {
            name: String::from("ParseResultF32"),
            type_args: Vec::new(),
        }),
        "string_to_float64" => Some(CoreType::Generic {
            name: String::from("ParseResultF64"),
            type_args: Vec::new(),
        }),
        "bytes_new" | "bytes_concatenate" | "bytes_from_hex" | "bytes_slice" => {
            Some(CoreType::Generic {
                name: String::from("Bytes"),
                type_args: Vec::new(),
            })
        }
        "path_from"
        | "join_path_components"
        | "path_parent_directory"
        | "normalize_path"
        | "absolute_path_sync" => Some(CoreType::Generic {
            name: String::from("FilesystemPath"),
            type_args: Vec::new(),
        }),
        "read_contents_sync" | "read_bytes_at_offset_sync" => Some(CoreType::Generic {
            name: String::from("Bytes"),
            type_args: Vec::new(),
        }),
        "stdout_writer" => Some(CoreType::Generic {
            name: String::from("StdoutWriter"),
            type_args: Vec::new(),
        }),
        "stdout_terminal" => Some(CoreType::Generic {
            name: String::from("StdoutTerminal"),
            type_args: Vec::new(),
        }),
        "frame_clock_new" => Some(CoreType::Generic {
            name: String::from("FrameClock"),
            type_args: Vec::new(),
        }),
        "read_lines_sync" => Some(CoreType::Array(alloc::boxed::Box::new(CoreType::String))),
        "print_text_sync"
        | "flush_standard_output_sync"
        | "sleep_ms_sync"
        | "frame_clock_wait_next_sync"
        | "terminal_clear_screen_on_sync"
        | "terminal_move_cursor_on_sync"
        | "terminal_draw_rows_sync"
        | "terminal_clear_screen_sync"
        | "terminal_move_cursor_sync"
        | "write_contents_sync"
        | "write_text_sync"
        | "write_contents_atomic_sync"
        | "write_text_atomic_sync"
        | "append_contents_sync"
        | "append_text_sync"
        | "write_bytes_at_offset_sync"
        | "create_file_sync"
        | "delete_file_sync"
        | "copy_file_sync"
        | "move_path_sync"
        | "create_directory_sync"
        | "create_directory_recursive_sync"
        | "delete_directory_sync"
        | "delete_directory_recursive_sync" => Some(CoreType::Unit),
        "path_exists_sync"
        | "is_file_sync"
        | "is_file_nofollow_sync"
        | "is_directory_sync"
        | "is_directory_nofollow_sync" => Some(CoreType::Boolean),
        "read_metadata_sync" | "read_metadata_nofollow_sync" => Some(CoreType::Generic {
            name: String::from("FileMetadata"),
            type_args: Vec::new(),
        }),
        "list_directory_sync" => Some(CoreType::Array(alloc::boxed::Box::new(CoreType::Generic {
            name: String::from("FilesystemPath"),
            type_args: Vec::new(),
        }))),
        _ => None,
    }
}

/// Map known runtime result wrappers to the success type produced by `guard`.
pub(super) fn known_guard_success_type(name: &str) -> Option<CoreType> {
    match name {
        "string_to_int8" => Some(CoreType::Int8),
        "string_to_int16" => Some(CoreType::Int16),
        "string_to_int32" => Some(CoreType::Int32),
        "string_to_int64" => Some(CoreType::Int64),
        "string_to_uint8" => Some(CoreType::UInt8),
        "string_to_uint16" => Some(CoreType::UInt16),
        "string_to_uint32" => Some(CoreType::UInt32),
        "string_to_uint64" => Some(CoreType::UInt64),
        "string_to_float32" => Some(CoreType::Float32),
        "string_to_float64" => Some(CoreType::Float64),
        "bytes_from_hex" | "bytes_slice" | "read_contents_sync" | "read_bytes_at_offset_sync" => {
            Some(CoreType::Generic {
                name: String::from("Bytes"),
                type_args: Vec::new(),
            })
        }
        "frame_clock_new" => Some(CoreType::Generic {
            name: String::from("FrameClock"),
            type_args: Vec::new(),
        }),
        "string_builder_finish" | "read_text_sync" | "read_first_line_sync" => {
            Some(CoreType::String)
        }
        "stdout_terminal" => Some(CoreType::Generic {
            name: String::from("StdoutTerminal"),
            type_args: Vec::new(),
        }),
        "print_text_sync"
        | "flush_standard_output_sync"
        | "writer_write_sync"
        | "writer_flush_sync"
        | "sleep_ms_sync"
        | "frame_clock_wait_next_sync"
        | "string_builder_push"
        | "terminal_clear_screen_on_sync"
        | "terminal_move_cursor_on_sync"
        | "terminal_draw_rows_sync"
        | "terminal_clear_screen_sync"
        | "terminal_move_cursor_sync"
        | "write_contents_sync"
        | "write_text_sync"
        | "write_contents_atomic_sync"
        | "write_text_atomic_sync"
        | "append_contents_sync"
        | "append_text_sync"
        | "write_bytes_at_offset_sync"
        | "create_file_sync"
        | "delete_file_sync"
        | "copy_file_sync"
        | "move_path_sync"
        | "create_directory_sync"
        | "create_directory_recursive_sync"
        | "delete_directory_sync"
        | "delete_directory_recursive_sync" => Some(CoreType::Unit),
        "absolute_path_sync" => Some(CoreType::Generic {
            name: String::from("FilesystemPath"),
            type_args: Vec::new(),
        }),
        "read_lines_sync" => Some(CoreType::Array(alloc::boxed::Box::new(CoreType::String))),
        "path_exists_sync"
        | "is_file_sync"
        | "is_file_nofollow_sync"
        | "is_directory_sync"
        | "is_directory_nofollow_sync" => Some(CoreType::Boolean),
        "read_metadata_sync" | "read_metadata_nofollow_sync" => Some(CoreType::Generic {
            name: String::from("FileMetadata"),
            type_args: Vec::new(),
        }),
        "list_directory_sync" => Some(CoreType::Array(alloc::boxed::Box::new(CoreType::Generic {
            name: String::from("FilesystemPath"),
            type_args: Vec::new(),
        }))),
        _ => None,
    }
}

/// Convert LLVM return type metadata to fallback `CoreType` when possible.
pub(super) fn llvm_return_type_to_core_type(
    return_type: Option<BasicTypeEnum<'_>>,
) -> Option<CoreType> {
    return_type.map(|llvm_type| match llvm_type {
        BasicTypeEnum::IntType(int_type) => match int_type.get_bit_width() {
            1 => CoreType::Boolean,
            8 => CoreType::Int8,
            16 => CoreType::Int16,
            32 => CoreType::Int32,
            _ => CoreType::Int64,
        },
        BasicTypeEnum::FloatType(float_type) => {
            if float_type.get_bit_width() == 32 {
                CoreType::Float32
            } else {
                CoreType::Float64
            }
        }
        BasicTypeEnum::PointerType(_) => CoreType::String,
        BasicTypeEnum::ArrayType(_) => CoreType::Array(alloc::boxed::Box::new(CoreType::Int64)),
        BasicTypeEnum::StructType(_)
        | BasicTypeEnum::VectorType(_)
        | BasicTypeEnum::ScalableVectorType(_) => CoreType::Unit,
    })
}
