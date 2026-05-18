//! Stdout text stdlib built-in registration.
//!
//! Registers the additive fallible stdout text APIs together with the nominal
//! error types they expose to Opalescent source code.

extern crate alloc;

use crate::token::Span;
use crate::type_system::checker::TypeChecker;
use crate::type_system::symbol_table::{SymbolInfo, SymbolType, Visibility};
use crate::type_system::types::CoreType;
use alloc::borrow::ToOwned;
use alloc::string::String;
use alloc::{vec, vec::Vec};

/// Interned error name for stdout write failures.
const WRITE_FAILURE_ERROR: &str = "WriteFailureError";
/// Interned error name for stdout flush failures.
const FLUSH_FAILURE_ERROR: &str = "FlushFailureError";
/// Interned error name for closed stdout sinks.
const SINK_CLOSED_ERROR: &str = "SinkClosedError";
/// Interned error name for terminal write failures.
const TERMINAL_WRITE_FAILURE_ERROR: &str = "TerminalWriteFailureError";
/// Interned error name for invalid terminal cursor positions.
const INVALID_CURSOR_POSITION_ERROR: &str = "InvalidCursorPositionError";

/// Opaque nominal type name for the shared stdout writer handle.
const STDOUT_WRITER_TYPE_NAME: &str = "StdoutWriter";
/// Opaque nominal type name for the shared stdout terminal handle.
const STDOUT_TERMINAL_TYPE_NAME: &str = "StdoutTerminal";

impl TypeChecker {
    /// Register additive fallible stdout text APIs and their nominal errors.
    pub(super) fn register_stdout_text_builtins(&mut self) {
        self.register_stdout_text_nominal_types();
        self.register_stdout_text_error_types();
        self.register_stdout_text_writer_builtins();
        self.register_stdout_terminal_builtins();
    }

    /// Register stdout text, writer, and flush builtins.
    fn register_stdout_text_writer_builtins(&mut self) {
        self.register_stdout_text_builtin(
            "print_text_sync",
            vec![CoreType::String],
            vec![CoreType::Unit],
            vec![
                error_core_type(WRITE_FAILURE_ERROR),
                error_core_type(SINK_CLOSED_ERROR),
            ],
        );

        self.register_stdout_text_builtin(
            "flush_standard_output_sync",
            Vec::new(),
            vec![CoreType::Unit],
            vec![
                error_core_type(FLUSH_FAILURE_ERROR),
                error_core_type(SINK_CLOSED_ERROR),
            ],
        );

        self.register_stdout_text_builtin(
            "stdout_writer",
            Vec::new(),
            vec![stdout_writer_core_type()],
            Vec::new(),
        );

        self.register_stdout_text_builtin(
            "writer_write_sync",
            vec![stdout_writer_core_type(), CoreType::String],
            vec![CoreType::Unit],
            vec![
                error_core_type(WRITE_FAILURE_ERROR),
                error_core_type(SINK_CLOSED_ERROR),
            ],
        );

        self.register_stdout_text_builtin(
            "writer_flush_sync",
            vec![stdout_writer_core_type()],
            vec![CoreType::Unit],
            vec![
                error_core_type(FLUSH_FAILURE_ERROR),
                error_core_type(SINK_CLOSED_ERROR),
            ],
        );
    }

