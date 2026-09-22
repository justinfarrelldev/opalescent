//! Manifest-backed ADT layout helpers for constructor lowering.

extern crate alloc;

use crate::ast::Expr;
use crate::codegen::expressions::CodegenEnv;
use crate::type_system::types::CoreType;
use alloc::{format, vec::Vec};

/// Infer a nominal expected type from manifest-backed constructor syntax.
#[must_use]
pub(super) fn constructor_nominal_expected_type(
    env: &CodegenEnv<'_>,
    callee: &Expr,
) -> Option<CoreType> {
    match *callee {
        Expr::Identifier { ref name, .. } if env.adt_field_layout(name).is_some() => {
            Some(nominal_type(name))
        }
        Expr::Member {
            ref object,
            ref member,
            ..
        } => {
            let Expr::Identifier {
                name: ref type_name,
                ..
            } = *object.as_ref()
            else {
                return None;
            };
            let variant_owner = format!("{type_name}.{member}");
            (env.adt_variant_discriminant(variant_owner.as_str())
                .is_some()
                || env.adt_field_layout(variant_owner.as_str()).is_some())
            .then(|| nominal_type(type_name))
        }
        _ => None,
    }
}

/// Build a non-generic nominal core type by local source name.
fn nominal_type(name: &str) -> CoreType {
    CoreType::Generic {
        name: name.to_owned(),
        type_args: Vec::new(),
    }
}
