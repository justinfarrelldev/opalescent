//! Typed constant record construction for independently scheduled examples.
extern crate alloc;
use super::{TypeChecker, native_error};
use crate::ast::testing::ExampleDeclaration;
use crate::ast::{AstNode, Expr};
use crate::type_system::checker::helpers::{coerce_literal_to_expected, type_mismatch_error};
use crate::type_system::errors::TypeError;
use crate::type_system::types::CoreType;
use alloc::collections::BTreeSet;

impl TypeChecker {
    /// Check one example against its declared record schema before any attempt.
    ///
    /// # Errors
    /// Rejects non-record schemas, missing/extra/duplicate/mistyped fields and
    /// expressions outside the supported constant-data subset.
    pub(crate) fn type_check_native_example(
        &mut self,
        record_type: &CoreType,
        example: &ExampleDeclaration,
    ) -> Result<(), TypeError> {
        let CoreType::Generic {
            ref name,
            ref type_args,
        } = *record_type
        else {
            return Err(native_error(
                example.span,
                "example schema must be an ordinary record type",
            ));
        };
        if self.adt_variants.contains_key(name) || self.native_testing.worlds.contains_key(name) {
            return Err(native_error(
                example.span,
                "example schema must be an ordinary record, not a World or sum type",
            ));
        }
        let fields = self.adt_fields.get(name).cloned().ok_or_else(|| {
            native_error(example.span, "example schema is not a declared record type")
        })?;
        let mut supplied = BTreeSet::new();
        for field in &example.fields {
            if !supplied.insert(field.name.clone()) {
                return Err(native_error(
                    field.span,
                    "duplicate example field initializer",
                ));
            }
            let expected = fields.get(&field.name).ok_or_else(|| {
                native_error(field.span, &format!("extra example field '{}'", field.name))
            })?;
            if !self.native_constant_data(&field.value) {
                return Err(native_error(
                    field.value.span(),
                    "example initializers support constant data, not runtime calls or test generation",
                ));
            }
            let expected = self.resolve_generic_adt_field_type(name, type_args, expected);
            let actual = self.type_check_expr(&field.value)?;
            let actual =
                coerce_literal_to_expected(&expected, &field.value, &actual).unwrap_or(actual);
            if !self.types_compatible(&expected, &actual) {
                return Err(type_mismatch_error(
                    &expected,
                    None,
                    &actual,
                    field.value.span(),
                ));
            }
        }
        for field_name in fields.keys() {
            if !supplied.contains(field_name) {
                return Err(native_error(
                    example.span,
                    &format!("missing example field '{field_name}'"),
                ));
            }
        }
        self.solve_constraints()?;
        Ok(())
    }

    /// V1 constant data includes literals, scalar operators/casts, nested arrays
    /// and ordinary record/enum constructors with constant fields. It excludes
    /// function calls, closures, runtime identifiers and resource constructors.
    fn native_constant_data(&self, expression: &Expr) -> bool {
        match *expression {
            Expr::Literal { .. } => true,
            Expr::Array { ref elements, .. } => elements
                .iter()
                .all(|element| self.native_constant_data(element)),
            Expr::Parenthesized { ref expr, .. } | Expr::Cast { ref expr, .. } => {
                self.native_constant_data(expr)
            }
            Expr::Unary { ref operand, .. } => self.native_constant_data(operand),
            Expr::Binary {
                ref left,
                ref right,
                ..
            } => self.native_constant_data(left) && self.native_constant_data(right),
            Expr::Constructor {
                ref callee,
                ref fields,
                ..
            } => {
                let owner = match **callee {
                    Expr::Identifier { ref name, .. } => Some(name.as_str()),
                    Expr::Member { ref object, .. } => match **object {
                        Expr::Identifier { ref name, .. } => Some(name.as_str()),
                        _ => None,
                    },
                    _ => None,
                };
                owner.is_some_and(|name| {
                    self.native_source_type(name).is_some_and(|core_type| {
                        !self.core_type_contains_affine_resource(&core_type)
                            && (self.adt_fields.contains_key(name)
                                || self.adt_variants.contains_key(name))
                    }) && fields
                        .iter()
                        .all(|field| self.native_constant_data(&field.value))
                })
            }
            _ => false,
        }
    }
}
