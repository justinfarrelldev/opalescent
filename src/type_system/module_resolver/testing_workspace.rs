//! Explicit test-only workspaces reuse ordinary filesystem values and errors.
use crate::ast::BorrowKind;
use crate::type_system::module_resolver::{ModuleAvailability, ModuleInterface, ModuleResolver};
use crate::type_system::{
    symbol_table::{SymbolType, Visibility},
    types::CoreType,
};

/// Register the minimal public temporary-directory contract used by native suites.
pub(super) fn register(resolver: &mut ModuleResolver) {
    let mut interface = ModuleInterface::with_availability(
        "standard.testing".to_owned(),
        ModuleAvailability::TestOnly,
    );
    let directory = named("TemporaryDirectory");
    interface
        .register_symbol(ModuleResolver::module_symbol(
            "TemporaryDirectory".to_owned(),
            SymbolType::Type,
            directory.clone(),
            Visibility::Public,
        ))
        .expect("unique testing type");
    let specs = [
        (
            "temporary_directory_create",
            vec![],
            directory.clone(),
            vec![named("CreateFailureError"), named("AllocationFailureError")],
            vec![],
        ),
        (
            "temporary_directory_file",
            vec![directory.clone(), CoreType::String],
            CoreType::String,
            vec![],
            vec![BorrowKind::Ref, BorrowKind::Owned],
        ),
        (
            "temporary_directory_remove",
            vec![directory],
            CoreType::Unit,
            vec![
                named("DeleteFailureError"),
                named("PermissionDeniedError"),
                named("InvalidPathError"),
                named("DirectoryNotFoundError"),
                named("IsNotADirectoryError"),
                named("DirectoryNotEmptyError"),
            ],
            vec![BorrowKind::MutableRef],
        ),
    ];
    for (name, parameters, result, error_types, borrows) in specs {
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
            .expect("unique testing function");
        interface.register_function_borrow_kinds(name.to_owned(), borrows);
    }
    resolver.register_module_interface(interface);
}

/// Built-in nominal identities are shared with the native runtime ABI.
fn named(name: &str) -> CoreType {
    CoreType::Generic {
        name: name.to_owned(),
        type_args: vec![],
    }
}
