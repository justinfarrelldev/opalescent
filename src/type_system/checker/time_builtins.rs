//! Time stdlib built-in registration.
//!
//! Registers the additive blocking sleep and frame clock APIs plus their nominal error types.

extern crate alloc;

use crate::token::Span;
use crate::type_system::checker::TypeChecker;
use crate::type_system::symbol_table::{SymbolInfo, SymbolType, Visibility};
use crate::type_system::types::CoreType;
use alloc::borrow::ToOwned;
use alloc::string::String;
use alloc::{vec, vec::Vec};

/// Interned error name for invalid sleep durations.
const INVALID_DURATION_ERROR: &str = "InvalidDurationError";
/// Interned error name for invalid frame rates.
const INVALID_FRAME_RATE_ERROR: &str = "InvalidFrameRateError";
/// Opaque nominal type name for the shared `FrameClock` handle.
const FRAME_CLOCK_TYPE_NAME: &str = "FrameClock";

impl TypeChecker {
    /// Register additive time built-ins and their nominal error types.
    pub(super) fn register_time_builtins(&mut self) {
        self.environment.register_type(
            INVALID_DURATION_ERROR.to_owned(),
            error_core_type(INVALID_DURATION_ERROR),
        );
        self.environment.register_type(
            INVALID_FRAME_RATE_ERROR.to_owned(),
            error_core_type(INVALID_FRAME_RATE_ERROR),
        );
        self.environment
            .register_type(FRAME_CLOCK_TYPE_NAME.to_owned(), frame_clock_core_type());

        self.register_time_builtin(
            "sleep_ms_sync",
            vec![CoreType::Int32],
            vec![CoreType::Unit],
            vec![error_core_type(INVALID_DURATION_ERROR)],
        );
        self.register_time_builtin(
            "frame_clock_new",
            vec![CoreType::Int32],
            vec![frame_clock_core_type()],
            vec![error_core_type(INVALID_FRAME_RATE_ERROR)],
        );
        self.register_time_builtin(
            "frame_clock_wait_next_sync",
            vec![frame_clock_core_type()],
            vec![CoreType::Unit],
            vec![error_core_type(INVALID_FRAME_RATE_ERROR)],
        );
    }

    /// Register a single time builtin in the environment and symbol table.
    fn register_time_builtin(
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

/// Build the nominal core type for the opaque `FrameClock` handle.
fn frame_clock_core_type() -> CoreType {
    CoreType::Generic {
        name: FRAME_CLOCK_TYPE_NAME.to_owned(),
        type_args: Vec::new(),
    }
}

/// Build the nominal core type for a payload-free time builtin error.
fn error_core_type(name: &str) -> CoreType {
    CoreType::Generic {
        name: name.to_owned(),
        type_args: Vec::new(),
    }
}
