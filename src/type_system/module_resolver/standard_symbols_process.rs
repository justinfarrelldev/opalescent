extern crate alloc;

use crate::type_system::symbol_table::SymbolType;
use crate::type_system::types::CoreType;
use alloc::{string::String, vec::Vec};

/// `process` module builtin symbol registrations.
pub(super) fn standard_symbols_process() -> Vec<(String, CoreType, SymbolType)> {
    let mut symbols = standard_symbols_process_functions();
    symbols.extend(standard_symbols_process_types());
    symbols
}

/// Returns all builtin `process` function symbols.
fn standard_symbols_process_functions() -> Vec<(String, CoreType, SymbolType)> {
    vec![
        function_symbol(
            "current_working_directory_sync",
            vec![],
            vec![filesystem_path_type()],
            vec![
                permission_denied_error(),
                invalid_path_error(),
                current_working_directory_unavailable_error(),
            ],
        ),
        function_symbol(
            "current_executable_path_sync",
            vec![],
            vec![filesystem_path_type()],
            vec![
                permission_denied_error(),
                invalid_path_error(),
                current_executable_path_unavailable_error(),
            ],
        ),
        function_symbol(
            "current_executable_directory_sync",
            vec![],
            vec![filesystem_path_type()],
            vec![
                permission_denied_error(),
                invalid_path_error(),
                current_executable_path_unavailable_error(),
            ],
        ),
        function_symbol(
            "set_current_working_directory_sync",
            vec![filesystem_path_type()],
            vec![CoreType::Unit],
            vec![
                file_not_found_error(),
                permission_denied_error(),
                is_not_a_directory_error(),
                invalid_path_error(),
            ],
        ),
        function_symbol(
            "get_environment_variable",
            vec![CoreType::String],
            vec![CoreType::String],
            vec![
                environment_variable_not_found_error(),
                invalid_environment_variable_name_error(),
                invalid_utf8_error(),
            ],
        ),
        function_symbol(
            "get_environment_variable_or",
            vec![CoreType::String, CoreType::String],
            vec![CoreType::String],
            vec![invalid_environment_variable_name_error(), invalid_utf8_error()],
        ),
        function_symbol(
            "environment_variable_exists",
            vec![CoreType::String],
            vec![CoreType::Boolean],
            vec![invalid_environment_variable_name_error()],
        ),
        function_symbol("exit_process", vec![CoreType::Int32], vec![CoreType::Unit], vec![]),
    ]
}

/// Returns all exported `process`-module error types.
fn standard_symbols_process_types() -> Vec<(String, CoreType, SymbolType)> {
    vec![
        type_symbol(current_working_directory_unavailable_error()),
        type_symbol(current_executable_path_unavailable_error()),
        type_symbol(environment_variable_not_found_error()),
        type_symbol(invalid_environment_variable_name_error()),
    ]
}

/// Constructs a function symbol triple for the module resolver.
fn function_symbol(
    name: &str,
    parameters: Vec<CoreType>,
    return_types: Vec<CoreType>,
    error_types: Vec<CoreType>,
) -> (String, CoreType, SymbolType) {
    (
        String::from(name),
        CoreType::Function {
            generic_params: Vec::new(),
            parameters,
            return_types,
            error_types,
        },
        SymbolType::Function,
    )
}

/// Constructs a type symbol triple for the module resolver.
fn type_symbol(core_type: CoreType) -> (String, CoreType, SymbolType) {
    let name = match core_type {
        CoreType::Generic { ref name, .. } => name.clone(),
        _ => unreachable!(),
    };

    (name, core_type, SymbolType::Type)
}

/// Returns the standard `FilesystemPath` type.
fn filesystem_path_type() -> CoreType {
    generic_type("FilesystemPath")
}

/// Returns the standard `PermissionDeniedError` type.
fn permission_denied_error() -> CoreType {
    generic_type("PermissionDeniedError")
}

/// Returns the standard `InvalidPathError` type.
fn invalid_path_error() -> CoreType {
    generic_type("InvalidPathError")
}

/// Returns the standard `CurrentWorkingDirectoryUnavailableError` type.
fn current_working_directory_unavailable_error() -> CoreType {
    generic_type("CurrentWorkingDirectoryUnavailableError")
}

/// Returns the standard `CurrentExecutablePathUnavailableError` type.
fn current_executable_path_unavailable_error() -> CoreType {
    generic_type("CurrentExecutablePathUnavailableError")
}

/// Returns the standard `FileNotFoundError` type.
fn file_not_found_error() -> CoreType {
    generic_type("FileNotFoundError")
}

/// Returns the standard `IsNotADirectoryError` type.
fn is_not_a_directory_error() -> CoreType {
    generic_type("IsNotADirectoryError")
}

/// Returns the standard `EnvironmentVariableNotFoundError` type.
fn environment_variable_not_found_error() -> CoreType {
    generic_type("EnvironmentVariableNotFoundError")
}

/// Returns the standard `InvalidEnvironmentVariableNameError` type.
fn invalid_environment_variable_name_error() -> CoreType {
    generic_type("InvalidEnvironmentVariableNameError")
}

/// Returns the standard `InvalidUtf8Error` type.
fn invalid_utf8_error() -> CoreType {
    generic_type("InvalidUtf8Error")
}

/// Constructs a generic zero-argument core type by name.
fn generic_type(name: &str) -> CoreType {
    CoreType::Generic {
        name: String::from(name),
        type_args: Vec::new(),
    }
}
