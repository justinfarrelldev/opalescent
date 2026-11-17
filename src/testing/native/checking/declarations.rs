//! Forward declarations and later state layouts for generated nominal handles.
extern crate alloc;
use super::{LoadedModule, NativeCheckError, invalid, type_error};
use crate::ast::{AstNode, Expr};
use crate::testing::native::doubles::ResolvedDoubleSet;
use crate::testing::native::identity::{DoubleIdentity, WorldIdentity};
use crate::testing::native::world_resolution::ResolvedWorldSet;
use crate::type_system::checker::TypeChecker;
use crate::type_system::types::CoreType;
use alloc::collections::BTreeMap;

/// Declare aliases independently of state values and application signatures.
pub(super) fn declare_aliases(
    module: &LoadedModule,
    checker: &mut TypeChecker,
    worlds: &ResolvedWorldSet,
    doubles: &ResolvedDoubleSet,
    world_types: &mut BTreeMap<WorldIdentity, CoreType>,
    double_types: &mut BTreeMap<DoubleIdentity, CoreType>,
) -> Result<(), NativeCheckError> {
    if let Some(bindings) = worlds.bindings.get(&module.path) {
        for (local, identity) in bindings {
            let nominal = world_name(identity);
            let core_type = checker
                .native_declare_world_alias(
                    local,
                    &nominal,
                    worlds.declarations[identity].declaration.span,
                )
                .map_err(|error| type_error(&module.path, error))?;
            world_types.insert(identity.clone(), core_type);
        }
    }
    if let Some(bindings) = doubles.bindings.get(&module.path) {
        for (local, identity) in bindings {
            let nominal = double_name(identity);
            let core_type = checker
                .native_declare_double_alias(
                    local,
                    &nominal,
                    doubles.declarations[identity].declaration.span,
                )
                .map_err(|error| type_error(&module.path, error))?;
            double_types.insert(identity.clone(), core_type);
        }
    }
    Ok(())
}

/// Resolve concrete state annotations after all source type imports/aliases.
pub(super) fn state_types(
    worlds: &ResolvedWorldSet,
    modules: &BTreeMap<std::path::PathBuf, super::CheckedNativeModule>,
) -> Result<BTreeMap<WorldIdentity, CoreType>, NativeCheckError> {
    let mut states = BTreeMap::new();
    for world in worlds.declarations.values() {
        let checker = &modules[&world.module_path].checker;
        let Expr::Lambda {
            ref return_types, ..
        } = world.declaration.setup.function
        else {
            return Err(type_error(
                &world.module_path,
                invalid(
                    world.declaration.span,
                    "World setup requires an ordinary typed function",
                ),
            ));
        };
        if return_types.len() != 1 {
            return Err(type_error(
                &world.module_path,
                invalid(
                    world.declaration.setup.function.span(),
                    "World setup returns exactly one concrete state value, not labeled multiple returns",
                ),
            ));
        }
        let state = checker
            .native_source_core_type(&return_types[0])
            .map_err(|error| type_error(&world.module_path, error))?;
        if state == CoreType::Unit {
            return Err(type_error(
                &world.module_path,
                invalid(
                    world.declaration.span,
                    "World setup must return a concrete state value",
                ),
            ));
        }
        states.insert(world.identity.clone(), state);
    }
    Ok(states)
}

/// Length-delimited tuple encoding, independent of aliases, lines or addresses.
pub(super) fn world_name(identity: &WorldIdentity) -> String {
    format!(
        "__opal_world_{}:{}{}:{}",
        identity.0.module.len(),
        identity.0.module,
        identity.0.name.len(),
        identity.0.name
    )
}

/// Double identities remain distinct even when two declarations target one edge.
pub(super) fn double_name(identity: &DoubleIdentity) -> String {
    format!(
        "__opal_double_{}:{}{}:{}",
        identity.0.module.len(),
        identity.0.module,
        identity.0.name.len(),
        identity.0.name
    )
}
