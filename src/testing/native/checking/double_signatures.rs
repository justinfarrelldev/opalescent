//! Concrete callable contracts for canonical, pre-optimization import edges.
extern crate alloc;
use super::{NativeCheckError, NativeTestProject, invalid, type_error};
use crate::ast::BorrowKind;
use crate::testing::native::doubles::ResolvedDoubleSet;
use crate::testing::native::identity::DoubleIdentity;
use crate::type_system::checker::TypeChecker;
use crate::type_system::types::CoreType;
use crate::type_system::{ModuleInterface, NativeDoubleSignature};
use alloc::collections::BTreeMap;
use std::path::PathBuf;

/// Resolve genuine declarations through normal compiler interfaces, preserving
/// nominal application errors and the existing reference/multi-return ABI.
pub(super) fn resolve(
    project: &NativeTestProject,
    doubles: &ResolvedDoubleSet,
    interfaces: &BTreeMap<PathBuf, ModuleInterface>,
) -> Result<BTreeMap<DoubleIdentity, NativeDoubleSignature>, NativeCheckError> {
    let mut signatures = BTreeMap::new();
    for double in doubles.declarations.values() {
        let function = &double.edge.imported_function.0;
        let interface = project
            .modules
            .values()
            .find(|module| module.identity == function.module)
            .map_or_else(
                || TypeChecker::new().module_interface(&function.module),
                |source| interfaces.get(&source.path).cloned(),
            );
        let interface = interface.ok_or_else(|| {
            type_error(
                &double.module_path,
                invalid(
                    double.declaration.span,
                    "double target has no checked application module interface",
                ),
            )
        })?;
        let symbol = interface.exports.get(&function.name).ok_or_else(|| {
            type_error(
                &double.module_path,
                invalid(
                    double.declaration.span,
                    "double target is not a public callable declaration",
                ),
            )
        })?;
        let CoreType::Function {
            ref generic_params,
            ref parameters,
            ref return_types,
            ref error_types,
        } = symbol.core_type
        else {
            return Err(type_error(
                &double.module_path,
                invalid(double.declaration.span, "double target is not callable"),
            ));
        };
        if !generic_params.is_empty() {
            return Err(type_error(
                &double.module_path,
                invalid(
                    double.declaration.span,
                    "double boundaries require a concrete callable signature; an uninstantiated generic or indirect boundary is unsupported",
                ),
            ));
        }
        let parameter_modes = interface.function_borrow_kinds(&function.name).map_or_else(
            || vec![BorrowKind::Owned; parameters.len()],
            <[BorrowKind]>::to_vec,
        );
        signatures.insert(
            double.identity.clone(),
            NativeDoubleSignature {
                parameters: parameters.clone(),
                parameter_modes,
                return_types: return_types.clone(),
                error_types: error_types.clone(),
            },
        );
    }
    Ok(signatures)
}
