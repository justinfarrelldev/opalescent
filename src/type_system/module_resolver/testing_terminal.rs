//! Synchronous explicit production-executable PTY support, restricted to tests.
use crate::ast::BorrowKind;
use crate::type_system::{
    module_resolver::{ModuleInterface, ModuleResolver},
    symbol_table::{SymbolType, Visibility},
    types::CoreType,
};

/// Add application descriptors and terminal drivers to the testing interface.
pub(super) fn register(interface: &mut ModuleInterface) {
    for name in ["ApplicationExecutable", "TerminalProcess"] {
        interface
            .register_symbol(ModuleResolver::module_symbol(
                name.to_owned(),
                SymbolType::Type,
                named(name),
                Visibility::Public,
            ))
            .expect("unique testing type");
    }
    let process = named("TerminalProcess");
    let specs = [
        (
            "application_executable",
            vec![],
            named("ApplicationExecutable"),
            vec![],
            vec![],
        ),
        (
            "terminal_process_launch",
            vec![
                named("ApplicationExecutable"),
                CoreType::String,
                CoreType::String,
                CoreType::Int64,
                CoreType::Int64,
            ],
            process.clone(),
            vec![named("CreateFailureError"), named("AllocationFailureError")],
            vec![BorrowKind::Owned; 5],
        ),
        (
            "terminal_process_send_text",
            vec![process.clone(), CoreType::String],
            CoreType::Unit,
            vec![named("WriteFailureError")],
            vec![BorrowKind::MutableRef, BorrowKind::Owned],
        ),
        (
            "terminal_process_send_key",
            vec![process.clone(), CoreType::String],
            CoreType::Unit,
            vec![named("WriteFailureError")],
            vec![BorrowKind::MutableRef, BorrowKind::Owned],
        ),
        (
            "terminal_process_row",
            vec![process.clone(), CoreType::Int64],
            CoreType::String,
            vec![],
            vec![BorrowKind::Ref, BorrowKind::Owned],
        ),
        (
            "terminal_process_cursor_line",
            vec![process.clone()],
            CoreType::Int64,
            vec![],
            vec![BorrowKind::Ref],
        ),
        (
            "terminal_process_cursor_column",
            vec![process.clone()],
            CoreType::Int64,
            vec![],
            vec![BorrowKind::Ref],
        ),
        (
            "terminal_process_exited",
            vec![process.clone()],
            CoreType::Boolean,
            vec![],
            vec![BorrowKind::Ref],
        ),
        (
            "terminal_process_exit_code",
            vec![process.clone()],
            CoreType::Int64,
            vec![],
            vec![BorrowKind::Ref],
        ),
        (
            "terminal_process_transcript",
            vec![process.clone()],
            CoreType::String,
            vec![],
            vec![BorrowKind::Ref],
        ),
        (
            "terminal_process_stop",
            vec![process],
            CoreType::Unit,
            vec![named("DeleteFailureError")],
            vec![BorrowKind::MutableRef],
        ),
    ];
    for spec in specs {
        register_function(interface, spec);
    }
}

/// Function name, logical parameters/result/errors and source borrowing contract.
type Specification = (
    &'static str,
    Vec<CoreType>,
    CoreType,
    Vec<CoreType>,
    Vec<BorrowKind>,
);

/// Publish one checked logical signature and its matching source borrowing modes.
fn register_function(interface: &mut ModuleInterface, spec: Specification) {
    let (name, parameters, result, error_types, borrows) = spec;
    interface
        .register_symbol(ModuleResolver::module_symbol(
            name.to_owned(),
            SymbolType::Function,
            CoreType::Function {
                generic_params: vec![],
                parameters,
                return_types: vec![result],
                error_types,
            },
            Visibility::Public,
        ))
        .expect("unique terminal testing function");
    interface.register_function_borrow_kinds(name.to_owned(), borrows);
}

/// Opaque runtime handles share canonical built-in names across source imports.
fn named(name: &str) -> CoreType {
    CoreType::Generic {
        name: name.to_owned(),
        type_args: vec![],
    }
}
