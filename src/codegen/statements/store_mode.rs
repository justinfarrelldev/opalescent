//! Ownership mode for assignment stores, preserving existing RC behavior.
use crate::ast::Expr;
use crate::codegen::binding_store::{StoreMode, binding_requires_rc_cleanup};
use crate::type_system::types::CoreType;

/// Fresh array values and owned call results transfer ownership to bindings.
pub fn assignment_store_mode(value: &Expr, binding_type: &CoreType) -> StoreMode {
    match *value {
        Expr::Array { .. } => StoreMode::TakeOwned,
        Expr::Call { ref callee, .. } if reserve_call_returns_owned_alias(callee.as_ref()) => {
            StoreMode::TakeOwned
        }
        Expr::Call { .. } if binding_requires_rc_cleanup(binding_type) => StoreMode::TakeOwned,
        _ => StoreMode::Retain,
    }
}

/// `reserve` noop paths return an owned alias, not a borrowed pointer.
fn reserve_call_returns_owned_alias(callee: &Expr) -> bool {
    matches!(*callee, Expr::Identifier { ref name, .. } if name == "reserve")
}
