extern crate alloc;

use crate::type_system::symbol_table::SymbolType;
use crate::type_system::types::{CoreType, GenericTypeParameter};
use alloc::{string::String, vec::Vec};

/// Extracted bytes/path/foundational-filesystem symbol registrations used by the standard module.
#[path = "standard_symbols_core_io_and_bytes_foundational_filesystem.rs"]
mod foundational_filesystem;
use self::foundational_filesystem::standard_symbols_bytes_and_foundational_filesystem;

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
                String::from("string_join"),
                CoreType::Function {
                    generic_params: Vec::new(),
                    parameters: vec![
                        CoreType::Array(alloc::boxed::Box::new(CoreType::String)),
                        CoreType::String,
                    ],
                    return_types: vec![CoreType::String],
                    error_types: Vec::new(),
                },
                SymbolType::Function,
            ),
            (
                String::from("string_builder_new"),
                CoreType::Function {
                    generic_params: Vec::new(),
                    parameters: Vec::new(),
                    return_types: vec![CoreType::Generic {
                        name: String::from("StringBuilder"),
                        type_args: Vec::new(),
                    }],
                    error_types: Vec::new(),
                },
                SymbolType::Function,
            ),
            (
                String::from("string_builder_push"),
                CoreType::Function {
                    generic_params: Vec::new(),
                    parameters: vec![
                        CoreType::Generic {
                            name: String::from("StringBuilder"),
                            type_args: Vec::new(),
                        },
                        CoreType::String,
                    ],
                    return_types: vec![CoreType::Unit],
                    error_types: vec![
                        CoreType::Generic {
                            name: String::from("BuilderFinishedError"),
                            type_args: Vec::new(),
                        },
                        CoreType::Generic {
                            name: String::from("AllocationFailureError"),
                            type_args: Vec::new(),
                        },
                    ],
                },
                SymbolType::Function,
            ),
            (
                String::from("string_builder_finish"),
                CoreType::Function {
                    generic_params: Vec::new(),
                    parameters: vec![CoreType::Generic {
                        name: String::from("StringBuilder"),
                        type_args: Vec::new(),
                    }],
                    return_types: vec![CoreType::String],
                    error_types: vec![
                        CoreType::Generic {
                            name: String::from("BuilderFinishedError"),
                            type_args: Vec::new(),
                        },
                        CoreType::Generic {
                            name: String::from("AllocationFailureError"),
                            type_args: Vec::new(),
                        },
                    ],
                },
                SymbolType::Function,
            ),
            (
                String::from("print_text_sync"),
                CoreType::Function {
                    generic_params: Vec::new(),
                    parameters: vec![CoreType::String],
                    return_types: vec![CoreType::Unit],
                    error_types: vec![
                        CoreType::Generic {
                            name: String::from("WriteFailureError"),
                            type_args: Vec::new(),
                        },
                        CoreType::Generic {
                            name: String::from("SinkClosedError"),
                            type_args: Vec::new(),
                        },
                    ],
                },
                SymbolType::Function,
            ),
            (
                String::from("flush_standard_output_sync"),
                CoreType::Function {
                    generic_params: Vec::new(),
                    parameters: Vec::new(),
                    return_types: vec![CoreType::Unit],
                    error_types: vec![
                        CoreType::Generic {
                            name: String::from("FlushFailureError"),
                            type_args: Vec::new(),
                        },
                        CoreType::Generic {
                            name: String::from("SinkClosedError"),
                            type_args: Vec::new(),
                        },
                    ],
                },
                SymbolType::Function,
            ),
            (
                String::from("stdout_writer"),
                CoreType::Function {
                    generic_params: Vec::new(),
                    parameters: Vec::new(),
                    return_types: vec![CoreType::Generic {
                        name: String::from("StdoutWriter"),
                        type_args: Vec::new(),
                    }],
                    error_types: Vec::new(),
                },
                SymbolType::Function,
            ),
            (
                String::from("writer_write_sync"),
                CoreType::Function {
                    generic_params: Vec::new(),
                    parameters: vec![
                        CoreType::Generic {
                            name: String::from("StdoutWriter"),
                            type_args: Vec::new(),
                        },
                        CoreType::String,
                    ],
                    return_types: vec![CoreType::Unit],
                    error_types: vec![
                        CoreType::Generic {
                            name: String::from("WriteFailureError"),
                            type_args: Vec::new(),
                        },
                        CoreType::Generic {
                            name: String::from("SinkClosedError"),
                            type_args: Vec::new(),
                        },
                    ],
                },
                SymbolType::Function,
            ),
            (
                String::from("writer_flush_sync"),
                CoreType::Function {
                    generic_params: Vec::new(),
                    parameters: vec![CoreType::Generic {
                        name: String::from("StdoutWriter"),
                        type_args: Vec::new(),
                    }],
                    return_types: vec![CoreType::Unit],
                    error_types: vec![
                        CoreType::Generic {
                            name: String::from("FlushFailureError"),
                            type_args: Vec::new(),
                        },
                        CoreType::Generic {
                            name: String::from("SinkClosedError"),
                            type_args: Vec::new(),
                        },
                    ],
                },
                SymbolType::Function,
            ),
            (
                String::from("stdout_terminal"),
                CoreType::Function {
                    generic_params: Vec::new(),
                    parameters: Vec::new(),
                    return_types: vec![CoreType::Generic {
                        name: String::from("StdoutTerminal"),
                        type_args: Vec::new(),
                    }],
                    error_types: Vec::new(),
                },
                SymbolType::Function,
            ),
            (
                String::from("terminal_supports_ansi"),
                CoreType::Function {
                    generic_params: Vec::new(),
                    parameters: vec![CoreType::Generic {
                        name: String::from("StdoutTerminal"),
                        type_args: Vec::new(),
                    }],
                    return_types: vec![CoreType::Boolean],
                    error_types: Vec::new(),
                },
                SymbolType::Function,
            ),
            (
                String::from("terminal_clear_screen_on_sync"),
                CoreType::Function {
                    generic_params: Vec::new(),
                    parameters: vec![CoreType::Generic {
                        name: String::from("StdoutTerminal"),
                        type_args: Vec::new(),
                    }],
                    return_types: vec![CoreType::Unit],
                    error_types: vec![
                        CoreType::Generic {
                            name: String::from("TerminalWriteFailureError"),
                            type_args: Vec::new(),
                        },
                        CoreType::Generic {
                            name: String::from("SinkClosedError"),
                            type_args: Vec::new(),
                        },
                    ],
                },
                SymbolType::Function,
            ),
            (
                String::from("terminal_move_cursor_on_sync"),
                CoreType::Function {
                    generic_params: Vec::new(),
                    parameters: vec![
                        CoreType::Generic {
                            name: String::from("StdoutTerminal"),
                            type_args: Vec::new(),
                        },
                        CoreType::Int32,
                        CoreType::Int32,
                    ],
                    return_types: vec![CoreType::Unit],
                    error_types: vec![
                        CoreType::Generic {
                            name: String::from("TerminalWriteFailureError"),
                            type_args: Vec::new(),
                        },
                        CoreType::Generic {
                            name: String::from("InvalidCursorPositionError"),
                            type_args: Vec::new(),
                        },
                        CoreType::Generic {
                            name: String::from("SinkClosedError"),
                            type_args: Vec::new(),
                        },
                    ],
                },
                SymbolType::Function,
            ),
            (
                String::from("terminal_draw_rows_sync"),
                CoreType::Function {
                    generic_params: Vec::new(),
                    parameters: vec![
                        CoreType::Generic {
                            name: String::from("StdoutTerminal"),
                            type_args: Vec::new(),
                        },
                        CoreType::Array(alloc::boxed::Box::new(CoreType::String)),
                    ],
                    return_types: vec![CoreType::Unit],
                    error_types: vec![
                        CoreType::Generic {
                            name: String::from("TerminalWriteFailureError"),
                            type_args: Vec::new(),
                        },
                        CoreType::Generic {
                            name: String::from("SinkClosedError"),
                            type_args: Vec::new(),
                        },
                    ],
                },
                SymbolType::Function,
            ),
            (
                String::from("terminal_clear_screen_sync"),
                CoreType::Function {
                    generic_params: Vec::new(),
                    parameters: Vec::new(),
                    return_types: vec![CoreType::Unit],
                    error_types: vec![
                        CoreType::Generic {
                            name: String::from("TerminalWriteFailureError"),
                            type_args: Vec::new(),
                        },
                        CoreType::Generic {
                            name: String::from("SinkClosedError"),
                            type_args: Vec::new(),
                        },
                    ],
                },
                SymbolType::Function,
            ),
            (
                String::from("terminal_move_cursor_sync"),
                CoreType::Function {
                    generic_params: Vec::new(),
                    parameters: vec![CoreType::Int32, CoreType::Int32],
                    return_types: vec![CoreType::Unit],
                    error_types: vec![
                        CoreType::Generic {
                            name: String::from("TerminalWriteFailureError"),
                            type_args: Vec::new(),
                        },
                        CoreType::Generic {
                            name: String::from("InvalidCursorPositionError"),
                            type_args: Vec::new(),
                        },
                        CoreType::Generic {
                            name: String::from("SinkClosedError"),
                            type_args: Vec::new(),
                        },
                    ],
                },
                SymbolType::Function,
            ),
            (
                String::from("sleep_ms_sync"),
                CoreType::Function {
                    generic_params: Vec::new(),
                    parameters: vec![CoreType::Int32],
                    return_types: vec![CoreType::Unit],
                    error_types: vec![CoreType::Generic {
                        name: String::from("InvalidDurationError"),
                        type_args: Vec::new(),
                    }],
                },
                SymbolType::Function,
            ),
            (
                String::from("frame_clock_new"),
                CoreType::Function {
                    generic_params: Vec::new(),
                    parameters: vec![CoreType::Int32],
                    return_types: vec![CoreType::Generic {
                        name: String::from("FrameClock"),
                        type_args: Vec::new(),
                    }],
                    error_types: vec![CoreType::Generic {
                        name: String::from("InvalidFrameRateError"),
                        type_args: Vec::new(),
                    }],
                },
                SymbolType::Function,
            ),
            (
                String::from("frame_clock_wait_next_sync"),
                CoreType::Function {
                    generic_params: Vec::new(),
                    parameters: vec![CoreType::Generic {
                        name: String::from("FrameClock"),
                        type_args: Vec::new(),
                    }],
                    return_types: vec![CoreType::Unit],
                    error_types: vec![CoreType::Generic {
                        name: String::from("InvalidFrameRateError"),
                        type_args: Vec::new(),
                    }],
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
            (
                String::from("append"),
                CoreType::Function {
                    generic_params: vec![GenericTypeParameter {
                        name: "T".to_owned(),
                        type_var: crate::type_system::types::TypeVar::new(9_002, "T".to_owned()),
                        constraints: Vec::new(),
                    }],
                    parameters: vec![
                        CoreType::Array(alloc::boxed::Box::new(CoreType::Variable(
                            crate::type_system::types::TypeVar::new(9_002, "T".to_owned()),
                        ))),
                        CoreType::Variable(crate::type_system::types::TypeVar::new(
                            9_002,
                            "T".to_owned(),
                        )),
                    ],
                    return_types: vec![CoreType::Array(alloc::boxed::Box::new(
                        CoreType::Variable(crate::type_system::types::TypeVar::new(
                            9_002,
                            "T".to_owned(),
                        )),
                    ))],
                    error_types: Vec::new(),
                },
                SymbolType::Function,
            ),
            (
                String::from("array_filled"),
                CoreType::Function {
                    generic_params: vec![GenericTypeParameter {
                        name: "T".to_owned(),
                        type_var: crate::type_system::types::TypeVar::new(9_003, "T".to_owned()),
                        constraints: Vec::new(),
                    }],
                    parameters: vec![
                        CoreType::Int64,
                        CoreType::Variable(crate::type_system::types::TypeVar::new(
                            9_003,
                            "T".to_owned(),
                        )),
                    ],
                    return_types: vec![CoreType::Array(alloc::boxed::Box::new(
                        CoreType::Variable(crate::type_system::types::TypeVar::new(
                            9_003,
                            "T".to_owned(),
                        )),
                    ))],
                    error_types: Vec::new(),
                },
                SymbolType::Function,
            ),
            (
                String::from("reserve"),
                CoreType::Function {
                    generic_params: vec![GenericTypeParameter {
                        name: "T".to_owned(),
                        type_var: crate::type_system::types::TypeVar::new(9_004, "T".to_owned()),
                        constraints: Vec::new(),
                    }],
                    parameters: vec![
                        CoreType::Array(alloc::boxed::Box::new(CoreType::Variable(
                            crate::type_system::types::TypeVar::new(9_004, "T".to_owned()),
                        ))),
                        CoreType::Int64,
                    ],
                    return_types: vec![CoreType::Array(alloc::boxed::Box::new(
                        CoreType::Variable(crate::type_system::types::TypeVar::new(
                            9_004,
                            "T".to_owned(),
                        )),
                    ))],
                    error_types: Vec::new(),
                },
                SymbolType::Function,
            ),
            (
                String::from("clear"),
                CoreType::Function {
                    generic_params: vec![GenericTypeParameter {
                        name: "T".to_owned(),
                        type_var: crate::type_system::types::TypeVar::new(9_005, "T".to_owned()),
                        constraints: Vec::new(),
                    }],
                    parameters: vec![CoreType::Array(alloc::boxed::Box::new(CoreType::Variable(
                        crate::type_system::types::TypeVar::new(9_005, "T".to_owned()),
                    )))],
                    return_types: vec![CoreType::Array(alloc::boxed::Box::new(
                        CoreType::Variable(crate::type_system::types::TypeVar::new(
                            9_005,
                            "T".to_owned(),
                        )),
                    ))],
                    error_types: Vec::new(),
                },
                SymbolType::Function,
            ),
            (
                String::from("StdoutWriter"),
                CoreType::Generic {
                    name: String::from("StdoutWriter"),
                    type_args: Vec::new(),
                },
                SymbolType::Type,
            ),
            (
                String::from("StdoutTerminal"),
                CoreType::Generic {
                    name: String::from("StdoutTerminal"),
                    type_args: Vec::new(),
                },
                SymbolType::Type,
            ),
            (
                String::from("FlushFailureError"),
                CoreType::Generic {
                    name: String::from("FlushFailureError"),
                    type_args: Vec::new(),
                },
                SymbolType::Type,
            ),
            (
                String::from("SinkClosedError"),
                CoreType::Generic {
                    name: String::from("SinkClosedError"),
                    type_args: Vec::new(),
                },
                SymbolType::Type,
            ),
            (
                String::from("TerminalWriteFailureError"),
                CoreType::Generic {
                    name: String::from("TerminalWriteFailureError"),
                    type_args: Vec::new(),
                },
                SymbolType::Type,
            ),
            (
                String::from("InvalidCursorPositionError"),
                CoreType::Generic {
                    name: String::from("InvalidCursorPositionError"),
                    type_args: Vec::new(),
                },
                SymbolType::Type,
            ),
            (
                String::from("InvalidDurationError"),
                CoreType::Generic {
                    name: String::from("InvalidDurationError"),
                    type_args: Vec::new(),
                },
                SymbolType::Type,
            ),
            (
                String::from("FrameClock"),
                CoreType::Generic {
                    name: String::from("FrameClock"),
                    type_args: Vec::new(),
                },
                SymbolType::Type,
            ),
            (
                String::from("StringBuilder"),
                CoreType::Generic {
                    name: String::from("StringBuilder"),
                    type_args: Vec::new(),
                },
                SymbolType::Type,
            ),
            (
                String::from("BuilderFinishedError"),
                CoreType::Generic {
                    name: String::from("BuilderFinishedError"),
                    type_args: Vec::new(),
                },
                SymbolType::Type,
            ),
            (
                String::from("AllocationFailureError"),
                CoreType::Generic {
                    name: String::from("AllocationFailureError"),
                    type_args: Vec::new(),
                },
                SymbolType::Type,
            ),
            (
                String::from("InvalidFrameRateError"),
                CoreType::Generic {
                    name: String::from("InvalidFrameRateError"),
                    type_args: Vec::new(),
                },
                SymbolType::Type,
            ),
        ]
    };
}

/// Core runtime, conversion, bytes, and foundational file-I/O builtins.
pub(super) fn standard_symbols_core_io_and_bytes() -> Vec<(String, CoreType, SymbolType)> {
    let mut symbols = standard_symbols_core_io_and_bytes_vec!();
    symbols.extend(standard_symbols_bytes_and_foundational_filesystem());
    symbols
}
