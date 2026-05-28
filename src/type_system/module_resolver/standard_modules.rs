extern crate alloc;

use super::ModuleInterface;
use super::ModuleResolver;
use super::standard_symbols_core_io_and_bytes::standard_symbols_core_io_and_bytes;
use super::standard_symbols_filesystem_operations::standard_symbols_filesystem_operations;
use super::standard_symbols_filesystem_types_and_errors::standard_symbols_filesystem_types_and_errors;
use super::standard_symbols_process::standard_symbols_process;
use crate::type_system::symbol_table::{SymbolType, Visibility};
use crate::type_system::types::CoreType;
use alloc::collections::BTreeMap;
use alloc::{string::String, vec::Vec};

/// Register built-in module interfaces used by imports.
pub(super) fn register_standard_modules(resolver: &mut ModuleResolver) {
    register_standard_module(resolver);
    register_math_module(resolver);
    register_process_module(resolver);
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
