//! Union body, activation, hook and transitive World needs with honest origins.
extern crate alloc;
use super::{EffectiveMetadata, HookScope, WorldUseOrigin, case_error};
use crate::ast::{BorrowKind, Expr, Type};
use crate::testing::native::identity::WorldIdentity;
use crate::testing::native::world_resolution::ResolvedWorldSet;
use crate::token::Span;
use crate::type_system::errors::TypeError;
use alloc::collections::{BTreeMap, BTreeSet};
use std::path::Path;

/// World ordering and origin map for a single source body, shared by examples.
type WorldNeeds = (
    Vec<WorldIdentity>,
    BTreeMap<WorldIdentity, BTreeSet<WorldUseOrigin>>,
);

/// Resolve the complete lifecycle graph without altering body injection order.
pub(super) fn resolve(
    worlds: &ResolvedWorldSet,
    module_path: &Path,
    metadata: &EffectiveMetadata,
    scopes: &[HookScope],
    span: Span,
) -> Result<WorldNeeds, TypeError> {
    let mut origins: BTreeMap<WorldIdentity, BTreeSet<WorldUseOrigin>> = BTreeMap::new();
    for identity in &metadata.worlds {
        origins
            .entry(identity.clone())
            .or_default()
            .insert(WorldUseOrigin::Injected);
    }
    for identity in &metadata.activated {
        origins
            .entry(identity.clone())
            .or_default()
            .insert(WorldUseOrigin::Activated);
    }
    for scope in scopes {
        for (index, hook) in scope.hooks.iter().enumerate() {
            let Expr::Lambda { ref params, .. } = hook.callback.function else {
                return Err(case_error(
                    hook.span,
                    "hook requires an ordinary typed callback",
                ));
            };
            let mut subscribed = BTreeSet::new();
            for (parameter_index, parameter) in params.iter().enumerate() {
                let Type::Basic { ref name, .. } = parameter.param_type else {
                    return Err(case_error(
                        parameter.span,
                        "hooks request nominal World references and an optional final test context",
                    ));
                };
                if name == "TestContext" {
                    if parameter_index.saturating_add(1) != params.len()
                        || parameter.borrow_kind != BorrowKind::MutableRef
                    {
                        return Err(case_error(
                            parameter.span,
                            "test context must be a final mutable ref parameter",
                        ));
                    }
                    continue;
                }
                let identity = worlds
                    .bindings
                    .get(module_path)
                    .and_then(|bindings| bindings.get(name))
                    .ok_or_else(|| {
                        case_error(
                            parameter.span,
                            &format!("hook parameter '{name}' is not a nominal World"),
                        )
                    })?;
                if parameter.borrow_kind == BorrowKind::Owned {
                    return Err(case_error(
                        parameter.span,
                        "World parameters require ref or mutable ref",
                    ));
                }
                if !subscribed.insert(identity) {
                    return Err(case_error(
                        parameter.span,
                        "duplicate canonical World in hook parameters",
                    ));
                }
                origins
                    .entry(identity.clone())
                    .or_default()
                    .insert(WorldUseOrigin::Hook {
                        scenarios: scope.scenarios.clone(),
                        index,
                    });
            }
        }
    }
    let roots: Vec<_> = origins.keys().cloned().collect();
    let order = worlds
        .graph
        .initialization_order(&roots)
        .map_err(|error| case_error(span, &error.to_string()))?;
    for identity in &order {
        if let Some(dependencies) = worlds.graph.dependencies(identity) {
            for dependency in dependencies {
                origins
                    .entry(dependency.clone())
                    .or_default()
                    .insert(WorldUseOrigin::Dependency {
                        dependent: identity.clone(),
                    });
            }
        }
    }
    Ok((order, origins))
}