    /// Register stdout terminal builtins.
    fn register_stdout_terminal_builtins(&mut self) {
        self.register_stdout_text_builtin(
            "stdout_terminal",
            Vec::new(),
            vec![stdout_terminal_core_type()],
            Vec::new(),
        );

        self.register_stdout_text_builtin(
            "terminal_supports_ansi",
            vec![stdout_terminal_core_type()],
            vec![CoreType::Boolean],
            Vec::new(),
        );

        self.register_stdout_text_builtin(
            "terminal_clear_screen_on_sync",
            vec![stdout_terminal_core_type()],
            vec![CoreType::Unit],
            vec![
                error_core_type(TERMINAL_WRITE_FAILURE_ERROR),
                error_core_type(SINK_CLOSED_ERROR),
            ],
        );

        self.register_stdout_text_builtin(
            "terminal_move_cursor_on_sync",
            vec![
                stdout_terminal_core_type(),
                CoreType::Int32,
                CoreType::Int32,
            ],
            vec![CoreType::Unit],
            vec![
                error_core_type(TERMINAL_WRITE_FAILURE_ERROR),
                error_core_type(INVALID_CURSOR_POSITION_ERROR),
                error_core_type(SINK_CLOSED_ERROR),
            ],
        );

        self.register_stdout_text_builtin(
            "terminal_draw_rows_sync",
            vec![
                stdout_terminal_core_type(),
                CoreType::Array(alloc::boxed::Box::new(CoreType::String)),
            ],
            vec![CoreType::Unit],
            vec![
                error_core_type(TERMINAL_WRITE_FAILURE_ERROR),
                error_core_type(SINK_CLOSED_ERROR),
            ],
        );

        self.register_stdout_text_builtin(
            "terminal_clear_screen_sync",
            Vec::new(),
            vec![CoreType::Unit],
            vec![
                error_core_type(TERMINAL_WRITE_FAILURE_ERROR),
                error_core_type(SINK_CLOSED_ERROR),
            ],
        );

        self.register_stdout_text_builtin(
            "terminal_move_cursor_sync",
            vec![CoreType::Int32, CoreType::Int32],
            vec![CoreType::Unit],
            vec![
                error_core_type(TERMINAL_WRITE_FAILURE_ERROR),
                error_core_type(INVALID_CURSOR_POSITION_ERROR),
                error_core_type(SINK_CLOSED_ERROR),
            ],
        );
    }

    /// Register opaque stdout handle nominal types.
    fn register_stdout_text_nominal_types(&mut self) {
        self.environment.register_type(
            STDOUT_WRITER_TYPE_NAME.to_owned(),
            stdout_writer_core_type(),
        );
        self.environment.register_type(
            STDOUT_TERMINAL_TYPE_NAME.to_owned(),
            stdout_terminal_core_type(),
        );
    }

    /// Register stdout error nominal types.
    fn register_stdout_text_error_types(&mut self) {
        self.environment.register_type(
            WRITE_FAILURE_ERROR.to_owned(),
            error_core_type(WRITE_FAILURE_ERROR),
        );
        self.environment.register_type(
            FLUSH_FAILURE_ERROR.to_owned(),
            error_core_type(FLUSH_FAILURE_ERROR),
        );
        self.environment.register_type(
            SINK_CLOSED_ERROR.to_owned(),
            error_core_type(SINK_CLOSED_ERROR),
        );
        self.environment.register_type(
            TERMINAL_WRITE_FAILURE_ERROR.to_owned(),
            error_core_type(TERMINAL_WRITE_FAILURE_ERROR),
        );
        self.environment.register_type(
            INVALID_CURSOR_POSITION_ERROR.to_owned(),
            error_core_type(INVALID_CURSOR_POSITION_ERROR),
        );
    }

    /// Register a single stdout builtin in the environment and symbol table.
    fn register_stdout_text_builtin(
        &mut self,
        name: &str,
        parameters: Vec<CoreType>,
        return_types: Vec<CoreType>,
        error_types: Vec<CoreType>,
    ) {
        let owned_name: String = name.to_owned();
        let signature = CoreType::Function {
            generic_params: Vec::new(),
            parameters,
            return_types,
            error_types,
        };
        self.environment
            .register_builtin(owned_name.clone(), signature.clone());
        self.symbol_table.register(SymbolInfo {
            name: owned_name,
            symbol_type: SymbolType::Function,
            core_type: signature,
            visibility: Visibility::Public,
            source_location: Span::single(crate::token::Position::start()),
            is_let_binding: false,
            is_mutable: false,
            read_count: 0,
            is_pure: false,
        });
    }
}

/// Build the nominal core type for the opaque `StdoutWriter` handle.
fn stdout_writer_core_type() -> CoreType {
    CoreType::Generic {
        name: STDOUT_WRITER_TYPE_NAME.to_owned(),
        type_args: Vec::new(),
    }
}

/// Build the nominal core type for the opaque `StdoutTerminal` handle.
fn stdout_terminal_core_type() -> CoreType {
    CoreType::Generic {
        name: STDOUT_TERMINAL_TYPE_NAME.to_owned(),
        type_args: Vec::new(),
    }
}

/// Build the nominal core type for a payload-free stdout builtin error.
fn error_core_type(name: &str) -> CoreType {
    CoreType::Generic {
        name: name.to_owned(),
        type_args: Vec::new(),
    }
}
