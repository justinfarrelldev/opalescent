//! Expression-statement checking, including binding-producing guards.
use super::{
    CoreType, Expr, GuardBindingInfo, GuardCheckRequest, GuardUsage, TypeChecker, TypeError,
};

impl TypeChecker {
    /// Check an ordinary expression or a guard used in statement position.
    pub(super) fn type_check_expression_statement(
        &mut self,
        expr: &Expr,
        expected_return: Option<&[CoreType]>,
    ) -> Result<(), TypeError> {
        if let Expr::Guard {
            expr: ref guarded_expr,
            binding_name: ref guard_name,
            binding_type: ref guard_type,
            is_mutable,
            else_branch: ref guard_else,
            span: guard_span,
            ..
        } = *expr
        {
            let binding_info = GuardBindingInfo {
                name: guard_name.as_str(),
                annotation: guard_type.as_ref(),
                is_mutable,
                span: guard_span,
            };
            self.type_check_guard_expr(GuardCheckRequest {
                expr: guarded_expr.as_ref(),
                binding: &binding_info,
                error_binding: None,
                else_branch: guard_else.as_ref(),
                usage: GuardUsage::Statement,
                expected_return,
            })?;
        } else {
            self.type_check_expr(expr)?;
        }
        Ok(())
    }
}
