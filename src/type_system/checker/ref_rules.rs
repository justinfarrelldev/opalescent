extern crate alloc;

use crate::ast::{Expr, LambdaBody, Stmt};
use crate::token::Span;
use crate::type_system::checker::TypeChecker;
use crate::type_system::errors::TypeError;

impl TypeChecker {
    /// Check if a name is a reference parameter.
    ///
    /// Currently returns false as `ref_params` tracking is not yet implemented.
    #[expect(clippy::unused_self, reason = "placeholder for future implementation")]
    #[expect(
        clippy::missing_const_for_fn,
        reason = "placeholder for future implementation"
    )]
    pub(super) fn is_ref_param(&self, _name: &str) -> bool {
        false
    }

    /// Check if an expression attempts to escape a reference parameter.
    ///
    /// Currently a no-op as `ref_params` tracking is not yet implemented.
    #[expect(
        clippy::unused_self,
        reason = "placeholder for future implementation"
    )]
    #[expect(
        clippy::unnecessary_wraps,
        reason = "placeholder for future implementation"
    )]
    #[expect(
        clippy::missing_const_for_fn,
        reason = "placeholder for future implementation"
    )]
    pub(super) fn check_ref_escape_in_expr(
        &self,
        _expr: &Expr,
        _context_description: &str,
        _span: Span,
    ) -> Result<(), TypeError> {
        Ok(())
    }

    /// Check if a lambda captures any reference parameters.
    ///
    /// Currently a no-op as `ref_params` tracking is not yet implemented.
    #[expect(
        clippy::unused_self,
        reason = "placeholder for future implementation"
    )]
    #[expect(
        clippy::unnecessary_wraps,
        reason = "placeholder for future implementation"
    )]
    #[expect(
        clippy::missing_const_for_fn,
        reason = "placeholder for future implementation"
    )]
    pub(super) fn check_lambda_captures_no_ref_params(
        &self,
        _body: &LambdaBody,
        _span: Span,
    ) -> Result<(), TypeError> {
        Ok(())
    }

    /// Check if an expression captures a reference parameter.
    #[expect(
        clippy::unused_self,
        reason = "placeholder for future implementation"
    )]
    #[expect(
        clippy::unnecessary_wraps,
        reason = "placeholder for future implementation"
    )]
    #[expect(
        clippy::missing_const_for_fn,
        reason = "placeholder for future implementation"
    )]
    fn check_expr_for_ref_capture(&self, _expr: &Expr, _span: Span) -> Result<(), TypeError> {
        Ok(())
    }

    /// Check if a statement captures a reference parameter.
    #[expect(
        clippy::unused_self,
        reason = "placeholder for future implementation"
    )]
    #[expect(
        clippy::unnecessary_wraps,
        reason = "placeholder for future implementation"
    )]
    #[expect(
        clippy::missing_const_for_fn,
        reason = "placeholder for future implementation"
    )]
    fn check_stmt_for_ref_capture(&self, _stmt: &Stmt, _span: Span) -> Result<(), TypeError> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::token::Position;

    /// Helper to create a test span
    fn test_span() -> Span {
        Span::single(Position::start())
    }

    /// Test 1: `is_ref_param` returns false when no ref params registered
    #[test]
    fn test_is_ref_param_returns_false_when_empty() {
        let checker = TypeChecker::new();
        assert!(
            !checker.is_ref_param("x"),
            "is_ref_param should return false when no ref params are registered"
        );
    }

    /// Test 2: `is_ref_param` returns false (placeholder for future implementation)
    #[test]
    fn test_is_ref_param_placeholder() {
        let checker = TypeChecker::new();
        assert!(
            !checker.is_ref_param("any_name"),
            "is_ref_param placeholder returns false"
        );
    }

    /// Test 3: `check_ref_escape_in_expr` returns Ok for a non-ref identifier
    #[test]
    fn test_check_ref_escape_in_expr_ok_for_non_ref_identifier() {
        let checker = TypeChecker::new();
        let expr = Expr::Identifier {
            name: alloc::string::String::from("y"),
            span: test_span(),
            id: crate::ast::NodeId(1),
        };
        let result = checker.check_ref_escape_in_expr(&expr, "return", test_span());
        assert!(
            result.is_ok(),
            "check_ref_escape_in_expr should return Ok for non-ref identifier"
        );
    }

    /// Test 4: `check_ref_escape_in_expr` returns Ok (placeholder)
    #[test]
    fn test_check_ref_escape_in_expr_placeholder() {
        let checker = TypeChecker::new();
        let expr = Expr::Identifier {
            name: alloc::string::String::from("x"),
            span: test_span(),
            id: crate::ast::NodeId(2),
        };
        let result = checker.check_ref_escape_in_expr(&expr, "return", test_span());
        assert!(result.is_ok(), "check_ref_escape_in_expr placeholder returns Ok");
    }

    /// Test 5: `check_lambda_captures_no_ref_params` returns Ok when no ref params exist
    #[test]
    fn test_check_lambda_captures_no_ref_params_ok_when_empty() {
        let checker = TypeChecker::new();
        let body = LambdaBody::Expression(alloc::boxed::Box::new(Expr::Identifier {
            name: alloc::string::String::from("x"),
            span: test_span(),
            id: crate::ast::NodeId(3),
        }));
        let result = checker.check_lambda_captures_no_ref_params(&body, test_span());
        assert!(
            result.is_ok(),
            "check_lambda_captures_no_ref_params should return Ok when no ref params exist"
        );
    }

    /// Test 6: `check_lambda_captures_no_ref_params` returns Ok (placeholder)
    #[test]
    fn test_check_lambda_captures_no_ref_params_placeholder() {
        let checker = TypeChecker::new();
        let body = LambdaBody::Expression(alloc::boxed::Box::new(Expr::Identifier {
            name: alloc::string::String::from("x"),
            span: test_span(),
            id: crate::ast::NodeId(4),
        }));
        let result = checker.check_lambda_captures_no_ref_params(&body, test_span());
        assert!(
            result.is_ok(),
            "check_lambda_captures_no_ref_params placeholder returns Ok"
        );
    }
}
