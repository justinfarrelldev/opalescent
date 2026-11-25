//! Conservative compile-time detection of guaranteed conflicting World activations.
extern crate alloc;
use super::{NativeCheckError, ResolvedDoubleSet, ResolvedSuite, ResolvedWorldSet, type_error};
use crate::{
    ast::{AstNode, Expr, LambdaBody, Stmt},
    type_system::errors::TypeError,
};
use alloc::collections::BTreeMap;

/// A setup returning only `new Double` cannot restore that activation before the next World.
pub(super) fn check(
    suite: &ResolvedSuite,
    worlds: &ResolvedWorldSet,
    doubles: &ResolvedDoubleSet,
) -> Result<(), NativeCheckError> {
    for body in suite.bodies.values() {
        let mut active = BTreeMap::new();
        for identity in &body.initialization_order {
            let world = &worlds.declarations[identity];
            let setup = &world.declaration.setup.function;
            let Some(local) = direct_activation(setup) else {
                // An arbitrary callback could restore an earlier handle through a dependency.
                active.clear();
                continue;
            };
            let Some(double) = doubles
                .bindings
                .get(&world.module_path)
                .and_then(|bindings| bindings.get(local))
            else {
                continue;
            };
            let edge = &doubles.declarations[double].edge;
            if let Some(previous) = active.insert(edge, identity) {
                let reason = format!(
                    "statically conflicting double activations in Worlds '{}' and '{}' on the same import edge",
                    previous.0.name, identity.0.name
                );
                return Err(type_error(
                    &world.module_path,
                    TypeError::ConstraintSolvingFailed {
                        reason,
                        span: TypeError::span_from_span(setup.span()),
                    },
                ));
            }
        }
    }
    Ok(())
}

/// Recognize only a direct constructor in a single-result callback, not arbitrary effects.
fn direct_activation(setup: &Expr) -> Option<&str> {
    let Expr::Lambda { ref body, .. } = *setup else {
        return None;
    };
    let result = match *body {
        LambdaBody::Expression(ref expression) => expression.as_ref(),
        LambdaBody::Block(ref statements) => {
            let [Stmt::Return { ref values, .. }] = *statements.as_slice() else {
                return None;
            };
            let [ref value] = *values.as_slice() else {
                return None;
            };
            &value.value
        }
    };
    let Expr::Constructor {
        ref callee,
        ref fields,
        ..
    } = *result
    else {
        return None;
    };
    if !fields.is_empty() {
        return None;
    }
    if let Expr::Identifier { ref name, .. } = **callee {
        Some(name)
    } else {
        None
    }
}
