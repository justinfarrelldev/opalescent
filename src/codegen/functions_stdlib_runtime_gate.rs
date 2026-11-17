//! Shared diagnostic for public terminal declarations not yet runtime-lowered.
use crate::codegen::error::CodegenError;

/// Return the explicit runtime-readiness diagnostic for proposal imports.
pub fn terminal_proposal_import_gate_error(
    module_name: &str,
    symbol_name: &str,
) -> Option<CodegenError> {
    crate::type_system::is_terminal_proposal_codegen_gated_import(module_name, symbol_name)
        .then(|| diagnostic(Some(module_name), symbol_name))
}

/// Return the explicit runtime-readiness diagnostic for proposal call resolution.
pub fn terminal_proposal_runtime_gate_error(symbol_name: &str) -> Option<CodegenError> {
    crate::type_system::is_terminal_proposal_codegen_gated_runtime_name(symbol_name)
        .then(|| diagnostic(None, symbol_name))
}

/// Preserve one consistent readiness diagnostic for imports and direct calls.
fn diagnostic(module_name: Option<&str>, symbol_name: &str) -> CodegenError {
    let location = module_name.map_or_else(
        || format!("runtime symbol '{symbol_name}'"),
        |module| format!("'{symbol_name}' from module '{module}'"),
    );
    CodegenError::new(format!(
        "terminal public API prerequisites are satisfied, but runtime lowering for {location} is not implemented yet"
    ))
}
