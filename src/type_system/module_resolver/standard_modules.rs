extern crate alloc;

use super::ModuleInterface;
use super::ModuleResolver;
use super::standard_symbols_core_io_and_bytes::standard_symbols_core_io_and_bytes;
use super::standard_symbols_filesystem_operations::standard_symbols_filesystem_operations;
use super::standard_symbols_filesystem_types_and_errors::standard_symbols_filesystem_types_and_errors;
use super::standard_symbols_process::standard_symbols_process;
use super::terminal_proposal_modules::register_terminal_proposal_modules;
use super::terminal_proposal_symbols::register_terminal_proposal_symbols;
use crate::token::{Position, Span};
use crate::type_system::symbol_table::{SymbolInfo, SymbolType, Visibility};
use crate::type_system::types::CoreType;
use alloc::collections::{BTreeMap, BTreeSet};
use alloc::{string::String, vec::Vec};

/// Register built-in module interfaces used by imports.
pub(super) fn register_standard_modules(resolver: &mut ModuleResolver) {
    register_math_module(resolver);
    register_process_module(resolver);
    register_core_prerequisite_module(resolver);
    register_terminal_proposal_modules(resolver);
    register_standard_module(resolver);
}

/// Register `standard` built-in module symbols.
fn register_standard_module(resolver: &mut ModuleResolver) {
    let mut interface = ModuleInterface::new(String::from("standard"));
    let mut standard_symbols = standard_symbols_core_io_and_bytes();
    standard_symbols.extend(standard_symbols_filesystem_operations());
    standard_symbols.extend(standard_symbols_filesystem_types_and_errors());

    for (name, core_type, symbol_type) in standard_symbols {
        let register_result = interface.register_symbol(ModuleResolver::module_symbol(
            name,
            symbol_type,
            core_type,
            Visibility::Public,
        ));
        if register_result.is_err() {
            return;
        }
    }
    register_standard_terminal_reexports(resolver, &mut interface);
    resolver.register_module_interface(interface);

    let mut fs_path_fields = BTreeMap::new();
    fs_path_fields.insert(String::from("raw"), CoreType::String);
    resolver.register_adt_fields_for_module(
        "standard",
        String::from("FilesystemPath"),
        fs_path_fields,
    );

    let mut fs_meta_fields = BTreeMap::new();
    fs_meta_fields.insert(String::from("size_bytes"), CoreType::Int64);
    fs_meta_fields.insert(String::from("is_directory"), CoreType::Boolean);
    fs_meta_fields.insert(String::from("is_symlink"), CoreType::Boolean);
    fs_meta_fields.insert(String::from("modified_unix_seconds"), CoreType::Int64);
    resolver.register_adt_fields_for_module(
        "standard",
        String::from("FileMetadata"),
        fs_meta_fields,
    );

    let mut fs_perms_fields = BTreeMap::new();
    fs_perms_fields.insert(String::from("readable"), CoreType::Boolean);
    fs_perms_fields.insert(String::from("writable"), CoreType::Boolean);
    fs_perms_fields.insert(String::from("executable"), CoreType::Boolean);
    resolver.register_adt_fields_for_module(
        "standard",
        String::from("FilePermissions"),
        fs_perms_fields,
    );
}

/// Register `process` built-in module symbols.
fn register_process_module(resolver: &mut ModuleResolver) {
    let mut interface = ModuleInterface::new(String::from("process"));
    let process_symbols = standard_symbols_process();

    for (name, core_type, symbol_type) in process_symbols {
        let register_result = interface.register_symbol(ModuleResolver::module_symbol(
            name,
            symbol_type,
            core_type,
            Visibility::Public,
        ));
        if register_result.is_err() {
            return;
        }
    }
    resolver.register_module_interface(interface);
}

/// Register future-gated core/system prerequisite symbols.
fn register_core_prerequisite_module(resolver: &mut ModuleResolver) {
    let mut interface = ModuleInterface::with_availability(
        String::from("standard.system"),
        super::ModuleAvailability::FuturePublicApi,
    );
    register_terminal_proposal_symbols(&mut interface);
    register_nominal_type_symbols_for_interface_function_surfaces(&mut interface);
    resolver.register_module_interface(interface);
}

/// Re-export selected terminal proposal symbols/types through the root `standard` namespace.
fn register_standard_terminal_reexports(
    resolver: &ModuleResolver,
    interface: &mut ModuleInterface,
) {
    for module_path in [
        "standard.system",
        "standard.terminal",
        "standard.terminal.chords",
    ] {
        let Some(source_interface) = resolver.module_interface(module_path) else {
            continue;
        };

        for symbol in source_interface.exports.values() {
            if interface.exports.contains_key(symbol.name.as_str()) {
                continue;
            }
            let _registered = interface.register_symbol(symbol.clone());
        }
        for (owner, fields) in source_interface.adt_fields {
            interface.adt_fields.entry(owner).or_insert(fields);
        }
        for (symbol_name, labels) in source_interface.function_return_labels {
            interface
                .function_return_labels
                .entry(symbol_name)
                .or_insert(labels);
        }
        for (symbol_name, borrow_kinds) in source_interface.function_borrow_kinds {
            interface
                .function_borrow_kinds
                .entry(symbol_name)
                .or_insert(borrow_kinds);
        }
        for declaration in source_interface.type_declarations.into_values() {
            interface
                .type_declarations
                .entry(declaration.name.clone())
                .or_insert(declaration);
        }
    }

    register_nominal_type_symbols_for_interface_function_surfaces(interface);
}

