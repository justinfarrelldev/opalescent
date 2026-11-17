//! Generated `.state` mutability follows existing second-class reference modes.
extern crate alloc;
use super::{TypeChecker, native_error};
use crate::ast::{AstNode, BorrowKind, Expr};
use crate::type_system::errors::TypeError;
use crate::type_system::types::CoreType;
use alloc::collections::BTreeMap;

impl TypeChecker {
    /// Generic or indirect owned parameter types must not bypass the nominal
    /// reference-only World contract after call-site type inference.
    pub(in crate::type_system::checker) fn validate_native_owned_world_argument(
        &self,
        argument_type: &CoreType,
        span: crate::token::Span,
    ) -> Result<(), TypeError> {
        if matches!(*argument_type, CoreType::Generic { ref name, .. } if self.native_testing.worlds.contains_key(name))
        {
            Err(native_error(
                span,
                "World handles must be passed using ref or mutable ref, including through generic helpers",
            ))
        } else {
            Ok(())
        }
    }

    /// Generated identities and runner metadata are not user record constructors.
    pub(in crate::type_system::checker) fn validate_native_handle_record_update(
        &self,
        receiver: &CoreType,
        span: crate::token::Span,
    ) -> Result<(), TypeError> {
        if matches!(*receiver, CoreType::Generic { ref name, .. } if self.native_testing.kinds.contains_key(name) || (self.native_testing.enabled && name == "__opal_testing.TestContext"))
        {
            Err(native_error(
                span,
                "generated World/double handles and TestContext values cannot be forged or substituted through record updates",
            ))
        } else {
            Ok(())
        }
    }

    /// Generated World values are always passed as existing second-class refs.
    pub(in crate::type_system::checker) fn validate_native_world_parameter_mode(
        &self,
        parameter: &crate::ast::Parameter,
        core_type: &CoreType,
    ) -> Result<(), TypeError> {
        if parameter.borrow_kind == BorrowKind::Owned
            && matches!(*core_type, CoreType::Generic { ref name, .. } if self.native_testing.worlds.contains_key(name))
        {
            Err(native_error(
                parameter.span,
                "World handle parameters require ref or mutable ref, including in ordinary helpers",
            ))
        } else {
            Ok(())
        }
    }

    /// Apply the existing no-overlapping-mutable-reference rule to generated
    /// World bindings/projections, without introducing retained runtime borrows.
    pub(in crate::type_system::checker) fn validate_native_world_call_borrows(
        &self,
        arguments: &[Expr],
    ) -> Result<(), TypeError> {
        let mut roots = BTreeMap::new();
        for argument in arguments {
            let Expr::BorrowArgument {
                ref target,
                borrow_kind,
                ..
            } = *argument
            else {
                continue;
            };
            let Some(root) = projection_root(target) else {
                continue;
            };
            let Some(symbol) = self.symbol_table().lookup(root) else {
                continue;
            };
            if !matches!(symbol.core_type, CoreType::Generic { ref name, .. } if self.native_testing.worlds.contains_key(name))
            {
                continue;
            }
            if let Some(previous) = roots.insert(root, borrow_kind) {
                if previous == BorrowKind::MutableRef || borrow_kind == BorrowKind::MutableRef {
                    return Err(native_error(
                        argument.span(),
                        "overlapping references to the same World state cannot include a mutable ref",
                    ));
                }
            }
        }
        Ok(())
    }

    /// Borrow a generated `.state` projection under the handle's existing mode.
    /// This is not a general temporary-reference or lifetime extension facility.
    pub(in crate::type_system::checker) fn validate_native_world_state_borrow(
        &self,
        target: &Expr,
        expected: BorrowKind,
        span: crate::token::Span,
    ) -> Result<bool, TypeError> {
        let Some(root) = state_projection_root(target) else {
            return Ok(false);
        };
        let Some(symbol) = self.symbol_table().lookup(root) else {
            return Ok(false);
        };
        if !matches!(symbol.core_type, CoreType::Generic { ref name, .. } if self.native_testing.worlds.contains_key(name))
        {
            return Ok(false);
        }
        self.ensure_owner_not_moved(root, span)?;
        if expected == BorrowKind::MutableRef {
            self.validate_native_world_mutation(target)?;
        }
        Ok(true)
    }

    /// Reject writes through readonly World references and user writes to the
    /// runner-managed metadata fields of a test context.
    pub(in crate::type_system::checker) fn validate_native_world_mutation(
        &self,
        target: &Expr,
    ) -> Result<(), TypeError> {
        let Some(root) = projection_root(target) else {
            return Ok(());
        };
        let Some(symbol) = self.symbol_table().lookup(root) else {
            return Ok(());
        };
        let CoreType::Generic { ref name, .. } = symbol.core_type else {
            return Ok(());
        };
        if self.native_testing.enabled && name == "__opal_testing.TestContext" {
            return Err(native_error(
                target.span(),
                "test context metadata is managed by the runner; use its diagnostic attachment APIs",
            ));
        }
        if self.native_testing.worlds.contains_key(name)
            && self.binding_borrow_kind(root) == Some(BorrowKind::Ref)
        {
            return Err(native_error(
                target.span(),
                "World state cannot be modified through a readonly ref handle",
            ));
        }
        Ok(())
    }
}

/// Recognize only projections rooted in a generated World `.state` accessor.
fn state_projection_root(expression: &Expr) -> Option<&str> {
    match *expression {
        Expr::Member {
            ref object,
            ref member,
            ..
        } => {
            if member == "state" {
                if let Expr::Identifier { ref name, .. } = **object {
                    return Some(name.as_str());
                }
            }
            state_projection_root(object)
        }
        Expr::Index { ref object, .. } => state_projection_root(object),
        Expr::Parenthesized { ref expr, .. } => state_projection_root(expr),
        _ => None,
    }
}

/// Find the original handle binding across ordinary field/index projections.
fn projection_root(expression: &Expr) -> Option<&str> {
    match *expression {
        Expr::Identifier { ref name, .. } => Some(name.as_str()),
        Expr::Member { ref object, .. } | Expr::Index { ref object, .. } => projection_root(object),
        Expr::Parenthesized { ref expr, .. } => projection_root(expr),
        Expr::BorrowArgument { ref target, .. } => projection_root(target),
        _ => None,
    }
}