/// Add public type symbols for nominal types that appear in function surfaces only.
fn register_nominal_type_symbols_for_interface_function_surfaces(interface: &mut ModuleInterface) {
    let mut names = BTreeSet::new();
    for symbol in interface.exports.values() {
        collect_nominal_type_names(&symbol.core_type, &mut names);
    }

    for name in names {
        if interface.exports.contains_key(name.as_str()) {
            continue;
        }
        let result = interface.register_symbol(SymbolInfo {
            name: name.clone(),
            symbol_type: SymbolType::Type,
            core_type: CoreType::Generic {
                name,
                type_args: Vec::new(),
            },
            visibility: Visibility::Public,
            source_location: Span::single(Position::start()),
            is_let_binding: false,
            is_mutable: false,
            read_count: 0,
            is_pure: true,
        });
        if result.is_err() {
            return;
        }
    }
}

/// Collect nominal type names referenced by a core type tree.
fn collect_nominal_type_names(core_type: &CoreType, names: &mut BTreeSet<String>) {
    match core_type {
        &CoreType::Generic {
            ref name,
            ref type_args,
        } => {
            names.insert(name.clone());
            for type_arg in type_args {
                collect_nominal_type_names(type_arg, names);
            }
        }
        &CoreType::Array(ref element) => collect_nominal_type_names(element.as_ref(), names),
        &CoreType::Function {
            ref generic_params,
            ref parameters,
            ref return_types,
            ref error_types,
        } => {
            for generic_param in generic_params {
                for constraint in &generic_param.constraints {
                    collect_nominal_type_names(constraint, names);
                }
            }
            for parameter in parameters {
                collect_nominal_type_names(parameter, names);
            }
            for return_type in return_types {
                collect_nominal_type_names(return_type, names);
            }
            for error_type in error_types {
                collect_nominal_type_names(error_type, names);
            }
        }
        &CoreType::Variable(_)
        | &CoreType::Int8
        | &CoreType::Int16
        | &CoreType::Int32
        | &CoreType::Int64
        | &CoreType::UInt8
        | &CoreType::UInt16
        | &CoreType::UInt32
        | &CoreType::UInt64
        | &CoreType::Float32
        | &CoreType::Float64
        | &CoreType::Boolean
        | &CoreType::String
        | &CoreType::Unit => {}
    }
}

/// Register `math` built-in module symbols.
fn register_math_module(resolver: &mut ModuleResolver) {
    let mut interface = ModuleInterface::new(String::from("math"));
    let math_symbols = [
        (
            String::from("random_int32"),
            CoreType::Function {
                generic_params: Vec::new(),
                parameters: vec![CoreType::Int64, CoreType::Int64],
                return_types: vec![CoreType::Int64],
                error_types: Vec::new(),
            },
        ),
        (
            String::from("random_int64"),
            CoreType::Function {
                generic_params: Vec::new(),
                parameters: vec![CoreType::Int64, CoreType::Int64],
                return_types: vec![CoreType::Int64],
                error_types: Vec::new(),
            },
        ),
        (
            String::from("sqrt"),
            CoreType::Function {
                generic_params: Vec::new(),
                parameters: vec![CoreType::Float64],
                return_types: vec![CoreType::Float64],
                error_types: Vec::new(),
            },
        ),
        (
            String::from("abs"),
            CoreType::Function {
                generic_params: Vec::new(),
                parameters: vec![CoreType::Int32],
                return_types: vec![CoreType::Int32],
                error_types: Vec::new(),
            },
        ),
        (
            String::from("sin"),
            CoreType::Function {
                generic_params: Vec::new(),
                parameters: vec![CoreType::Float64],
                return_types: vec![CoreType::Float64],
                error_types: Vec::new(),
            },
        ),
        (
            String::from("cos"),
            CoreType::Function {
                generic_params: Vec::new(),
                parameters: vec![CoreType::Float64],
                return_types: vec![CoreType::Float64],
                error_types: Vec::new(),
            },
        ),
    ];

    for (name, core_type) in math_symbols {
        let register_result = interface.register_symbol(ModuleResolver::module_symbol(
            name,
            SymbolType::Function,
            core_type,
            Visibility::Public,
        ));
        if register_result.is_err() {
            return;
        }
    }
    resolver.register_module_interface(interface);
}
